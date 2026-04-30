//! Goalie data loading — the parallel of `PlayerRepository` but for the
//! goalie pool. Phase G.1.
//!
//! `RosterResponse` already includes a `goalies: Vec<RosterPlayer>` field
//! (the data path that `PlayerRepository` drops on the floor). This
//! repository picks them up, joins with the bundled or snapshotted
//! `goalie-stats.json`, and produces a `Vec<Goalie>` ready for the TUI
//! Goalies tab + the CLI `query goalies` command (G.5).

use crate::{
    bundled,
    error::FetchError,
    schema::{GoalieStats, RosterResponse},
    snapshot::{SnapshotStore, SnapshotTier},
};
use icelines_core::{
    model::{Goalie, GoalieBio, GoalieSeasonStats, TeamAbbr},
    name::normalize_name,
};
use std::collections::HashMap;

use crate::teams::ALL_NHL_TEAMS as ALL_TEAMS;

pub struct GoalieRepository {
    store:  SnapshotStore,
    season: String,
}

impl GoalieRepository {
    pub fn new(store: SnapshotStore, season: impl Into<String>) -> Self {
        Self { store, season: season.into() }
    }

    /// Resolve the goalie-stats vector for the configured season.
    /// Falls through chunked → legacy snapshot → embedded → installed
    /// per `bundled::load_goalies_with_fallback`.
    pub fn stats(&self) -> Vec<GoalieStats> {
        bundled::load_goalies_with_fallback(&self.season, &self.store)
            .unwrap_or_default()
    }

    /// Read one team's roster from the snapshot store. Returns None
    /// when the team's roster hasn't been fetched yet.
    fn roster(&self, team: &str) -> Option<RosterResponse> {
        self.store.read_tier(&SnapshotTier::Rosters, &format!("{team}.json")).ok()
    }

    /// Build a `Vec<Goalie>` for the entire league. Joins each team's
    /// roster goalie list with the season-stats vector by `playerId`.
    /// Roster goalies whose `playerId` doesn't appear in the stats
    /// vector are still included with `stats = None` (rookies / call-ups
    /// who haven't played yet).
    ///
    /// When no rosters are available (cold start / bundled-only build)
    /// falls back to constructing goalies directly from the stats
    /// vector. Each gets a synthesised `team` from the stats row's
    /// primary team.
    pub fn load_all(&self) -> Result<Vec<Goalie>, FetchError> {
        let stats = self.stats();
        let stats_idx: HashMap<u32, &GoalieStats> = stats.iter()
            .map(|s| (s.player_id, s))
            .collect();

        let has_rosters = ALL_TEAMS.iter().any(|t| self.roster(t).is_some());

        let mut out: Vec<Goalie> = Vec::new();
        if has_rosters {
            for team in ALL_TEAMS {
                let Some(roster) = self.roster(team) else { continue };
                let team_abbr = TeamAbbr(team.to_string());
                for rp in &roster.goalies {
                    let stats_row = stats_idx.get(&rp.id).copied();
                    out.push(make_goalie(rp, stats_row, &team_abbr));
                }
            }
        } else {
            // No roster snapshots — synthesise goalies straight from the
            // stats vector. Used by tests + cold-start runs that only
            // have bundled data.
            for s in &stats {
                let team = TeamAbbr(s.primary_team().to_string());
                out.push(make_goalie_from_stats_only(s, &team));
            }
        }

        // Defensive dedup — a mid-season trade can leave the same
        // playerId on two team rosters in the same snapshot.
        out.sort_by_key(|g| g.nhl_id);
        out.dedup_by_key(|g| g.nhl_id);
        // Re-sort by SV% descending (qualifying first) for the default
        // leaderboard order. Caller can re-sort as needed.
        out.sort_by(|a, b| {
            let av = a.stats.as_ref().and_then(|s| s.save_pct).unwrap_or(0.0);
            let bv = b.stats.as_ref().and_then(|s| s.save_pct).unwrap_or(0.0);
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(out)
    }

    /// Load goalies for one team. Returns an empty Vec when the team's
    /// roster isn't snapshotted and no stats row carries the team in
    /// its `team_abbrevs`.
    pub fn load_team(&self, abbrev: &str) -> Result<Vec<Goalie>, FetchError> {
        let stats = self.stats();
        let stats_idx: HashMap<u32, &GoalieStats> = stats.iter()
            .map(|s| (s.player_id, s))
            .collect();

        let team_abbr = TeamAbbr(abbrev.to_string());
        let mut out: Vec<Goalie> = Vec::new();

        if let Some(roster) = self.roster(abbrev) {
            for rp in &roster.goalies {
                let stats_row = stats_idx.get(&rp.id).copied();
                out.push(make_goalie(rp, stats_row, &team_abbr));
            }
        } else {
            for s in &stats {
                if s.team_abbrevs.split(',').any(|t| t.trim() == abbrev) {
                    out.push(make_goalie_from_stats_only(s, &team_abbr));
                }
            }
        }
        Ok(out)
    }
}

fn make_goalie(
    rp: &crate::schema::RosterPlayer,
    stats: Option<&GoalieStats>,
    team: &TeamAbbr,
) -> Goalie {
    let full_name = format!("{} {}", rp.first_name, rp.last_name);
    let bio = GoalieBio {
        birth_date:        rp.birth_date.clone(),
        birth_country:     rp.birth_country.clone(),
        nationality_code:  rp.birth_country.clone(),
        catches:           rp.shoots_catches.clone(),
        height_in_inches:  rp.height_in_inches,
        weight_lbs:        rp.weight_in_pounds,
        draft_year:        None,
        draft_round:       None,
        draft_overall:     None,
        rookie_season:     None,
    };
    Goalie {
        nhl_id:          rp.id,
        full_name:       full_name.clone(),
        name_normalized: normalize_name(&full_name),
        team:            team.clone(),
        stats:           stats.map(season_stats_from),
        bio,
        headshot_url:    rp.headshot.clone(),
        sweater_number:  rp.sweater_number,
    }
}

fn make_goalie_from_stats_only(s: &GoalieStats, team: &TeamAbbr) -> Goalie {
    let bio = GoalieBio {
        birth_date:        None,
        birth_country:     None,
        nationality_code:  None,
        catches:           s.shoots_catches.clone(),
        height_in_inches:  None,
        weight_lbs:        None,
        draft_year:        None,
        draft_round:       None,
        draft_overall:     None,
        rookie_season:     None,
    };
    Goalie {
        nhl_id:          s.player_id,
        full_name:       s.goalie_full_name.clone(),
        name_normalized: normalize_name(&s.goalie_full_name),
        team:            team.clone(),
        stats:           Some(season_stats_from(s)),
        bio,
        headshot_url:    None,
        sweater_number:  None,
    }
}

fn season_stats_from(s: &GoalieStats) -> GoalieSeasonStats {
    GoalieSeasonStats {
        games_played:          s.games_played,
        games_started:         s.games_started,
        wins:                  s.wins,
        losses:                s.losses,
        ot_losses:             s.ot_losses,
        ties:                  s.ties,
        shots_against:         s.shots_against,
        goals_against:         s.goals_against,
        saves:                 s.saves,
        save_pct:              s.save_pct,
        goals_against_average: s.goals_against_average,
        shutouts:              s.shutouts,
        time_on_ice:           s.time_on_ice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn empty_repo() -> (TempDir, GoalieRepository) {
        let dir = TempDir::new().unwrap();
        let store = SnapshotStore::new(dir.path());
        let repo  = GoalieRepository::new(store, "20242025");
        (dir, repo)
    }

    #[test]
    fn l1_goalie_repo_loads_from_bundled_when_no_rosters() {
        // No roster snapshots → repo synthesises goalies from the
        // bundled goalie-stats.json. 20242025 has 103 goalies.
        let (_keep, repo) = empty_repo();
        let goalies = repo.load_all().expect("load");
        assert!(goalies.len() >= 60,
            "expected current-season goalie count, got {}", goalies.len());
        // Hellebuyck (id 8476945) must be in there with 47 wins.
        let hb = goalies.iter().find(|g| g.nhl_id == 8476945)
            .expect("Hellebuyck in 24-25 fixture");
        assert_eq!(hb.full_name, "Connor Hellebuyck");
        assert_eq!(hb.team.as_str(), "WPG");
        let stats = hb.stats.as_ref().expect("Hellebuyck has stats");
        assert_eq!(stats.wins, 47);
        assert_eq!(stats.shutouts, 8);
        assert!(stats.save_pct.unwrap() > 0.92);
        assert!(stats.save_pct.unwrap() < 0.93);
    }

    #[test]
    fn l1_goalie_repo_default_sort_is_save_pct_descending() {
        let (_keep, repo) = empty_repo();
        let goalies = repo.load_all().expect("load");
        // Top of list has highest SV%.
        let svs: Vec<f32> = goalies.iter()
            .filter_map(|g| g.stats.as_ref().and_then(|s| s.save_pct))
            .collect();
        for w in svs.windows(2) {
            assert!(w[0] >= w[1],
                "expected SV% descending, got {} before {}", w[0], w[1]);
        }
    }

    #[test]
    fn l1_goalie_repo_load_team_filters_by_abbrev() {
        // No rosters available — the cold-start path filters by
        // team_abbrevs. WPG should yield Hellebuyck (and his backup).
        let (_keep, repo) = empty_repo();
        let wpg = repo.load_team("WPG").expect("load WPG");
        assert!(wpg.iter().any(|g| g.full_name == "Connor Hellebuyck"),
            "expected Hellebuyck on WPG, got {:?}",
            wpg.iter().map(|g| &g.full_name).collect::<Vec<_>>());
        // Every returned goalie's team should be WPG.
        for g in &wpg {
            assert_eq!(g.team.as_str(), "WPG");
        }
    }

    #[test]
    fn l1_goalie_repo_dedup_by_nhl_id() {
        // Whether or not the season has trades, every loaded goalie must
        // be unique by nhl_id. This is the safety net behind the
        // load_all dedup_by_key call.
        let (_keep, repo) = empty_repo();
        let goalies = repo.load_all().expect("load");
        let ids: std::collections::HashSet<u32> = goalies.iter().map(|g| g.nhl_id).collect();
        assert_eq!(ids.len(), goalies.len(),
            "every loaded goalie has a unique nhl_id");
    }

    #[test]
    fn l1_goalie_repo_traded_goalie_appears_once_with_primary_team() {
        // 24-25 bundled data contains real mid-season trades —
        // James Reimer (8473503) is listed as "ANA,BUF" because he
        // started in Anaheim and was traded to Buffalo. The repo must:
        //   - emit exactly one Goalie struct for him (no double-count)
        //   - point his team at the primary (first listed) abbrev
        //   - still let load_team("BUF") find him via the comma-split path
        let (_keep, repo) = empty_repo();
        let goalies = repo.load_all().expect("load");
        let reimer_rows: Vec<_> = goalies.iter()
            .filter(|g| g.nhl_id == 8473503)
            .collect();
        assert_eq!(
            reimer_rows.len(), 1,
            "traded goalie must dedup to one row, got {}",
            reimer_rows.len(),
        );
        let reimer = reimer_rows[0];
        // ANA is the first listed — that's the primary stop.
        assert_eq!(reimer.team.as_str(), "ANA",
            "primary team must be the first comma-listed abbrev");

        // load_team must see him on BOTH stops via the comma-split
        // fallback, since the filter happens against team_abbrevs.
        let on_ana = repo.load_team("ANA").expect("load ANA");
        let on_buf = repo.load_team("BUF").expect("load BUF");
        assert!(on_ana.iter().any(|g| g.nhl_id == 8473503),
            "Reimer must be findable on ANA");
        assert!(on_buf.iter().any(|g| g.nhl_id == 8473503),
            "Reimer must be findable on BUF after the trade");
    }

    #[test]
    fn l1_goalie_repo_no_team_abbrevs_string_leaks_to_team_field() {
        // Defensive: regardless of whether a goalie was traded, the
        // `team` field on the Goalie struct must be a plain 3-letter
        // abbrev — never the raw "EDM,PIT" string. UI columns assume a
        // bounded width and would mis-render otherwise.
        let (_keep, repo) = empty_repo();
        let goalies = repo.load_all().expect("load");
        for g in &goalies {
            assert!(
                !g.team.as_str().contains(','),
                "goalie {} ({}) leaked a comma into team field: '{}'",
                g.full_name, g.nhl_id, g.team.as_str(),
            );
            assert!(
                g.team.as_str().len() <= 3,
                "goalie {} team abbrev '{}' is longer than 3 chars",
                g.full_name, g.team.as_str(),
            );
        }
    }

    #[test]
    fn l1_goalie_qualifying_default_threshold() {
        let (_keep, repo) = empty_repo();
        let goalies = repo.load_all().expect("load");
        // 15-GP threshold pulls the Vezina-eligible field. There should
        // be more than one starter but well fewer than the full pool.
        let qualified: Vec<_> = goalies.iter().filter(|g| g.qualified(15)).collect();
        assert!(qualified.len() >= 20,
            "20+ goalies should clear 15 GP, got {}", qualified.len());
        assert!(qualified.len() < goalies.len(),
            "not every roster goalie clears 15 GP");
    }
}
