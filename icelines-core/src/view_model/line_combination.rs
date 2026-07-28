//! Renderer-neutral lineup comparison for The Blender.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::team_lineup::{
    rebuild_special_teams, PlayerScorePositionGroup, TeamLineupPlayerView, TeamLineupProjectionView,
};
use super::team_season_forecast::{TeamSeasonAdaptiveLineupChoice, TeamSeasonAdaptiveLineupPolicy};
use super::EvidenceLabel;

pub const LINE_COMBINATION_FORECAST_SCHEMA: &str = "line_combination_forecast.v1";
pub const LINE_COMBINATION_FORECAST_METHOD: &str = "lineup_weighted_fit.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineCombinationPairEvidenceKind {
    /// Exact shared deployment intervals without an on-ice performance join.
    ObservedDeployment,
    /// Shift-aligned performance evidence, such as a goal/shot/xG impact join.
    ObservedShift,
    CoarseSameGame,
    SimulatedAssumption,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineCombinationPairEvidenceInput {
    pub player_one_id: u32,
    pub player_two_id: u32,
    /// Signed fit value from -1 through 1. This is not reinterpreted as causality.
    pub fit: f64,
    pub sample: u32,
    pub kind: LineCombinationPairEvidenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineCombinationUnitKind {
    ForwardLine,
    DefensePair,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineCombinationUnitView {
    pub kind: LineCombinationUnitKind,
    pub unit: u8,
    pub player_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineCombinationScoreView {
    pub talent_placement: f64,
    /// Lineup-weighted GP confidence from 0 through 1.
    pub talent_confidence: f64,
    pub role_fit: f64,
    pub pair_evidence: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineCombinationCandidateView {
    pub id: String,
    pub label: String,
    pub rank: usize,
    pub is_baseline: bool,
    pub evidence_label: EvidenceLabel,
    pub strength_delta: f64,
    pub score: LineCombinationScoreView,
    pub units: Vec<LineCombinationUnitView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCombinationForecastConfig {
    pub max_candidates: usize,
    /// Permit natural wings to switch sides and natural centers to fill wing
    /// vacancies. The role-fit penalty remains visible in the score.
    pub allow_off_wing: bool,
}

impl Default for LineCombinationForecastConfig {
    fn default() -> Self {
        Self {
            max_candidates: 24,
            allow_off_wing: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineCombinationForecastView {
    pub schema: String,
    pub method: String,
    pub team: String,
    pub roster_season: u32,
    pub baseline_id: String,
    pub candidates: Vec<LineCombinationCandidateView>,
    pub player_leaderboards: LineCombinationPlayerLeaderboardsView,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineCombinationPlayerInfluenceView {
    pub player_id: u32,
    pub display_name: String,
    pub position_group: PlayerScorePositionGroup,
    pub overall_score: Option<f64>,
    pub reliability_adjusted_score: Option<f64>,
    pub sample_games: u32,
    pub overall_evidence_label: EvidenceLabel,
    /// Mean exact shared-ice affinity on a 0 through 100 scale. Deployment is
    /// descriptive and is deliberately separate from teammate performance.
    pub deployment_affinity_score: Option<f64>,
    pub deployment_observations: usize,
    /// Authority-weighted mean pair fit on a signed -100 through 100 scale.
    pub teammate_effect_score: Option<f64>,
    pub pair_evidence_label: EvidenceLabel,
    pub pair_observations: usize,
    pub partners_helped: usize,
    pub partners_hurt: usize,
    pub evidence_kinds: Vec<LineCombinationPairEvidenceKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineCombinationPlayerLeaderboardsView {
    pub best_overall: Vec<LineCombinationPlayerInfluenceView>,
    pub deployment_anchors: Vec<LineCombinationPlayerInfluenceView>,
    pub positive_multipliers: Vec<LineCombinationPlayerInfluenceView>,
    pub negative_multipliers: Vec<LineCombinationPlayerInfluenceView>,
}

/// Convert a ranked Blender document into an ordered Bench policy. The
/// submitted baseline always opens the season; alternatives retain forecast rank.
pub fn build_adaptive_lineup_policy(
    forecast: &LineCombinationForecastView,
    review_games: u8,
    minimum_points_percentage: f64,
    max_changes: u8,
    max_choices: usize,
) -> Result<TeamSeasonAdaptiveLineupPolicy, String> {
    if !(2..=20).contains(&review_games) {
        return Err("The Bench review_games must be between 2 and 20".to_owned());
    }
    if !minimum_points_percentage.is_finite() || !(0.0..=1.0).contains(&minimum_points_percentage) {
        return Err("The Bench minimum_points_percentage must be between 0 and 1".to_owned());
    }
    if max_choices == 0 || max_choices > 12 {
        return Err("The Bench max_choices must be between 1 and 12".to_owned());
    }
    let baseline = forecast
        .candidates
        .iter()
        .find(|candidate| candidate.id == forecast.baseline_id)
        .ok_or_else(|| "The Bench requires the Blender baseline candidate".to_owned())?;
    let mut selected = vec![baseline];
    selected.extend(
        forecast
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.id != forecast.baseline_id && candidate.strength_delta.abs() > 1e-9
            })
            .take(max_choices.saturating_sub(1)),
    );
    if usize::from(max_changes) > selected.len().saturating_sub(1) {
        return Err("The Bench max_changes exceeds selected lineup transitions".to_owned());
    }
    Ok(TeamSeasonAdaptiveLineupPolicy {
        team: forecast.team.clone(),
        review_games,
        minimum_points_percentage,
        max_changes,
        choices: selected
            .into_iter()
            .map(|candidate| TeamSeasonAdaptiveLineupChoice {
                id: candidate.id.clone(),
                label: candidate.label.clone(),
                strength_delta: candidate.strength_delta,
            })
            .collect(),
    })
}

pub fn build_line_combination_forecast(
    baseline: &TeamLineupProjectionView,
    pair_evidence: &[LineCombinationPairEvidenceInput],
    config: LineCombinationForecastConfig,
) -> Result<LineCombinationForecastView, String> {
    if config.max_candidates == 0 || config.max_candidates > 100 {
        return Err("The Blender max_candidates must be between 1 and 100".to_owned());
    }
    validate_pair_evidence(baseline, pair_evidence)?;

    let effective_baseline = if config.allow_off_wing {
        complete_flexible_forward_shape(baseline)
    } else {
        baseline.clone()
    };
    let baseline = &effective_baseline;
    let mut configurations = vec![(
        "baseline".to_owned(),
        "Submitted lineup".to_owned(),
        baseline.clone(),
    )];
    for first in 0..12 {
        for second in first + 1..12 {
            let Some(candidate) =
                swap_forward_slots(baseline, first, second, config.allow_off_wing)
            else {
                continue;
            };
            let ids = forward_player_ids(baseline, first, second);
            let names = forward_player_names(baseline, first, second);
            configurations.push((
                format!("swap-{}-{}", ids.0.min(ids.1), ids.0.max(ids.1)),
                format!("Swap {} and {}", names.0, names.1),
                candidate,
            ));
        }
    }
    for first in 0..6 {
        for second in first + 1..6 {
            if first / 2 == second / 2 {
                continue;
            }
            let Some(candidate) = swap_defense_slots(baseline, first, second) else {
                continue;
            };
            let ids = defense_player_ids(baseline, first, second);
            let names = defense_player_names(baseline, first, second);
            configurations.push((
                format!("swap-{}-{}", ids.0.min(ids.1), ids.0.max(ids.1)),
                format!("Swap {} and {}", names.0, names.1),
                candidate,
            ));
        }
    }
    let mut seen = BTreeSet::new();
    configurations.retain(|(id, _, _)| seen.insert(id.clone()));

    let baseline_score = score_lineup(baseline, pair_evidence);
    let mut candidates = configurations
        .into_iter()
        .map(|(id, label, lineup)| {
            let score = score_lineup(&lineup, pair_evidence);
            let strength_delta = ((score.total - baseline_score.total) * 0.25).clamp(-5.0, 5.0);
            LineCombinationCandidateView {
                is_baseline: id == "baseline",
                id,
                label,
                rank: 0,
                evidence_label: EvidenceLabel::Simulated,
                strength_delta,
                score,
                units: lineup_units(&lineup),
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        b.score
            .total
            .total_cmp(&a.score.total)
            .then_with(|| a.id.cmp(&b.id))
    });
    candidates.truncate(config.max_candidates);
    if !candidates.iter().any(|candidate| candidate.is_baseline) {
        let score = baseline_score;
        candidates.pop();
        candidates.push(LineCombinationCandidateView {
            id: "baseline".to_owned(),
            label: "Submitted lineup".to_owned(),
            rank: 0,
            is_baseline: true,
            evidence_label: EvidenceLabel::Simulated,
            strength_delta: 0.0,
            score,
            units: lineup_units(baseline),
        });
        candidates.sort_by(|a, b| {
            b.score
                .total
                .total_cmp(&a.score.total)
                .then_with(|| a.id.cmp(&b.id))
        });
    }
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = index + 1;
    }

    let mut warnings = baseline
        .warnings
        .iter()
        .filter(|warning| {
            warning.code != "incomplete_roster_shape"
                || baseline
                    .forward_lines
                    .iter()
                    .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
                    .any(Option::is_none)
        })
        .map(|warning| warning.message.clone())
        .collect::<Vec<_>>();
    if config.allow_off_wing
        && baseline.forward_lines.iter().any(|line| {
            [
                (&line.left_wing, Position::LeftWing),
                (&line.center, Position::Center),
                (&line.right_wing, Position::RightWing),
            ]
            .into_iter()
            .any(|(player, position)| {
                player
                    .as_ref()
                    .is_some_and(|player| !player.eligible_positions.contains(&position))
            })
        })
    {
        warnings.push(
            "Flexible forward completion used estimated out-of-position assignments.".to_owned(),
        );
    }
    if pair_evidence.is_empty() {
        warnings.push(
            "No observed pair evidence was supplied; pair contribution is neutral.".to_owned(),
        );
    }
    Ok(LineCombinationForecastView {
        schema: LINE_COMBINATION_FORECAST_SCHEMA.to_owned(),
        method: LINE_COMBINATION_FORECAST_METHOD.to_owned(),
        team: baseline.team.clone(),
        roster_season: baseline.roster_season,
        baseline_id: "baseline".to_owned(),
        candidates,
        player_leaderboards: build_player_leaderboards(baseline, pair_evidence),
        warnings,
        disclosures: vec![
            "Candidates are the submitted lineup plus deterministic legal one-swap alternatives; this is a bounded comparison, not an exhaustive global optimizer.".to_owned(),
            "Talent placement weights each player score by GP/(GP+20), so small samples carry less lineup authority; role fit uses declared eligibility and applies a visible penalty to allowed off-wing assignments. Missing player scores are neutral rather than invented.".to_owned(),
            "Best-overall player order regresses each raw score toward neutral 50 using games/(games+20); raw and reliability-adjusted scores remain separate in the document.".to_owned(),
            "Pair evidence is accepted as labeled input; coarse same-game evidence and simulated assumptions do not establish causal chemistry.".to_owned(),
            "Strength deltas are bounded scenario assumptions on the IceCast 0-100 team-strength scale and require historical calibration before predictive promotion.".to_owned(),
        ],
    })
}

fn build_player_leaderboards(
    lineup: &TeamLineupProjectionView,
    evidence: &[LineCombinationPairEvidenceInput],
) -> LineCombinationPlayerLeaderboardsView {
    let mut best_overall = lineup_players(lineup)
        .into_iter()
        .map(|player| {
            let pair_rows = evidence
                .iter()
                .filter(|row| {
                    row.player_one_id == player.player_id || row.player_two_id == player.player_id
                })
                .collect::<Vec<_>>();
            let deployment_effects = pair_rows
                .iter()
                .filter(|row| row.kind == LineCombinationPairEvidenceKind::ObservedDeployment)
                .map(|row| adjusted_pair_fit(row))
                .collect::<Vec<_>>();
            let effects = pair_rows
                .iter()
                .filter(|row| row.kind != LineCombinationPairEvidenceKind::ObservedDeployment)
                .map(|row| adjusted_pair_fit(row))
                .collect::<Vec<_>>();
            let deployment_affinity_score = (!deployment_effects.is_empty()).then(|| {
                deployment_effects.iter().sum::<f64>() / deployment_effects.len() as f64 * 100.0
            });
            let teammate_effect_score = (!effects.is_empty())
                .then(|| effects.iter().sum::<f64>() / effects.len() as f64 * 100.0);
            let evidence_kinds = pair_rows
                .iter()
                .map(|row| row.kind)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            LineCombinationPlayerInfluenceView {
                player_id: player.player_id,
                display_name: player.display_name.clone(),
                position_group: player.score.position_group,
                overall_score: player.score.value,
                reliability_adjusted_score: player.score.value.map(|value| {
                    let sample = f64::from(player.score.sample_games);
                    50.0 + (value - 50.0) * sample / (sample + 20.0)
                }),
                sample_games: player.score.sample_games,
                overall_evidence_label: player.score.evidence_label,
                deployment_affinity_score,
                deployment_observations: deployment_effects.len(),
                teammate_effect_score,
                pair_evidence_label: pair_evidence_label(
                    &pair_rows
                        .iter()
                        .filter(|row| {
                            row.kind != LineCombinationPairEvidenceKind::ObservedDeployment
                        })
                        .map(|row| row.kind)
                        .collect::<Vec<_>>(),
                ),
                pair_observations: effects.len(),
                partners_helped: effects.iter().filter(|effect| **effect > 0.0).count(),
                partners_hurt: effects.iter().filter(|effect| **effect < 0.0).count(),
                evidence_kinds,
            }
        })
        .collect::<Vec<_>>();
    best_overall.sort_by(|a, b| {
        b.reliability_adjusted_score
            .unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&a.reliability_adjusted_score.unwrap_or(f64::NEG_INFINITY))
            .then_with(|| a.display_name.cmp(&b.display_name))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    let mut deployment_anchors = best_overall
        .iter()
        .filter(|row| row.deployment_affinity_score.is_some())
        .cloned()
        .collect::<Vec<_>>();
    deployment_anchors.sort_by(|a, b| {
        b.deployment_affinity_score
            .unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&a.deployment_affinity_score.unwrap_or(f64::NEG_INFINITY))
            .then_with(|| a.display_name.cmp(&b.display_name))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    let mut positive_multipliers = best_overall
        .iter()
        .filter(|row| row.teammate_effect_score.is_some_and(|value| value > 0.0))
        .cloned()
        .collect::<Vec<_>>();
    positive_multipliers.sort_by(influence_descending);
    let mut negative_multipliers = best_overall
        .iter()
        .filter(|row| row.teammate_effect_score.is_some_and(|value| value < 0.0))
        .cloned()
        .collect::<Vec<_>>();
    negative_multipliers.sort_by(|a, b| influence_descending(b, a));
    LineCombinationPlayerLeaderboardsView {
        best_overall,
        deployment_anchors,
        positive_multipliers,
        negative_multipliers,
    }
}

fn adjusted_pair_fit(row: &LineCombinationPairEvidenceInput) -> f64 {
    let confidence = (f64::from(row.sample) / 20.0).clamp(0.0, 1.0);
    let authority = match row.kind {
        LineCombinationPairEvidenceKind::ObservedDeployment => 0.60,
        LineCombinationPairEvidenceKind::ObservedShift => 1.0,
        LineCombinationPairEvidenceKind::CoarseSameGame => 0.35,
        LineCombinationPairEvidenceKind::SimulatedAssumption => 0.20,
    };
    row.fit * confidence * authority
}

fn pair_evidence_label(kinds: &[LineCombinationPairEvidenceKind]) -> EvidenceLabel {
    if kinds.is_empty() {
        EvidenceLabel::NoRead
    } else if kinds.contains(&LineCombinationPairEvidenceKind::SimulatedAssumption) {
        EvidenceLabel::Simulated
    } else if kinds.contains(&LineCombinationPairEvidenceKind::CoarseSameGame) {
        EvidenceLabel::UnderReview
    } else {
        EvidenceLabel::Confirmed
    }
}

fn influence_descending(
    a: &LineCombinationPlayerInfluenceView,
    b: &LineCombinationPlayerInfluenceView,
) -> std::cmp::Ordering {
    b.teammate_effect_score
        .unwrap_or(f64::NEG_INFINITY)
        .total_cmp(&a.teammate_effect_score.unwrap_or(f64::NEG_INFINITY))
        .then_with(|| a.display_name.cmp(&b.display_name))
        .then_with(|| a.player_id.cmp(&b.player_id))
}

fn validate_pair_evidence(
    lineup: &TeamLineupProjectionView,
    evidence: &[LineCombinationPairEvidenceInput],
) -> Result<(), String> {
    let players = lineup_player_ids(lineup);
    let mut pairs = BTreeSet::new();
    for row in evidence {
        if row.player_one_id == row.player_two_id
            || !players.contains(&row.player_one_id)
            || !players.contains(&row.player_two_id)
        {
            return Err(
                "The Blender pair evidence must reference two distinct roster players".to_owned(),
            );
        }
        if !row.fit.is_finite() || !(-1.0..=1.0).contains(&row.fit) {
            return Err("The Blender pair fit must be between -1 and 1".to_owned());
        }
        let key = ordered_pair(row.player_one_id, row.player_two_id);
        if !pairs.insert(key) {
            return Err("The Blender pair evidence must contain unique player pairs".to_owned());
        }
    }
    Ok(())
}

fn score_lineup(
    lineup: &TeamLineupProjectionView,
    evidence: &[LineCombinationPairEvidenceInput],
) -> LineCombinationScoreView {
    let forward_prior = position_group_prior(
        lineup
            .forward_lines
            .iter()
            .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
            .flatten(),
    );
    let defense_prior = position_group_prior(
        lineup
            .defense_pairs
            .iter()
            .flat_map(|pair| [&pair.left, &pair.right])
            .flatten(),
    );
    let mut talent_sum = 0.0;
    let mut talent_weight = 0.0;
    let mut confidence_sum = 0.0;
    let mut confidence_weight = 0.0;
    let mut role_sum = 0.0;
    let mut role_count = 0.0;
    for line in &lineup.forward_lines {
        let weight = [1.0, 0.82, 0.62, 0.45][usize::from(line.line - 1)];
        for (player, position) in [
            (&line.left_wing, Position::LeftWing),
            (&line.center, Position::Center),
            (&line.right_wing, Position::RightWing),
        ] {
            if let Some(player) = player {
                if let Some(value) = player.score.value {
                    let confidence = sample_confidence(player.score.sample_games);
                    let adjusted = forward_prior + (value - forward_prior) * confidence;
                    talent_sum += adjusted * weight;
                    talent_weight += weight;
                    confidence_sum += weight * confidence;
                    confidence_weight += weight;
                }
                role_sum += if player.eligible_positions.contains(&position) {
                    1.0
                } else {
                    0.65
                };
                role_count += 1.0;
            }
        }
    }
    for pair in &lineup.defense_pairs {
        let weight = [0.95, 0.72, 0.52][usize::from(pair.pair - 1)];
        for player in [&pair.left, &pair.right].into_iter().flatten() {
            if let Some(value) = player.score.value {
                let confidence = sample_confidence(player.score.sample_games);
                let adjusted = defense_prior + (value - defense_prior) * confidence;
                talent_sum += adjusted * weight;
                talent_weight += weight;
                confidence_sum += weight * confidence;
                confidence_weight += weight;
            }
            role_sum += 1.0;
            role_count += 1.0;
        }
    }
    let talent_placement = if talent_weight == 0.0 {
        50.0
    } else {
        talent_sum / talent_weight
    };
    let role_fit = if role_count == 0.0 {
        0.0
    } else {
        role_sum / role_count * 4.0
    };
    let talent_confidence = if confidence_weight == 0.0 {
        0.0
    } else {
        confidence_sum / confidence_weight
    };
    let evidence_by_pair = evidence
        .iter()
        .map(|row| (ordered_pair(row.player_one_id, row.player_two_id), row))
        .collect::<BTreeMap<_, _>>();
    let mut pair_evidence = 0.0;
    for unit in lineup_units(lineup) {
        for first in 0..unit.player_ids.len() {
            for second in first + 1..unit.player_ids.len() {
                if let Some(row) = evidence_by_pair.get(&ordered_pair(
                    unit.player_ids[first],
                    unit.player_ids[second],
                )) {
                    let confidence = (f64::from(row.sample) / 20.0).clamp(0.0, 1.0);
                    let authority = match row.kind {
                        LineCombinationPairEvidenceKind::ObservedDeployment => 0.60,
                        LineCombinationPairEvidenceKind::ObservedShift => 1.0,
                        LineCombinationPairEvidenceKind::CoarseSameGame => 0.35,
                        LineCombinationPairEvidenceKind::SimulatedAssumption => 0.20,
                    };
                    pair_evidence += row.fit * confidence * authority;
                }
            }
        }
    }
    pair_evidence = pair_evidence.clamp(-4.0, 4.0);
    LineCombinationScoreView {
        talent_placement,
        talent_confidence,
        role_fit,
        pair_evidence,
        total: talent_placement + role_fit + pair_evidence,
    }
}

fn swap_forward_slots(
    baseline: &TeamLineupProjectionView,
    first: usize,
    second: usize,
    allow_off_wing: bool,
) -> Option<TeamLineupProjectionView> {
    let mut lineup = baseline.clone();
    let first_position = forward_slot_position(first);
    let second_position = forward_slot_position(second);
    let first_player = forward_slot(&lineup, first)?.clone();
    let second_player = forward_slot(&lineup, second)?.clone();
    if !can_fill_forward_slot(&first_player, second_position, allow_off_wing)
        || !can_fill_forward_slot(&second_player, first_position, allow_off_wing)
    {
        return None;
    }
    *forward_slot_mut(&mut lineup, first) = Some(second_player);
    *forward_slot_mut(&mut lineup, second) = Some(first_player);
    Some(lineup)
}

fn can_fill_forward_slot(
    player: &TeamLineupPlayerView,
    position: Position,
    allow_off_wing: bool,
) -> bool {
    player.eligible_positions.contains(&position)
        || (allow_off_wing
            && matches!(position, Position::LeftWing | Position::RightWing)
            && player
                .eligible_positions
                .iter()
                .any(|eligible| eligible.is_forward()))
}

pub fn complete_flexible_forward_shape(
    baseline: &TeamLineupProjectionView,
) -> TeamLineupProjectionView {
    let mut lineup = baseline.clone();
    // The strict allocator may place a C/LW or C/RW player on their primary
    // wing before discovering that the selected roster needs them at center.
    // Repair that assignment first; the newly open wing can then be filled by
    // an extra under the manager's off-wing policy.
    for target in [1usize, 4, 7, 10] {
        if forward_slot(&lineup, target).is_some() {
            continue;
        }
        let source = [0usize, 2, 3, 5, 6, 8, 9, 11].into_iter().find(|source| {
            forward_slot(&lineup, *source)
                .is_some_and(|player| player.eligible_positions.contains(&Position::Center))
        });
        if let Some(source) = source {
            let player = forward_slot_mut(&mut lineup, source).take();
            *forward_slot_mut(&mut lineup, target) = player;
        }
    }
    let mut candidates = lineup
        .extras
        .iter()
        .filter(|player| player.primary_position.is_forward())
        .cloned()
        .collect::<Vec<_>>();
    let prior = position_group_prior(
        lineup_players(&lineup)
            .into_iter()
            .filter(|player| player.primary_position.is_forward()),
    );
    candidates.sort_by(|a, b| {
        let adjusted = |player: &TeamLineupPlayerView| {
            let value = player.score.value.unwrap_or(prior);
            prior + (value - prior) * sample_confidence(player.score.sample_games)
        };
        adjusted(b)
            .total_cmp(&adjusted(a))
            .then_with(|| a.display_name.cmp(&b.display_name))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    let mut used = BTreeSet::new();
    for line in &mut lineup.forward_lines {
        for (position, slot) in [
            (Position::LeftWing, &mut line.left_wing),
            (Position::Center, &mut line.center),
            (Position::RightWing, &mut line.right_wing),
        ] {
            if slot.is_none() {
                if let Some(player) = candidates
                    .iter()
                    .find(|player| {
                        !used.contains(&player.player_id)
                            && can_fill_forward_slot(player, position, true)
                    })
                    .cloned()
                {
                    used.insert(player.player_id);
                    *slot = Some(player);
                }
            }
        }
    }
    lineup
        .extras
        .retain(|player| !used.contains(&player.player_id));
    let still_incomplete =
        lineup.forward_lines.iter().any(|line| {
            line.left_wing.is_none() || line.center.is_none() || line.right_wing.is_none()
        }) || lineup
            .defense_pairs
            .iter()
            .any(|pair| pair.left.is_none() || pair.right.is_none())
            || lineup.goalies.starter.is_none()
            || lineup.goalies.backup.is_none();
    if !still_incomplete {
        lineup
            .warnings
            .retain(|warning| warning.code != "incomplete_roster_shape");
    }
    rebuild_special_teams(&mut lineup);
    lineup
}

fn sample_confidence(games: u32) -> f64 {
    let games = f64::from(games);
    games / (games + 20.0)
}

fn position_group_prior<'a>(players: impl Iterator<Item = &'a TeamLineupPlayerView>) -> f64 {
    let values = players
        .filter_map(|player| player.score.value)
        .collect::<Vec<_>>();
    if values.is_empty() {
        50.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn swap_defense_slots(
    baseline: &TeamLineupProjectionView,
    first: usize,
    second: usize,
) -> Option<TeamLineupProjectionView> {
    let mut lineup = baseline.clone();
    let first_player = defense_slot(&lineup, first)?.clone();
    let second_player = defense_slot(&lineup, second)?.clone();
    *defense_slot_mut(&mut lineup, first) = Some(second_player);
    *defense_slot_mut(&mut lineup, second) = Some(first_player);
    Some(lineup)
}

fn forward_slot(lineup: &TeamLineupProjectionView, index: usize) -> Option<&TeamLineupPlayerView> {
    let line = &lineup.forward_lines[index / 3];
    match index % 3 {
        0 => line.left_wing.as_ref(),
        1 => line.center.as_ref(),
        _ => line.right_wing.as_ref(),
    }
}

fn forward_slot_mut(
    lineup: &mut TeamLineupProjectionView,
    index: usize,
) -> &mut Option<TeamLineupPlayerView> {
    let line = &mut lineup.forward_lines[index / 3];
    match index % 3 {
        0 => &mut line.left_wing,
        1 => &mut line.center,
        _ => &mut line.right_wing,
    }
}

fn forward_slot_position(index: usize) -> Position {
    match index % 3 {
        0 => Position::LeftWing,
        1 => Position::Center,
        _ => Position::RightWing,
    }
}

fn defense_slot(lineup: &TeamLineupProjectionView, index: usize) -> Option<&TeamLineupPlayerView> {
    let pair = &lineup.defense_pairs[index / 2];
    if index.is_multiple_of(2) {
        pair.left.as_ref()
    } else {
        pair.right.as_ref()
    }
}

fn defense_slot_mut(
    lineup: &mut TeamLineupProjectionView,
    index: usize,
) -> &mut Option<TeamLineupPlayerView> {
    let pair = &mut lineup.defense_pairs[index / 2];
    if index.is_multiple_of(2) {
        &mut pair.left
    } else {
        &mut pair.right
    }
}

fn forward_player_ids(
    lineup: &TeamLineupProjectionView,
    first: usize,
    second: usize,
) -> (u32, u32) {
    (
        forward_slot(lineup, first)
            .expect("candidate slot occupied")
            .player_id,
        forward_slot(lineup, second)
            .expect("candidate slot occupied")
            .player_id,
    )
}

fn defense_player_ids(
    lineup: &TeamLineupProjectionView,
    first: usize,
    second: usize,
) -> (u32, u32) {
    (
        defense_slot(lineup, first)
            .expect("candidate slot occupied")
            .player_id,
        defense_slot(lineup, second)
            .expect("candidate slot occupied")
            .player_id,
    )
}

fn forward_player_names(
    lineup: &TeamLineupProjectionView,
    first: usize,
    second: usize,
) -> (String, String) {
    (
        forward_slot(lineup, first)
            .expect("candidate slot occupied")
            .display_name
            .clone(),
        forward_slot(lineup, second)
            .expect("candidate slot occupied")
            .display_name
            .clone(),
    )
}

fn defense_player_names(
    lineup: &TeamLineupProjectionView,
    first: usize,
    second: usize,
) -> (String, String) {
    (
        defense_slot(lineup, first)
            .expect("candidate slot occupied")
            .display_name
            .clone(),
        defense_slot(lineup, second)
            .expect("candidate slot occupied")
            .display_name
            .clone(),
    )
}

fn lineup_units(lineup: &TeamLineupProjectionView) -> Vec<LineCombinationUnitView> {
    let mut units = lineup
        .forward_lines
        .iter()
        .map(|line| LineCombinationUnitView {
            kind: LineCombinationUnitKind::ForwardLine,
            unit: line.line,
            player_ids: [&line.left_wing, &line.center, &line.right_wing]
                .into_iter()
                .flatten()
                .map(|player| player.player_id)
                .collect(),
        })
        .collect::<Vec<_>>();
    units.extend(lineup.defense_pairs.iter().map(|pair| {
        LineCombinationUnitView {
            kind: LineCombinationUnitKind::DefensePair,
            unit: pair.pair,
            player_ids: [&pair.left, &pair.right]
                .into_iter()
                .flatten()
                .map(|player| player.player_id)
                .collect(),
        }
    }));
    units
}

fn lineup_player_ids(lineup: &TeamLineupProjectionView) -> BTreeSet<u32> {
    let mut players = lineup_units(lineup)
        .into_iter()
        .flat_map(|unit| unit.player_ids)
        .collect::<BTreeSet<_>>();
    players.extend(lineup.goalies.starter.iter().map(|player| player.player_id));
    players.extend(lineup.goalies.backup.iter().map(|player| player.player_id));
    players.extend(lineup.extras.iter().map(|player| player.player_id));
    players
}

fn lineup_players(lineup: &TeamLineupProjectionView) -> Vec<&TeamLineupPlayerView> {
    let mut players = lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .flatten()
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right])
                .flatten(),
        )
        .chain(lineup.goalies.starter.iter())
        .chain(lineup.goalies.backup.iter())
        .chain(lineup.extras.iter())
        .collect::<Vec<_>>();
    players.sort_by_key(|player| player.player_id);
    players
}

fn ordered_pair(first: u32, second: u32) -> (u32, u32) {
    (first.min(second), first.max(second))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::team_ceiling::TeamCeilingLens;
    use crate::view_model::team_lineup::{
        build_team_lineup_projection, LineupAssignmentEvidence, TeamLineupPlayerInput,
        TeamLineupRequestedSlot,
    };

    fn player(
        id: u32,
        position: Position,
        slot: TeamLineupRequestedSlot,
        value: f64,
    ) -> TeamLineupPlayerInput {
        TeamLineupPlayerInput {
            player_id: id,
            display_name: format!("Player {id}"),
            team: "NYR".to_owned(),
            prior_team: None,
            primary_position: position,
            eligible_positions: vec![position],
            headshot_canonical_url: None,
            games_played: 82,
            lens_scores: TeamCeilingLens::ALL
                .into_iter()
                .map(|lens| (lens, Some(value)))
                .collect(),
            score_evidence: EvidenceLabel::Estimated,
            power_play_role_score: Some(value),
            penalty_kill_role_score: Some(value),
            special_teams_evidence: Some(EvidenceLabel::Estimated),
            requested_slot: Some(slot),
            assignment_evidence: LineupAssignmentEvidence::Scenario,
        }
    }

    fn lineup() -> TeamLineupProjectionView {
        let mut players = Vec::new();
        let mut id = 1;
        for line in 1..=4 {
            for (position, forward_position) in [
                (
                    Position::LeftWing,
                    super::super::team_lineup::LineupForwardPosition::LeftWing,
                ),
                (
                    Position::Center,
                    super::super::team_lineup::LineupForwardPosition::Center,
                ),
                (
                    Position::RightWing,
                    super::super::team_lineup::LineupForwardPosition::RightWing,
                ),
            ] {
                let value = if id == 2 {
                    20.0
                } else if id == 11 {
                    120.0
                } else {
                    60.0
                };
                players.push(player(
                    id,
                    position,
                    TeamLineupRequestedSlot::Forward {
                        line,
                        position: forward_position,
                    },
                    value,
                ));
                id += 1;
            }
        }
        for pair in 1..=3 {
            for right_side in [false, true] {
                players.push(player(
                    id,
                    Position::Defense,
                    TeamLineupRequestedSlot::Defense { pair, right_side },
                    50.0,
                ));
                id += 1;
            }
        }
        for starter in [true, false] {
            players.push(player(
                id,
                Position::Goalie,
                TeamLineupRequestedSlot::Goalie { starter },
                70.0,
            ));
            id += 1;
        }
        build_team_lineup_projection("NYR", 20262027, players).unwrap()
    }

    #[test]
    fn ranks_legal_swap_above_a_misplaced_baseline_and_keeps_baseline() {
        let view = build_line_combination_forecast(
            &lineup(),
            &[],
            LineCombinationForecastConfig {
                max_candidates: 8,
                ..LineCombinationForecastConfig::default()
            },
        )
        .unwrap();
        assert_eq!(view.schema, LINE_COMBINATION_FORECAST_SCHEMA);
        assert!(view
            .candidates
            .iter()
            .any(|candidate| candidate.is_baseline));
        assert!(!view.candidates[0].is_baseline);
        assert!(view.candidates[0].strength_delta > 0.0);
        assert!(view.candidates[0].label.contains("Player"));
        assert!(view
            .warnings
            .iter()
            .any(|warning| warning.contains("No observed pair evidence")));
        assert!(!view.player_leaderboards.best_overall.is_empty());
        assert!(view.player_leaderboards.positive_multipliers.is_empty());
        assert!(view.player_leaderboards.negative_multipliers.is_empty());
    }

    #[test]
    fn pair_evidence_rewards_players_when_kept_on_the_same_unit() {
        let evidence = LineCombinationPairEvidenceInput {
            player_one_id: 1,
            player_two_id: 2,
            fit: 1.0,
            sample: 20,
            kind: LineCombinationPairEvidenceKind::ObservedShift,
        };
        let view = build_line_combination_forecast(
            &lineup(),
            &[evidence],
            LineCombinationForecastConfig::default(),
        )
        .unwrap();
        let baseline = view
            .candidates
            .iter()
            .find(|candidate| candidate.is_baseline)
            .unwrap();
        assert_eq!(baseline.score.pair_evidence, 1.0);
        assert_eq!(view.player_leaderboards.positive_multipliers.len(), 2);
        assert_eq!(
            view.player_leaderboards.positive_multipliers[0].pair_evidence_label,
            EvidenceLabel::Confirmed
        );
    }

    #[test]
    fn observed_deployment_does_not_claim_teammate_performance_impact() {
        let view = build_line_combination_forecast(
            &lineup(),
            &[LineCombinationPairEvidenceInput {
                player_one_id: 1,
                player_two_id: 2,
                fit: 0.75,
                sample: 20,
                kind: LineCombinationPairEvidenceKind::ObservedDeployment,
            }],
            LineCombinationForecastConfig::default(),
        )
        .unwrap();
        assert_eq!(view.player_leaderboards.deployment_anchors.len(), 2);
        assert!(view.player_leaderboards.positive_multipliers.is_empty());
        assert!(view.player_leaderboards.negative_multipliers.is_empty());
        assert!(view.player_leaderboards.deployment_anchors[0]
            .deployment_affinity_score
            .is_some());
        assert_eq!(
            view.player_leaderboards.deployment_anchors[0].pair_evidence_label,
            EvidenceLabel::NoRead
        );
    }

    #[test]
    fn small_gp_samples_have_less_lineup_placement_authority() {
        let mut uncertain = lineup();
        let prospect = uncertain.forward_lines[2].left_wing.as_mut().unwrap();
        prospect.score.value = Some(80.0);
        prospect.score.sample_games = 11;
        let uncertain_swap = swap_forward_slots(&uncertain, 6, 9, false).unwrap();
        let uncertain_delta = score_lineup(&uncertain, &[]).talent_placement
            - score_lineup(&uncertain_swap, &[]).talent_placement;

        let mut established = uncertain.clone();
        established.forward_lines[2]
            .left_wing
            .as_mut()
            .unwrap()
            .score
            .sample_games = 82;
        let established_swap = swap_forward_slots(&established, 6, 9, false).unwrap();
        let established_delta = score_lineup(&established, &[]).talent_placement
            - score_lineup(&established_swap, &[]).talent_placement;

        assert!(uncertain_delta.abs() < established_delta.abs());
        assert!(score_lineup(&uncertain, &[]).talent_confidence < 0.8);
    }

    #[test]
    fn coach_can_allow_off_wing_swaps_with_a_role_penalty() {
        let lineup = lineup();
        assert!(swap_forward_slots(&lineup, 6, 8, false).is_none());
        let off_wing = swap_forward_slots(&lineup, 6, 8, true).unwrap();
        assert!(score_lineup(&off_wing, &[]).role_fit < score_lineup(&lineup, &[]).role_fit);
    }

    #[test]
    fn flexible_coach_promotes_extra_forwards_into_empty_wings() {
        let mut incomplete = lineup();
        for (line, right) in [(1usize, false), (2, false), (3, false), (3, true)] {
            let slot = if right {
                &mut incomplete.forward_lines[line].right_wing
            } else {
                &mut incomplete.forward_lines[line].left_wing
            };
            let mut player = slot.take().unwrap();
            player.primary_position = Position::Center;
            player.eligible_positions = vec![Position::Center];
            incomplete.extras.push(player);
        }
        let completed = complete_flexible_forward_shape(&incomplete);
        assert!(completed
            .forward_lines
            .iter()
            .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
            .all(Option::is_some));
        assert_eq!(completed.extras.len(), incomplete.extras.len() - 4);
        assert!(score_lineup(&completed, &[]).role_fit < 4.0);
    }

    #[test]
    fn rejects_pair_evidence_outside_the_roster() {
        let error = build_line_combination_forecast(
            &lineup(),
            &[LineCombinationPairEvidenceInput {
                player_one_id: 1,
                player_two_id: 999,
                fit: 0.5,
                sample: 10,
                kind: LineCombinationPairEvidenceKind::CoarseSameGame,
            }],
            LineCombinationForecastConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("distinct roster players"));
    }

    #[test]
    fn bench_policy_opens_with_baseline_then_uses_ranked_alternatives() {
        let forecast = build_line_combination_forecast(
            &lineup(),
            &[],
            LineCombinationForecastConfig {
                max_candidates: 8,
                ..LineCombinationForecastConfig::default()
            },
        )
        .unwrap();
        let policy = build_adaptive_lineup_policy(&forecast, 6, 0.5, 2, 3).unwrap();
        assert_eq!(policy.team, "NYR");
        assert_eq!(policy.choices.len(), 3);
        assert_eq!(policy.choices[0].id, "baseline");
        assert!(policy.choices[1].strength_delta >= policy.choices[2].strength_delta);
    }

    #[test]
    fn bench_policy_rejects_invalid_review_settings_before_simulation() {
        let forecast = build_line_combination_forecast(
            &lineup(),
            &[],
            LineCombinationForecastConfig::default(),
        )
        .unwrap();
        assert!(build_adaptive_lineup_policy(&forecast, 1, 0.5, 1, 2).is_err());
        assert!(build_adaptive_lineup_policy(&forecast, 6, 1.1, 1, 2).is_err());
    }

    #[test]
    fn best_overall_regresses_tiny_samples_instead_of_ranking_raw_spikes_first() {
        let mut lineup = lineup();
        let tiny_sample = lineup.forward_lines[0].left_wing.as_mut().unwrap();
        tiny_sample.score.value = Some(100.0);
        tiny_sample.score.sample_games = 1;
        let established = lineup.forward_lines[0].center.as_mut().unwrap();
        established.score.value = Some(80.0);
        established.score.sample_games = 82;
        let view =
            build_line_combination_forecast(&lineup, &[], LineCombinationForecastConfig::default())
                .unwrap();
        let tiny_rank = view
            .player_leaderboards
            .best_overall
            .iter()
            .position(|player| player.player_id == 1)
            .unwrap();
        let established_rank = view
            .player_leaderboards
            .best_overall
            .iter()
            .position(|player| player.player_id == 2)
            .unwrap();
        assert!(established_rank < tiny_rank);
    }
}
