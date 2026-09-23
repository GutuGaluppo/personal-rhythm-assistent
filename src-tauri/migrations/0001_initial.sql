-- Milestone 1: settings, activity_events, sessions.
-- Later milestones add their own tables via new numbered migrations.
-- Timestamps are RFC 3339 UTC strings with millisecond precision
-- (see persistence::time), so lexicographic order == chronological order.

CREATE TABLE settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,          -- JSON
    updated_at TEXT NOT NULL
);

-- Neutral facts only (IMPLEMENTATION.md §7). No titles, URLs or content.
CREATE TABLE activity_events (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp        TEXT NOT NULL,
    type             TEXT NOT NULL
                     CHECK (type IN ('active_application', 'idle', 'application_switch')),
    bundle_id        TEXT,
    application_name TEXT,
    seconds          INTEGER,
    from_bundle_id   TEXT,
    to_bundle_id     TEXT
);
CREATE INDEX idx_activity_events_timestamp ON activity_events (timestamp);

CREATE TABLE sessions (
    id               TEXT PRIMARY KEY NOT NULL,
    started_at       TEXT NOT NULL,
    ended_at         TEXT,
    active_minutes   REAL NOT NULL DEFAULT 0,
    idle_minutes     REAL NOT NULL DEFAULT 0,
    context_switches INTEGER NOT NULL DEFAULT 0,
    category         TEXT NOT NULL
                     CHECK (category IN ('Create', 'Learn', 'Explore', 'Move', 'Life',
                                         'People', 'Recover', 'Think', 'Unknown')),
    application_ids  TEXT NOT NULL DEFAULT '[]',   -- JSON array of bundle ids
    project_id       TEXT
);
CREATE INDEX idx_sessions_started_at ON sessions (started_at);
