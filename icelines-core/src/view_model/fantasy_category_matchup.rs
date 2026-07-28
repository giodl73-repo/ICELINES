use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::{
    build_fantasy_daily_lineup, FantasyAssistantRules, FantasyCategoryAggregation,
    FantasyCategoryDirection, FantasyCategoryRule, FantasyCompetitionMode, FantasyCompetitionRules,
    FantasyLineupPlayerInput, FantasyMatchupStrategy, FantasyPlayerAvailabilityStatus,
};

pub const FANTASY_CATEGORY_MATCHUP_SCHEMA: &str = "fantasy_category_matchup.v1";
pub const FANTASY_CATEGORY_SNAPSHOT_SCHEMA: &str = "fantasy_category_snapshot.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyCategoryScope {
    Skater,
    Goalie,
    All,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryRateInput {
    pub numerator_per_game: f64,
    pub denominator_per_game: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryPlayerInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub lineup_priority_per_game: f64,
    /// Expected appearance share on a scheduled team game; 1.0 for skaters.
    pub appearance_probability: f64,
    pub game_dates: BTreeSet<NaiveDate>,
    pub status: FantasyPlayerAvailabilityStatus,
    pub category_rates: BTreeMap<String, FantasyCategoryRateInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryTeamInput {
    pub team: String,
    pub players: Vec<FantasyCategoryPlayerInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategorySnapshotComponents {
    pub numerator: f64,
    pub denominator: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategorySnapshotRow {
    pub key: String,
    pub user: FantasyCategorySnapshotComponents,
    pub opponent: FantasyCategorySnapshotComponents,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategorySnapshotInput {
    pub schema: String,
    pub through_date: NaiveDate,
    pub source: String,
    pub user_goalie_appearances: f64,
    pub opponent_goalie_appearances: f64,
    pub categories: Vec<FantasyCategorySnapshotRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryMatchupInput {
    pub league: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub rules: FantasyCompetitionRules,
    pub roster_rules: FantasyAssistantRules,
    pub strategy: FantasyMatchupStrategy,
    pub user_is_higher_seed: Option<bool>,
    pub category_scopes: BTreeMap<String, FantasyCategoryScope>,
    pub user: FantasyCategoryTeamInput,
    pub opponent: FantasyCategoryTeamInput,
    pub current_snapshot: Option<FantasyCategorySnapshotInput>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryProjectedValue {
    pub numerator: f64,
    pub denominator: f64,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyCategoryProjectedResult {
    Win,
    Tie,
    Loss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyCategoryClassification {
    Safe,
    Press,
    Volatile,
    LowReturn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryMatchupRow {
    pub key: String,
    pub label: String,
    pub direction: FantasyCategoryDirection,
    pub aggregation: FantasyCategoryAggregation,
    pub scope: FantasyCategoryScope,
    pub user_current: FantasyCategoryProjectedValue,
    pub user_remaining: FantasyCategoryProjectedValue,
    pub user: FantasyCategoryProjectedValue,
    pub opponent_current: FantasyCategoryProjectedValue,
    pub opponent_remaining: FantasyCategoryProjectedValue,
    pub opponent: FantasyCategoryProjectedValue,
    pub projected_result: FantasyCategoryProjectedResult,
    pub user_win_probability: f64,
    pub tie_probability: f64,
    pub opponent_win_probability: f64,
    pub classification: FantasyCategoryClassification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryMatchupView {
    pub schema: String,
    pub competition_mode: String,
    pub league: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub matchup_state: String,
    pub current_through_date: Option<NaiveDate>,
    pub current_totals_source: Option<String>,
    pub strategy: FantasyMatchupStrategy,
    pub user_team: String,
    pub opponent_team: String,
    pub user_goalie_appearances: f64,
    pub opponent_goalie_appearances: f64,
    pub user_current_goalie_appearances: f64,
    pub opponent_current_goalie_appearances: f64,
    pub user_remaining_goalie_appearances: f64,
    pub opponent_remaining_goalie_appearances: f64,
    pub minimum_goalie_appearances: u8,
    pub user_meets_goalie_minimum: bool,
    pub opponent_meets_goalie_minimum: bool,
    pub projected_category_wins: usize,
    pub projected_category_ties: usize,
    pub projected_category_losses: usize,
    pub projected_matchup_result: FantasyCategoryProjectedResult,
    pub modeled_matchup_win_probability: f64,
    pub categories: Vec<FantasyCategoryMatchupRow>,
    pub recommendation: String,
    pub model_notes: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn build_fantasy_category_matchup(
    input: FantasyCategoryMatchupInput,
) -> Result<FantasyCategoryMatchupView, String> {
    input.rules.validate()?;
    input.roster_rules.validate()?;
    if input.rules.mode != FantasyCompetitionMode::Categories {
        return Err("category matchup requires category competition rules".to_owned());
    }
    if input.week_end < input.week_start {
        return Err("matchup week end cannot precede its start".to_owned());
    }
    if input.user.team == input.opponent.team {
        return Err("category matchup requires two different teams".to_owned());
    }
    if input.rules.matchup_tie_policy == super::FantasyMatchupTiePolicy::HigherSeedWins
        && input.user_is_higher_seed.is_none()
    {
        return Err(
            "higher_seed_wins tie policy requires the user's matchup seed ordering".to_owned(),
        );
    }
    for rule in &input.rules.categories {
        if !input.category_scopes.contains_key(&rule.key) {
            return Err(format!(
                "category '{}' is missing its player scope",
                rule.key
            ));
        }
    }

    let (
        projection_start,
        user_current_values,
        opponent_current_values,
        user_current_goalie_appearances,
        opponent_current_goalie_appearances,
        matchup_state,
        current_through_date,
        current_totals_source,
    ) = match &input.current_snapshot {
        Some(snapshot) => {
            let (user, opponent) = validate_snapshot(
                snapshot,
                input.week_start,
                input.week_end,
                &input.rules.categories,
            )?;
            (
                snapshot.through_date + Duration::days(1),
                user,
                opponent,
                snapshot.user_goalie_appearances,
                snapshot.opponent_goalie_appearances,
                if snapshot.through_date == input.week_end {
                    "final"
                } else {
                    "in_week"
                },
                Some(snapshot.through_date),
                Some(snapshot.source.clone()),
            )
        }
        None => (
            input.week_start,
            zero_components(&input.rules.categories),
            zero_components(&input.rules.categories),
            0.0,
            0.0,
            "pre_week",
            None,
            None,
        ),
    };

    let (user_remaining_values, user_remaining_goalie_appearances) = project_team(
        &input.user,
        projection_start,
        input.week_end,
        &input.rules.categories,
        &input.roster_rules,
    )?;
    let (opponent_remaining_values, opponent_remaining_goalie_appearances) = project_team(
        &input.opponent,
        projection_start,
        input.week_end,
        &input.rules.categories,
        &input.roster_rules,
    )?;
    let user_goalie_appearances =
        user_current_goalie_appearances + user_remaining_goalie_appearances;
    let opponent_goalie_appearances =
        opponent_current_goalie_appearances + opponent_remaining_goalie_appearances;
    let minimum = f64::from(input.rules.minimum_goalie_appearances);
    let user_meets_goalie_minimum = user_goalie_appearances >= minimum;
    let opponent_meets_goalie_minimum = opponent_goalie_appearances >= minimum;

    let mut categories = Vec::with_capacity(input.rules.categories.len());
    for rule in &input.rules.categories {
        let scope = input.category_scopes[&rule.key];
        let user_current = projected_value(rule, user_current_values.get(&rule.key));
        let user_remaining = projected_value(rule, user_remaining_values.get(&rule.key));
        let user = projected_value(
            rule,
            Some(&add_components(
                user_current_values.get(&rule.key),
                user_remaining_values.get(&rule.key),
            )),
        );
        let opponent_current = projected_value(rule, opponent_current_values.get(&rule.key));
        let opponent_remaining = projected_value(rule, opponent_remaining_values.get(&rule.key));
        let opponent = projected_value(
            rule,
            Some(&add_components(
                opponent_current_values.get(&rule.key),
                opponent_remaining_values.get(&rule.key),
            )),
        );
        let goalie_minimum_result =
            (scope == FantasyCategoryScope::Goalie && minimum > 0.0).then(|| {
                match (user_meets_goalie_minimum, opponent_meets_goalie_minimum) {
                    (true, false) => FantasyCategoryProjectedResult::Win,
                    (false, true) => FantasyCategoryProjectedResult::Loss,
                    (false, false) => FantasyCategoryProjectedResult::Tie,
                    (true, true) => compare_values(rule, user.value, opponent.value),
                }
            });
        let projected_result = goalie_minimum_result
            .unwrap_or_else(|| compare_values(rule, user.value, opponent.value));
        let (user_win_probability, tie_probability, opponent_win_probability) =
            result_probabilities(
                rule,
                &user,
                &opponent,
                projected_result,
                goalie_minimum_result,
            );
        let classification = classify(
            projected_result,
            user_win_probability,
            tie_probability,
            opponent_win_probability,
        );
        categories.push(FantasyCategoryMatchupRow {
            key: rule.key.clone(),
            label: rule.label.clone(),
            direction: rule.direction,
            aggregation: rule.aggregation,
            scope,
            user_current,
            user_remaining,
            user,
            opponent_current,
            opponent_remaining,
            opponent,
            projected_result,
            user_win_probability,
            tie_probability,
            opponent_win_probability,
            classification,
        });
    }

    let projected_category_wins = categories
        .iter()
        .filter(|row| row.projected_result == FantasyCategoryProjectedResult::Win)
        .count();
    let projected_category_ties = categories
        .iter()
        .filter(|row| row.projected_result == FantasyCategoryProjectedResult::Tie)
        .count();
    let projected_category_losses =
        categories.len() - projected_category_wins - projected_category_ties;
    let projected_matchup_result = if projected_category_wins > projected_category_losses {
        FantasyCategoryProjectedResult::Win
    } else if projected_category_wins < projected_category_losses {
        FantasyCategoryProjectedResult::Loss
    } else {
        match input.rules.matchup_tie_policy {
            super::FantasyMatchupTiePolicy::Tie => FantasyCategoryProjectedResult::Tie,
            super::FantasyMatchupTiePolicy::HigherSeedWins => {
                if input.user_is_higher_seed == Some(true) {
                    FantasyCategoryProjectedResult::Win
                } else {
                    FantasyCategoryProjectedResult::Loss
                }
            }
        }
    };
    let expected_score = categories
        .iter()
        .map(|row| row.user_win_probability + row.tie_probability * 0.5)
        .sum::<f64>();
    let centered = expected_score - categories.len() as f64 * 0.5;
    let modeled_matchup_win_probability = logistic(centered * 1.5);
    let recommendation = match input.strategy {
        FantasyMatchupStrategy::Floor if projected_category_wins > projected_category_losses => {
            "Protect the projected majority and goalie minimum; avoid trading a safe category for a volatile one."
        }
        FantasyMatchupStrategy::Floor => {
            "Stabilize goalie qualification and the closest counting categories before adding variance."
        }
        FantasyMatchupStrategy::Balanced if projected_category_wins > projected_category_losses => {
            "Protect category breadth; prioritize moves that preserve the projected majority."
        }
        FantasyMatchupStrategy::Balanced if categories.iter().any(|row| {
            row.projected_result == FantasyCategoryProjectedResult::Loss
                && matches!(row.classification, FantasyCategoryClassification::Press | FantasyCategoryClassification::Volatile)
        }) => {
            "Press the closest projected losses; they offer the best path to flipping the matchup."
        }
        FantasyMatchupStrategy::Balanced => {
            "The projected deficit is broad; target a multi-category move instead of chasing one low-return category."
        }
        FantasyMatchupStrategy::Upside => {
            "Prioritize volatile and press categories where one lineup or streaming swing can flip the result."
        }
    };

    Ok(FantasyCategoryMatchupView {
        schema: FANTASY_CATEGORY_MATCHUP_SCHEMA.to_owned(),
        competition_mode: "categories".to_owned(),
        league: input.league,
        week_start: input.week_start,
        week_end: input.week_end,
        matchup_state: matchup_state.to_owned(),
        current_through_date,
        current_totals_source,
        strategy: input.strategy,
        user_team: input.user.team,
        opponent_team: input.opponent.team,
        user_goalie_appearances,
        opponent_goalie_appearances,
        user_current_goalie_appearances,
        opponent_current_goalie_appearances,
        user_remaining_goalie_appearances,
        opponent_remaining_goalie_appearances,
        minimum_goalie_appearances: input.rules.minimum_goalie_appearances,
        user_meets_goalie_minimum,
        opponent_meets_goalie_minimum,
        projected_category_wins,
        projected_category_ties,
        projected_category_losses,
        projected_matchup_result,
        modeled_matchup_win_probability,
        categories,
        recommendation: recommendation.to_owned(),
        model_notes: vec![
            match &input.current_snapshot {
                Some(snapshot) => format!(
                    "current category components through {} are fixed; only later game dates are projected",
                    snapshot.through_date
                ),
                None => "pre-week category projection from completed-season per-game rates and legal daily assignments".to_owned(),
            },
            "ratio categories sum numerator and denominator components before division; player percentages are never averaged".to_owned(),
            "category probabilities are deterministic uncertainty approximations, not betting odds".to_owned(),
        ],
        warnings: input.warnings,
    })
}

#[allow(clippy::type_complexity)]
fn validate_snapshot(
    snapshot: &FantasyCategorySnapshotInput,
    week_start: NaiveDate,
    week_end: NaiveDate,
    rules: &[FantasyCategoryRule],
) -> Result<(BTreeMap<String, (f64, f64)>, BTreeMap<String, (f64, f64)>), String> {
    if snapshot.schema != FANTASY_CATEGORY_SNAPSHOT_SCHEMA {
        return Err(format!(
            "unsupported fantasy category snapshot schema '{}'",
            snapshot.schema
        ));
    }
    if snapshot.through_date < week_start || snapshot.through_date > week_end {
        return Err("category snapshot through_date must be inside the selected week".to_owned());
    }
    if snapshot.source.trim().is_empty() {
        return Err("category snapshot requires a source label".to_owned());
    }
    for (label, appearances) in [
        ("user", snapshot.user_goalie_appearances),
        ("opponent", snapshot.opponent_goalie_appearances),
    ] {
        if !appearances.is_finite() || appearances < 0.0 {
            return Err(format!(
                "category snapshot {label} goalie appearances must be finite and non-negative"
            ));
        }
    }

    let rules_by_key = rules
        .iter()
        .map(|rule| (rule.key.as_str(), rule))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut user = BTreeMap::new();
    let mut opponent = BTreeMap::new();
    for row in &snapshot.categories {
        let Some(rule) = rules_by_key.get(row.key.as_str()) else {
            return Err(format!(
                "category snapshot contains unconfigured category '{}'",
                row.key
            ));
        };
        if !seen.insert(row.key.clone()) {
            return Err(format!(
                "category snapshot contains duplicate category '{}'",
                row.key
            ));
        }
        validate_snapshot_components(rule, "user", &row.user)?;
        validate_snapshot_components(rule, "opponent", &row.opponent)?;
        user.insert(row.key.clone(), (row.user.numerator, row.user.denominator));
        opponent.insert(
            row.key.clone(),
            (row.opponent.numerator, row.opponent.denominator),
        );
    }
    let missing = rules
        .iter()
        .filter(|rule| !seen.contains(&rule.key))
        .map(|rule| rule.key.clone())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "category snapshot is missing configured categories: {}",
            missing.join(", ")
        ));
    }
    Ok((user, opponent))
}

fn validate_snapshot_components(
    rule: &FantasyCategoryRule,
    side: &str,
    components: &FantasyCategorySnapshotComponents,
) -> Result<(), String> {
    if !components.numerator.is_finite()
        || !components.denominator.is_finite()
        || components.denominator < 0.0
    {
        return Err(format!(
            "category '{}' {side} components must be finite with a non-negative denominator",
            rule.key
        ));
    }
    match rule.aggregation {
        FantasyCategoryAggregation::Sum if components.denominator.abs() > f64::EPSILON => Err(
            format!(
                "counting category '{}' {side} denominator must be zero",
                rule.key
            ),
        ),
        FantasyCategoryAggregation::Ratio
            if components.numerator < 0.0
                || (components.denominator <= f64::EPSILON
                    && components.numerator.abs() > f64::EPSILON) =>
        {
            Err(format!(
                "ratio category '{}' {side} requires non-negative components and a denominator when its numerator is non-zero",
                rule.key
            ))
        }
        _ => Ok(()),
    }
}

fn zero_components(rules: &[FantasyCategoryRule]) -> BTreeMap<String, (f64, f64)> {
    rules
        .iter()
        .map(|rule| (rule.key.clone(), (0.0, 0.0)))
        .collect()
}

fn add_components(current: Option<&(f64, f64)>, remaining: Option<&(f64, f64)>) -> (f64, f64) {
    let current = current.copied().unwrap_or_default();
    let remaining = remaining.copied().unwrap_or_default();
    (current.0 + remaining.0, current.1 + remaining.1)
}

#[allow(clippy::type_complexity)]
fn project_team(
    input: &FantasyCategoryTeamInput,
    week_start: NaiveDate,
    week_end: NaiveDate,
    rules: &[FantasyCategoryRule],
    roster_rules: &FantasyAssistantRules,
) -> Result<(BTreeMap<String, (f64, f64)>, f64), String> {
    let mut totals = rules
        .iter()
        .map(|rule| (rule.key.clone(), (0.0, 0.0)))
        .collect::<BTreeMap<_, _>>();
    let mut goalie_appearances = 0.0;
    let mut date = week_start;
    while date <= week_end {
        let lineup = build_fantasy_daily_lineup(
            roster_rules.clone(),
            input
                .players
                .iter()
                .map(|player| FantasyLineupPlayerInput {
                    player_key: player.player_key.clone(),
                    display_name: player.player.clone(),
                    nhl_team: player.nhl_team.clone(),
                    platform_positions: player.positions.clone(),
                    projected_value: player.lineup_priority_per_game,
                    has_game: player.game_dates.contains(&date),
                    status: player.status,
                    locked_slot: None,
                    locked: false,
                })
                .collect(),
        )?;
        let active = lineup
            .active
            .iter()
            .filter(|row| row.has_game && row.status.expected_available())
            .map(|row| row.player_key.as_str())
            .collect::<BTreeSet<_>>();
        for player in input
            .players
            .iter()
            .filter(|player| active.contains(player.player_key.as_str()))
        {
            if player.positions.contains(&Position::Goalie) {
                goalie_appearances += player.appearance_probability.clamp(0.0, 1.0);
            }
            for rule in rules {
                if let Some(rate) = player.category_rates.get(&rule.key) {
                    let entry = totals.entry(rule.key.clone()).or_default();
                    entry.0 += rate.numerator_per_game * player.appearance_probability;
                    entry.1 += rate.denominator_per_game * player.appearance_probability;
                }
            }
        }
        date += Duration::days(1);
    }
    Ok((totals, goalie_appearances))
}

fn projected_value(
    rule: &FantasyCategoryRule,
    components: Option<&(f64, f64)>,
) -> FantasyCategoryProjectedValue {
    let (numerator, denominator) = components.copied().unwrap_or_default();
    let value = match rule.aggregation {
        FantasyCategoryAggregation::Sum => Some(numerator),
        FantasyCategoryAggregation::Ratio if denominator > f64::EPSILON => {
            Some(numerator / denominator)
        }
        FantasyCategoryAggregation::Ratio => None,
    };
    FantasyCategoryProjectedValue {
        numerator,
        denominator,
        value,
    }
}

fn compare_values(
    rule: &FantasyCategoryRule,
    user: Option<f64>,
    opponent: Option<f64>,
) -> FantasyCategoryProjectedResult {
    let (Some(user), Some(opponent)) = (user, opponent) else {
        return match (user, opponent) {
            (Some(_), None) => FantasyCategoryProjectedResult::Win,
            (None, Some(_)) => FantasyCategoryProjectedResult::Loss,
            (None, None) => FantasyCategoryProjectedResult::Tie,
            (Some(_), Some(_)) => unreachable!("both values were handled above"),
        };
    };
    let difference = match rule.direction {
        FantasyCategoryDirection::HigherWins => user - opponent,
        FantasyCategoryDirection::LowerWins => opponent - user,
    };
    if difference.abs() <= rule.tie_epsilon {
        FantasyCategoryProjectedResult::Tie
    } else if difference > 0.0 {
        FantasyCategoryProjectedResult::Win
    } else {
        FantasyCategoryProjectedResult::Loss
    }
}

fn result_probabilities(
    rule: &FantasyCategoryRule,
    user: &FantasyCategoryProjectedValue,
    opponent: &FantasyCategoryProjectedValue,
    result: FantasyCategoryProjectedResult,
    goalie_minimum_result: Option<FantasyCategoryProjectedResult>,
) -> (f64, f64, f64) {
    if goalie_minimum_result.is_some() && result != FantasyCategoryProjectedResult::Tie {
        return if result == FantasyCategoryProjectedResult::Win {
            (1.0, 0.0, 0.0)
        } else {
            (0.0, 0.0, 1.0)
        };
    }
    let (Some(user_value), Some(opponent_value)) = (user.value, opponent.value) else {
        return match result {
            FantasyCategoryProjectedResult::Win => (1.0, 0.0, 0.0),
            FantasyCategoryProjectedResult::Tie => (0.0, 1.0, 0.0),
            FantasyCategoryProjectedResult::Loss => (0.0, 0.0, 1.0),
        };
    };
    let signed_gap = match rule.direction {
        FantasyCategoryDirection::HigherWins => user_value - opponent_value,
        FantasyCategoryDirection::LowerWins => opponent_value - user_value,
    };
    let scale = match rule.aggregation {
        FantasyCategoryAggregation::Sum => {
            (user.numerator.abs() + opponent.numerator.abs() + 1.0).sqrt() * 0.5
        }
        FantasyCategoryAggregation::Ratio => {
            let sample = (user.denominator + opponent.denominator).max(1.0).sqrt();
            (user_value.abs().max(opponent_value.abs()).max(0.01) * 0.35 / sample)
                .max(rule.tie_epsilon.max(0.0001))
        }
    };
    let tie_probability = ((-signed_gap.abs() / scale).exp() * 0.15).clamp(0.0, 0.15);
    let decisive = 1.0 - tie_probability;
    let user_win_probability = logistic(signed_gap / scale) * decisive;
    (
        user_win_probability,
        tie_probability,
        decisive - user_win_probability,
    )
}

fn classify(
    result: FantasyCategoryProjectedResult,
    user_win: f64,
    tie: f64,
    opponent_win: f64,
) -> FantasyCategoryClassification {
    match result {
        FantasyCategoryProjectedResult::Win if user_win >= 0.7 => {
            FantasyCategoryClassification::Safe
        }
        FantasyCategoryProjectedResult::Win => FantasyCategoryClassification::Volatile,
        FantasyCategoryProjectedResult::Tie => FantasyCategoryClassification::Press,
        FantasyCategoryProjectedResult::Loss if opponent_win <= 0.7 || tie >= 0.1 => {
            FantasyCategoryClassification::Press
        }
        FantasyCategoryProjectedResult::Loss => FantasyCategoryClassification::LowReturn,
    }
}

fn logistic(value: f64) -> f64 {
    1.0 / (1.0 + (-1.702 * value).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::{
        FantasyActiveSlotKind, FantasyMatchupTiePolicy, FANTASY_COMPETITION_RULES_SCHEMA,
    };

    fn roster_rules() -> FantasyAssistantRules {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([
            (FantasyActiveSlotKind::Center, 1),
            (FantasyActiveSlotKind::Goalie, 1),
        ]);
        rules
    }

    fn rules() -> FantasyCompetitionRules {
        FantasyCompetitionRules {
            schema: FANTASY_COMPETITION_RULES_SCHEMA.to_owned(),
            mode: FantasyCompetitionMode::Categories,
            categories: vec![
                FantasyCategoryRule {
                    key: "goals".to_owned(),
                    label: "G".to_owned(),
                    direction: FantasyCategoryDirection::HigherWins,
                    aggregation: FantasyCategoryAggregation::Sum,
                    tie_epsilon: 0.0,
                },
                FantasyCategoryRule {
                    key: "save_percentage".to_owned(),
                    label: "SV%".to_owned(),
                    direction: FantasyCategoryDirection::HigherWins,
                    aggregation: FantasyCategoryAggregation::Ratio,
                    tie_epsilon: 0.0001,
                },
            ],
            minimum_goalie_appearances: 1,
            matchup_tie_policy: FantasyMatchupTiePolicy::Tie,
        }
    }

    fn player(
        key: &str,
        position: Position,
        date: NaiveDate,
        rates: BTreeMap<String, FantasyCategoryRateInput>,
    ) -> FantasyCategoryPlayerInput {
        FantasyCategoryPlayerInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: "NYR".to_owned(),
            positions: vec![position],
            lineup_priority_per_game: 1.0,
            appearance_probability: 1.0,
            game_dates: BTreeSet::from([date]),
            status: FantasyPlayerAvailabilityStatus::Healthy,
            category_rates: rates,
        }
    }

    #[test]
    fn category_matchup_aggregates_ratio_components_instead_of_percentages() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let goalie_rates = BTreeMap::from([(
            "save_percentage".to_owned(),
            FantasyCategoryRateInput {
                numerator_per_game: 27.0,
                denominator_per_game: 30.0,
            },
        )]);
        let view = build_fantasy_category_matchup(FantasyCategoryMatchupInput {
            league: "League".to_owned(),
            week_start: date,
            week_end: date,
            rules: rules(),
            roster_rules: roster_rules(),
            strategy: FantasyMatchupStrategy::Balanced,
            user_is_higher_seed: None,
            category_scopes: BTreeMap::from([
                ("goals".to_owned(), FantasyCategoryScope::Skater),
                ("save_percentage".to_owned(), FantasyCategoryScope::Goalie),
            ]),
            user: FantasyCategoryTeamInput {
                team: "Dawgs".to_owned(),
                players: vec![player("goalie", Position::Goalie, date, goalie_rates)],
            },
            opponent: FantasyCategoryTeamInput {
                team: "Rival".to_owned(),
                players: vec![player(
                    "other goalie",
                    Position::Goalie,
                    date,
                    BTreeMap::from([(
                        "save_percentage".to_owned(),
                        FantasyCategoryRateInput {
                            numerator_per_game: 28.0,
                            denominator_per_game: 32.0,
                        },
                    )]),
                )],
            },
            current_snapshot: None,
            warnings: Vec::new(),
        })
        .unwrap();
        let save_percentage = view
            .categories
            .iter()
            .find(|row| row.key == "save_percentage")
            .unwrap();
        assert_eq!(save_percentage.user.value, Some(0.9));
        assert_eq!(save_percentage.opponent.value, Some(0.875));
        assert_eq!(
            save_percentage.projected_result,
            FantasyCategoryProjectedResult::Win
        );
        assert!(view.user_meets_goalie_minimum);
    }

    #[test]
    fn goalie_minimum_forces_goalie_category_loss_without_fabricated_ratio() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let view = build_fantasy_category_matchup(FantasyCategoryMatchupInput {
            league: "League".to_owned(),
            week_start: date,
            week_end: date,
            rules: rules(),
            roster_rules: roster_rules(),
            strategy: FantasyMatchupStrategy::Floor,
            user_is_higher_seed: None,
            category_scopes: BTreeMap::from([
                ("goals".to_owned(), FantasyCategoryScope::Skater),
                ("save_percentage".to_owned(), FantasyCategoryScope::Goalie),
            ]),
            user: FantasyCategoryTeamInput {
                team: "Dawgs".to_owned(),
                players: Vec::new(),
            },
            opponent: FantasyCategoryTeamInput {
                team: "Rival".to_owned(),
                players: vec![player(
                    "goalie",
                    Position::Goalie,
                    date,
                    BTreeMap::from([(
                        "save_percentage".to_owned(),
                        FantasyCategoryRateInput {
                            numerator_per_game: 27.0,
                            denominator_per_game: 30.0,
                        },
                    )]),
                )],
            },
            current_snapshot: None,
            warnings: Vec::new(),
        })
        .unwrap();
        let row = view
            .categories
            .iter()
            .find(|row| row.key == "save_percentage")
            .unwrap();
        assert_eq!(row.projected_result, FantasyCategoryProjectedResult::Loss);
        assert_eq!(row.user.value, None);
        assert_eq!(row.opponent_win_probability, 1.0);
    }

    #[test]
    fn category_snapshot_is_fixed_and_only_later_dates_are_projected() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let tuesday = monday + Duration::days(1);
        let mut competition = rules();
        competition.minimum_goalie_appearances = 0;
        let scorer = FantasyCategoryPlayerInput {
            player_key: "scorer".to_owned(),
            player: "scorer".to_owned(),
            nhl_team: "NYR".to_owned(),
            positions: vec![Position::Center],
            lineup_priority_per_game: 1.0,
            appearance_probability: 1.0,
            game_dates: BTreeSet::from([monday, tuesday]),
            status: FantasyPlayerAvailabilityStatus::Healthy,
            category_rates: BTreeMap::from([(
                "goals".to_owned(),
                FantasyCategoryRateInput {
                    numerator_per_game: 1.0,
                    denominator_per_game: 0.0,
                },
            )]),
        };
        let view = build_fantasy_category_matchup(FantasyCategoryMatchupInput {
            league: "League".to_owned(),
            week_start: monday,
            week_end: tuesday,
            rules: competition,
            roster_rules: roster_rules(),
            strategy: FantasyMatchupStrategy::Balanced,
            user_is_higher_seed: None,
            category_scopes: BTreeMap::from([
                ("goals".to_owned(), FantasyCategoryScope::Skater),
                ("save_percentage".to_owned(), FantasyCategoryScope::Goalie),
            ]),
            user: FantasyCategoryTeamInput {
                team: "Dawgs".to_owned(),
                players: vec![scorer],
            },
            opponent: FantasyCategoryTeamInput {
                team: "Rival".to_owned(),
                players: Vec::new(),
            },
            current_snapshot: Some(FantasyCategorySnapshotInput {
                schema: FANTASY_CATEGORY_SNAPSHOT_SCHEMA.to_owned(),
                through_date: monday,
                source: "Yahoo matchup page".to_owned(),
                user_goalie_appearances: 0.0,
                opponent_goalie_appearances: 0.0,
                categories: vec![
                    FantasyCategorySnapshotRow {
                        key: "goals".to_owned(),
                        user: FantasyCategorySnapshotComponents {
                            numerator: 2.0,
                            denominator: 0.0,
                        },
                        opponent: FantasyCategorySnapshotComponents {
                            numerator: 1.0,
                            denominator: 0.0,
                        },
                    },
                    FantasyCategorySnapshotRow {
                        key: "save_percentage".to_owned(),
                        user: FantasyCategorySnapshotComponents {
                            numerator: 0.0,
                            denominator: 0.0,
                        },
                        opponent: FantasyCategorySnapshotComponents {
                            numerator: 0.0,
                            denominator: 0.0,
                        },
                    },
                ],
            }),
            warnings: Vec::new(),
        })
        .unwrap();

        let goals = view
            .categories
            .iter()
            .find(|row| row.key == "goals")
            .unwrap();
        assert_eq!(view.matchup_state, "in_week");
        assert_eq!(view.current_through_date, Some(monday));
        assert_eq!(goals.user_current.numerator, 2.0);
        assert_eq!(goals.user_remaining.numerator, 1.0);
        assert_eq!(goals.user.value, Some(3.0));
        assert_eq!(goals.opponent.value, Some(1.0));
    }

    #[test]
    fn category_snapshot_rejects_missing_categories_and_invalid_counting_denominators() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let competition = rules();
        let snapshot = FantasyCategorySnapshotInput {
            schema: FANTASY_CATEGORY_SNAPSHOT_SCHEMA.to_owned(),
            through_date: monday,
            source: "manual paste".to_owned(),
            user_goalie_appearances: 0.0,
            opponent_goalie_appearances: 0.0,
            categories: vec![FantasyCategorySnapshotRow {
                key: "goals".to_owned(),
                user: FantasyCategorySnapshotComponents {
                    numerator: 2.0,
                    denominator: 1.0,
                },
                opponent: FantasyCategorySnapshotComponents {
                    numerator: 1.0,
                    denominator: 0.0,
                },
            }],
        };
        let error = validate_snapshot(
            &snapshot,
            monday,
            monday + Duration::days(6),
            &competition.categories,
        )
        .unwrap_err();
        assert!(error.contains("denominator must be zero"));

        let mut corrected = snapshot;
        corrected.categories[0].user.denominator = 0.0;
        let error = validate_snapshot(
            &corrected,
            monday,
            monday + Duration::days(6),
            &competition.categories,
        )
        .unwrap_err();
        assert!(error.contains("missing configured categories"));
    }
}
