-- Migrate the episodic vector index from a regular JSON table to the
-- sqlite-vec `vec0` virtual table.
--
-- Three things here are load-bearing:
--
-- * `id TEXT PRIMARY KEY` is declared explicitly. Without it `vec0` exposes
--   only the implicit `rowid` and the `memory m JOIN vec_memory v ON m.id = v.id`
--   join by id is impossible (SQLite errors with
--   `table vec_items has no column named id`).
-- * `distance_metric=cosine` makes `distance = 1 - similarity` (identical
--   vectors → 0, orthogonal → 1), which is the formula the retrieval
--   pipeline's decay depends on. Without it `vec0` defaults to L2 and the
--   threshold would filter backwards.
-- * `DROP TABLE IF EXISTS` first, because databases created before this
--   migration still hold the old regular `vec_memory` table.
--
-- The dimension (`1024`) must match `EMBEDDING_DIMENSION`; start-up verifies
-- this and refuses to run on a mismatch (D10).

DROP TABLE IF EXISTS vec_memory;

CREATE VIRTUAL TABLE vec_memory USING vec0(
    id TEXT PRIMARY KEY,
    embedding float[1024] distance_metric=cosine
);
