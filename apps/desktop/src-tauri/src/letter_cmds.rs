//! Cover letter commands: write (draft, independent review, revision), edit, check, export.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use waypoint_ai::cover::{self, Draft, Requirement, Review};
use waypoint_ai::{JsonRequest, JsonResponse};
use waypoint_core::{Job, Profile};
use waypoint_docs::factgate::{Issue, Severity};
use waypoint_docs::letter;
use waypoint_docs::{LetterDoc, ResumeDoc, Template, render};

use crate::AppState;
use crate::commands::ai_config_and_key;
use crate::resume_cmds::template_id;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn parse_template(s: &str) -> Template {
    if s == "modern" { Template::Modern } else { Template::Clean }
}

/// How the AI got to this letter, kept with every saved version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetterNotes {
    requirements: Vec<Requirement>,
    review: Review,
    changes: Vec<String>,
    declined: Vec<String>,
    written_at: String,
    model: String,
}

#[derive(Serialize)]
pub struct LetterState {
    doc: LetterDoc,
    template: Template,
    notes: Option<LetterNotes>,
    /// `None` until the letter has been written or saved.
    saved_at: Option<String>,
    exported_to: Option<String>,
    /// Set when the fit evaluation says the experience match is weak.
    fit_warning: Option<String>,
}

/// The resume this letter should match: the job's tailored one, else the base, else the profile.
async fn matching_resume(state: &AppState, job_id: i64, profile: &Profile) -> (ResumeDoc, Template) {
    for id in [Some(job_id), None] {
        if let Ok(Some(s)) = state.db.latest_resume(id).await
            && let Ok(doc) = serde_json::from_value::<ResumeDoc>(s.data) {
                return (doc, parse_template(&s.template));
            }
    }
    (ResumeDoc::from_profile(profile), Template::Clean)
}

fn blank(resume: &ResumeDoc, job: &Job) -> LetterDoc {
    let mut l = LetterDoc::with_header(resume, "en");
    l.recipient = ["Hiring team", job.company.as_str(), job.location.as_str()].iter().filter(|s| !s.trim().is_empty()).map(|s| s.to_string()).collect();
    l.salutation = format!("Dear {} hiring team,", job.company);
    l.closing = "Kind regards,".into();
    l
}

async fn fit_warning(state: &AppState, job_id: i64) -> Option<String> {
    let e = state.db.latest_evaluation(job_id).await.ok().flatten()?;
    let score = e.payload["dimensions"]["experience"]["score"].as_f64()?;
    (score < 50.0).then(|| format!("Your experience match for this role scored {score:.0}. The letter will have to bridge a lot, so check that it's worth applying."))
}

#[tauri::command]
pub async fn letter_load(state: State<'_, AppState>, job_id: i64) -> CmdResult<LetterState> {
    let fit_warning = fit_warning(&state, job_id).await;
    if let Some(s) = state.db.latest_cover_letter(job_id).await.map_err(err)?
        && let Ok(doc) = serde_json::from_value::<LetterDoc>(s.data) {
            return Ok(LetterState {
                doc,
                template: parse_template(&s.template),
                notes: s.notes.and_then(|n| serde_json::from_value(n).ok()),
                saved_at: Some(s.created_at.to_rfc3339()),
                exported_to: s.file_path,
                fit_warning,
            });
        }
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    let job = state.db.get_job(job_id).await.map_err(err)?;
    let (resume, template) = matching_resume(&state, job_id, &profile).await;
    Ok(LetterState { doc: blank(&resume, &job), template, notes: None, saved_at: None, exported_to: None, fit_warning })
}

#[derive(Clone, Serialize)]
struct Progress {
    job_id: i64,
    /// drafting | reviewing | revising
    stage: &'static str,
}

async fn call(state: &AppState, job_id: i64, kind: &str, system: &str, user: &str, schema_name: &str, schema: &serde_json::Value) -> CmdResult<JsonResponse> {
    let (cfg, key) = ai_config_and_key(state).await?;
    let req = JsonRequest { system, user, schema_name, schema, max_tokens: 16000 };
    let resp = state.llm.complete_json(&cfg, key.as_deref(), &req).await.map_err(err)?;
    let _ = state.db.record_ai_call(kind, Some(job_id), &resp.model, resp.input_tokens, resp.output_tokens).await;
    Ok(resp)
}

/// Draft, review with a fresh context, revise. The result is saved straight away so a paid
/// run is never lost, with the reviewer's notes alongside it.
#[tauri::command]
pub async fn letter_write(app: AppHandle, state: State<'_, AppState>, job_id: i64) -> CmdResult<LetterState> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    if profile.experience.is_empty() {
        return Err("Add your experience to your profile first. The letter is built only from what's there.".into());
    }
    let job = state.db.get_job(job_id).await.map_err(err)?;
    if job.description.trim().len() < 80 {
        return Err("This posting has no description to write a letter for.".into());
    }
    let evaluation = state.db.latest_evaluation(job_id).await.map_err(err)?.map(|e| e.payload);
    let progress = |stage| {
        let _ = app.emit("letter://progress", Progress { job_id, stage });
    };

    progress("drafting");
    let (system, user) = cover::draft_prompt(&profile, &job, evaluation.as_ref());
    let resp = call(&state, job_id, "cover_draft", &system, &user, "cover_letter", &cover::draft_schema()).await?;
    let draft: Draft = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected draft shape: {e}"))?;

    progress("reviewing");
    let (system, user) = cover::review_prompt(&profile, &job, &draft);
    let resp = call(&state, job_id, "cover_review", &system, &user, "cover_letter_review", &cover::review_schema()).await?;
    let review: Review = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected review shape: {e}"))?;

    progress("revising");
    let (system, user) = cover::revise_prompt(&profile, &job, evaluation.as_ref(), &draft, &review);
    let resp = call(&state, job_id, "cover_revise", &system, &user, "cover_letter_revision", &cover::revision_schema()).await?;
    let model = resp.model.clone();
    let revised: Draft = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected revision shape: {e}"))?;

    // Header, date and paper come from the matching resume; the AI writes only the letter.
    let (resume, resume_template) = matching_resume(&state, job_id, &profile).await;
    let template = match state.db.latest_cover_letter(job_id).await.map_err(err)? {
        Some(s) => parse_template(&s.template),
        None => resume_template,
    };
    let mut doc = LetterDoc::with_header(&resume, &revised.language);
    doc.recipient = if revised.recipient.is_empty() { blank(&resume, &job).recipient } else { revised.recipient };
    doc.subject = revised.subject;
    doc.salutation = revised.salutation;
    doc.opening = revised.opening;
    doc.highlights = revised.highlights;
    doc.body = revised.body;
    doc.closing = if revised.closing.trim().is_empty() { "Kind regards,".into() } else { revised.closing };

    let notes = LetterNotes {
        requirements: if revised.requirements.is_empty() { draft.requirements } else { revised.requirements },
        review,
        changes: revised.changes,
        declined: revised.declined,
        written_at: Utc::now().to_rfc3339(),
        model,
    };
    let notes_json = serde_json::to_value(&notes).map_err(err)?;
    state.db.save_cover_letter(Some(job_id), template_id(template), &serde_json::to_value(&doc).map_err(err)?, Some(&notes_json), None).await.map_err(err)?;
    Ok(LetterState { doc, template, notes: Some(notes), saved_at: Some(Utc::now().to_rfc3339()), exported_to: None, fit_warning: fit_warning(&state, job_id).await })
}

#[derive(Serialize)]
pub struct Rendered {
    pages: Vec<String>,
    issues: Vec<Issue>,
    words: usize,
}

async fn issues_for(state: &AppState, job_id: i64, doc: &LetterDoc) -> CmdResult<Vec<Issue>> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    let job = state.db.get_job(job_id).await.map_err(err)?;
    let posting = format!("{}\n{}\n{}\n{}", job.title, job.company, job.location, job.description);
    Ok(letter::check(doc, &profile, &posting, &job.company))
}

#[tauri::command]
pub async fn letter_render(state: State<'_, AppState>, job_id: i64, doc: LetterDoc, template: Template) -> CmdResult<Rendered> {
    let mut issues = issues_for(&state, job_id, &doc).await?;
    let words = doc.body_words();
    let pages = tauri::async_runtime::spawn_blocking(move || render::compile_letter(&doc, template).map(|d| render::svg_pages(&d)))
        .await
        .map_err(err)?
        .map_err(err)?;
    if pages.len() > 1 {
        issues.push(Issue { severity: Severity::Warn, path: "length".into(), message: format!("It runs to {} pages. A cover letter should fit on one.", pages.len()) });
    }
    Ok(Rendered { pages, issues, words })
}

async fn latest_notes(state: &AppState, job_id: i64) -> Option<serde_json::Value> {
    state.db.latest_cover_letter(job_id).await.ok().flatten().and_then(|s| s.notes)
}

/// Saving keeps the reviewer's notes from the version it was edited from.
#[tauri::command]
pub async fn letter_save(state: State<'_, AppState>, job_id: i64, doc: LetterDoc, template: Template) -> CmdResult<i64> {
    let notes = latest_notes(&state, job_id).await;
    state.db.save_cover_letter(Some(job_id), template_id(template), &serde_json::to_value(&doc).map_err(err)?, notes.as_ref(), None).await.map_err(err)
}

#[derive(Deserialize)]
pub struct ExportRequest {
    job_id: i64,
    doc: LetterDoc,
    template: Template,
    path: String,
}

#[tauri::command]
pub async fn letter_export(state: State<'_, AppState>, req: ExportRequest) -> CmdResult<String> {
    let blocked: Vec<String> = issues_for(&state, req.job_id, &req.doc).await?.into_iter().filter(|i| i.severity == Severity::Block).map(|i| i.message).collect();
    if !blocked.is_empty() {
        return Err(format!("Fix these before exporting: {}", blocked.join(" ")));
    }
    let doc = req.doc.clone();
    let template = req.template;
    let pdf = tauri::async_runtime::spawn_blocking(move || render::compile_letter(&doc, template).and_then(|d| render::pdf(&d)))
        .await
        .map_err(err)?
        .map_err(err)?;
    std::fs::write(&req.path, &pdf).map_err(|e| format!("Couldn't write {}: {e}", req.path))?;
    let notes = latest_notes(&state, req.job_id).await;
    state.db.save_cover_letter(Some(req.job_id), template_id(req.template), &serde_json::to_value(&req.doc).map_err(err)?, notes.as_ref(), Some(&req.path)).await.map_err(err)?;
    Ok(req.path)
}
