use anyhow::Result;
use icelines_core::PlayerGoalRecordInput;

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind};
use crate::nhl_api::parse_boxscore;

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
