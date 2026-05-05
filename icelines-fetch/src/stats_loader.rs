//! Phase Hart.3 — `StatsRepository` loader.
//!
//! Populates a fresh `StatsRepository` from the bundled+snapshot data
//! tiers. Surfaces partial-fetch conditions via `LoadOutcome.missing` so
//! callers can render specific banners ("MoneyPuck unavailable") rather
//! than silently shipping `advanced=None` for every player.
//!
//! Parallel-run with the legacy `PlayerRepository::load_all()` path:
//! both can run from the same snapshot store. Hart.4 sub-phases migrate
//! consumers commit-by-commit; Hart.5 deletes the legacy path.

use std::collections::HashMap;

use icelines_core::contract::PlayerContract;
use icelines_core::identity::{PlayerBio, PlayerId, PlayerIdentity};
use icelines_core::model::{Position, Season, TeamAbbr};
use icelines_core::name::normalize_name;
use icelines_core::scoring::compute_pace_score;
use icelines_core::season_stats::{
    AdvancedStats, GoalieSeasonStats, RealtimeStats, SeasonStatsBuilder, SeasonType, StatTotals,
    TeamStint, SYNTHETIC_DATE_PREFIX,
};
use icelines_core::stats_catalog::{Tier1ReportFile, Tier1Row};
use icelines_core::stats_repository::{RepoError, StatsRepository};
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::bundled;
use crate::moneypuck::MoneyPuckStats;
use crate::schema::{
    GoalieStats, PlayerContract as LegacyContract, SkaterBio, SkaterRealtime, SkaterStats,
};
use crate::snapshot::{SnapshotMetaFlags, SnapshotStore, SnapshotTier};

/// Bundled-JSON file format version this binary understands. Bumps on
/// non-`Option` field additions to existing types in the bundles.
/// Aliases `SnapshotMetaFlags::CURRENT_BUNDLE_SCHEMA_VERSION` so the
/// reader (this loader) and writer (`SnapshotMetaFlags::save`) stay
/// in lockstep.
pub const MAX_KNOWN_BUNDLE_SCHEMA: u32 = SnapshotMetaFlags::CURRENT_BUNDLE_SCHEMA_VERSION;

/// In-memory `StatsRepository` model version. Bumps on every breaking
/// change to the `icelines-core` model. Phase Hart starts at 1.
pub const MAX_KNOWN_REPO_VERSION: u32 = SnapshotMetaFlags::CURRENT_REPOSITORY_VERSION;

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LoadError {
    #[error("season {season} not bundled in this build")]
    SeasonNotBundled { season: String },
    #[error("season {season} has no {season_type} bundle")]
    MissingBundle {
        season: String,
        season_type: SeasonType,
    },
    #[error("bundle schema version {found} unknown (this binary supports up to {max_known})")]
    BundleSchemaUnknown { found: u32, max_known: u32 },
    #[error("bundle repository version {found} unknown (this binary supports up to {max_known})")]
    RepoVersionUnknown { found: u32, max_known: u32 },
    #[error("repository error: {0}")]
    Repo(#[from] RepoError),
    /// Hart.6.4 (Tape B2 / Bench B2): a loaded row has a `seasonId` that
    /// disagrees with the requested season. Indicates either a
    /// mis-authored bundled file or a bug in the fetch CLI that wrote
    /// the wrong season into a snapshot. The loader fails-loud here
    /// rather than silently mixing seasons in `StatsRepository`.
    #[error(
        "season-id mismatch: requested {expected}, found {found} in {count} row(s) — \
         re-run `icelines fetch stats` for season {expected} or fix the bundled file"
    )]
    SeasonIdMismatch {
        expected: u32,
        found: u32,
        count: usize,
    },
    /// Phase Lindsay L.1.4: per-report I/O or parse failure on a
    /// Tier-1 file (`timeonice.json`, `goalsForAgainst.json`, etc.).
    /// Carries the file kind + a stringified failure cause.
    /// (Field name `cause` not `source` — thiserror treats `source`
    /// as a magic `#[source]` field expecting `std::error::Error`.)
    #[error("Tier-1 report load failed for {kind}: {cause}")]
    ReportLoad { kind: String, cause: String },
    // Hart.4.1 v0.2 (Gap H): LoadError::Bundle and the BundleError enum
    // were dropped. They were dead code (no path produced one — bundle
    // reads went through .map_err(|_| SeasonNotBundled)). Reintroduce
    // when a real call site emerges that needs distinct
    // I/O-vs-Parse-vs-NotBundled error fidelity.
}

// ── LoadOutcome / MissingSource ─────────────────────────────────────────────

/// Per-source partial-fetch signal. Each variant maps to a specific
/// user-facing banner in the CLI / TUI.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MissingSource {
    Realtime {
        season: String,
        season_type: SeasonType,
        reason: String,
    },
    MoneyPuck {
        season: String,
        reason: String,
    },
    Contracts {
        reason: String,
    },
    GoalieStats {
        season: String,
        season_type: SeasonType,
        reason: String,
    },
}

impl MissingSource {
    /// Short label used in user-facing status banners. Each variant has a
    /// fixed string; this is the canonical mapping shared across surfaces
    /// (TUI status bar, CLI WARN line, HTTP X-IceLines-Missing header).
    pub fn label(&self) -> &'static str {
        match self {
            Self::Realtime { .. } => "realtime",
            Self::MoneyPuck { .. } => "MoneyPuck",
            Self::Contracts { .. } => "contracts",
            Self::GoalieStats { .. } => "goalie stats",
        }
    }
}

/// Map a `&[MissingSource]` to a one-line user-facing banner. Called by
/// the TUI status bar after a load completes; CLI / HTTP surfaces use
/// the same helper. Returns the empty string for an empty slice so
/// callers can compose without a separate branch.
pub fn format_missing_sources(missing: &[MissingSource]) -> String {
    if missing.is_empty() {
        return String::new();
    }
    let labels: Vec<&str> = missing.iter().map(MissingSource::label).collect();
    format!("Missing data: {}", labels.join(", "))
}

#[cfg(test)]
mod missing_source_tests {
    use super::*;

    #[test]
    fn l0_format_missing_sources_empty_returns_empty() {
        assert_eq!(format_missing_sources(&[]), "");
    }

    #[test]
    fn l0_format_missing_sources_single_realtime() {
        let m = MissingSource::Realtime {
            season: "20242025".into(),
            season_type: SeasonType::Regular,
            reason: "snapshot absent".into(),
        };
        assert_eq!(format_missing_sources(&[m]), "Missing data: realtime");
    }

    #[test]
    fn l0_format_missing_sources_all_four_in_order() {
        let entries = vec![
            MissingSource::Realtime {
                season: "20242025".into(),
                season_type: SeasonType::Regular,
                reason: "x".into(),
            },
            MissingSource::MoneyPuck {
                season: "20242025".into(),
                reason: "x".into(),
            },
            MissingSource::Contracts { reason: "x".into() },
            MissingSource::GoalieStats {
                season: "20242025".into(),
                season_type: SeasonType::Regular,
                reason: "x".into(),
            },
        ];
        assert_eq!(
            format_missing_sources(&entries),
            "Missing data: realtime, MoneyPuck, contracts, goalie stats"
        );
    }

    #[test]
    fn l0_label_is_stable_per_variant() {
        // Locks the labels — changes here must update both the help
        // banner and any downstream consumer that pattern-matches on
        // the visible string (CI dashboards, log scrapers).
        assert_eq!(
            MissingSource::Realtime {
                season: "x".into(),
                season_type: SeasonType::Regular,
                reason: "x".into()
            }
            .label(),
            "realtime"
        );
        assert_eq!(
            MissingSource::MoneyPuck {
                season: "x".into(),
                reason: "x".into()
            }
            .label(),
            "MoneyPuck"
        );
        assert_eq!(
            MissingSource::Contracts { reason: "x".into() }.label(),
            "contracts"
        );
        assert_eq!(
            MissingSource::GoalieStats {
                season: "x".into(),
                season_type: SeasonType::Regular,
                reason: "x".into()
            }
            .label(),
            "goalie stats"
        );
    }
}

/// Result of populating a `StatsRepository` from one (season, type)
/// load. `missing` is empty for a clean load; non-empty entries identify
/// specific tiers that didn't materialize. `missing_files` is a
/// finer-grained diagnostic — file names attempted but not found.
#[derive(Debug)]
pub struct LoadOutcome {
    pub repo: StatsRepository,
    pub missing: Vec<MissingSource>,
    pub missing_files: Vec<String>,
    pub fetched_at: String,
}

// ── Loader ──────────────────────────────────────────────────────────────────

/// Populate a fresh `StatsRepository` for the given (season, type).
///
/// Source order:
/// - bios + stats: snapshot first, bundled fallback (the existing
///   `bundled::load_*_with_fallback` chain).
/// - realtime / moneypuck / contracts: snapshot-only — flagged as
///   `MissingSource` if not present.
/// - goalie-stats: bundled-only for v0.13 — flagged if missing.
///
/// Hart.6.4: type-keyed source selection. For `Regular`, reads from the
/// existing bios/stats/goalies fallback chain. For `Playoff`, reads from
/// the parallel `load_playoff_*_with_fallback` chain (Hart.6.2).
/// Empty playoff bios surfaces as `MissingBundle { season_type: Playoff }`
/// per Forge F4. Cross-season rows surface as `SeasonIdMismatch`.
pub fn load_into_repo(
    season: Season,
    season_type: SeasonType,
    store: &SnapshotStore,
) -> Result<LoadOutcome, LoadError> {
    let season_str = season.as_str();

    // Schema-version gate (DI-28 / Lindsay L.1.3). Missing _meta.json
    // (cold-start) is fine — `SnapshotMetaFlags::default()` yields
    // version 0, which we treat as "pre-Hart, no version stamp" and
    // accept. Only positive values that *exceed* what this binary knows
    // are an error.
    //
    // **DI-28 contract**: this check fires at load-time (file-open
    // boundary), NOT deferred to `StatsRepository::repo_swap`. An old
    // binary opening a Lindsay-stamped (v=2) snapshot must error here
    // with `LoadError::RepoVersionUnknown { found, max_known }`. The
    // L1 test `l1_lindsay_load_rejects_repository_version_above_known`
    // synthesizes a future-stamped `_meta.json` and asserts this fence
    // fires before any chunk is touched.
    //
    // TODO(Hart.N): when MAX_KNOWN_BUNDLE_SCHEMA bumps past 1, this
    // gate must dispatch a migrator on `version < MAX_KNOWN` per the
    // plan ("incoming < known = run a migrator"). Today it accepts
    // silently because only versions 0 and 1 exist. Reviewer reading
    // this comment after a future bump: write the migrator.
    let meta = SnapshotMetaFlags::load(store.root(), &season_str);
    if meta.bundle_schema_version > MAX_KNOWN_BUNDLE_SCHEMA {
        return Err(LoadError::BundleSchemaUnknown {
            found: meta.bundle_schema_version,
            max_known: MAX_KNOWN_BUNDLE_SCHEMA,
        });
    }
    if meta.repository_version > MAX_KNOWN_REPO_VERSION {
        return Err(LoadError::RepoVersionUnknown {
            found: meta.repository_version,
            max_known: MAX_KNOWN_REPO_VERSION,
        });
    }

    // ── Tier reads ──────────────────────────────────────────────────────────

    // Bios — fallback chain. Hard-fail if neither snapshot nor bundle has them.
    // Loader contract: identities are required, stats and below can be empty.
    // Empty playoff bios surfaces as MissingBundle (carries season_type) per
    // Forge F4; empty regular bios surfaces as SeasonNotBundled (regular is
    // assumed always-present for any season the binary knows).
    let (bios, stats, goalie_stats) = match season_type {
        SeasonType::Regular => {
            let bios = bundled::load_bios_with_fallback(&season_str, store).map_err(|_| {
                LoadError::SeasonNotBundled {
                    season: season_str.clone(),
                }
            })?;
            if bios.is_empty() {
                return Err(LoadError::SeasonNotBundled {
                    season: season_str.clone(),
                });
            }
            let stats = bundled::load_stats_with_fallback(&season_str, store).unwrap_or_default();
            let goalie_stats = bundled::get_goalie_stats(&season_str).unwrap_or_default();
            (bios, stats, goalie_stats)
        }
        SeasonType::Playoff => {
            let bios =
                bundled::load_playoff_bios_with_fallback(&season_str, store).unwrap_or_default();
            if bios.is_empty() {
                return Err(LoadError::MissingBundle {
                    season: season_str.clone(),
                    season_type,
                });
            }
            let stats =
                bundled::load_playoff_stats_with_fallback(&season_str, store).unwrap_or_default();
            let goalie_stats =
                bundled::load_playoff_goalies_with_fallback(&season_str, store).unwrap_or_default();
            (bios, stats, goalie_stats)
        }
    };

    // Tape B2: reject cross-season rows. `season_id` on bios is Option<u32>
    // (bundled-pre-Hart.6 lacks it — None passes); `season_id` on stats is
    // also Option<u32>. None means "trust this row" (bundled compat); Some(x)
    // where x != requested.0 is a mismatch. GoalieStats's season_id is u32
    // (always present per pre-Hart schema).
    let expected = season.0;
    let mismatched_bios: Vec<u32> = bios
        .iter()
        .filter_map(|b| b.season_id)
        .filter(|sid| *sid != expected)
        .collect();
    let mismatched_stats: Vec<u32> = stats
        .iter()
        .filter_map(|s| s.season_id)
        .filter(|sid| *sid != expected)
        .collect();
    let mismatched_goalies: Vec<u32> = goalie_stats
        .iter()
        .map(|g| g.season_id)
        .filter(|sid| *sid != expected)
        .collect();
    let total_mismatched =
        mismatched_bios.len() + mismatched_stats.len() + mismatched_goalies.len();
    if total_mismatched > 0 {
        // Surface the first observed mismatched id so the error message
        // points at concrete data, not just a count.
        let found = mismatched_bios
            .first()
            .or_else(|| mismatched_stats.first())
            .or_else(|| mismatched_goalies.first())
            .copied()
            .unwrap_or(0);
        return Err(LoadError::SeasonIdMismatch {
            expected,
            found,
            count: total_mismatched,
        });
    }

    let mut missing: Vec<MissingSource> = Vec::new();
    let mut missing_files: Vec<String> = Vec::new();

    // For each tier: distinguish "absent file", "corrupt file", and
    // "empty array" (treated as missing for UI parity — the user sees
    // the same "no realtime data" banner either way). The reason
    // string carries the underlying error so a corrupt/permission
    // failure surfaces in diagnostics rather than being collapsed
    // into "not present".
    // Hart.6.4 / D6: Realtime is regular-season only. Playoff realtime
    // is collected through the live game feed (not a separate dataset),
    // so the playoff path returns an empty vec WITHOUT pushing to
    // `missing` — the absence is by design, not a partial fetch.
    // Phase Reports — realtime missing is no longer surfaced as a
    // banner. The Reports overlay (Reports.4) lets users opt in/out
    // explicitly; an absent realtime.json is downstream-handled per
    // player (Option<&SkaterRealtime>) and the Hits/Blocks columns
    // either render with their values or get hidden when the user
    // toggles realtime off. The missing-data banner only ever fired
    // for realtime in practice (other reports also missing-banner
    // but those branches stay) and was noise-not-signal.
    // Realtime resolution — try the season-and-tier-aware reader
    // first (scans the full snapshot index for the right season's
    // realtime/realtime.json). The active-snapshot-chain reader
    // (`read_tier`) misses when the user fetched realtime in a
    // separate call, since the active pointer floats with the most
    // recent fetch. We fall through to chain lookup only as a
    // backstop to preserve legacy behavior for already-installed
    // setups that put realtime in the active chain.
    let realtime: Vec<SkaterRealtime> = match season_type {
        SeasonType::Regular => store
            .read_tier_any_for_season::<Vec<SkaterRealtime>>(
                &SnapshotTier::Realtime,
                "realtime.json",
                &season_str,
            )
            .or_else(|_| {
                store.read_tier::<Vec<SkaterRealtime>>(&SnapshotTier::Realtime, "realtime.json")
            })
            .unwrap_or_default(),
        SeasonType::Playoff => Vec::new(),
    };
    // Hart.6.4 / D6: MoneyPuck doesn't expose a playoff endpoint
    // (verified against /skaters.csv format). Playoff path surfaces a
    // MoneyPuck `MissingSource` with an explanatory reason rather than
    // attempting a read that always fails.
    let moneypuck: Vec<MoneyPuckStats> = match season_type {
        SeasonType::Regular => match store
            .read_tier::<Vec<MoneyPuckStats>>(&SnapshotTier::MoneyPuck, "moneypuck.json")
        {
            Ok(m) if !m.is_empty() => m,
            Ok(_empty) => {
                missing.push(MissingSource::MoneyPuck {
                    season: season_str.clone(),
                    reason: "moneypuck.json present but empty".into(),
                });
                missing_files.push("snapshot:moneypuck.json".into());
                Vec::new()
            }
            Err(e) => {
                missing.push(MissingSource::MoneyPuck {
                    season: season_str.clone(),
                    reason: format!("moneypuck.json unreadable: {e}"),
                });
                missing_files.push("snapshot:moneypuck.json".into());
                Vec::new()
            }
        },
        SeasonType::Playoff => {
            missing.push(MissingSource::MoneyPuck {
                season: season_str.clone(),
                reason:
                    "advanced stats not populated for playoff season_type — Hart.6 v1 limitation"
                        .into(),
            });
            Vec::new()
        }
    };
    let contracts: Vec<LegacyContract> =
        match store.read_tier::<Vec<LegacyContract>>(&SnapshotTier::Contracts, "contracts.json") {
            Ok(c) if !c.is_empty() => c,
            Ok(_empty) => {
                missing.push(MissingSource::Contracts {
                    reason: "contracts.json present but empty".into(),
                });
                missing_files.push("snapshot:contracts.json".into());
                Vec::new()
            }
            Err(e) => {
                missing.push(MissingSource::Contracts {
                    reason: format!("contracts.json unreadable: {e}"),
                });
                missing_files.push("snapshot:contracts.json".into());
                Vec::new()
            }
        };
    if goalie_stats.is_empty() {
        missing.push(MissingSource::GoalieStats {
            season: season_str.clone(),
            season_type,
            reason: "goalie-stats.json not bundled for this season".into(),
        });
        missing_files.push("bundled:goalie-stats.json".into());
    }

    // ── Indexes ─────────────────────────────────────────────────────────────

    let stats_idx: HashMap<u32, &SkaterStats> = stats.iter().map(|s| (s.player_id, s)).collect();
    let realtime_idx: HashMap<u32, &SkaterRealtime> =
        realtime.iter().map(|r| (r.player_id, r)).collect();
    let moneypuck_idx: HashMap<u32, &MoneyPuckStats> =
        moneypuck.iter().map(|m| (m.player_id, m)).collect();
    let contracts_idx: HashMap<u32, &LegacyContract> =
        contracts.iter().map(|c| (c.player_id, c)).collect();

    // Dedup bios by player_id, last-occurrence-wins. The NHL bios endpoint
    // emits one row per team-stint for traded players; keep the most-recent
    // (current-team) row so the identity reflects the player's current club.
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let bios_dedup: Vec<&SkaterBio> = bios
        .iter()
        .rev()
        .filter(|b| seen.insert(b.player_id))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // ── Populate repository ─────────────────────────────────────────────────

    // Gaps.2/3 — bump the LRU cap to 80 windows so a downstream lazy
    // career fan-out doesn't evict the active season (38 historical
    // seasons × 2 types ≈ 76 windows + active = 77; cap of 80 gives
    // headroom). Memory cost is bounded by actual rows inserted —
    // single-player career windows hold ~1 row each.
    let mut repo = StatsRepository::with_lru_cap(80);

    // 1. Skater identities + stats.
    for bio in &bios_dedup {
        let Some(position) = Position::from_api_code(&bio.position_code) else {
            continue;
        };
        // Skip pure goalie rows that crept into bios (none expected, but defensive).
        if matches!(position, Position::Goalie) {
            continue;
        }
        let pid = PlayerId(bio.player_id);
        let identity = build_identity(pid, bio);
        repo.upsert_identity(identity)?;

        let stats_row = stats_idx.get(&bio.player_id).copied();
        let realtime_row = realtime_idx.get(&bio.player_id).copied();
        let mp_row = moneypuck_idx.get(&bio.player_id).copied();
        let stats = build_skater_stats(
            pid,
            season,
            season_type,
            position,
            bio,
            stats_row,
            realtime_row,
            mp_row,
        );
        repo.upsert_stats(stats)?;

        if let Some(c) = contracts_idx.get(&bio.player_id) {
            repo.upsert_contract(pid, build_contract(c));
        }
    }

    // 2. Goalies (different source — goalie-stats.json carries name/team).
    for g in &goalie_stats {
        let pid = PlayerId(g.player_id);
        // A goalie's identity may already exist from contracts; insert
        // bare-bones identity if not.
        if repo.identity(pid).is_none() {
            repo.upsert_identity(build_goalie_identity(g))?;
        }
        let stats = build_goalie_season_stats(pid, season, season_type, g);
        repo.upsert_stats(stats)?;
        if let Some(c) = contracts_idx.get(&g.player_id) {
            repo.upsert_contract(pid, build_contract(c));
        }
    }

    Ok(LoadOutcome {
        repo,
        missing,
        missing_files,
        fetched_at: now_iso8601(),
    })
}

// ── Phase Lindsay L.1.4 — Tier-1 per-report loader ──────────────────────────
//
// `load_report_with_fallback<R>` reads a Tier-1 per-report file for one
// (season, season_type) window and returns the parsed rows. Decision tree:
//   1. snapshot_dir/<season>/<season_type>/<file.filename>  — primary
//   2. bundled in-binary fallback (currently no Lindsay reports are
//      bundled — that's L.7 historical work). The slot is here so the
//      loader signature doesn't change when L.7 wires it up.
//   3. neither present → `Ok(None)`. The Tier-1 substruct on `SeasonStats`
//      stays `None` (DI-09 distinction between "not loaded" and "real zero").
//
// Per-row seasonId fence (DI-29) fires before any rows reach the caller.

/// API response wrapper used by every NHL stats endpoint:
/// `{ "data": [...rows], "total": N }`. Re-declared here as a local
/// helper rather than re-exported from `crate::schema::PagedResponse`
/// to keep the loader's per-report helpers self-contained.
#[derive(Debug, serde::Deserialize)]
struct PagedResponseLocal<R> {
    data: Vec<R>,
    #[serde(default)]
    #[allow(dead_code)] // shape pin: the API emits it but we don't consume it
    total: u32,
}

/// Per-(season, season_type) Tier-1 report loader.
///
/// **DI-29 (seasonId fence)** — every row whose `season_id()` returns
/// `Some(x)` is checked against the requested season; mismatch errors
/// `LoadError::SeasonIdMismatch` BEFORE any data reaches the caller.
/// Rows with `None` (pre-Hart.6 fixtures, hand-edited test data) are
/// trusted — same precedent as `load_into_repo`'s existing Hart.6.4
/// fence.
///
/// **DI-09 / "not loaded" vs "real zero"** — `Ok(None)` means the file
/// is absent at every fallback level. Caller stores `None` on the
/// substruct; consumers reading `view.stats.time_on_ice` see `None`
/// and render "—" instead of `0`. An empty-data file (`{"data":[],
/// "total":0}`) parses to `Ok(Some(Vec::new()))` — that's a real
/// "zero rows for this season," distinct from "not loaded".
///
/// **Read-only — never mutates the snapshot.** Even a v=1 snapshot
/// stays v=1 on disk (WIRE checkpoint follow-up #1 — read-only contract
/// pinned at the function level + the
/// `l1_lindsay_load_report_does_not_mutate_snapshot` test below).
pub fn load_report_with_fallback<R>(
    snapshot_dir: &std::path::Path,
    season: Season,
    season_type: SeasonType,
    file: &Tier1ReportFile,
) -> Result<Option<Vec<R>>, LoadError>
where
    R: Tier1Row + DeserializeOwned,
{
    // 1. Snapshot dir path: <snapshot_dir>/<season>/<season_type>/<filename>
    let season_str = season.as_str();
    let path = snapshot_dir
        .join(&season_str)
        .join(season_type.label())
        .join(file.filename);

    let raw_bytes = match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        // Treat any missing-file as "fall through to bundled". Other I/O
        // errors propagate so a corrupted disk surfaces clearly.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(LoadError::ReportLoad {
                kind: format!("{} ({} {})", file.filename, season.0, season_type.label()),
                cause: format!("I/O: {e}"),
            });
        }
    };

    // 2. Bundled fallback (placeholder — L.7 wires the include_bytes! map
    //    when the 38 historical seasons get bundled). For now this is a
    //    no-op; the call slot is here so L.7 doesn't change the signature.
    let bytes_opt =
        raw_bytes.or_else(|| bundled::report_for_lindsay(&season_str, season_type, file.kind));

    let bytes = match bytes_opt {
        Some(b) => b,
        None => return Ok(None), // (3) neither present
    };

    // Parse the standard `{ "data": [...], "total": N }` envelope.
    let parsed: PagedResponseLocal<R> =
        serde_json::from_slice(&bytes).map_err(|e| LoadError::ReportLoad {
            kind: format!("{} ({} {})", file.filename, season.0, season_type.label()),
            cause: format!("JSON parse: {e}"),
        })?;

    // DI-29 — per-row seasonId fence. Trust rows with `None` (bundled
    // / hand-edited compat); reject any row with a `Some(x)` that
    // disagrees with the requested season.
    let expected = season.0;
    let mismatched: Vec<u32> = parsed
        .data
        .iter()
        .filter_map(|r| r.season_id())
        .filter(|sid| *sid != expected)
        .collect();
    if !mismatched.is_empty() {
        return Err(LoadError::SeasonIdMismatch {
            expected,
            found: mismatched[0],
            count: mismatched.len(),
        });
    }

    Ok(Some(parsed.data))
}

/// Gaps.2/5 — resolve a player NAME (case-insensitive partial match)
/// to a PlayerId by walking bundled season bios newest-first. Returns
/// the first matching id, or `None` if no bundle carries the player.
///
/// Used by the player/compare CLI commands so a query like
/// `query player Wayne Gretzky` resolves even when the active season
/// is post-1999 and Gretzky's bio isn't in the active repo.
///
/// Order: skater bios first across all bundled seasons, then goalie
/// bios. Newest-first so a name collision (rare) prefers the most
/// recent player.
pub fn resolve_player_id_by_name(name: &str) -> Option<u32> {
    use crate::bundled;
    let needle = icelines_core::name::normalize_name(name);

    // Skater bios first.
    for season_id in bundled::BUNDLED_SEASONS {
        if let Some(bios) = bundled::get_bios(season_id) {
            if let Some(bio) = bios.iter().find(|b| {
                icelines_core::name::normalize_name(&b.skater_full_name).contains(&needle)
            }) {
                return Some(bio.player_id);
            }
        }
    }
    // Goalie summary rows carry the name on `goalie_full_name`.
    for season_id in bundled::BUNDLED_SEASONS {
        if let Some(goalies) = bundled::get_goalie_stats(season_id) {
            if let Some(g) = goalies.iter().find(|g| {
                icelines_core::name::normalize_name(&g.goalie_full_name).contains(&needle)
            }) {
                return Some(g.player_id);
            }
        }
    }
    None
}

// ── Phase UX.1 — lazy per-player career loader ──────────────────────────────

/// Pull a single player's bios + stats rows from every bundled season
/// and merge them into `repo`. Used by the TUI player card to surface a
/// player's full historical record without paying the cost of loading
/// every player × every season into the active repo.
///
/// Behavior:
/// - Iterates `BUNDLED_SEASONS` (newest-first); for each season, walks
///   the bundled bios looking for a row whose `playerId == pid`.
/// - When found, upserts identity (if not already present) and stats
///   for both Regular and Playoff (if the playoff bundle carries the
///   player) into `repo`.
/// - Skips seasons the player didn't appear in.
/// - Returns the count of (season, season_type) windows inserted —
///   useful for tests and status messaging.
///
/// Idempotent: re-calling for the same `pid` is a no-op aside from
/// re-running the bundle scans (~5 ms). Caller should dedupe via a
/// HashSet at the App layer to avoid the redundant scans.
///
/// Doesn't touch realtime / moneypuck — those are snapshot-only and
/// not bundled. Career-table consumers handle missing realtime via the
/// per-season `Option<&SkaterRealtime>` already.
pub fn load_player_career_into_repo(
    repo: &mut StatsRepository,
    pid: PlayerId,
) -> Result<usize, RepoError> {
    use crate::bundled;
    use crate::snapshot::{SnapshotStore, SnapshotTier};
    use icelines_core::season_stats::SeasonType;

    let mut inserted = 0usize;
    // Identity merge across many seasons trips `LikelyIdReissue` when
    // `firstSeasonForGameType` differs between regular and playoff bios
    // (the API anchors playoff bios to the player's first PLAYOFF
    // season, regular bios to first regular season — they won't match
    // for any player). The fan-out only needs ONE identity row; subsequent
    // seasons write stats directly. Track first-success per pid here.
    let mut identity_inserted = repo.identity(pid).is_some();

    // Realtime data lives in the snapshot store (not bundled). Cache
    // per-season reads so fanning out across 38 seasons doesn't
    // re-parse the same JSON 38 times. Per UX.B/realtime fix: the
    // player card was rendering "—" for hits/blocks/TK/GV because
    // the fan-out passed None and overwrote the active-season row
    // that DID have realtime loaded. Fix: read snapshot realtime per
    // season (where it exists) and merge it into the upsert.
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let mut realtime_cache: std::collections::HashMap<&'static str, Vec<SkaterRealtime>> =
        std::collections::HashMap::new();

    for season_id in bundled::BUNDLED_SEASONS {
        let season_u32: u32 = match season_id.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let season = Season(season_u32);

        // Lazy-resolve realtime for this season once.
        let realtime_for_season: &Vec<SkaterRealtime> =
            realtime_cache.entry(season_id).or_insert_with(|| {
                store
                    .read_tier_any_for_season::<Vec<SkaterRealtime>>(
                        &SnapshotTier::Realtime,
                        "realtime.json",
                        season_id,
                    )
                    .unwrap_or_default()
            });
        let realtime_row = realtime_for_season.iter().find(|r| r.player_id == pid.0);

        // ── Regular season ──
        if let Some(bios) = bundled::get_bios(season_id) {
            if let Some(bio) = bios.iter().find(|b| b.player_id == pid.0) {
                let position = match Position::from_api_code(&bio.position_code) {
                    Some(p) if !matches!(p, Position::Goalie) => p,
                    _ => continue, // Goalie rows handled by the goalie path
                };
                if !identity_inserted {
                    let identity = build_identity(pid, bio);
                    repo.upsert_identity(identity)?;
                    identity_inserted = true;
                }

                let stats_vec = bundled::get_stats(season_id).unwrap_or_default();
                let stats_row = stats_vec.iter().find(|s| s.player_id == pid.0);
                let stats = build_skater_stats(
                    pid,
                    season,
                    SeasonType::Regular,
                    position,
                    bio,
                    stats_row,
                    realtime_row,
                    None,
                );
                repo.upsert_stats(stats)?;
                inserted += 1;
            }
        }

        // ── Playoff season ──
        if let Some(po_bios) = bundled::get_playoff_bios(season_id) {
            if let Some(bio) = po_bios.iter().find(|b| b.player_id == pid.0) {
                let position = match Position::from_api_code(&bio.position_code) {
                    Some(p) if !matches!(p, Position::Goalie) => p,
                    _ => continue,
                };
                if !identity_inserted {
                    let identity = build_identity(pid, bio);
                    repo.upsert_identity(identity)?;
                    identity_inserted = true;
                }

                let po_stats_vec = bundled::get_playoff_stats(season_id).unwrap_or_default();
                let mut stats_row_owned =
                    po_stats_vec.iter().find(|s| s.player_id == pid.0).cloned();
                // Playoff stats bundles don't carry teamAbbrevs (verified
                // 2026-05-04 against bundled JSON). Borrow the team from
                // the same year's REGULAR stats — Stanley Cup playoffs
                // always use the team that ended the regular season, so
                // this is correct even for traded players. If both rows
                // are missing teamAbbrevs we fall through to the bio's
                // current_team_abbrev (the historical-wrong fallback).
                if let Some(po_row) = stats_row_owned.as_mut() {
                    if po_row.team_abbrevs.is_none() {
                        let reg_stats_vec = bundled::get_stats(season_id).unwrap_or_default();
                        if let Some(reg_row) = reg_stats_vec.iter().find(|s| s.player_id == pid.0) {
                            // For multi-team trades, the playoff team is
                            // the LAST entry in the regular-season list.
                            po_row.team_abbrevs = reg_row
                                .team_abbrevs
                                .as_ref()
                                .map(|s| s.split(',').next_back().unwrap_or("").trim().to_owned());
                        }
                    }
                }
                let stats = build_skater_stats(
                    pid,
                    season,
                    SeasonType::Playoff,
                    position,
                    bio,
                    stats_row_owned.as_ref(),
                    None,
                    None,
                );
                repo.upsert_stats(stats)?;
                inserted += 1;
            }
        }
    }

    Ok(inserted)
}

// ── Mappers ─────────────────────────────────────────────────────────────────

fn build_identity(pid: PlayerId, bio: &SkaterBio) -> PlayerIdentity {
    PlayerIdentity {
        id: pid,
        full_name: bio.skater_full_name.clone(),
        name_normalized: normalize_name(&bio.skater_full_name),
        headshot_canonical_url: Some(format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            pid.0
        )),
        bio: PlayerBio {
            birth_date: bio.birth_date.clone(),
            birth_country: bio.birth_country.clone(),
            nationality_code: bio.nationality_code.clone(),
            birth_city: bio.birth_city.clone(),
            birth_state_province: bio.birth_state_province_code.clone(),
            height_in_inches: bio.height,
            weight_lbs: bio.weight,
            draft_year: bio.draft_year.map(|v| v as u16),
            draft_round: bio.draft_round.map(|v| v as u8),
            draft_overall: bio.draft_overall.map(|v| v as u16),
            shoots_catches: bio.shoots_catches.clone(),
            rookie_season: bio.first_season_for_game_type.map(|s| s.to_string()),
        },
    }
}

fn build_goalie_identity(g: &GoalieStats) -> PlayerIdentity {
    // **Sparse path** — used only when `/goalie/bios` data isn't loaded.
    // Pre-Lindsay this was the only goalie-identity source. Phase Lindsay
    // L.2.6 introduces `merge_goalie_bios_into_identity` for the full-data
    // path that pulls birth/draft/height/weight from the dedicated
    // `goalie_bios` substruct on `SeasonStats`. When both are available,
    // the adapter overwrites the sparse fields below with goalie/bios
    // data — see `merge_goalie_bios_into_identity`.
    PlayerIdentity {
        id: PlayerId(g.player_id),
        full_name: g.goalie_full_name.clone(),
        name_normalized: normalize_name(&g.goalie_full_name),
        headshot_canonical_url: Some(format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            g.player_id
        )),
        bio: PlayerBio {
            shoots_catches: g.shoots_catches.clone(),
            ..Default::default()
        },
    }
}

/// Phase Lindsay L.2.6 — goalie bios merge adapter.
///
/// Pre-Lindsay the goalie identity path used `skater/bios` as a fallback
/// — wrong shape (no goalie position context, missing
/// `firstSeasonForGameType` in goalie semantics, `shoots_catches` field
/// reused with skater meaning). Lindsay introduces the dedicated
/// `goalie/bios` endpoint and a typed `GoalieBios` substruct on
/// `SeasonStats`. This adapter merges that substruct INTO an existing
/// (likely-sparse) `PlayerIdentity`, producing a full bio.
///
/// **Field-mapping table** (per L-B4 spec):
///
/// | source on `GoalieBios` | target on `PlayerIdentity.bio` | notes |
/// |---|---|---|
/// | `birth_date` | `birth_date` | passthrough |
/// | `birth_country_code` | `birth_country` | rename |
/// | `birth_city` | `birth_city` | passthrough |
/// | `nationality_code` | `nationality_code` | passthrough |
/// | `height_in_inches` | `height_in_inches` | passthrough |
/// | `weight_in_pounds` | `weight_lbs` | rename to match Hart `PlayerBio` |
/// | `shoots_catches` | `shoots_catches` | passthrough; "L"/"R" are goalie catches, NOT skater shoots |
/// | `draft_year`/`round`/`overall` | parse `Option<String>` → `Option<u16>/u8/u16` | API emits strings (pre-1979 non-numeric); parser drops non-numeric values |
/// | `first_season_for_game_type` | (no Hart `PlayerBio` field today) | dropped — future field if needed |
///
/// **Read-only on the input** — takes `&GoalieBios`, returns a new
/// `PlayerIdentity` with bio merged. Existing identity fields (`id`,
/// `full_name`) are preserved from the input identity.
pub fn merge_goalie_bios_into_identity(
    base: &PlayerIdentity,
    bios: &icelines_core::season_stats::GoalieBios,
) -> PlayerIdentity {
    PlayerIdentity {
        id: base.id,
        full_name: base.full_name.clone(),
        name_normalized: base.name_normalized.clone(),
        headshot_canonical_url: base.headshot_canonical_url.clone(),
        bio: PlayerBio {
            // Bios fields — overwrite from the goalie/bios source when
            // present, fall back to base when absent.
            birth_date: bios
                .birth_date
                .clone()
                .or_else(|| base.bio.birth_date.clone()),
            birth_country: bios
                .birth_country_code
                .clone()
                .or_else(|| base.bio.birth_country.clone()),
            birth_city: bios
                .birth_city
                .clone()
                .or_else(|| base.bio.birth_city.clone()),
            nationality_code: bios
                .nationality_code
                .clone()
                .or_else(|| base.bio.nationality_code.clone()),
            height_in_inches: bios.height_in_inches.or(base.bio.height_in_inches),
            weight_lbs: bios.weight_in_pounds.or(base.bio.weight_lbs),
            shoots_catches: bios
                .shoots_catches
                .clone()
                .or_else(|| base.bio.shoots_catches.clone()),
            // Draft fields — API emits as String; parse to numeric.
            // Non-numeric values (pre-1979 "Undrafted") drop to None.
            draft_year: bios
                .draft_year
                .as_ref()
                .and_then(|s| s.parse::<u16>().ok())
                .or(base.bio.draft_year),
            draft_round: bios
                .draft_round
                .as_ref()
                .and_then(|s| s.parse::<u8>().ok())
                .or(base.bio.draft_round),
            draft_overall: bios
                .draft_overall
                .as_ref()
                .and_then(|s| s.parse::<u16>().ok())
                .or(base.bio.draft_overall),
            // Other Hart `PlayerBio` fields — preserve from base.
            // (`birth_state_province`, `rookie_season` aren't on
            // `GoalieBios`; goalie `first_season_for_game_type` isn't a
            // Hart bio field today.)
            ..base.bio.clone()
        },
    }
}

#[allow(clippy::too_many_arguments)] // 8 inputs by design — bio + 4 sources + 3 keys
fn build_skater_stats(
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
    position: Position,
    bio: &SkaterBio,
    stats: Option<&SkaterStats>,
    realtime: Option<&SkaterRealtime>,
    mp: Option<&MoneyPuckStats>,
) -> icelines_core::season_stats::SeasonStats {
    // Field-for-field parity with the OLD `make_player` path so the
    // parallel-run field-parity test holds.
    let goals = stats.map(|s| s.goals).unwrap_or(bio.goals);
    let assists = stats.map(|s| s.assists).unwrap_or(bio.assists);
    let gp = stats.map(|s| s.games_played).unwrap_or(bio.games_played);
    let pp_goals = stats.map(|s| s.pp_goals).unwrap_or(0);
    let pp_points = stats.map(|s| s.pp_points).unwrap_or(0);
    let sh_goals = stats.map(|s| s.sh_goals).unwrap_or(0);
    let sh_points = stats.map(|s| s.sh_points).unwrap_or(0);
    let gwg = stats.map(|s| s.game_winning_goals).unwrap_or(0);
    let ot_goals = stats.map(|s| s.ot_goals).unwrap_or(0);
    let plus_minus = stats.map(|s| s.plus_minus).unwrap_or(0);
    let shots = stats.map(|s| s.shots).unwrap_or(0);
    let shooting_pct = stats.and_then(|s| s.shooting_pctg);
    let toi_per_game_sec = stats.and_then(|s| s.time_on_ice_per_game).map(|v| v as u32);
    let faceoff_win_pct = stats.and_then(|s| s.faceoff_win_pct);

    let totals = StatTotals {
        gp,
        goals,
        assists,
        points: goals + assists,
        plus_minus,
        // L.7a — `realtime.pim` is Option<u32> after the upstream API
        // removed pim from the realtime endpoint (PIM now lives only on
        // `/skater/summary`). Bind via `and_then` so a missing value
        // collapses to 0; downstream summary-merge can replace this once
        // the merge path lands.
        pim: realtime.and_then(|r| r.pim).unwrap_or(0),
        shots,
        shooting_pct,
        toi_per_game_sec,
        pp_goals,
        pp_points,
        sh_goals,
        sh_points,
        gwg,
        ot_goals,
        faceoff_win_pct,
        pace_score: compute_pace_score(goals, assists, gp),
    };

    // Per-season team resolution (UX bug fix 2026-05-04):
    // `bio.current_team_abbrev` is the player's CURRENT team regardless of
    // which season the bio row belongs to — so e.g. Tye Kartye's 2024-25
    // bundled bio reports NYR even though he played that whole season for
    // SEA. The per-season `stats.team_abbrevs` field carries the actual
    // historical team(s) for the season ("SEA" or "SEA,NYR" mid-season
    // trade). Prefer it whenever it's present.
    //
    // For multi-team rows we synthesize a SINGLE TeamStint per season
    // because the bundled feed doesn't break out date ranges. The
    // career-table renderer formats the comma-separated abbrev as-is
    // (e.g. "SEA/NYR"). True per-stint splits would need the roster
    // history endpoint and is out of scope for this fix.
    let team_str = stats
        .and_then(|s| s.team_abbrevs.as_deref())
        .filter(|t| !t.is_empty())
        .map(|t| t.replace(',', "/"))
        .unwrap_or_else(|| {
            bio.current_team_abbrev
                .as_deref()
                .unwrap_or("RET")
                .to_owned()
        });
    let stint = TeamStint {
        team: TeamAbbr(team_str),
        started: None,
        ended: None,
        gp,
        goals,
        assists,
        points: goals + assists,
        goalie: None,
    };

    let mut builder = SeasonStatsBuilder::new(pid, season, season_type, position)
        .with_totals(totals)
        .add_team_stint(stint);

    if let Some(rt) = realtime {
        builder = builder.with_realtime(RealtimeStats {
            hits: rt.hits,
            blocked_shots: rt.blocked_shots,
            takeaways: rt.takeaways,
            giveaways: rt.giveaways,
            missed_shots: rt.missed_shots,
        });
    }
    if let Some(m) = mp {
        builder = builder.with_advanced(AdvancedStats {
            xg: Some(m.xg_all as f64),
            xg_per_60: Some(m.xg_per_60 as f64),
            cf_pct: Some(m.cf_pct_5v5 as f64),
            ff_pct: Some(m.ff_pct_5v5 as f64),
            xgf_pct: Some(m.xgf_pct_5v5 as f64),
        });
    }

    builder.build()
}

fn build_goalie_season_stats(
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
    g: &GoalieStats,
) -> icelines_core::season_stats::SeasonStats {
    // The legacy goalie row carries `team_abbrevs` as a comma-separated
    // string for traded goalies (e.g. "BOS,OTT"). For Hart.3 we synthesize
    // one TeamStint per token; per-stint goalie counts (W/L/GS split by
    // team) are NOT in the bundled data — only the season-aggregate
    // GoalieSeasonStats has them. Hart.6 captures real per-stint history.
    let teams: Vec<&str> = g
        .team_abbrevs
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    let n = teams.len().max(1) as u32;

    let stints: Vec<TeamStint> = if teams.is_empty() {
        vec![TeamStint {
            team: TeamAbbr("RET".into()),
            started: None,
            ended: None,
            gp: g.games_played,
            goals: g.goals,
            assists: g.assists,
            points: g.points,
            goalie: None,
        }]
    } else {
        // Roughly equal split — sum-equals invariant on (gp, goals,
        // assists, points). The remainder lands on the LAST stint so
        // current-home semantics stay correct.
        //
        // FORGE: synthesize monotonically-increasing `started` strings
        // ("AAAA-01", "AAAA-02", …) so the builder's stint sort
        // preserves chronological insertion order. `team_abbrevs` is
        // documented chronological (legacy schema.rs comment) — without
        // synthetic dates the (None, None) sort tiebreak goes
        // alphabetical, flipping `last()` for traded goalies whose
        // chronological order is non-alphabetical (e.g. "OTT,BOS"
        // would sort to [BOS, OTT] and report BOS as the destination
        // when OTT was). Hart.6 replaces with real start/end dates.
        teams
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let is_last = i == teams.len() - 1;
                let take_n = |total: u32| -> u32 {
                    if is_last {
                        total - (total / n) * (n - 1)
                    } else {
                        total / n
                    }
                };
                TeamStint {
                    team: TeamAbbr((*t).to_owned()),
                    started: Some(format!("{SYNTHETIC_DATE_PREFIX}-{:02}", i + 1)),
                    ended: None,
                    gp: take_n(g.games_played),
                    goals: take_n(g.goals),
                    assists: take_n(g.assists),
                    points: take_n(g.points),
                    goalie: None,
                }
            })
            .collect()
    };

    let totals = StatTotals {
        gp: g.games_played,
        goals: g.goals,
        assists: g.assists,
        points: g.points,
        plus_minus: 0,
        pim: g.penalty_minutes,
        shots: 0,
        shooting_pct: None,
        toi_per_game_sec: g.time_on_ice.checked_div(g.games_played),
        pp_goals: 0,
        pp_points: 0,
        sh_goals: 0,
        sh_points: 0,
        gwg: 0,
        ot_goals: 0,
        faceoff_win_pct: None,
        pace_score: None,
    };

    let goalie = GoalieSeasonStats {
        games_started: g.games_started,
        wins: g.wins,
        losses: g.losses,
        ot_losses: g.ot_losses,
        ties: g.ties,
        shots_against: g.shots_against,
        goals_against: g.goals_against,
        saves: g.saves,
        save_pct: g.save_pct,
        goals_against_average: g.goals_against_average,
        shutouts: g.shutouts,
        time_on_ice_sec: g.time_on_ice,
    };

    SeasonStatsBuilder::new(pid, season, season_type, Position::Goalie)
        .with_totals(totals)
        .replace_team_stints(stints)
        .with_goalie(goalie)
        .build()
}

fn build_contract(c: &LegacyContract) -> PlayerContract {
    PlayerContract {
        expiry_year: c.expiry_year,
        expiry_type: c.expiry_type.clone(),
        salary: c.salary,
    }
}

fn now_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_hart3_load_error_repo_wraps_repo_error() {
        let inner = RepoError::StatsWithoutIdentity {
            id: PlayerId(1),
            season: Season(20232024),
            season_type: SeasonType::Regular,
        };
        let outer: LoadError = inner.into();
        assert!(matches!(outer, LoadError::Repo(_)));
    }

    #[test]
    fn l0_hart3_missing_source_partial_eq_all_variants() {
        let realtime = MissingSource::Realtime {
            season: "20242025".into(),
            season_type: SeasonType::Regular,
            reason: "x".into(),
        };
        let mp = MissingSource::MoneyPuck {
            season: "20242025".into(),
            reason: "x".into(),
        };
        let contracts = MissingSource::Contracts { reason: "x".into() };
        let goalie = MissingSource::GoalieStats {
            season: "20242025".into(),
            season_type: SeasonType::Regular,
            reason: "x".into(),
        };
        assert_eq!(realtime.clone(), realtime);
        assert_eq!(mp.clone(), mp);
        assert_eq!(contracts.clone(), contracts);
        assert_eq!(goalie.clone(), goalie);
        assert_ne!(realtime, mp.clone());
    }

    /// Hart.6.3 — re-points the original Hart.3 fence at 2025-26 (Cup
    /// not yet contested → ships as `[]` → loader still returns
    /// MissingBundle{Playoff}). Other 4 bundled seasons now load real
    /// playoff data; that path is fenced in
    /// `l1_load_into_repo_playoff_succeeds_when_tier_file_present`
    /// and the per-season Hart.6.3 bundled-data tests.
    #[test]
    fn l0_hart6_3_playoff_returns_missing_bundle_for_2025_26_until_cup_contested() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SnapshotStore::new(dir.path());
        let err = load_into_repo(Season(20252026), SeasonType::Playoff, &store).unwrap_err();
        match err {
            LoadError::MissingBundle { season_type, .. } => {
                assert_eq!(season_type, SeasonType::Playoff);
            }
            other => panic!("expected MissingBundle, got {other:?}"),
        }
        // WIRE: Display must use lowercase "playoff", not Debug "Playoff".
        let err = load_into_repo(Season(20252026), SeasonType::Playoff, &store).unwrap_err();
        let s = err.to_string();
        assert!(s.contains("playoff"), "Display should use lowercase: {s}");
        assert!(!s.contains("Playoff"), "Display must not leak Debug: {s}");
    }

    /// Hart.4.1 v0.2 Gap F: BENCH-mandated mid-playoff goalie trade
    /// synthetic. L0 inside stats_loader.rs so we can exercise
    /// `build_goalie_season_stats` directly without pub-ing it
    /// (FORGE #2 — keeping the loader's public surface narrow).
    /// Sum-equals invariant: per-stint gp/wins/losses == aggregate.
    #[test]
    fn l0_hart4_1_mid_playoff_goalie_trade_synthetic() {
        let g = GoalieStats {
            player_id: 9000001,
            goalie_full_name: "Test Goalie".into(),
            last_name: "Goalie".into(),
            team_abbrevs: "BOS,FLA".into(),
            season_id: 20232024,
            shoots_catches: Some("L".into()),
            games_played: 10,
            games_started: 10,
            wins: 5,
            losses: 5,
            ot_losses: Some(0),
            ties: None,
            shots_against: 280,
            goals_against: 28,
            saves: 252,
            save_pct: Some(0.900),
            goals_against_average: Some(2.80),
            shutouts: 0,
            time_on_ice: 600 * 60,
            goals: 0,
            assists: 0,
            points: 0,
            penalty_minutes: 4,
        };
        let stats =
            build_goalie_season_stats(PlayerId(9000001), Season(20232024), SeasonType::Playoff, &g);

        // Insertion order preserved through the builder sort because of
        // the SYNTHETIC_DATE_PREFIX-prefixed `started` strings (Hart.3.1
        // fix). BOS first chronologically per the legacy team_abbrevs
        // convention ("BOS,FLA" = origin first).
        assert_eq!(stats.team_stints.len(), 2, "two stints from team_abbrevs");
        assert_eq!(
            stats.team_stints[0].team.as_str(),
            "BOS",
            "origin team first"
        );
        assert_eq!(
            stats.team_stints[1].team.as_str(),
            "FLA",
            "destination team last"
        );

        // Sum-equals on counters that the goalie split applies to.
        let stint_gp: u32 = stats.team_stints.iter().map(|s| s.gp).sum();
        assert_eq!(stint_gp, stats.totals.gp, "stint gp sum != totals.gp");
        // Goalie aggregate is one struct (NOT split per-stint in the
        // legacy data); the assertion is on the SeasonStats total, not
        // on per-stint goalie counts (those are None for the loader-
        // synthesized stints — Hart.6 captures real per-stint data).
        let goalie = stats.goalie.as_ref().expect("goalie row populated");
        assert_eq!(goalie.games_started, 10);
        assert_eq!(goalie.wins, 5);
        assert_eq!(goalie.losses, 5);
        assert!(stats.is_goalie());
    }

    // ─── Phase Lindsay L.2.6 — goalie bios merge adapter ──────────────────

    #[test]
    fn l0_lindsay_merge_goalie_bios_full_data_path() {
        use icelines_core::season_stats::GoalieBios;
        let base = PlayerIdentity {
            id: PlayerId(8476434),
            full_name: "Bob Goalie".into(),
            name_normalized: "bob goalie".into(),
            headshot_canonical_url: Some("https://example.test/8476434.png".into()),
            bio: PlayerBio {
                shoots_catches: Some("L".into()),
                ..Default::default()
            },
        };
        let bios = GoalieBios {
            birth_city: Some("Helsinki".into()),
            birth_country_code: Some("FIN".into()),
            birth_date: Some("1989-09-04".into()),
            current_team_abbrev: Some("FLA".into()),
            draft_overall: Some("11".into()),
            draft_round: Some("1".into()),
            draft_year: Some("2007".into()),
            first_season_for_game_type: Some(20132014),
            height_in_centimeters: Some(187),
            height_in_inches: Some(74),
            nationality_code: Some("FIN".into()),
            shoots_catches: Some("L".into()),
            weight_in_pounds: Some(196),
        };

        let merged = merge_goalie_bios_into_identity(&base, &bios);

        // Identity preserved.
        assert_eq!(merged.id, PlayerId(8476434));
        assert_eq!(merged.full_name, "Bob Goalie");
        // Bios merged in.
        assert_eq!(merged.bio.birth_city.as_deref(), Some("Helsinki"));
        assert_eq!(merged.bio.birth_country.as_deref(), Some("FIN"));
        assert_eq!(merged.bio.birth_date.as_deref(), Some("1989-09-04"));
        assert_eq!(merged.bio.nationality_code.as_deref(), Some("FIN"));
        assert_eq!(merged.bio.height_in_inches, Some(74));
        assert_eq!(merged.bio.weight_lbs, Some(196));
        assert_eq!(merged.bio.shoots_catches.as_deref(), Some("L"));
        assert_eq!(merged.bio.draft_year, Some(2007));
        assert_eq!(merged.bio.draft_round, Some(1));
        assert_eq!(merged.bio.draft_overall, Some(11));
    }

    #[test]
    fn l0_lindsay_merge_goalie_bios_pre_1979_undrafted_drops_non_numeric() {
        use icelines_core::season_stats::GoalieBios;
        let base = PlayerIdentity {
            id: PlayerId(8400000),
            full_name: "Old Goalie".into(),
            name_normalized: "old goalie".into(),
            headshot_canonical_url: None,
            bio: PlayerBio::default(),
        };
        // Pre-1979: API emits "Undrafted" for draft fields.
        let bios = GoalieBios {
            draft_overall: Some("Undrafted".into()),
            draft_round: Some("Undrafted".into()),
            draft_year: Some("Undrafted".into()),
            ..Default::default()
        };
        let merged = merge_goalie_bios_into_identity(&base, &bios);
        assert_eq!(
            merged.bio.draft_year, None,
            "non-numeric draft year drops to None, not panic"
        );
        assert_eq!(merged.bio.draft_round, None);
        assert_eq!(merged.bio.draft_overall, None);
    }

    #[test]
    fn l0_lindsay_merge_goalie_bios_falls_back_to_base_when_field_absent() {
        use icelines_core::season_stats::GoalieBios;
        let base = PlayerIdentity {
            id: PlayerId(8400000),
            full_name: "Base Goalie".into(),
            name_normalized: "base goalie".into(),
            headshot_canonical_url: None,
            bio: PlayerBio {
                birth_date: Some("1990-01-01".into()),
                weight_lbs: Some(180),
                ..Default::default()
            },
        };
        // GoalieBios mostly empty.
        let bios = GoalieBios {
            shoots_catches: Some("R".into()),
            ..Default::default()
        };
        let merged = merge_goalie_bios_into_identity(&base, &bios);
        // Base values preserved when GoalieBios is None for that field.
        assert_eq!(merged.bio.birth_date.as_deref(), Some("1990-01-01"));
        assert_eq!(merged.bio.weight_lbs, Some(180));
        // GoalieBios overrides for what it does have.
        assert_eq!(merged.bio.shoots_catches.as_deref(), Some("R"));
    }
}
