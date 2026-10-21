//! People you know: import LinkedIn's Connections.csv, find who works where you're
//! applying, and draft a referral or advice message.

use serde::Serialize;
use tauri::State;

use waypoint_ai::followup::Message;
use waypoint_ai::{JsonRequest, outreach};
use waypoint_core::contacts::{self, Contact, Match, company_tokens, match_company};

use crate::AppState;
use crate::commands::{ai_config_and_key, load_job};

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
pub struct Imported {
    added: usize,
    updated: usize,
    without_company: usize,
}

#[tauri::command]
pub async fn contacts_import(state: State<'_, AppState>, path: String) -> CmdResult<Imported> {
    let meta = std::fs::metadata(&path).map_err(|e| format!("Couldn't open {path}: {e}"))?;
    if meta.len() > 50 * 1024 * 1024 {
        return Err("That file is too large to be a connections export.".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Couldn't read {path}: {e}"))?;
    let parsed = contacts::parse_connections(&text)?;
    let (added, updated) = state.db.import_contacts(&parsed.contacts).await.map_err(err)?;
    Ok(Imported { added, updated, without_company: parsed.without_company })
}

#[tauri::command]
pub async fn contacts_list(state: State<'_, AppState>) -> CmdResult<Vec<Contact>> {
    state.db.list_contacts().await.map_err(err)
}

#[tauri::command]
pub async fn contacts_clear(state: State<'_, AppState>) -> CmdResult<u64> {
    state.db.clear_contacts().await.map_err(err)
}

#[derive(Serialize)]
pub struct ContactMatch {
    contact: Contact,
    strength: Match,
}

/// Connections whose current company matches the job's company, best matches first.
#[tauri::command]
pub async fn contacts_for_job(state: State<'_, AppState>, job_id: i64) -> CmdResult<Vec<ContactMatch>> {
    let job = state.db.get_job(job_id).await.map_err(err)?;
    let target = company_tokens(&job.company);
    let mut out: Vec<ContactMatch> = state
        .db
        .list_contacts()
        .await
        .map_err(err)?
        .into_iter()
        .filter_map(|c| match_company(&company_tokens(&c.company), &target).map(|strength| ContactMatch { contact: c, strength }))
        .collect();
    out.sort_by(|a, b| a.strength.cmp(&b.strength).then(b.contact.connected_on.cmp(&a.contact.connected_on)));
    out.truncate(25);
    Ok(out)
}

#[tauri::command]
pub async fn contact_message(state: State<'_, AppState>, job_id: i64, contact_id: i64, ask: String, notes: String) -> CmdResult<Message> {
    let ask = if ask == "advice" { "advice" } else { "referral" };
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    if profile.name.trim().is_empty() {
        return Err("Add your profile first so the message can introduce you.".into());
    }
    let job = load_job(&state, job_id).await?;
    let contact = state.db.list_contacts().await.map_err(err)?.into_iter().find(|c| c.id == contact_id).ok_or("That contact no longer exists.")?;
    let (system, user) = outreach::prompt(&profile, &job, &contact, ask, &notes);
    let (cfg, key) = ai_config_and_key(&state).await?;
    let schema = outreach::schema();
    let req = JsonRequest { system: &system, user: &user, schema_name: "outreach_message", schema: &schema, max_tokens: 4000 };
    let resp = state.llm.complete_json(&cfg, key.as_deref(), &req).await.map_err(err)?;
    let _ = state.db.record_ai_call("outreach", Some(job_id), &resp.model, resp.input_tokens, resp.output_tokens).await;
    serde_json::from_value(resp.value).map_err(|e| format!("Unexpected message shape: {e}"))
}
