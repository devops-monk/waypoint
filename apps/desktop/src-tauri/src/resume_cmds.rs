//! Resume Studio commands. Typst layout is CPU work, so rendering runs off the async runtime.

use serde::{Deserialize, Serialize};
use tauri::State;

use waypoint_ai::prompts;
use waypoint_docs::ats::{self, AtsReport};
use waypoint_docs::factgate::{self, Issue, Severity};
use waypoint_docs::tailor::{self, Proposal, TailorPlan};
use waypoint_docs::{ResumeDoc, Template, render};

use crate::AppState;
use crate::commands::ai_config_and_key;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn template_id(t: Template) -> &'static str {
    match t {
        Template::Clean => "clean",
        Template::Modern => "modern",
    }
}

#[derive(Serialize)]
pub struct ResumeState {
    doc: ResumeDoc,
    template: Template,
    /// When this version was last saved; `None` for a fresh copy of the profile.
    saved_at: Option<String>,
    exported_to: Option<String>,
}

/// The resume to edit: the latest saved version for this job, else the saved base resume,
/// else a fresh copy of the profile.
#[tauri::command]
pub async fn resume_load(state: State<'_, AppState>, job_id: Option<i64>) -> CmdResult<ResumeState> {
    let mut stored = state.db.latest_resume(job_id).await.map_err(err)?;
    if stored.is_none() && job_id.is_some() {
        stored = state.db.latest_resume(None).await.map_err(err)?;
    }
    if let Some(s) = stored.filter(|s| s.job_id == job_id || s.job_id.is_none())
        && let Ok(doc) = serde_json::from_value::<ResumeDoc>(s.data) {
            let template = if s.template == "modern" { Template::Modern } else { Template::Clean };
            let own = s.job_id == job_id;
            return Ok(ResumeState { doc, template, saved_at: own.then(|| s.created_at.to_rfc3339()), exported_to: if own { s.file_path } else { None } });
        }
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    Ok(ResumeState { doc: ResumeDoc::from_profile(&profile), template: Template::Clean, saved_at: None, exported_to: None })
}

#[derive(Serialize)]
pub struct Rendered {
    pages: Vec<String>,
    issues: Vec<Issue>,
}

#[tauri::command]
pub async fn resume_render(state: State<'_, AppState>, doc: ResumeDoc, template: Template) -> CmdResult<Rendered> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    let issues = factgate::check(&doc, &profile);
    let pages = tauri::async_runtime::spawn_blocking(move || render::compile(&doc, template).map(|d| render::svg_pages(&d)))
        .await
        .map_err(err)?
        .map_err(err)?;
    Ok(Rendered { pages, issues })
}

async fn job_keywords(state: &AppState, job_id: Option<i64>) -> Vec<String> {
    let Some(id) = job_id else { return vec![] };
    match state.db.latest_evaluation(id).await {
        Ok(Some(e)) => e.payload["keywords"].as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        _ => vec![],
    }
}

#[tauri::command]
pub async fn resume_tailor(state: State<'_, AppState>, job_id: i64) -> CmdResult<Proposal> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    if profile.experience.is_empty() {
        return Err("Add your experience to your profile first. Tailoring only rearranges what's there.".into());
    }
    let job = state.db.get_job(job_id).await.map_err(err)?;
    if job.description.trim().len() < 80 {
        return Err("This posting has no description to tailor against.".into());
    }
    let keywords = job_keywords(&state, Some(job_id)).await;
    let (cfg, key) = ai_config_and_key(&state).await?;
    let schema = prompts::tailor_schema();
    let (system, user) = prompts::tailor_prompt(&profile, &job, &keywords);
    let req = waypoint_ai::JsonRequest { system: &system, user: &user, schema_name: "resume_tailoring", schema: &schema, max_tokens: 16000 };
    let resp = state.llm.complete_json(&cfg, key.as_deref(), &req).await.map_err(err)?;
    let plan: TailorPlan = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected tailoring shape: {e}"))?;
    // Start from the user's current base resume so contact details and paper size carry over.
    let base = match state.db.latest_resume(None).await.map_err(err)? {
        Some(s) => serde_json::from_value(s.data).unwrap_or_else(|_| ResumeDoc::from_profile(&profile)),
        None => ResumeDoc::from_profile(&profile),
    };
    Ok(tailor::propose(&profile, &plan, &base))
}

#[tauri::command]
pub async fn resume_save(state: State<'_, AppState>, job_id: Option<i64>, doc: ResumeDoc, template: Template) -> CmdResult<i64> {
    state.db.save_resume(job_id, template_id(template), &serde_json::to_value(&doc).map_err(err)?, None).await.map_err(err)
}

#[derive(Deserialize)]
pub struct ExportRequest {
    job_id: Option<i64>,
    doc: ResumeDoc,
    template: Template,
    path: String,
}

#[derive(Serialize)]
pub struct ExportResult {
    path: String,
    ats: AtsReport,
}

/// Write the PDF, save this exact version against the job, and read it back as an ATS would.
/// Refuses while any claim fails the fact gate.
#[tauri::command]
pub async fn resume_export(state: State<'_, AppState>, req: ExportRequest) -> CmdResult<ExportResult> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    let blocked: Vec<String> = factgate::check(&req.doc, &profile).into_iter().filter(|i| i.severity == Severity::Block).map(|i| i.message).collect();
    if !blocked.is_empty() {
        return Err(format!("Fix these before exporting: {}", blocked.join(" ")));
    }
    let doc = req.doc.clone();
    let template = req.template;
    let (pdf, pages) = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<(Vec<u8>, usize)> {
        let d = render::compile(&doc, template)?;
        Ok((render::pdf(&d)?, d.pages().len()))
    })
    .await
    .map_err(err)?
    .map_err(err)?;
    std::fs::write(&req.path, &pdf).map_err(|e| format!("Couldn't write {}: {e}", req.path))?;
    let keywords = job_keywords(&state, req.job_id).await;
    let report = ats::check_pdf(&pdf, pages, &req.doc, &keywords);
    state.db.save_resume(req.job_id, template_id(req.template), &serde_json::to_value(&req.doc).map_err(err)?, Some(&req.path)).await.map_err(err)?;
    Ok(ExportResult { path: req.path, ats: report })
}

/// The ATS read-back without writing a file, for the live check panel.
#[tauri::command]
pub async fn resume_ats(state: State<'_, AppState>, job_id: Option<i64>, doc: ResumeDoc, template: Template) -> CmdResult<AtsReport> {
    let keywords = job_keywords(&state, job_id).await;
    let d2 = doc.clone();
    let (pdf, pages) = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<(Vec<u8>, usize)> {
        let d = render::compile(&d2, template)?;
        Ok((render::pdf(&d)?, d.pages().len()))
    })
    .await
    .map_err(err)?
    .map_err(err)?;
    Ok(ats::check_pdf(&pdf, pages, &doc, &keywords))
}
