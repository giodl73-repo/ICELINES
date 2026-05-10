use serde::{Deserialize, Serialize};

use crate::identity::GameId;
use crate::view_model::context::{SourceKind, SourceState, ViewContext, ViewWarning};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameView {
    pub context: ViewContext,
    pub game_id: GameId,
    pub away_abbrev: String,
    pub home_abbrev: String,
    pub away_score: u8,
    pub home_score: u8,
    pub state_label: String,
    pub is_live: bool,
    pub auto_refresh: bool,
    pub goalies: Vec<GameGoalieRow>,
    pub goals: Vec<GameGoalRow>,
    pub away_top_skaters: Vec<GameSkaterRow>,
    pub home_top_skaters: Vec<GameSkaterRow>,
    pub warnings: Vec<ViewWarning>,
}

impl GameView {
    pub fn from_boxscore(mut context: ViewContext, boxscore: GameBoxscoreInput) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::GameLog));

        let state = boxscore.game_state.as_deref().unwrap_or("");
        let last = boxscore.last_period.as_deref().unwrap_or("");
        let state_label = match (state, last) {
            ("FINAL" | "OFF", "OT") => "Final/OT",
            ("FINAL" | "OFF", "SO") => "Final/SO",
            ("FINAL" | "OFF", _) => "Final",
            ("LIVE" | "CRIT", _) => "LIVE",
            ("PRE", _) => "Pre-game",
            _ => "",
        }
        .to_string();
        let is_live = matches!(state, "LIVE" | "CRIT");
        let auto_refresh = matches!(state, "LIVE" | "CRIT" | "PRE");

        let mut away_skaters = boxscore.away_skaters;
        away_skaters.sort_by_key(|skater| std::cmp::Reverse(skater.goals + skater.assists));
        let mut home_skaters = boxscore.home_skaters;
        home_skaters.sort_by_key(|skater| std::cmp::Reverse(skater.goals + skater.assists));

        Self {
            context,
            game_id: GameId(boxscore.game_id),
            away_abbrev: boxscore.away_abbrev,
            home_abbrev: boxscore.home_abbrev,
            away_score: boxscore.away_score,
            home_score: boxscore.home_score,
            state_label,
            is_live,
            auto_refresh,
            goalies: boxscore
                .goalies
                .into_iter()
                .map(|goalie| GameGoalieRow {
                    player_id: goalie.player_id,
                    player_name: goalie.player_name,
                    saves: goalie.saves,
                    shots: goalie.shots,
                    decision: goalie.decision,
                })
                .collect(),
            goals: boxscore
                .goals
                .into_iter()
                .map(|goal| GameGoalRow {
                    period: goal.period,
                    time_in_period: goal.time_in_period,
                    scorer_team: goal.scorer_team,
                    scorer_name: goal.scorer_name,
                })
                .collect(),
            away_top_skaters: top_skaters(away_skaters),
            home_top_skaters: top_skaters(home_skaters),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameBoxscoreInput {
    pub game_id: u64,
    pub away_abbrev: String,
    pub home_abbrev: String,
    pub away_score: u8,
    pub home_score: u8,
    pub game_state: Option<String>,
    pub last_period: Option<String>,
    pub goals: Vec<GameGoalInput>,
    pub goalies: Vec<GameGoalieInput>,
    pub away_skaters: Vec<GameSkaterInput>,
    pub home_skaters: Vec<GameSkaterInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameGoalieInput {
    pub player_id: u32,
    pub player_name: String,
    pub saves: u32,
    pub shots: u32,
    pub decision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameGoalInput {
    pub period: u8,
    pub time_in_period: String,
    pub scorer_team: String,
    pub scorer_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSkaterInput {
    pub player_id: u32,
    pub player_name: String,
    pub position: String,
    pub goals: u32,
    pub assists: u32,
    pub plus_minus: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameGoalieRow {
    pub player_id: u32,
    pub player_name: String,
    pub saves: u32,
    pub shots: u32,
    pub decision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameGoalRow {
    pub period: u8,
    pub time_in_period: String,
    pub scorer_team: String,
    pub scorer_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSkaterRow {
    pub player_id: u32,
    pub player_name: String,
    pub position: String,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
}

fn top_skaters(skaters: Vec<GameSkaterInput>) -> Vec<GameSkaterRow> {
    skaters
        .into_iter()
        .take(5)
        .map(|skater| {
            let points = skater.goals + skater.assists;
            GameSkaterRow {
                player_id: skater.player_id,
                player_name: skater.player_name,
                position: skater.position,
                goals: skater.goals,
                assists: skater.assists,
                points,
                plus_minus: skater.plus_minus,
            }
        })
        .collect()
}
