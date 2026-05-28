diesel::table! {
    game_rounds (id) {
        id             -> Int8,
        table_id       -> Int4,
        round_number   -> Int4,
        dealer_points  -> Int4,
        result_json    -> Text,
        played_at      -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::models::RoundOutcomeType;

    round_players (id) {
        id        -> Int8,
        round_id  -> Int8,
        user_id   -> Int8,
        bet       -> Int8,
        outcome   -> RoundOutcomeType,
        payout    -> Int8,
        played_at -> Timestamptz,
    }
}

diesel::table! {
    user_stats (user_id) {
        user_id       -> Int8,
        games_played  -> Int4,
        wins          -> Int4,
        losses        -> Int4,
        pushes        -> Int4,
        total_wagered -> Int8,
        total_won     -> Int8,
        updated_at    -> Timestamptz,
    }
}

diesel::joinable!(round_players -> game_rounds (round_id));

diesel::allow_tables_to_appear_in_same_query!(
    game_rounds,
    round_players,
    user_stats,
);
