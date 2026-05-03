//! Transaction model + classifier for the league-wide moves feed.
//!
//! Phase T (transactions hub). Spec: `design/specs/transactions.md`.
//!
//! No I/O lives in this module — the source-side fetcher (ESPN) and the
//! snapshot persistence both live in `icelines-fetch`. This module owns
//! the pure logic: data shape, regex classification, sanitization,
//! trade-group hashing.

pub mod classifier;
pub mod grouping;
pub mod sanitize;
pub mod search;

pub use classifier::{classify, other_rate, CURRENT_CLASSIFIER_VERSION};
pub use grouping::trade_group_id;
pub use sanitize::sanitize as sanitize_description;
pub use search::{description_matches_query, transactions_for_player};

use serde::{Deserialize, Serialize};

use crate::model::TeamAbbr;

/// Earliest season for which ESPN's transactions archive returns data.
/// Verified 2026-04-30 via the historical probe
/// (`cargo run --example probe_espn_seasons`). 21-22 returned 2200 rows,
/// 22-23 returned 2000, 23-24 returned 2200, 24-25 returned 2000,
/// 25-26 returned 1600 (in-progress). Earlier seasons may exist on the
/// ESPN side; the bundle window stops at 21-22 to keep binary size in
/// check.
pub const TRANSACTIONS_EARLIEST_SEASON: &str = "20212022";

/// Persisted transaction shape, post-classification + sanitization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    /// Calendar date YYYY-MM-DD. T.3 is responsible for converting raw
    /// ESPN timestamps (typically `2026-04-29T04:00:00Z`) into NHL
    /// operational TZ (America/New_York) before bucketing.
    pub date: String,

    /// Primary team in canonical NHL form (TBL not TB; SJS not SJ).
    /// `None` for league-wide rows; surfaced as the synthetic `LEAGUE`
    /// bucket in CLI / TUI filters.
    pub team: Option<TeamAbbr>,

    /// Classified kind. `Other` is a real outcome — never bail when a
    /// description doesn't match a known pattern.
    pub kind: TransactionKind,

    /// Sanitized prose. Control chars stripped, whitespace normalized.
    /// Kept verbatim otherwise so we can re-classify on a regex update
    /// without re-fetching from ESPN (see `CURRENT_CLASSIFIER_VERSION`).
    pub description: String,

    /// Stable hash over (date, team, description) for dedup / idempotency.
    pub id: String,

    /// Trade-mirror grouping. Set when this row appears to be one side of
    /// a multi-team move. UI collapses rows sharing a non-None group_id.
    /// See `grouping::trade_group_id`.
    #[serde(default)]
    pub trade_group_id: Option<String>,

    /// Classifier version that produced `kind`. On load, if this is less
    /// than `CURRENT_CLASSIFIER_VERSION`, re-run `classify()` against
    /// `description` so bundled snapshots don't fossilize stale classes.
    pub classifier_version: u16,
}

/// Coarse-grained kind. Free-form prose is classified into one of these
/// for filtering / coloring; the original text is preserved on the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionKind {
    Trade,
    WaiverPlacement,
    WaiverClear,
    WaiverClaim,
    Signing,
    Recall,
    Reassignment,
    InjuryReserve,
    /// Fallback when no rule matches. Real outcome — observability test
    /// caps this at <5% of bundled rows so future ESPN prose drift fails CI.
    Other,
}

impl TransactionKind {
    /// Lower-case label for CLI `--kind` filtering and observability logs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Trade => "trade",
            Self::WaiverPlacement => "waiver_placement",
            Self::WaiverClear => "waiver_clear",
            Self::WaiverClaim => "waiver_claim",
            Self::Signing => "signing",
            Self::Recall => "recall",
            Self::Reassignment => "reassignment",
            Self::InjuryReserve => "ir",
            Self::Other => "other",
        }
    }

    /// All variants in display order — used by the TUI legend card and
    /// CLI `--kind` validation hint.
    pub const ALL: &'static [TransactionKind] = &[
        Self::Trade,
        Self::Signing,
        Self::Recall,
        Self::Reassignment,
        Self::WaiverPlacement,
        Self::WaiverClear,
        Self::WaiverClaim,
        Self::InjuryReserve,
        Self::Other,
    ];

    /// Parse a CLI `--kind` arg into a TransactionKind. Accepts the
    /// short `waiver` form which expands to all three waiver kinds (the
    /// caller decides how to handle the multi-match — typically OR).
    pub fn parse_filter(s: &str) -> Result<Vec<TransactionKind>, String> {
        match s.to_ascii_lowercase().as_str() {
            "trade" => Ok(vec![Self::Trade]),
            "signing" | "sign" => Ok(vec![Self::Signing]),
            "recall" => Ok(vec![Self::Recall]),
            "reassignment" | "reassign" => Ok(vec![Self::Reassignment]),
            "ir" | "injuredreserve" => Ok(vec![Self::InjuryReserve]),
            "waiver" => Ok(vec![
                Self::WaiverPlacement,
                Self::WaiverClear,
                Self::WaiverClaim,
            ]),
            "waiver_placement" | "waiver-placement" => Ok(vec![Self::WaiverPlacement]),
            "waiver_clear" | "waiver-clear" => Ok(vec![Self::WaiverClear]),
            "waiver_claim" | "waiver-claim" => Ok(vec![Self::WaiverClaim]),
            "other" => Ok(vec![Self::Other]),
            unknown => Err(format!(
                "unknown kind '{unknown}'. valid: trade, signing, recall, \
                 reassignment, ir, waiver (or waiver_placement / waiver_clear \
                 / waiver_claim), other"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_transactions_earliest_season_constant_is_set() {
        // BENCH-mandated: prove the probe was run.
        assert!(
            !TRANSACTIONS_EARLIEST_SEASON.is_empty(),
            "TRANSACTIONS_EARLIEST_SEASON must be set after T.2 probe"
        );
        assert_eq!(
            TRANSACTIONS_EARLIEST_SEASON.len(),
            8,
            "season ID must be 8 digits, got: '{TRANSACTIONS_EARLIEST_SEASON}'"
        );
        assert!(
            TRANSACTIONS_EARLIEST_SEASON
                .chars()
                .all(|c| c.is_ascii_digit()),
            "season ID must be all digits"
        );
    }

    #[test]
    fn l0_kind_label_uniqueness() {
        let mut seen = std::collections::HashSet::new();
        for k in TransactionKind::ALL {
            assert!(seen.insert(k.label()), "duplicate label for kind: {:?}", k);
        }
    }

    #[test]
    fn l0_kind_parse_filter_known_kinds() {
        assert_eq!(
            TransactionKind::parse_filter("trade").unwrap(),
            vec![TransactionKind::Trade]
        );
        assert_eq!(
            TransactionKind::parse_filter("TRADE").unwrap(),
            vec![TransactionKind::Trade]
        );
        assert_eq!(
            TransactionKind::parse_filter("ir").unwrap(),
            vec![TransactionKind::InjuryReserve]
        );
    }

    #[test]
    fn l0_kind_parse_filter_waiver_expands_to_three() {
        let v = TransactionKind::parse_filter("waiver").unwrap();
        assert_eq!(
            v.len(),
            3,
            "bare 'waiver' must expand to all three waiver kinds"
        );
    }

    #[test]
    fn l0_kind_parse_filter_unknown_returns_helpful_error() {
        let err = TransactionKind::parse_filter("trades").unwrap_err();
        assert!(
            err.contains("unknown kind"),
            "error must say 'unknown kind', got: {err}"
        );
        assert!(
            err.contains("trade"),
            "error must list valid options, got: {err}"
        );
    }

    #[test]
    fn l0_transaction_serde_roundtrip() {
        let tx = Transaction {
            date: "2026-04-29".to_owned(),
            team: Some(TeamAbbr("TBL".to_owned())),
            kind: TransactionKind::Trade,
            description: "Acquired D X from NSH".to_owned(),
            id: "deadbeef".to_owned(),
            trade_group_id: Some("group1".to_owned()),
            classifier_version: 1,
        };
        let json = serde_json::to_string(&tx).unwrap();
        let back: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(tx, back);
    }

    #[test]
    fn l0_transaction_team_none_serializes_cleanly() {
        let tx = Transaction {
            date: "2026-04-29".to_owned(),
            team: None,
            kind: TransactionKind::Other,
            description: "League-wide reassignment deadline".to_owned(),
            id: "abc".to_owned(),
            trade_group_id: None,
            classifier_version: 1,
        };
        let json = serde_json::to_string(&tx).unwrap();
        assert!(
            json.contains("\"team\":null"),
            "team=None must serialize as null, got: {json}"
        );
    }
}
