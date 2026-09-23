-- Post-v0.1: Today's plan, a free-text checklist scoped to a local day, from
-- real usage during the Milestone 13 behavioral test.

CREATE TABLE daily_plan_items (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    day        TEXT NOT NULL,   -- local calendar day, "YYYY-MM-DD"
    text       TEXT NOT NULL CHECK (length(text) BETWEEN 1 AND 280),
    created_at TEXT NOT NULL,
    done_at    TEXT
);
CREATE INDEX idx_daily_plan_items_day ON daily_plan_items (day, created_at);
