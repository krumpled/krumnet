create extension if not exists "uuid-ossp";

drop schema if exists krumnet cascade;

create schema krumnet;

drop table if exists krumnet.users cascade;

create table krumnet.users (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  default_email varchar unique not null,
  name varchar not null,
  created_at timestamp with time zone default now()
);

drop table if exists krumnet.google_accounts cascade;

create table krumnet.google_accounts (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  email varchar unique not null,
  name varchar not null,
  google_id varchar unique not null,
  user_id varchar(36) references krumnet.users(id) not null
);

drop table if exists krumnet.lobbies cascade;

create table krumnet.lobbies (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  job_id varchar not null,
  name varchar unique not null,

  created_at timestamp with time zone default now(),
  closed_at timestamp with time zone
);

drop table if exists krumnet.lobby_memberships cascade;

create table krumnet.lobby_memberships (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  user_id varchar(36) references krumnet.users(id) not null,
  lobby_id varchar(36) references krumnet.lobbies(id) not null,
  invited_by varchar(36) references krumnet.users(id),
  joined_at timestamp with time zone,
  left_at timestamp with time zone,
  constraint single_membership unique (user_id, lobby_id)
);

drop table if exists krumnet.games cascade;

create table krumnet.games (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  job_id varchar not null,
  name varchar unique not null,
  lobby_id varchar(36) references krumnet.lobbies(id) not null,
  created_at timestamp with time zone default now(),
  ended_at timestamp with time zone
);

drop table if exists krumnet.game_memberships cascade;

create table krumnet.game_memberships (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  user_id varchar(36) references krumnet.users(id) not null,
  lobby_id varchar(36) references krumnet.lobbies(id) not null,
  lobby_member_id varchar(36) references krumnet.lobby_memberships(id) not null,
  game_id varchar(36) references krumnet.games(id) not null,
  created_at timestamp with time zone default now(),
  left_at timestamp with time zone
);

create index on krumnet.game_memberships (lobby_id);

drop table if exists krumnet.prompts cascade;

create table krumnet.prompts (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  number serial,
  prompt varchar unique not null,
  source varchar,
  created_by varchar(36) references krumnet.users(id),
  approved boolean not null default false,
  created_at timestamp with time zone default now()
);

drop table if exists krumnet.game_rounds cascade;

create table krumnet.game_rounds (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  lobby_id varchar(36) references krumnet.lobbies(id) not null,
  game_id varchar(36) references krumnet.games(id) not null,
  position int not null,
  prompt varchar,
  created_at timestamp with time zone default now(),
  started_at timestamp with time zone,
  fulfilled_at timestamp with time zone,
  completed_at timestamp with time zone,
  unique (game_id, position),
  constraint started_after_created check (started_at >= created_at),
  constraint fulfilled_after_started check (fulfilled_at > started_at),
  constraint completed_after_fulfilled check (completed_at > fulfilled_at)
);

create index on krumnet.game_rounds (lobby_id);

drop table if exists krumnet.game_round_entries cascade;

create table krumnet.game_round_entries (
  id varchar(36)  unique default uuid_generate_v4() PRIMARY KEY,
  round_id varchar(36) references krumnet.game_rounds(id) not null,
  member_id varchar(36) references krumnet.game_memberships(id) not null,
  user_id varchar(36) references krumnet.users(id) not null,
  game_id varchar(36) references krumnet.games(id) not null,
  lobby_id varchar(36) references krumnet.lobbies(id) not null,
  entry varchar,
  auto boolean default false,
  created_at timestamp with time zone default now(),
  UNIQUE (round_id, member_id)
);

create index on krumnet.game_round_entries (lobby_id);
create index on krumnet.game_round_entries (game_id);

drop table if exists krumnet.game_round_entry_votes cascade;

create table krumnet.game_round_entry_votes (
  id varchar(36) unique default uuid_generate_v4() PRIMARY KEY,
  round_id varchar(36) references krumnet.game_rounds(id) not null,
  lobby_id varchar(36) references krumnet.lobbies(id) not null,
  game_id varchar(36) references krumnet.games(id) not null,
  member_id varchar(36) references krumnet.game_memberships(id) not null,
  user_id varchar(36) references krumnet.users(id) not null,
  entry_id varchar(36) references krumnet.game_round_entries(id) not null,
  created_at timestamp with time zone default now(),
  UNIQUE (round_id, member_id)
);
