CREATE TYPE round_outcome AS ENUM ('win', 'blackjack', 'push', 'loss');

CREATE TABLE round_players (
    id          BIGSERIAL PRIMARY KEY,
    round_id    BIGINT NOT NULL REFERENCES game_rounds(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL,
    bet         BIGINT NOT NULL,
    outcome     round_outcome NOT NULL,
    payout      BIGINT NOT NULL,
    played_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX round_players_user_id_idx  ON round_players(user_id);
CREATE INDEX round_players_round_id_idx ON round_players(round_id);
