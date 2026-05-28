ALTER TABLE users ADD COLUMN provider VARCHAR(10) NOT NULL DEFAULT 'local';
UPDATE users SET provider = 'github' WHERE email LIKE '%@github.local';
