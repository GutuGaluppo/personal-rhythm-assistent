-- Milestone 9: the Interest Inbox. Plain notes to self; no tags, no scoring.

CREATE TABLE interests (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    text              TEXT NOT NULL CHECK (length(text) BETWEEN 1 AND 280),
    created_at        TEXT NOT NULL,
    archived_at       TEXT,
    -- Local calendar day ("2026-03-04") this was last offered as the day's suggestion.
    last_suggested_on TEXT
);
CREATE INDEX idx_interests_active ON interests (archived_at, created_at);
