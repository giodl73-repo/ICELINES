//! Playoffs tab: per-year bracket cache with non-blocking fetch.
//!
//! Mirrors `tonight.rs` / `schedule.rs`. Bracket year is the second half of
//! the season string (e.g. "20252026" → 2026).
//!
//! Phase 8c — historical seasons resolve through `bundled::load_playoffs`
//! (installed bundle preferred, binary-embedded fallback) before any network
//! call. Live API is only consulted for the current season.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use icelines_fetch::bundled::load_playoffs;
use icelines_fetch::nhl_api::{NhlApiClient, PlayoffBracket};

#[derive(Debug, Clone, Default)]
pub enum PlayoffsState {
    #[default]
    Idle,
    Loading,
    Loaded(PlayoffBracket),
    Error(String),
}

pub type PlayoffsCache = Arc<Mutex<HashMap<u16, PlayoffsState>>>;

pub fn new_cache() -> PlayoffsCache {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Convert a season string (e.g. "20252026") to the playoff year (2026).
pub fn playoff_year_for_season(season: &str) -> Option<u16> {
    if season.len() == 8 {
        season[4..].parse().ok()
    } else {
        None
    }
}

/// Try to resolve a bracket from bundled / installed historical data and
/// stash it in the cache. Returns `true` if the cache was populated (so the
/// caller can skip the network fetch). Phase 8c.
fn try_load_bundle(cache: &PlayoffsCache, year: u16, season: &str) -> bool {
    if let Some(b) = load_playoffs(season) {
        cache
            .lock()
            .unwrap()
            .insert(year, PlayoffsState::Loaded(b.to_bracket()));
        return true;
    }
    false
}

/// True iff `season` is the current NHL season — when not, we prefer bundled
/// historical data over the live API and never auto-fetch.
fn is_current_season(season: &str) -> bool {
    season == icelines_core::CURRENT_SEASON_STR
}

/// Trigger a background fetch for the bracket year if not already loaded.
///
/// Resolution order (Phase 8c):
/// 1. Bundled / installed `playoffs.json` for the season — synchronous, no I/O cost
/// 2. Live API `/v1/playoff-bracket/{year}` — only for the current season
/// 3. Idle (no data) — UI shows the placeholder
pub fn maybe_fetch_bracket(cache: PlayoffsCache, year: u16, season: &str) {
    {
        let map = cache.lock().unwrap();
        if matches!(
            map.get(&year),
            Some(PlayoffsState::Loading)
                | Some(PlayoffsState::Loaded(_))
                | Some(PlayoffsState::Error(_))
        ) {
            return;
        }
    }
    if try_load_bundle(&cache, year, season) {
        return;
    }
    // Historical season with no bundled data → show explicit message; do not
    // attempt a network fetch (the live API only covers recent seasons).
    if !is_current_season(season) {
        cache.lock().unwrap().insert(
            year,
            PlayoffsState::Error(format!("no bundled playoff data for season {season}")),
        );
        return;
    }
    if !crate::config::live_feeds_enabled() {
        cache.lock().unwrap().insert(
            year,
            PlayoffsState::Error(crate::tui::tonight::LIVE_DISABLED_MSG.to_owned()),
        );
        return;
    }
    cache.lock().unwrap().insert(year, PlayoffsState::Loading);
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = client.fetch_playoff_bracket(year).await;
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(b) => map.insert(year, PlayoffsState::Loaded(b)),
            Err(e) => map.insert(year, PlayoffsState::Error(e.to_string())),
        };
    });
}

/// Force-refetch (used by the `r` retry key). For historical seasons this
/// just re-reads the bundled file; for the current season it goes back to
/// the live API.
pub fn force_fetch_bracket(cache: PlayoffsCache, year: u16, season: &str) {
    if try_load_bundle(&cache, year, season) {
        return;
    }
    if !is_current_season(season) {
        cache.lock().unwrap().insert(
            year,
            PlayoffsState::Error(format!("no bundled playoff data for season {season}")),
        );
        return;
    }
    if !crate::config::live_feeds_enabled() {
        cache.lock().unwrap().insert(
            year,
            PlayoffsState::Error(crate::tui::tonight::LIVE_DISABLED_MSG.to_owned()),
        );
        return;
    }
    cache.lock().unwrap().insert(year, PlayoffsState::Loading);
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = client.fetch_playoff_bracket(year).await;
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(b) => map.insert(year, PlayoffsState::Loaded(b)),
            Err(e) => map.insert(year, PlayoffsState::Error(e.to_string())),
        };
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_playoffs_year_from_season_string() {
        assert_eq!(playoff_year_for_season("20252026"), Some(2026));
        assert_eq!(playoff_year_for_season("19931994"), Some(1994));
        // 7-char or malformed → None
        assert_eq!(playoff_year_for_season("2025"), None);
        assert_eq!(playoff_year_for_season("not-a-season"), None);
    }

    #[test]
    fn l0_maybe_fetch_loads_bundled_historical_synchronously() {
        // 1993-94 ships with bundled playoffs.json; no tokio runtime needed
        // because the path returns before spawning.
        let cache = new_cache();
        maybe_fetch_bracket(cache.clone(), 1994, "19931994");
        let state = cache.lock().unwrap().get(&1994).cloned();
        match state {
            Some(PlayoffsState::Loaded(b)) => {
                assert_eq!(b.season, "19931994");
                assert_eq!(b.rounds.len(), 4);
            }
            other => panic!("expected Loaded, got {other:?}"),
        }
    }

    #[test]
    fn l0_maybe_fetch_historical_without_bundle_returns_error() {
        // 1995-96 has no bundled playoffs.json — should produce a clear error
        // message rather than spinning Loading forever or hitting the live API.
        let cache = new_cache();
        maybe_fetch_bracket(cache.clone(), 1996, "19951996");
        let state = cache.lock().unwrap().get(&1996).cloned();
        match state {
            Some(PlayoffsState::Error(e)) => {
                assert!(
                    e.contains("no bundled playoff data"),
                    "expected explicit no-bundle message, got: {e}"
                );
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn l0_force_fetch_bundled_historical_replaces_existing_state() {
        // Pre-seed Error to verify force_fetch overwrites it for historical seasons.
        let cache = new_cache();
        cache
            .lock()
            .unwrap()
            .insert(1994, PlayoffsState::Error("stale".to_owned()));
        force_fetch_bracket(cache.clone(), 1994, "19931994");
        let state = cache.lock().unwrap().get(&1994).cloned();
        assert!(matches!(state, Some(PlayoffsState::Loaded(_))));
    }
}
