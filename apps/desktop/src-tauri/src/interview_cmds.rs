//! Interview prep: the story bank, a prep pack per job, practice feedback and the debrief.

use serde::Serialize;
use tauri::State;

use waypoint_ai::interview::{self, DraftStories, Feedback, PrepPack};
use waypoint_ai::{JsonRequest, JsonResponse};
use waypoint_core::{Practice, Story};
use waypoint_docs::factgate;

use crate::AppState;
use crate::commands::{ai_config_and_key, load_job};

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

async fn call(state: &AppState, kind: &str, job_id: Option<i64>, system: &str, user: &str, schema_name: &str, schema: &serde_json::Value) -> CmdResult<JsonResponse> {
    let (cfg, key) = ai_config_and_key(state).await?;
    let req = JsonRequest { system, user, schema_name, schema, max_tokens: 16000 };
    let resp = state.llm.complete_json(&cfg, key.as_deref(), &req).await.map_err(err)?;
    let _ = state.db.record_ai_call(kind, job_id, &resp.model, resp.input_tokens, resp.output_tokens).await;
    Ok(resp)
}

// ── Story bank ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn stories_list(state: State<'_, AppState>) -> CmdResult<Vec<Story>> {
    state.db.list_stories().await.map_err(err)
}

/// Saving a story means the user has reviewed it, so it stops being a draft.
#[tauri::command]
pub async fn story_save(state: State<'_, AppState>, story: Story) -> CmdResult<i64> {
    if story.title.trim().is_empty() {
        return Err("Give the story a short title.".into());
    }
    let story = Story { draft: false, tags: story.tags.into_iter().map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect(), ..story };
    state.db.save_story(&story).await.map_err(err)
}

#[tauri::command]
pub async fn story_delete(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state.db.delete_story(id).await.map_err(err)
}

#[derive(Serialize)]
pub struct Suggested {
    added: usize,
    /// Stories with a number that isn't in the profile, to double-check.
    flagged: Vec<String>,
}

/// Draft STAR stories from the profile. They're saved as drafts with a list of what to
/// add from memory, and stay drafts until the user saves them.
#[tauri::command]
pub async fn stories_suggest(state: State<'_, AppState>) -> CmdResult<Suggested> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    if profile.experience.iter().all(|e| e.highlights.is_empty()) {
        return Err("Add some highlights to your experience in Profile first. Stories are built from them.".into());
    }
    let existing = state.db.list_stories().await.map_err(err)?;
    let (system, user) = interview::stories_prompt(&profile, &existing);
    let resp = call(&state, "stories_suggest", None, &system, &user, "interview_stories", &interview::stories_schema()).await?;
    let drafts: DraftStories = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected stories shape: {e}"))?;
    let mut flagged = Vec::new();
    let mut added = 0;
    for d in drafts.stories {
        let text = [&d.situation, &d.task, &d.action, &d.result].map(|s| s.as_str()).join(" ");
        let bad = factgate::unsupported_numbers(&text, &profile);
        if !bad.is_empty() {
            flagged.push(format!("{} ({})", d.title, bad.join(", ")));
        }
        let source = profile.experience.get(d.role_index).map(|e| format!("{}, {}", e.title, e.company)).unwrap_or_default();
        let mut notes = Vec::new();
        if !d.missing.is_empty() {
            notes.push(format!("To add from memory: {}", d.missing.join("; ")));
        }
        if !bad.is_empty() {
            notes.push(format!("Check: {} isn't in your profile.", bad.join(", ")));
        }
        let story = Story { title: d.title, source, situation: d.situation, task: d.task, action: d.action, result: d.result, reflection: notes.join("\n"), tags: d.tags, draft: true, ..Default::default() };
        state.db.save_story(&story).await.map_err(err)?;
        added += 1;
    }
    Ok(Suggested { added, flagged })
}

// ── Prep pack, practice, debrief ────────────────────────────────────────

#[derive(Serialize)]
pub struct PrepState {
    pack: Option<PrepPack>,
    done: Vec<String>,
    debrief: String,
    model: String,
    updated_at: Option<String>,
    practice: Vec<Practice>,
}

async fn prep_state(state: &AppState, job_id: i64) -> CmdResult<PrepState> {
    let practice = state.db.list_practice(job_id).await.map_err(err)?;
    Ok(match state.db.get_prep(job_id).await.map_err(err)? {
        Some(p) => PrepState { pack: serde_json::from_value(p.pack).ok(), done: p.done, debrief: p.debrief, model: p.model, updated_at: Some(p.updated_at.to_rfc3339()), practice },
        None => PrepState { pack: None, done: vec![], debrief: String::new(), model: String::new(), updated_at: None, practice },
    })
}

#[tauri::command]
pub async fn prep_get(state: State<'_, AppState>, job_id: i64) -> CmdResult<PrepState> {
    prep_state(&state, job_id).await
}

#[tauri::command]
pub async fn prep_build(state: State<'_, AppState>, job_id: i64) -> CmdResult<PrepState> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    if profile.experience.is_empty() {
        return Err("Add your experience to your profile first.".into());
    }
    let job = load_job(&state, job_id).await?;
    if job.description.trim().len() < 80 {
        return Err("This posting has no description to prepare from.".into());
    }
    let stories: Vec<Story> = state.db.list_stories().await.map_err(err)?.into_iter().filter(|s| !s.draft).collect();
    let evaluation = state.db.latest_evaluation(job_id).await.map_err(err)?.map(|e| e.payload);
    let (system, user) = interview::pack_prompt(&profile, &job, &stories, evaluation.as_ref());
    let resp = call(&state, "prep_pack", Some(job_id), &system, &user, "interview_prep_pack", &interview::pack_schema()).await?;
    let mut pack: PrepPack = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected prep pack shape: {e}"))?;
    pack.keep_known_stories(&stories);
    state.db.save_prep_pack(job_id, &serde_json::to_value(&pack).map_err(err)?, &resp.model).await.map_err(err)?;
    prep_state(&state, job_id).await
}

#[tauri::command]
pub async fn prep_notes(state: State<'_, AppState>, job_id: i64, done: Vec<String>, debrief: String) -> CmdResult<()> {
    state.db.set_prep_notes(job_id, &done, &debrief).await.map_err(err)
}

#[tauri::command]
pub async fn practice_answer(state: State<'_, AppState>, job_id: i64, question: String, answer: String) -> CmdResult<Practice> {
    if answer.split_whitespace().count() < 15 {
        return Err("Write your answer the way you'd say it, at least a few sentences.".into());
    }
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    let job = load_job(&state, job_id).await?;
    let stories: Vec<Story> = state.db.list_stories().await.map_err(err)?.into_iter().filter(|s| !s.draft).collect();
    let (system, user) = interview::practice_prompt(&profile, &job, &stories, &question, &answer);
    let resp = call(&state, "practice_feedback", Some(job_id), &system, &user, "practice_feedback", &interview::practice_schema()).await?;
    let feedback: Feedback = serde_json::from_value(resp.value).map_err(|e| format!("Unexpected feedback shape: {e}"))?;
    let value = serde_json::to_value(&feedback).map_err(err)?;
    let id = state.db.save_practice(job_id, &question, &answer, &value).await.map_err(err)?;
    Ok(Practice { id, job_id, question, answer, feedback: value, created_at: chrono::Utc::now() })
}
