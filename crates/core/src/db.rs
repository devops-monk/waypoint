//! SQLite storage. Queries use the runtime API so the crate builds without a live database.

use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

use crate::dedup::{canonical_url, job_key};
use crate::models::*;
use crate::profile::Profile;
use crate::status::Status;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

fn ts(s: Option<String>) -> Option<DateTime<Utc>> {
    s.and_then(|v| DateTime::parse_from_rfc3339(&v).ok()).map(|d| d.with_timezone(&Utc))
}

fn ts_req(s: String) -> DateTime<Utc> {
    ts(Some(s)).unwrap_or_else(Utc::now)
}

fn salary_from(row: &SqliteRow) -> Option<Salary> {
    let min: Option<f64> = row.get("salary_min");
    let max: Option<f64> = row.get("salary_max");
    if min.is_none() && max.is_none() {
        return None;
    }
    Some(Salary { min, max, currency: row.get("salary_currency"), period: row.get("salary_period") })
}

fn status_from(row: &SqliteRow) -> Status {
    Status::parse(row.get::<String, _>("status").as_str()).unwrap_or(Status::New)
}

const LATEST_EVAL: &str = "LEFT JOIN (SELECT e.job_id, e.score, e.band FROM evaluation e \
     WHERE e.id = (SELECT id FROM evaluation WHERE job_id = e.job_id ORDER BY created_at DESC, id DESC LIMIT 1)) ev ON ev.job_id = j.id";

impl Db {
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        Self::connect(opts).await
    }

    pub async fn in_memory() -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        // A single connection: each in-memory connection would otherwise be its own database.
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await?;
        MIGRATOR.run(&pool).await?;
        Ok(Db { pool })
    }

    async fn connect(opts: SqliteConnectOptions) -> Result<Self> {
        let pool = SqlitePoolOptions::new().max_connections(4).connect_with(opts).await?;
        MIGRATOR.run(&pool).await.context("running migrations")?;
        Ok(Db { pool })
    }

    // ── Jobs ────────────────────────────────────────────────────────────

    /// Insert new postings and refresh ones already seen (matched by dedup key or canonical URL).
    /// User-owned fields (status, notes, starred) are never touched by a refresh.
    pub async fn upsert_jobs(&self, drafts: &[JobDraft]) -> Result<UpsertOutcome> {
        let now = Utc::now().to_rfc3339();
        let mut out = UpsertOutcome::default();
        let mut tx = self.pool.begin().await?;
        for d in drafts {
            if d.url.trim().is_empty() || d.title.trim().is_empty() {
                continue;
            }
            let key = job_key(&d.company, &d.title, &d.url);
            let canon = canonical_url(&d.url);
            let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM job WHERE dedup_key = ? OR canonical_url = ? LIMIT 1")
                .bind(&key)
                .bind(&canon)
                .fetch_optional(&mut *tx)
                .await?;
            let (smin, smax, scur, sper) = match &d.salary {
                Some(s) => (s.min, s.max, s.currency.clone(), s.period.clone()),
                None => (None, None, None, None),
            };
            let posted = d.posted_at.map(|t| t.to_rfc3339());
            if let Some(id) = existing {
                sqlx::query(
                    "UPDATE job SET last_seen_at = ?, liveness = 'active', location = ?, remote = ?,
                        description = CASE WHEN ? <> '' THEN ? ELSE description END,
                        salary_min = COALESCE(?, salary_min), salary_max = COALESCE(?, salary_max),
                        salary_currency = COALESCE(?, salary_currency), salary_period = COALESCE(?, salary_period),
                        posted_at = COALESCE(posted_at, ?)
                     WHERE id = ?",
                )
                .bind(&now)
                .bind(&d.location)
                .bind(d.remote)
                .bind(&d.description)
                .bind(&d.description)
                .bind(smin)
                .bind(smax)
                .bind(&scur)
                .bind(&sper)
                .bind(&posted)
                .bind(id)
                .execute(&mut *tx)
                .await?;
                out.updated += 1;
            } else {
                let id: i64 = sqlx::query_scalar(
                    "INSERT INTO job (dedup_key, url, canonical_url, source, board, external_id, company, title, location,
                        remote, employment_type, department, description, salary_min, salary_max, salary_currency,
                        salary_period, posted_at, first_seen_at, last_seen_at, status, status_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'new', ?) RETURNING id",
                )
                .bind(&key)
                .bind(&d.url)
                .bind(&canon)
                .bind(&d.source)
                .bind(&d.board)
                .bind(&d.external_id)
                .bind(&d.company)
                .bind(&d.title)
                .bind(&d.location)
                .bind(d.remote)
                .bind(&d.employment_type)
                .bind(&d.department)
                .bind(&d.description)
                .bind(smin)
                .bind(smax)
                .bind(&scur)
                .bind(&sper)
                .bind(&posted)
                .bind(&now)
                .bind(&now)
                .bind(&now)
                .fetch_one(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO status_event (job_id, from_status, to_status, at) VALUES (?, NULL, 'new', ?)")
                    .bind(id)
                    .bind(&now)
                    .execute(&mut *tx)
                    .await?;
                out.inserted += 1;
            }
        }
        tx.commit().await?;
        Ok(out)
    }

    pub async fn list_jobs(&self, f: &JobFilter) -> Result<Vec<JobSummary>> {
        let mut q: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
            "SELECT j.id, j.company, j.title, j.location, j.remote, j.source, j.url, j.salary_min, j.salary_max,
                    j.salary_currency, j.salary_period, j.posted_at, j.first_seen_at, j.status, j.starred,
                    ev.score, ev.band
             FROM job j {LATEST_EVAL} WHERE 1 = 1"
        ));
        if let Some(statuses) = f.statuses.as_ref().filter(|s| !s.is_empty()) {
            q.push(" AND j.status IN (");
            let mut sep = q.separated(", ");
            for s in statuses {
                sep.push_bind(s.as_str());
            }
            q.push(")");
        }
        if let Some(text) = f.query.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let like = format!("%{}%", text.replace('%', "\\%").replace('_', "\\_"));
            q.push(" AND (j.title LIKE ").push_bind(like.clone()).push(" ESCAPE '\\'");
            q.push(" OR j.company LIKE ").push_bind(like.clone()).push(" ESCAPE '\\'");
            q.push(" OR j.location LIKE ").push_bind(like).push(" ESCAPE '\\')");
        }
        if f.remote_only {
            q.push(" AND j.remote = 1");
        }
        if f.starred_only {
            q.push(" AND j.starred = 1");
        }
        if let Some(min) = f.min_score {
            q.push(" AND ev.score >= ").push_bind(min);
        }
        q.push(" ORDER BY ev.score IS NULL, ev.score DESC, COALESCE(j.posted_at, j.first_seen_at) DESC LIMIT ")
            .push_bind(f.limit.unwrap_or(500));

        let rows = q.build().fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| JobSummary {
                id: r.get("id"),
                company: r.get("company"),
                title: r.get("title"),
                location: r.get("location"),
                remote: r.get("remote"),
                source: r.get("source"),
                url: r.get("url"),
                salary: salary_from(r),
                posted_at: ts(r.get("posted_at")),
                first_seen_at: ts_req(r.get("first_seen_at")),
                status: status_from(r),
                starred: r.get("starred"),
                score: r.get("score"),
                band: r.get("band"),
            })
            .collect())
    }

    pub async fn get_job(&self, id: i64) -> Result<Job> {
        let r = sqlx::query(&format!("SELECT j.*, ev.score, ev.band FROM job j {LATEST_EVAL} WHERE j.id = ?"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("job {id} not found"))?;
        Ok(Job {
            id: r.get("id"),
            dedup_key: r.get("dedup_key"),
            url: r.get("url"),
            source: r.get("source"),
            board: r.get("board"),
            company: r.get("company"),
            title: r.get("title"),
            location: r.get("location"),
            remote: r.get("remote"),
            employment_type: r.get("employment_type"),
            department: r.get("department"),
            description: r.get("description"),
            salary: salary_from(&r),
            posted_at: ts(r.get("posted_at")),
            first_seen_at: ts_req(r.get("first_seen_at")),
            last_seen_at: ts_req(r.get("last_seen_at")),
            liveness: r.get("liveness"),
            status: status_from(&r),
            status_at: ts_req(r.get("status_at")),
            starred: r.get("starred"),
            notes: r.get("notes"),
            score: r.get("score"),
            band: r.get("band"),
        })
    }

    pub async fn set_status(&self, id: i64, to: Status, note: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        let from: Option<String> = sqlx::query_scalar("SELECT status FROM job WHERE id = ?").bind(id).fetch_optional(&mut *tx).await?;
        let from = from.ok_or_else(|| anyhow!("job {id} not found"))?;
        if from == to.as_str() {
            return Ok(());
        }
        sqlx::query("UPDATE job SET status = ?, status_at = ? WHERE id = ?").bind(to.as_str()).bind(&now).bind(id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO status_event (job_id, from_status, to_status, at, note) VALUES (?, ?, ?, ?, ?)")
            .bind(id)
            .bind(&from)
            .bind(to.as_str())
            .bind(&now)
            .bind(note)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn set_starred(&self, id: i64, starred: bool) -> Result<()> {
        sqlx::query("UPDATE job SET starred = ? WHERE id = ?").bind(starred).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    /// Fill in a description fetched on demand (sources that list jobs without one).
    pub async fn set_description(&self, id: i64, description: &str) -> Result<()> {
        sqlx::query("UPDATE job SET description = ? WHERE id = ? AND description = ''").bind(description).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_notes(&self, id: i64, notes: &str) -> Result<()> {
        sqlx::query("UPDATE job SET notes = ? WHERE id = ?").bind(notes).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn pipeline_counts(&self) -> Result<Vec<PipelineCounts>> {
        let rows = sqlx::query("SELECT status, COUNT(*) AS n FROM job GROUP BY status").fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r| PipelineCounts { status: status_from(r), count: r.get("n") }).collect())
    }

    // ── Evaluations ─────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn save_evaluation(&self, job_id: i64, profile_version: i64, score: Option<f64>, band: &str, payload: &serde_json::Value, model: &str, input_tokens: i64, output_tokens: i64) -> Result<i64> {
        let id = sqlx::query_scalar(
            "INSERT INTO evaluation (job_id, profile_version, score, band, payload, model, input_tokens, output_tokens, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(job_id)
        .bind(profile_version)
        .bind(score)
        .bind(band)
        .bind(payload.to_string())
        .bind(model)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(Utc::now().to_rfc3339())
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn latest_evaluation(&self, job_id: i64) -> Result<Option<StoredEvaluation>> {
        let row = sqlx::query("SELECT * FROM evaluation WHERE job_id = ? ORDER BY created_at DESC, id DESC LIMIT 1")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| StoredEvaluation {
            id: r.get("id"),
            job_id: r.get("job_id"),
            score: r.get("score"),
            band: r.get("band"),
            payload: serde_json::from_str(r.get::<String, _>("payload").as_str()).unwrap_or_default(),
            model: r.get("model"),
            input_tokens: r.get("input_tokens"),
            output_tokens: r.get("output_tokens"),
            created_at: ts_req(r.get("created_at")),
        }))
    }

    /// Total tokens spent per model, for the cost-transparency panel.
    pub async fn token_usage(&self) -> Result<Vec<(String, i64, i64, i64)>> {
        let rows = sqlx::query(
            "SELECT model, COUNT(*) AS n, SUM(input_tokens) AS i, SUM(output_tokens) AS o FROM (
                SELECT model, input_tokens, output_tokens FROM evaluation
                UNION ALL SELECT model, input_tokens, output_tokens FROM ai_call
             ) GROUP BY model ORDER BY n DESC",
        )
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(|r| (r.get("model"), r.get("n"), r.get("i"), r.get("o"))).collect())
    }

    // ── Boards ──────────────────────────────────────────────────────────

    pub async fn add_board(&self, provider: &str, slug: &str, company: &str) -> Result<i64> {
        let id = sqlx::query_scalar(
            "INSERT INTO board (provider, slug, company) VALUES (?, ?, ?)
             ON CONFLICT(provider, slug) DO UPDATE SET company = excluded.company, enabled = 1 RETURNING id",
        )
        .bind(provider)
        .bind(slug)
        .bind(company)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn list_boards(&self) -> Result<Vec<Board>> {
        let rows = sqlx::query("SELECT * FROM board ORDER BY company COLLATE NOCASE").fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| Board {
                id: r.get("id"),
                provider: r.get("provider"),
                slug: r.get("slug"),
                company: r.get("company"),
                enabled: r.get("enabled"),
                last_run_at: ts(r.get("last_run_at")),
                last_error: r.get("last_error"),
                last_count: r.get("last_count"),
            })
            .collect())
    }

    pub async fn set_board_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE board SET enabled = ? WHERE id = ?").bind(enabled).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn remove_board(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM board WHERE id = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn record_board_run(&self, id: i64, count: Option<i64>, error: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE board SET last_run_at = ?, last_count = COALESCE(?, last_count), last_error = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(count)
            .bind(error)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn record_scan_run(&self, started: DateTime<Utc>, boards: i64, fetched: i64, new_jobs: i64, errors: i64) -> Result<()> {
        sqlx::query("INSERT INTO scan_run (started_at, finished_at, boards, fetched, new_jobs, errors) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(started.to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .bind(boards)
            .bind(fetched)
            .bind(new_jobs)
            .bind(errors)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Resumes ─────────────────────────────────────────────────────────

    pub async fn save_resume(&self, job_id: Option<i64>, template: &str, data: &serde_json::Value, file_path: Option<&str>) -> Result<i64> {
        let id = sqlx::query_scalar("INSERT INTO resume_version (job_id, template, data, file_path, created_at) VALUES (?, ?, ?, ?, ?) RETURNING id")
            .bind(job_id)
            .bind(template)
            .bind(data.to_string())
            .bind(file_path)
            .bind(Utc::now().to_rfc3339())
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    /// Latest saved resume for a job, or the base resume when `job_id` is `None`.
    pub async fn latest_resume(&self, job_id: Option<i64>) -> Result<Option<StoredResume>> {
        let row = sqlx::query("SELECT * FROM resume_version WHERE job_id IS ? ORDER BY created_at DESC, id DESC LIMIT 1")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| StoredResume {
            id: r.get("id"),
            job_id: r.get("job_id"),
            template: r.get("template"),
            data: serde_json::from_str(r.get::<String, _>("data").as_str()).unwrap_or_default(),
            file_path: r.get("file_path"),
            created_at: ts_req(r.get("created_at")),
        }))
    }

    // ── Cover letters ───────────────────────────────────────────────────

    pub async fn save_cover_letter(&self, job_id: Option<i64>, template: &str, data: &serde_json::Value, notes: Option<&serde_json::Value>, file_path: Option<&str>) -> Result<i64> {
        let id = sqlx::query_scalar("INSERT INTO cover_letter (job_id, template, data, notes, file_path, created_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id")
            .bind(job_id)
            .bind(template)
            .bind(data.to_string())
            .bind(notes.map(|n| n.to_string()))
            .bind(file_path)
            .bind(Utc::now().to_rfc3339())
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn latest_cover_letter(&self, job_id: i64) -> Result<Option<StoredLetter>> {
        let row = sqlx::query("SELECT * FROM cover_letter WHERE job_id = ? ORDER BY created_at DESC, id DESC LIMIT 1")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| StoredLetter {
            id: r.get("id"),
            job_id: r.get("job_id"),
            template: r.get("template"),
            data: serde_json::from_str(r.get::<String, _>("data").as_str()).unwrap_or_default(),
            notes: r.get::<Option<String>, _>("notes").and_then(|n| serde_json::from_str(&n).ok()),
            file_path: r.get("file_path"),
            created_at: ts_req(r.get("created_at")),
        }))
    }

    /// Record token usage for an AI call that isn't an evaluation.
    pub async fn record_ai_call(&self, kind: &str, job_id: Option<i64>, model: &str, input_tokens: i64, output_tokens: i64) -> Result<()> {
        sqlx::query("INSERT INTO ai_call (kind, job_id, model, input_tokens, output_tokens, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(kind)
            .bind(job_id)
            .bind(model)
            .bind(input_tokens)
            .bind(output_tokens)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Profile & settings ──────────────────────────────────────────────

    pub async fn current_profile(&self) -> Result<(i64, Profile)> {
        let row = sqlx::query("SELECT version, data FROM profile ORDER BY version DESC LIMIT 1").fetch_optional(&self.pool).await?;
        Ok(match row {
            Some(r) => (r.get("version"), serde_json::from_str(r.get::<String, _>("data").as_str()).unwrap_or_default()),
            None => (0, Profile::default()),
        })
    }

    /// Saves a new profile version; returns it. A no-op save keeps the current version
    /// so cached evaluations stay valid.
    pub async fn save_profile(&self, profile: &Profile) -> Result<i64> {
        let (version, current) = self.current_profile().await?;
        if version > 0 && &current == profile {
            return Ok(version);
        }
        let next = version + 1;
        sqlx::query("INSERT INTO profile (version, data, created_at) VALUES (?, ?, ?)")
            .bind(next)
            .bind(serde_json::to_string(profile)?)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(next)
    }

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(sqlx::query_scalar("SELECT value FROM setting WHERE key = ?").bind(key).fetch_optional(&self.pool).await?)
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT INTO setting (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(title: &str, url: &str) -> JobDraft {
        JobDraft { source: "greenhouse".into(), url: url.into(), company: "Acme".into(), title: title.into(), location: "Remote".into(), remote: true, ..Default::default() }
    }

    #[tokio::test]
    async fn upsert_dedups_by_key_and_url_and_keeps_user_state() {
        let db = Db::in_memory().await.unwrap();
        let first = db.upsert_jobs(&[draft("Rust Engineer", "https://boards.greenhouse.io/acme/jobs/1"), draft("Designer", "https://boards.greenhouse.io/acme/jobs/2")]).await.unwrap();
        assert_eq!((first.inserted, first.updated), (2, 0));

        let id = db.list_jobs(&JobFilter { query: Some("rust".into()), ..Default::default() }).await.unwrap()[0].id;
        db.set_status(id, Status::Applied, "sent via site").await.unwrap();

        // Same posting reached with tracking params and a retitle: matched by canonical URL.
        let again = db.upsert_jobs(&[draft("Senior Rust Engineer", "https://boards.greenhouse.io/acme/jobs/1?gh_src=x")]).await.unwrap();
        assert_eq!((again.inserted, again.updated), (0, 1));
        assert_eq!(db.get_job(id).await.unwrap().status, Status::Applied);
    }

    #[tokio::test]
    async fn filters_and_latest_evaluation_ordering() {
        let db = Db::in_memory().await.unwrap();
        db.upsert_jobs(&[draft("A", "https://x.test/1"), draft("B", "https://x.test/2")]).await.unwrap();
        let jobs = db.list_jobs(&JobFilter::default()).await.unwrap();
        let b = jobs.iter().find(|j| j.title == "B").unwrap().id;
        db.save_evaluation(b, 1, Some(40.0), "Weak Fit", &serde_json::json!({}), "m", 10, 5).await.unwrap();
        db.save_evaluation(b, 1, Some(80.0), "Strong Fit", &serde_json::json!({}), "m", 10, 5).await.unwrap();

        let ranked = db.list_jobs(&JobFilter::default()).await.unwrap();
        assert_eq!(ranked[0].title, "B");
        assert_eq!(ranked[0].score, Some(80.0));
        assert_eq!(db.list_jobs(&JobFilter { min_score: Some(50.0), ..Default::default() }).await.unwrap().len(), 1);
        let applied = db.list_jobs(&JobFilter { statuses: Some(vec![Status::Applied]), ..Default::default() }).await.unwrap();
        assert!(applied.is_empty());
    }

    #[tokio::test]
    async fn cover_letters_keep_notes_and_usage_counts_every_call() {
        let db = Db::in_memory().await.unwrap();
        db.upsert_jobs(&[draft("A", "https://x.test/1")]).await.unwrap();
        let job = db.list_jobs(&JobFilter::default()).await.unwrap()[0].id;
        assert!(db.latest_cover_letter(job).await.unwrap().is_none());
        db.save_cover_letter(Some(job), "clean", &serde_json::json!({"v": 1}), Some(&serde_json::json!({"verdict": "ready"})), None).await.unwrap();
        db.save_cover_letter(Some(job), "clean", &serde_json::json!({"v": 2}), None, Some("/tmp/l.pdf")).await.unwrap();
        let l = db.latest_cover_letter(job).await.unwrap().unwrap();
        assert_eq!(l.data["v"], 2);
        assert!(l.notes.is_none());
        db.record_ai_call("cover_draft", Some(job), "claude-opus-5-5", 1000, 400).await.unwrap();
        db.record_ai_call("cover_review", Some(job), "claude-opus-5-5", 1200, 300).await.unwrap();
        let usage = db.token_usage().await.unwrap();
        assert_eq!(usage, vec![("claude-opus-5-5".to_string(), 2, 2200, 700)]);
    }

    #[tokio::test]
    async fn resumes_are_kept_per_job_and_base() {
        let db = Db::in_memory().await.unwrap();
        db.upsert_jobs(&[draft("A", "https://x.test/1")]).await.unwrap();
        let job = db.list_jobs(&JobFilter::default()).await.unwrap()[0].id;
        db.save_resume(None, "clean", &serde_json::json!({"v": "base"}), None).await.unwrap();
        db.save_resume(Some(job), "modern", &serde_json::json!({"v": 1}), None).await.unwrap();
        db.save_resume(Some(job), "modern", &serde_json::json!({"v": 2}), Some("/tmp/a.pdf")).await.unwrap();
        assert_eq!(db.latest_resume(None).await.unwrap().unwrap().data["v"], "base");
        let latest = db.latest_resume(Some(job)).await.unwrap().unwrap();
        assert_eq!((latest.data["v"].as_i64(), latest.file_path.as_deref()), (Some(2), Some("/tmp/a.pdf")));
    }

    #[tokio::test]
    async fn profile_versions_only_bump_on_change() {
        let db = Db::in_memory().await.unwrap();
        let mut p = Profile { name: "Ada".into(), ..Default::default() };
        assert_eq!(db.save_profile(&p).await.unwrap(), 1);
        assert_eq!(db.save_profile(&p).await.unwrap(), 1);
        p.skills.push("Rust".into());
        assert_eq!(db.save_profile(&p).await.unwrap(), 2);
        assert_eq!(db.current_profile().await.unwrap().1.skills, vec!["Rust".to_string()]);
    }
}
