-- Add migration script here
ALTER TABLE
    repos
ADD
    COLUMN banned BOOLEAN NOT NULL DEFAULT FALSE;
