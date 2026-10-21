-- Interview prep: a bank of STAR stories reused across interviews, a prep pack per job,
-- and practice answers with feedback.
CREATE TABLE story (
    id          INTEGER PRIMARY KEY,
    title       TEXT NOT NULL,
    source      TEXT NOT NULL DEFAULT '',   -- role it comes from, e.g. "Senior Engineer, Acme"
    situation   TEXT NOT NULL DEFAULT '',
    task        TEXT NOT NULL DEFAULT '',
    action      TEXT NOT NULL DEFAULT '',
    result      TEXT NOT NULL DEFAULT '',
    reflection  TEXT NOT NULL DEFAULT '',
    tags        TEXT NOT NULL DEFAULT '[]',  -- JSON array of competencies
    draft       INTEGER NOT NULL DEFAULT 0,  -- AI-suggested and not yet reviewed
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE interview_prep (
    job_id      INTEGER PRIMARY KEY REFERENCES job(id) ON DELETE CASCADE,
    pack        TEXT NOT NULL,              -- prep pack JSON
    done        TEXT NOT NULL DEFAULT '[]', -- checklist items ticked off
    debrief     TEXT NOT NULL DEFAULT '',
    model       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE practice (
    id          INTEGER PRIMARY KEY,
    job_id      INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    question    TEXT NOT NULL,
    answer      TEXT NOT NULL,
    feedback    TEXT NOT NULL,              -- feedback JSON
    created_at  TEXT NOT NULL
);
CREATE INDEX practice_job_idx ON practice(job_id, created_at DESC);
