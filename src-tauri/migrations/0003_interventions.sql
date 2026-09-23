-- Milestone 7: what was shown, and how the user answered.

CREATE TABLE interventions (
    id             TEXT PRIMARY KEY NOT NULL,
    shown_at       TEXT NOT NULL,
    is_retry       INTEGER NOT NULL DEFAULT 0,
    active_minutes REAL NOT NULL,
    reasons        TEXT NOT NULL DEFAULT '[]'   -- JSON array: the evidence shown to the user
);
CREATE INDEX idx_interventions_shown_at ON interventions (shown_at);

-- One row per answer: the energy answer, then the final action (or 'ignored').
CREATE TABLE intervention_feedback (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    intervention_id TEXT NOT NULL REFERENCES interventions (id) ON DELETE CASCADE,
    at              TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('energy', 'action')),
    value           TEXT NOT NULL
);
CREATE INDEX idx_intervention_feedback_intervention ON intervention_feedback (intervention_id);
