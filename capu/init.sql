CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    money BIGINT NOT NULL DEFAULT 0,
    avatar TEXT NOT NULL DEFAULT ''
);

INSERT INTO users (username, email, password_hash, money, avatar)
VALUES
    ('capu_player', 'capu@example.com', 'hashed_password', 1200, 'avatar_knight')
ON CONFLICT (email) DO UPDATE
SET username = EXCLUDED.username,
    money = EXCLUDED.money,
    avatar = EXCLUDED.avatar;
