use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SqlType;
use std::io::Write;

#[derive(SqlType)]
#[diesel(postgres_type(name = "friendship_status"))]
pub struct FriendshipStatusType;

impl diesel::query_builder::QueryId for FriendshipStatusType {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::AsExpression, diesel::FromSqlRow, serde::Serialize, serde::Deserialize)]
#[diesel(sql_type = FriendshipStatusType)]
pub enum FriendshipStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "declined")]
    Declined,
}

impl ToSql<FriendshipStatusType, Pg> for FriendshipStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let s = match self {
            FriendshipStatus::Pending  => "pending",
            FriendshipStatus::Accepted => "accepted",
            FriendshipStatus::Declined => "declined",
        };
        out.write_all(s.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<FriendshipStatusType, Pg> for FriendshipStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"pending"  => Ok(FriendshipStatus::Pending),
            b"accepted" => Ok(FriendshipStatus::Accepted),
            b"declined" => Ok(FriendshipStatus::Declined),
            other => Err(format!("Unknown friendship_status: {:?}", other).into()),
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub money: i64,
    pub avatar: String,
    pub provider: String,
    pub bio: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password_hash: &'a str,
    pub money: i64,
    pub avatar: &'a str,
    pub provider: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::users)]
pub struct UserProfileChangeset {
    pub username: Option<String>,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub password_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = crate::schema::refresh_tokens)]
#[diesel(belongs_to(User))]
pub struct RefreshToken {
    pub id: i64,
    pub user_id: i32,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::refresh_tokens)]
pub struct NewRefreshToken<'a> {
    pub user_id: i32,
    pub token_hash: &'a str,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<&'a str>,
    pub ip_address: Option<&'a str>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = crate::schema::friendships)]
pub struct Friendship {
    pub id: i32,
    pub requester_id: i32,
    pub addressee_id: i32,
    pub status: FriendshipStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::friendships)]
pub struct NewFriendship {
    pub requester_id: i32,
    pub addressee_id: i32,
}
