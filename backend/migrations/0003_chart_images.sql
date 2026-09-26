-- Phase 3: a chart image for Task 1 pieces, and whether an AI review saw it.
-- Same conventions as 0001: UTC timestamps as text.

-- One image per piece. Stored in the database (not as a file) so backups
-- made with VACUUM INTO include it. The browser shrinks it before upload,
-- so a row is usually 50-500 KB.
CREATE TABLE piece_images (
    piece_id   INTEGER PRIMARY KEY REFERENCES writing_pieces(id) ON DELETE CASCADE,
    mime       TEXT NOT NULL CHECK (mime IN ('image/png', 'image/webp')),
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    data       BLOB NOT NULL,
    created_at TEXT NOT NULL
);

-- 1 when the chart image was sent with the essay (vision model), else 0.
ALTER TABLE ai_feedback ADD COLUMN with_image INTEGER NOT NULL DEFAULT 0;
