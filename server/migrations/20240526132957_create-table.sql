CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS user_period_data (
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    period_type TEXT NOT NULL,
    total_score INTEGER NOT NULL,
    executed_prs INTEGER NOT NULL,
    largest_score INTEGER NOT NULL,
    prs_opened INTEGER NOT NULL,
    prs_merged INTEGER NOT NULL,
    PRIMARY KEY (user_id, period_type)
);

CREATE TABLE IF NOT EXISTS streak_user_data (
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    streak_id INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    best INTEGER NOT NULL,
    latest_time_string TEXT NOT NULL,
    PRIMARY KEY (user_id, streak_id)
);

-- Indexes for leaderboard queries
CREATE INDEX IF NOT EXISTS idx_user_period_data_total_score ON user_period_data (total_score);

CREATE INDEX IF NOT EXISTS idx_user_period_data_prs_opened ON user_period_data (prs_opened);

CREATE INDEX IF NOT EXISTS idx_user_period_data_prs_merged ON user_period_data (prs_merged);
