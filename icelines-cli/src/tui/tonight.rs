//! Scores tab cache: per-date.
//!
//! Each date is fetched independently — past dates are cached permanently
//! (final scores don't change), today is treated as "now" via the `/v1/schedule/now`
//! endpoint. The map key is the canonical date string, and `today_key()` returns
//! the canonical key for "today / live" data.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use icelines_fetch::nhl_api::{Boxscore, NhlApiClient, ScheduledGame};

#[derive(Debug, Clone, Default)]
pub enum TonightState {
    #[default]
    Idle,
    Loading,
    Loaded(Vec<ScheduledGame>),
    Error(String),
}

pub type TonightCache = Arc<Mutex<HashMap<String, TonightState>>>;

/// The canonical "today/live" key. Empty string sentinels keep the public API
/// stable when callers want to refer to "today" without computing a date.
pub const TODAY_KEY: &str = "";

pub fn new_cache() -> TonightCache {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Trigger a background fetch for `date_key`. Empty string means "today" —
/// fetched via `/v1/schedule/now`. Any other value is interpreted as
/// `YYYY-MM-DD` and fetched via `/v1/schedule/{date}`.
pub fn maybe_fetch(cache: TonightCache, date_key: String) {
    {
        let mut map = cache.lock().unwrap();
        match map.get(&date_key) {
            Some(TonightState::Loading)
            | Some(TonightState::Loaded(_))
            | Some(TonightState::Error(_)) => return,
            _ => {}
        }
        map.insert(date_key.clone(), TonightState::Loading);
    }
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = if date_key == TODAY_KEY {
            client.fetch_today_schedule().await
        } else {
            client.fetch_schedule_for_date(&date_key).await
        };
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(games) => map.insert(date_key, TonightState::Loaded(games)),
            Err(e)    => map.insert(date_key, TonightState::Error(e.to_string())),
        };
    });
}

/// Force-refetch even if the cache has data (used by the `r` key).
pub fn force_fetch(cache: TonightCache, date_key: String) {
    cache.lock().unwrap().insert(date_key.clone(), TonightState::Loading);
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = if date_key == TODAY_KEY {
            client.fetch_today_schedule().await
        } else {
            client.fetch_schedule_for_date(&date_key).await
        };
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(games) => map.insert(date_key, TonightState::Loaded(games)),
            Err(e)    => map.insert(date_key, TonightState::Error(e.to_string())),
        };
    });
}

/// Return the cache state for a date key, defaulting to Idle if missing.
pub fn lookup(cache: &TonightCache, date_key: &str) -> TonightState {
    let map = cache.lock().unwrap();
    map.get(date_key).cloned().unwrap_or(TonightState::Idle)
}

// ── Per-game boxscore cache ──────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub enum BoxscoreState {
    #[default]
    Idle,
    Loading,
    Loaded(Boxscore),
    Error(String),
}

pub type BoxscoreCache = Arc<Mutex<HashMap<u64, BoxscoreState>>>;

pub fn new_boxscore_cache() -> BoxscoreCache {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn maybe_fetch_boxscore(cache: BoxscoreCache, game_id: u64) {
    {
        let mut map = cache.lock().unwrap();
        match map.get(&game_id) {
            Some(BoxscoreState::Loading)
            | Some(BoxscoreState::Loaded(_))
            | Some(BoxscoreState::Error(_)) => return,
            _ => {}
        }
        map.insert(game_id, BoxscoreState::Loading);
    }
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = client.fetch_boxscore(game_id).await;
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(b)  => map.insert(game_id, BoxscoreState::Loaded(b)),
            Err(e) => map.insert(game_id, BoxscoreState::Error(e.to_string())),
        };
    });
}

pub fn lookup_boxscore(cache: &BoxscoreCache, game_id: u64) -> BoxscoreState {
    let map = cache.lock().unwrap();
    map.get(&game_id).cloned().unwrap_or(BoxscoreState::Idle)
}
