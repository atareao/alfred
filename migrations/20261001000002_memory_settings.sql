-- Seed the four episodic-memory knobs into the `settings` table.
--
-- Same idiom as `20260929000001_prompts.sql`: the upsert only overwrites
-- empty/NULL values so user customisations are preserved, and it is idempotent.
-- The values are read on every retrieval/assembly call (`MemoryRepo` and
-- `ContextBuilder`), so editing them in the UI takes effect without a restart.

INSERT INTO settings (key, value, updated_at)
VALUES ('MEMORY_HALF_LIFE_DAYS', '90', datetime('now'))
ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
WHERE settings.value = '' OR settings.value IS NULL;

INSERT INTO settings (key, value, updated_at)
VALUES ('SIMILARITY_THRESHOLD', '0.5', datetime('now'))
ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
WHERE settings.value = '' OR settings.value IS NULL;

INSERT INTO settings (key, value, updated_at)
VALUES ('RAG_BUDGET_TOKENS', '800', datetime('now'))
ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
WHERE settings.value = '' OR settings.value IS NULL;

INSERT INTO settings (key, value, updated_at)
VALUES ('MEMORY_KNN_CANDIDATES', '20', datetime('now'))
ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
WHERE settings.value = '' OR settings.value IS NULL;
