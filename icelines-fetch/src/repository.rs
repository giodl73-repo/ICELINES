//! PlayerRepository — single, authoritative data-loading API for all commands.
//!
//! Every command that needs player data calls this instead of reaching into
//! the snapshot store or bundled data directly. The fallback chain is owned
//! here and tested here; commands just ask for players.
//!
//! Fallback chain:
//!   1. ~/.icelines/snapshots/ (fresh from `icelines fetch`)
//!   2. Binary-bundled data (ships with every release, 5 seasons)
//!   3. Err — tell the user to run icelines fetch

use icelines_core::model::{Player, Season, TeamAbbr};
use std::collections::HashMap;

use crate::{
    bundled,
    error::FetchError,
    moneypuck::MoneyPuckStats,
    player_builder::{build_players, build_players_from_bios, index_bios, index_stats},
    schema::{PlayerContract, RosterResponse, SkaterBio, SkaterRealtime, SkaterStats},
    snapshot::{SnapshotStore, SnapshotTier},
};

const ALL_TEAMS: &[&str] = &[
    "ANA","BOS","BUF","CAR","CBJ","CGY","CHI","COL","DAL","DET","EDM","FLA","LAK",
    "MIN","MTL","NJD","NSH","NYI","NYR","OTT","PHI","PIT","SEA","SJS","STL","TBL",
    "TOR","UTA","VAN","VGK","WPG","WSH",
];

pub struct PlayerRepository {
    store:  SnapshotStore,
    season: String,
}

impl PlayerRepository {
    pub fn new(store: SnapshotStore, season: impl Into<String>) -> Self {
        Self { store, season: season.into() }
    }

    // ── Core load ─────────────────────────────────────────────────────────────

    /// Load bios via fallback chain. Never panics.
    pub fn bios(&self) -> Result<Vec<SkaterBio>, FetchError> {
        bundled::load_bios_with_fallback(&self.season, &self.store)
    }

    /// Load stats via fallback chain. Returns empty vec if unavailable.
    pub fn stats(&self) -> Vec<SkaterStats> {
        bundled::load_stats_with_fallback(&self.season, &self.store)
            .unwrap_or_default()
    }

    /// Load realtime stats from snapshot. Returns empty map if not fetched.
    pub fn realtime(&self) -> HashMap<u32, SkaterRealtime> {
        self.store
            .read_tier::<Vec<SkaterRealtime>>(&SnapshotTier::Realtime, "realtime.json")
            .unwrap_or_default()
            .into_iter()
            .map(|r| (r.player_id, r))
            .collect()
    }

    /// Load MoneyPuck stats from snapshot. Returns empty map if not fetched.
    pub fn moneypuck(&self) -> HashMap<u32, MoneyPuckStats> {
        self.store
            .read_tier::<Vec<MoneyPuckStats>>(&SnapshotTier::MoneyPuck, "moneypuck.json")
            .unwrap_or_default()
            .into_iter()
            .map(|s| (s.player_id, s))
            .collect()
    }

    /// Load contract data from snapshot. Returns empty map if not fetched.
    pub fn contracts(&self) -> HashMap<u32, PlayerContract> {
        self.store
            .read_tier::<Vec<PlayerContract>>(&SnapshotTier::Contracts, "contracts.json")
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.player_id, c))
            .collect()
    }

    /// Load roster for one team from snapshot. Returns None if not available.
    pub fn roster(&self, team: &str) -> Option<RosterResponse> {
        self.store.read_tier(&SnapshotTier::Rosters, &format!("{team}.json")).ok()
    }

    // ── High-level player loading ─────────────────────────────────────────────

    /// Load all players across all 32 teams, sorted by pace descending.
    /// Works immediately after install (bundled data), no fetch required.
    pub fn load_all(&self) -> Result<Vec<Player>, FetchError> {
        let bios        = self.bios()?;
        let stats       = self.stats();
        let realtime    = self.realtime();
        let mp          = self.moneypuck();
        let contracts   = self.contracts();
        let bio_idx     = index_bios(&bios);
        let stats_idx   = index_stats(&stats);
        let season      = Season(self.season.parse().unwrap_or(icelines_core::CURRENT_SEASON));

        let has_rosters = ALL_TEAMS.iter()
            .any(|t| self.roster(t).is_some());

        let mut players = if has_rosters {
            ALL_TEAMS.iter().flat_map(|t| {
                let Some(r) = self.roster(t) else { return vec![] };
                let team = TeamAbbr(t.to_string());
                let mut v = build_players(&r.forwards,   &bio_idx, &stats_idx, &realtime, &mp, &contracts, season, &team);
                v.extend(build_players(&r.defensemen, &bio_idx, &stats_idx, &realtime, &mp, &contracts, season, &team));
                v
            }).collect()
        } else {
            // No roster snapshot — build from bios (bundled data cold-start path)
            build_players_from_bios(&bios, &stats_idx, &realtime, &mp, &contracts, season)
        };

        // Safety-net dedup: roster path can also produce duplicates when a player
        // appears in multiple teams' rosters (trade edge case in snapshot data).
        dedup_by_nhl_id(&mut players);

        icelines_core::scoring::sort_by_pace(&mut players);
        Ok(players)
    }

    /// Load players for a single team.
    pub fn load_team(&self, abbrev: &str) -> Result<Vec<Player>, FetchError> {
        let bios        = self.bios()?;
        let stats       = self.stats();
        let realtime    = self.realtime();
        let mp          = self.moneypuck();
        let contracts   = self.contracts();
        let bio_idx     = index_bios(&bios);
        let stats_idx   = index_stats(&stats);
        let season      = Season(self.season.parse().unwrap_or(icelines_core::CURRENT_SEASON));
        let team        = TeamAbbr(abbrev.to_string());

        let players = if let Some(r) = self.roster(abbrev) {
            let mut v = build_players(&r.forwards,   &bio_idx, &stats_idx, &realtime, &mp, &contracts, season, &team);
            v.extend(build_players(&r.defensemen, &bio_idx, &stats_idx, &realtime, &mp, &contracts, season, &team));
            v
        } else {
            // No roster snapshot — filter bundled bios by team
            build_players_from_bios(&bios, &stats_idx, &realtime, &mp, &contracts, season)
                .into_iter()
                .filter(|p| p.team.as_str() == abbrev)
                .collect()
        };

        Ok(players)
    }
}

/// Remove duplicate players (same nhl_id) keeping the first occurrence.
/// Trades produce multiple rows in the NHL API; after building we may have
/// a player listed twice. The roster path already deduplicates at the bio
/// level, but this is a final safety net.
fn dedup_by_nhl_id(players: &mut Vec<Player>) {
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    players.retain(|p| {
        match p.nhl_id {
            Some(id) => seen.insert(id),
            None     => true, // keep players without an ID (shouldn't happen)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn repo() -> (TempDir, PlayerRepository) {
        let dir   = TempDir::new().unwrap();
        let store = SnapshotStore::new(dir.path());
        let repo  = PlayerRepository::new(store, icelines_core::CURRENT_SEASON_STR);
        (dir, repo)
    }

    #[test]
    fn l1_repository_load_all_from_bundled_no_snapshot() {
        let (_dir, repo) = repo();
        // No snapshot — must fall back to bundled data and succeed
        let players = repo.load_all().expect("load_all must succeed with bundled data");
        assert!(players.len() > 500,
            "expected 900+ players from bundled data, got {}", players.len());
    }

    #[test]
    fn l1_repository_players_sorted_by_pace() {
        let (_dir, repo) = repo();
        let players = repo.load_all().unwrap();
        let paces: Vec<f64> = players.iter()
            .filter_map(|p| p.pace_score.map(|s| s.pace_82))
            .take(10).collect();
        let is_sorted = paces.windows(2).all(|w| w[0] >= w[1]);
        assert!(is_sorted, "top 10 players must be sorted by pace desc");
    }

    #[test]
    fn l1_repository_load_team_sea() {
        let (_dir, repo) = repo();
        let players = repo.load_team("SEA").unwrap();
        // All players must be on SEA
        assert!(players.iter().all(|p| p.team.as_str() == "SEA"),
            "load_team(SEA) must only return SEA players");
        assert!(!players.is_empty(), "SEA must have players in bundled data");
    }

    #[test]
    fn l1_repository_unknown_season_fallback() {
        let dir   = TempDir::new().unwrap();
        let store = SnapshotStore::new(dir.path());
        let far_future = PlayerRepository::new(store, "20991000");
        assert!(far_future.load_all().is_err(),
            "unknown season with no snapshot must return error");
    }

    #[test]
    fn l1_repository_realtime_empty_when_no_snapshot() {
        let (_dir, repo) = repo();
        let rt = repo.realtime();
        assert!(rt.is_empty(), "realtime must be empty HashMap without a snapshot");
    }

    #[test]
    fn l1_repository_moneypuck_empty_when_no_snapshot() {
        let (_dir, repo) = repo();
        let mp = repo.moneypuck();
        assert!(mp.is_empty(), "moneypuck must be empty HashMap without a snapshot");
    }

    #[test]
    fn l1_repository_contracts_empty_when_no_snapshot() {
        let (_dir, repo) = repo();
        let c = repo.contracts();
        assert!(c.is_empty(), "contracts must be empty HashMap without a snapshot");
    }

    #[test]
    fn l1_repository_player_contract_fields_none_when_not_fetched() {
        let (_dir, repo) = repo();
        let players = repo.load_all().unwrap();
        assert!(
            players.iter().all(|p| p.contract_expiry_year.is_none()),
            "contract_expiry_year must be None without contracts snapshot"
        );
        assert!(
            players.iter().all(|p| p.expiry_type.is_none()),
            "expiry_type must be None without contracts snapshot"
        );
        assert!(
            players.iter().all(|p| p.salary.is_none()),
            "salary must be None without contracts snapshot"
        );
    }

    #[test]
    fn l1_repository_moneypuck_fields_none_when_not_fetched() {
        let (_dir, repo) = repo();
        let players = repo.load_all().unwrap();
        assert!(
            players.iter().all(|p| p.xg.is_none()),
            "xg must be None without MoneyPuck snapshot"
        );
        assert!(
            players.iter().all(|p| p.cf_pct_5v5.is_none()),
            "cf_pct_5v5 must be None without MoneyPuck snapshot"
        );
        assert!(
            players.iter().all(|p| p.ff_pct_5v5.is_none()),
            "ff_pct_5v5 must be None without MoneyPuck snapshot"
        );
        assert!(
            players.iter().all(|p| p.xgf_pct_5v5.is_none()),
            "xgf_pct_5v5 must be None without MoneyPuck snapshot"
        );
    }

    #[test]
    fn l1_repository_realtime_fields_zero_when_not_fetched() {
        let (_dir, repo) = repo();
        let players = repo.load_all().unwrap();
        // Without a realtime snapshot, all physical stats should be 0
        assert!(
            players.iter().all(|p| p.hits == 0),
            "hits must be 0 without realtime snapshot"
        );
        assert!(
            players.iter().all(|p| p.blocked_shots == 0),
            "blocked_shots must be 0 without realtime snapshot"
        );
        assert!(
            players.iter().all(|p| p.takeaways == 0),
            "takeaways must be 0 without realtime snapshot"
        );
        assert!(
            players.iter().all(|p| p.giveaways == 0),
            "giveaways must be 0 without realtime snapshot"
        );
    }

    #[test]
    fn l1_repository_bundled_players_have_positions() {
        let (_dir, repo) = repo();
        let players = repo.load_all().unwrap();
        // Every player built from bundled data should have a valid position
        assert!(
            !players.is_empty(),
            "bundled data must produce players"
        );
        // Verify we have forwards and defensemen in bundled data
        let has_forwards = players.iter().any(|p| p.position.is_forward());
        let has_defense = players.iter().any(|p| p.position.is_defense());
        assert!(has_forwards, "bundled data must include forwards");
        assert!(has_defense, "bundled data must include defensemen");
    }

    #[test]
    fn l1_repository_load_team_returns_only_team_players() {
        let (_dir, repo) = repo();
        // Load multiple teams and verify isolation
        for team in &["EDM", "TOR", "NYR"] {
            let players = repo.load_team(team).unwrap();
            assert!(
                players.iter().all(|p| p.team.as_str() == *team),
                "load_team({team}) must only return {team} players"
            );
        }
    }
}
