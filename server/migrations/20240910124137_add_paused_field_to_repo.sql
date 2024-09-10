-- Add migration script here
ALTER TABLE
    repos
ADD
    COLUMN paused BOOLEAN NOT NULL DEFAULT FALSE;
