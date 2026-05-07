//! Phase Foster +20 — orchestrate Favorites group + boxscore JSON →
//! `FavoritesView`.
//!
//! Reads the user's Favorites group (player normalized names + team
//! abbrevs), resolves each player to a PlayerId via the bundled
//! bios index, walks today's NHL slate to find which games involve
//! the favorited entities, then loads each game's boxscore body
//! from disk (Foster +3 persists it under
//! `data/boxscores/<date>/<game_id>.json`) and pulls the per-night
//! line out via `boxscore_to_night_line::extract_skater_line`.
//!
//! Failure modes:
//! - Player not in bundled bios → silently dropped (no row)
//! - Team didn't play that night → `DnpReason::TeamBye`
//! - Boxscore not yet on disk → `DnpReason::DataPending` (the user
//!   can run `icelines fetch boxscore --date D` to populate)
//! - Player resolved + game on disk but they're not in the lineup
//!   → `DnpReason::Scratched`

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use chrono::NaiveDate;
use icelines_core::entity::EntityRef;
use icelines_core::favorites::{
    AggregateView, DnpReason, EventRow, FavoritesView, GameResult, PlayerNightRow, TeamNightRow,
};
use icelines_core::identity::PlayerId;
use icelines_core::model::TeamAbbr;
use icelines_core::timeframe::Timeframe;
use icelines_fetch::boxscore_to_night_line;
use icelines_fetch::manifest::{DataKey, DataKind};
use icelines_fetch::nhl_api::ScheduledGame;

use crate::db::{GroupDb, MemberKind};

/// Compute the populated `FavoritesView` for a single anchor date.
/// `slate` is the day's schedule (caller fetches via
/// `NhlApiClient::fetch_schedule_for_date` or pulls from cache);
/// `data_root` is `~/.icelines/data/` so we can read persisted
/// boxscore JSON via the manifest.
pub fn compute_favorites_view(
    db: &GroupDb,
    group: &str,
    date: NaiveDate,
    range: Timeframe,
    slate: &[ScheduledGame],
    data_root: &std::path::Path,
) -> Result<FavoritesView> {
    let members = db
        .list_members_with_kind(group)
        .context("read group members")?;

    // Split into favorited PIDs (resolved) + favorited teams.
    let mut favorited_pids: HashMap<PlayerId, String> = HashMap::new(); // pid → display name
    let mut favorited_teams: HashSet<String> = HashSet::new();
    for (key, kind) in &members {
        match kind {
            MemberKind::Team => {
                favorited_teams.insert(key.to_uppercase());
            }
            MemberKind::Player => {
                if let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(key) {
                    favorited_pids.insert(PlayerId(pid), key.clone());
                }
                // Unresolved player names (rookies, retired pre-bundle) get
                // dropped silently — augment_career_history_for_player
                // handles the lazy-fetch on add; here we just skip until
                // the bundled bios catch up.
            }
        }
    }

    // Open a DataStore so we can read persisted boxscore JSON via
    // the manifest. Keep it read-only — we don't mutate anything
    // during view computation.
    let store = icelines_fetch::datastore::DataStore::open(data_root)
        .context("open DataStore for favorites")?;

    // Index slate by team abbrev so we can answer "did EDM play
    // tonight, and if so what game_id?" in O(1).
    let mut games_by_team: HashMap<String, &ScheduledGame> = HashMap::new();
    for g in slate {
        games_by_team.insert(g.away_abbrev.to_uppercase(), g);
        games_by_team.insert(g.home_abbrev.to_uppercase(), g);
    }

    // ── Favorited players → PlayerNightRow per pid ─────────────────
    let mut player_rows: Vec<PlayerNightRow> = Vec::new();
    for (pid, display_name) in &favorited_pids {
        match resolve_player_row(*pid, display_name, &games_by_team, &store) {
            Ok(row) => player_rows.push(row),
            Err(reason) => player_rows.push(PlayerNightRow::DidNotPlay {
                player: EntityRef::Player(*pid),
                reason,
            }),
        }
    }

    // ── Favorited teams → TeamNightRow per team ────────────────────
    let mut team_rows: Vec<TeamNightRow> = Vec::new();
    for abbr in &favorited_teams {
        team_rows.push(resolve_team_row(abbr, &games_by_team));
    }

    // Stable sort: skater rows first by player display name (best-
    // effort via EntityRef Display), then DNPs.
    player_rows.sort_by_key(|r| match r {
        PlayerNightRow::Skater(s) => (0u8, s.player.to_string()),
        PlayerNightRow::Goalie(g) => (1u8, g.player.to_string()),
        PlayerNightRow::DidNotPlay { player, .. } => (2u8, player.to_string()),
    });
    team_rows.sort_by_key(|t| t.team_abbr.0.clone());

    let (range_start, range_end) = range.range(date);
    Ok(FavoritesView {
        date,
        range,
        players: player_rows,
        teams: team_rows,
        events: Vec::<EventRow>::new(),
        aggregate: AggregateView {
            range_start,
            range_end,
            player_rollups: Vec::new(),
            team_rollups: Vec::new(),
        },
    })
}

/// Resolve one favorited player to a `PlayerNightRow` or a
/// `DnpReason` on every miss path. Returns `Err(DnpReason)` so the
/// caller can wrap into `PlayerNightRow::DidNotPlay`.
fn resolve_player_row(
    pid: PlayerId,
    _display_name: &str,
    games_by_team: &HashMap<String, &ScheduledGame>,
    store: &icelines_fetch::datastore::DataStore,
) -> std::result::Result<PlayerNightRow, DnpReason> {
    // Find the player's current team via the bundled bios. Cheap
    // because resolve_player_id_by_name has already proven the bios
    // index has them.
    let team = bundled_player_team(pid).ok_or(DnpReason::DataPending)?;
    let game = match games_by_team.get(&team.to_uppercase()) {
        Some(g) => g,
        None => return Err(DnpReason::TeamBye),
    };
    // Is the boxscore body on disk?
    let raw = store
        .load_boxscore_raw(DataKey::Game(icelines_core::identity::GameId(game.game_id)))
        .ok_or(DnpReason::DataPending)?;
    // Re-parse — same routine the live fetcher uses.
    let parsed = icelines_fetch::nhl_api::parse_boxscore(&raw, game.game_id);
    match boxscore_to_night_line::extract_skater_line(&parsed, pid.0) {
        Some(line) => Ok(PlayerNightRow::Skater(line)),
        None => {
            // Player's team played but they're not in the lineup —
            // healthy scratch / IR / not on the active roster.
            Err(DnpReason::Scratched)
        }
    }
}

fn resolve_team_row(
    abbr: &str,
    games_by_team: &HashMap<String, &ScheduledGame>,
) -> TeamNightRow {
    let entity = EntityRef::Team(TeamAbbr(abbr.to_string()));
    let team_abbr = TeamAbbr(abbr.to_string());
    match games_by_team.get(abbr) {
        None => TeamNightRow {
            team: entity,
            team_abbr,
            score: String::new(),
            result: None,
            opponent: None,
            top_skater: None,
            top_goalie: None,
            on_bye: true,
        },
        Some(g) => {
            let is_home = g.home_abbrev.eq_ignore_ascii_case(abbr);
            let team_score = if is_home { g.home_score } else { g.away_score }
                .map(|s| s as u32);
            let opp_score = if is_home { g.away_score } else { g.home_score }
                .map(|s| s as u32);
            let opponent = TeamAbbr(if is_home {
                g.away_abbrev.clone()
            } else {
                g.home_abbrev.clone()
            });
            let score = match (team_score, opp_score) {
                (Some(t), Some(o)) => format!("{t}-{o}"),
                _ => String::new(),
            };
            let result = match (team_score, opp_score, g.game_state.as_deref()) {
                (Some(t), Some(o), Some("FINAL" | "OFF")) if t > o => Some(GameResult::Win),
                (Some(t), Some(o), Some("FINAL" | "OFF")) if t < o => Some(GameResult::Loss),
                (Some(_), Some(_), Some("FINAL" | "OFF")) => Some(GameResult::OtLoss),
                (Some(_), Some(_), Some("LIVE" | "CRIT")) => Some(GameResult::InProgress),
                _ => None,
            };
            TeamNightRow {
                team: entity,
                team_abbr,
                score,
                result,
                opponent: Some(opponent),
                top_skater: None,
                top_goalie: None,
                on_bye: false,
            }
        }
    }
}

/// Walk the bundled bios for the most recent season this PID
/// appears in and return their team abbrev. Returns `None` for PIDs
/// the bundle doesn't know about.
fn bundled_player_team(pid: PlayerId) -> Option<String> {
    for season in icelines_fetch::bundled::BUNDLED_SEASONS {
        if let Some(bios) = icelines_fetch::bundled::get_bios(season) {
            if let Some(b) = bios.iter().find(|b| b.player_id == pid.0) {
                if let Some(team) = &b.current_team_abbrev {
                    return Some(team.clone());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_fetch::nhl_api::ScheduledGame;

    fn fixture_game(date: &str, away: &str, home: &str) -> ScheduledGame {
        ScheduledGame {
            date: date.into(),
            game_id: 2025020001,
            game_type: 2,
            away_abbrev: away.into(),
            away_name: away.into(),
            home_abbrev: home.into(),
            home_name: home.into(),
            start_time_utc: format!("{date}T23:00:00Z"),
            away_score: Some(3),
            home_score: Some(7),
            game_state: Some("FINAL".into()),
            last_period: Some("REG".into()),
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    #[test]
    fn l0_foster_plus20_resolve_team_row_on_bye() {
        let games_by_team: HashMap<String, &ScheduledGame> = HashMap::new();
        let row = resolve_team_row("EDM", &games_by_team);
        assert!(row.on_bye);
        assert!(row.opponent.is_none());
        assert!(row.score.is_empty());
    }

    #[test]
    fn l0_foster_plus20_resolve_team_row_home_winner() {
        let g = fixture_game("2026-01-15", "CGY", "EDM");
        let mut games: HashMap<String, &ScheduledGame> = HashMap::new();
        games.insert("CGY".into(), &g);
        games.insert("EDM".into(), &g);
        let row = resolve_team_row("EDM", &games);
        assert!(!row.on_bye);
        assert_eq!(row.score, "7-3");
        assert!(matches!(row.result, Some(GameResult::Win)));
        assert_eq!(row.opponent.as_ref().unwrap().0, "CGY");
    }

    #[test]
    fn l0_foster_plus20_resolve_team_row_away_loser() {
        let g = fixture_game("2026-01-15", "CGY", "EDM");
        let mut games: HashMap<String, &ScheduledGame> = HashMap::new();
        games.insert("CGY".into(), &g);
        games.insert("EDM".into(), &g);
        let row = resolve_team_row("CGY", &games);
        assert_eq!(row.score, "3-7");
        assert!(matches!(row.result, Some(GameResult::Loss)));
        assert_eq!(row.opponent.as_ref().unwrap().0, "EDM");
    }

    #[test]
    fn l0_foster_plus20_resolve_team_row_in_progress() {
        let mut g = fixture_game("2026-01-15", "CGY", "EDM");
        g.game_state = Some("LIVE".into());
        let mut games: HashMap<String, &ScheduledGame> = HashMap::new();
        games.insert("EDM".into(), &g);
        let row = resolve_team_row("EDM", &games);
        assert!(matches!(row.result, Some(GameResult::InProgress)));
    }
}
