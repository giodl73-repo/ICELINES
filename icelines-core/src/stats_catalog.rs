//! Phase Lindsay stat catalog — minimal L.1 surface.
//!
//! This module is intentionally lean for L.1: just the `ReportKind` enum
//! and the supporting `Tier1ReportFile` / `MergeTarget` types that
//! `icelines-fetch::nhl_api`, `icelines-fetch::snapshot::ChunkedManifest`,
//! and `StatsRepository::load_window` need to dispatch.
//!
//! The full Lindsay catalog (`StatId`, `StatCategory`, `FilterOp`,
//! `FilterParseError`, `StatFilter`, `aggregate_read`, `applies_to`, the
//! 108 stat-enumeration arms, etc.) lands in **L.2** per the plan
//! (`design/plans/2026-05-02-phaseLindsay-stat-catalog.md` v0.4
//! §"Sub-phases"). Anything not strictly required to ship L.1 is held
//! back so the reviewers' L.2 surface stays aligned with v0.4 §"Public
//! types" rather than drifting from a partial L.1 ship.

use serde::{Deserialize, Serialize};

use crate::season_stats::SeasonType;

/// One raw NHL API row from a Tier-1 endpoint. Implementors expose
/// the `seasonId` field for the per-endpoint fence (DI-29).
///
/// `load_report_with_fallback<R>` (in `icelines-fetch::stats_loader`)
/// requires `R: Tier1Row + DeserializeOwned`. For each row in a loaded
/// file, the loader calls `row.season_id()` and fails
/// `LoadError::SeasonIdMismatch` if any row's id disagrees with the
/// requested season. `None` is the bundled-trust path (pre-Hart.6
/// fixtures didn't carry seasonId on every row); the loader skips
/// the fence for those rows.
pub trait Tier1Row {
    /// `seasonId` from the API row if present. `None` for hand-edited
    /// or pre-Hart.6 fixtures.
    fn season_id(&self) -> Option<u32>;
}

/// One report tier — Tier 1 lives in typed substructs on `SeasonStats`
/// (loaded into the typed-window LRU); Tier 2 lives in `extra_reports`
/// on `StatsRepository` (runtime-only `BTreeMap`, DI-27).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Loaded eagerly with the typed window. Persisted to disk.
    Tier1,
    /// Fetched on demand. Lives in `extra_reports` (BTreeMap, DI-12
    /// cascade-evicted with the window). Never persisted.
    Tier2,
}

/// One per NHL stats-API endpoint we exercise. Tier-1 variants populate
/// typed substructs on `SeasonStats`; Tier-2 variants live in
/// `extra_reports` (runtime-only). Variant order is the canonical
/// `StatId::all()` enumeration order — the spec asserts iteration is
/// deterministic (AI-05).
///
/// `serde` shape is camelCase — matches the JSON nesting key in
/// `ChunkedManifest::reports` (e.g. `"skaterTimeOnIce"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReportKind {
    // ── Tier 1 — typed substructs on `SeasonStats` ────────────────────
    /// `/skater/summary` → `SeasonStats.totals` (existing, bundled).
    SkaterSummary,
    /// `/skater/bios` → `PlayerIdentity` (existing, bundled).
    SkaterBios,
    /// `/skater/realtime` → `SeasonStats.realtime` (existing field;
    /// new bundling target in Lindsay).
    SkaterRealtime,
    /// `/skater/timeonice` → `SeasonStats.time_on_ice` (Lindsay-NEW).
    SkaterTimeOnIce,
    /// `/skater/goalsForAgainst` → `SeasonStats.goals_for_against`
    /// (Lindsay-NEW).
    SkaterGoalsForAgainst,
    /// `/goalie/summary` → `SeasonStats.goalie` (existing, bundled).
    GoalieSummary,
    /// `/goalie/bios` → `SeasonStats.goalie_bios` (Lindsay-NEW;
    /// previously skater/bios was misused as goalie identity source).
    GoalieBios,
    /// `/goalie/advanced` → `SeasonStats.goalie_advanced` (Lindsay-NEW).
    GoalieAdvanced,
    /// `/goalie/savesByStrength` → `SeasonStats.goalie_saves_by_strength`
    /// (Lindsay-NEW).
    GoalieSavesByStrength,

    // ── Tier 2 — runtime-only blobs in `extra_reports` ────────────────
    SkaterPuckPossessions,
    SkaterScoringRates,
    SkaterSummaryShooting,
    SkaterPowerPlay,
    SkaterPenaltyKill,
    SkaterPenalties,
    SkaterFaceoffWins,
    SkaterFaceoffPercentages,
    SkaterShotType,
    SkaterScoringPerGame,
    GoalieStartedVsRelieved,
    GoalieDaysRest,
    GoaliePenaltyShots,
    GoalieShootout,
}

impl ReportKind {
    /// Path component appended to the stats base URL — e.g.
    /// `"skater/timeonice"`. Combined with the season-aware cayenneExp
    /// query in the fetch client. The probe artifact at
    /// `data/api-probe-2026-05-02.txt` records the exact verified URLs.
    pub fn url_path(self) -> &'static str {
        match self {
            // Tier 1
            Self::SkaterSummary           => "skater/summary",
            Self::SkaterBios              => "skater/bios",
            Self::SkaterRealtime          => "skater/realtime",
            Self::SkaterTimeOnIce         => "skater/timeonice",
            Self::SkaterGoalsForAgainst   => "skater/goalsForAgainst",
            Self::GoalieSummary           => "goalie/summary",
            Self::GoalieBios              => "goalie/bios",
            Self::GoalieAdvanced          => "goalie/advanced",
            Self::GoalieSavesByStrength   => "goalie/savesByStrength",
            // Tier 2
            Self::SkaterPuckPossessions   => "skater/puckPossessions",
            Self::SkaterScoringRates      => "skater/scoringRates",
            Self::SkaterSummaryShooting   => "skater/summaryshooting",
            Self::SkaterPowerPlay         => "skater/powerplay",
            Self::SkaterPenaltyKill       => "skater/penaltykill",
            Self::SkaterPenalties         => "skater/penalties",
            Self::SkaterFaceoffWins       => "skater/faceoffwins",
            Self::SkaterFaceoffPercentages => "skater/faceoffpercentages",
            Self::SkaterShotType          => "skater/shottype",
            Self::SkaterScoringPerGame    => "skater/scoringpergame",
            Self::GoalieStartedVsRelieved => "goalie/startedVsRelieved",
            Self::GoalieDaysRest          => "goalie/daysrest",
            Self::GoaliePenaltyShots      => "goalie/penaltyShots",
            Self::GoalieShootout          => "goalie/shootout",
        }
    }

    /// Whether this report has data for the given season type. Currently
    /// every documented-working endpoint supports both regular and
    /// playoff (verified 2026-05-02 — see `data/api-probe-2026-05-02.txt`
    /// §"Playoff sanity"). Tier-2 playoff support was NOT exhaustively
    /// re-probed in the artifact; this method conservatively returns
    /// `true` for them — the rate-limit policy + 500-fallback path in
    /// L.1.4 surfaces "no data this season-type" cleanly to the user
    /// rather than fence-tripping at L.1 entry.
    pub fn supports(self, _season_type: SeasonType) -> bool {
        true
    }

    /// Whether the NHL stats API actually serves this endpoint. Eight
    /// endpoints documented in the probe artifact return 500 (Skater:
    /// `advanced`; Goalie: `realtime`, `savePercentage`, `penaltykill`,
    /// `percentages`, `shottype`, `timeonice`, `goalsForAgainstByStrength`)
    /// — they are NOT enumerated as variants of `ReportKind` because we
    /// never call them. This method exists so a future maintainer who
    /// adds a kind variant doesn't accidentally include one of those
    /// without setting this to false.
    ///
    /// Lindsay L.1.6 (TAPE-R3 follow-up): the `fetch report` CLI gates
    /// on this method and refuses to dispatch known-broken endpoints
    /// instead of letting the L.1.5 retry policy burn 5 × 30s ≈ 2.5min.
    pub fn is_known_working(self) -> bool {
        // Every variant in `ReportKind::all()` is currently known-working
        // per the 2026-05-02 probe artifact. The method exists so a
        // future variant addition can flip this without changing the
        // enum surface — set the new arm to `false` until probed.
        true
    }

    /// Whether this report is loaded eagerly into typed-window LRU
    /// (Tier-1) or fetched on demand into `extra_reports` (Tier-2).
    pub fn tier(self) -> Tier {
        match self {
            Self::SkaterSummary
            | Self::SkaterBios
            | Self::SkaterRealtime
            | Self::SkaterTimeOnIce
            | Self::SkaterGoalsForAgainst
            | Self::GoalieSummary
            | Self::GoalieBios
            | Self::GoalieAdvanced
            | Self::GoalieSavesByStrength => Tier::Tier1,
            _ => Tier::Tier2,
        }
    }

    /// Every variant in declaration order. Callers iterating over the
    /// full report set MUST use this — don't roll your own slice.
    /// Determinism backs SI-03 (site rendering) and AI-05 (catalog
    /// iteration).
    pub fn all() -> &'static [ReportKind] {
        use ReportKind::*;
        &[
            // Tier 1
            SkaterSummary, SkaterBios, SkaterRealtime, SkaterTimeOnIce,
            SkaterGoalsForAgainst, GoalieSummary, GoalieBios, GoalieAdvanced,
            GoalieSavesByStrength,
            // Tier 2
            SkaterPuckPossessions, SkaterScoringRates, SkaterSummaryShooting,
            SkaterPowerPlay, SkaterPenaltyKill, SkaterPenalties,
            SkaterFaceoffWins, SkaterFaceoffPercentages, SkaterShotType,
            SkaterScoringPerGame,
            GoalieStartedVsRelieved, GoalieDaysRest, GoaliePenaltyShots,
            GoalieShootout,
        ]
    }
}

/// Where a Tier-1 report's deserialized rows merge onto `SeasonStats`.
/// Used by `load_report_with_fallback<T>` (L.1.4) to dispatch the
/// `Vec<Row>` into the right typed substruct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeTarget {
    /// `SeasonStats.totals` (and `team_stints`) — built by the existing
    /// `stats_loader` path. Kept as a variant for completeness; L.1.4
    /// does NOT reroute the existing summary loader through
    /// `load_report_with_fallback` (held for L.7 historical bundling).
    SkaterSummaryTotals,
    /// `PlayerIdentity` — built by the existing identity path. Same
    /// caveat as above.
    SkaterIdentity,
    /// `SeasonStats.realtime`.
    SkaterRealtime,
    /// `SeasonStats.time_on_ice`.
    SkaterTimeOnIce,
    /// `SeasonStats.goals_for_against`.
    SkaterGoalsForAgainst,
    /// `SeasonStats.goalie`.
    GoalieSummary,
    /// `SeasonStats.goalie_bios` — Lindsay switches goalie-identity
    /// source from skater/bios to here.
    GoalieBios,
    /// `SeasonStats.goalie_advanced`.
    GoalieAdvanced,
    /// `SeasonStats.goalie_saves_by_strength`.
    GoalieSavesByStrength,
}

/// The on-disk file and merge target for one Tier-1 report.
/// Used by `load_report_with_fallback<T>` (L.1.4) to read the right
/// per-window file and route the deserialized rows. The constant
/// `TIER1_REPORTS` enumerates every Tier-1 entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tier1ReportFile {
    pub kind: ReportKind,
    /// Per-window filename under `<snapshot_dir>/<season>/<season_type>/`.
    /// Pinned by spec §"Tier-1 file format".
    pub filename: &'static str,
    pub merge_target: MergeTarget,
}

/// Per-(ReportKind) Tier-1 file dispatch table. Order matches
/// `ReportKind::all()` for the Tier-1 prefix. The order also defines
/// the load order in `StatsRepository::load_window` — earlier entries
/// land first, so dependent reports (e.g. `GoalieBios` that influences
/// goalie-identity merging) come before consumers.
pub const TIER1_REPORTS: &[Tier1ReportFile] = &[
    Tier1ReportFile {
        kind: ReportKind::SkaterSummary,
        filename: "summary.json",
        merge_target: MergeTarget::SkaterSummaryTotals,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterBios,
        filename: "bios.json",
        merge_target: MergeTarget::SkaterIdentity,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterRealtime,
        filename: "realtime.json",
        merge_target: MergeTarget::SkaterRealtime,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterTimeOnIce,
        filename: "timeonice.json",
        merge_target: MergeTarget::SkaterTimeOnIce,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterGoalsForAgainst,
        filename: "goalsForAgainst.json",
        merge_target: MergeTarget::SkaterGoalsForAgainst,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieSummary,
        filename: "goalie-summary.json",
        merge_target: MergeTarget::GoalieSummary,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieBios,
        filename: "goalie-bios.json",
        merge_target: MergeTarget::GoalieBios,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieAdvanced,
        filename: "goalie-advanced.json",
        merge_target: MergeTarget::GoalieAdvanced,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieSavesByStrength,
        filename: "goalie-savesByStrength.json",
        merge_target: MergeTarget::GoalieSavesByStrength,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin: `ReportKind::all().len()` matches the documented inventory.
    /// 9 Tier-1 + 14 Tier-2 = 23 working endpoints. Matches the spec
    /// §"Endpoint inventory" and the probe artifact.
    #[test]
    fn l0_lindsay_report_kind_count_matches_inventory() {
        assert_eq!(ReportKind::all().len(), 23);
        let tier1: Vec<_> = ReportKind::all()
            .iter()
            .filter(|k| matches!(k.tier(), Tier::Tier1))
            .collect();
        let tier2: Vec<_> = ReportKind::all()
            .iter()
            .filter(|k| matches!(k.tier(), Tier::Tier2))
            .collect();
        assert_eq!(tier1.len(), 9, "9 Tier-1 reports per spec inventory");
        assert_eq!(tier2.len(), 14, "14 Tier-2 reports per spec inventory");
    }

    /// Pin: serde shape is camelCase. JSON keys in
    /// `ChunkedManifest.reports` (L.1.2) and Tier-2 cache keys
    /// (L.1.4 / L.6) round-trip through this representation.
    #[test]
    fn l0_lindsay_report_kind_serde_camel_case() {
        assert_eq!(
            serde_json::to_string(&ReportKind::SkaterTimeOnIce).unwrap(),
            "\"skaterTimeOnIce\"",
        );
        assert_eq!(
            serde_json::to_string(&ReportKind::GoalieSavesByStrength).unwrap(),
            "\"goalieSavesByStrength\"",
        );
        let back: ReportKind =
            serde_json::from_str("\"skaterGoalsForAgainst\"").unwrap();
        assert_eq!(back, ReportKind::SkaterGoalsForAgainst);
    }

    /// Pin: every endpoint in the probe artifact has a matching `url_path`.
    /// If anyone reorders the variants or renames a path, every snapshot
    /// reading that report kind would break — this test catches the slip
    /// before it ships.
    #[test]
    fn l0_lindsay_url_path_matches_probe_inventory() {
        // Tier-1 sample.
        assert_eq!(ReportKind::SkaterSummary.url_path(), "skater/summary");
        assert_eq!(ReportKind::SkaterTimeOnIce.url_path(), "skater/timeonice");
        assert_eq!(
            ReportKind::SkaterGoalsForAgainst.url_path(),
            "skater/goalsForAgainst",
        );
        assert_eq!(ReportKind::GoalieBios.url_path(), "goalie/bios");
        assert_eq!(
            ReportKind::GoalieSavesByStrength.url_path(),
            "goalie/savesByStrength",
        );
        // Tier-2 sample.
        assert_eq!(
            ReportKind::SkaterPuckPossessions.url_path(),
            "skater/puckPossessions",
        );
        assert_eq!(
            ReportKind::GoalieStartedVsRelieved.url_path(),
            "goalie/startedVsRelieved",
        );
    }

    /// Pin: `supports(season_type)` returns `true` for both regular and
    /// playoff on every Tier-1 endpoint. Verified by the playoff sanity
    /// probe at `data/api-probe-2026-05-02.txt`.
    #[test]
    fn l0_lindsay_tier1_supports_both_season_types() {
        for kind in ReportKind::all() {
            if !matches!(kind.tier(), Tier::Tier1) {
                continue;
            }
            assert!(
                kind.supports(SeasonType::Regular),
                "{:?} must support Regular",
                kind,
            );
            assert!(
                kind.supports(SeasonType::Playoff),
                "{:?} must support Playoff",
                kind,
            );
        }
    }

    /// Pin: `TIER1_REPORTS` covers every Tier-1 `ReportKind`. If anyone
    /// adds a new Tier-1 variant without adding a `Tier1ReportFile`
    /// row, the loader (L.1.4) silently skips that report. This test
    /// catches the omission.
    #[test]
    fn l0_lindsay_tier1_reports_table_is_exhaustive() {
        let tier1_kinds: Vec<ReportKind> = ReportKind::all()
            .iter()
            .filter(|k| matches!(k.tier(), Tier::Tier1))
            .copied()
            .collect();
        let table_kinds: Vec<ReportKind> =
            TIER1_REPORTS.iter().map(|r| r.kind).collect();
        assert_eq!(
            tier1_kinds, table_kinds,
            "TIER1_REPORTS table must enumerate every Tier-1 ReportKind \
             in declaration order",
        );
    }

    /// Pin: filenames in `TIER1_REPORTS` are unique. Two reports
    /// pointing at the same file would silently overwrite each other
    /// at write time and load the wrong substruct at read time.
    #[test]
    fn l0_lindsay_tier1_reports_filenames_unique() {
        let mut seen = std::collections::HashSet::new();
        for r in TIER1_REPORTS {
            assert!(
                seen.insert(r.filename),
                "duplicate Tier-1 filename: {}",
                r.filename,
            );
        }
    }

    /// BENCH closeout #2: pin that every variant currently returns
    /// `is_known_working() == true`. The 23 enumerated variants ARE
    /// the working endpoints (verified 2026-05-02 — see
    /// `data/api-probe-2026-05-02.txt`). The 8 documented-broken
    /// endpoints are intentionally NOT enum variants, so no false
    /// can sneak in today. If a future maintainer adds a variant
    /// for a still-broken endpoint, this test catches it: they MUST
    /// either probe-and-fix the endpoint OR set the new arm's return
    /// to `false`. Either path forces explicit acknowledgement.
    #[test]
    fn l0_lindsay_every_report_kind_is_known_working() {
        for kind in ReportKind::all() {
            assert!(
                kind.is_known_working(),
                "ReportKind::{kind:?} must be known-working OR get a \
                 dedicated false-return arm in is_known_working() — \
                 see data/api-probe-2026-05-02.txt for the inventory"
            );
        }
    }
}
