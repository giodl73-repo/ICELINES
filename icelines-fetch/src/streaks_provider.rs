use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::{PlayerGameLineInput, PlayerShotLineInput};

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind, ManifestEntry};
use crate::nhl_api::{parse_boxscore, parse_play_by_play, Boxscore, PlayByPlay, SkaterLine};
use icelines_core::identity::GameId;
use std::collections::BTreeMap;

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

pub fn load_player_shot_lines(
    store: &DataStore,
    player_id: u32,
) -> (Vec<PlayerShotLineInput>, bool) {
    let mut source_loaded = false;
    let mut lines = Vec::new();
    for entry in store.manifest().list(DataKind::PlayByPlay) {
        let Some(game_id) = game_id_from_entry(&entry.key) else {
            continue;
        };
        let Some(raw) = store.load_play_by_play_raw(DataKey::Game(GameId(game_id))) else {
            continue;
        };
        source_loaded = true;
        let parsed = parse_play_by_play(&raw, game_id);
        let date = parsed.game_date.clone().or_else(|| entry_date(&entry));
        let boxscore = read_boxscore_for_game(store, game_id);
        lines.extend(
            shot_lines_from_play_by_play(&parsed, date, boxscore.as_ref())
                .into_iter()
                .filter(|line| line.player_id == player_id),
        );
    }
    lines.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.game_id.cmp(&b.game_id)));
    (lines, source_loaded)
}

pub fn load_team_shot_lines(
    store: &DataStore,
    team: &str,
    season: Season,
    season_type: SeasonType,
) -> (Vec<PlayerShotLineInput>, bool) {
    let team = team.trim().to_ascii_uppercase();
    let mut source_loaded = false;
    let mut lines = Vec::new();
    for entry in store.manifest().list(DataKind::PlayByPlay) {
        let Some(game_id) = game_id_from_entry(&entry.key) else {
            continue;
        };
        if !game_id_matches_window(game_id, season, season_type) {
            continue;
        }
        let Some(raw) = store.load_play_by_play_raw(DataKey::Game(GameId(game_id))) else {
            continue;
        };
        let parsed = parse_play_by_play(&raw, game_id);
        if !(parsed.away_abbrev.eq_ignore_ascii_case(&team)
            || parsed.home_abbrev.eq_ignore_ascii_case(&team))
        {
            continue;
        }
        source_loaded = true;
        let date = parsed.game_date.clone().or_else(|| entry_date(&entry));
        let boxscore = read_boxscore_for_game(store, game_id);
        lines.extend(
            shot_lines_from_play_by_play(&parsed, date, boxscore.as_ref())
                .into_iter()
                .filter(|line| line.team.eq_ignore_ascii_case(&team)),
        );
    }
    lines.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.game_id.cmp(&b.game_id))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    (lines, source_loaded)
}

fn read_boxscore(entry: &ManifestEntry) -> Option<(Boxscore, Option<String>)> {
    let game_id = game_id_from_entry(&entry.key)?;
    let bytes = std::fs::read(&entry.path).ok()?;
    let raw = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    let date = raw["gameDate"].as_str().map(str::to_owned);
    Some((parse_boxscore(&raw, game_id), date))
}

fn read_boxscore_for_game(store: &DataStore, game_id: u64) -> Option<(Boxscore, Option<String>)> {
    let key = DataKey::Game(GameId(game_id));
    let entry = store.manifest().get(DataKind::Boxscore, &key)?;
    read_boxscore(&entry)
}

fn entry_date(entry: &ManifestEntry) -> Option<String> {
    entry
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(str::to_owned)
}

fn shot_lines_from_play_by_play(
    parsed: &PlayByPlay,
    date: Option<String>,
    boxscore: Option<&(Boxscore, Option<String>)>,
) -> Vec<PlayerShotLineInput> {
    let mut by_player: BTreeMap<(u64, u32), PlayerShotLineInput> = BTreeMap::new();
    if let Some((boxscore, boxscore_date)) = boxscore {
        let line_date = date.clone().or_else(|| boxscore_date.clone());
        for skater in boxscore
            .away_skaters
            .iter()
            .chain(boxscore.home_skaters.iter())
        {
            let opponent = opponent_for_team(
                &boxscore.away_abbrev,
                &boxscore.home_abbrev,
                &skater.team_abbrev,
            );
            by_player.insert(
                (boxscore.game_id, skater.player_id),
                PlayerShotLineInput {
                    game_id: boxscore.game_id,
                    date: line_date.clone(),
                    player_id: skater.player_id,
                    player_name: skater.player_name.clone(),
                    team: skater.team_abbrev.clone(),
                    opponent,
                    shots_on_goal: 0,
                    shot_attempts: 0,
                },
            );
        }
    }

    for event in &parsed.scoring_events {
        let Some(player_id) = event.scoring_attempt_player_id() else {
            continue;
        };
        let team = event
            .event_owner_team_abbrev
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let opponent = opponent_for_team(&parsed.away_abbrev, &parsed.home_abbrev, &team);
        let line = by_player
            .entry((event.game_id, player_id))
            .or_insert_with(|| PlayerShotLineInput {
                game_id: event.game_id,
                date: event.date.clone().or_else(|| date.clone()),
                player_id,
                player_name: player_id.to_string(),
                team: team.clone(),
                opponent,
                shots_on_goal: 0,
                shot_attempts: 0,
            });
        if event.kind.counts_as_shot_on_goal() {
            line.shots_on_goal += 1;
        }
        if event.kind.counts_as_attempt() {
            line.shot_attempts += 1;
        }
    }

    by_player.into_values().collect()
}

fn opponent_for_team(away: &str, home: &str, team: &str) -> String {
    if team.eq_ignore_ascii_case(away) {
        home.to_string()
    } else if team.eq_ignore_ascii_case(home) {
        away.to_string()
    } else {
        "unknown".to_string()
    }
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

    #[test]
    fn l1_load_player_shot_lines_aggregates_play_by_play_with_zero_breaks() {
        let tmp = tempdir().unwrap();
        let store = DataStore::open(tmp.path()).unwrap();
        for game_id in 2025020001..=2025020003 {
            upsert_boxscore(
                &store,
                game_id,
                write_boxscore(
                    tmp.path().join(format!("{game_id}-boxscore.json")),
                    &format!(
                        r#"{{
                            "gameDate":"2025-10-0{}",
                            "awayTeam":{{"abbrev":"SEA","score":1}},
                            "homeTeam":{{"abbrev":"EDM","score":2}},
                            "playerByGameStats":{{
                              "awayTeam":{{"forwards":[],"defense":[]}},
                              "homeTeam":{{"forwards":[{{"playerId":97,"name":{{"default":"Test Player"}},"position":"C","goals":0,"assists":0}}],"defense":[]}}
                            }}
                        }}"#,
                        game_id - 2025020000
                    ),
                ),
            );
        }
        upsert_play_by_play(
            &store,
            2025020001,
            write_play_by_play(
                tmp.path().join("2025020001-play.json"),
                r#"{
                    "id":2025020001,
                    "gameDate":"2025-10-01",
                    "awayTeam":{"id":55,"abbrev":"SEA"},
                    "homeTeam":{"id":22,"abbrev":"EDM"},
                    "plays":[{
                      "eventId":1,
                      "periodDescriptor":{"number":1,"periodType":"REG"},
                      "timeInPeriod":"01:00",
                      "typeDescKey":"shot-on-goal",
                      "details":{"eventOwnerTeamId":22,"shootingPlayerId":97}
                    }]
                }"#,
            ),
        );
        upsert_play_by_play(
            &store,
            2025020002,
            write_play_by_play(
                tmp.path().join("2025020002-play.json"),
                r#"{
                    "id":2025020002,
                    "gameDate":"2025-10-02",
                    "awayTeam":{"id":55,"abbrev":"SEA"},
                    "homeTeam":{"id":22,"abbrev":"EDM"},
                    "plays":[]
                }"#,
            ),
        );
        upsert_play_by_play(
            &store,
            2025020003,
            write_play_by_play(
                tmp.path().join("2025020003-play.json"),
                r#"{
                    "id":2025020003,
                    "gameDate":"2025-10-03",
                    "awayTeam":{"id":55,"abbrev":"SEA"},
                    "homeTeam":{"id":22,"abbrev":"EDM"},
                    "plays":[{
                      "eventId":3,
                      "periodDescriptor":{"number":1,"periodType":"REG"},
                      "timeInPeriod":"03:00",
                      "typeDescKey":"missed-shot",
                      "details":{"eventOwnerTeamId":22,"shootingPlayerId":97}
                    }]
                }"#,
            ),
        );

        let (lines, source_loaded) = load_player_shot_lines(&store, 97);

        assert!(source_loaded);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].shots_on_goal, 1);
        assert_eq!(lines[0].shot_attempts, 1);
        assert_eq!(lines[1].shots_on_goal, 0);
        assert_eq!(lines[1].shot_attempts, 0);
        assert_eq!(lines[2].shots_on_goal, 0);
        assert_eq!(lines[2].shot_attempts, 1);
        let view = icelines_core::PlayerStreaksView::from_game_and_shot_lines(
            icelines_core::ViewContext::new(icelines_core::ViewWindow::new(
                Season(20252026),
                SeasonType::Regular,
            )),
            97,
            "Test Player",
            &[],
            &lines,
            source_loaded,
        );
        let attempts = view
            .rows
            .iter()
            .find(|row| row.metric == "shot-attempts")
            .unwrap();
        assert_eq!(attempts.longest, 1);
        assert_eq!(attempts.current, 1);
    }

    #[test]
    fn l1_load_team_shot_lines_filters_team_and_window() {
        let tmp = tempdir().unwrap();
        let store = DataStore::open(tmp.path()).unwrap();
        upsert_boxscore(
            &store,
            2025020001,
            write_boxscore(
                tmp.path().join("edm-boxscore.json"),
                r#"{
                    "gameDate":"2025-10-01",
                    "awayTeam":{"abbrev":"SEA","score":1},
                    "homeTeam":{"abbrev":"EDM","score":2},
                    "playerByGameStats":{
                      "awayTeam":{"forwards":[],"defense":[]},
                      "homeTeam":{"forwards":[{"playerId":97,"name":{"default":"Home Player"},"position":"C","goals":0,"assists":0},{"playerId":10,"name":{"default":"Zero Player"},"position":"C","goals":0,"assists":0}],"defense":[]}
                    }
                }"#,
            ),
        );
        upsert_boxscore(
            &store,
            2025020002,
            write_boxscore(
                tmp.path().join("sea-boxscore.json"),
                r#"{
                    "gameDate":"2025-10-02",
                    "awayTeam":{"abbrev":"EDM","score":1},
                    "homeTeam":{"abbrev":"SEA","score":2},
                    "playerByGameStats":{
                      "awayTeam":{"forwards":[],"defense":[]},
                      "homeTeam":{"forwards":[{"playerId":97,"name":{"default":"Traded Player"},"position":"C","goals":0,"assists":0}],"defense":[]}
                    }
                }"#,
            ),
        );
        upsert_play_by_play(
            &store,
            2025020001,
            write_play_by_play(
                tmp.path().join("edm-play.json"),
                r#"{
                    "id":2025020001,
                    "gameDate":"2025-10-01",
                    "awayTeam":{"id":55,"abbrev":"SEA"},
                    "homeTeam":{"id":22,"abbrev":"EDM"},
                    "plays":[{
                      "eventId":1,
                      "periodDescriptor":{"number":1,"periodType":"REG"},
                      "timeInPeriod":"01:00",
                      "typeDescKey":"shot-on-goal",
                      "details":{"eventOwnerTeamId":22,"shootingPlayerId":97}
                    }]
                }"#,
            ),
        );
        upsert_play_by_play(
            &store,
            2025020002,
            write_play_by_play(
                tmp.path().join("sea-play.json"),
                r#"{
                    "id":2025020002,
                    "gameDate":"2025-10-02",
                    "awayTeam":{"id":22,"abbrev":"EDM"},
                    "homeTeam":{"id":55,"abbrev":"SEA"},
                    "plays":[{
                      "eventId":2,
                      "periodDescriptor":{"number":1,"periodType":"REG"},
                      "timeInPeriod":"02:00",
                      "typeDescKey":"shot-on-goal",
                      "details":{"eventOwnerTeamId":55,"shootingPlayerId":97}
                    }]
                }"#,
            ),
        );

        let (lines, source_loaded) =
            load_team_shot_lines(&store, "EDM", Season(20252026), SeasonType::Regular);

        assert!(source_loaded);
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.team == "EDM"));
        let home_player = lines.iter().find(|line| line.player_id == 97).unwrap();
        assert_eq!(home_player.shots_on_goal, 1);
        let zero_player = lines.iter().find(|line| line.player_id == 10).unwrap();
        assert_eq!(zero_player.shot_attempts, 0);
    }

    fn write_boxscore(path: std::path::PathBuf, body: &str) -> std::path::PathBuf {
        std::fs::write(&path, body).unwrap();
        path
    }

    fn write_play_by_play(path: std::path::PathBuf, body: &str) -> std::path::PathBuf {
        std::fs::write(&path, body).unwrap();
        path
    }

    fn upsert_boxscore(store: &DataStore, game_id: u64, path: std::path::PathBuf) {
        upsert_manifest(store, DataKind::Boxscore, game_id, path);
    }

    fn upsert_play_by_play(store: &DataStore, game_id: u64, path: std::path::PathBuf) {
        upsert_manifest(store, DataKind::PlayByPlay, game_id, path);
    }

    fn upsert_manifest(store: &DataStore, kind: DataKind, game_id: u64, path: std::path::PathBuf) {
        store
            .manifest()
            .upsert(
                kind,
                ManifestEntry {
                    key: DataKey::Game(GameId(game_id)),
                    path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();
    }
}
