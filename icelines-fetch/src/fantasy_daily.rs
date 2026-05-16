use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use icelines_core::identity::GameId;
use icelines_core::model::Season;
use icelines_core::name::normalize_name;
use icelines_core::scheme::Scheme;
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::{
    FantasyDailyDeltaInput, FantasyDailyDeltaView, FantasyDailyLineInput, FantasyDailyPlayerInput,
    FantasyDailyTeamInput, SourceKind, SourceState,
};

use crate::boxscore_to_night_line::{extract_goalie_line, extract_skater_line};
use crate::datastore::DataStore;
use crate::fantasy_db::{FantasyDb, FantasyLeagueSnapshot};
use crate::manifest::{DataKey, DataKind, ManifestEntry};
use crate::nhl_api::parse_boxscore;

pub fn build_fantasy_daily_delta_view(
    db: &FantasyDb,
    store: &DataStore,
    date: NaiveDate,
    season: Season,
    season_type: SeasonType,
    league_name: Option<&str>,
) -> Result<FantasyDailyDeltaView> {
    let snapshot = db.league_snapshot(league_name)?;
    let scheme = Scheme::builtin_named(&snapshot.scoring_scheme).with_context(|| {
        format!(
            "unknown fantasy scoring scheme '{}' for league '{}'",
            snapshot.scoring_scheme, snapshot.league
        )
    })?;
    let mut warnings = Vec::new();
    let (lines, source_state) = cached_daily_lines(store, date, &mut warnings);

    let teams = daily_teams_from_snapshot(&snapshot, &lines);
    Ok(FantasyDailyDeltaView::from_input(
        FantasyDailyDeltaInput {
            season,
            season_type,
            date,
            league: snapshot.league,
            scoring_scheme: snapshot.scoring_scheme,
            teams,
            warnings,
            source_state,
        },
        &scheme,
    ))
}

fn daily_teams_from_snapshot(
    snapshot: &FantasyLeagueSnapshot,
    lines: &HashMap<String, ResolvedDailyLine>,
) -> Vec<FantasyDailyTeamInput> {
    snapshot
        .teams
        .iter()
        .map(|team| FantasyDailyTeamInput {
            team: team.name.clone(),
            owner: team.owner.clone(),
            is_user_team: team.name == snapshot.user_team,
            roster: team
                .roster
                .iter()
                .map(|roster_key| {
                    let normalized = normalize_name(roster_key);
                    let resolved = lines.get(&normalized);
                    FantasyDailyPlayerInput {
                        display_name: resolved
                            .map(|line| line.display_name.clone())
                            .unwrap_or_else(|| display_name_from_roster_key(roster_key)),
                        roster_key: roster_key.clone(),
                        position: resolved
                            .map(|line| line.position.clone())
                            .unwrap_or_else(|| "?".to_string()),
                        line: resolved.map(|line| line.line.clone()),
                    }
                })
                .collect(),
        })
        .collect()
}

fn cached_daily_lines(
    store: &DataStore,
    date: NaiveDate,
    warnings: &mut Vec<String>,
) -> (HashMap<String, ResolvedDailyLine>, Vec<SourceState>) {
    let wanted = date.format("%Y-%m-%d").to_string();
    let mut saw_date_entry = false;
    let mut loaded_boxscores = 0usize;
    let mut lines = HashMap::new();

    for entry in store.manifest().list(DataKind::Boxscore) {
        let path_date = entry_date(&entry);
        if path_date
            .as_deref()
            .is_some_and(|entry_date| entry_date != wanted)
        {
            continue;
        }

        let Some(game_id) = game_id_from_entry(&entry) else {
            continue;
        };
        let Some(raw) = store.load_boxscore_raw(DataKey::Game(GameId(game_id))) else {
            if path_date.as_deref() == Some(wanted.as_str()) {
                saw_date_entry = true;
                warnings.push(format!(
                    "cached boxscore manifest entry for game {game_id} is missing or invalid"
                ));
            }
            continue;
        };
        let raw_date = raw["gameDate"].as_str().map(str::to_owned).or(path_date);
        if raw_date.as_deref() != Some(wanted.as_str()) {
            continue;
        }
        saw_date_entry = true;
        loaded_boxscores += 1;

        let boxscore = parse_boxscore(&raw, game_id);
        for skater in boxscore
            .away_skaters
            .iter()
            .chain(boxscore.home_skaters.iter())
        {
            if skater.player_name.trim().is_empty() {
                continue;
            }
            let Some(line) = extract_skater_line(&boxscore, skater.player_id) else {
                continue;
            };
            lines.insert(
                normalize_name(&skater.player_name),
                ResolvedDailyLine {
                    display_name: skater.player_name.clone(),
                    position: skater.position.clone(),
                    line: FantasyDailyLineInput::Skater(line),
                },
            );
        }
        for goalie in &boxscore.goalies {
            if goalie.player_name.trim().is_empty() {
                continue;
            }
            let Some(line) = extract_goalie_line(&boxscore, goalie.player_id, &goalie.player_name)
            else {
                continue;
            };
            lines.insert(
                normalize_name(&goalie.player_name),
                ResolvedDailyLine {
                    display_name: goalie.player_name.clone(),
                    position: "G".to_string(),
                    line: FantasyDailyLineInput::Goalie(line),
                },
            );
        }
    }

    let source_state = if loaded_boxscores > 0 {
        vec![
            SourceState::complete(SourceKind::FantasyImport),
            SourceState::complete(SourceKind::Boxscore),
        ]
    } else {
        warnings.push(if saw_date_entry {
            format!("no readable cached boxscores found for {wanted}")
        } else {
            format!("no cached boxscores found for {wanted}")
        });
        vec![
            SourceState::complete(SourceKind::FantasyImport),
            SourceState::missing(SourceKind::Boxscore),
        ]
    };
    (lines, source_state)
}

#[derive(Clone)]
struct ResolvedDailyLine {
    display_name: String,
    position: String,
    line: FantasyDailyLineInput,
}

fn game_id_from_entry(entry: &ManifestEntry) -> Option<u64> {
    match entry.key {
        DataKey::Game(GameId(game_id)) => Some(game_id),
        _ => None,
    }
}

fn entry_date(entry: &ManifestEntry) -> Option<String> {
    entry
        .path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

fn display_name_from_roster_key(roster_key: &str) -> String {
    roster_key
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atomic_write::write_bytes_atomic;
    use crate::manifest::ManifestEntry;
    use icelines_core::{FetchSource, Freshness, Ttl};

    fn setup_db(mark_user_team: bool) -> FantasyDb {
        let db = FantasyDb::open_in_memory().expect("open db");
        let league_id = db
            .create_league("Daily League", "yahoo-standard")
            .expect("create league");
        db.set_active_league("Daily League")
            .expect("set active league");
        let my_team = db
            .create_team(&league_id, "My Team", "Me")
            .expect("create my team");
        let rival = db
            .create_team(&league_id, "Rival", "Them")
            .expect("create rival");
        if mark_user_team {
            db.set_user_team(&league_id, "My Team")
                .expect("mark user team");
        }
        db.add_player(&my_team, "matty beniers")
            .expect("add skater");
        db.add_player(&my_team, "joey daccord").expect("add goalie");
        db.add_player(&rival, "missing player")
            .expect("add missing player");
        db
    }

    fn store_with_boxscore(date: &str, state: &str) -> (tempfile::TempDir, DataStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::open(dir.path()).expect("open store");
        let game_id = 2025020342;
        let path = dir
            .path()
            .join("boxscores")
            .join(date)
            .join(format!("{game_id}.json"));
        let raw = serde_json::json!({
            "id": game_id,
            "gameDate": date,
            "gameState": state,
            "gameOutcome": { "lastPeriodType": "REG" },
            "awayTeam": { "abbrev": "VAN", "score": 1 },
            "homeTeam": { "abbrev": "SEA", "score": 4 },
            "playerByGameStats": {
                "awayTeam": { "forwards": [], "defense": [], "goalies": [] },
                "homeTeam": {
                    "forwards": [{
                        "playerId": 8482665,
                        "name": { "default": "Matty Beniers" },
                        "position": "C",
                        "toi": "18:20",
                        "goals": 2,
                        "assists": 1,
                        "plusMinus": 1,
                        "sog": 5,
                        "hits": 3,
                        "blockedShots": 2,
                        "takeaways": 1,
                        "giveaways": 0,
                        "pim": 0
                    }],
                    "defense": [],
                    "goalies": [{
                        "playerId": 8478916,
                        "name": { "default": "Joey Daccord" },
                        "saves": 30,
                        "shotsAgainst": 31,
                        "decision": "W"
                    }]
                }
            }
        });
        write_bytes_atomic(&path, raw.to_string().as_bytes()).expect("write boxscore");
        store
            .manifest()
            .upsert(
                DataKind::Boxscore,
                ManifestEntry {
                    key: DataKey::Game(GameId(game_id)),
                    path,
                    freshness: Freshness {
                        fetched_at: chrono::Utc::now(),
                        source: FetchSource::Live,
                        ttl: Ttl::Static,
                    },
                },
            )
            .expect("upsert manifest");
        (dir, store)
    }

    #[test]
    fn l1_fantasy_daily_delta_scores_cached_final_boxscore() {
        let db = setup_db(true);
        let (_dir, store) = store_with_boxscore("2026-01-15", "FINAL");
        let view = build_fantasy_daily_delta_view(
            &db,
            &store,
            NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
            Season(20252026),
            SeasonType::Regular,
            None,
        )
        .expect("daily view");

        assert_eq!(view.league, "Daily League");
        assert_eq!(view.teams[0].team, "My Team");
        assert_eq!(view.teams[0].scored_players, 2);
        // Matty: 2G*3 + 1A*2 + 3H*0.5 + 2BLK*0.5 = 10.5.
        // Daccord: W*5 + 30 saves*0.15 + 1 GA*-1 = 8.5.
        assert!((view.teams[0].daily_points - 19.0).abs() < 0.001);
        assert_eq!(view.teams[0].players[0].display_name, "Matty Beniers");
        assert_eq!(
            view.source_state[1].state,
            icelines_core::Completeness::Complete
        );
    }

    #[test]
    fn l1_fantasy_daily_delta_missing_cache_is_partial_not_zero_success() {
        let db = setup_db(true);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::open(dir.path()).expect("open store");
        let view = build_fantasy_daily_delta_view(
            &db,
            &store,
            NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
            Season(20252026),
            SeasonType::Regular,
            None,
        )
        .expect("daily view");

        assert_eq!(
            view.context.completeness,
            icelines_core::Completeness::Partial
        );
        assert_eq!(
            view.source_state[1].state,
            icelines_core::Completeness::Unavailable
        );
        assert!(
            view.warnings
                .iter()
                .any(|warning| warning.contains("no cached boxscores")),
            "missing cache must be explicit"
        );
    }

    #[test]
    fn l1_fantasy_daily_delta_unfinalized_cache_does_not_count() {
        let db = setup_db(true);
        let (_dir, store) = store_with_boxscore("2026-01-15", "LIVE");
        let view = build_fantasy_daily_delta_view(
            &db,
            &store,
            NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
            Season(20252026),
            SeasonType::Regular,
            None,
        )
        .expect("daily view");

        let my_team = view
            .teams
            .iter()
            .find(|team| team.team == "My Team")
            .expect("my team row");
        assert_eq!(my_team.scored_players, 0);
        assert_eq!(my_team.daily_points, 0.0);
        assert!(
            view.warnings
                .iter()
                .any(|warning| warning.contains("not finalized")),
            "unfinalized game line must be explicit"
        );
    }

    #[test]
    fn l1_fantasy_daily_delta_requires_user_team() {
        let db = setup_db(false);
        let (_dir, store) = store_with_boxscore("2026-01-15", "FINAL");
        let err = build_fantasy_daily_delta_view(
            &db,
            &store,
            NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
            Season(20252026),
            SeasonType::Regular,
            None,
        )
        .expect_err("missing user team should error");
        assert!(
            err.to_string().contains("no user team marked"),
            "unexpected error: {err}"
        );
    }
}
