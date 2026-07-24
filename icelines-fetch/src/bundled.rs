//! Historical season data bundled directly into the binary via include_bytes!().
//!
//! All 38 NHL seasons since 1987-88 ship with every icelines binary — no
//! download required. `icelines fetch all` updates the current season in
//! `~/.icelines/snapshots/` and takes precedence via the normal snapshot
//! store lookup.
//!
//! Data sources:
//! - NHL API bios + summary endpoints (regular + playoff)
//! - Goalie summaries (Phase G.1) for all 38 seasons
//! - Transactions (Phase T.3) — modern era only (2021-22 through 2025-26)
//!   because ESPN's site.api doesn't carry pre-2021 transaction logs.
//!
//! Historical seasons are immutable — they never change after the season ends.
//!
//! The 2004-05 lockout year is intentionally absent.
//!
//! L.7b (2026-05-03) — expanded the bundle from 5 to 38 seasons via the
//! table-driven layout below. Binary grew ~23 MB → ~56 MB; closes the
//! "BUNDLED_SEASONS != all-seasons" gap that once forced `data install` for
//! historical queries. Release-backed installs are retired; source fetches now
//! refresh local snapshots.

use crate::{
    error::FetchError,
    playoffs_bundle::PlayoffsBundle,
    schema::{GoalieStats, SkaterBio, SkaterStats},
};

// ── Embedded season data (compiled into binary at build time) ─────────────────

/// Builds one (season, &[u8]) tuple for inclusion in a per-kind lookup
/// table. Used by the `BUNDLED_*` slices below — `include_bytes!` is
/// the only way to embed a literal-path file at compile time, so the
/// season list is open-coded once per kind.
macro_rules! season_entry {
    ($season:literal, $file:literal) => {
        (
            $season,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../data/seasons/",
                $season,
                "/",
                $file
            )) as &[u8],
        )
    };
}

/// Per-kind lookup tables. Newest first to match `BUNDLED_SEASONS` order
/// (which `aggregate.rs` and `dashboard_panel.rs` rely on).
///
/// 38 entries each — every season since 1987-88 except the 2004-05
/// lockout. Order must match `BUNDLED_SEASONS`.
static BUNDLED_BIOS: &[(&str, &[u8])] = &[
    season_entry!("20252026", "bios.json"),
    season_entry!("20242025", "bios.json"),
    season_entry!("20232024", "bios.json"),
    season_entry!("20222023", "bios.json"),
    season_entry!("20212022", "bios.json"),
    season_entry!("20202021", "bios.json"),
    season_entry!("20192020", "bios.json"),
    season_entry!("20182019", "bios.json"),
    season_entry!("20172018", "bios.json"),
    season_entry!("20162017", "bios.json"),
    season_entry!("20152016", "bios.json"),
    season_entry!("20142015", "bios.json"),
    season_entry!("20132014", "bios.json"),
    season_entry!("20122013", "bios.json"),
    season_entry!("20112012", "bios.json"),
    season_entry!("20102011", "bios.json"),
    season_entry!("20092010", "bios.json"),
    season_entry!("20082009", "bios.json"),
    season_entry!("20072008", "bios.json"),
    season_entry!("20062007", "bios.json"),
    season_entry!("20052006", "bios.json"),
    season_entry!("20032004", "bios.json"),
    season_entry!("20022003", "bios.json"),
    season_entry!("20012002", "bios.json"),
    season_entry!("20002001", "bios.json"),
    season_entry!("19992000", "bios.json"),
    season_entry!("19981999", "bios.json"),
    season_entry!("19971998", "bios.json"),
    season_entry!("19961997", "bios.json"),
    season_entry!("19951996", "bios.json"),
    season_entry!("19941995", "bios.json"),
    season_entry!("19931994", "bios.json"),
    season_entry!("19921993", "bios.json"),
    season_entry!("19911992", "bios.json"),
    season_entry!("19901991", "bios.json"),
    season_entry!("19891990", "bios.json"),
    season_entry!("19881989", "bios.json"),
    season_entry!("19871988", "bios.json"),
];

static BUNDLED_STATS: &[(&str, &[u8])] = &[
    season_entry!("20252026", "stats.json"),
    season_entry!("20242025", "stats.json"),
    season_entry!("20232024", "stats.json"),
    season_entry!("20222023", "stats.json"),
    season_entry!("20212022", "stats.json"),
    season_entry!("20202021", "stats.json"),
    season_entry!("20192020", "stats.json"),
    season_entry!("20182019", "stats.json"),
    season_entry!("20172018", "stats.json"),
    season_entry!("20162017", "stats.json"),
    season_entry!("20152016", "stats.json"),
    season_entry!("20142015", "stats.json"),
    season_entry!("20132014", "stats.json"),
    season_entry!("20122013", "stats.json"),
    season_entry!("20112012", "stats.json"),
    season_entry!("20102011", "stats.json"),
    season_entry!("20092010", "stats.json"),
    season_entry!("20082009", "stats.json"),
    season_entry!("20072008", "stats.json"),
    season_entry!("20062007", "stats.json"),
    season_entry!("20052006", "stats.json"),
    season_entry!("20032004", "stats.json"),
    season_entry!("20022003", "stats.json"),
    season_entry!("20012002", "stats.json"),
    season_entry!("20002001", "stats.json"),
    season_entry!("19992000", "stats.json"),
    season_entry!("19981999", "stats.json"),
    season_entry!("19971998", "stats.json"),
    season_entry!("19961997", "stats.json"),
    season_entry!("19951996", "stats.json"),
    season_entry!("19941995", "stats.json"),
    season_entry!("19931994", "stats.json"),
    season_entry!("19921993", "stats.json"),
    season_entry!("19911992", "stats.json"),
    season_entry!("19901991", "stats.json"),
    season_entry!("19891990", "stats.json"),
    season_entry!("19881989", "stats.json"),
    season_entry!("19871988", "stats.json"),
];

static BUNDLED_GOALIES: &[(&str, &[u8])] = &[
    season_entry!("20252026", "goalie-stats.json"),
    season_entry!("20242025", "goalie-stats.json"),
    season_entry!("20232024", "goalie-stats.json"),
    season_entry!("20222023", "goalie-stats.json"),
    season_entry!("20212022", "goalie-stats.json"),
    season_entry!("20202021", "goalie-stats.json"),
    season_entry!("20192020", "goalie-stats.json"),
    season_entry!("20182019", "goalie-stats.json"),
    season_entry!("20172018", "goalie-stats.json"),
    season_entry!("20162017", "goalie-stats.json"),
    season_entry!("20152016", "goalie-stats.json"),
    season_entry!("20142015", "goalie-stats.json"),
    season_entry!("20132014", "goalie-stats.json"),
    season_entry!("20122013", "goalie-stats.json"),
    season_entry!("20112012", "goalie-stats.json"),
    season_entry!("20102011", "goalie-stats.json"),
    season_entry!("20092010", "goalie-stats.json"),
    season_entry!("20082009", "goalie-stats.json"),
    season_entry!("20072008", "goalie-stats.json"),
    season_entry!("20062007", "goalie-stats.json"),
    season_entry!("20052006", "goalie-stats.json"),
    season_entry!("20032004", "goalie-stats.json"),
    season_entry!("20022003", "goalie-stats.json"),
    season_entry!("20012002", "goalie-stats.json"),
    season_entry!("20002001", "goalie-stats.json"),
    season_entry!("19992000", "goalie-stats.json"),
    season_entry!("19981999", "goalie-stats.json"),
    season_entry!("19971998", "goalie-stats.json"),
    season_entry!("19961997", "goalie-stats.json"),
    season_entry!("19951996", "goalie-stats.json"),
    season_entry!("19941995", "goalie-stats.json"),
    season_entry!("19931994", "goalie-stats.json"),
    season_entry!("19921993", "goalie-stats.json"),
    season_entry!("19911992", "goalie-stats.json"),
    season_entry!("19901991", "goalie-stats.json"),
    season_entry!("19891990", "goalie-stats.json"),
    season_entry!("19881989", "goalie-stats.json"),
    season_entry!("19871988", "goalie-stats.json"),
];

static BUNDLED_PLAYOFF_BIOS: &[(&str, &[u8])] = &[
    season_entry!("20252026", "playoff-bios.json"),
    season_entry!("20242025", "playoff-bios.json"),
    season_entry!("20232024", "playoff-bios.json"),
    season_entry!("20222023", "playoff-bios.json"),
    season_entry!("20212022", "playoff-bios.json"),
    season_entry!("20202021", "playoff-bios.json"),
    season_entry!("20192020", "playoff-bios.json"),
    season_entry!("20182019", "playoff-bios.json"),
    season_entry!("20172018", "playoff-bios.json"),
    season_entry!("20162017", "playoff-bios.json"),
    season_entry!("20152016", "playoff-bios.json"),
    season_entry!("20142015", "playoff-bios.json"),
    season_entry!("20132014", "playoff-bios.json"),
    season_entry!("20122013", "playoff-bios.json"),
    season_entry!("20112012", "playoff-bios.json"),
    season_entry!("20102011", "playoff-bios.json"),
    season_entry!("20092010", "playoff-bios.json"),
    season_entry!("20082009", "playoff-bios.json"),
    season_entry!("20072008", "playoff-bios.json"),
    season_entry!("20062007", "playoff-bios.json"),
    season_entry!("20052006", "playoff-bios.json"),
    season_entry!("20032004", "playoff-bios.json"),
    season_entry!("20022003", "playoff-bios.json"),
    season_entry!("20012002", "playoff-bios.json"),
    season_entry!("20002001", "playoff-bios.json"),
    season_entry!("19992000", "playoff-bios.json"),
    season_entry!("19981999", "playoff-bios.json"),
    season_entry!("19971998", "playoff-bios.json"),
    season_entry!("19961997", "playoff-bios.json"),
    season_entry!("19951996", "playoff-bios.json"),
    season_entry!("19941995", "playoff-bios.json"),
    season_entry!("19931994", "playoff-bios.json"),
    season_entry!("19921993", "playoff-bios.json"),
    season_entry!("19911992", "playoff-bios.json"),
    season_entry!("19901991", "playoff-bios.json"),
    season_entry!("19891990", "playoff-bios.json"),
    season_entry!("19881989", "playoff-bios.json"),
    season_entry!("19871988", "playoff-bios.json"),
];

static BUNDLED_PLAYOFF_STATS: &[(&str, &[u8])] = &[
    season_entry!("20252026", "playoff-stats.json"),
    season_entry!("20242025", "playoff-stats.json"),
    season_entry!("20232024", "playoff-stats.json"),
    season_entry!("20222023", "playoff-stats.json"),
    season_entry!("20212022", "playoff-stats.json"),
    season_entry!("20202021", "playoff-stats.json"),
    season_entry!("20192020", "playoff-stats.json"),
    season_entry!("20182019", "playoff-stats.json"),
    season_entry!("20172018", "playoff-stats.json"),
    season_entry!("20162017", "playoff-stats.json"),
    season_entry!("20152016", "playoff-stats.json"),
    season_entry!("20142015", "playoff-stats.json"),
    season_entry!("20132014", "playoff-stats.json"),
    season_entry!("20122013", "playoff-stats.json"),
    season_entry!("20112012", "playoff-stats.json"),
    season_entry!("20102011", "playoff-stats.json"),
    season_entry!("20092010", "playoff-stats.json"),
    season_entry!("20082009", "playoff-stats.json"),
    season_entry!("20072008", "playoff-stats.json"),
    season_entry!("20062007", "playoff-stats.json"),
    season_entry!("20052006", "playoff-stats.json"),
    season_entry!("20032004", "playoff-stats.json"),
    season_entry!("20022003", "playoff-stats.json"),
    season_entry!("20012002", "playoff-stats.json"),
    season_entry!("20002001", "playoff-stats.json"),
    season_entry!("19992000", "playoff-stats.json"),
    season_entry!("19981999", "playoff-stats.json"),
    season_entry!("19971998", "playoff-stats.json"),
    season_entry!("19961997", "playoff-stats.json"),
    season_entry!("19951996", "playoff-stats.json"),
    season_entry!("19941995", "playoff-stats.json"),
    season_entry!("19931994", "playoff-stats.json"),
    season_entry!("19921993", "playoff-stats.json"),
    season_entry!("19911992", "playoff-stats.json"),
    season_entry!("19901991", "playoff-stats.json"),
    season_entry!("19891990", "playoff-stats.json"),
    season_entry!("19881989", "playoff-stats.json"),
    season_entry!("19871988", "playoff-stats.json"),
];

static BUNDLED_PLAYOFF_GOALIES: &[(&str, &[u8])] = &[
    season_entry!("20252026", "playoff-goalie-stats.json"),
    season_entry!("20242025", "playoff-goalie-stats.json"),
    season_entry!("20232024", "playoff-goalie-stats.json"),
    season_entry!("20222023", "playoff-goalie-stats.json"),
    season_entry!("20212022", "playoff-goalie-stats.json"),
    season_entry!("20202021", "playoff-goalie-stats.json"),
    season_entry!("20192020", "playoff-goalie-stats.json"),
    season_entry!("20182019", "playoff-goalie-stats.json"),
    season_entry!("20172018", "playoff-goalie-stats.json"),
    season_entry!("20162017", "playoff-goalie-stats.json"),
    season_entry!("20152016", "playoff-goalie-stats.json"),
    season_entry!("20142015", "playoff-goalie-stats.json"),
    season_entry!("20132014", "playoff-goalie-stats.json"),
    season_entry!("20122013", "playoff-goalie-stats.json"),
    season_entry!("20112012", "playoff-goalie-stats.json"),
    season_entry!("20102011", "playoff-goalie-stats.json"),
    season_entry!("20092010", "playoff-goalie-stats.json"),
    season_entry!("20082009", "playoff-goalie-stats.json"),
    season_entry!("20072008", "playoff-goalie-stats.json"),
    season_entry!("20062007", "playoff-goalie-stats.json"),
    season_entry!("20052006", "playoff-goalie-stats.json"),
    season_entry!("20032004", "playoff-goalie-stats.json"),
    season_entry!("20022003", "playoff-goalie-stats.json"),
    season_entry!("20012002", "playoff-goalie-stats.json"),
    season_entry!("20002001", "playoff-goalie-stats.json"),
    season_entry!("19992000", "playoff-goalie-stats.json"),
    season_entry!("19981999", "playoff-goalie-stats.json"),
    season_entry!("19971998", "playoff-goalie-stats.json"),
    season_entry!("19961997", "playoff-goalie-stats.json"),
    season_entry!("19951996", "playoff-goalie-stats.json"),
    season_entry!("19941995", "playoff-goalie-stats.json"),
    season_entry!("19931994", "playoff-goalie-stats.json"),
    season_entry!("19921993", "playoff-goalie-stats.json"),
    season_entry!("19911992", "playoff-goalie-stats.json"),
    season_entry!("19901991", "playoff-goalie-stats.json"),
    season_entry!("19891990", "playoff-goalie-stats.json"),
    season_entry!("19881989", "playoff-goalie-stats.json"),
    season_entry!("19871988", "playoff-goalie-stats.json"),
];

// Transactions — Phases T.3 + T.6. Modern era only — pre-2021 transaction
// logs aren't on ESPN's site.api. Captured via
// `cargo run --example probe_espn_seasons -- --write-bundle`.
static BUNDLED_TRANSACTIONS: &[(&str, &[u8])] = &[
    season_entry!("20252026", "transactions.json"),
    season_entry!("20242025", "transactions.json"),
    season_entry!("20232024", "transactions.json"),
    season_entry!("20222023", "transactions.json"),
    season_entry!("20212022", "transactions.json"),
];

/// Lookup helper — returns the embedded bytes for `season` from `table`,
/// or `None` if the season isn't in that table.
#[inline]
fn lookup<'a>(table: &'a [(&'static str, &'static [u8])], season: &str) -> Option<&'a [u8]> {
    table.iter().find_map(|(s, b)| (*s == season).then_some(*b))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// List of bundled seasons, newest first. Now covers every NHL season
/// from 1987-88 forward except the 2004-05 lockout (38 entries).
pub const BUNDLED_SEASONS: &[&str] = &[
    "20252026", "20242025", "20232024", "20222023", "20212022", "20202021", "20192020", "20182019",
    "20172018", "20162017", "20152016", "20142015", "20132014", "20122013", "20112012", "20102011",
    "20092010", "20082009", "20072008", "20062007", "20052006", "20032004", "20022003", "20012002",
    "20002001", "19992000", "19981999", "19971998", "19961997", "19951996", "19941995", "19931994",
    "19921993", "19911992", "19901991", "19891990", "19881989", "19871988",
];

/// Modern-era seasons that carry the full Phase Lindsay Tier-1 report
/// suite (realtime, time-on-ice, goalsForAgainst, goalie-advanced,
/// goalie-savesByStrength). Subset of `BUNDLED_SEASONS`.
///
/// Used by `aggregate.rs` / `dashboard_panel.rs` callers that intend to
/// scope to "rich-data seasons" rather than the full historical span.
pub const MODERN_BUNDLED_SEASONS: &[&str] =
    &["20252026", "20242025", "20232024", "20222023", "20212022"];

// Phase Calder.2 — career history is intentionally NOT bundled.
//
// We considered embedding `data/career_history.json` (~30 MB
// compact) into the binary, but evaluated against:
//   * Lazy load via `/v1/player/{id}/landing` for the player-card
//     use case (50 ms one-time per player, cached in
//     `~/.icelines/career_history.json`)
//   * `icelines fetch career` for the cohort-query use case (one-time
//     ~100 s populate before Calder.4 cross-league leaderboards work)
// the +30 MB binary cost wasn't earning its keep. See
// design/notes/2026-05-06-Calder-2-bundle-vs-lazy.md for the call.

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
    let bytes = lookup(BUNDLED_BIOS, season)?;
    serde_json::from_slice(bytes).ok()
}

/// Deserialize bundled stats for a season. Returns None if season not bundled.
pub fn get_stats(season: &str) -> Option<Vec<SkaterStats>> {
    let bytes = lookup(BUNDLED_STATS, season)?;
    serde_json::from_slice(bytes).ok()
}

/// Deserialize bundled goalie stats for a season (Phase G.1). Returns
/// `None` when the season isn't one of the 38 embedded seasons. Use
/// `get_goalie_stats_installed` to read from `~/.icelines/seasons/` for
/// pre-1987 seasons or fresher data brought in via source fetches.
pub fn get_goalie_stats(season: &str) -> Option<Vec<GoalieStats>> {
    let bytes = lookup(BUNDLED_GOALIES, season)?;
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
    pub season: String,
    pub source: String,
    pub fetched_at: String,
    pub classifier_version: u16,
    pub rows: Vec<icelines_core::Transaction>,
}

/// Read embedded transactions for a bundled season. Returns None for any
/// season not in the include_bytes! set (modern era only — pre-2021
/// transaction logs aren't available on ESPN's site.api).
pub fn get_transactions(season: &str) -> Option<TransactionsEnvelope> {
    let bytes = lookup(BUNDLED_TRANSACTIONS, season)?;
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
        &crate::snapshot::SnapshotTier::Stats,
        "transactions.json",
    ) {
        env
    } else if let Some(env) = get_transactions(season) {
        env
    } else if let Some(env) = get_transactions_installed(season) {
        env
    } else {
        return Err(FetchError::PlayerNotFound {
            name: format!(
                "no transactions for season {season} — run `icelines fetch transactions`"
            ),
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
    if let Ok(rows) = store
        .read_tier::<Vec<GoalieStats>>(&crate::snapshot::SnapshotTier::Stats, "goalie-stats.json")
    {
        return Ok(rows);
    }
    // 2. Bundled data.
    if let Some(rows) = get_goalie_stats(season) {
        return Ok(rows);
    }
    // 3. Installed (historical) bundle.
    if let Some(rows) = get_goalie_stats_installed(season) {
        return Ok(rows);
    }
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
    Some(
        std::path::Path::new(&home)
            .join(".icelines")
            .join("seasons")
            .join(season_id)
            .join(format!("bundle-{season_id}")),
    )
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
pub fn get_playoff_stats_installed(season_id: &str) -> Option<Vec<crate::schema::SkaterStats>> {
    let path = season_bundle_dir(season_id)?.join("playoff-stats.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Read playoff goalie stats from an installed season bundle.
pub fn get_playoff_goalie_stats_installed(season_id: &str) -> Option<Vec<GoalieStats>> {
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
    let bytes = lookup(BUNDLED_PLAYOFF_BIOS, season_id)?;
    serde_json::from_slice(bytes).ok()
}

/// Playoff stats for a bundled season. See `get_playoff_bios` for
/// semantics; same `BUNDLED_SEASONS` membership rule applies.
pub fn get_playoff_stats(season_id: &str) -> Option<Vec<crate::schema::SkaterStats>> {
    let bytes = lookup(BUNDLED_PLAYOFF_STATS, season_id)?;
    serde_json::from_slice(bytes).ok()
}

/// Playoff goalie stats for a bundled season.
pub fn get_playoff_goalie_stats(season_id: &str) -> Option<Vec<GoalieStats>> {
    let bytes = lookup(BUNDLED_PLAYOFF_GOALIES, season_id)?;
    serde_json::from_slice(bytes).ok()
}

// ── Historical playoffs (Phase 8c) ───────────────────────────────────────────

/// Embedded `playoffs.json` files. Each entry is `(season_id, &[u8])`. Add new
/// historical seasons here as their bundles are authored. The 1993-94 NYR Cup
/// run is the canonical first fixture per `design/specs/playoffs.md`.
static BUNDLED_PLAYOFFS: &[(&str, &[u8])] = &[(
    "19931994",
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/seasons/19931994/playoffs.json"
    )),
)];

/// List of seasons with bundled playoff data.
pub fn bundled_playoff_seasons() -> Vec<&'static str> {
    BUNDLED_PLAYOFFS.iter().map(|(s, _)| *s).collect()
}

/// Deserialize bundled `playoffs.json` for a season. Returns None if no
/// bundle has been authored for that season yet.
pub fn get_playoffs(season_id: &str) -> Option<PlayoffsBundle> {
    let bytes = BUNDLED_PLAYOFFS
        .iter()
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
        name: format!(
            "no playoff bios for season {season} — run `icelines fetch stats --type playoff`"
        ),
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
        name: format!(
            "no playoff stats for season {season} — run `icelines fetch stats --type playoff`"
        ),
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
        return Err(crate::snapshot::SnapshotError::NotFound {
            name: format!("{active}/chunked.json"),
        });
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
        return Err(crate::snapshot::SnapshotError::NotFound {
            name: format!("{active}/chunked.json"),
        });
    }
    store.read_chunked_stats(active, icelines_core::season_stats::SeasonType::Playoff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::CURRENT_SEASON_STR;

    #[test]
    fn l0_bundled_current_season_bios_parse() {
        // Verify bundled JSON parses correctly — catches malformed data at compile time
        let bios = get_bios("20252026").expect("20252026 must be bundled");
        assert!(!bios.is_empty(), "bundled bios must not be empty");
        assert!(
            bios.len() > 500,
            "expected 900+ players, got {}",
            bios.len()
        );
    }

    #[test]
    fn l0_bundled_current_season_stats_parse() {
        let stats = get_stats("20252026").expect("20252026 must be bundled");
        assert!(
            stats.len() > 500,
            "expected 900+ players, got {}",
            stats.len()
        );
    }

    #[test]
    fn l0_bundled_historical_season_parses() {
        let bios = get_bios("20242025").expect("20242025 must be bundled");
        assert!(!bios.is_empty());
        // Each bio must have a player_id
        assert!(bios.iter().all(|b| b.player_id > 0));
    }

    #[test]
    fn l0_bundled_unknown_season_returns_none() {
        // 2004-05 is the lockout year — never has data, never bundled.
        assert!(get_bios("20042005").is_none());
        assert!(get_stats("20042005").is_none());
        // Pre-1987 seasons have no data files in the repo.
        assert!(get_bios("19861987").is_none());
        assert!(get_stats("19861987").is_none());
    }

    #[test]
    fn l0_release_current_season_is_at_or_one_ahead_of_completed_bundles() {
        let newest_bundle = BUNDLED_SEASONS
            .first()
            .copied()
            .expect("at least one completed season must be bundled");
        let current = icelines_core::Season::try_new(
            CURRENT_SEASON_STR
                .parse()
                .expect("current season is numeric"),
        )
        .expect("current season is valid");
        let bundled = icelines_core::Season::try_new(
            newest_bundle.parse().expect("bundled season is numeric"),
        )
        .expect("bundled season is valid");
        assert!(
            current.start_year() == bundled.start_year()
                || current.start_year() == bundled.start_year() + 1,
            "current roster season must match or immediately follow the newest completed stats bundle"
        );
        assert!(
            get_bios(newest_bundle).is_some(),
            "newest completed season must carry bundled bios for cold-start release smoke"
        );
        assert!(
            get_stats(newest_bundle).is_some(),
            "newest completed season must carry bundled stats for cold-start release smoke"
        );
        assert!(
            get_goalie_stats(newest_bundle).is_some(),
            "newest completed season must carry bundled goalie stats for cold-start release smoke"
        );
        assert!(
            !BUNDLED_SEASONS.contains(&"20042005"),
            "2004-05 lockout must remain excluded from release bundles"
        );
    }

    /// L.7b — every season in `BUNDLED_SEASONS` has bios + stats +
    /// goalie-stats + playoff-{bios,stats,goalie-stats}. 38 seasons × 6
    /// kinds = 228 cells must all be `Some`. Catches a missing data
    /// file at the bundle-authoring stage rather than at runtime.
    #[test]
    fn l0_bundled_all_seasons_have_all_six_kinds() {
        assert_eq!(
            BUNDLED_SEASONS.len(),
            38,
            "BUNDLED_SEASONS drift — expected 38 entries (1987-88 forward, no 2004-05 lockout)"
        );
        for season in BUNDLED_SEASONS {
            assert!(
                get_bios(season).is_some(),
                "season {season} missing bios.json"
            );
            assert!(
                get_stats(season).is_some(),
                "season {season} missing stats.json"
            );
            assert!(
                get_goalie_stats(season).is_some(),
                "season {season} missing goalie-stats.json"
            );
            assert!(
                get_playoff_bios(season).is_some(),
                "season {season} missing playoff-bios.json"
            );
            assert!(
                get_playoff_stats(season).is_some(),
                "season {season} missing playoff-stats.json"
            );
            assert!(
                get_playoff_goalie_stats(season).is_some(),
                "season {season} missing playoff-goalie-stats.json"
            );
        }
    }

    /// L.7b — `MODERN_BUNDLED_SEASONS` is a strict subset of
    /// `BUNDLED_SEASONS` and matches the 5 seasons that carry the full
    /// Phase Lindsay Tier-1 report suite.
    #[test]
    fn l0_modern_bundled_is_subset_of_all_bundled() {
        for season in MODERN_BUNDLED_SEASONS {
            assert!(
                BUNDLED_SEASONS.contains(season),
                "modern season {season} must also be in BUNDLED_SEASONS"
            );
        }
        assert_eq!(MODERN_BUNDLED_SEASONS.len(), 5);
        // Modern is the head of BUNDLED_SEASONS (newest 5).
        assert_eq!(MODERN_BUNDLED_SEASONS, &BUNDLED_SEASONS[..5]);
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
        let cup = b
            .rounds
            .iter()
            .find(|r| r.round == 4)
            .expect("round 4 present");
        assert_eq!(cup.series.len(), 1, "Cup Final has one series");
        assert_eq!(cup.series[0].results.len(), 7, "Cup Final ran 7 games");
        // Convert via to_bracket and verify wins were derived correctly.
        let br = b.to_bracket();
        let cup_series = &br
            .rounds
            .iter()
            .find(|r| r.round_number == 4)
            .unwrap()
            .series[0];
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
    /// 37 of 38 carry real playoff data (every contested year);
    /// 2025-26 ships as `[]` (Cup not yet contested).
    #[test]
    fn l0_hart6_3_get_playoff_bios_parses_for_all_bundled_seasons() {
        for season in BUNDLED_SEASONS {
            let v = get_playoff_bios(season);
            assert!(v.is_some(), "season {season} must be in the bundled set");
        }
        // 2025-26 is the deliberate empty (Cup not yet contested at
        // bundle-authoring time). Asserted explicitly so a future
        // accidental deletion of the playoff-bios.json file surfaces.
        assert!(
            get_playoff_bios("20252026").unwrap().is_empty(),
            "2025-26 ships as [] until the playoffs are bundled"
        );
        // Every other bundled season had a contested playoff and must
        // carry ≥100 roster rows.
        for season in BUNDLED_SEASONS.iter().filter(|s| **s != "20252026") {
            let count = get_playoff_bios(season).unwrap().len();
            assert!(
                count > 100,
                "{season} playoff bios must have ≥100 rows (NHL playoff rosters), got {count}"
            );
        }
    }

    /// Hart.6.3 — every bundled season's playoff stats parse and the
    /// row count matches the bios count for that season (every player
    /// who has a bio also has a stats row). Skips 2025-26 (empty until
    /// playoffs run).
    #[test]
    fn l0_hart6_3_get_playoff_stats_matches_bios_count_per_season() {
        for season in BUNDLED_SEASONS.iter().filter(|s| **s != "20252026") {
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
        for season in BUNDLED_SEASONS.iter().filter(|s| **s != "20252026") {
            let expected: u32 = season.parse().unwrap();
            let stats = get_playoff_stats(season).unwrap();
            for s in &stats {
                assert_eq!(
                    s.season_id,
                    Some(expected),
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
        for season in BUNDLED_SEASONS.iter().filter(|s| **s != "20252026") {
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
        // 2004-05 lockout — never had data and can never be bundled.
        assert!(get_playoff_bios("20042005").is_none());
        assert!(get_playoff_stats("20042005").is_none());
        assert!(get_playoff_goalie_stats("20042005").is_none());
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
                            let v: serde_json::Value = serde_json::from_slice(&bytes)
                                .unwrap_or_else(|e| {
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
