-- Milestone 10: pauses taken, and one summary per local day.

-- Every pause that was actually started, from any entry point.
CREATE TABLE pauses (
    id              TEXT PRIMARY KEY NOT NULL,
    started_at      TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('silence', 'meditation', 'walking', 'stretching')),
    planned_seconds INTEGER NOT NULL,
    ended_at        TEXT
);
CREATE INDEX idx_pauses_started_at ON pauses (started_at);

-- Kept indefinitely by default (IMPLEMENTATION.md §21). `payload` is the finished
-- summary as JSON, written once the day is over, so it stays readable after the
-- sessions it came from have been pruned. `reflection` is the user's own words.
CREATE TABLE daily_summaries (
    date         TEXT PRIMARY KEY NOT NULL,   -- local calendar day, YYYY-MM-DD
    finalized_at TEXT,
    payload      TEXT,
    reflection   TEXT
);
