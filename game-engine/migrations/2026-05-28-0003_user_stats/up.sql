CREATE TABLE user_stats (
    user_id         BIGINT PRIMARY KEY,
    games_played    INT NOT NULL DEFAULT 0,
    wins            INT NOT NULL DEFAULT 0,
    losses          INT NOT NULL DEFAULT 0,
    pushes          INT NOT NULL DEFAULT 0,
    total_wagered   BIGINT NOT NULL DEFAULT 0,
    total_won       BIGINT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
