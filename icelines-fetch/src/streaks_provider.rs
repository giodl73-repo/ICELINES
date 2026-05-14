use icelines_core::view_model::PlayerGameLineInput;

use crate::datastore::DataStore;
use crate::manifest::DataKind;
use crate::nhl_api::parse_boxscore;

pub fn load_player_game_lines(store: &DataStore, player_id: u32) -> Vec<PlayerGameLineInput> {
    let mut lines = Vec::new();
    for entry in store.manifest().list(DataKind::Boxscore) {
        let Ok(bytes) = std::fs::read(&entry.path) else {
            continue;
        };
        let Ok(raw) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Some(game_id) = game_id_from_entry(&entry.key) else {
            continue;
        };
        let date = raw["gameDate"].as_str().map(str::to_owned);
        let boxscore = parse_boxscore(&raw, game_id);

        for line in boxscore
            .away_skaters
            .iter()
            .chain(boxscore.home_skaters.iter())
            .filter(|line| line.player_id == player_id)
        {
            let opponent = if line.team_abbrev == boxscore.home_abbrev {
                boxscore.away_abbrev.clone()
            } else {
                boxscore.home_abbrev.clone()
            };
            lines.push(PlayerGameLineInput {
                game_id,
                date: date.clone(),
                player_id: line.player_id,
                player_name: line.player_name.clone(),
                team: line.team_abbrev.clone(),
                opponent,
                goals: line.goals,
                assists: line.assists,
            });
        }
    }
    lines.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.game_id.cmp(&b.game_id)));
    lines
}

fn game_id_from_entry(key: &crate::manifest::DataKey) -> Option<u64> {
    match key {
        crate::manifest::DataKey::Game(id) => Some(id.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datastore::DataStore;
    use crate::manifest::{DataKey, DataKind, ManifestEntry};
    use chrono::Utc;
    use icelines_core::freshness::{FetchSource, Freshness, Ttl};
    use icelines_core::identity::GameId;
    use tempfile::tempdir;

    #[test]
    fn l1_load_player_game_lines_reads_cached_boxscores() {
        let tmp = tempdir().unwrap();
        let store = DataStore::open(tmp.path()).unwrap();
        let body_path = tmp.path().join("boxscore.json");
        std::fs::write(
            &body_path,
            r#"{
                "gameDate":"2025-10-10",
                "awayTeam":{"abbrev":"SEA","score":1},
                "homeTeam":{"abbrev":"EDM","score":2},
                "playerByGameStats":{
                  "awayTeam":{"forwards":[],"defense":[]},
                  "homeTeam":{"forwards":[{"playerId":97,"name":{"default":"Test Player"},"position":"C","goals":1,"assists":2}],"defense":[]}
                }
            }"#,
        )
        .unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Boxscore,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025020001)),
                    path: body_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();

        let lines = load_player_game_lines(&store, 97);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].opponent, "SEA");
        assert_eq!(lines[0].goals, 1);
        assert_eq!(lines[0].assists, 2);
    }
}
