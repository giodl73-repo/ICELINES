//! Historical prospect-to-NHL conversion measurement.
//!
//! This contract compares a frozen prospect baseline with later observed NHL
//! participation, role, and optional performance. It measures realized value
//! from supplied cohorts; it does not infer causal development quality.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::prospect_study::ProspectStudyEvidenceInput;

pub const PROSPECT_CONVERSION_INPUT_SCHEMA: &str = "prospect_conversion_input.v1";
pub const PROSPECT_CONVERSION_BOARD_SCHEMA: &str = "prospect_conversion_board.v1";
pub const PROSPECT_CONVERSION_METHOD: &str = "prospect_conversion_observed.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectConversionDisposition {
    Retained,
    Traded,
    Departed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProspectConversionRankBlocker {
    InsufficientCohort { observed: usize, required: usize },
    LowBaselineConfidence { observed: f64, required: f64 },
    LowOutcomeCoverage { observed: f64, required: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionBaselineInput {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub baseline_season: u32,
    /// Attention-free 0..100 prospect signal frozen at the baseline season.
    pub observed_signal_score: f64,
    /// 0..1 workload confidence attached to the frozen prospect study.
    pub workload_confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectNhlOutcomeInput {
    pub player_id: u32,
    pub through_season: u32,
    pub nhl_games_played: u32,
    pub nhl_toi_seconds: u64,
    /// Optional canonical 0..100 NHL performance measure. Missing performance
    /// reduces outcome coverage and can leave the organization unranked.
    pub performance_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance_basis: Option<String>,
    pub disposition: ProspectConversionDisposition,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionConfig {
    pub minimum_horizon_seasons: u8,
    pub skater_established_games: u32,
    pub goalie_established_games: u32,
    pub forward_role_seconds_per_game: u32,
    pub defense_role_seconds_per_game: u32,
    pub goalie_role_seconds_per_game: u32,
    pub arrival_weight: f64,
    pub role_weight: f64,
    pub performance_weight: f64,
    pub baseline_floor: f64,
    pub maximum_efficiency_index: f64,
    pub minimum_rankable_players: usize,
    pub minimum_rankable_baseline_confidence: f64,
    pub minimum_rankable_outcome_coverage: f64,
}

impl Default for ProspectConversionConfig {
    fn default() -> Self {
        Self {
            minimum_horizon_seasons: 3,
            skater_established_games: 82,
            goalie_established_games: 40,
            forward_role_seconds_per_game: 900,
            defense_role_seconds_per_game: 1_080,
            goalie_role_seconds_per_game: 3_000,
            arrival_weight: 0.40,
            role_weight: 0.30,
            performance_weight: 0.30,
            baseline_floor: 20.0,
            maximum_efficiency_index: 200.0,
            minimum_rankable_players: 5,
            minimum_rankable_baseline_confidence: 0.50,
            minimum_rankable_outcome_coverage: 0.80,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionInput {
    pub schema: String,
    pub baselines: Vec<ProspectConversionBaselineInput>,
    pub outcomes: Vec<ProspectNhlOutcomeInput>,
    #[serde(default)]
    pub config: ProspectConversionConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionMethodologyView {
    pub method: String,
    pub config: ProspectConversionConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionPlayerView {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub baseline_season: u32,
    pub through_season: u32,
    pub horizon_seasons: u32,
    pub baseline_signal_score: f64,
    pub workload_confidence: f64,
    pub nhl_games_played: u32,
    pub nhl_toi_seconds: u64,
    pub arrival_score: f64,
    pub role_score: f64,
    pub performance_score: Option<f64>,
    pub realized_value_score: f64,
    pub outcome_coverage: f64,
    pub conversion_delta: f64,
    pub efficiency_index: f64,
    pub established: bool,
    pub disposition: ProspectConversionDisposition,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionOrganizationView {
    pub organization: String,
    pub players: usize,
    pub converted_players: usize,
    pub established_players: usize,
    pub retained_players: usize,
    pub traded_players: usize,
    pub baseline_signal_score: f64,
    pub baseline_confidence: f64,
    pub realized_value_score: f64,
    pub conversion_delta: f64,
    pub efficiency_index: f64,
    pub outcome_coverage: f64,
    pub conversion_rank: Option<usize>,
    pub rank_blockers: Vec<ProspectConversionRankBlocker>,
    pub player_results: Vec<ProspectConversionPlayerView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionBoardView {
    pub schema: String,
    pub source_schema: String,
    pub methodology: ProspectConversionMethodologyView,
    pub baseline_seasons: Vec<u32>,
    pub through_season: u32,
    pub organizations: usize,
    pub players: usize,
    pub ranked_organizations: usize,
    pub programs: Vec<ProspectConversionOrganizationView>,
    pub disclosures: Vec<String>,
}

pub fn build_prospect_conversion_board(
    input: &ProspectConversionInput,
) -> Result<ProspectConversionBoardView, String> {
    validate_config(input.config)?;
    if input.schema != PROSPECT_CONVERSION_INPUT_SCHEMA || input.baselines.is_empty() {
        return Err("invalid prospect conversion input".to_owned());
    }

    let mut baselines = BTreeMap::new();
    for baseline in &input.baselines {
        if baseline.player_id == 0
            || baseline.player.trim().is_empty()
            || baseline.organization.trim().is_empty()
            || baseline.position.trim().is_empty()
            || !matches!(
                baseline.position.trim().to_ascii_uppercase().as_str(),
                "F" | "C" | "LW" | "RW" | "W" | "D" | "LD" | "RD" | "G"
            )
            || season_start_year(baseline.baseline_season).is_none()
            || !baseline.observed_signal_score.is_finite()
            || !(0.0..=100.0).contains(&baseline.observed_signal_score)
            || !baseline.workload_confidence.is_finite()
            || !(0.0..=1.0).contains(&baseline.workload_confidence)
            || baselines.insert(baseline.player_id, baseline).is_some()
        {
            return Err("invalid or duplicate prospect conversion baseline".to_owned());
        }
    }

    let mut outcomes = BTreeMap::new();
    for outcome in &input.outcomes {
        if outcome.player_id == 0
            || season_start_year(outcome.through_season).is_none()
            || outcome
                .performance_score
                .is_some_and(|score| !score.is_finite() || !(0.0..=100.0).contains(&score))
            || !matches!(
                (&outcome.performance_score, &outcome.performance_basis),
                (Some(_), Some(basis)) if !basis.trim().is_empty()
            ) && !matches!(
                (&outcome.performance_score, &outcome.performance_basis),
                (None, None)
            )
            || outcome.evidence.is_empty()
            || outcome.evidence.iter().any(|item| {
                item.label.trim().is_empty()
                    || !(item.source_url.starts_with("https://")
                        || item.source_url.starts_with("http://"))
            })
            || outcomes.insert(outcome.player_id, outcome).is_some()
        {
            return Err("invalid or duplicate prospect NHL outcome".to_owned());
        }
    }
    if baselines.len() != outcomes.len()
        || baselines
            .keys()
            .any(|player_id| !outcomes.contains_key(player_id))
    {
        return Err(
            "prospect conversion baselines and outcomes must cover identical players".to_owned(),
        );
    }

    let through_seasons = outcomes
        .values()
        .map(|outcome| outcome.through_season)
        .collect::<BTreeSet<_>>();
    if through_seasons.len() != 1 {
        return Err("prospect conversion outcomes require one common through season".to_owned());
    }
    let through_season = *through_seasons.iter().next().expect("validated outcomes");
    let through_start = season_start_year(through_season).expect("validated season");
    let mut baseline_seasons = BTreeSet::new();
    let mut by_organization = BTreeMap::<String, Vec<ProspectConversionPlayerView>>::new();

    for (player_id, baseline) in baselines {
        let outcome = outcomes[&player_id];
        let baseline_start = season_start_year(baseline.baseline_season).expect("validated season");
        let horizon_seasons = through_start.saturating_sub(baseline_start);
        if horizon_seasons < u32::from(input.config.minimum_horizon_seasons) {
            return Err(format!(
                "prospect {} has only {} completed horizon season(s)",
                player_id, horizon_seasons
            ));
        }
        baseline_seasons.insert(baseline.baseline_season);
        let goalie = position_group(&baseline.position) == "G";
        let established_games = if goalie {
            input.config.goalie_established_games
        } else {
            input.config.skater_established_games
        };
        let role_benchmark = match position_group(&baseline.position) {
            "G" => input.config.goalie_role_seconds_per_game,
            "D" => input.config.defense_role_seconds_per_game,
            _ => input.config.forward_role_seconds_per_game,
        };
        let arrival_score =
            100.0 * (f64::from(outcome.nhl_games_played) / f64::from(established_games)).min(1.0);
        let seconds_per_game = if outcome.nhl_games_played == 0 {
            0.0
        } else {
            outcome.nhl_toi_seconds as f64 / f64::from(outcome.nhl_games_played)
        };
        let role_score = 100.0 * (seconds_per_game / f64::from(role_benchmark)).min(1.0);
        let available_weight = input.config.arrival_weight
            + input.config.role_weight
            + if outcome.performance_score.is_some() {
                input.config.performance_weight
            } else {
                0.0
            };
        let realized_value_score = (input.config.arrival_weight * arrival_score
            + input.config.role_weight * role_score
            + input.config.performance_weight * outcome.performance_score.unwrap_or(0.0))
            / available_weight;
        let outcome_coverage = available_weight;
        let efficiency_index = (100.0 * realized_value_score
            / baseline
                .observed_signal_score
                .max(input.config.baseline_floor))
        .min(input.config.maximum_efficiency_index);
        by_organization
            .entry(baseline.organization.clone())
            .or_default()
            .push(ProspectConversionPlayerView {
                player_id,
                player: baseline.player.clone(),
                organization: baseline.organization.clone(),
                position: baseline.position.clone(),
                baseline_season: baseline.baseline_season,
                through_season,
                horizon_seasons,
                baseline_signal_score: round_score(baseline.observed_signal_score),
                workload_confidence: baseline.workload_confidence,
                nhl_games_played: outcome.nhl_games_played,
                nhl_toi_seconds: outcome.nhl_toi_seconds,
                arrival_score: round_score(arrival_score),
                role_score: round_score(role_score),
                performance_score: outcome.performance_score.map(round_score),
                realized_value_score: round_score(realized_value_score),
                outcome_coverage: round_ratio(outcome_coverage),
                conversion_delta: round_score(
                    realized_value_score - baseline.observed_signal_score,
                ),
                efficiency_index: round_score(efficiency_index),
                established: outcome.nhl_games_played >= established_games,
                disposition: outcome.disposition,
                evidence: outcome.evidence.clone(),
            });
    }

    let mut programs = Vec::with_capacity(by_organization.len());
    for (organization, mut players) in by_organization {
        players.sort_by(|left, right| {
            right
                .realized_value_score
                .total_cmp(&left.realized_value_score)
                .then_with(|| left.player.cmp(&right.player))
                .then_with(|| left.player_id.cmp(&right.player_id))
        });
        let confidence_weight = players
            .iter()
            .map(|player| player.workload_confidence)
            .sum::<f64>();
        let baseline_signal_score = weighted_mean(&players, confidence_weight, |player| {
            player.baseline_signal_score
        });
        let realized_value_score = weighted_mean(&players, confidence_weight, |player| {
            player.realized_value_score
        });
        let outcome_coverage = weighted_mean(&players, confidence_weight, |player| {
            player.outcome_coverage
        });
        let baseline_confidence = players
            .iter()
            .map(|player| player.workload_confidence)
            .sum::<f64>()
            / players.len() as f64;
        let efficiency_index = (100.0 * realized_value_score
            / baseline_signal_score.max(input.config.baseline_floor))
        .min(input.config.maximum_efficiency_index);
        let mut rank_blockers = Vec::new();
        if players.len() < input.config.minimum_rankable_players {
            rank_blockers.push(ProspectConversionRankBlocker::InsufficientCohort {
                observed: players.len(),
                required: input.config.minimum_rankable_players,
            });
        }
        if baseline_confidence < input.config.minimum_rankable_baseline_confidence {
            rank_blockers.push(ProspectConversionRankBlocker::LowBaselineConfidence {
                observed: round_ratio(baseline_confidence),
                required: input.config.minimum_rankable_baseline_confidence,
            });
        }
        if outcome_coverage < input.config.minimum_rankable_outcome_coverage {
            rank_blockers.push(ProspectConversionRankBlocker::LowOutcomeCoverage {
                observed: round_ratio(outcome_coverage),
                required: input.config.minimum_rankable_outcome_coverage,
            });
        }
        programs.push(ProspectConversionOrganizationView {
            organization,
            players: players.len(),
            converted_players: players
                .iter()
                .filter(|player| player.nhl_games_played > 0)
                .count(),
            established_players: players.iter().filter(|player| player.established).count(),
            retained_players: players
                .iter()
                .filter(|player| player.disposition == ProspectConversionDisposition::Retained)
                .count(),
            traded_players: players
                .iter()
                .filter(|player| player.disposition == ProspectConversionDisposition::Traded)
                .count(),
            baseline_signal_score: round_score(baseline_signal_score),
            baseline_confidence: round_ratio(baseline_confidence),
            realized_value_score: round_score(realized_value_score),
            conversion_delta: round_score(realized_value_score - baseline_signal_score),
            efficiency_index: round_score(efficiency_index),
            outcome_coverage: round_ratio(outcome_coverage),
            conversion_rank: None,
            rank_blockers,
            player_results: players,
        });
    }
    let mut rankable = programs
        .iter()
        .enumerate()
        .filter(|(_, program)| program.rank_blockers.is_empty())
        .map(|(index, program)| (index, program.efficiency_index))
        .collect::<Vec<_>>();
    rankable.sort_by(|left, right| {
        right.1.total_cmp(&left.1).then_with(|| {
            programs[left.0]
                .organization
                .cmp(&programs[right.0].organization)
        })
    });
    for (offset, (index, _)) in rankable.iter().enumerate() {
        programs[*index].conversion_rank = Some(offset + 1);
    }
    programs.sort_by(|left, right| {
        left.conversion_rank
            .unwrap_or(usize::MAX)
            .cmp(&right.conversion_rank.unwrap_or(usize::MAX))
            .then_with(|| left.organization.cmp(&right.organization))
    });

    Ok(ProspectConversionBoardView {
        schema: PROSPECT_CONVERSION_BOARD_SCHEMA.to_owned(),
        source_schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
        methodology: ProspectConversionMethodologyView {
            method: PROSPECT_CONVERSION_METHOD.to_owned(),
            config: input.config,
        },
        baseline_seasons: baseline_seasons.into_iter().collect(),
        through_season,
        organizations: programs.len(),
        players: input.baselines.len(),
        ranked_organizations: rankable.len(),
        programs,
        disclosures: vec![
            "Conversion compares a frozen attention-free prospect signal with later observed NHL arrival, role, and optional canonical performance evidence.".to_owned(),
            "Performance is never imputed. Missing performance lowers outcome coverage, and organizations below the configured coverage floor remain unranked.".to_owned(),
            "Small or weakly observed cohorts remain unranked under the configured player-count and baseline-confidence floors; rank blockers remain visible on every organization row.".to_owned(),
            "The efficiency index compares realized value with baseline signal using a disclosed denominator floor and cap; it rewards over-delivery without allowing tiny baselines to explode.".to_owned(),
            "Retention and trade disposition are reported but do not add value without a separately sourced return model.".to_owned(),
            "This is cohort conversion, not proof that an organization caused or prevented an individual outcome.".to_owned(),
        ],
    })
}

fn validate_config(config: ProspectConversionConfig) -> Result<(), String> {
    let weight_sum = config.arrival_weight + config.role_weight + config.performance_weight;
    if config.minimum_horizon_seasons == 0
        || config.skater_established_games == 0
        || config.goalie_established_games == 0
        || config.forward_role_seconds_per_game == 0
        || config.defense_role_seconds_per_game == 0
        || config.goalie_role_seconds_per_game == 0
        || [
            config.arrival_weight,
            config.role_weight,
            config.performance_weight,
        ]
        .iter()
        .any(|weight| !weight.is_finite() || *weight < 0.0)
        || (weight_sum - 1.0).abs() > 1e-9
        || config.arrival_weight + config.role_weight <= 0.0
        || !config.baseline_floor.is_finite()
        || !(0.0..=100.0).contains(&config.baseline_floor)
        || !config.maximum_efficiency_index.is_finite()
        || config.maximum_efficiency_index < 100.0
        || config.minimum_rankable_players == 0
        || !config.minimum_rankable_baseline_confidence.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_rankable_baseline_confidence)
        || !config.minimum_rankable_outcome_coverage.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_rankable_outcome_coverage)
    {
        return Err("invalid prospect conversion configuration".to_owned());
    }
    Ok(())
}

fn weighted_mean(
    players: &[ProspectConversionPlayerView],
    total_weight: f64,
    value: impl Fn(&ProspectConversionPlayerView) -> f64,
) -> f64 {
    if total_weight <= 0.0 {
        players.iter().map(|player| value(player)).sum::<f64>() / players.len() as f64
    } else {
        players
            .iter()
            .map(|player| value(player) * player.workload_confidence)
            .sum::<f64>()
            / total_weight
    }
}

fn position_group(position: &str) -> &'static str {
    match position.trim().to_ascii_uppercase().as_str() {
        "G" => "G",
        "D" | "LD" | "RD" => "D",
        _ => "F",
    }
}

fn season_start_year(season: u32) -> Option<u32> {
    let start = season / 10_000;
    let end = season % 10_000;
    (start >= 1900 && end == start + 1).then_some(start)
}

fn round_score(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_ratio(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(
        player_id: u32,
        organization: &str,
        position: &str,
        signal: f64,
    ) -> ProspectConversionBaselineInput {
        ProspectConversionBaselineInput {
            player_id,
            player: format!("Prospect {player_id}"),
            organization: organization.to_owned(),
            position: position.to_owned(),
            baseline_season: 20222023,
            observed_signal_score: signal,
            workload_confidence: 1.0,
        }
    }

    fn outcome(
        player_id: u32,
        games: u32,
        seconds_per_game: u64,
        performance_score: Option<f64>,
    ) -> ProspectNhlOutcomeInput {
        ProspectNhlOutcomeInput {
            player_id,
            through_season: 20252026,
            nhl_games_played: games,
            nhl_toi_seconds: u64::from(games) * seconds_per_game,
            performance_score,
            performance_basis: performance_score.map(|_| "Canonical NHL value score".to_owned()),
            disposition: ProspectConversionDisposition::Retained,
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Official NHL outcome".to_owned(),
                source_url: format!("https://api-web.nhle.com/v1/player/{player_id}/landing"),
            }],
        }
    }

    #[test]
    fn ranks_complete_conversion_cohorts_by_realized_efficiency() {
        let config = ProspectConversionConfig {
            minimum_rankable_players: 1,
            ..ProspectConversionConfig::default()
        };
        let input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baselines: vec![
                baseline(1, "SEA", "RW", 60.0),
                baseline(2, "NYR", "D", 60.0),
            ],
            outcomes: vec![
                outcome(1, 82, 900, Some(80.0)),
                outcome(2, 20, 600, Some(40.0)),
            ],
            config,
        };
        let view = build_prospect_conversion_board(&input).unwrap();
        assert_eq!(view.schema, PROSPECT_CONVERSION_BOARD_SCHEMA);
        assert_eq!(view.ranked_organizations, 2);
        assert_eq!(view.programs[0].organization, "SEA");
        assert_eq!(view.programs[0].conversion_rank, Some(1));
        assert!(view.programs[0].rank_blockers.is_empty());
        assert!(view.programs[0].efficiency_index > view.programs[1].efficiency_index);
        assert_eq!(view.programs[0].established_players, 1);
    }

    #[test]
    fn missing_performance_is_visible_and_leaves_program_unranked() {
        let config = ProspectConversionConfig {
            minimum_rankable_players: 1,
            ..ProspectConversionConfig::default()
        };
        let input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baselines: vec![baseline(1, "SEA", "RW", 60.0)],
            outcomes: vec![outcome(1, 82, 900, None)],
            config,
        };
        let view = build_prospect_conversion_board(&input).unwrap();
        assert_eq!(view.ranked_organizations, 0);
        assert_eq!(view.programs[0].conversion_rank, None);
        assert_eq!(view.programs[0].outcome_coverage, 0.7);
        assert!(matches!(
            view.programs[0].rank_blockers[0],
            ProspectConversionRankBlocker::LowOutcomeCoverage {
                observed: 0.7,
                required: 0.8
            }
        ));
        assert_eq!(view.programs[0].player_results[0].performance_score, None);
    }

    #[test]
    fn default_rank_floors_explain_small_and_low_confidence_cohorts() {
        let mut weak_baseline = baseline(1, "SEA", "RW", 60.0);
        weak_baseline.workload_confidence = 0.4;
        let input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baselines: vec![weak_baseline],
            outcomes: vec![outcome(1, 82, 900, Some(80.0))],
            config: ProspectConversionConfig::default(),
        };
        let view = build_prospect_conversion_board(&input).unwrap();
        assert_eq!(view.ranked_organizations, 0);
        assert_eq!(view.programs[0].rank_blockers.len(), 2);
        assert!(matches!(
            view.programs[0].rank_blockers[0],
            ProspectConversionRankBlocker::InsufficientCohort {
                observed: 1,
                required: 5
            }
        ));
        assert!(matches!(
            view.programs[0].rank_blockers[1],
            ProspectConversionRankBlocker::LowBaselineConfidence {
                observed: 0.4,
                required: 0.5
            }
        ));
    }

    #[test]
    fn rejects_short_horizons_and_mismatched_player_sets() {
        let mut input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baselines: vec![baseline(1, "SEA", "RW", 60.0)],
            outcomes: vec![outcome(1, 82, 900, Some(80.0))],
            config: ProspectConversionConfig::default(),
        };
        input.outcomes[0].through_season = 20242025;
        assert!(build_prospect_conversion_board(&input)
            .unwrap_err()
            .contains("horizon"));
        input.outcomes[0].through_season = 20252026;
        input.outcomes[0].player_id = 2;
        assert!(build_prospect_conversion_board(&input)
            .unwrap_err()
            .contains("identical players"));
    }
}
