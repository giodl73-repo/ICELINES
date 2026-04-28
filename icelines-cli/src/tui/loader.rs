//! Background player loading for the TUI.
//!
//! Loads all players from bundled/snapshot data without blocking the event loop.

use std::sync::{Arc, Mutex};
use icelines_core::model::Player;

/// Shared loading state readable from the event loop.
#[derive(Debug, Clone)]
pub struct LoadState {
    inner: Arc<Mutex<LoadInner>>,
}

#[derive(Debug)]
struct LoadInner {
    pub players: Vec<Player>,
    pub loading: bool,
    pub error:   Option<String>,
}

impl LoadState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LoadInner {
                players: Vec::new(),
                loading: true,
                error:   None,
            })),
        }
    }

    #[allow(dead_code)]
    pub fn is_loading(&self) -> bool {
        self.inner.lock().map(|g| g.loading).unwrap_or(false)
    }

    pub fn take_players(&self) -> Option<Vec<Player>> {
        let mut g = self.inner.lock().ok()?;
        if !g.loading && g.error.is_none() && !g.players.is_empty() {
            Some(std::mem::take(&mut g.players))
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn error(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|g| g.error.clone())
    }

    fn set_done(&self, players: Vec<Player>) {
        if let Ok(mut g) = self.inner.lock() {
            g.players = players;
            g.loading = false;
        }
    }

    fn set_error(&self, msg: String) {
        if let Ok(mut g) = self.inner.lock() {
            g.error   = Some(msg);
            g.loading = false;
        }
    }
}

/// Spawn a tokio task that loads all players and stores them in `state`.
pub fn spawn_loader(state: LoadState) {
    tokio::spawn(async move {
        match crate::commands::players::load_all_players() {
            Ok(players) => state.set_done(players),
            Err(e)      => state.set_error(e.to_string()),
        }
    });
}
