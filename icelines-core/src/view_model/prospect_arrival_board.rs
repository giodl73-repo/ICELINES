//! Authority-aware all-team summary of prospect-arrival calibration.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    ProspectArrivalLeagueCalibrationView, CANONICAL_TEAMS,
    PROSPECT_ARRIVAL_LEAGUE_CALIBRATION_SCHEMA,
};

pub const PROSPECT_ARRIVAL_BOARD_SCHEMA: &str = "prospect_arrival_board.v1";
pub const PROSPECT_ARRIVAL_BOARD_METHOD: &str = "prospect_arrival_board.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectArrivalRankState {
    Ranked,
    Withheld,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalBoardTeamView {
    pub organization: String,
    pub target_skaters: usize,
    pub calibrated_skaters: usize,
    pub excluded_skaters: usize,
    pub calibration_coverage: f64,
    pub expected_arrivals: f64,
    pub expected_established_roles: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_arrival_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<usize>,
    pub rank_state: ProspectArrivalRankState,
    #[serde(default)]
    pub rank_blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalBoardView {
    pub schema: String,
    pub method_version: String,
    pub forecast_season: u32,
    pub generated_at: String,
    pub source_artifact_fingerprint: String,
    pub population_authority_complete: bool,
    pub rank_state: ProspectArrivalRankState,
    #[serde(default)]
    pub rank_blockers: Vec<String>,
    pub target_skaters: usize,
    pub calibrated_skaters: usize,
    pub excluded_skaters: usize,
    pub teams: Vec<ProspectArrivalBoardTeamView>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl ProspectArrivalBoardView {
    pub fn team(&self, organization: &str) -> Option<&ProspectArrivalBoardTeamView> {
        self.teams
            .iter()
            .find(|row| row.organization == organization)
    }

    /// Authoritative ranks lead only after the league comparability gate passes.
    /// A withheld board stays in canonical team order rather than creating a
    /// shadow rank from partial prospect coverage.
    pub fn teams_in_display_order(&self) -> Vec<&ProspectArrivalBoardTeamView> {
        let mut rows = self.teams.iter().collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.rank
                .unwrap_or(usize::MAX)
                .cmp(&right.rank.unwrap_or(usize::MAX))
                .then_with(|| left.organization.cmp(&right.organization))
        });
        rows
    }

    pub fn calculate_fingerprint(&self) -> Result<String, ProspectArrivalBoardError> {
        let mut canonical = self.clone();
        canonical.fingerprint.clear();
        canonical.rank_blockers.sort();
        canonical
            .teams
            .sort_by(|left, right| left.organization.cmp(&right.organization));
        for team in &mut canonical.teams {
            team.rank_blockers.sort();
        }
        hash_json(&canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProspectArrivalBoardError {
    #[error("unsupported prospect arrival league schema: {0}")]
    UnsupportedSchema(String),
    #[error("prospect arrival league artifact does not contain the canonical 32 teams")]
    InvalidLeagueEnvelope,
    #[error("prospect arrival league totals do not reconcile")]
    TotalsDoNotReconcile,
    #[error("prospect arrival board requires a generated-at timestamp")]
    MissingGeneratedAt,
    #[error("prospect arrival board JSON failed: {0}")]
    InvalidJson(String),
}

pub fn build_prospect_arrival_board(
    arrival: &ProspectArrivalLeagueCalibrationView,
    generated_at: impl Into<String>,
) -> Result<ProspectArrivalBoardView, ProspectArrivalBoardError> {
    if arrival.schema != PROSPECT_ARRIVAL_LEAGUE_CALIBRATION_SCHEMA {
        return Err(ProspectArrivalBoardError::UnsupportedSchema(
            arrival.schema.clone(),
        ));
    }
    let generated_at = generated_at.into();
    if generated_at.trim().is_empty() {
        return Err(ProspectArrivalBoardError::MissingGeneratedAt);
    }
    let expected = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| *team)
        .collect::<std::collections::BTreeSet<_>>();
    let actual = arrival
        .teams
        .iter()
        .map(|team| team.organization.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if arrival.organizations_requested != CANONICAL_TEAMS.len()
        || arrival.organizations_represented != CANONICAL_TEAMS.len()
        || arrival.teams.len() != CANONICAL_TEAMS.len()
        || actual != expected
    {
        return Err(ProspectArrivalBoardError::InvalidLeagueEnvelope);
    }
    if arrival.target_skaters != arrival.calibrated_skaters + arrival.excluded_skaters
        || arrival.target_skaters
            != arrival
                .teams
                .iter()
                .map(|team| team.target_skaters)
                .sum::<usize>()
        || arrival.calibrated_skaters
            != arrival
                .teams
                .iter()
                .map(|team| team.calibrated_skaters)
                .sum::<usize>()
        || arrival.excluded_skaters
            != arrival
                .teams
                .iter()
                .map(|team| team.excluded_skaters)
                .sum::<usize>()
        || arrival.teams.iter().any(|team| {
            team.target_skaters != team.calibrated_skaters + team.excluded_skaters
                || team.calibrated_skaters != team.calibrations.len()
                || team.excluded_skaters != team.exclusions.len()
        })
    {
        return Err(ProspectArrivalBoardError::TotalsDoNotReconcile);
    }

    let source_artifact_fingerprint = hash_json(arrival)?;
    let population_authority_complete = arrival
        .population_authority
        .as_ref()
        .is_some_and(|authority| authority.population_complete);
    let mut rank_blockers = Vec::new();
    if !population_authority_complete {
        rank_blockers
            .push("complete sealed prospect population authority was not supplied".to_owned());
    }
    if arrival.excluded_skaters > 0 {
        rank_blockers.push(format!(
            "{} target skaters lack a comparable arrival calibration",
            arrival.excluded_skaters
        ));
    }
    let rank_state = if rank_blockers.is_empty() {
        ProspectArrivalRankState::Ranked
    } else {
        ProspectArrivalRankState::Withheld
    };

    let mut teams = arrival
        .teams
        .iter()
        .map(|team| {
            let probabilities = team
                .calibrations
                .iter()
                .map(|calibration| {
                    calibration
                        .horizon_adjusted_arrival_probability
                        .unwrap_or(calibration.calibrated_arrival_probability)
                })
                .collect::<Vec<_>>();
            let expected_arrivals = canonical_zero(probabilities.iter().sum::<f64>());
            let expected_established_roles = canonical_zero(
                team.calibrations
                    .iter()
                    .map(|calibration| {
                        calibration
                            .horizon_adjusted_established_probability
                            .or(calibration.calibrated_established_probability)
                            .unwrap_or(calibration.empirical_established_rate)
                    })
                    .sum::<f64>(),
            );
            let top_arrival_probability = probabilities
                .iter()
                .copied()
                .max_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
            let calibration_coverage = if team.target_skaters == 0 {
                1.0
            } else {
                team.calibrated_skaters as f64 / team.target_skaters as f64
            };
            let mut team_blockers = Vec::new();
            if !population_authority_complete {
                team_blockers.push("population authority incomplete".to_owned());
            }
            if team.excluded_skaters > 0 {
                team_blockers.push(format!(
                    "{} of {} target skaters excluded",
                    team.excluded_skaters, team.target_skaters
                ));
            }
            ProspectArrivalBoardTeamView {
                organization: team.organization.clone(),
                target_skaters: team.target_skaters,
                calibrated_skaters: team.calibrated_skaters,
                excluded_skaters: team.excluded_skaters,
                calibration_coverage,
                expected_arrivals,
                expected_established_roles,
                top_arrival_probability,
                rank: None,
                rank_state,
                rank_blockers: team_blockers,
            }
        })
        .collect::<Vec<_>>();

    if rank_state == ProspectArrivalRankState::Ranked {
        let mut order = (0..teams.len()).collect::<Vec<_>>();
        order.sort_by(|left, right| {
            teams[*right]
                .expected_arrivals
                .partial_cmp(&teams[*left].expected_arrivals)
                .unwrap_or(Ordering::Equal)
                .then_with(|| teams[*left].organization.cmp(&teams[*right].organization))
        });
        for (offset, index) in order.into_iter().enumerate() {
            teams[index].rank = Some(offset + 1);
            teams[index].rank_blockers.clear();
        }
    }
    teams.sort_by(|left, right| left.organization.cmp(&right.organization));

    let mut board = ProspectArrivalBoardView {
        schema: PROSPECT_ARRIVAL_BOARD_SCHEMA.to_owned(),
        method_version: PROSPECT_ARRIVAL_BOARD_METHOD.to_owned(),
        forecast_season: arrival.forecast_season,
        generated_at,
        source_artifact_fingerprint,
        population_authority_complete,
        rank_state,
        rank_blockers,
        target_skaters: arrival.target_skaters,
        calibrated_skaters: arrival.calibrated_skaters,
        excluded_skaters: arrival.excluded_skaters,
        teams,
        disclosures: vec![
            "Expected arrivals and established roles are sums of player-level horizon probabilities, not guaranteed player counts.".to_owned(),
            "League ranks publish only when population authority is complete and every target skater has a comparable calibration.".to_owned(),
            "A rank-withheld board stays in canonical team order; partial calibrated values never become a shadow ranking.".to_owned(),
            "Goalies remain outside this skater-arrival cohort until a separately calibrated goalie authority exists.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    board.fingerprint = board.calculate_fingerprint()?;
    Ok(board)
}

fn hash_json(value: &impl Serialize) -> Result<String, ProspectArrivalBoardError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ProspectArrivalBoardError::InvalidJson(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ProspectArrivalLeagueCalibrationView {
        serde_json::from_str(include_str!(
            "../../../examples/icecast-prospect-arrival-league-2026-27.json"
        ))
        .unwrap()
    }

    #[test]
    fn incomplete_population_retains_values_but_withholds_shadow_ranks() {
        let board = build_prospect_arrival_board(&fixture(), "2026-09-15T12:00:00Z").unwrap();
        assert_eq!(board.teams.len(), CANONICAL_TEAMS.len());
        assert_eq!(board.rank_state, ProspectArrivalRankState::Withheld);
        assert!(board.teams.iter().all(|team| team.rank.is_none()));
        assert_eq!(
            board
                .teams_in_display_order()
                .iter()
                .map(|team| team.organization.as_str())
                .collect::<Vec<_>>(),
            CANONICAL_TEAMS
                .iter()
                .map(|(team, _)| *team)
                .collect::<Vec<_>>()
        );
        let nyr = board.team("NYR").unwrap();
        assert_eq!(nyr.calibrated_skaters, 2);
        assert!(nyr.expected_arrivals > 0.0);
        assert_eq!(board.calculate_fingerprint().unwrap(), board.fingerprint);
    }

    #[test]
    fn complete_comparable_population_receives_deterministic_ranks() {
        let mut arrival = fixture();
        for team in &mut arrival.teams {
            team.target_skaters = team.calibrated_skaters;
            team.excluded_skaters = 0;
            team.exclusions.clear();
        }
        arrival.target_skaters = arrival.calibrated_skaters;
        arrival.excluded_skaters = 0;
        arrival.population_authority = Some(crate::ProspectArrivalLeaguePopulationAuthorityView {
            source_package_fingerprint: "a".repeat(64),
            population_complete: true,
            supplied_studies: arrival.calibrated_skaters,
            controlled_studies: arrival.calibrated_skaters,
            control_exclusions: 0,
        });
        let board = build_prospect_arrival_board(&arrival, "2026-09-15T12:00:00Z").unwrap();
        assert_eq!(board.rank_state, ProspectArrivalRankState::Ranked);
        assert_eq!(
            board
                .teams
                .iter()
                .filter(|team| team.rank.is_some())
                .count(),
            CANONICAL_TEAMS.len()
        );
        assert_eq!(board.teams_in_display_order()[0].rank, Some(1));
    }
}
