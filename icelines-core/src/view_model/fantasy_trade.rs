use serde::{Deserialize, Serialize};

use crate::model::Position;

pub const FANTASY_TRADE_EVALUATION_SCHEMA: &str = "fantasy_trade_evaluation.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTradePlayerEvaluation {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub league_value: f64,
    pub league_value_per_game: f64,
    pub remaining_games: u32,
    pub projected_remaining_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTradeTeamEvaluation {
    pub team: String,
    pub before_value: f64,
    pub after_value: f64,
    pub value_delta: f64,
    pub remaining_games_delta: i32,
    pub roster_size_after: usize,
    pub standard_capacity: usize,
    pub missing_active_slots_before: usize,
    pub missing_active_slots_after: usize,
    pub legal: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTradeEvaluationView {
    pub schema: String,
    pub executed: bool,
    pub saved_offer_id: Option<String>,
    pub league: String,
    pub scoring_scheme: String,
    pub sending_team: String,
    pub receiving_team: String,
    pub sends: Vec<FantasyTradePlayerEvaluation>,
    pub receives: Vec<FantasyTradePlayerEvaluation>,
    pub sending_team_result: FantasyTradeTeamEvaluation,
    pub receiving_team_result: FantasyTradeTeamEvaluation,
    pub package_value_gap: f64,
    pub package_value_gap_percent: f64,
    pub recommendation: String,
    pub warnings: Vec<String>,
}

impl FantasyTradeEvaluationView {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != FANTASY_TRADE_EVALUATION_SCHEMA {
            return Err(format!(
                "unsupported fantasy trade schema '{}'",
                self.schema
            ));
        }
        if self.league.trim().is_empty()
            || self.scoring_scheme.trim().is_empty()
            || self.sending_team.trim().is_empty()
            || self.receiving_team.trim().is_empty()
        {
            return Err("fantasy trade evaluation requires league, scoring, and both teams".into());
        }
        if self.sending_team == self.receiving_team {
            return Err("fantasy trade teams must be different".into());
        }
        if self.sends.is_empty() || self.receives.is_empty() {
            return Err("fantasy trade evaluation requires players on both sides".into());
        }
        for value in [self.package_value_gap, self.package_value_gap_percent]
            .into_iter()
            .chain(self.sends.iter().flat_map(|player| {
                [
                    player.league_value,
                    player.league_value_per_game,
                    player.projected_remaining_value,
                ]
            }))
            .chain(self.receives.iter().flat_map(|player| {
                [
                    player.league_value,
                    player.league_value_per_game,
                    player.projected_remaining_value,
                ]
            }))
        {
            if !value.is_finite() {
                return Err("fantasy trade evaluation contains a non-finite value".into());
            }
        }
        Ok(())
    }
}
