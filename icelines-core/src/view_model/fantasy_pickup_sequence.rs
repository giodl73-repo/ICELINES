//! Bounded, deterministic Monday-Sunday fantasy acquisition planning.
//!
//! The optimizer evaluates every retained sequence by rebuilding the roster and
//! re-running the canonical daily lineup assignment after each prefix.  It does
//! not add together independently ranked pickup rows.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::fantasy_assistant::{
    build_fantasy_daily_lineup, FantasyAssistantRules, FantasyLineupPlayerInput,
    FantasyPlayerAvailabilityStatus, FantasyWeekBudgetView,
};
use super::fantasy_today::{FantasyTodayEvidenceRow, FantasyTodayReadinessRow, FantasyTodayState};
use crate::model::Position;
use crate::season_stats::SeasonType;

pub const FANTASY_PICKUP_SEQUENCE_SCHEMA: &str = "fantasy_pickup_sequence.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyPickupSequenceContext {
    pub league_id: String,
    pub league_name: String,
    pub fantasy_team_id: String,
    pub fantasy_team_name: String,
    pub stats_season: String,
    pub season_type: SeasonType,
    pub competition_mode: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub timezone: String,
    pub generated_at: DateTime<Utc>,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPickupSequencePlayerInput {
    pub player_key: String,
    pub nhl_player_id: Option<u32>,
    pub display_name: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    pub projected_per_game: Option<f64>,
    pub game_dates: BTreeSet<NaiveDate>,
    pub status: FantasyPlayerAvailabilityStatus,
    pub initially_rostered: bool,
    pub droppable: bool,
    pub usable_at: DateTime<Utc>,
    pub drop_lock_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPickupTransitionInput {
    pub transition_id: String,
    pub ordinal: u32,
    pub effective_at: DateTime<Utc>,
    pub local_date: NaiveDate,
    pub add_player_key: String,
    pub drop_player_key: Option<String>,
    #[serde(default)]
    pub matchup_points_delta: f64,
    #[serde(default)]
    pub future_schedule_option_value: f64,
    #[serde(default)]
    pub waiver_reacquisition_cost: f64,
    #[serde(default)]
    pub acquisition_budget_cost: f64,
    #[serde(default)]
    pub uncertainty_discount: f64,
    pub conditional_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPickupSequenceInput {
    pub context: FantasyPickupSequenceContext,
    pub rules: FantasyAssistantRules,
    pub budget: FantasyWeekBudgetView,
    pub players: Vec<FantasyPickupSequencePlayerInput>,
    pub transitions: Vec<FantasyPickupTransitionInput>,
    pub max_moves: u8,
    pub beam_width: usize,
    pub alternative_limit: usize,
    #[serde(default)]
    pub readiness: Vec<FantasyTodayReadinessRow>,
    #[serde(default)]
    pub evidence: Vec<FantasyTodayEvidenceRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyPickupCoverageRow {
    pub date: NaiveDate,
    pub scheduled_players: usize,
    pub usable_starts: usize,
    pub benched_collisions: usize,
    pub open_active_slots: usize,
    pub newly_started_player_keys: Vec<String>,
    pub displaced_player_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPickupSequenceMoveRow {
    pub ordinal: u32,
    pub transition_id: String,
    pub effective_at: DateTime<Utc>,
    pub local_date: NaiveDate,
    pub add_player_key: String,
    pub add_player: String,
    /// Frozen platform eligibility used to distinguish goalie and skater moves
    /// during later decision review. Older v1 rows decode as unknown.
    #[serde(default)]
    pub add_positions: Vec<Position>,
    pub drop_player_key: Option<String>,
    pub drop_player: Option<String>,
    #[serde(default)]
    pub drop_positions: Vec<Position>,
    pub marginal_active_value: f64,
    pub newly_usable_dates: Vec<NaiveDate>,
    pub covered_player_keys: Vec<String>,
    pub firmness: String,
    pub conditional_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPickupSequenceRow {
    pub sequence_id: String,
    pub projected_value_delta: f64,
    pub active_points_delta: f64,
    pub incremental_usable_starts: i32,
    pub moves_used: u8,
    pub reserve_after: u8,
    pub moves: Vec<FantasyPickupSequenceMoveRow>,
    pub daily_coverage: Vec<FantasyPickupCoverageRow>,
    pub pre_roster_fingerprint: String,
    pub post_roster_fingerprint: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPickupSequenceView {
    pub schema: String,
    pub context: FantasyPickupSequenceContext,
    pub state: FantasyTodayState,
    pub budget: FantasyWeekBudgetView,
    pub primary_sequence: FantasyPickupSequenceRow,
    pub alternatives: Vec<FantasyPickupSequenceRow>,
    pub holdback_recommendation: String,
    pub readiness: Vec<FantasyTodayReadinessRow>,
    pub evidence: Vec<FantasyTodayEvidenceRow>,
    pub evaluated_states: usize,
    pub beam_width: usize,
    pub truncated: bool,
    pub material_fingerprint: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FantasyPickupSequenceError {
    #[error("week must start on Monday and end six days later")]
    InvalidWeek,
    #[error("planner context and budget week do not match")]
    BudgetWeekMismatch,
    #[error("beam width must be greater than zero")]
    EmptyBeam,
    #[error("duplicate player key '{0}'")]
    DuplicatePlayer(String),
    #[error("duplicate transition ordinal {0} at one effective instant")]
    DuplicateTransitionOrdinal(u32),
    #[error("transition '{0}' references an unknown player")]
    UnknownTransitionPlayer(String),
    #[error("transition '{0}' is outside the planning week or before evaluation")]
    InvalidTransitionTime(String),
    #[error("player '{0}' has no finite projection")]
    NonFiniteProjection(String),
    #[error("transition '{0}' has a non-finite value component")]
    NonFiniteTransition(String),
    #[error("daily lineup failed: {0}")]
    DailyLineup(String),
    #[error("cannot serialize pickup sequence fingerprint: {0}")]
    Fingerprint(String),
}

#[derive(Debug, Clone)]
struct SearchState {
    transition_indexes: Vec<usize>,
    evaluation: SequenceEvaluation,
}

#[derive(Debug, Clone, Default)]
struct SequenceEvaluation {
    active_value: f64,
    usable_starts: usize,
    coverage: Vec<FantasyPickupCoverageRow>,
    final_roster: BTreeSet<String>,
}

pub fn build_fantasy_pickup_sequence(
    mut input: FantasyPickupSequenceInput,
) -> Result<FantasyPickupSequenceView, FantasyPickupSequenceError> {
    validate_input(&input)?;
    input.transitions.sort_by_key(transition_key);
    let players = input
        .players
        .iter()
        .map(|player| (player.player_key.clone(), player))
        .collect::<BTreeMap<_, _>>();
    let initial_roster = input
        .players
        .iter()
        .filter(|player| player.initially_rostered)
        .map(|player| player.player_key.clone())
        .collect::<BTreeSet<_>>();
    let baseline = evaluate_sequence(&input, &players, &initial_roster, &[])?;
    let mut complete = vec![SearchState {
        transition_indexes: Vec::new(),
        evaluation: baseline.clone(),
    }];
    let mut frontier = complete.clone();
    let mut evaluated_states = 1usize;
    let mut truncated = false;
    let hard_depth = input
        .max_moves
        .min(input.budget.acquisitions_remaining)
        .min(input.rules.weekly_acquisition_limit);

    for _depth in 0..hard_depth {
        let mut next = Vec::new();
        for state in &frontier {
            let start = state
                .transition_indexes
                .last()
                .map_or(0, |index| index.saturating_add(1));
            for index in start..input.transitions.len() {
                let transition = &input.transitions[index];
                if !transition_is_legal(&input, &players, &initial_roster, state, transition)? {
                    continue;
                }
                let mut indexes = state.transition_indexes.clone();
                indexes.push(index);
                let evaluation = evaluate_sequence(&input, &players, &initial_roster, &indexes)?;
                if evaluation
                    .coverage
                    .iter()
                    .any(|row| row.open_active_slots > input.rules.active_slot_count())
                {
                    continue;
                }
                evaluated_states += 1;
                next.push(SearchState {
                    transition_indexes: indexes,
                    evaluation,
                });
            }
        }
        if next.is_empty() {
            break;
        }
        next.sort_by(|a, b| compare_states(&input, &baseline, a, b));
        if next.len() > input.beam_width {
            next.truncate(input.beam_width);
            truncated = true;
        }
        complete.extend(next.iter().cloned());
        frontier = next;
    }

    complete.sort_by(|a, b| compare_states(&input, &baseline, a, b));
    complete.dedup_by(|a, b| a.transition_indexes == b.transition_indexes);
    let mut rows = complete
        .iter()
        .map(|state| build_sequence_row(&input, &players, &initial_roster, &baseline, state))
        .collect::<Result<Vec<_>, _>>()?;
    rows.sort_by(compare_sequence_rows);
    let primary_sequence = rows.remove(0);
    let alternatives = rows
        .into_iter()
        .filter(|row| row.sequence_id != primary_sequence.sequence_id)
        .take(input.alternative_limit)
        .collect::<Vec<_>>();
    let state = input
        .readiness
        .iter()
        .map(|row| row.state)
        .max_by_key(|state| readiness_rank(*state))
        .unwrap_or(FantasyTodayState::Ready);
    let holdback_recommendation = if primary_sequence.moves.is_empty() {
        format!(
            "Hold — the current roster is the best evaluated plan; preserve {} acquisition(s)",
            input.budget.acquisitions_remaining
        )
    } else if primary_sequence.reserve_after > 0 {
        format!(
            "Use {} move(s) and preserve {} acquisition(s) for injury or goalie uncertainty",
            primary_sequence.moves_used, primary_sequence.reserve_after
        )
    } else {
        format!(
            "The bounded plan uses all {} remaining acquisition(s)",
            primary_sequence.moves_used
        )
    };
    let mut warnings = Vec::new();
    if truncated {
        warnings.push(format!(
            "bounded beam search retained at most {} states per acquisition depth",
            input.beam_width
        ));
    }
    if input.context.competition_mode != "points" {
        warnings.push(
            "category posture is not yet included in the v1 objective; sequence is provisional"
                .to_owned(),
        );
    }
    let material_fingerprint = fingerprint(&(
        &input.context.league_id,
        &input.context.fantasy_team_id,
        input.context.week_start,
        input.context.week_end,
        &input.context.stats_season,
        input.context.season_type,
        &input.context.competition_mode,
        &input.budget,
        &input.players,
        &input.transitions,
        &primary_sequence,
        &alternatives,
    ))?;
    let final_state =
        if state == FantasyTodayState::Ready && input.context.competition_mode != "points" {
            FantasyTodayState::Provisional
        } else {
            state
        };

    Ok(FantasyPickupSequenceView {
        schema: FANTASY_PICKUP_SEQUENCE_SCHEMA.to_owned(),
        context: input.context,
        state: final_state,
        budget: input.budget,
        primary_sequence,
        alternatives,
        holdback_recommendation,
        readiness: input.readiness,
        evidence: input.evidence,
        evaluated_states,
        beam_width: input.beam_width,
        truncated,
        material_fingerprint,
        warnings,
    })
}

fn validate_input(input: &FantasyPickupSequenceInput) -> Result<(), FantasyPickupSequenceError> {
    if input.context.week_start.weekday() != Weekday::Mon
        || (input.context.week_end - input.context.week_start).num_days() != 6
    {
        return Err(FantasyPickupSequenceError::InvalidWeek);
    }
    if input.context.week_start != input.budget.week_start
        || input.context.week_end != input.budget.week_end
    {
        return Err(FantasyPickupSequenceError::BudgetWeekMismatch);
    }
    if input.beam_width == 0 {
        return Err(FantasyPickupSequenceError::EmptyBeam);
    }
    input
        .rules
        .validate()
        .map_err(FantasyPickupSequenceError::DailyLineup)?;
    let mut player_keys = BTreeSet::new();
    for player in &input.players {
        if !player_keys.insert(player.player_key.clone()) {
            return Err(FantasyPickupSequenceError::DuplicatePlayer(
                player.player_key.clone(),
            ));
        }
        if player
            .projected_per_game
            .is_some_and(|value| !value.is_finite())
        {
            return Err(FantasyPickupSequenceError::NonFiniteProjection(
                player.player_key.clone(),
            ));
        }
    }
    let mut ordinal_keys = BTreeSet::new();
    for transition in &input.transitions {
        if !player_keys.contains(&transition.add_player_key)
            || transition
                .drop_player_key
                .as_ref()
                .is_some_and(|key| !player_keys.contains(key))
        {
            return Err(FantasyPickupSequenceError::UnknownTransitionPlayer(
                transition.transition_id.clone(),
            ));
        }
        if transition.effective_at < input.context.evaluated_at
            || transition.local_date < input.context.week_start
            || transition.local_date > input.context.week_end
        {
            return Err(FantasyPickupSequenceError::InvalidTransitionTime(
                transition.transition_id.clone(),
            ));
        }
        if !ordinal_keys.insert((transition.effective_at, transition.ordinal)) {
            return Err(FantasyPickupSequenceError::DuplicateTransitionOrdinal(
                transition.ordinal,
            ));
        }
        if [
            transition.matchup_points_delta,
            transition.future_schedule_option_value,
            transition.waiver_reacquisition_cost,
            transition.acquisition_budget_cost,
            transition.uncertainty_discount,
        ]
        .into_iter()
        .any(|value| !value.is_finite())
        {
            return Err(FantasyPickupSequenceError::NonFiniteTransition(
                transition.transition_id.clone(),
            ));
        }
    }
    Ok(())
}

fn transition_is_legal(
    input: &FantasyPickupSequenceInput,
    players: &BTreeMap<String, &FantasyPickupSequencePlayerInput>,
    initial_roster: &BTreeSet<String>,
    state: &SearchState,
    transition: &FantasyPickupTransitionInput,
) -> Result<bool, FantasyPickupSequenceError> {
    let moves_after = state.transition_indexes.len() as u8 + 1;
    let allowed = if input
        .budget
        .injury_reserve_releases_on
        .is_some_and(|date| transition.local_date >= date)
    {
        input.budget.acquisitions_remaining
    } else {
        input.budget.proactive_acquisitions_remaining
    };
    if moves_after > allowed.min(input.max_moves) {
        return Ok(false);
    }
    let roster = roster_at_transition(input, initial_roster, &state.transition_indexes, transition);
    let add = players.get(&transition.add_player_key).ok_or_else(|| {
        FantasyPickupSequenceError::UnknownTransitionPlayer(transition.transition_id.clone())
    })?;
    if roster.contains(&transition.add_player_key) || add.usable_at > transition.effective_at {
        return Ok(false);
    }
    let dropped_before = state.transition_indexes.iter().any(|index| {
        input.transitions[*index]
            .drop_player_key
            .as_ref()
            .is_some_and(|key| key == &transition.add_player_key)
    });
    if dropped_before {
        return Ok(false);
    }
    if let Some(drop_key) = transition.drop_player_key.as_ref() {
        if drop_key == &transition.add_player_key || !roster.contains(drop_key) {
            return Ok(false);
        }
        let drop = players.get(drop_key).ok_or_else(|| {
            FantasyPickupSequenceError::UnknownTransitionPlayer(transition.transition_id.clone())
        })?;
        if !drop.droppable
            || drop
                .drop_lock_at
                .is_some_and(|lock_at| lock_at <= transition.effective_at)
        {
            return Ok(false);
        }
    }
    let mut resulting_roster = roster;
    apply_transition(&mut resulting_roster, transition);
    roster_fits_at(input, players, &resulting_roster, transition.local_date)
}

fn roster_at_transition(
    input: &FantasyPickupSequenceInput,
    initial_roster: &BTreeSet<String>,
    indexes: &[usize],
    before: &FantasyPickupTransitionInput,
) -> BTreeSet<String> {
    let mut roster = initial_roster.clone();
    for index in indexes {
        let transition = &input.transitions[*index];
        if transition_key(transition) >= transition_key(before) {
            continue;
        }
        apply_transition(&mut roster, transition);
    }
    roster
}

fn roster_fits_at(
    input: &FantasyPickupSequenceInput,
    players: &BTreeMap<String, &FantasyPickupSequencePlayerInput>,
    roster: &BTreeSet<String>,
    date: NaiveDate,
) -> Result<bool, FantasyPickupSequenceError> {
    let lineup =
        build_fantasy_daily_lineup(input.rules.clone(), lineup_inputs(players, roster, date))
            .map_err(FantasyPickupSequenceError::DailyLineup)?;
    Ok(lineup.overflow.is_empty())
}

fn evaluate_sequence(
    input: &FantasyPickupSequenceInput,
    players: &BTreeMap<String, &FantasyPickupSequencePlayerInput>,
    initial_roster: &BTreeSet<String>,
    indexes: &[usize],
) -> Result<SequenceEvaluation, FantasyPickupSequenceError> {
    let mut result = SequenceEvaluation::default();
    let mut roster = initial_roster.clone();
    let mut cursor = 0usize;
    for offset in 0..=6 {
        let date = input.context.week_start + chrono::Duration::days(offset);
        while cursor < indexes.len() && input.transitions[indexes[cursor]].local_date <= date {
            apply_transition(&mut roster, &input.transitions[indexes[cursor]]);
            cursor += 1;
        }
        let lineup =
            build_fantasy_daily_lineup(input.rules.clone(), lineup_inputs(players, &roster, date))
                .map_err(FantasyPickupSequenceError::DailyLineup)?;
        if !lineup.overflow.is_empty() {
            return Err(FantasyPickupSequenceError::DailyLineup(format!(
                "hypothetical roster exceeds capacity on {date}"
            )));
        }
        let active_keys = lineup
            .active
            .iter()
            .filter(|row| row.has_game && row.status == FantasyPlayerAvailabilityStatus::Healthy)
            .map(|row| row.player_key.clone())
            .collect::<BTreeSet<_>>();
        let scheduled_players = roster
            .iter()
            .filter(|key| {
                players
                    .get(*key)
                    .is_some_and(|player| player.game_dates.contains(&date))
            })
            .count();
        result.active_value += lineup
            .active
            .iter()
            .filter(|row| row.has_game && row.status == FantasyPlayerAvailabilityStatus::Healthy)
            .map(|row| row.projected_value)
            .sum::<f64>();
        result.usable_starts += active_keys.len();
        result.coverage.push(FantasyPickupCoverageRow {
            date,
            scheduled_players,
            usable_starts: active_keys.len(),
            benched_collisions: scheduled_players.saturating_sub(active_keys.len()),
            open_active_slots: lineup.missing_active_slots.len(),
            newly_started_player_keys: active_keys.into_iter().collect(),
            displaced_player_keys: Vec::new(),
        });
    }
    result.final_roster = roster;
    Ok(result)
}

fn lineup_inputs(
    players: &BTreeMap<String, &FantasyPickupSequencePlayerInput>,
    roster: &BTreeSet<String>,
    date: NaiveDate,
) -> Vec<FantasyLineupPlayerInput> {
    roster
        .iter()
        .filter_map(|key| players.get(key).copied())
        .map(|player| FantasyLineupPlayerInput {
            player_key: player.player_key.clone(),
            display_name: player.display_name.clone(),
            nhl_team: player.nhl_team.clone(),
            platform_positions: player.platform_positions.clone(),
            projected_value: player.projected_per_game.unwrap_or_default(),
            has_game: player.game_dates.contains(&date),
            status: player.status,
            locked_slot: None,
            locked: false,
        })
        .collect()
}

fn apply_transition(roster: &mut BTreeSet<String>, transition: &FantasyPickupTransitionInput) {
    if let Some(drop_key) = &transition.drop_player_key {
        roster.remove(drop_key);
    }
    roster.insert(transition.add_player_key.clone());
}

fn build_sequence_row(
    input: &FantasyPickupSequenceInput,
    players: &BTreeMap<String, &FantasyPickupSequencePlayerInput>,
    initial_roster: &BTreeSet<String>,
    baseline: &SequenceEvaluation,
    state: &SearchState,
) -> Result<FantasyPickupSequenceRow, FantasyPickupSequenceError> {
    let mut previous = baseline.clone();
    let mut moves = Vec::new();
    for (position, index) in state.transition_indexes.iter().enumerate() {
        let prefix = &state.transition_indexes[..=position];
        let current = evaluate_sequence(input, players, initial_roster, prefix)?;
        let transition = &input.transitions[*index];
        let add = players[&transition.add_player_key];
        let drop = transition
            .drop_player_key
            .as_ref()
            .and_then(|key| players.get(key).copied());
        let newly_usable_dates = current
            .coverage
            .iter()
            .zip(&previous.coverage)
            .filter(|(after, before)| after.usable_starts > before.usable_starts)
            .map(|(after, _)| after.date)
            .collect::<Vec<_>>();
        let covered_player_keys = current
            .coverage
            .iter()
            .zip(&previous.coverage)
            .flat_map(|(after, before)| {
                after
                    .newly_started_player_keys
                    .iter()
                    .filter(|key| !before.newly_started_player_keys.contains(key))
                    .cloned()
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        moves.push(FantasyPickupSequenceMoveRow {
            ordinal: transition.ordinal,
            transition_id: transition.transition_id.clone(),
            effective_at: transition.effective_at,
            local_date: transition.local_date,
            add_player_key: transition.add_player_key.clone(),
            add_player: add.display_name.clone(),
            add_positions: add.platform_positions.clone(),
            drop_player_key: transition.drop_player_key.clone(),
            drop_player: drop.map(|player| player.display_name.clone()),
            drop_positions: drop
                .map(|player| player.platform_positions.clone())
                .unwrap_or_default(),
            marginal_active_value: current.active_value - previous.active_value,
            newly_usable_dates,
            covered_player_keys,
            firmness: if transition.conditional_reason.is_some() {
                "conditional"
            } else {
                "firm"
            }
            .to_owned(),
            conditional_reason: transition.conditional_reason.clone(),
        });
        previous = current;
    }
    let active_points_delta = state.evaluation.active_value - baseline.active_value;
    let adjustment = state
        .transition_indexes
        .iter()
        .map(|index| {
            let transition = &input.transitions[*index];
            transition.matchup_points_delta + transition.future_schedule_option_value
                - transition.waiver_reacquisition_cost
                - transition.acquisition_budget_cost
                - transition.uncertainty_discount
        })
        .sum::<f64>();
    let moves_used = state.transition_indexes.len() as u8;
    let reserve_after = input
        .budget
        .acquisitions_remaining
        .saturating_sub(moves_used);
    let pre_roster_fingerprint = fingerprint(initial_roster)?;
    let post_roster_fingerprint = fingerprint(&state.evaluation.final_roster)?;
    let sequence_key = state
        .transition_indexes
        .iter()
        .map(|index| transition_key(&input.transitions[*index]))
        .collect::<Vec<_>>();
    let sequence_id = fingerprint(&sequence_key)?;
    let mut daily_coverage = state.evaluation.coverage.clone();
    for (after, before) in daily_coverage.iter_mut().zip(&baseline.coverage) {
        let after_keys = after
            .newly_started_player_keys
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let before_keys = before
            .newly_started_player_keys
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        after.newly_started_player_keys = after_keys.difference(&before_keys).cloned().collect();
        after.displaced_player_keys = before_keys.difference(&after_keys).cloned().collect();
    }
    let reasons = if moves.is_empty() {
        vec!["current roster is the best evaluated bounded state".to_owned()]
    } else {
        vec![format!(
            "adds {} usable start(s) and {:.2} projected net value after recomputing daily assignments",
            state.evaluation.usable_starts as i32 - baseline.usable_starts as i32,
            active_points_delta + adjustment
        )]
    };
    Ok(FantasyPickupSequenceRow {
        sequence_id,
        projected_value_delta: active_points_delta + adjustment,
        active_points_delta,
        incremental_usable_starts: state.evaluation.usable_starts as i32
            - baseline.usable_starts as i32,
        moves_used,
        reserve_after,
        moves,
        daily_coverage,
        pre_roster_fingerprint,
        post_roster_fingerprint,
        reasons,
    })
}

fn transition_key(
    transition: &FantasyPickupTransitionInput,
) -> (DateTime<Utc>, u32, String, String, String) {
    (
        transition.effective_at,
        transition.ordinal,
        transition.add_player_key.clone(),
        transition.drop_player_key.clone().unwrap_or_default(),
        transition.transition_id.clone(),
    )
}

fn score_state(
    input: &FantasyPickupSequenceInput,
    baseline: &SequenceEvaluation,
    state: &SearchState,
) -> f64 {
    state.evaluation.active_value - baseline.active_value
        + state
            .transition_indexes
            .iter()
            .map(|index| {
                let transition = &input.transitions[*index];
                transition.matchup_points_delta + transition.future_schedule_option_value
                    - transition.waiver_reacquisition_cost
                    - transition.acquisition_budget_cost
                    - transition.uncertainty_discount
            })
            .sum::<f64>()
}

fn compare_states(
    input: &FantasyPickupSequenceInput,
    baseline: &SequenceEvaluation,
    a: &SearchState,
    b: &SearchState,
) -> std::cmp::Ordering {
    score_state(input, baseline, b)
        .total_cmp(&score_state(input, baseline, a))
        .then_with(|| a.transition_indexes.len().cmp(&b.transition_indexes.len()))
        .then_with(|| a.transition_indexes.cmp(&b.transition_indexes))
}

fn compare_sequence_rows(
    a: &FantasyPickupSequenceRow,
    b: &FantasyPickupSequenceRow,
) -> std::cmp::Ordering {
    b.projected_value_delta
        .total_cmp(&a.projected_value_delta)
        .then_with(|| a.moves_used.cmp(&b.moves_used))
        .then_with(|| a.sequence_id.cmp(&b.sequence_id))
}

fn readiness_rank(state: FantasyTodayState) -> u8 {
    match state {
        FantasyTodayState::Ready => 0,
        FantasyTodayState::Provisional => 1,
        FantasyTodayState::Blocked => 2,
    }
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, FantasyPickupSequenceError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| FantasyPickupSequenceError::Fingerprint(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use proptest::prelude::*;

    use super::*;

    const VALUE_EPSILON: f64 = 1e-9;

    fn monday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).expect("fixture Monday is valid")
    }

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0)
            .single()
            .expect("fixture timestamp is valid")
    }

    fn player(
        key: &str,
        value: f64,
        dates: &[u32],
        rostered: bool,
        position: Position,
    ) -> FantasyPickupSequencePlayerInput {
        FantasyPickupSequencePlayerInput {
            player_key: key.to_owned(),
            nhl_player_id: None,
            display_name: key.to_owned(),
            nhl_team: key.to_uppercase(),
            platform_positions: vec![position],
            projected_per_game: Some(value),
            game_dates: dates
                .iter()
                .map(|day| NaiveDate::from_ymd_opt(2026, 11, *day).expect("fixture date"))
                .collect(),
            status: FantasyPlayerAvailabilityStatus::Healthy,
            initially_rostered: rostered,
            droppable: true,
            usable_at: at(9, 8),
            drop_lock_at: None,
        }
    }

    fn transition(
        id: &str,
        ordinal: u32,
        day: u32,
        add: &str,
        drop: &str,
    ) -> FantasyPickupTransitionInput {
        FantasyPickupTransitionInput {
            transition_id: id.to_owned(),
            ordinal,
            effective_at: at(day, 8),
            local_date: NaiveDate::from_ymd_opt(2026, 11, day).expect("fixture date"),
            add_player_key: add.to_owned(),
            drop_player_key: Some(drop.to_owned()),
            matchup_points_delta: 0.0,
            future_schedule_option_value: 0.0,
            waiver_reacquisition_cost: 0.0,
            acquisition_budget_cost: 0.25,
            uncertainty_discount: 0.0,
            conditional_reason: None,
        }
    }

    fn input() -> FantasyPickupSequenceInput {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(super::super::FantasyActiveSlotKind::Defense, 1)]);
        rules.bench_slots = 1;
        let budget = FantasyWeekBudgetView {
            schema: "fantasy_week_budget.v1".to_owned(),
            timezone: "UTC".to_owned(),
            week_start: monday(),
            week_end: monday() + Duration::days(6),
            acquisition_limit: 4,
            acquisitions_used: 0,
            acquisitions_remaining: 4,
            can_add: true,
            injury_reserve: 1,
            injury_reserve_active: 1,
            proactive_acquisitions_remaining: 3,
            can_proactively_add: true,
            injury_reserve_releases_on: Some(monday() + Duration::days(5)),
        };
        FantasyPickupSequenceInput {
            context: FantasyPickupSequenceContext {
                league_id: "league".to_owned(),
                league_name: "League".to_owned(),
                fantasy_team_id: "team".to_owned(),
                fantasy_team_name: "Team".to_owned(),
                stats_season: "20262027".to_owned(),
                season_type: SeasonType::Regular,
                competition_mode: "points".to_owned(),
                week_start: monday(),
                week_end: monday() + Duration::days(6),
                timezone: "UTC".to_owned(),
                generated_at: at(9, 7),
                evaluated_at: at(9, 7),
            },
            rules,
            budget,
            players: vec![
                player("starter", 4.0, &[9, 11, 13], true, Position::Defense),
                player("bench", 2.0, &[9, 11], true, Position::Defense),
                player("quiet", 3.0, &[10, 12, 14], false, Position::Defense),
                player("late", 3.5, &[15], false, Position::Defense),
            ],
            transitions: vec![transition("quiet-for-bench", 1, 9, "quiet", "bench")],
            max_moves: 4,
            beam_width: 64,
            alternative_limit: 3,
            readiness: Vec::new(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn l0_sequence_recomputes_quiet_night_value() {
        let view = build_fantasy_pickup_sequence(input()).expect("sequence should build");
        assert_eq!(view.primary_sequence.moves_used, 1);
        assert_eq!(view.primary_sequence.incremental_usable_starts, 3);
        // Quiet scores 3 points on three newly usable dates, less 0.25 move cost.
        assert!((view.primary_sequence.projected_value_delta - 8.75).abs() < VALUE_EPSILON);
        assert_eq!(view.primary_sequence.moves[0].newly_usable_dates.len(), 3);
    }

    #[test]
    fn l0_no_move_wins_when_churn_is_negative() {
        let mut fixture = input();
        fixture.transitions[0].acquisition_budget_cost = 20.0;
        let view = build_fantasy_pickup_sequence(fixture).expect("sequence should build");
        assert!(view.primary_sequence.moves.is_empty());
        assert!(view.holdback_recommendation.starts_with("Hold"));
    }

    #[test]
    fn l0_sequence_rejects_locked_drop_and_preserves_reserve() {
        let mut fixture = input();
        fixture.players[1].drop_lock_at = Some(at(9, 7));
        let view = build_fantasy_pickup_sequence(fixture).expect("sequence should build");
        assert!(view.primary_sequence.moves.is_empty());
        assert_eq!(view.primary_sequence.reserve_after, 4);
    }

    #[test]
    fn l0_sequence_is_deterministic_and_generation_time_is_not_material() {
        let first = build_fantasy_pickup_sequence(input()).expect("first sequence");
        let mut changed = input();
        changed.context.generated_at += Duration::hours(1);
        let second = build_fantasy_pickup_sequence(changed).expect("second sequence");
        assert_eq!(first.primary_sequence, second.primary_sequence);
        assert_eq!(first.material_fingerprint, second.material_fingerprint);
    }

    #[test]
    fn l0_sequence_rejects_non_finite_values_and_duplicate_ordinals() {
        let mut non_finite = input();
        non_finite.players[0].projected_per_game = Some(f64::NAN);
        assert!(matches!(
            build_fantasy_pickup_sequence(non_finite),
            Err(FantasyPickupSequenceError::NonFiniteProjection(_))
        ));

        let mut duplicate = input();
        let mut second = duplicate.transitions[0].clone();
        second.transition_id = "duplicate".to_owned();
        duplicate.transitions.push(second);
        assert!(matches!(
            build_fantasy_pickup_sequence(duplicate),
            Err(FantasyPickupSequenceError::DuplicateTransitionOrdinal(1))
        ));
    }

    #[test]
    fn l0_sequence_releases_reserve_on_saturday() {
        let mut fixture = input();
        fixture.budget.proactive_acquisitions_remaining = 0;
        fixture.transitions = vec![transition("late-for-bench", 1, 14, "late", "bench")];
        let view = build_fantasy_pickup_sequence(fixture).expect("sequence should build");
        assert_eq!(view.primary_sequence.moves_used, 1);
    }

    #[test]
    fn l0_sequence_can_stream_out_an_earlier_pickup() {
        let mut fixture = input();
        fixture
            .players
            .push(player("early", 3.0, &[10], false, Position::Defense));
        fixture.players[3].game_dates = [
            NaiveDate::from_ymd_opt(2026, 11, 12).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(),
        ]
        .into_iter()
        .collect();
        fixture.transitions = vec![
            transition("early-for-bench", 1, 9, "early", "bench"),
            transition("late-for-bench", 2, 11, "late", "bench"),
            transition("late-for-early", 3, 11, "late", "early"),
        ];
        let view = build_fantasy_pickup_sequence(fixture).expect("sequence should build");
        assert_eq!(view.primary_sequence.moves_used, 2);
        assert_eq!(view.primary_sequence.moves[0].add_player_key, "early");
        assert_eq!(
            view.primary_sequence.moves[1].drop_player_key.as_deref(),
            Some("early")
        );
    }

    proptest! {
        #[test]
        fn l0_sequence_never_exceeds_move_or_budget_prefix(
            max_moves in 0_u8..=4,
            acquisitions_remaining in 0_u8..=4,
        ) {
            let mut fixture = input();
            fixture.max_moves = max_moves;
            fixture.budget.acquisitions_remaining = acquisitions_remaining;
            fixture.budget.can_add = acquisitions_remaining > 0;
            fixture.budget.proactive_acquisitions_remaining =
                acquisitions_remaining.saturating_sub(fixture.budget.injury_reserve_active);
            fixture.budget.can_proactively_add =
                fixture.budget.proactive_acquisitions_remaining > 0;

            let view = build_fantasy_pickup_sequence(fixture).expect("bounded sequence");
            prop_assert!(view.primary_sequence.moves_used <= max_moves);
            prop_assert!(view.primary_sequence.moves_used <= acquisitions_remaining);
            prop_assert_eq!(
                view.primary_sequence.reserve_after,
                acquisitions_remaining - view.primary_sequence.moves_used
            );
            for pair in view.primary_sequence.moves.windows(2) {
                prop_assert!(pair[0].effective_at <= pair[1].effective_at);
            }
        }
    }
}
