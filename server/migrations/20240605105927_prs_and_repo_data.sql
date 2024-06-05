CREATE TABLE IF NOT EXISTS organizations (
    id SERIAL PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS repos (
    id SERIAL PRIMARY KEY,
    organization_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    UNIQUE (organization_id, name)
);

CREATE TABLE IF NOT EXISTS pull_requests (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER REFERENCES repos(id) ON DELETE CASCADE,
    number INTEGER NOT NULL,
    author_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL,
    merged_at TIMESTAMP,
    executed BOOLEAN NOT NULL,
    score INTEGER,
    UNIQUE (repo_id, number)
);
