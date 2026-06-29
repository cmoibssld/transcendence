// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "friendship_status"))]
    pub struct FriendshipStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::FriendshipStatus;

    friendships (id) {
        id -> Int4,
        requester_id -> Int4,
        addressee_id -> Int4,
        status -> FriendshipStatus,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    refresh_tokens (id) {
        id -> Int8,
        user_id -> Int4,
        token_hash -> Text,
        created_at -> Timestamptz,
        expires_at -> Timestamptz,
        revoked_at -> Nullable<Timestamptz>,
        user_agent -> Nullable<Text>,
        ip_address -> Nullable<Text>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 50]
        username -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        password_hash -> Text,
        money -> Int8,
        avatar -> Text,
        #[max_length = 10]
        provider -> Varchar,
    }
}

diesel::joinable!(refresh_tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    friendships,refresh_tokens,users,);
