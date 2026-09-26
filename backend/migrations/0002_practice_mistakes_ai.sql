-- Phase 2: practice log, mistake log, AI writing feedback.
-- Same conventions as 0001: UTC timestamps as text, dates as 'YYYY-MM-DD'.

-- Practice outside the app (listening, reading, speaking...).
CREATE TABLE practice_sessions (
    id           INTEGER PRIMARY KEY,
    practiced_on TEXT NOT NULL,                -- 'YYYY-MM-DD', local date
    skill        TEXT NOT NULL CHECK (skill IN ('listening', 'reading', 'speaking', 'other')),
    minutes      INTEGER NOT NULL CHECK (minutes > 0),
    note         TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL
);
CREATE INDEX idx_practice_day ON practice_sessions(practiced_on);

-- Recurring mistakes, e.g. "missing plural -s".
CREATE TABLE mistake_tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

-- How often each mistake appears in a piece. Tagged on the piece (not a
-- version): the first draft shows real habits, rewrites hide them.
CREATE TABLE piece_mistakes (
    piece_id INTEGER NOT NULL REFERENCES writing_pieces(id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES mistake_tags(id) ON DELETE CASCADE,
    count    INTEGER NOT NULL DEFAULT 1 CHECK (count > 0),
    PRIMARY KEY (piece_id, tag_id)
);

-- Every call to the AI reviewer. result_json is NULL when the model's reply
-- could not be used; the row still counts towards the daily token quota.
CREATE TABLE ai_feedback (
    id            INTEGER PRIMARY KEY,
    version_id    INTEGER NOT NULL REFERENCES writing_versions(id) ON DELETE CASCADE,
    model         TEXT NOT NULL,
    result_json   TEXT,
    input_tokens  INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    created_at    TEXT NOT NULL
);
CREATE INDEX idx_ai_feedback_version ON ai_feedback(version_id, created_at);
CREATE INDEX idx_ai_feedback_usage ON ai_feedback(model, created_at);
