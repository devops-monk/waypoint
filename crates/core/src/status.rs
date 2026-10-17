//! Application lifecycle states. The set and aliases follow career-ops
//! `templates/states.yml` (MIT); the order of the non-terminal variants is the pipeline order.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Found by a scan, not yet triaged.
    New,
    Saved,
    Evaluated,
    Applied,
    Responded,
    Assessment,
    Interview,
    Offer,
    Hired,
    Rejected,
    Discarded,
    Skip,
}

impl Status {
    pub const ALL: [Status; 12] = [
        Status::New,
        Status::Saved,
        Status::Evaluated,
        Status::Applied,
        Status::Responded,
        Status::Assessment,
        Status::Interview,
        Status::Offer,
        Status::Hired,
        Status::Rejected,
        Status::Discarded,
        Status::Skip,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Status::New => "new",
            Status::Saved => "saved",
            Status::Evaluated => "evaluated",
            Status::Applied => "applied",
            Status::Responded => "responded",
            Status::Assessment => "assessment",
            Status::Interview => "interview",
            Status::Offer => "offer",
            Status::Hired => "hired",
            Status::Rejected => "rejected",
            Status::Discarded => "discarded",
            Status::Skip => "skip",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Status::Offer | Status::Hired | Status::Rejected | Status::Discarded | Status::Skip)
    }

    /// Parse a canonical id or any alias from states.yml, case-insensitively.
    pub fn parse(raw: &str) -> Option<Status> {
        let s = raw.trim().to_lowercase();
        let found = match s.as_str() {
            "new" | "inbox" => Status::New,
            "saved" | "bookmarked" | "shortlisted" => Status::Saved,
            "evaluated" | "evaluada" | "condicional" | "hold" | "evaluar" | "verificar" => Status::Evaluated,
            "applied" | "aplicado" | "enviada" | "aplicada" | "sent" => Status::Applied,
            "responded" | "respondido" => Status::Responded,
            "assessment" | "screening" | "online assessment" | "online_assessment" | "online screening" => Status::Assessment,
            "interview" | "entrevista" => Status::Interview,
            "offer" | "oferta" => Status::Offer,
            "hired" | "contratado" | "contratada" | "accepted" | "accept" => Status::Hired,
            "rejected" | "rechazado" | "rechazada" => Status::Rejected,
            "discarded" | "descartado" | "descartada" | "cerrada" | "cancelada" | "withdrawn" => Status::Discarded,
            "skip" | "no_aplicar" | "no aplicar" | "monitor" | "geo blocker" | "geo_blocker" => Status::Skip,
            _ => return None,
        };
        Some(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_aliases() {
        for s in Status::ALL {
            assert_eq!(Status::parse(s.as_str()), Some(s));
        }
        assert_eq!(Status::parse("Entrevista"), Some(Status::Interview));
        assert_eq!(Status::parse("geo blocker"), Some(Status::Skip));
        assert_eq!(Status::parse("whatever"), None);
        assert!(Status::Hired.is_terminal() && !Status::Interview.is_terminal());
    }
}
