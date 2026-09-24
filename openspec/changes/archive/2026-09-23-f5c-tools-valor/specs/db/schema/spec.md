# Schema: New Tables for F5c

## Contracts

```sql
-- Meal plans: weekly meal schedule
CREATE TABLE IF NOT EXISTS meal_plans (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    week_start TEXT NOT NULL,           -- ISO date (Monday)
    meals TEXT NOT NULL DEFAULT '{}',   -- JSON: { "monday": { "lunch": "...", "dinner": "..." }, ... }
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Shopping list items
CREATE TABLE IF NOT EXISTS shopping_list (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    item TEXT NOT NULL,
    quantity TEXT,
    category TEXT,                      -- "verduras", "lácteos", "despensa", "carnes", "congelados"
    checked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Habits tracking
CREATE TABLE IF NOT EXISTS habits (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    name TEXT NOT NULL,
    frequency TEXT NOT NULL DEFAULT 'daily' CHECK(frequency IN ('daily', 'weekly')),
    target INTEGER,                     -- target per period (e.g. 30 min)
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Habit logs (one per day per habit)
CREATE TABLE IF NOT EXISTS habit_logs (
    habit_id TEXT NOT NULL REFERENCES habits(id),
    date TEXT NOT NULL,                  -- ISO date
    completed INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (habit_id, date)
);
```

## Scenarios

### Migration: tables created on init
**Given** a fresh database  
**When** `run_migrations` is called  
**Then** the tables `meal_plans`, `shopping_list`, `habits`, `habit_logs` exist

### Migration: idempotent
**Given** a database with existing tables  
**When** `run_migrations` is called again  
**Then** no error occurs (CREATE IF NOT EXISTS)

### FTS5: not needed for these tables
**Given** meal plans, shopping lists, and habits are structured data  
**Then** they are not indexed in FTS5 (queried by profile_id + date, not full-text search)