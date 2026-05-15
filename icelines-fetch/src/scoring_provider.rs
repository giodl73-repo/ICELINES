use anyhow::Result;
use icelines_core::identity::GameId;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    GameScoringReportView, ScoringEventInput, ScoringEventSummary, TeamScoringProfileView,
    TonightFavoritePlayerScoringRow, TonightFavoriteTeamScoringRow, TonightScoringIntelView,
    ViewContext,
};

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind};
use crate::nhl_api::parse_play_by_play;

pub fn load_scoring_event_inputs(store: &DataStore) -> Result<Vec<ScoringEventInput>> {
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
        for mut event in parsed.scoring_events {
            event.date = event.date.or_else(|| date.clone());
            out.push(event);
        }
    }

    out.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then(a.game_id.cmp(&b.game_id))
            .then(a.period.cmp(&b.period))
            .then(a.event_id.cmp(&b.event_id))
    });
    Ok(out)
}

pub fn load_game_scoring_report(
    store: &DataStore,
    context: ViewContext,
    game_id: u64,
) -> GameScoringReportView {
    let key = DataKey::Game(GameId(game_id));
    let date = store
        .manifest()
        .get(DataKind::PlayByPlay, &key)
        .and_then(|entry| {
            entry
                .path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(str::to_owned)
        });
    let Some(raw) = store.load_play_by_play_raw(key) else {
        return GameScoringReportView::from_source_events(context, game_id, false, Vec::new());
    };

    let parsed = parse_play_by_play(&raw, game_id);
    let mut events = parsed.scoring_events;
    for event in &mut events {
        event.date = event.date.take().or_else(|| date.clone());
    }
    GameScoringReportView::from_source_events(context, game_id, true, events)
}

pub fn load_team_scoring_profile(
    store: &DataStore,
    context: ViewContext,
    team: &str,
) -> TeamScoringProfileView {
    let team = team.to_ascii_uppercase();
    let mut source_loaded = false;
    let mut events = Vec::new();
    for entry in store.manifest().list(DataKind::PlayByPlay) {
        let DataKey::Game(game_id) = entry.key else {
            continue;
        };
        if !game_matches_window(
            game_id.0,
            context.window.season.0,
            context.window.season_type,
        ) {
            continue;
        }
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
        if parsed.away_abbrev.eq_ignore_ascii_case(&team)
            || parsed.home_abbrev.eq_ignore_ascii_case(&team)
        {
            source_loaded = true;
        }
        for mut event in parsed.scoring_events.into_iter().filter(|event| {
            event
                .event_owner_team_abbrev
                .as_deref()
                .is_some_and(|event_team| event_team.eq_ignore_ascii_case(&team))
        }) {
            event.date = event.date.take().or_else(|| date.clone());
            events.push(event);
        }
    }
    events.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then(a.game_id.cmp(&b.game_id))
            .then(a.period.cmp(&b.period))
            .then(a.event_id.cmp(&b.event_id))
    });
    TeamScoringProfileView::from_source_events(context, team, source_loaded, events)
}

pub fn load_tonight_scoring_intel(
    store: &DataStore,
    context: ViewContext,
    date: &str,
    favorite_teams: &[String],
    favorite_players: &[(String, Option<u32>)],
) -> TonightScoringIntelView {
    let mut source_loaded = false;
    let mut games_loaded = 0usize;
    let mut events = Vec::new();
    for entry in store.manifest().list(DataKind::PlayByPlay) {
        let entry_date = entry
            .path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        if entry_date != Some(date) {
            continue;
        }
        let DataKey::Game(game_id) = entry.key else {
            continue;
        };
        let Some(raw) = store.load_play_by_play_raw(DataKey::Game(game_id)) else {
            continue;
        };
        source_loaded = true;
        games_loaded += 1;
        let parsed = parse_play_by_play(&raw, game_id.0);
        events.extend(parsed.scoring_events.into_iter().map(|mut event| {
            event.date = event.date.take().or_else(|| Some(date.to_string()));
            event
        }));
    }
    events.sort_by(|a, b| {
        a.game_id
            .cmp(&b.game_id)
            .then(a.period.cmp(&b.period))
            .then(a.event_id.cmp(&b.event_id))
    });

    let team_rows = favorite_teams
        .iter()
        .map(|team| {
            let team_events: Vec<_> = events
                .iter()
                .filter(|event| {
                    event
                        .event_owner_team_abbrev
                        .as_deref()
                        .is_some_and(|event_team| event_team.eq_ignore_ascii_case(team))
                })
                .cloned()
                .collect();
            TonightFavoriteTeamScoringRow {
                team: team.to_ascii_uppercase(),
                events_loaded: team_events.len(),
                summary: ScoringEventSummary::from_events(&team_events),
            }
        })
        .collect();

    let player_rows = favorite_players
        .iter()
        .map(|(key, player_id)| {
            let player_events: Vec<_> = events
                .iter()
                .filter(|event| {
                    player_id.is_some_and(|pid| {
                        event.shooting_player_id == Some(pid)
                            || event.scoring_player_id == Some(pid)
                    })
                })
                .cloned()
                .collect();
            TonightFavoritePlayerScoringRow {
                player_key: key.clone(),
                player_id: *player_id,
                events_loaded: player_events.len(),
                summary: ScoringEventSummary::from_events(&player_events),
            }
        })
        .collect();

    TonightScoringIntelView::from_favorites(
        context,
        date.to_string(),
        games_loaded,
        source_loaded,
        &events,
        team_rows,
        player_rows,
    )
}

fn game_matches_window(game_id: u64, season: u32, season_type: SeasonType) -> bool {
    let start_year = season / 10_000;
    let game_start_year = (game_id / 1_000_000) as u32;
    let game_type = ((game_id / 10_000) % 100) as u8;
    game_start_year == start_year
        && matches!(
            (season_type, game_type),
            (SeasonType::Regular, 2) | (SeasonType::Playoff, 3)
        )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::Utc;
    use icelines_core::freshness::{FetchSource, Freshness, Ttl};
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;
    use icelines_core::ShotEventKind;
    use icelines_core::{Completeness, SourceKind, ViewWindow};
    use serde_json::json;

    use super::*;
    use crate::manifest::ManifestEntry;

    #[test]
    fn l1_scoring_provider_reads_manifest_backed_play_by_play() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("data");
        let play_path = root
            .join("play_by_play")
            .join("2025-10-07")
            .join("2025020001.json");
        fs::create_dir_all(play_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &play_path,
            serde_json::to_vec(&json!({
                "id": 2025020001u64,
                "awayTeam": {"id": 16, "abbrev": "CHI"},
                "homeTeam": {"id": 24, "abbrev": "LAK"},
                "plays": [
                    {
                        "eventId": 21,
                        "periodDescriptor": {"number": 1, "periodType": "REG"},
                        "timeInPeriod": "03:10",
                        "typeDescKey": "shot-on-goal",
                        "details": {
                            "eventOwnerTeamId": 16,
                            "shootingPlayerId": 8483493,
                            "goalieInNetId": 8475683,
                            "xCoord": 66,
                            "yCoord": -1,
                            "zoneCode": "O",
                            "shotType": "snap"
                        }
                    }
                ]
            }))
            .expect("serialize"),
        )
        .expect("write play-by-play");

        let store = DataStore::open(&root).expect("open store");
        store
            .manifest()
            .upsert(
                DataKind::PlayByPlay,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025020001)),
                    path: play_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .expect("upsert manifest");

        let events = load_scoring_event_inputs(&store).expect("load events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].game_id, 2025020001);
        assert_eq!(events[0].date.as_deref(), Some("2025-10-07"));
        assert_eq!(events[0].kind, ShotEventKind::ShotOnGoal);
        assert_eq!(events[0].event_owner_team_abbrev.as_deref(), Some("CHI"));
        assert_eq!(events[0].location.x_coord, Some(66));
    }

    #[test]
    fn l1_game_scoring_report_marks_missing_play_by_play_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("data");
        let store = DataStore::open(&root).expect("open store");
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));

        let report = load_game_scoring_report(&store, context, 2025020001);

        assert!(report.events.is_empty());
        assert_eq!(
            report.context.source_state[0].source,
            SourceKind::PlayByPlay
        );
        assert_eq!(
            report.context.source_state[0].state,
            Completeness::Unavailable
        );
    }

    #[test]
    fn l1_game_scoring_report_marks_loaded_zero_event_play_by_play_complete() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("data");
        let play_path = root
            .join("play_by_play")
            .join("2025-10-07")
            .join("2025020002.json");
        fs::create_dir_all(play_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &play_path,
            serde_json::to_vec(&json!({
                "id": 2025020002u64,
                "gameDate": "2025-10-07",
                "awayTeam": {"id": 16, "abbrev": "CHI"},
                "homeTeam": {"id": 24, "abbrev": "LAK"},
                "plays": [
                    {
                        "eventId": 12,
                        "periodDescriptor": {"number": 1, "periodType": "REG"},
                        "timeInPeriod": "04:00",
                        "typeDescKey": "penalty",
                        "details": {
                            "eventOwnerTeamId": 16,
                            "typeCode": "MIN",
                            "duration": 2
                        }
                    }
                ]
            }))
            .expect("serialize"),
        )
        .expect("write play-by-play");

        let store = DataStore::open(&root).expect("open store");
        store
            .manifest()
            .upsert(
                DataKind::PlayByPlay,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025020002)),
                    path: play_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .expect("upsert manifest");
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));

        let report = load_game_scoring_report(&store, context, 2025020002);

        assert!(report.events.is_empty());
        assert_eq!(report.summary.shot_attempts, 0);
        assert_eq!(
            report.context.source_state[0].source,
            SourceKind::PlayByPlay
        );
        assert_eq!(report.context.source_state[0].state, Completeness::Complete);
    }

    #[test]
    fn l1_team_scoring_profile_filters_team_and_window() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("data");
        let play_path = root
            .join("play_by_play")
            .join("2025-10-07")
            .join("2025020001.json");
        fs::create_dir_all(play_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &play_path,
            serde_json::to_vec(&json!({
                "id": 2025020001u64,
                "gameDate": "2025-10-07",
                "awayTeam": {"id": 16, "abbrev": "CHI"},
                "homeTeam": {"id": 24, "abbrev": "LAK"},
                "plays": [
                    {
                        "eventId": 21,
                        "periodDescriptor": {"number": 1, "periodType": "REG"},
                        "timeInPeriod": "03:10",
                        "typeDescKey": "shot-on-goal",
                        "details": {
                            "eventOwnerTeamId": 16,
                            "shootingPlayerId": 8483493,
                            "xCoord": 66,
                            "yCoord": -1
                        }
                    },
                    {
                        "eventId": 22,
                        "periodDescriptor": {"number": 1, "periodType": "REG"},
                        "timeInPeriod": "03:20",
                        "typeDescKey": "shot-on-goal",
                        "details": {
                            "eventOwnerTeamId": 24,
                            "shootingPlayerId": 8471685,
                            "xCoord": -66,
                            "yCoord": 1
                        }
                    }
                ]
            }))
            .expect("serialize"),
        )
        .expect("write play-by-play");

        let store = DataStore::open(&root).expect("open store");
        store
            .manifest()
            .upsert(
                DataKind::PlayByPlay,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025020001)),
                    path: play_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .expect("upsert manifest");
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));

        let view = load_team_scoring_profile(&store, context, "CHI");

        assert_eq!(view.team, "CHI");
        assert_eq!(view.events.len(), 1);
        assert_eq!(view.summary.shots_on_goal, 1);
        assert_eq!(view.context.source_state[0].state, Completeness::Complete);
    }

    #[test]
    fn l1_tonight_scoring_intel_filters_favorites_by_date() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("data");
        let play_path = root
            .join("play_by_play")
            .join("2025-10-07")
            .join("2025020001.json");
        fs::create_dir_all(play_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &play_path,
            serde_json::to_vec(&json!({
                "id": 2025020001u64,
                "gameDate": "2025-10-07",
                "awayTeam": {"id": 16, "abbrev": "CHI"},
                "homeTeam": {"id": 24, "abbrev": "LAK"},
                "plays": [{
                    "eventId": 21,
                    "periodDescriptor": {"number": 1, "periodType": "REG"},
                    "timeInPeriod": "03:10",
                    "typeDescKey": "goal",
                    "details": {
                        "eventOwnerTeamId": 16,
                        "scoringPlayerId": 8483493,
                        "xCoord": 66,
                        "yCoord": -1
                    }
                }]
            }))
            .expect("serialize"),
        )
        .expect("write play-by-play");
        let store = DataStore::open(&root).expect("open store");
        store
            .manifest()
            .upsert(
                DataKind::PlayByPlay,
                ManifestEntry {
                    key: DataKey::Game(GameId(2025020001)),
                    path: play_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .expect("upsert manifest");
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));

        let view = load_tonight_scoring_intel(
            &store,
            context,
            "2025-10-07",
            &["CHI".to_string()],
            &[("8483493".to_string(), Some(8483493))],
        );

        assert_eq!(view.games_loaded, 1);
        assert_eq!(view.summary.goals, 1);
        assert_eq!(view.favorite_teams[0].summary.goals, 1);
        assert_eq!(view.favorite_players[0].summary.goals, 1);
        assert_eq!(view.context.source_state[0].state, Completeness::Complete);
    }
}
