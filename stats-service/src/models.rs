use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SqlType;
use std::io::Write;

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::schema::game_rounds)]
pub struct GameRound {
    pub id:            i64,
    pub table_id:      i32,
    pub round_number:  i32,
    pub dealer_points: i32,
    pub result_json:   String,
    pub played_at:     DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::game_rounds)]
pub struct NewGameRound<'a> {
    pub table_id:      i32,
    pub round_number:  i32,
    pub dealer_points: i32,
    pub result_json:   &'a str,
}

#[derive(SqlType, Clone)]
#[diesel(postgres_type(name = "round_outcome"))]
pub struct RoundOutcomeType;

impl diesel::query_builder::QueryId for RoundOutcomeType {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::AsExpression, diesel::FromSqlRow, serde::Serialize, serde::Deserialize)]
#[diesel(sql_type = RoundOutcomeType)]
pub enum RoundOutcome {
    #[serde(rename = "win")]
    Win,
    #[serde(rename = "blackjack")]
    Blackjack,
    #[serde(rename = "push")]
    Push,
    #[serde(rename = "loss")]
    Loss,
}

impl ToSql<RoundOutcomeType, Pg> for RoundOutcome {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let s = match self {
            RoundOutcome::Win       => "win",
            RoundOutcome::Blackjack => "blackjack",
            RoundOutcome::Push      => "push",
            RoundOutcome::Loss      => "loss",
        };
        out.write_all(s.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<RoundOutcomeType, Pg> for RoundOutcome {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"win"       => Ok(RoundOutcome::Win),
            b"blackjack" => Ok(RoundOutcome::Blackjack),
            b"push"      => Ok(RoundOutcome::Push),
            b"loss"      => Ok(RoundOutcome::Loss),
            other => Err(format!("Unknown round_outcome: {:?}", other).into()),
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::schema::round_players)]
pub struct RoundPlayer {
    pub id:        i64,
    pub round_id:  i64,
    pub user_id:   i64,
    pub bet:       i64,
    pub outcome:   RoundOutcome,
    pub payout:    i64,
    pub played_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::round_players)]
pub struct NewRoundPlayer {
    pub round_id: i64,
    pub user_id:  i64,
    pub bet:      i64,
    pub outcome:  RoundOutcome,
    pub payout:   i64,
}

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::schema::user_stats)]
#[diesel(primary_key(user_id))]
pub struct UserStats {
    pub user_id:       i64,
    pub games_played:  i32,
    pub wins:          i32,
    pub losses:        i32,
    pub pushes:        i32,
    pub total_wagered: i64,
    pub total_won:     i64,
    pub updated_at:    DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::user_stats)]
pub struct NewUserStats {
    pub user_id:       i64,
    pub games_played:  i32,
    pub wins:          i32,
    pub losses:        i32,
    pub pushes:        i32,
    pub total_wagered: i64,
    pub total_won:     i64,
}
