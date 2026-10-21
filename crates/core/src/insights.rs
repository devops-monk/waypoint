//! Search insights from the status history: the funnel, who answers, how fast, and whether
//! a high fit score actually predicts a reply. After career-ops `analyze-patterns.mjs` and
//! `funnel-velocity.mjs`. Pure functions over rows, so they're easy to test.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::Serialize;

use crate::status::Status;

pub struct JobRow {
    pub id: i64,
    pub source: String,
    pub status: Status,
    pub score: Option<f64>,
}

pub struct EventRow {
    pub job_id: i64,
    pub to: Status,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Stage {
    pub key: &'static str,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SourceRate {
    pub source: String,
    pub applied: usize,
    pub heard_back: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Week {
    /// Monday of the week.
    pub start: NaiveDate,
    pub applied: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FitOutcome {
    pub heard_back_avg: f64,
    pub heard_back_n: usize,
    pub silent_avg: f64,
    pub silent_n: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Insights {
    pub funnel: Vec<Stage>,
    pub applied: usize,
    pub heard_back: usize,
    pub median_days_to_reply: Option<f64>,
    pub active: usize,
    pub by_source: Vec<SourceRate>,
    pub weeks: Vec<Week>,
    /// Only when both groups have at least three scored roles.
    pub fit: Option<FitOutcome>,
}

/// Any answer counts as hearing back, a rejection included.
fn is_reply(s: Status) -> bool {
    matches!(s, Status::Responded | Status::Assessment | Status::Interview | Status::Offer | Status::Hired | Status::Rejected)
}

fn rank(s: Status) -> u8 {
    match s {
        Status::Saved | Status::Evaluated => 1,
        Status::Applied => 2,
        Status::Responded | Status::Assessment => 3,
        Status::Interview => 4,
        Status::Offer => 5,
        Status::Hired => 6,
        _ => 0,
    }
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    Some(if v.len() & 1 == 0 { (v[m - 1] + v[m]) / 2.0 } else { v[m] })
}

pub fn compute(jobs: &[JobRow], events: &[EventRow], today: NaiveDate, weeks: usize) -> Insights {
    // Furthest stage each job reached, from its history and its current status.
    let mut furthest: BTreeMap<i64, u8> = jobs.iter().map(|j| (j.id, rank(j.status))).collect();
    let mut applied_at: BTreeMap<i64, DateTime<Utc>> = BTreeMap::new();
    let mut first_reply: BTreeMap<i64, DateTime<Utc>> = BTreeMap::new();
    let mut sorted: Vec<&EventRow> = events.iter().collect();
    sorted.sort_by_key(|e| e.at);
    for e in &sorted {
        let r = furthest.entry(e.job_id).or_insert(0);
        *r = (*r).max(rank(e.to));
        if e.to == Status::Applied {
            applied_at.entry(e.job_id).or_insert(e.at);
        } else if is_reply(e.to) && applied_at.contains_key(&e.job_id) {
            first_reply.entry(e.job_id).or_insert(e.at);
        }
    }
    let reached = |min: u8| furthest.values().filter(|r| **r >= min).count();
    // A job that is past Applied now was applied to, even if it skipped the column.
    let applied_ids: Vec<i64> = furthest.iter().filter(|(_, r)| **r >= 2).map(|(id, _)| *id).collect();
    let heard: Vec<i64> = applied_ids.iter().copied().filter(|id| first_reply.contains_key(id) || furthest[id] >= 3 || jobs.iter().any(|j| j.id == *id && j.status == Status::Rejected)).collect();

    let funnel = vec![
        Stage { key: "saved", count: reached(1) },
        Stage { key: "applied", count: applied_ids.len() },
        Stage { key: "heard_back", count: heard.len() },
        Stage { key: "interview", count: reached(4) },
        Stage { key: "offer", count: reached(5) },
        Stage { key: "hired", count: reached(6) },
    ];

    let days: Vec<f64> = first_reply.iter().filter_map(|(id, r)| applied_at.get(id).map(|a| (*r - *a).num_minutes() as f64 / 1440.0)).filter(|d| *d >= 0.0).collect();

    let mut by_source: BTreeMap<&str, SourceRate> = BTreeMap::new();
    for j in jobs.iter().filter(|j| applied_ids.contains(&j.id)) {
        let s = by_source.entry(j.source.as_str()).or_insert(SourceRate { source: j.source.clone(), applied: 0, heard_back: 0 });
        s.applied += 1;
        if heard.contains(&j.id) {
            s.heard_back += 1;
        }
    }
    let mut by_source: Vec<SourceRate> = by_source.into_values().collect();
    by_source.sort_by(|a, b| b.applied.cmp(&a.applied).then(a.source.cmp(&b.source)));

    let this_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let weeks: Vec<Week> = (0..weeks)
        .rev()
        .map(|i| {
            let start = this_monday - Duration::weeks(i as i64);
            let end = start + Duration::weeks(1);
            Week { start, applied: applied_at.values().filter(|a| { let d = a.date_naive(); d >= start && d < end }).count() }
        })
        .collect();

    let scored = |want: bool| -> Vec<f64> { jobs.iter().filter(|j| applied_ids.contains(&j.id) && heard.contains(&j.id) == want).filter_map(|j| j.score).collect() };
    let (yes, no) = (scored(true), scored(false));
    let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let fit = (yes.len() >= 3 && no.len() >= 3).then(|| FitOutcome { heard_back_avg: avg(&yes), heard_back_n: yes.len(), silent_avg: avg(&no), silent_n: no.len() });

    let active = jobs.iter().filter(|j| matches!(j.status, Status::Applied | Status::Responded | Status::Assessment | Status::Interview | Status::Offer)).count();

    Insights { funnel, applied: applied_ids.len(), heard_back: heard.len(), median_days_to_reply: median(days), active, by_source, weeks, fit }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, m, d, 9, 0, 0).unwrap()
    }

    #[test]
    fn funnel_rates_speed_and_fit() {
        let job = |id, source: &str, status, score| JobRow { id, source: source.into(), status, score: Some(score) };
        let jobs = vec![
            job(1, "greenhouse", Status::Interview, 85.0),
            job(2, "greenhouse", Status::Rejected, 80.0),
            job(3, "greenhouse", Status::Applied, 50.0),
            job(4, "ashby", Status::Applied, 55.0),
            job(5, "ashby", Status::Responded, 78.0),
            job(6, "ashby", Status::Applied, 45.0),
            job(7, "remotive", Status::Saved, 60.0),
        ];
        let ev = |job_id, to, t| EventRow { job_id, to, at: t };
        let events = vec![
            ev(1, Status::Applied, at(10, 1)), ev(1, Status::Responded, at(10, 4)), ev(1, Status::Interview, at(10, 9)),
            ev(2, Status::Applied, at(10, 2)), ev(2, Status::Rejected, at(10, 12)),
            ev(3, Status::Applied, at(10, 13)),
            ev(4, Status::Applied, at(10, 14)),
            ev(5, Status::Applied, at(10, 6)), ev(5, Status::Responded, at(10, 8)),
            ev(6, Status::Applied, at(10, 15)),
            ev(7, Status::Saved, at(10, 15)),
        ];
        let i = compute(&jobs, &events, NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(), 4);
        let count = |k: &str| i.funnel.iter().find(|s| s.key == k).unwrap().count;
        assert_eq!((count("saved"), count("applied"), count("heard_back"), count("interview")), (7, 6, 3, 1));
        assert_eq!(i.median_days_to_reply, Some(3.0), "replies after 3, 10 and 2 days");
        assert_eq!(i.by_source[0], SourceRate { source: "ashby".into(), applied: 3, heard_back: 1 });
        assert_eq!(i.weeks.iter().map(|w| w.applied).collect::<Vec<_>>(), vec![2, 1, 3, 0], "weeks from 28 Sep to 19 Oct");
        let fit = i.fit.unwrap();
        assert_eq!((fit.heard_back_n, fit.silent_n), (3, 3));
        assert!(fit.heard_back_avg > fit.silent_avg);
        assert_eq!(i.active, 5);
    }

    #[test]
    fn empty_history_is_all_zero() {
        let i = compute(&[], &[], NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(), 12);
        assert_eq!((i.applied, i.median_days_to_reply, i.fit, i.weeks.len()), (0, None, None, 12));
    }
}
