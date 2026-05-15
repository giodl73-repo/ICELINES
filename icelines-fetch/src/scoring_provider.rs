use anyhow::Result;
use icelines_core::identity::GameId;
use icelines_core::{GameScoringReportView, ScoringEventInput, ViewContext};

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
}
