CREATE TABLE game_rounds (
    id BIGSERIAL PRIMARY KEY,
    table_id INTEGER NOT NULL,
    round_number INTEGER NOT NULL,
    dealer_points INTEGER NOT NULL,
    result_json TEXT NOT NULL,
    played_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX game_rounds_table_id_idx ON game_rounds(table_id);
CREATE INDEX game_rounds_played_at_idx ON game_rounds(played_at);
