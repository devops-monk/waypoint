-- People you know, imported from LinkedIn's Connections.csv export. Other people's data:
-- it stays here and "Remove all" deletes every row.
CREATE TABLE contact (
    id            INTEGER PRIMARY KEY,
    first_name    TEXT NOT NULL,
    last_name     TEXT NOT NULL DEFAULT '',
    company       TEXT NOT NULL DEFAULT '',
    position      TEXT NOT NULL DEFAULT '',
    email         TEXT NOT NULL DEFAULT '',
    url           TEXT NOT NULL DEFAULT '',
    connected_on  TEXT,
    imported_at   TEXT NOT NULL
);
CREATE UNIQUE INDEX contact_url_idx ON contact(url) WHERE url <> '';
