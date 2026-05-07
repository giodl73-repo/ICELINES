//! Phase Art Ross A.2.4 — `IcelinesProvider`: the production
//! `DataProvider` impl.
//!
//! Walks the Boxscore manifest shard, parses each persisted
//! boxscore, and builds `GameStatLine` records for sliding-window
//! evaluation. Lives in `icelines-fetch` so the upward dep from
//! `icelines-query` to `icelines-fetch` stays clean (impl, not
//! interface).
//!
//! For A.2.4 only `fetch_game_lines` is wired against real data;
//! `ensure` is a no-op (full on-demand fetching ships in a
//! follow-on commit when the network-poking story matures).

use std::path::PathBuf;

use icelines_query::data_provider::{
    DataProvider, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::sliding_window::GameStatLine;

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind};
use crate::nhl_api::parse_boxscore;

/// Production `DataProvider` impl. Holds a `DataStore` reference
/// (via owned `PathBuf` so the trait object can be stored) so the
/// CLI/web/TUI surface can construct one once and pass it through
/// `EvalCtx`.
pub struct IcelinesProvider {
    data_root: PathBuf,
}

impl IcelinesProvider {
    /// Construct an `IcelinesProvider` rooted at `~/.icelines/data`.
    pub fn new(data_root: PathBuf) -> Self {
        Self { data_root }
    }

    /// Open the underlying `DataStore` for one operation. Errors
    /// (path issues, manifest read failures) collapse to "no data"
    /// — the executor sees an empty Vec and falls through to
    /// false. A.2.5's fail-closed default makes this safe.
    fn open_store(&self) -> Option<DataStore> {
        DataStore::open(&self.data_root).ok()
    }
}

impl DataProvider for IcelinesProvider {
    fn ensure(
        &self,
        _req: &PlanRequirement,
        _events: &mut dyn FnMut(FetchEvent),
    ) -> Result<(), FetchError> {
        // A.2.4 — no-op. A follow-on commit will scan the manifest
        // for missing date ranges and trigger the existing
        // `fetch boxscore --date YYYY-MM-DD` path. Today the
        // executor walks whatever's locally persisted; this is
        // sufficient for the killer query against bundled data.
        Ok(())
    }

    fn fetch_game_lines(&self, player_id: u32, _season: u32) -> Vec<GameStatLine> {
        let store = match self.open_store() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut out: Vec<GameStatLine> = Vec::new();
        for entry in store.manifest().list(DataKind::Boxscore) {
            let DataKey::Game(game_id) = entry.key else { continue };

            // Boxscore manifest paths are
            // `data/boxscores/YYYY-MM-DD/<id>.json` — pull the
            // date out of the parent dir.
            let date_str = match entry
                .path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
            {
                Some(s) => s,
                None => continue,
            };
            let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => continue,
            };

            let raw = match store.load_boxscore_raw(DataKey::Game(game_id)) {
                Some(r) => r,
                None => continue,
            };
            let parsed = parse_boxscore(&raw, game_id.0);

            // Find this player's skater line in either roster.
            let line_opt = parsed
                .away_skaters
                .iter()
                .chain(parsed.home_skaters.iter())
                .find(|s| s.player_id == player_id);

            if let Some(s) = line_opt {
                out.push(GameStatLine {
                    player_id,
                    date,
                    game_id: game_id.0,
                    team_abbrev: s.team_abbrev.clone(),
                    goals: s.goals,
                    assists: s.assists,
                    plus_minus: s.plus_minus,
                    sog: s.sog,
                    hits: s.hits,
                    blocked_shots: s.blocked_shots,
                    takeaways: s.takeaways,
                    giveaways: s.giveaways,
                    pim: s.pim,
                    toi_seconds: s.toi_seconds,
                });
            }
        }

        // Sort ascending by date — the aggregator depends on this.
        out.sort_by(|a, b| a.date.cmp(&b.date));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L1 — provider returns empty Vec for a player with no
    /// boxscore data on disk (e.g. fresh tempdir). The executor
    /// fail-closed default then evaluates SlidingWindow atoms
    /// to false.
    #[test]
    fn l1_provider_empty_when_no_boxscores() {
        let dir = tempfile::tempdir().expect("tempdir");
        let provider = IcelinesProvider::new(dir.path().join(".icelines").join("data"));
        let lines = provider.fetch_game_lines(8478402, 20252026);
        assert!(lines.is_empty());
    }

    /// L1 — `ensure` is a no-op in A.2.4. Verifies the contract
    /// so future callers don't surprise themselves.
    #[test]
    fn l1_provider_ensure_is_noop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let provider = IcelinesProvider::new(dir.path().to_path_buf());
        let req = PlanRequirement::default();
        let mut events: Vec<FetchEvent> = Vec::new();
        let result = provider.ensure(&req, &mut |e| events.push(e));
        assert!(result.is_ok());
        assert!(events.is_empty());
    }

    /// L0 — IcelinesProvider impls DataProvider. Compile-time
    /// check via trait object.
    #[test]
    fn l0_provider_implements_trait() {
        let dir = tempfile::tempdir().expect("tempdir");
        let provider = IcelinesProvider::new(dir.path().to_path_buf());
        let _: &dyn DataProvider = &provider;
    }
}
