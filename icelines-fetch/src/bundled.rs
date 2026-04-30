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

// ── Public API ────────────────────────────────────────────────────────────────

/// List of bundled seasons, newest first.
pub const BUNDLED_SEASONS: &[&str] = &[
    "20252026", "20242025", "20232024", "20222023", "20212022",
];

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
    store.read_chunked_stats(active)
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
}
