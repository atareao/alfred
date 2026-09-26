-- Recreate tasks table with GTD statuses
CREATE TABLE IF NOT EXISTS tasks_new (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'inbox' CHECK(status IN ('inbox', 'todo', 'doing', 'waiting', 'someday', 'done')),
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('low', 'medium', 'high')),
    project TEXT,
    due_date TEXT,
    scope TEXT NOT NULL DEFAULT 'shared' CHECK(scope IN ('shared', 'personal')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Copy existing data, mapping old statuses to new ones
INSERT INTO tasks_new (id, profile_id, content, status, priority, project, due_date, scope, created_at, updated_at)
SELECT id, profile_id, content,
    CASE status
        WHEN 'pending' THEN 'inbox'
        WHEN 'completed' THEN 'done'
        WHEN 'cancelled' THEN 'done'
        ELSE 'inbox'
    END,
    priority, project, due_date, scope, created_at, updated_at
FROM tasks;

-- Drop old table
DROP TABLE IF EXISTS tasks;

-- Rename new table
ALTER TABLE tasks_new RENAME TO tasks;