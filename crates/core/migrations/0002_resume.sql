-- Saved resumes: the base resume (job_id NULL) and one tailored version per export.
CREATE TABLE resume_version (
    id          INTEGER PRIMARY KEY,
    job_id      INTEGER REFERENCES job(id) ON DELETE SET NULL,
    template    TEXT NOT NULL,
    data        TEXT NOT NULL,          -- ResumeDoc JSON
    file_path   TEXT,                   -- where the PDF was exported, if it was
    created_at  TEXT NOT NULL
);
CREATE INDEX resume_version_job_idx ON resume_version(job_id, created_at DESC);
