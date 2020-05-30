select
  count(*)
from
  krumnet.game_rounds as rounds
where
  rounds.game_id = $1
and
  rounds.completed_at is null;
