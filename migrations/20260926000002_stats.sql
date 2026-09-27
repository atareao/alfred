CREATE TABLE IF NOT EXISTS llm_requests (
    id                TEXT PRIMARY KEY,
    model             TEXT NOT NULL,
    provider          TEXT,
    profile_id        TEXT REFERENCES profiles(id),
    prompt_tokens     INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens      INTEGER NOT NULL DEFAULT 0,
    cached_tokens     INTEGER NOT NULL DEFAULT 0,
    reasoning_tokens  INTEGER NOT NULL DEFAULT 0,
    cost              REAL NOT NULL DEFAULT 0.0,
    is_byok           INTEGER NOT NULL DEFAULT 0,
    duration_ms       INTEGER,
    cache_hit         INTEGER NOT NULL DEFAULT 0,
    status            TEXT NOT NULL DEFAULT 'success'
                      CHECK(status IN ('success', 'error', 'timeout')),
    error_message     TEXT,
    tool_calls        TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);