-- Waypoint schema v1. SQLite is the single source of truth; files are import/export only.

CREATE TABLE job (
    id              INTEGER PRIMARY KEY,
    dedup_key       TEXT NOT NULL UNIQUE,
    url             TEXT NOT NULL,
    canonical_url   TEXT NOT NULL,
    source          TEXT NOT NULL,          -- provider id: greenhouse, lever, ashby, remotive, manual
    board           TEXT,                   -- provider-specific board slug
    external_id     TEXT,
    company         TEXT NOT NULL,
    title           TEXT NOT NULL,
    location        TEXT NOT NULL DEFAULT '',
    remote          INTEGER NOT NULL DEFAULT 0,
    employment_type TEXT,
    department      TEXT,
    description     TEXT NOT NULL DEFAULT '',
    salary_min      REAL,
    salary_max      REAL,
    salary_currency TEXT,
    salary_period   TEXT,
    posted_at       TEXT,                   -- RFC 3339
    first_seen_at   TEXT NOT NULL,
    last_seen_at    TEXT NOT NULL,
    liveness        TEXT NOT NULL DEFAULT 'active', -- active | expired | unknown
    status          TEXT NOT NULL DEFAULT 'new',
    status_at       TEXT NOT NULL,
    starred         INTEGER NOT NULL DEFAULT 0,
    notes           TEXT NOT NULL DEFAULT ''
);
CREATE INDEX job_status_idx ON job(status);
CREATE INDEX job_canonical_url_idx ON job(canonical_url);
CREATE INDEX job_company_idx ON job(company);

CREATE TABLE status_event (
    id          INTEGER PRIMARY KEY,
    job_id      INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    from_status TEXT,
    to_status   TEXT NOT NULL,
    at          TEXT NOT NULL,
    note        TEXT NOT NULL DEFAULT ''
);
CREATE INDEX status_event_job_idx ON status_event(job_id);

CREATE TABLE evaluation (
    id              INTEGER PRIMARY KEY,
    job_id          INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    profile_version INTEGER NOT NULL,
    score           REAL,                   -- NULL when a gate failed
    band            TEXT NOT NULL,
    payload         TEXT NOT NULL,          -- full structured evaluation JSON
    model           TEXT NOT NULL,
    input_tokens    INTEGER NOT NULL DEFAULT 0,
    output_tokens   INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL
);
CREATE INDEX evaluation_job_idx ON evaluation(job_id, created_at DESC);

-- Companies the user tracks; each maps to one ATS board.
CREATE TABLE board (
    id          INTEGER PRIMARY KEY,
    provider    TEXT NOT NULL,
    slug        TEXT NOT NULL,
    company     TEXT NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    last_run_at TEXT,
    last_error  TEXT,
    last_count  INTEGER,
    UNIQUE(provider, slug)
);

CREATE TABLE scan_run (
    id          INTEGER PRIMARY KEY,
    started_at  TEXT NOT NULL,
    finished_at TEXT,
    boards      INTEGER NOT NULL DEFAULT 0,
    fetched     INTEGER NOT NULL DEFAULT 0,
    new_jobs    INTEGER NOT NULL DEFAULT 0,
    errors      INTEGER NOT NULL DEFAULT 0
);

-- Versioned candidate profile (structured JSON). The latest row is current.
CREATE TABLE profile (
    version    INTEGER PRIMARY KEY,
    data       TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE setting (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
