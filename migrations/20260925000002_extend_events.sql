ALTER TABLE events ADD COLUMN category TEXT NOT NULL DEFAULT 'default';
ALTER TABLE events ADD COLUMN all_day INTEGER NOT NULL DEFAULT 0;
ALTER TABLE events ADD COLUMN rrule TEXT;
ALTER TABLE events ADD COLUMN reminder_minutes_before INTEGER;

CREATE INDEX IF NOT EXISTS idx_events_category ON events(category);
CREATE INDEX IF NOT EXISTS idx_events_start_time ON events(start_time);