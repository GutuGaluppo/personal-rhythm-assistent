-- Post-v0.1: quick (untimed) pauses and an optional reason tag, from real usage
-- during the Milestone 13 behavioral test.
--
-- SQLite cannot drop a NOT NULL constraint via ALTER TABLE, so `pauses` is
-- rebuilt here (create/copy/drop/rename) rather than extended with ADD COLUMN
-- as earlier migrations do -- this is the one exception to that pattern.
-- `pauses` has no incoming foreign keys, so no FK pragma juggling is needed.

CREATE TABLE pauses_new (
    id              TEXT PRIMARY KEY NOT NULL,
    started_at      TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('silence', 'meditation', 'walking', 'stretching')),
    -- NULL means a quick, untimed pause: no deadline, ended only by the user.
    planned_seconds INTEGER,
    ended_at        TEXT,
    reason          TEXT CHECK (reason IS NULL OR reason IN
                        ('breakfast', 'lunch', 'dinner', 'appointment', 'call', 'other')),
    -- Only meaningful (and set) when reason = 'other'.
    reason_note     TEXT
);

INSERT INTO pauses_new (id, started_at, kind, planned_seconds, ended_at)
    SELECT id, started_at, kind, planned_seconds, ended_at FROM pauses;

DROP TABLE pauses;
ALTER TABLE pauses_new RENAME TO pauses;

CREATE INDEX idx_pauses_started_at ON pauses (started_at);
