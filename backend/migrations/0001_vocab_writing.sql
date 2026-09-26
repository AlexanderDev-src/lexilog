-- Phase 1: vocabulary cards (with FSRS state), review history, writing.
--
-- Conventions:
--   * Timestamps are UTC text, e.g. '2026-09-26T03:15:00Z'. This format
--     sorts correctly as plain text, so `due < ?` comparisons just work.
--   * Plain dates are 'YYYY-MM-DD'.
--   * Optional text is NOT NULL DEFAULT '' instead of NULL, so the Rust
--     side can use `String` instead of `Option<String>`.

CREATE TABLE cards (
    id          INTEGER PRIMARY KEY,
    word        TEXT NOT NULL,
    meaning     TEXT NOT NULL DEFAULT '',
    example     TEXT NOT NULL DEFAULT '',     -- the real sentence where I found it
    source      TEXT NOT NULL DEFAULT '',     -- "Guardian", "Cambridge 18 Test 2"
    -- FSRS memory state. stability IS NULL means a new, never-reviewed card.
    stability   REAL,
    difficulty  REAL,
    due         TEXT NOT NULL,                -- new card: due = created_at
    last_review TEXT,
    reps        INTEGER NOT NULL DEFAULT 0,
    lapses      INTEGER NOT NULL DEFAULT 0,   -- times forgotten after a gap
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_cards_due ON cards(due);
CREATE INDEX idx_cards_word ON cards(word COLLATE NOCASE);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE  -- "environment", "education"
);

CREATE TABLE card_tags (
    card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, tag_id)
);

-- Full review history. The FSRS optimizer only needs card_id, rating and
-- reviewed_at; the before/after columns help debugging and a future "undo".
CREATE TABLE review_logs (
    id                INTEGER PRIMARY KEY,
    card_id           INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    rating            INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 4), -- 1 Again .. 4 Easy
    reviewed_at       TEXT NOT NULL,
    elapsed_days      INTEGER NOT NULL,       -- local calendar days since the previous review
    stability_before  REAL,                   -- NULL on the first review
    difficulty_before REAL,
    stability_after   REAL NOT NULL,
    difficulty_after  REAL NOT NULL,
    scheduled_days    REAL NOT NULL,          -- wait until next review (0.007 = 10 minutes)
    duration_ms       INTEGER                 -- think time, optional
);
CREATE INDEX idx_review_logs_card ON review_logs(card_id, reviewed_at);
CREATE INDEX idx_review_logs_time ON review_logs(reviewed_at);

CREATE TABLE writing_pieces (
    id         INTEGER PRIMARY KEY,
    kind       TEXT NOT NULL CHECK (kind IN ('task1', 'task2', 'paragraph')),
    prompt     TEXT NOT NULL DEFAULT '',
    written_on TEXT NOT NULL,                 -- 'YYYY-MM-DD', editable
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE writing_versions (
    id            INTEGER PRIMARY KEY,
    piece_id      INTEGER NOT NULL REFERENCES writing_pieces(id) ON DELETE CASCADE,
    version_no    INTEGER NOT NULL,           -- 1 = first draft
    body          TEXT NOT NULL DEFAULT '',
    word_count    INTEGER NOT NULL DEFAULT 0, -- stored so lists don't re-count
    seconds_spent INTEGER,                    -- from the timer; NULL = timer not used
    feedback      TEXT NOT NULL DEFAULT '',   -- UI arrives in Phase 2
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    UNIQUE (piece_id, version_no)
);
