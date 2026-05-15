use std::collections::HashMap;

use icelines_core::model::Season;
use icelines_core::{
    ScheduledGameInput, TeamScoringOutlookSourceStatus, TeamScoringOutlookView, ViewContext,
};
use serde_json::Value;

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind};

pub fn load_team_scoring_outlook(
    store: &DataStore,
    context: ViewContext,
    team: &str,
) -> TeamScoringOutlookView {
    let team = team.trim().to_ascii_uppercase();
    let schedule = load_schedule_inputs(store, context.window.season);
    let remaining_games = schedule.remaining_games_for(&team);
    TeamScoringOutlookView::from_schedule_games(
        context,
        team,
        schedule.source_loaded(),
        schedule.source_partial(),
        schedule.games,
        remaining_games,
    )
}

pub fn schedule_remaining_for_team(
    store: &DataStore,
    season: Season,
    team: &str,
) -> (Option<u32>, TeamScoringOutlookSourceStatus) {
    let team = team.trim().to_ascii_uppercase();
    let schedule = load_schedule_inputs(store, season);
    let status = schedule.source_status();
    (schedule.remaining_games_for(&team), status)
}

#[derive(Debug, Clone, Default)]
struct ScheduleInputs {
    games: Vec<ScheduledGameInput>,
    saw_schedule_source: bool,
    saw_complete_source: bool,
}

impl ScheduleInputs {
    fn source_loaded(&self) -> bool {
        self.saw_schedule_source
    }

    fn source_partial(&self) -> bool {
        self.saw_schedule_source && !self.saw_complete_source
    }

    fn source_status(&self) -> TeamScoringOutlookSourceStatus {
        TeamScoringOutlookSourceStatus::from_flags(self.source_loaded(), self.source_partial())
    }

    fn remaining_games_for(&self, team: &str) -> Option<u32> {
        self.saw_complete_source.then(|| {
            self.games
                .iter()
                .filter(|game| game.game_type == 2)
                .filter(|game| !is_final_state(game.game_state.as_deref()))
                .filter(|game| {
                    game.away_abbrev.eq_ignore_ascii_case(team)
                        || game.home_abbrev.eq_ignore_ascii_case(team)
                })
                .count() as u32
        })
    }
}

fn load_schedule_inputs(store: &DataStore, season: Season) -> ScheduleInputs {
    let mut schedule = ScheduleInputs::default();
    let mut by_game_id = HashMap::new();

    for entry in store.manifest().list(DataKind::Schedule) {
        let Ok(bytes) = std::fs::read(&entry.path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        schedule.saw_schedule_source = true;
        let complete_source = matches!(entry.key, DataKey::Season(entry_season) if entry_season == season)
            || value.get("games").and_then(Value::as_array).is_some();
        if complete_source {
            schedule.saw_complete_source = true;
        }
        for game in schedule_games_from_value(&value, season) {
            by_game_id.insert(game.game_id, game);
        }
    }

    schedule.games = by_game_id.into_values().collect();
    schedule.games.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then(a.game_id.cmp(&b.game_id))
            .then(a.away_abbrev.cmp(&b.away_abbrev))
            .then(a.home_abbrev.cmp(&b.home_abbrev))
    });
    schedule
}

fn schedule_games_from_value(value: &Value, season: Season) -> Vec<ScheduledGameInput> {
    let mut games = Vec::new();

    if let Some(raw_games) = value.get("games").and_then(Value::as_array) {
        games.extend(
            raw_games
                .iter()
                .filter_map(|game| parse_schedule_input(game, None, season)),
        );
    }

    if let Some(game_week) = value.get("gameWeek").and_then(Value::as_array) {
        for day in game_week {
            let fallback_date = day.get("date").and_then(Value::as_str);
            if let Some(raw_games) = day.get("games").and_then(Value::as_array) {
                games.extend(
                    raw_games
                        .iter()
                        .filter_map(|game| parse_schedule_input(game, fallback_date, season)),
                );
            }
        }
    }

    if let Some(raw_games) = value.as_array() {
        games.extend(
            raw_games
                .iter()
                .filter_map(|game| parse_schedule_input(game, None, season)),
        );
    }

    games
}

fn parse_schedule_input(
    value: &Value,
    fallback_date: Option<&str>,
    season: Season,
) -> Option<ScheduledGameInput> {
    let game = crate::nhl_api::parse_game(value, fallback_date)?;
    if !game_date_matches_season(&game.date, season) {
        return None;
    }
    Some(ScheduledGameInput {
        game_id: game.game_id,
        date: game.date,
        game_type: game.game_type,
        away_abbrev: game.away_abbrev,
        away_name: game.away_name,
        home_abbrev: game.home_abbrev,
        home_name: game.home_name,
        start_time_utc: game.start_time_utc,
        away_score: game.away_score,
        home_score: game.home_score,
        game_state: game.game_state,
        last_period: game.last_period,
        series_game: game.series_game,
        away_wins: game.away_wins,
        home_wins: game.home_wins,
    })
}

fn game_date_matches_season(date: &str, season: Season) -> bool {
    let Some(year) = date.get(0..4).and_then(|value| value.parse::<u32>().ok()) else {
        return true;
    };
    year == season.0 / 10_000 || year == season.0 % 10_000
}

fn is_final_state(state: Option<&str>) -> bool {
    matches!(
        state.map(str::to_ascii_uppercase).as_deref(),
        Some("FINAL" | "OFF" | "OFFICIAL")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use icelines_core::freshness::{FetchSource, Freshness, Ttl};
    use icelines_core::season_stats::SeasonType;
    use icelines_core::{Completeness, ViewWindow};
    use serde_json::json;

    #[test]
    fn l1_team_outlook_reads_cached_schedule_without_network() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = DataStore::open(temp.path()).expect("store");
        write_schedule(
            &store,
            DataKey::Season(Season(20252026)),
            json!({
                "games": [
                    final_game(1, "2025-10-01", "EDM", "CGY", 4, 2),
                    future_game(2, "2025-10-03", "VAN", "EDM")
                ]
            }),
        );

        let view = load_team_scoring_outlook(
            &store,
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "EDM",
        );

        assert_eq!(view.source_status, TeamScoringOutlookSourceStatus::Loaded);
        assert_eq!(view.games_played, 1);
        assert_eq!(view.remaining_games, Some(1));
        assert_eq!(view.rows[0].current_total, 4);
        assert_eq!(view.rows[1].current_total, 2);
    }

    #[test]
    fn l1_team_outlook_partial_date_cache_nulls_remaining_games() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = DataStore::open(temp.path()).expect("store");
        write_schedule(
            &store,
            DataKey::Date("2025-10-01".to_string()),
            json!({
                "gameWeek": [{
                    "date": "2025-10-01",
                    "games": [final_game(1, "2025-10-01", "EDM", "CGY", 4, 2)]
                }]
            }),
        );

        let view = load_team_scoring_outlook(
            &store,
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "EDM",
        );

        assert_eq!(
            view.source_status,
            TeamScoringOutlookSourceStatus::PartialSource
        );
        assert_eq!(view.context.completeness, Completeness::Partial);
        assert_eq!(view.remaining_games, None);
        assert_eq!(view.rows[0].projected_finish, None);
    }

    fn write_schedule(store: &DataStore, key: DataKey, value: serde_json::Value) {
        let suffix = match &key {
            DataKey::Season(season) => format!("season-{}", season.0),
            DataKey::Date(date) => format!("date-{date}"),
            other => format!("{other:?}")
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
                .collect(),
        };
        let path = store.root().join(format!("schedule-{suffix}.json"));
        std::fs::write(&path, serde_json::to_vec(&value).expect("json")).expect("write schedule");
        store
            .manifest()
            .upsert(
                DataKind::Schedule,
                crate::manifest::ManifestEntry {
                    key,
                    path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .expect("manifest upsert");
    }

    fn final_game(
        id: u64,
        date: &str,
        away: &str,
        home: &str,
        away_score: u8,
        home_score: u8,
    ) -> serde_json::Value {
        json!({
            "id": id,
            "gameDate": date,
            "gameType": 2,
            "gameState": "FINAL",
            "awayTeam": {"abbrev": away, "score": away_score},
            "homeTeam": {"abbrev": home, "score": home_score},
            "startTimeUTC": format!("{date}T23:00:00Z")
        })
    }

    fn future_game(id: u64, date: &str, away: &str, home: &str) -> serde_json::Value {
        json!({
            "id": id,
            "gameDate": date,
            "gameType": 2,
            "gameState": "FUT",
            "awayTeam": {"abbrev": away},
            "homeTeam": {"abbrev": home},
            "startTimeUTC": format!("{date}T23:00:00Z")
        })
    }
}
