//! Historical season data bundled directly into the binary via include_bytes!().
//!
//! Five seasons ship with every icelines binary — no download required.
//! `icelines fetch all` updates the current season in ~/.icelines/snapshots/
//! and takes precedence via the normal snapshot store lookup.
//!
//! Data source: NHL API bios + summary endpoints.
//! Historical seasons are immutable — they never change after the season ends.

use crate::{
    error::FetchError,
    playoffs_bundle::PlayoffsBundle,
    schema::{GoalieStats, SkaterBio, SkaterStats},
};

// ── Embedded season data (compiled into binary at build time) ─────────────────

macro_rules! season_bytes {
    ($season:literal, $file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/seasons/",
            $season,
            "/",
            $file
        ))
    };
}

static BIOS_20252026:  &[u8] = season_bytes!("20252026", "bios.json");
static STATS_20252026: &[u8] = season_bytes!("20252026", "stats.json");

static BIOS_20242025:  &[u8] = season_bytes!("20242025", "bios.json");
static STATS_20242025: &[u8] = season_bytes!("20242025", "stats.json");

static BIOS_20232024:  &[u8] = season_bytes!("20232024", "bios.json");
static STATS_20232024: &[u8] = season_bytes!("20232024", "stats.json");

static BIOS_20222023:  &[u8] = season_bytes!("20222023", "bios.json");
static STATS_20222023: &[u8] = season_bytes!("20222023", "stats.json");

static BIOS_20212022:  &[u8] = season_bytes!("20212022", "bios.json");
static STATS_20212022: &[u8] = season_bytes!("20212022", "stats.json");

// Goalie summaries — Phase G.1. Same five seasons embedded, separate
// arrays so the bins/stats lookups stay narrow.
static GOALIES_20252026: &[u8] = season_bytes!("20252026", "goalie-stats.json");
static GOALIES_20242025: &[u8] = season_bytes!("20242025", "goalie-stats.json");
static GOALIES_20232024: &[u8] = season_bytes!("20232024", "goalie-stats.json");
static GOALIES_20222023: &[u8] = season_bytes!("20222023", "goalie-stats.json");
static GOALIES_20212022: &[u8] = season_bytes!("20212022", "goalie-stats.json");

// Transactions — Phases T.3 + T.6. All five bundled seasons captured from
// ESPN's site.api via `cargo run --example probe_espn_seasons -- --write-bundle`.
static TRANSACTIONS_20252026: &[u8] = season_bytes!("20252026", "transactions.json");
static TRANSACTIONS_20242025: &[u8] = season_bytes!("20242025", "transactions.json");
static TRANSACTIONS_20232024: &[u8] = season_bytes!("20232024", "transactions.json");
static TRANSACTIONS_20222023: &[u8] = season_bytes!("20222023", "transactions.json");
static TRANSACTIONS_20212022: &[u8] = season_bytes!("20212022", "transactions.json");

// Hart.6.3 — playoff bios + stats + goalies for the five bundled seasons.
// The 2025-26 file ships as `[]` (Cup not yet contested as of 2026-05-02);
// the load surfaces MissingBundle{Playoff} cleanly via Hart.6.4 dispatch.
// Authored 2026-05-02 by `icelines fetch stats|goalies --type playoff`
// against api.nhle.com (Hart.6.5 surface).
static PLAYOFF_BIOS_20252026:    &[u8] = season_bytes!("20252026", "playoff-bios.json");
static PLAYOFF_BIOS_20242025:    &[u8] = season_bytes!("20242025", "playoff-bios.json");
static PLAYOFF_BIOS_20232024:    &[u8] = season_bytes!("20232024", "playoff-bios.json");
static PLAYOFF_BIOS_20222023:    &[u8] = season_bytes!("20222023", "playoff-bios.json");
static PLAYOFF_BIOS_20212022:    &[u8] = season_bytes!("20212022", "playoff-bios.json");

static PLAYOFF_STATS_20252026:   &[u8] = season_bytes!("20252026", "playoff-stats.json");
static PLAYOFF_STATS_20242025:   &[u8] = season_bytes!("20242025", "playoff-stats.json");
static PLAYOFF_STATS_20232024:   &[u8] = season_bytes!("20232024", "playoff-stats.json");
static PLAYOFF_STATS_20222023:   &[u8] = season_bytes!("20222023", "playoff-stats.json");
static PLAYOFF_STATS_20212022:   &[u8] = season_bytes!("20212022", "playoff-stats.json");

static PLAYOFF_GOALIES_20252026: &[u8] = season_bytes!("20252026", "playoff-goalie-stats.json");
static PLAYOFF_GOALIES_20242025: &[u8] = season_bytes!("20242025", "playoff-goalie-stats.json");
static PLAYOFF_GOALIES_20232024: &[u8] = season_bytes!("20232024", "playoff-goalie-stats.json");
static PLAYOFF_GOALIES_20222023: &[u8] = season_bytes!("20222023", "playoff-goalie-stats.json");
static PLAYOFF_GOALIES_20212022: &[u8] = season_bytes!("20212022", "playoff-goalie-stats.json");

// ── Public API ────────────────────────────────────────────────────────────────

/// List of bundled seasons, newest first.
pub const BUNDLED_SEASONS: &[&str] = &[
    "20252026", "20242025", "20232024", "20222023", "20212022",
];

/// Phase Lindsay L.1.4 — bundled fallback for the new Tier-1 reports
/// (timeonice, goalsForAgainst, goalie-advanced, goalie-savesByStrength,
/// goalie-bios). Returns `None` for every kind today; **L.7** populates
/// the include_bytes! map when the 38 historical seasons get bundled.
///
/// The slot exists at L.1.4 so `load_report_with_fallback` doesn't
/// change shape between L.1 and L.7. Adding a bundled report later is
/// one match arm + one `include_bytes!` in this function.
///
/// Returns the raw JSON bytes of `{"data": [...], "total": N}` for
/// (season, season_type, kind), or `None` when not bundled.
pub fn report_for_lindsay(
    _season: &str,
    _season_type: icelines_core::season_stats::SeasonType,
    _kind: icelines_core::stats_catalog::ReportKind,
) -> Option<Vec<u8>> {
    // L.7 will replace this with a per-(season, season_type, kind)
    // dispatch returning `Some(include_bytes!(…).to_vec())`.
    None
}

/// Deserialize bundled bios for a season. Returns None if season not bundled.
pub fn get_bios(season: &str) -> Option<Vec<SkaterBio>> {
    let bytes = match season {
        "20252026" => BIOS_20252026,
        "20242025" => BIOS_20242025,
        "20232024" => BIOS_20232024,
        "20222023" => BIOS_20222023,
        "20212022" => BIOS_20212022,
        _          => return None,
    };
    serde_json::from_slice(bytes).ok()
}

/// Deserialize bundled stats for a season. Returns None if season not bundled.
pub fn get_stats(season: &str) -> Option<Vec<SkaterStats>> {
    let bytes = match season {
        "20252026" => STATS_20252026,
        "20242025" => STATS_20242025,
        "20232024" => STATS_20232024,
        "20222023" => STATS_20222023,
        "20212022" => STATS_20212022,
        _          => return None,
    };
    serde_json::from_slice(bytes).ok()
}

/// Deserialize bundled goalie stats for a season (Phase G.1). Returns
/// None when the season isn't one of the five embedded current seasons.
/// Use `get_goalie_stats_installed` to read from `~/.icelines/seasons/`
/// for historical seasons that were brought in via `data install`.
pub fn get_goalie_stats(season: &str) -> Option<Vec<GoalieStats>> {
    let bytes = match season {
        "20252026" => GOALIES_20252026,
        "20242025" => GOALIES_20242025,
        "20232024" => GOALIES_20232024,
        "20222023" => GOALIES_20222023,
        "20212022" => GOALIES_20212022,
        _          => return None,
    };
    serde_json::from_slice(bytes).ok()
}

/// Read goalie stats from an installed season bundle. Returns None when
/// the bundle is not installed (~/.icelines/seasons/...) or pre-dates
/// G.0's bundling of `goalie-stats.json` into release tarballs.
pub fn get_goalie_stats_installed(season_id: &str) -> Option<Vec<GoalieStats>> {
    let path = season_bundle_dir(season_id)?.join("goalie-stats.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

// ── Transactions (Phase T.3) ─────────────────────────────────────────────────

/// On-disk envelope for `transactions.json`. Includes provenance
/// (`source`, `fetched_at`, `classifier_version`) so a stale snapshot
/// can be re-classified on load without re-fetching.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionsEnvelope {
    pub season:             String,
    pub source:             String,
    pub fetched_at:         String,
    pub classifier_version: u16,
    pub rows:               Vec<icelines_core::Transaction>,
}

/// Read embedded transactions for a bundled season. Returns None for any
/// season not in the include_bytes! set.
pub fn get_transactions(season: &str) -> Option<TransactionsEnvelope> {
    let bytes = match season {
        "20252026" => TRANSACTIONS_20252026,
        "20242025" => TRANSACTIONS_20242025,
        "20232024" => TRANSACTIONS_20232024,
        "20222023" => TRANSACTIONS_20222023,
        "20212022" => TRANSACTIONS_20212022,
        _          => return None,
    };
    serde_json::from_slice(bytes).ok()
}

/// Read transactions from an installed season bundle (~/.icelines/seasons/...).
pub fn get_transactions_installed(season_id: &str) -> Option<TransactionsEnvelope> {
    let path = season_bundle_dir(season_id)?.join("transactions.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Resolve transactions: legacy snapshot → embedded → installed bundle.
/// On load, any row whose `classifier_version < CURRENT_CLASSIFIER_VERSION`
/// is re-classified against `description` so bundled snapshots and live
/// data never disagree on `kind`. Forward-compat: rows with a higher
/// version are left alone.
pub fn load_transactions_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<TransactionsEnvelope, FetchError> {
    use icelines_core::transactions::CURRENT_CLASSIFIER_VERSION;

    let mut envelope = if let Ok(env) = store.read_tier::<TransactionsEnvelope>(
        &crate::snapshot::SnapshotTier::Stats, "transactions.json",
    ) {
        env
    } else if let Some(env) = get_transactions(season) {
        env
    } else if let Some(env) = get_transactions_installed(season) {
        env
    } else {
        return Err(FetchError::PlayerNotFound {
            name: format!("no transactions for season {season} — run `icelines fetch transactions`"),
        });
    };

    // Re-classification on stale envelope. Only when ALL rows look stale
    // (`envelope.classifier_version < CURRENT`) — that's the fast path.
    // Mixed-version envelopes (some rows from a partial fetch) re-classify
    // per row.
    if envelope.classifier_version < CURRENT_CLASSIFIER_VERSION {
        for row in &mut envelope.rows {
            if row.classifier_version < CURRENT_CLASSIFIER_VERSION {
                row.kind = icelines_core::classify(&row.description);
                row.classifier_version = CURRENT_CLASSIFIER_VERSION;
            }
        }
        envelope.classifier_version = CURRENT_CLASSIFIER_VERSION;
    }

    Ok(envelope)
}

/// Resolve goalie stats: chunked snapshot → legacy snapshot → embedded
/// → installed bundle. Mirrors `load_bios_with_fallback` / `load_stats_*`
/// for parity. The snapshot tier path lands when G.2 wires
/// `fetch goalies` to write a goalie-stats tier.
pub fn load_goalies_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<GoalieStats>, FetchError> {
    // 1. Legacy file-per-tier active snapshot (chunked path lands in G.2+).
    if let Ok(rows) = store.read_tier::<Vec<GoalieStats>>(
        &crate::snapshot::SnapshotTier::Stats, "goalie-stats.json",
    ) {
        return Ok(rows);
    }
    // 2. Bundled data.
    if let Some(rows) = get_goalie_stats(season) { return Ok(rows); }
    // 3. Installed (historical) bundle.
    if let Some(rows) = get_goalie_stats_installed(season) { return Ok(rows); }
    Err(FetchError::PlayerNotFound {
        name: format!("no goalie stats for season {season} — run `icelines fetch goalies`"),
    })
}

// ── Installed season data (from ~/.icelines/seasons/) ────────────────────────

/// Returns the path to a season's bundle directory, or None if home can't be determined.
fn season_bundle_dir(season_id: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    Some(std::path::Path::new(&home)
        .join(".icelines")
        .join("seasons")
        .join(season_id)
        .join(format!("bundle-{season_id}")))
}

/// Returns true if a season has been installed to disk.
pub fn is_installed(season_id: &str) -> bool {
    season_bundle_dir(season_id)
        .map(|d| d.join("bios.json").exists())
        .unwrap_or(false)
}

/// Read bios from an installed season bundle. Returns None if not installed.
pub fn get_bios_installed(season_id: &str) -> Option<Vec<crate::schema::SkaterBio>> {
    let path = season_bundle_dir(season_id)?.join("bios.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Read stats from an installed season bundle. Returns None if not installed.
pub fn get_stats_installed(season_id: &str) -> Option<Vec<crate::schema::SkaterStats>> {
    let path = season_bundle_dir(season_id)?.join("stats.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

// ── Hart.6.2 — playoff installed-bundle accessors ───────────────────────────
//
// Mirror chain of `get_bios_installed` / `get_stats_installed` /
// `get_goalie_stats_installed` for the playoff variant. Reads from the
// `playoff-bios.json` / `playoff-stats.json` / `playoff-goalie-stats.json`
// files inside the installed bundle directory. Pre-Hart.6 installed
// bundles don't carry these files; consumers see `None` and fall through.

/// Read playoff bios from an installed season bundle. Returns None if
/// the bundle isn't installed or the file isn't present.
pub fn get_playoff_bios_installed(season_id: &str) -> Option<Vec<crate::schema::SkaterBio>> {
    let path = season_bundle_dir(season_id)?.join("playoff-bios.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Read playoff stats from an installed season bundle. Returns None
/// if the bundle isn't installed or the file isn't present.
pub fn get_playoff_stats_installed(
    season_id: &str,
) -> Option<Vec<crate::schema::SkaterStats>> {
    let path = season_bundle_dir(season_id)?.join("playoff-stats.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Read playoff goalie stats from an installed season bundle.
pub fn get_playoff_goalie_stats_installed(
    season_id: &str,
) -> Option<Vec<GoalieStats>> {
    let path = season_bundle_dir(season_id)?.join("playoff-goalie-stats.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

// ── Hart.6.3 — playoff embedded-bundle accessors ────────────────────────────
//
// Replaces the Hart.6.2 stubs. Each bundled season carries playoff data
// authored 2026-05-02 by `icelines fetch stats|goalies --type playoff`.
// 2025-26 ships as `[]` (Cup not yet contested) — Hart.6.4 dispatch
// converts an empty-bios result to MissingBundle{Playoff} so the TUI /
// CLI can surface a clean "playoffs haven't started" banner.

/// Playoff bios for a bundled season. Returns `None` if the season
/// isn't in `BUNDLED_SEASONS`; otherwise the embedded array (which may
/// be empty for current-season-not-yet-played).
pub fn get_playoff_bios(season_id: &str) -> Option<Vec<crate::schema::SkaterBio>> {
    let bytes = match season_id {
        "20252026" => PLAYOFF_BIOS_20252026,
        "20242025" => PLAYOFF_BIOS_20242025,
        "20232024" => PLAYOFF_BIOS_20232024,
        "20222023" => PLAYOFF_BIOS_20222023,
        "20212022" => PLAYOFF_BIOS_20212022,
        _          => return None,
    };
    serde_json::from_slice(bytes).ok()
}

/// Playoff stats for a bundled season. See `get_playoff_bios` for
/// semantics; same `BUNDLED_SEASONS` membership rule applies.
pub fn get_playoff_stats(season_id: &str) -> Option<Vec<crate::schema::SkaterStats>> {
    let bytes = match season_id {
        "20252026" => PLAYOFF_STATS_20252026,
        "20242025" => PLAYOFF_STATS_20242025,
        "20232024" => PLAYOFF_STATS_20232024,
        "20222023" => PLAYOFF_STATS_20222023,
        "20212022" => PLAYOFF_STATS_20212022,
        _          => return None,
    };
    serde_json::from_slice(bytes).ok()
}

/// Playoff goalie stats for a bundled season.
pub fn get_playoff_goalie_stats(season_id: &str) -> Option<Vec<GoalieStats>> {
    let bytes = match season_id {
        "20252026" => PLAYOFF_GOALIES_20252026,
        "20242025" => PLAYOFF_GOALIES_20242025,
        "20232024" => PLAYOFF_GOALIES_20232024,
        "20222023" => PLAYOFF_GOALIES_20222023,
        "20212022" => PLAYOFF_GOALIES_20212022,
        _          => return None,
    };
    serde_json::from_slice(bytes).ok()
}

// ── Historical playoffs (Phase 8c) ───────────────────────────────────────────

/// Embedded `playoffs.json` files. Each entry is `(season_id, &[u8])`. Add new
/// historical seasons here as their bundles are authored. The 1993-94 NYR Cup
/// run is the canonical first fixture per `design/specs/playoffs.md`.
static BUNDLED_PLAYOFFS: &[(&str, &[u8])] = &[
    ("19931994", include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/seasons/19931994/playoffs.json"
    ))),
];

/// List of seasons with bundled playoff data.
pub fn bundled_playoff_seasons() -> Vec<&'static str> {
    BUNDLED_PLAYOFFS.iter().map(|(s, _)| *s).collect()
}

/// Deserialize bundled `playoffs.json` for a season. Returns None if no
/// bundle has been authored for that season yet.
pub fn get_playoffs(season_id: &str) -> Option<PlayoffsBundle> {
    let bytes = BUNDLED_PLAYOFFS.iter()
        .find_map(|(s, b)| (*s == season_id).then_some(*b))?;
    serde_json::from_slice(bytes).ok()
}

/// Read `playoffs.json` from an installed season bundle in the user's
/// `~/.icelines/seasons/` directory. Returns `None` when the bundle is not
/// installed or does not include a playoffs file. Takes precedence over
/// `get_playoffs` when both are available — installed bundles can be updated
/// without rebuilding the binary.
pub fn get_playoffs_installed(season_id: &str) -> Option<PlayoffsBundle> {
    let path = season_bundle_dir(season_id)?.join("playoffs.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Resolve `playoffs.json` for a season. Prefers an installed bundle (so users
/// can refresh historical data without rebuilding) and falls back to the
/// binary-embedded version.
pub fn load_playoffs(season_id: &str) -> Option<PlayoffsBundle> {
    get_playoffs_installed(season_id).or_else(|| get_playoffs(season_id))
}

/// Load bios: try the snapshot store first, falling back to bundled data.
///
/// Resolution order (Phase 8h):
/// 1. Active snapshot — chunked layout (`chunked.json`) if present
/// 2. Active snapshot — legacy `stats/bios.json`
/// 3. Bundled data shipped with the binary
pub fn load_bios_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<SkaterBio>, FetchError> {
    // 1. Chunked active snapshot
    if let Ok((bios, _)) = read_chunked_active(store) {
        return Ok(bios);
    }
    // 2. Legacy file-per-tier active snapshot
    if let Ok(bios) = store.read_tier(&crate::snapshot::SnapshotTier::Stats, "bios.json") {
        return Ok(bios);
    }
    // 3. Bundled data shipped with binary
    get_bios(season).ok_or_else(|| FetchError::PlayerNotFound {
        name: format!("no bios for season {season} — run `icelines fetch stats`"),
    })
}

/// Load stats: try the snapshot store first, falling back to bundled data.
/// See `load_bios_with_fallback` for the full resolution order.
pub fn load_stats_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<SkaterStats>, FetchError> {
    // 1. Chunked active snapshot
    if let Ok((_, stats)) = read_chunked_active(store) {
        return Ok(stats);
    }
    // 2. Legacy file-per-tier active snapshot
    if let Ok(stats) = store.read_tier(&crate::snapshot::SnapshotTier::Stats, "stats.json") {
        return Ok(stats);
    }
    get_stats(season).ok_or_else(|| FetchError::PlayerNotFound {
        name: format!("no stats for season {season} — run `icelines fetch stats`"),
    })
}

/// Hart.6.2 — load playoff bios. Mirrors `load_bios_with_fallback`:
/// chunked active → legacy file-per-tier active → embedded bundled →
/// installed bundled. The legacy path uses `playoff-bios.json` inside
/// the existing `Stats` tier dir (D3 co-location).
///
/// Hart.6.9 — snapshot reads are now season-filtered: only snapshots
/// whose `meta.season` matches `season` are consulted, so a current-
/// season active snapshot doesn't shadow a historical-season query.
pub fn load_playoff_bios_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<SkaterBio>, FetchError> {
    if let Ok((bios, _)) = read_chunked_active_playoff_for_season(store, season) {
        return Ok(bios);
    }
    if let Ok(bios) = store.read_tier_for_season(
        &crate::snapshot::SnapshotTier::Stats,
        "playoff-bios.json",
        season,
    ) {
        return Ok(bios);
    }
    if let Some(bios) = get_playoff_bios(season) {
        return Ok(bios);
    }
    if let Some(bios) = get_playoff_bios_installed(season) {
        return Ok(bios);
    }
    Err(FetchError::PlayerNotFound {
        name: format!("no playoff bios for season {season} — run `icelines fetch stats --type playoff`"),
    })
}

/// Hart.6.2 — load playoff stats. Same chain as
/// `load_playoff_bios_with_fallback`.
pub fn load_playoff_stats_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<SkaterStats>, FetchError> {
    if let Ok((_, stats)) = read_chunked_active_playoff_for_season(store, season) {
        return Ok(stats);
    }
    if let Ok(stats) = store.read_tier_for_season(
        &crate::snapshot::SnapshotTier::Stats,
        "playoff-stats.json",
        season,
    ) {
        return Ok(stats);
    }
    if let Some(stats) = get_playoff_stats(season) {
        return Ok(stats);
    }
    if let Some(stats) = get_playoff_stats_installed(season) {
        return Ok(stats);
    }
    Err(FetchError::PlayerNotFound {
        name: format!("no playoff stats for season {season} — run `icelines fetch stats --type playoff`"),
    })
}

/// Hart.6.2 — load playoff goalie stats. Mirrors
/// `load_goalies_with_fallback`. Snapshot path uses
/// `playoff-goalie-stats.json` co-located with `goalie-stats.json`.
pub fn load_playoff_goalies_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<GoalieStats>, FetchError> {
    if let Ok(rows) = store.read_tier_for_season::<Vec<GoalieStats>>(
        &crate::snapshot::SnapshotTier::Stats,
        "playoff-goalie-stats.json",
        season,
    ) {
        return Ok(rows);
    }
    if let Some(rows) = get_playoff_goalie_stats(season) {
        return Ok(rows);
    }
    if let Some(rows) = get_playoff_goalie_stats_installed(season) {
        return Ok(rows);
    }
    Err(FetchError::PlayerNotFound {
        name: format!("no playoff goalie stats for season {season} — run `icelines fetch goalies --type playoff`"),
    })
}

/// Read both bios + stats from the active chunked snapshot, if any. Returns
/// `Err` if no snapshot is active, the active snapshot is not chunked, or
/// any chunk fails its integrity check.
fn read_chunked_active(
    store: &crate::snapshot::SnapshotStore,
) -> Result<(Vec<SkaterBio>, Vec<SkaterStats>), crate::snapshot::SnapshotError> {
    let manifest = store.load_manifest()?;
    let active = manifest
        .active
        .as_deref()
        .ok_or(crate::snapshot::SnapshotError::NoActiveSnapshot)?;
    if !store.is_chunked(active) {
        return Err(crate::snapshot::SnapshotError::NotFound { name: format!("{active}/chunked.json") });
    }
    store.read_chunked_stats(active, icelines_core::season_stats::SeasonType::Regular)
}

/// Hart.6.2 — same as `read_chunked_active` but for the playoff
/// (bios, stats) pair. Returns `NotFound` if the active snapshot was
/// written without playoff data.
/// Hart.6.9 — season-filtered. Only reads the active snapshot's chunked
/// playoff data when its `meta.season` matches `requested_season`.
fn read_chunked_active_playoff_for_season(
    store: &crate::snapshot::SnapshotStore,
    requested_season: &str,
) -> Result<(Vec<SkaterBio>, Vec<SkaterStats>), crate::snapshot::SnapshotError> {
    let manifest = store.load_manifest()?;
    let active = manifest
        .active
        .as_deref()
        .ok_or(crate::snapshot::SnapshotError::NoActiveSnapshot)?;
    let meta = store.load_meta(active)?;
    if meta.season != requested_season {
        return Err(crate::snapshot::SnapshotError::NotFound {
            name: format!(
                "{active}/chunked.json: snapshot season {} != requested {}",
                meta.season, requested_season
            ),
        });
    }
    if !store.is_chunked(active) {
        return Err(crate::snapshot::SnapshotError::NotFound { name: format!("{active}/chunked.json") });
    }
    store.read_chunked_stats(active, icelines_core::season_stats::SeasonType::Playoff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_bundled_current_season_bios_parse() {
        // Verify bundled JSON parses correctly — catches malformed data at compile time
        let result = serde_json::from_slice::<Vec<SkaterBio>>(BIOS_20252026);
        match &result {
            Err(e) => panic!("20252026 bios failed to parse: {e}"),
            Ok(bios) => {
                assert!(!bios.is_empty(), "bundled bios must not be empty");
                assert!(bios.len() > 500, "expected 900+ players, got {}", bios.len());
            }
        }
    }

    #[test]
    fn l0_bundled_current_season_stats_parse() {
        let result = serde_json::from_slice::<Vec<SkaterStats>>(STATS_20252026);
        match &result {
            Err(e) => panic!("20252026 stats failed to parse: {e}"),
            Ok(stats) => assert!(stats.len() > 500, "expected 900+ players, got {}", stats.len()),
        }
    }

    #[test]
    fn l0_bundled_historical_season_parses() {
        let bios = get_bios("20242025").expect("20242025 must be bundled");
        assert!(!bios.is_empty());
        // Each bio must have a player_id
        assert!(bios.iter().all(|b| b.player_id > 0));
    }

    #[test]
    fn l0_bundled_all_5_seasons_present() {
        for season in BUNDLED_SEASONS {
            assert!(get_bios(season).is_some(), "season {season} bios not bundled");
            assert!(get_stats(season).is_some(), "season {season} stats not bundled");
        }
    }

    #[test]
    fn l0_bundled_unknown_season_returns_none() {
        assert!(get_bios("19951996").is_none());
        assert!(get_stats("19951996").is_none());
    }

    // ── Phase 8c: bundled playoffs ─────────────────────────────────────────

    #[test]
    fn l0_bundled_playoffs_19931994_parses() {
        let b = get_playoffs("19931994").expect("19931994 must be bundled");
        assert_eq!(b.season, "19931994");
        assert_eq!(b.champion.as_deref(), Some("NYR"));
        assert_eq!(b.rounds.len(), 4);
    }

    #[test]
    fn l0_bundled_playoffs_unknown_season_returns_none() {
        assert!(get_playoffs("19951996").is_none());
    }

    #[test]
    fn l0_bundled_playoffs_19931994_cup_final_has_seven_games() {
        let b = get_playoffs("19931994").expect("19931994 bundled");
        let cup = b.rounds.iter().find(|r| r.round == 4).expect("round 4 present");
        assert_eq!(cup.series.len(), 1, "Cup Final has one series");
        assert_eq!(cup.series[0].results.len(), 7, "Cup Final ran 7 games");
        // Convert via to_bracket and verify wins were derived correctly.
        let br = b.to_bracket();
        let cup_series = &br.rounds.iter().find(|r| r.round_number == 4).unwrap().series[0];
        assert_eq!(cup_series.top_seed_wins, 4);
        assert_eq!(cup_series.bottom_seed_wins, 3);
        assert_eq!(cup_series.games.len(), 7);
    }

    #[test]
    fn l0_bundled_playoffs_load_prefers_installed_then_embedded() {
        // No installed bundle in test env → falls back to embedded.
        let b = load_playoffs("19931994").expect("must resolve");
        assert_eq!(b.season, "19931994");
    }

    // ── Hart.6.2 — playoff stub accessors ───────────────────────────────────

    /// Hart.6.3 — every bundled season's playoff bios deserialize cleanly.
    /// 4 of 5 carry real playoff data (2021-22 through 2024-25 Cup runs);
    /// 2025-26 ships as `[]` (Cup not yet contested).
    #[test]
    fn l0_hart6_3_get_playoff_bios_parses_for_all_5_bundled_seasons() {
        for season in BUNDLED_SEASONS {
            let v = get_playoff_bios(season);
            assert!(v.is_some(), "season {season} must be in the bundled set");
        }
        // 2025-26 is the deliberate empty (Cup not yet contested at
        // bundle-authoring time). Asserted explicitly so a future
        // accidental deletion of the playoff-bios.json file surfaces.
        assert!(get_playoff_bios("20252026").unwrap().is_empty(),
            "2025-26 ships as [] until the playoffs are bundled");
        // Other 4 must have real data.
        for season in ["20242025", "20232024", "20222023", "20212022"] {
            let count = get_playoff_bios(season).unwrap().len();
            assert!(count > 100,
                "{season} playoff bios must have ≥100 rows (NHL playoff rosters), got {count}");
        }
    }

    /// Hart.6.3 — every bundled season's playoff stats parse and the
    /// row count matches the bios count for that season (every player
    /// who has a bio also has a stats row).
    #[test]
    fn l0_hart6_3_get_playoff_stats_matches_bios_count_per_season() {
        for season in ["20242025", "20232024", "20222023", "20212022"] {
            let bios = get_playoff_bios(season).unwrap();
            let stats = get_playoff_stats(season).unwrap();
            assert_eq!(
                bios.len(),
                stats.len(),
                "{season} bios/stats row count mismatch — bios={}, stats={}",
                bios.len(),
                stats.len(),
            );
        }
    }

    /// Hart.6.3 — every bundled playoff stats row has its `season_id`
    /// matching the requested season. Catches a bundle-authoring bug
    /// where seasons could get crossed.
    #[test]
    fn l0_hart6_3_playoff_stats_seasonid_matches_filename() {
        for season in ["20242025", "20232024", "20222023", "20212022"] {
            let expected: u32 = season.parse().unwrap();
            let stats = get_playoff_stats(season).unwrap();
            for s in &stats {
                assert_eq!(
                    s.season_id, Some(expected),
                    "row in {season} carries seasonId={:?}; expected Some({expected})",
                    s.season_id
                );
            }
        }
    }

    /// Hart.6.3 — every bundled playoff goalie row has its `season_id`
    /// matching the requested season. (Goalie season_id is u32, not
    /// Option.)
    #[test]
    fn l0_hart6_3_playoff_goalies_seasonid_matches_filename() {
        for season in ["20242025", "20232024", "20222023", "20212022"] {
            let expected: u32 = season.parse().unwrap();
            let goalies = get_playoff_goalie_stats(season).unwrap();
            for g in &goalies {
                assert_eq!(
                    g.season_id, expected,
                    "row in {season} carries seasonId={}; expected {expected}",
                    g.season_id
                );
            }
        }
    }

    /// `load_playoff_bios_with_fallback` falls through the chain:
    /// chunked → tier file → embedded → installed. In a tempdir test
    /// env nothing is on disk, so the embedded bundle fires.
    /// Hart.6.3 — embedded data is now real (was empty in Hart.6.2),
    /// so the assertion verifies a populated vec for 2024-25.
    #[test]
    fn l0_hart6_3_load_playoff_bios_falls_through_to_embedded_bundle() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::snapshot::SnapshotStore::new(dir.path());
        let bios = load_playoff_bios_with_fallback("20242025", &store)
            .expect("bundled playoff bios must resolve");
        assert!(
            bios.len() > 100,
            "20242025 playoff bios must have ≥100 rows, got {}",
            bios.len()
        );
    }

    /// 2025-26 ships as `[]` (Cup not yet contested). The fallback chain
    /// still returns Ok(vec![]) — Hart.6.4 dispatch is what converts
    /// that to MissingBundle{Playoff} for the user-visible error.
    #[test]
    fn l0_hart6_3_load_playoff_bios_2025_26_returns_empty_until_played() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::snapshot::SnapshotStore::new(dir.path());
        let bios = load_playoff_bios_with_fallback("20252026", &store)
            .expect("2025-26 stub must resolve to Ok(empty)");
        assert!(bios.is_empty(), "2025-26 ships as [] until playoffs run");
    }

    /// `get_playoff_bios` returns `None` for any season outside the
    /// bundled set. Asserts ONLY on the in-binary stub — does not call
    /// `load_playoff_*_with_fallback` because that chain reads
    /// `$HOME/.icelines/seasons/<id>/...` for the installed-bundle
    /// step, which races with parallel tests that mutate HOME (the
    /// `with_temp_home` SQLite tests in icelines-cli).
    #[test]
    fn l0_hart6_2_get_playoff_data_returns_none_for_unbundled() {
        // 19961997 is not in BUNDLED_SEASONS — returns None.
        assert!(get_playoff_bios("19961997").is_none());
        assert!(get_playoff_stats("19961997").is_none());
        assert!(get_playoff_goalie_stats("19961997").is_none());
        // 18800000 — clearly fictitious, also None.
        assert!(get_playoff_bios("18800000").is_none());
    }

    // ── Phase Lindsay L.7 — L-B20 cross-product parse test ──────────────

    /// L-B20: every bundled (season × season_type × Tier-1 kind) cell
    /// either returns `None` (not bundled at this slot) OR returns
    /// bytes that parse as the envelope shape `{"data": [...], "total":
    /// N}`. Today the L.7 fallback returns None for every cell —
    /// test passes vacuously. The moment a `report_for_lindsay`
    /// dispatch arm lands `Some(include_bytes!(…))`, this test starts
    /// validating that the embedded bytes deserialize cleanly.
    #[test]
    fn l1_lindsay_l7_each_tier1_report_parses_for_all_bundled_seasons() {
        use icelines_core::season_stats::SeasonType;
        use icelines_core::stats_catalog::{ReportKind, Tier};

        let tier1_kinds: Vec<ReportKind> = ReportKind::all()
            .iter()
            .filter(|k| matches!(k.tier(), Tier::Tier1))
            .copied()
            .collect();
        assert!(
            tier1_kinds.len() >= 9,
            "expected ≥9 Tier-1 kinds (got {}) — catalog drift?",
            tier1_kinds.len(),
        );

        let mut some_count = 0usize;
        let mut none_count = 0usize;
        for season in BUNDLED_SEASONS {
            for st in [SeasonType::Regular, SeasonType::Playoff] {
                for kind in &tier1_kinds {
                    match report_for_lindsay(season, st, *kind) {
                        Some(bytes) => {
                            some_count += 1;
                            // Envelope shape: must parse as JSON object
                            // with `data` (array) + `total` (number).
                            let v: serde_json::Value =
                                serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                                    panic!(
                                        "L-B20: bundled bytes for ({season}, \
                                         {st:?}, {kind:?}) are not valid JSON: {e}"
                                    )
                                });
                            let obj = v.as_object().unwrap_or_else(|| {
                                panic!(
                                    "L-B20: bundled JSON for ({season}, {st:?}, \
                                     {kind:?}) is not an object — got {v:?}"
                                )
                            });
                            assert!(
                                obj.contains_key("data"),
                                "L-B20: bundled JSON for ({season}, {st:?}, \
                                 {kind:?}) missing `data` field"
                            );
                            assert!(
                                obj["data"].is_array(),
                                "L-B20: bundled `data` for ({season}, {st:?}, \
                                 {kind:?}) is not an array"
                            );
                            assert!(
                                obj.contains_key("total"),
                                "L-B20: bundled JSON for ({season}, {st:?}, \
                                 {kind:?}) missing `total` field"
                            );
                        }
                        None => none_count += 1,
                    }
                }
            }
        }

        // Sanity: total cells = BUNDLED_SEASONS × 2 × tier1_kinds.
        let expected_cells = BUNDLED_SEASONS.len() * 2 * tier1_kinds.len();
        assert_eq!(
            some_count + none_count,
            expected_cells,
            "L-B20: cell count mismatch (some={some_count}, none={none_count}, \
             expected={expected_cells})"
        );
        // No assertion on some/none ratio — today everything is None,
        // L.7 will gradually flip cells to Some as bundles land.
    }
}
