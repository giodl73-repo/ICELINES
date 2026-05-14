use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::PlayerGameLineInput;

use crate::datastore::DataStore;
use crate::manifest::{DataKind, ManifestEntry};
use crate::nhl_api::{parse_boxscore, Boxscore, SkaterLine};

pub fn load_player_game_lines(store: &DataStore, player_id: u32) -> Vec<PlayerGameLineInput> {
    let mut lines = Vec::new();
    for entry in store.manifest().list(DataKind::Boxscore) {
        if let Some((boxscore, date)) = read_boxscore(&entry) {
            lines.extend(
                boxscore
                    .away_skaters
                    .iter()
                    .chain(boxscore.home_skaters.iter())
                    .filter(|line| line.player_id == player_id)
                    .map(|line| game_line_from_skater(line, &boxscore, date.clone())),
            );
        }
    }
    lines.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.game_id.cmp(&b.game_id)));
    lines
}

pub fn load_team_game_lines(
    store: &DataStore,
    team: &str,
    season: Season,
    season_type: SeasonType,
) -> Vec<PlayerGameLineInput> {
    let team = team.trim().to_ascii_uppercase();
    let mut lines = Vec::new();
    for entry in store.manifest().list(DataKind::Boxscore) {
        let Some(game_id) = game_id_from_entry(&entry.key) else {
            continue;
        };
        if !game_id_matches_window(game_id, season, season_type) {
            continue;
        }
        let Some((boxscore, date)) = read_boxscore(&entry) else {
            continue;
        };
        lines.extend(
            boxscore
                .away_skaters
                .iter()
                .chain(boxscore.home_skaters.iter())
                .filter(|line| line.team_abbrev.eq_ignore_ascii_case(&team))
                .map(|line| game_line_from_skater(line, &boxscore, date.clone())),
        );
    }
    lines.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.game_id.cmp(&b.game_id))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    lines
}

fn read_boxscore(entry: &ManifestEntry) -> Option<(Boxscore, Option<String>)> {
    let game_id = game_id_from_entry(&entry.key)?;
    let bytes = std::fs::read(&entry.path).ok()?;
    let raw = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    let date = raw["gameDate"].as_str().map(str::to_owned);
    Some((parse_boxscore(&raw, game_id), date))
}

fn game_line_from_skater(
    line: &SkaterLine,
    boxscore: &Boxscore,
    date: Option<String>,
) -> PlayerGameLineInput {
    let opponent = if line.team_abbrev == boxscore.home_abbrev {
        boxscore.away_abbrev.clone()
    } else {
        boxscore.home_abbrev.clone()
    };
    PlayerGameLineInput {
        game_id: boxscore.game_id,
        date,
        player_id: line.player_id,
        player_name: line.player_name.clone(),
        team: line.team_abbrev.clone(),
        opponent,
        goals: line.goals,
        assists: line.assists,
    }
}

fn game_id_matches_window(game_id: u64, season: Season, season_type: SeasonType) -> bool {
    let start_year = season.0 / 10_000;
    let game_year = (game_id / 1_000_000) as u32;
    let game_type = ((game_id / 10_000) % 100) as u8;
    let expected_type = match season_type {
        SeasonType::Regular => 2,
        SeasonType::Playoff => 3,
    };
    game_year == start_year && game_type == expected_type
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

    #[test]
    fn l1_load_team_game_lines_filters_team_and_window() {
        let tmp = tempdir().unwrap();
        let store = DataStore::open(tmp.path()).unwrap();
        let edm_path = write_boxscore(
            tmp.path().join("edm.json"),
            r#"{
                "gameDate":"2025-10-10",
                "awayTeam":{"abbrev":"SEA","score":1},
                "homeTeam":{"abbrev":"EDM","score":2},
                "playerByGameStats":{
                  "awayTeam":{"forwards":[{"playerId":10,"name":{"default":"Away Player"},"position":"C","goals":1,"assists":0}],"defense":[]},
                  "homeTeam":{"forwards":[{"playerId":97,"name":{"default":"Home Player"},"position":"C","goals":1,"assists":2}],"defense":[]}
                }
            }"#,
        );
        let playoff_path = write_boxscore(
            tmp.path().join("playoff.json"),
            r#"{
                "gameDate":"2026-04-20",
                "awayTeam":{"abbrev":"EDM","score":1},
                "homeTeam":{"abbrev":"SEA","score":2},
                "playerByGameStats":{
                  "awayTeam":{"forwards":[{"playerId":97,"name":{"default":"Home Player"},"position":"C","goals":1,"assists":0}],"defense":[]},
                  "homeTeam":{"forwards":[],"defense":[]}
                }
            }"#,
        );
        store
            .manifest()
            .upsert(
                DataKind::Boxscore,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025020001)),
                    path: edm_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Boxscore,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025030001)),
                    path: playoff_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();

        let lines = load_team_game_lines(&store, "EDM", Season(20252026), SeasonType::Regular);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].player_id, 97);
        assert_eq!(lines[0].opponent, "SEA");
    }

    fn write_boxscore(path: std::path::PathBuf, body: &str) -> std::path::PathBuf {
        std::fs::write(&path, body).unwrap();
        path
    }
}
