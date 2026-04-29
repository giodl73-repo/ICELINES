//! Playoffs tab: per-year bracket cache with non-blocking fetch.
//!
//! Mirrors `tonight.rs` / `schedule.rs`. Bracket year is the second half of
//! the season string (e.g. "20252026" → 2026).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

/// Trigger a background fetch for the bracket year if not already loaded.
pub fn maybe_fetch_bracket(cache: PlayoffsCache, year: u16) {
    if !crate::config::live_feeds_enabled() {
        cache.lock().unwrap()
            .insert(year, PlayoffsState::Error(
                crate::tui::tonight::LIVE_DISABLED_MSG.to_owned()));
        return;
    }
    {
        let mut map = cache.lock().unwrap();
        match map.get(&year) {
            Some(PlayoffsState::Loading)
            | Some(PlayoffsState::Loaded(_))
            | Some(PlayoffsState::Error(_)) => return,
            _ => {}
        }
        map.insert(year, PlayoffsState::Loading);
    }
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = client.fetch_playoff_bracket(year).await;
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(b)  => map.insert(year, PlayoffsState::Loaded(b)),
            Err(e) => map.insert(year, PlayoffsState::Error(e.to_string())),
        };
    });
}

/// Force-refetch (used by the `r` retry key).
pub fn force_fetch_bracket(cache: PlayoffsCache, year: u16) {
    if !crate::config::live_feeds_enabled() {
        cache.lock().unwrap()
            .insert(year, PlayoffsState::Error(
                crate::tui::tonight::LIVE_DISABLED_MSG.to_owned()));
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
            Ok(b)  => map.insert(year, PlayoffsState::Loaded(b)),
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
}
