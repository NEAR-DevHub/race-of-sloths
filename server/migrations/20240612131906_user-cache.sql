-- Add migration script here
CREATE TABLE IF NOT EXISTS user_cached_metadata (
    user_id INTEGER PRIMARY KEY,
    image_base64 TEXT NOT NULL,
    load_time TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
