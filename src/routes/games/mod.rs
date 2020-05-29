use async_std::io::Read as AsyncRead;
use chrono::{DateTime, Utc};
use log::{debug, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use std::io::Result;
use std::marker::Unpin;

use crate::{
  errors,
  http::{query_values, Uri},
  interchange, read_size_async,
  routes::lobbies::LOAD_LOBBY_DETAILS,
  Authority, Context, Response,
};

const LOAD_GAME: &'static str = include_str!("data-store/load-game-details.sql");
const LOAD_MEMBERS: &'static str = include_str!("data-store/load-game-members.sql");
const LOAD_ROUNDS: &'static str = include_str!("data-store/load-rounds.sql");
const GAME_FOR_ENTRY: &'static str = include_str!("data-store/game-for-entry-creation.sql");
const CREATE_ENTRY: &'static str = include_str!("data-store/create-round-entry.sql");

#[derive(Debug, Deserialize)]
struct EntryPayload {
  pub round_id: String,
  pub entry: String,
}

pub async fn create_entry<R: AsyncRead + Unpin>(
  context: &Context,
  reader: &mut R,
) -> Result<Response> {
  let uid = match context.authority() {
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<EntryPayload>(&contents)?;
  let authority = match context
    .records()
    .query(GAME_FOR_ENTRY, &[&payload.round_id, &uid])?
    .iter()
    .nth(0)
    .and_then(|row| {
      let lobby_id = row.try_get::<_, String>(0).map_err(log_err).ok()?;
      let game_id = row.try_get::<_, String>(1).map_err(log_err).ok()?;
      let round_id = row.try_get::<_, String>(2).map_err(log_err).ok()?;
      let member_id = row.try_get::<_, String>(3).map_err(log_err).ok()?;
      let user_id = row.try_get::<_, String>(4).map_err(log_err).ok()?;
      Some((lobby_id, game_id, round_id, member_id, user_id))
    }) {
    None => {
      warn!(
        "unable to find game for user '{}' by round '{}'",
        uid, payload.round_id
      );
      return Ok(Response::not_found().cors(context.cors()));
    }
    Some(game) => game,
  };

  let created = context
    .records()
    .query(
      CREATE_ENTRY,
      &[
        &authority.2,   // round_id
        &authority.3,   // member_id
        &payload.entry, // entry
        &authority.1,   // game_id
        &authority.0,   // lobby_id
        &authority.4,   // user_id
      ],
    )
    .map_err(log_err)?
    .iter()
    .nth(0)
    .and_then(|row| {
      let entry_id = row.try_get::<_, String>(0).map_err(log_err).ok()?;
      let entry = row.try_get::<_, String>(1).map_err(log_err).ok()?;
      let round_id = row.try_get::<_, String>(2).map_err(log_err).ok()?;
      Some((entry_id, entry, round_id))
    });

  debug!("creating round entry for user '{}' - {:?}", uid, created);

  match created {
    Some((_entry_id, entry, round_id)) => {
      debug!("successfully created entry - {:?}", entry);

      context
        .jobs()
        .queue(&interchange::jobs::Job::CheckRoundFulfillment {
          round_id,
          result: None,
        })
        .await
        .map(|_id| Response::default().cors(context.cors()))
        .or_else(|e| {
          log_err(e);
          Ok(Response::default().cors(context.cors()))
        })
    }
    None => {
      warn!("round entry creation did not return information from inserted entry");
      return Ok(Response::default().cors(context.cors()));
    }
  }
}

#[derive(Deserialize)]
pub struct CreatePayload {
  pub lobby_id: String,
}

fn log_err<E: std::error::Error>(error: E) -> E {
  warn!("error - {}", error);
  error
}

fn members_for_game(context: &Context, id: &String) -> Result<Vec<interchange::http::GameMember>> {
  context
    .records()
    .query(LOAD_MEMBERS, &[&id])?
    .iter()
    .map(|r| {
      let member_id = r.try_get("member_id").map_err(errors::humanize_error)?;
      let joined = r.try_get("created_at").map_err(errors::humanize_error)?;
      let user_id = r.try_get("user_id").map_err(errors::humanize_error)?;
      let email = r.try_get("user_email").map_err(errors::humanize_error)?;
      let name = r.try_get("user_name").map_err(errors::humanize_error)?;

      debug!("found member '{}'", id);

      Ok(interchange::http::GameMember {
        member_id,
        user_id,
        email,
        name,
        joined,
      })
    })
    .collect()
}

fn rounds_for_game(context: &Context, id: &String) -> Result<Vec<interchange::http::GameRound>> {
  context
    .records()
    .query(LOAD_ROUNDS, &[&id])?
    .iter()
    .map(|row| {
      let id = row.try_get("id").map_err(errors::humanize_error)?;
      let position = row
        .try_get::<_, i32>("pos")
        .map_err(errors::humanize_error)? as u32;
      let prompt = row.try_get("prompt").map_err(errors::humanize_error)?;
      let created = row.try_get("created_at").map_err(errors::humanize_error)?;
      let started = row.try_get("started_at").map_err(errors::humanize_error)?;
      let completed = row
        .try_get("completed_at")
        .map_err(errors::humanize_error)?;
      let fulfilled = row
        .try_get("fulfilled_at")
        .map_err(errors::humanize_error)?;

      debug!("found round '{}' ({:?}, {:?})", id, position, completed);

      Ok(interchange::http::GameRound {
        id,
        position,
        prompt,
        created,
        started,
        fulfilled,
        completed,
      })
    })
    .collect()
}

async fn find_game(context: &Context, uid: &String, gid: &String) -> Result<Response> {
  let (id, created, name) = match context
    .records()
    .query(LOAD_GAME, &[gid, uid])?
    .iter()
    .nth(0)
    .and_then(|r| {
      let id = r.try_get::<_, String>(0).ok()?;
      let created = r
        .try_get::<_, DateTime<Utc>>(1)
        .map_err(|e| {
          warn!("unable to parse time value as datetime - {}", e);
          errors::e("bad date time")
        })
        .ok()?;
      let name = r.try_get::<_, String>(2).ok()?;

      Some((id, created, name))
    }) {
    Some(contents) => contents,
    None => return Ok(Response::not_found().cors(context.cors())),
  };

  debug!("found game '{}', created '{:?}'", id, created);

  let rounds = rounds_for_game(context, &id).map_err(log_err)?;
  let members = members_for_game(context, &id).map_err(log_err)?;

  debug!("found members[{:?}] rounds[{:?}]", members, &rounds);

  let result = interchange::http::GameDetails {
    id,
    created,
    name,
    members,
    rounds,
  };

  Response::ok_json(&result).map(|r| r.cors(context.cors()))
}

pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
  };

  let ids = query_values(uri, "ids[]");

  if ids.len() != 1 {
    debug!("find all games not implemented yet");
    return Ok(Response::not_found().cors(context.cors()));
  }

  let gid = ids.iter().nth(0).ok_or(errors::e("invalid id"))?;
  debug!("attempting to find game from single id - {:?}", gid);
  find_game(context, uid, gid).await
}

pub async fn create<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
    Authority::User { id, .. } => id,
  };
  debug!("creating new game for user - {}", uid);
  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<CreatePayload>(&contents)?;

  if let None = context
    .records()
    .query(LOAD_LOBBY_DETAILS, &[&payload.lobby_id, &uid])?
    .iter()
    .nth(0)
  {
    warn!(
      "unable to find lobby '{}' for user '{}'",
      payload.lobby_id, uid
    );
    return Ok(Response::not_found().cors(context.cors()));
  }

  debug!(
    "lobby exists and ready for new game, queuing job for lobby '{}'",
    payload.lobby_id
  );

  let job_id = context
    .jobs()
    .queue(&interchange::jobs::Job::CreateGame {
      creator: uid.clone(),
      lobby_id: payload.lobby_id.clone(),
      result: None,
    })
    .await?;

  Response::ok_json(interchange::http::JobHandle {
    id: job_id.clone(),
    result: None,
  })
  .map(|r| r.cors(context.cors()))
}
