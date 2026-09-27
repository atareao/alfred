-- Episodic memory stores individual "fichas" (cards) of remembered information.
-- Each memory record holds one discrete fact or observation with metadata
-- for categorization and source tracking.

CREATE TABLE IF NOT EXISTS memory (
    id           TEXT PRIMARY KEY,
    content      TEXT NOT NULL,
    tokens_count INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    metadata     TEXT DEFAULT '{}'
);

-- Vector embeddings for memory records, stored as JSON arrays.
-- In a future iteration this will be migrated to a sqlite-vec virtual table
-- to enable efficient vector search (k-NN) on embeddings.
CREATE TABLE IF NOT EXISTS vec_memory (
    id        TEXT PRIMARY KEY,              -- same ID as memory.id
    embedding TEXT NOT NULL DEFAULT '[]'     -- f32 vector as JSON array
);

-- Partial index for finding messages that haven't been indexed into memory yet
CREATE INDEX IF NOT EXISTS idx_messages_unindexed
ON messages(created_at)
WHERE is_indexed = 0;

-- Partial index for messages that have a summary reference
CREATE INDEX IF NOT EXISTS idx_messages_summary_ref
ON messages(summary_ref)
WHERE summary_ref IS NOT NULL;

-- Migrate legacy data from old memories table if it exists
INSERT OR IGNORE INTO memory (id, content, tokens_count, created_at, metadata)
SELECT id, content, 0, created_at, '{"source":"legacy","category":"' || COALESCE(category,'general') || '"}'
FROM memories;

INSERT OR IGNORE INTO vec_memory (id, embedding)
SELECT id, COALESCE((SELECT embedding FROM memory_embeddings WHERE id = memories.id), '[]')
FROM memories;

-- Remove legacy tables
DROP TABLE IF EXISTS memories_fts;
DROP TABLE IF EXISTS memories;
DROP TABLE IF EXISTS memory_embeddings;