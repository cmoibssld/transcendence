// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Text,
        money -> Int8,
        avatar -> Text,
        provider -> Varchar,
        bio -> Text,
        updated_at -> Timestamptz,
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
    use diesel::sql_types::*;
    use crate::models::FriendshipStatusType;

    friendships (id) {
        id -> Int4,
        requester_id -> Int4,
        addressee_id -> Int4,
        status -> FriendshipStatusType,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(refresh_tokens -> users (user_id));
diesel::joinable!(friendships -> users (requester_id));

diesel::allow_tables_to_appear_in_same_query!(
    users,
    refresh_tokens,
    friendships,
);
