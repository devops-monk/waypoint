-- Follow-ups: every touch you log (a nudge, a reply, a thank-you note), plus per-job
-- reminder controls.
CREATE TABLE follow_up (
    id       INTEGER PRIMARY KEY,
    job_id   INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    kind     TEXT NOT NULL,                 -- follow_up | reply | thank_you
    channel  TEXT NOT NULL DEFAULT '',      -- email | linkedin | phone | other
    note     TEXT NOT NULL DEFAULT '',
    at       TEXT NOT NULL
);
CREATE INDEX follow_up_job_idx ON follow_up(job_id, at);

ALTER TABLE job ADD COLUMN follow_up_snooze TEXT;                      -- YYYY-MM-DD
ALTER TABLE job ADD COLUMN follow_up_off INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job ADD COLUMN follow_up_notified TEXT;                    -- last day a notification went out
