//! When to follow up on an application. Port of career-ops `followup-cadence.mjs`:
//!
//! - Applied: nudge after 7 days, again 7 days after that, then stop after 2 (the role has
//!   gone cold; chasing further rarely helps).
//! - Responded or assessment: reply within a day, then keep in touch every 3 days.
//! - Interview: a thank-you note the next day, then every 3 days.
//!
//! The clock starts when the job entered its current status, and only touches logged
//! since then count. A snooze date overrides the schedule (and can revive a cold role);
//! turning reminders off for a job overrides everything.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::status::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cadence {
    pub applied_first: i64,
    pub applied_subsequent: i64,
    pub applied_max: usize,
    pub responded_initial: i64,
    pub responded_subsequent: i64,
    pub interview_thankyou: i64,
}

impl Default for Cadence {
    fn default() -> Self {
        Cadence { applied_first: 7, applied_subsequent: 7, applied_max: 2, responded_initial: 1, responded_subsequent: 3, interview_thankyou: 1 }
    }
}

impl Cadence {
    /// Keep user-entered values in a sane range.
    pub fn clamped(self) -> Self {
        let d = |v: i64| v.clamp(1, 60);
        Cadence {
            applied_first: d(self.applied_first),
            applied_subsequent: d(self.applied_subsequent),
            applied_max: self.applied_max.clamp(1, 5),
            responded_initial: d(self.responded_initial),
            responded_subsequent: d(self.responded_subsequent),
            interview_thankyou: d(self.interview_thankyou),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    /// They wrote to you and haven't had a reply within a day.
    Urgent,
    Due,
    Waiting,
    /// The applied cadence ran out with no answer.
    Cold,
    /// Reminders turned off for this job.
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Check in on an application.
    FollowUp,
    /// Answer their message.
    Reply,
    /// Thank them after an interview.
    ThankYou,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub urgency: Urgency,
    pub action: Action,
    /// When the next touch is due; `None` once cold or off.
    pub next_on: Option<NaiveDate>,
    /// Touches logged since the job entered its status.
    pub touches: usize,
    pub last_touch: Option<DateTime<Utc>>,
}

/// Statuses that get reminders. Saved roles haven't been applied to; offers and closed
/// roles have their own rhythm.
pub fn tracks(status: Status) -> bool {
    matches!(status, Status::Applied | Status::Responded | Status::Assessment | Status::Interview)
}

pub struct Input<'a> {
    pub status: Status,
    pub status_at: DateTime<Utc>,
    /// Every logged touch for the job, any order.
    pub touches: &'a [DateTime<Utc>],
    pub snooze_until: Option<NaiveDate>,
    pub off: bool,
}

pub fn plan(input: &Input, cadence: &Cadence, today: NaiveDate) -> Option<Plan> {
    if !tracks(input.status) {
        return None;
    }
    let since = input.status_at.date_naive();
    let mut touches: Vec<DateTime<Utc>> = input.touches.iter().copied().filter(|t| *t >= input.status_at).collect();
    touches.sort();
    let count = touches.len();
    let last = touches.last().copied();
    let days = |n: i64| Duration::days(n);
    let action = match input.status {
        Status::Applied => Action::FollowUp,
        Status::Interview if count == 0 => Action::ThankYou,
        Status::Interview => Action::FollowUp,
        _ if count == 0 => Action::Reply,
        _ => Action::FollowUp,
    };

    let next_on = match input.status {
        Status::Applied if count >= cadence.applied_max => None,
        Status::Applied => Some(match last {
            None => since + days(cadence.applied_first),
            Some(l) => l.date_naive() + days(cadence.applied_subsequent),
        }),
        Status::Interview => Some(match last {
            None => since + days(cadence.interview_thankyou),
            Some(l) => l.date_naive() + days(cadence.responded_subsequent),
        }),
        _ => Some(match last {
            None => since + days(cadence.responded_initial),
            Some(l) => l.date_naive() + days(cadence.responded_subsequent),
        }),
    };

    let mut plan = Plan { urgency: Urgency::Waiting, action, next_on, touches: count, last_touch: last };
    plan.urgency = match next_on {
        None => Urgency::Cold,
        // They wrote and you haven't answered: urgent from day one, not after the deadline.
        Some(_) if action == Action::Reply => Urgency::Urgent,
        Some(d) if d <= today => Urgency::Due,
        Some(_) => Urgency::Waiting,
    };

    // A snooze still in the future pushes the date out; a snooze that has arrived is due,
    // unless a touch has been logged since it was set (handled by the caller clearing it).
    if let Some(s) = input.snooze_until {
        plan.next_on = Some(s);
        plan.urgency = if s <= today { Urgency::Due } else { Urgency::Waiting };
    }
    if input.off {
        plan.urgency = Urgency::Off;
        plan.next_on = None;
    }
    Some(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 10, 0, 0).unwrap()
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn run(status: Status, status_at: DateTime<Utc>, touches: &[DateTime<Utc>], today: NaiveDate) -> Plan {
        plan(&Input { status, status_at, touches, snooze_until: None, off: false }, &Cadence::default(), today).unwrap()
    }

    #[test]
    fn applied_nudges_twice_then_goes_cold() {
        let applied = at(2026, 10, 1);
        let p = run(Status::Applied, applied, &[], day(2026, 10, 5));
        assert_eq!((p.urgency, p.next_on), (Urgency::Waiting, Some(day(2026, 10, 8))));
        let p = run(Status::Applied, applied, &[], day(2026, 10, 8));
        assert_eq!(p.urgency, Urgency::Due);
        let p = run(Status::Applied, applied, &[at(2026, 10, 8)], day(2026, 10, 10));
        assert_eq!((p.urgency, p.next_on, p.touches), (Urgency::Waiting, Some(day(2026, 10, 15)), 1));
        let p = run(Status::Applied, applied, &[at(2026, 10, 8), at(2026, 10, 15)], day(2026, 10, 30));
        assert_eq!((p.urgency, p.next_on), (Urgency::Cold, None));
    }

    #[test]
    fn replies_are_urgent_and_interviews_get_a_thank_you() {
        let p = run(Status::Responded, at(2026, 10, 10), &[], day(2026, 10, 10));
        assert_eq!((p.urgency, p.action), (Urgency::Urgent, Action::Reply));
        let p = run(Status::Responded, at(2026, 10, 10), &[at(2026, 10, 10)], day(2026, 10, 12));
        assert_eq!((p.urgency, p.action, p.next_on), (Urgency::Waiting, Action::FollowUp, Some(day(2026, 10, 13))));
        let p = run(Status::Interview, at(2026, 10, 10), &[], day(2026, 10, 11));
        assert_eq!((p.urgency, p.action), (Urgency::Due, Action::ThankYou));
    }

    #[test]
    fn touches_before_the_current_status_dont_count() {
        // Two nudges while applied, then they replied: the responded clock starts fresh.
        let p = run(Status::Responded, at(2026, 10, 20), &[at(2026, 10, 8), at(2026, 10, 15)], day(2026, 10, 20));
        assert_eq!((p.touches, p.action), (0, Action::Reply));
    }

    #[test]
    fn snooze_and_off_override_the_schedule() {
        let input = Input { status: Status::Applied, status_at: at(2026, 9, 1), touches: &[at(2026, 9, 8), at(2026, 9, 15)], snooze_until: Some(day(2026, 10, 25)), off: false };
        let p = plan(&input, &Cadence::default(), day(2026, 10, 20)).unwrap();
        assert_eq!((p.urgency, p.next_on), (Urgency::Waiting, Some(day(2026, 10, 25))), "a snooze revives a cold role");
        let p = plan(&Input { off: true, ..input }, &Cadence::default(), day(2026, 10, 26)).unwrap();
        assert_eq!(p.urgency, Urgency::Off);
        assert!(plan(&Input { status: Status::Saved, status_at: at(2026, 9, 1), touches: &[], snooze_until: None, off: false }, &Cadence::default(), day(2026, 10, 1)).is_none());
    }
}
