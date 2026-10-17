//! Commands exposed to the UI. Each returns `Result<T, String>` so errors arrive in
//! JavaScript as readable messages.

use std::path::PathBuf;

use chrono::Utc;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use waypoint_ai::prompts::{self, Evaluation};
use waypoint_ai::{LlmConfig, Provider};
use waypoint_core::{Board, Job, JobDraft, JobFilter, JobSummary, PipelineCounts, Profile, Status, StoredEvaluation};
use waypoint_sources::{DetectedBoard, ProviderInfo};

use crate::AppState;
use crate::secrets;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ── Jobs ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_jobs(state: State<'_, AppState>, filter: JobFilter) -> CmdResult<Vec<JobSummary>> {
    state.db.list_jobs(&filter).await.map_err(err)
}

#[tauri::command]
pub async fn get_job(state: State<'_, AppState>, id: i64) -> CmdResult<Job> {
    state.db.get_job(id).await.map_err(err)
}

#[tauri::command]
pub async fn set_status(state: State<'_, AppState>, id: i64, status: Status, note: Option<String>) -> CmdResult<()> {
    state.db.set_status(id, status, note.as_deref().unwrap_or("")).await.map_err(err)
}

#[tauri::command]
pub async fn set_starred(state: State<'_, AppState>, id: i64, starred: bool) -> CmdResult<()> {
    state.db.set_starred(id, starred).await.map_err(err)
}

#[tauri::command]
pub async fn set_notes(state: State<'_, AppState>, id: i64, notes: String) -> CmdResult<()> {
    state.db.set_notes(id, &notes).await.map_err(err)
}

#[tauri::command]
pub async fn pipeline_counts(state: State<'_, AppState>) -> CmdResult<Vec<PipelineCounts>> {
    state.db.pipeline_counts().await.map_err(err)
}

#[derive(Debug, Deserialize)]
pub struct ManualJob {
    pub url: String,
    pub company: String,
    pub title: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub description: String,
}

/// Save a posting the user found elsewhere (LinkedIn, a referral, an email).
#[tauri::command]
pub async fn add_job_manual(state: State<'_, AppState>, job: ManualJob) -> CmdResult<i64> {
    if job.title.trim().is_empty() || job.company.trim().is_empty() {
        return Err("A job needs at least a title and a company.".into());
    }
    let url = if job.url.trim().is_empty() { format!("manual://{}", Utc::now().timestamp_millis()) } else { job.url.trim().to_string() };
    let draft = JobDraft {
        source: "manual".into(),
        url: url.clone(),
        company: job.company.trim().into(),
        title: job.title.trim().into(),
        remote: waypoint_sources::text::looks_remote(&job.location),
        location: job.location.trim().into(),
        description: job.description.trim().into(),
        ..Default::default()
    };
    state.db.upsert_jobs(&[draft]).await.map_err(err)?;
    let found = state.db.list_jobs(&JobFilter { query: Some(job.title.trim().into()), limit: Some(50), ..Default::default() }).await.map_err(err)?;
    let id = found.iter().find(|j| j.url == url).map(|j| j.id).ok_or("saved job not found")?;
    state.db.set_status(id, Status::Saved, "added manually").await.map_err(err)?;
    Ok(id)
}

// ── Boards & scanning ───────────────────────────────────────────────────

#[tauri::command]
pub fn providers() -> Vec<ProviderInfo> {
    waypoint_sources::PROVIDERS.to_vec()
}

#[derive(Serialize)]
pub struct SeedBoard {
    provider: String,
    slug: String,
    company: String,
}

#[tauri::command]
pub fn seed_boards() -> Vec<SeedBoard> {
    waypoint_sources::seed_boards().into_iter().map(|s| SeedBoard { provider: s.provider, slug: s.slug, company: s.company }).collect()
}

#[tauri::command]
pub async fn list_boards(state: State<'_, AppState>) -> CmdResult<Vec<Board>> {
    state.db.list_boards().await.map_err(err)
}

#[tauri::command]
pub fn detect_board(url: String) -> Option<DetectedBoard> {
    waypoint_sources::detect(&url)
}

#[tauri::command]
pub async fn add_board(state: State<'_, AppState>, provider: String, slug: String, company: String) -> CmdResult<i64> {
    if !waypoint_sources::PROVIDERS.iter().any(|p| p.id == provider) {
        return Err(format!("Unknown source {provider:?}"));
    }
    let company = if company.trim().is_empty() { slug.clone() } else { company.trim().to_string() };
    state.db.add_board(&provider, slug.trim(), &company).await.map_err(err)
}

#[tauri::command]
pub async fn remove_board(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state.db.remove_board(id).await.map_err(err)
}

#[tauri::command]
pub async fn set_board_enabled(state: State<'_, AppState>, id: i64, enabled: bool) -> CmdResult<()> {
    state.db.set_board_enabled(id, enabled).await.map_err(err)
}

#[derive(Clone, Serialize)]
pub struct ScanProgress {
    done: usize,
    total: usize,
    company: String,
    found: usize,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct ScanSummary {
    boards: usize,
    fetched: usize,
    new_jobs: usize,
    errors: usize,
}

/// Scan every enabled board. Hosts are throttled inside `Http`, so boards on different
/// ATS hosts run in parallel while each host still sees at most one request per interval.
#[tauri::command]
pub async fn scan_all(app: AppHandle, state: State<'_, AppState>) -> CmdResult<ScanSummary> {
    let started = Utc::now();
    let boards: Vec<Board> = state.db.list_boards().await.map_err(err)?.into_iter().filter(|b| b.enabled).collect();
    let total = boards.len();
    let http = state.http.clone();
    let mut stream = futures::stream::iter(boards.into_iter().map(|b| {
        let http = http.clone();
        async move {
            let result = waypoint_sources::fetch(&http, &b.provider, &b.slug, &b.company).await;
            (b, result)
        }
    }))
    .buffer_unordered(6);

    let (mut done, mut fetched, mut new_jobs, mut errors) = (0, 0, 0, 0);
    while let Some((board, result)) = stream.next().await {
        done += 1;
        let (found, error) = match result {
            Ok(drafts) => {
                let outcome = state.db.upsert_jobs(&drafts).await.map_err(err)?;
                fetched += drafts.len();
                new_jobs += outcome.inserted;
                let _ = state.db.record_board_run(board.id, Some(drafts.len() as i64), None).await;
                (outcome.inserted, None)
            }
            Err(e) => {
                errors += 1;
                let msg = e.to_string();
                let _ = state.db.record_board_run(board.id, None, Some(&msg)).await;
                (0, Some(msg))
            }
        };
        let _ = app.emit("scan://progress", ScanProgress { done, total, company: board.company.clone(), found, error });
    }
    let _ = state.db.record_scan_run(started, total as i64, fetched as i64, new_jobs as i64, errors as i64).await;
    Ok(ScanSummary { boards: total, fetched, new_jobs, errors })
}

// ── Profile ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_profile(state: State<'_, AppState>) -> CmdResult<Profile> {
    Ok(state.db.current_profile().await.map_err(err)?.1)
}

#[tauri::command]
pub async fn save_profile(state: State<'_, AppState>, profile: Profile) -> CmdResult<i64> {
    state.db.save_profile(&profile).await.map_err(err)
}

fn read_resume_text(path: &PathBuf) -> Result<String, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let text = match ext.as_str() {
        "pdf" => pdf_extract::extract_text(path).map_err(|e| format!("Couldn't read that PDF: {e}"))?,
        "txt" | "md" | "markdown" => std::fs::read_to_string(path).map_err(err)?,
        _ => return Err("Use a PDF, .txt or .md resume.".into()),
    };
    let text = text.trim().to_string();
    if text.len() < 200 {
        return Err("That file has almost no extractable text. If it's a scanned PDF, export a text PDF or paste your resume instead.".into());
    }
    Ok(text)
}

/// Parse a resume into a draft profile with the configured AI. The draft is returned for
/// the user to review; it is not saved until they confirm.
#[tauri::command]
pub async fn import_resume(state: State<'_, AppState>, path: Option<String>, text: Option<String>) -> CmdResult<Profile> {
    let resume = match (path, text) {
        (Some(p), _) => read_resume_text(&PathBuf::from(p))?,
        (None, Some(t)) if t.trim().len() >= 200 => t,
        _ => return Err("Paste at least a few paragraphs of your resume.".into()),
    };
    let (cfg, key) = ai_config_and_key(&state).await?;
    let schema = prompts::profile_schema();
    let (system, user) = prompts::profile_prompt(&resume);
    let req = waypoint_ai::JsonRequest { system: &system, user: &user, schema_name: "candidate_profile", schema: &schema, max_tokens: 16000 };
    let resp = state.llm.complete_json(&cfg, key.as_deref(), &req).await.map_err(err)?;
    let mut extracted: Profile = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected profile shape: {e}"))?;
    // Keep preferences the user already set; the resume never carries them.
    extracted.preferences = state.db.current_profile().await.map_err(err)?.1.preferences;
    Ok(extracted)
}

// ── AI settings ─────────────────────────────────────────────────────────

const AI_CONFIG_KEY: &str = "ai_config";

#[derive(Serialize)]
pub struct AiSettings {
    config: Option<LlmConfig>,
    keys: Vec<(Provider, bool)>,
}

#[tauri::command]
pub async fn get_ai_settings(state: State<'_, AppState>) -> CmdResult<AiSettings> {
    let config = state.db.get_setting(AI_CONFIG_KEY).await.map_err(err)?.and_then(|s| serde_json::from_str(&s).ok());
    let keys = [Provider::Anthropic, Provider::OpenAI, Provider::OpenRouter].into_iter().map(|p| (p, matches!(secrets::get(p), Ok(Some(_))))).collect();
    Ok(AiSettings { config, keys })
}

#[tauri::command]
pub async fn save_ai_settings(state: State<'_, AppState>, config: LlmConfig, api_key: Option<String>) -> CmdResult<()> {
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        secrets::set(config.provider, &key).map_err(|e| format!("Couldn't save the key to the system keychain: {e}"))?;
    }
    state.db.set_setting(AI_CONFIG_KEY, &serde_json::to_string(&config).map_err(err)?).await.map_err(err)
}

#[tauri::command]
pub fn delete_api_key(provider: Provider) -> CmdResult<()> {
    secrets::delete(provider).map_err(err)
}

async fn ai_config_and_key(state: &AppState) -> Result<(LlmConfig, Option<String>), String> {
    let cfg: LlmConfig = state
        .db
        .get_setting(AI_CONFIG_KEY)
        .await
        .map_err(err)?
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or("Choose an AI provider in Settings first.")?;
    let key = if cfg.provider.needs_key() { secrets::get(cfg.provider).map_err(err)? } else { None };
    Ok((cfg, key))
}

// ── Evaluation ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct EvaluationView {
    stored: StoredEvaluation,
    evaluation: Option<Evaluation>,
}

fn view(stored: StoredEvaluation) -> EvaluationView {
    let evaluation = serde_json::from_value(stored.payload.clone()).ok();
    EvaluationView { stored, evaluation }
}

#[tauri::command]
pub async fn latest_evaluation(state: State<'_, AppState>, job_id: i64) -> CmdResult<Option<EvaluationView>> {
    Ok(state.db.latest_evaluation(job_id).await.map_err(err)?.map(view))
}

#[tauri::command]
pub async fn evaluate_job(state: State<'_, AppState>, job_id: i64) -> CmdResult<EvaluationView> {
    let (version, profile) = state.db.current_profile().await.map_err(err)?;
    if profile.is_empty() {
        return Err("Add your profile first: Waypoint only scores jobs against facts you've given it.".into());
    }
    let job = state.db.get_job(job_id).await.map_err(err)?;
    if job.description.trim().len() < 80 {
        return Err("This posting has no description to evaluate. Paste the job text into its notes or re-add it with the description.".into());
    }
    let (cfg, key) = ai_config_and_key(&state).await?;
    let schema = prompts::evaluation_schema();
    let (system, user) = prompts::evaluation_prompt(&profile, &job);
    let resp = state.llm.complete_json(&cfg, key.as_deref(), &prompts::evaluation_request(&system, &user, &schema)).await.map_err(err)?;
    let evaluation: Evaluation = serde_json::from_value(resp.value.clone()).map_err(|e| format!("Unexpected evaluation shape: {e}"))?;
    let fit = evaluation.fit();
    let band = serde_json::to_value(fit.band).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    state.db.save_evaluation(job_id, version, fit.score, &band, &resp.value, &resp.model, resp.input_tokens, resp.output_tokens).await.map_err(err)?;
    if job.status == Status::New || job.status == Status::Saved {
        state.db.set_status(job_id, Status::Evaluated, "").await.map_err(err)?;
    }
    let stored = state.db.latest_evaluation(job_id).await.map_err(err)?.ok_or("evaluation not saved")?;
    Ok(view(stored))
}

#[derive(Serialize)]
pub struct UsageRow {
    model: String,
    calls: i64,
    input_tokens: i64,
    output_tokens: i64,
}

#[tauri::command]
pub async fn token_usage(state: State<'_, AppState>) -> CmdResult<Vec<UsageRow>> {
    Ok(state.db.token_usage().await.map_err(err)?.into_iter().map(|(model, calls, input_tokens, output_tokens)| UsageRow { model, calls, input_tokens, output_tokens }).collect())
}
