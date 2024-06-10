ALTER TABLE
    pull_requests
ADD
    COLUMN rating INTEGER NOT NULL DEFAULT 0,
ADD
    COLUMN permanent_bonus INTEGER NOT NULL DEFAULT 0,
ADD
    COLUMN streak_bonus INTEGER NOT NULL DEFAULT 0;

ALTER TABLE
    users
ADD
    COLUMN permanent_bonus INTEGER NOT NULL DEFAULT 0;

ALTER TABLE
    user_period_data
ADD
    COLUMN total_rating INTEGER NOT NULL DEFAULT 0;

ALTER TABLE
    user_period_data
ADD
    COLUMN largest_rating_per_pr INTEGER NOT NULL DEFAULT 0;
