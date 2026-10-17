//! Fit scoring. Weights and bands are ported from ai-job-search `tools/rank_state.py` /
//! `04-job-evaluation.md` (MIT). The LLM scores each dimension; the arithmetic, gates and
//! band are computed here so the headline number is deterministic and auditable.

use serde::{Deserialize, Serialize};

pub const W_TECHNICAL: f64 = 0.30;
pub const W_EXPERIENCE: f64 = 0.25;
pub const W_BEHAVIORAL: f64 = 0.15;
pub const W_CAREER: f64 = 0.30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Gate {
    Pass,
    Flag,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Band {
    #[serde(rename = "Strong Fit")]
    Strong,
    #[serde(rename = "Good Fit")]
    Good,
    #[serde(rename = "Moderate Fit")]
    Moderate,
    #[serde(rename = "Weak Fit")]
    Weak,
    #[serde(rename = "Poor Fit")]
    Poor,
    /// A hard gate failed; the role is not scored.
    #[serde(rename = "Ineligible")]
    Ineligible,
}

impl Band {
    pub fn from_score(score: f64) -> Band {
        match score {
            s if s >= 75.0 => Band::Strong,
            s if s >= 60.0 => Band::Good,
            s if s >= 45.0 => Band::Moderate,
            s if s >= 30.0 => Band::Weak,
            _ => Band::Poor,
        }
    }
}

/// Per-dimension scores, each 0-100.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dimensions {
    pub technical: f64,
    pub experience: f64,
    pub behavioral: f64,
    pub career: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Gates {
    pub eligibility: Gate,
    pub language: Gate,
    pub location: Gate,
}

impl Gates {
    pub fn any_fail(&self) -> bool {
        [self.eligibility, self.language, self.location].contains(&Gate::Fail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FitScore {
    /// Weighted 0-100, rounded to one decimal; `None` when a gate failed.
    pub score: Option<f64>,
    pub band: Band,
}

pub fn weighted(d: &Dimensions) -> f64 {
    let clamp = |v: f64| v.clamp(0.0, 100.0);
    let raw = clamp(d.technical) * W_TECHNICAL + clamp(d.experience) * W_EXPERIENCE + clamp(d.behavioral) * W_BEHAVIORAL + clamp(d.career) * W_CAREER;
    (raw * 10.0).round() / 10.0
}

pub fn fit(d: &Dimensions, g: &Gates) -> FitScore {
    if g.any_fail() {
        return FitScore { score: None, band: Band::Ineligible };
    }
    let score = weighted(d);
    FitScore { score: Some(score), band: Band::from_score(score) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASS: Gates = Gates { eligibility: Gate::Pass, language: Gate::Pass, location: Gate::Pass };

    #[test]
    fn weights_sum_to_one() {
        assert!((W_TECHNICAL + W_EXPERIENCE + W_BEHAVIORAL + W_CAREER - 1.0).abs() < 1e-9);
    }

    #[test]
    fn weighted_score_and_bands() {
        let d = Dimensions { technical: 80.0, experience: 70.0, behavioral: 60.0, career: 90.0 };
        // 24 + 17.5 + 9 + 27
        assert_eq!(weighted(&d), 77.5);
        assert_eq!(fit(&d, &PASS).band, Band::Strong);
        assert_eq!(Band::from_score(60.0), Band::Good);
        assert_eq!(Band::from_score(44.9), Band::Weak);
        assert_eq!(Band::from_score(10.0), Band::Poor);
    }

    #[test]
    fn flag_scores_but_fail_vetoes() {
        let d = Dimensions { technical: 90.0, experience: 90.0, behavioral: 90.0, career: 90.0 };
        let flagged = Gates { language: Gate::Flag, ..PASS };
        assert_eq!(fit(&d, &flagged).score, Some(90.0));
        let failed = Gates { location: Gate::Fail, ..PASS };
        assert_eq!(fit(&d, &failed), FitScore { score: None, band: Band::Ineligible });
    }

    #[test]
    fn out_of_range_inputs_are_clamped() {
        let d = Dimensions { technical: 150.0, experience: -5.0, behavioral: 100.0, career: 100.0 };
        assert_eq!(weighted(&d), 30.0 + 0.0 + 15.0 + 30.0);
    }
}
