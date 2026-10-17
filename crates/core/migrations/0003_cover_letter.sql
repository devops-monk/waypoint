-- Cover letters, one row per saved version, plus the reviewer's notes that produced it.
CREATE TABLE cover_letter (
    id          INTEGER PRIMARY KEY,
    job_id      INTEGER REFERENCES job(id) ON DELETE SET NULL,
    template    TEXT NOT NULL,
    data        TEXT NOT NULL,          -- LetterDoc JSON
    notes       TEXT,                   -- requirement coverage + review JSON, when AI wrote it
    file_path   TEXT,
    created_at  TEXT NOT NULL
);
CREATE INDEX cover_letter_job_idx ON cover_letter(job_id, created_at DESC);

-- Token usage for AI calls other than evaluations (which keep theirs on the evaluation row).
CREATE TABLE ai_call (
    id            INTEGER PRIMARY KEY,
    kind          TEXT NOT NULL,        -- profile_import | tailor | cover_draft | cover_review | cover_revise
    job_id        INTEGER REFERENCES job(id) ON DELETE SET NULL,
    model         TEXT NOT NULL,
    input_tokens  INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    created_at    TEXT NOT NULL
);
