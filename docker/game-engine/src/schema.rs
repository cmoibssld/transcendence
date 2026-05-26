diesel::table! {
    game_rounds (id) {
        id -> Int8,
        table_id -> Int4,
        round_number -> Int4,
        dealer_points -> Int4,
        result_json -> Text,
        played_at -> Timestamptz,
    }
}
