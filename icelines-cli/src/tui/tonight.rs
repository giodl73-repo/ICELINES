use std::sync::{Arc, Mutex};
use icelines_fetch::nhl_api::{NhlApiClient, ScheduledGame};

#[derive(Debug, Clone, Default)]
pub enum TonightState {
    #[default]
    Idle,
    Loading,
    Loaded(Vec<ScheduledGame>),
    Error(String),
}

pub type TonightCache = Arc<Mutex<TonightState>>;

pub fn new_cache() -> TonightCache {
    Arc::new(Mutex::new(TonightState::Idle))
}

/// Trigger a background fetch if the cache is still Idle.
pub fn maybe_fetch(cache: TonightCache) {
    {
        let state = cache.lock().unwrap();
        if !matches!(*state, TonightState::Idle) {
            return;
        }
    }
    *cache.lock().unwrap() = TonightState::Loading;

    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::new();
        match client.fetch_today_schedule().await {
            Ok(games) => *cache2.lock().unwrap() = TonightState::Loaded(games),
            Err(e)    => *cache2.lock().unwrap() = TonightState::Error(e.to_string()),
        }
    });
}
