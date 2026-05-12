use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use icelines_core::model::Season;
use serde_json::Value;

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduleRemainingCache {
    pub remaining_by_team: HashMap<String, u32>,
    pub complete_teams: HashSet<String>,
}

impl ScheduleRemainingCache {
    pub fn is_empty(&self) -> bool {
        self.remaining_by_team.is_empty()
    }
}

pub fn default_data_root() -> Option<PathBuf> {
    std::env::var_os("ICELINES_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|home| PathBuf::from(home).join(".icelines").join("data"))
        })
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".icelines").join("data"))
        })
}

pub fn remaining_games_by_team_from_cache(season: Season) -> ScheduleRemainingCache {
    let Some(root) = default_data_root() else {
        return ScheduleRemainingCache::default();
    };
    if !root.exists() {
        return ScheduleRemainingCache::default();
    }
    let Ok(store) = DataStore::open(root) else {
        return ScheduleRemainingCache::default();
    };

    let mut cache = ScheduleRemainingCache::default();
    let mut seen_games = HashSet::new();

    for entry in store.manifest().list(DataKind::Schedule) {
        let Ok(bytes) = std::fs::read(&entry.path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let complete_source = matches!(entry.key, DataKey::Season(entry_season) if entry_season == season)
            || value.get("games").and_then(Value::as_array).is_some();
        ingest_schedule_value(&value, season, complete_source, &mut cache, &mut seen_games);
    }

    cache
}

pub fn remaining_games_from_schedule_value(
    value: &Value,
    season: Season,
    complete_source: bool,
) -> ScheduleRemainingCache {
    let mut cache = ScheduleRemainingCache::default();
    let mut seen_games = HashSet::new();
    ingest_schedule_value(value, season, complete_source, &mut cache, &mut seen_games);
    cache
}

fn ingest_schedule_value(
    value: &Value,
    season: Season,
    complete_source: bool,
    cache: &mut ScheduleRemainingCache,
    seen_games: &mut HashSet<u64>,
) {
    if let Some(games) = value.get("games").and_then(Value::as_array) {
        for game in games {
            ingest_schedule_game(game, season, complete_source, cache, seen_games);
        }
    }

    if let Some(game_week) = value.get("gameWeek").and_then(Value::as_array) {
        for day in game_week {
            if let Some(games) = day.get("games").and_then(Value::as_array) {
                for game in games {
                    ingest_schedule_game(game, season, complete_source, cache, seen_games);
                }
            }
        }
    }

    if let Some(games) = value.as_array() {
        for game in games {
            ingest_schedule_game(game, season, complete_source, cache, seen_games);
        }
    }
}

fn ingest_schedule_game(
    game: &Value,
    season: Season,
    complete_source: bool,
    cache: &mut ScheduleRemainingCache,
    seen_games: &mut HashSet<u64>,
) {
    if schedule_game_type(game).is_some_and(|game_type| game_type != 2) {
        return;
    }
    if schedule_game_is_final(game) {
        return;
    }
    if !schedule_game_date_matches_season(game, season) {
        return;
    }
    if let Some(game_id) = schedule_game_id(game) {
        if !seen_games.insert(game_id) {
            return;
        }
    }

    let Some(away) = schedule_game_team(game, "awayTeam", "away_abbrev") else {
        return;
    };
    let Some(home) = schedule_game_team(game, "homeTeam", "home_abbrev") else {
        return;
    };

    for team in [away, home] {
        *cache.remaining_by_team.entry(team.clone()).or_insert(0) += 1;
        if complete_source {
            cache.complete_teams.insert(team);
        }
    }
}

fn schedule_game_id(game: &Value) -> Option<u64> {
    game.get("id")
        .or_else(|| game.get("gameId"))
        .or_else(|| game.get("game_id"))
        .and_then(Value::as_u64)
}

fn schedule_game_type(game: &Value) -> Option<u64> {
    game.get("gameType")
        .or_else(|| game.get("game_type"))
        .and_then(Value::as_u64)
}

fn schedule_game_is_final(game: &Value) -> bool {
    let Some(state) = game
        .get("gameState")
        .or_else(|| game.get("game_state"))
        .and_then(Value::as_str)
    else {
        return false;
    };
    matches!(
        state.to_ascii_uppercase().as_str(),
        "FINAL" | "OFF" | "OFFICIAL"
    )
}

fn schedule_game_date_matches_season(game: &Value, season: Season) -> bool {
    let Some(date) = game
        .get("gameDate")
        .or_else(|| game.get("date"))
        .and_then(Value::as_str)
    else {
        return true;
    };
    let Some(year) = date.get(0..4).and_then(|value| value.parse::<u32>().ok()) else {
        return true;
    };
    let start_year = season.0 / 10000;
    let end_year = season.0 % 10000;
    year == start_year || year == end_year
}

fn schedule_game_team(game: &Value, raw_key: &str, normalized_key: &str) -> Option<String> {
    game.get(raw_key)
        .and_then(|team| team.get("abbrev"))
        .or_else(|| game.get(normalized_key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|team| !team.is_empty())
        .map(|team| team.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn l0_schedule_remaining_cache_counts_unplayed_regular_games() {
        let value = json!({
            "games": [
                {
                    "id": 2025020001u64,
                    "gameDate": "2026-03-15",
                    "gameType": 2,
                    "gameState": "FUT",
                    "awayTeam": { "abbrev": "EDM" },
                    "homeTeam": { "abbrev": "SEA" }
                },
                {
                    "id": 2025020001u64,
                    "gameDate": "2026-03-15",
                    "gameType": 2,
                    "gameState": "FUT",
                    "awayTeam": { "abbrev": "EDM" },
                    "homeTeam": { "abbrev": "SEA" }
                },
                {
                    "id": 2025020002u64,
                    "gameDate": "2026-03-16",
                    "gameType": 2,
                    "gameState": "FINAL",
                    "awayTeam": { "abbrev": "EDM" },
                    "homeTeam": { "abbrev": "CGY" }
                },
                {
                    "id": 2025030001u64,
                    "gameDate": "2026-04-20",
                    "gameType": 3,
                    "gameState": "FUT",
                    "awayTeam": { "abbrev": "EDM" },
                    "homeTeam": { "abbrev": "VGK" }
                },
                {
                    "id": 2024020001u64,
                    "gameDate": "2024-11-01",
                    "gameType": 2,
                    "gameState": "FUT",
                    "awayTeam": { "abbrev": "EDM" },
                    "homeTeam": { "abbrev": "VAN" }
                }
            ]
        });

        let cache = remaining_games_from_schedule_value(&value, Season(20252026), true);

        assert_eq!(cache.remaining_by_team.get("EDM"), Some(&1));
        assert_eq!(cache.remaining_by_team.get("SEA"), Some(&1));
        assert!(!cache.remaining_by_team.contains_key("CGY"));
        assert!(!cache.remaining_by_team.contains_key("VGK"));
        assert!(!cache.remaining_by_team.contains_key("VAN"));
        assert!(cache.complete_teams.contains("EDM"));
        assert!(cache.complete_teams.contains("SEA"));
    }
}
