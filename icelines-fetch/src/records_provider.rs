use std::path::PathBuf;

use anyhow::{Context, Result};
use icelines_core::{FightRecordInput, PlayerGoalRecordInput};

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind};
use crate::nhl_api::{parse_boxscore, parse_play_by_play};

pub fn default_data_root() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("cannot determine home directory")?;
    Ok(home.join(".icelines").join("data"))
}

pub fn open_default_store() -> Result<DataStore> {
    let data_root = default_data_root()?;
    DataStore::open(&data_root).context("open DataStore")
}

pub fn load_goal_record_inputs_from_default_store() -> Result<Vec<PlayerGoalRecordInput>> {
    let store = open_default_store()?;
    load_goal_record_inputs(&store)
}

pub fn load_play_by_play_goal_record_inputs_from_default_store(
) -> Result<Vec<PlayerGoalRecordInput>> {
    let store = open_default_store()?;
    load_play_by_play_goal_record_inputs(&store)
}

pub fn load_fight_record_inputs_from_default_store() -> Result<Vec<FightRecordInput>> {
    let store = open_default_store()?;
    load_fight_record_inputs(&store)
}

pub fn load_goal_record_inputs(store: &DataStore) -> Result<Vec<PlayerGoalRecordInput>> {
    let mut out = Vec::new();
    for entry in store.manifest().list(DataKind::Boxscore) {
        let DataKey::Game(game_id) = entry.key else {
            continue;
        };
        let date = entry
            .path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(str::to_owned);
        let Some(raw) = store.load_boxscore_raw(DataKey::Game(game_id)) else {
            continue;
        };
        let parsed = parse_boxscore(&raw, game_id.0);
        for goal in parsed.goals {
            let opponent_team = if goal.scorer_team == parsed.home_abbrev {
                parsed.away_abbrev.clone()
            } else if goal.scorer_team == parsed.away_abbrev {
                parsed.home_abbrev.clone()
            } else {
                String::new()
            };
            out.push(PlayerGoalRecordInput {
                game_id: game_id.0,
                date: date.clone(),
                scorer_id: goal.scorer_id,
                scorer_name: goal.scorer_name,
                scorer_team: goal.scorer_team,
                opponent_team,
                period: goal.period,
                time_in_period: goal.time_in_period,
                goalie_id: None,
                goalie_name: None,
                empty_net: false,
            });
        }
    }
    Ok(out)
}

pub fn load_play_by_play_goal_record_inputs(
    store: &DataStore,
) -> Result<Vec<PlayerGoalRecordInput>> {
    let goalie_names = goalie_name_map();
    let scorer_names = skater_name_map();
    let mut out = Vec::new();
    for entry in store.manifest().list(DataKind::PlayByPlay) {
        let DataKey::Game(game_id) = entry.key else {
            continue;
        };
        let date = entry
            .path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(str::to_owned);
        let Some(raw) = store.load_play_by_play_raw(DataKey::Game(game_id)) else {
            continue;
        };
        let parsed = parse_play_by_play(&raw, game_id.0);
        for goal in &parsed.goals {
            let scorer_team = team_abbrev_for_id(&parsed, goal.event_owner_team_id);
            let opponent_team = opponent_team_for(&parsed, &scorer_team);
            out.push(PlayerGoalRecordInput {
                game_id: game_id.0,
                date: date.clone().or_else(|| parsed.game_date.clone()),
                scorer_id: goal.scoring_player_id,
                scorer_name: goal
                    .scoring_player_id
                    .and_then(|id| scorer_names.get(&id).cloned())
                    .unwrap_or_default(),
                scorer_team,
                opponent_team,
                period: goal.period,
                time_in_period: goal.time_in_period.clone(),
                goalie_id: goal.goalie_in_net_id,
                goalie_name: goal
                    .goalie_in_net_id
                    .and_then(|id| goalie_names.get(&id).cloned()),
                empty_net: goal.goalie_in_net_id.is_none(),
            });
        }
    }
    Ok(out)
}

fn team_abbrev_for_id(parsed: &crate::nhl_api::PlayByPlay, team_id: Option<u32>) -> String {
    match team_id {
        Some(id) if Some(id) == parsed.home_team_id => parsed.home_abbrev.clone(),
        Some(id) if Some(id) == parsed.away_team_id => parsed.away_abbrev.clone(),
        _ => String::new(),
    }
}

fn opponent_team_for(parsed: &crate::nhl_api::PlayByPlay, scorer_team: &str) -> String {
    if scorer_team == parsed.home_abbrev {
        parsed.away_abbrev.clone()
    } else if scorer_team == parsed.away_abbrev {
        parsed.home_abbrev.clone()
    } else {
        String::new()
    }
}

fn goalie_name_map() -> std::collections::HashMap<u32, String> {
    let mut names = std::collections::HashMap::new();
    for season in crate::bundled::BUNDLED_SEASONS {
        if let Some(goalies) = crate::bundled::get_goalie_stats(season) {
            for goalie in goalies {
                names
                    .entry(goalie.player_id)
                    .or_insert(goalie.goalie_full_name);
            }
        }
    }
    names
}

fn skater_name_map() -> std::collections::HashMap<u32, String> {
    let mut names = std::collections::HashMap::new();
    for season in crate::bundled::BUNDLED_SEASONS {
        if let Some(bios) = crate::bundled::get_bios(season) {
            for bio in bios {
                names.entry(bio.player_id).or_insert(bio.skater_full_name);
            }
        }
    }
    names
}

pub fn load_fight_record_inputs(store: &DataStore) -> Result<Vec<FightRecordInput>> {
    let names = person_name_map();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for entry in store.manifest().list(DataKind::PlayByPlay) {
        let DataKey::Game(game_id) = entry.key else {
            continue;
        };
        let date = entry
            .path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(str::to_owned);
        let Some(raw) = store.load_play_by_play_raw(DataKey::Game(game_id)) else {
            continue;
        };
        let parsed = parse_play_by_play(&raw, game_id.0);
        for penalty in &parsed.penalties {
            if penalty.desc_key.as_deref() != Some("fighting") {
                continue;
            }
            let (Some(committed), Some(drawn)) =
                (penalty.committed_by_player_id, penalty.drawn_by_player_id)
            else {
                continue;
            };
            let low = committed.min(drawn);
            let high = committed.max(drawn);
            let key = (
                game_id.0,
                penalty.period,
                penalty.time_in_period.clone(),
                low,
                high,
                "fighting",
            );
            if !seen.insert(key) {
                continue;
            }

            let player_team = team_abbrev_for_id(&parsed, penalty.event_owner_team_id);
            let opponent_team = opponent_team_for(&parsed, &player_team);
            let committed_name = names.get(&committed).cloned().unwrap_or_default();
            let drawn_name = names.get(&drawn).cloned().unwrap_or_default();
            let fight_date = date.clone().or_else(|| parsed.game_date.clone());

            out.push(FightRecordInput {
                game_id: game_id.0,
                date: fight_date.clone(),
                player_id: committed,
                player_name: committed_name.clone(),
                player_team: player_team.clone(),
                opponent_id: drawn,
                opponent_name: drawn_name.clone(),
                opponent_team: opponent_team.clone(),
                period: penalty.period,
                time_in_period: penalty.time_in_period.clone(),
            });
            out.push(FightRecordInput {
                game_id: game_id.0,
                date: fight_date,
                player_id: drawn,
                player_name: drawn_name,
                player_team: opponent_team,
                opponent_id: committed,
                opponent_name: committed_name,
                opponent_team: player_team,
                period: penalty.period,
                time_in_period: penalty.time_in_period.clone(),
            });
        }
    }
    Ok(out)
}

fn person_name_map() -> std::collections::HashMap<u32, String> {
    let mut names = skater_name_map();
    for (id, name) in goalie_name_map() {
        names.entry(id).or_insert(name);
    }
    names
}

#[cfg(test)]
mod tests {
    use super::load_fight_record_inputs;
    use crate::atomic_write::write_bytes_atomic;
    use crate::datastore::DataStore;
    use crate::manifest::{DataKey, DataKind, ManifestEntry};
    use icelines_core::identity::GameId;

    #[test]
    fn l1_fight_records_dedupe_reciprocal_penalties() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::open(dir.path()).unwrap();
        let path = dir
            .path()
            .join("play_by_play")
            .join("2026-01-15")
            .join("2025020342.json");
        let body = serde_json::json!({
            "id": 2025020342,
            "gameDate": "2026-01-15",
            "awayTeam": { "id": 1, "abbrev": "SEA" },
            "homeTeam": { "id": 2, "abbrev": "EDM" },
            "plays": [
                {
                    "eventId": 1,
                    "periodDescriptor": { "number": 1, "periodType": "REG" },
                    "timeInPeriod": "10:20",
                    "typeDescKey": "penalty",
                    "details": {
                        "eventOwnerTeamId": 1,
                        "typeCode": "MAJ",
                        "descKey": "fighting",
                        "duration": 5,
                        "committedByPlayerId": 10,
                        "drawnByPlayerId": 20
                    }
                },
                {
                    "eventId": 2,
                    "periodDescriptor": { "number": 1, "periodType": "REG" },
                    "timeInPeriod": "10:20",
                    "typeDescKey": "penalty",
                    "details": {
                        "eventOwnerTeamId": 2,
                        "typeCode": "MAJ",
                        "descKey": "fighting",
                        "duration": 5,
                        "committedByPlayerId": 20,
                        "drawnByPlayerId": 10
                    }
                }
            ]
        });
        write_bytes_atomic(&path, body.to_string().as_bytes()).unwrap();
        store
            .manifest()
            .upsert(
                DataKind::PlayByPlay,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025020342)),
                    path,
                    freshness: icelines_core::Freshness {
                        fetched_at: chrono::Utc::now(),
                        source: icelines_core::FetchSource::Live,
                        ttl: icelines_core::Ttl::Static,
                    },
                },
            )
            .unwrap();

        let fights = load_fight_record_inputs(&store).unwrap();

        assert_eq!(
            fights.len(),
            2,
            "one fight should produce two directed rows"
        );
        assert_eq!(fights[0].player_id, 10);
        assert_eq!(fights[0].opponent_id, 20);
        assert_eq!(fights[0].player_team, "SEA");
        assert_eq!(fights[0].opponent_team, "EDM");
    }
}
