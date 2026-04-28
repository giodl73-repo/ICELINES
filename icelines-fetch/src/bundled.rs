//! Historical season data bundled directly into the binary via include_bytes!().
//!
//! Five seasons ship with every icelines binary — no download required.
//! `icelines fetch all` updates the current season in ~/.icelines/snapshots/
//! and takes precedence via the normal snapshot store lookup.
//!
//! Data source: NHL API bios + summary endpoints.
//! Historical seasons are immutable — they never change after the season ends.

use crate::{error::FetchError, schema::{SkaterBio, SkaterStats}};

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

/// Load bios: try snapshot store first (fresh), fall back to bundled data.
pub fn load_bios_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<SkaterBio>, FetchError> {
    // 1. Fresh snapshot (from icelines fetch)
    if let Ok(bios) = store.read_tier(&crate::snapshot::SnapshotTier::Stats, "bios.json") {
        return Ok(bios);
    }
    // 2. Bundled data shipped with binary
    get_bios(season).ok_or_else(|| FetchError::PlayerNotFound {
        name: format!("no bios for season {season} — run `icelines fetch stats`"),
    })
}

/// Load stats: try snapshot store first (fresh), fall back to bundled data.
pub fn load_stats_with_fallback(
    season: &str,
    store: &crate::snapshot::SnapshotStore,
) -> Result<Vec<SkaterStats>, FetchError> {
    if let Ok(stats) = store.read_tier(&crate::snapshot::SnapshotTier::Stats, "stats.json") {
        return Ok(stats);
    }
    get_stats(season).ok_or_else(|| FetchError::PlayerNotFound {
        name: format!("no stats for season {season} — run `icelines fetch stats`"),
    })
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
}
