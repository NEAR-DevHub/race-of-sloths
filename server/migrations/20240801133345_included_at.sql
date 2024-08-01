ALTER TABLE
    pull_requests
ADD
    COLUMN included_at TIMESTAMP NOT NULL DEFAULT now();
