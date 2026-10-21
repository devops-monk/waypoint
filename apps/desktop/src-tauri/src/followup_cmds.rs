//! Follow-up reminders: the schedule, logging touches, snoozing, AI-drafted messages, and a
//! background check that sends a system notification when something falls due.

use std::time::Duration;

use chrono::{Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;

use waypoint_ai::followup::{self, Message};
use waypoint_core::followup::{self as cadence, Action, Cadence, Plan, Urgency};
use waypoint_core::{Status, Touch};

use crate::AppState;
use crate::commands::{ai_config_and_key, load_job};

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

const SETTINGS_KEY: &str = "follow_up_settings";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct FollowUpSettings {
    cadence: Cadence,
    /// Send a system notification when a follow-up falls due.
    notify: bool,
}

impl Default for FollowUpSettings {
    fn default() -> Self {
        FollowUpSettings { cadence: Cadence::default(), notify: true }
    }
}

async fn settings(state: &AppState) -> FollowUpSettings {
    state.db.get_setting(SETTINGS_KEY).await.ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

#[derive(Debug, Clone, Serialize)]
pub struct FollowUpItem {
    job_id: i64,
    company: String,
    title: String,
    location: String,
    url: String,
    status: Status,
    status_at: String,
    days_in_status: i64,
    snoozed: bool,
    #[serde(flatten)]
    plan: Plan,
}

async fn items(state: &AppState, cadence_cfg: &Cadence) -> CmdResult<Vec<FollowUpItem>> {
    let jobs = state.db.follow_up_jobs().await.map_err(err)?;
    let today = today();
    let mut out: Vec<FollowUpItem> = jobs
        .into_iter()
        .filter_map(|j| {
            let plan = cadence::plan(&cadence::Input { status: j.status, status_at: j.status_at, touches: &j.touches, snooze_until: j.snooze_until, off: j.off }, cadence_cfg, today)?;
            Some(FollowUpItem {
                job_id: j.id,
                days_in_status: (today - j.status_at.with_timezone(&Local).date_naive()).num_days().max(0),
                snoozed: j.snooze_until.is_some_and(|s| s > today),
                company: j.company,
                title: j.title,
                location: j.location,
                url: j.url,
                status: j.status,
                status_at: j.status_at.to_rfc3339(),
                plan,
            })
        })
        .collect();
    out.sort_by(|a, b| a.plan.urgency.cmp(&b.plan.urgency).then(a.plan.next_on.cmp(&b.plan.next_on)));
    Ok(out)
}

#[derive(Serialize)]
pub struct FollowUpBoard {
    items: Vec<FollowUpItem>,
    settings: FollowUpSettings,
    today: NaiveDate,
}

#[tauri::command]
pub async fn followups(state: State<'_, AppState>) -> CmdResult<FollowUpBoard> {
    let s = settings(&state).await;
    Ok(FollowUpBoard { items: items(&state, &s.cadence).await?, settings: s, today: today() })
}

#[tauri::command]
pub async fn followup_log(state: State<'_, AppState>, job_id: i64, kind: String, channel: String, note: String) -> CmdResult<i64> {
    if !matches!(kind.as_str(), "follow_up" | "reply" | "thank_you") {
        return Err(format!("Unknown follow-up type {kind:?}"));
    }
    let channel = if matches!(channel.as_str(), "email" | "linkedin" | "phone" | "other") { channel } else { "other".into() };
    state.db.log_touch(job_id, &kind, &channel, note.trim()).await.map_err(err)
}

#[tauri::command]
pub async fn followup_history(state: State<'_, AppState>, job_id: i64) -> CmdResult<Vec<Touch>> {
    state.db.touches(job_id).await.map_err(err)
}

#[tauri::command]
pub async fn followup_delete(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state.db.delete_touch(id).await.map_err(err)
}

/// Push the next reminder out by `days`, or clear the snooze with `None`.
#[tauri::command]
pub async fn followup_snooze(state: State<'_, AppState>, job_id: i64, days: Option<i64>) -> CmdResult<()> {
    let until = days.map(|d| today() + chrono::Duration::days(d.clamp(1, 90)));
    state.db.set_follow_up_snooze(job_id, until).await.map_err(err)
}

#[tauri::command]
pub async fn followup_set_off(state: State<'_, AppState>, job_id: i64, off: bool) -> CmdResult<()> {
    state.db.set_follow_up_off(job_id, off).await.map_err(err)
}

#[tauri::command]
pub async fn save_followup_settings(state: State<'_, AppState>, settings: FollowUpSettings) -> CmdResult<()> {
    let s = FollowUpSettings { cadence: settings.cadence.clamped(), notify: settings.notify };
    state.db.set_setting(SETTINGS_KEY, &serde_json::to_string(&s).map_err(err)?).await.map_err(err)
}

/// Draft the message for the next touch. The user copies and sends it themselves.
#[tauri::command]
pub async fn followup_draft(state: State<'_, AppState>, job_id: i64, action: Action, notes: String) -> CmdResult<Message> {
    let (_, profile) = state.db.current_profile().await.map_err(err)?;
    if profile.name.trim().is_empty() {
        return Err("Add your profile first so the message can sound like you.".into());
    }
    let job = load_job(&state, job_id).await?;
    let history: Vec<String> = state.db.touches(job_id).await.map_err(err)?.into_iter().take(5).map(|t| format!("{} ({}) {}", t.kind.replace('_', " "), t.at.date_naive(), t.note)).collect();
    let action = match action {
        Action::FollowUp => "follow_up",
        Action::Reply => "reply",
        Action::ThankYou => "thank_you",
    };
    // A thank-you note is better for knowing what was discussed: fall back to the debrief.
    let notes = if notes.trim().is_empty() && action == "thank_you" {
        state.db.get_prep(job_id).await.ok().flatten().map(|p| p.debrief).unwrap_or_default()
    } else {
        notes
    };
    let days = (Utc::now() - job.status_at).num_days().max(0);
    let (system, user) = followup::prompt(&followup::Context { profile: &profile, job: &job, action, days_in_status: days, history: &history, notes: &notes });
    let (cfg, key) = ai_config_and_key(&state).await?;
    let schema = followup::schema();
    let req = waypoint_ai::JsonRequest { system: &system, user: &user, schema_name: "follow_up_message", schema: &schema, max_tokens: 4000 };
    let resp = state.llm.complete_json(&cfg, key.as_deref(), &req).await.map_err(err)?;
    let _ = state.db.record_ai_call("follow_up_draft", Some(job_id), &resp.model, resp.input_tokens, resp.output_tokens).await;
    serde_json::from_value(resp.value).map_err(|e| format!("Unexpected message shape: {e}"))
}

fn action_phrase(a: Action) -> &'static str {
    match a {
        Action::FollowUp => "Follow up",
        Action::Reply => "Reply",
        Action::ThankYou => "Send a thank-you note",
    }
}

/// One pass of the background check: notify about follow-ups that became due today and
/// haven't been notified yet. At most one notification per pass.
async fn notify_due(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let s = settings(&state).await;
    if !s.notify {
        return Ok(());
    }
    let today = today();
    let jobs = state.db.follow_up_jobs().await?;
    let notified: std::collections::HashMap<i64, Option<NaiveDate>> = jobs.iter().map(|j| (j.id, j.notified_on)).collect();
    let fresh: Vec<FollowUpItem> = items(&state, &s.cadence)
        .await
        .map_err(anyhow::Error::msg)?
        .into_iter()
        .filter(|i| matches!(i.plan.urgency, Urgency::Urgent | Urgency::Due) && notified.get(&i.job_id).copied().flatten() != Some(today))
        .collect();
    if fresh.is_empty() {
        return Ok(());
    }
    let (title, body) = match fresh.as_slice() {
        [one] => (format!("{} at {}", action_phrase(one.plan.action), one.company), one.title.clone()),
        many => {
            let names: Vec<&str> = many.iter().take(3).map(|i| i.company.as_str()).collect();
            let more = if many.len() > 3 { format!(" and {} more", many.len() - 3) } else { String::new() };
            (format!("{} follow-ups due", many.len()), format!("{}{more}", names.join(", ")))
        }
    };
    app.notification().builder().title(title).body(body).show()?;
    state.db.mark_follow_up_notified(&fresh.iter().map(|i| i.job_id).collect::<Vec<_>>(), today).await?;
    Ok(())
}

/// Check shortly after launch, then every 30 minutes while the app is open.
pub fn start_notifier(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        loop {
            if let Err(e) = notify_due(&app).await {
                tracing::warn!("follow-up notification check failed: {e}");
            }
            tokio::time::sleep(Duration::from_secs(30 * 60)).await;
        }
    });
}
