BEGIN;

-- Step 1: Create the new streak table
CREATE TABLE IF NOT EXISTS streak (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    period TEXT NOT NULL
);

-- Step 2: Insert any missing streak entries into the new streak table
INSERT INTO
    streak (id, name, period)
VALUES
    (0, 'Weekly Pull Request', 'Weekly'),
    (
        1,
        'Monthly Pull Request with score higher 8',
        'Monthly'
    );

ALTER TABLE
    streak_user_data
ADD
    COLUMN new_streak_id INTEGER;

-- Step 4: Copy data from the old streak_id column to the new_streak_id column
UPDATE
    streak_user_data
SET
    new_streak_id = streak_user_data.streak_id;

-- Step 5: Drop the old streak_id column
ALTER TABLE
    streak_user_data DROP COLUMN streak_id;

-- Step 6: Rename the new_streak_id column to streak_id
ALTER TABLE
    streak_user_data RENAME COLUMN new_streak_id TO streak_id;

-- Step 7: Add the foreign key constraint to the new streak_id column
ALTER TABLE
    streak_user_data
ADD
    CONSTRAINT fk_streak_id FOREIGN KEY (streak_id) REFERENCES streak(id) ON DELETE CASCADE;

-- Step 8: Re-add the primary key constraint
ALTER TABLE
    streak_user_data
ADD
    PRIMARY KEY (user_id, streak_id);

ALTER TABLE
    repos
ADD
    COLUMN primary_language TEXT,
ADD
    COLUMN open_issues INTEGER,
ADD
    COLUMN stars INTEGER,
ADD
    COLUMN forks INTEGER;

COMMIT;
