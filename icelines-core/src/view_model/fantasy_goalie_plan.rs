use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{FantasyCompetitionMode, FantasyMatchupStrategy, FantasyObservationFreshness};

pub const FANTASY_GOALIE_PLAN_SCHEMA: &str = "fantasy_goalie_plan.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyGoalieStartState {
    ConfirmedStarting,
    ReportedStarting,
    EstimatedStarting,
    ConfirmedBackup,
    ReportedBackup,
    Unknown,
}

impl FantasyGoalieStartState {
    pub fn is_confirmed(self) -> bool {
        matches!(self, Self::ConfirmedStarting | Self::ConfirmedBackup)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyGoalieStartObservation {
    pub player_key: String,
    pub game_date: NaiveDate,
    pub state: FantasyGoalieStartState,
    pub source: String,
    pub source_url: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyResolvedGoalieStart {
    pub player_key: String,
    pub game_date: NaiveDate,
    pub reported_state: FantasyGoalieStartState,
    pub effective_state: FantasyGoalieStartState,
    pub freshness: FantasyObservationFreshness,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub observed_at: Option<DateTime<Utc>>,
    pub age_minutes: Option<i64>,
    pub requires_refresh: bool,
    pub detail: Option<String>,
}

pub fn resolve_fantasy_goalie_start(
    player_key: impl Into<String>,
    game_date: NaiveDate,
    observations: &[FantasyGoalieStartObservation],
    now: DateTime<Utc>,
    max_age_minutes: i64,
) -> FantasyResolvedGoalieStart {
    let player_key = player_key.into();
    let latest = observations
        .iter()
        .filter(|row| row.player_key == player_key && row.game_date == game_date)
        .max_by(|a, b| {
            a.observed_at
                .cmp(&b.observed_at)
                .then_with(|| a.fetched_at.cmp(&b.fetched_at))
                .then_with(|| a.source.cmp(&b.source))
        });
    let Some(observation) = latest else {
        return FantasyResolvedGoalieStart {
            player_key,
            game_date,
            reported_state: FantasyGoalieStartState::Unknown,
            effective_state: FantasyGoalieStartState::Unknown,
            freshness: FantasyObservationFreshness::Missing,
            source: None,
            source_url: None,
            observed_at: None,
            age_minutes: None,
            requires_refresh: true,
            detail: None,
        };
    };
    let age_minutes = now
        .signed_duration_since(observation.observed_at)
        .num_minutes();
    let freshness = if age_minutes < -5 {
        FantasyObservationFreshness::FutureDated
    } else if age_minutes > max_age_minutes.max(0) {
        FantasyObservationFreshness::Stale
    } else {
        FantasyObservationFreshness::Fresh
    };
    let effective_state = if freshness == FantasyObservationFreshness::Fresh {
        observation.state
    } else {
        FantasyGoalieStartState::Unknown
    };
    FantasyResolvedGoalieStart {
        player_key,
        game_date,
        reported_state: observation.state,
        effective_state,
        freshness,
        source: Some(observation.source.clone()),
        source_url: observation.source_url.clone(),
        observed_at: Some(observation.observed_at),
        age_minutes: Some(age_minutes),
        requires_refresh: freshness != FantasyObservationFreshness::Fresh
            || !observation.state.is_confirmed(),
        detail: observation.detail.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyGoalieGameInput {
    pub date: NaiveDate,
    pub start_time_utc: Option<DateTime<Utc>>,
    pub opponent: String,
    pub home: bool,
    pub team_back_to_back: bool,
    /// Opponent scoring index where 1.0 is league average and higher is harder.
    pub opponent_offense_index: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyGoaliePlanPlayerInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub rostered: bool,
    pub acquisition_eligible: bool,
    pub games: Vec<FantasyGoalieGameInput>,
    pub projected_points_per_start: f64,
    pub historical_start_probability: f64,
    pub expected_save_percentage: Option<f64>,
    pub expected_goals_against_average: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyGoaliePlanInput {
    pub league: String,
    pub team: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub focus_date: Option<NaiveDate>,
    pub strategy: FantasyMatchupStrategy,
    pub competition_mode: FantasyCompetitionMode,
    pub goalie_slots: u8,
    pub minimum_goalie_appearances: u8,
    pub current_goalie_appearances: f64,
    pub evaluated_at: DateTime<Utc>,
    pub max_age_minutes: i64,
    pub acquisitions_remaining: u8,
    pub goalies: Vec<FantasyGoaliePlanPlayerInput>,
    pub observations: Vec<FantasyGoalieStartObservation>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyGoaliePlanAction {
    Start,
    Bench,
    Wait,
    Refresh,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyGoalieRefreshUrgency {
    CheckLater,
    RefreshSoon,
    RefreshNow,
    Locked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyGoaliePlanRow {
    pub date: NaiveDate,
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub opponent: String,
    pub home: bool,
    pub team_back_to_back: bool,
    pub opponent_offense_index: f64,
    pub game_start_utc: Option<DateTime<Utc>>,
    pub refresh_deadline_utc: Option<DateTime<Utc>>,
    pub minutes_to_lock: Option<i64>,
    pub refresh_urgency: FantasyGoalieRefreshUrgency,
    pub evidence: FantasyResolvedGoalieStart,
    pub start_probability: f64,
    pub projected_points: f64,
    pub poor_start_points: f64,
    pub expected_save_percentage: Option<f64>,
    pub poor_start_save_percentage: Option<f64>,
    pub expected_goals_against_average: Option<f64>,
    pub poor_start_goals_against_average: Option<f64>,
    pub action: FantasyGoaliePlanAction,
    pub conditional: bool,
    pub occupies_goalie_slot: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyGoalieStreamCandidateRow {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub acquisition_eligible: bool,
    pub game_dates: Vec<NaiveDate>,
    pub opponents: Vec<String>,
    pub back_to_back_games: usize,
    pub confirmed_start_dates: Vec<NaiveDate>,
    pub reported_or_estimated_start_dates: Vec<NaiveDate>,
    pub next_game_start_utc: Option<DateTime<Utc>>,
    pub next_refresh_utc: Option<DateTime<Utc>>,
    pub next_safety_check_utc: Option<DateTime<Utc>>,
    pub expected_appearance_gain: f64,
    pub projected_points_gain: f64,
    pub poor_start_points: f64,
    pub expected_save_percentage: Option<f64>,
    pub expected_goals_against_average: Option<f64>,
    pub poor_start_save_percentage: Option<f64>,
    pub poor_start_goals_against_average: Option<f64>,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyGoaliePortfolioComparison {
    pub rostered_goalies: usize,
    pub acquisitions_remaining: u8,
    pub best_third_goalie: Option<String>,
    pub expected_appearance_gain: f64,
    pub projected_points_gain: f64,
    pub two_goalie_expected_total: f64,
    pub three_goalie_expected_total: f64,
    pub two_goalie_minimum_shortfall: f64,
    pub three_goalie_minimum_shortfall: f64,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyGoaliePlanView {
    pub schema: String,
    pub competition_mode: String,
    pub league: String,
    pub team: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub focus_date: Option<NaiveDate>,
    pub strategy: FantasyMatchupStrategy,
    pub goalie_slots: u8,
    pub minimum_goalie_appearances: u8,
    pub current_goalie_appearances: f64,
    pub expected_remaining_appearances: f64,
    pub confirmed_remaining_appearances: usize,
    pub expected_total_appearances: f64,
    pub confirmed_floor_total_appearances: f64,
    pub minimum_shortfall: f64,
    pub minimum_at_risk: bool,
    pub next_required_refresh_utc: Option<DateTime<Utc>>,
    pub next_safety_check_utc: Option<DateTime<Utc>>,
    pub next_game_lock_utc: Option<DateTime<Utc>>,
    pub refreshes_due_now: usize,
    pub safety_checks_due_now: usize,
    pub unresolved_rostered_goalies_on_focus_date: usize,
    pub rows: Vec<FantasyGoaliePlanRow>,
    pub stream_candidates: Vec<FantasyGoalieStreamCandidateRow>,
    pub portfolio: FantasyGoaliePortfolioComparison,
    pub recommendation: String,
    pub model_notes: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn build_fantasy_goalie_plan(
    input: FantasyGoaliePlanInput,
) -> Result<FantasyGoaliePlanView, String> {
    if input.week_end < input.week_start {
        return Err("goalie-plan week end cannot precede its start".to_owned());
    }
    if input.goalie_slots == 0 {
        return Err("goalie-plan requires at least one goalie slot".to_owned());
    }
    if input.max_age_minutes < 0 {
        return Err("goalie-plan max evidence age cannot be negative".to_owned());
    }
    if !input.current_goalie_appearances.is_finite() || input.current_goalie_appearances < 0.0 {
        return Err("current goalie appearances must be finite and non-negative".to_owned());
    }
    if input
        .focus_date
        .is_some_and(|date| date < input.week_start || date > input.week_end)
    {
        return Err("goalie-plan focus date must be inside the selected week".to_owned());
    }

    let mut by_date = BTreeMap::<NaiveDate, Vec<FantasyGoaliePlanRow>>::new();
    let mut candidate_rows = BTreeMap::<String, Vec<FantasyGoaliePlanRow>>::new();
    for goalie in &input.goalies {
        if !goalie.historical_start_probability.is_finite()
            || !goalie.projected_points_per_start.is_finite()
        {
            return Err(format!(
                "goalie '{}' has non-finite projection inputs",
                goalie.player
            ));
        }
        for game in goalie
            .games
            .iter()
            .filter(|game| game.date >= input.week_start && game.date <= input.week_end)
        {
            if !game.opponent_offense_index.is_finite() || game.opponent_offense_index <= 0.0 {
                return Err(format!(
                    "goalie '{}' has an invalid opponent offense index",
                    goalie.player
                ));
            }
            let evidence = resolve_fantasy_goalie_start(
                goalie.player_key.clone(),
                game.date,
                &input.observations,
                input.evaluated_at,
                input.max_age_minutes,
            );
            let probability = start_probability(
                evidence.effective_state,
                goalie.historical_start_probability.clamp(0.0, 1.0),
                game.team_back_to_back,
            );
            let matchup_factor = (2.0 - game.opponent_offense_index).clamp(0.70, 1.30);
            let adjusted_points_per_start = goalie.projected_points_per_start * matchup_factor;
            let minutes_to_lock = game.start_time_utc.map(|start| {
                start
                    .signed_duration_since(input.evaluated_at)
                    .num_minutes()
            });
            let refresh_urgency = refresh_urgency(minutes_to_lock);
            let row = FantasyGoaliePlanRow {
                date: game.date,
                player_key: goalie.player_key.clone(),
                player: goalie.player.clone(),
                nhl_team: goalie.nhl_team.clone(),
                opponent: game.opponent.clone(),
                home: game.home,
                team_back_to_back: game.team_back_to_back,
                opponent_offense_index: game.opponent_offense_index,
                game_start_utc: game.start_time_utc,
                refresh_deadline_utc: game
                    .start_time_utc
                    .map(|start| start - chrono::Duration::minutes(30)),
                minutes_to_lock,
                refresh_urgency,
                evidence,
                start_probability: probability,
                projected_points: probability * adjusted_points_per_start,
                poor_start_points: adjusted_points_per_start
                    - adjusted_points_per_start.abs() * 0.70,
                expected_save_percentage: goalie.expected_save_percentage.map(|value| {
                    (value - (game.opponent_offense_index - 1.0) * 0.012).clamp(0.0, 1.0)
                }),
                poor_start_save_percentage: goalie.expected_save_percentage.map(|value| {
                    (value - (game.opponent_offense_index - 1.0) * 0.012 - 0.035).max(0.0)
                }),
                expected_goals_against_average: goalie
                    .expected_goals_against_average
                    .map(|value| value * game.opponent_offense_index),
                poor_start_goals_against_average: goalie
                    .expected_goals_against_average
                    .map(|value| value * game.opponent_offense_index + 1.25),
                action: FantasyGoaliePlanAction::Refresh,
                conditional: true,
                occupies_goalie_slot: false,
                reason: String::new(),
            };
            if goalie.rostered {
                by_date.entry(game.date).or_default().push(row);
            } else {
                candidate_rows
                    .entry(goalie.player_key.clone())
                    .or_default()
                    .push(row);
            }
        }
    }

    let mut rows = Vec::new();
    for (_, mut options) in by_date {
        options.sort_by(|a, b| {
            decision_score(b, input.strategy, input.competition_mode)
                .total_cmp(&decision_score(a, input.strategy, input.competition_mode))
                .then_with(|| a.player_key.cmp(&b.player_key))
        });
        for (index, mut row) in options.into_iter().enumerate() {
            let within_capacity = index < usize::from(input.goalie_slots);
            let (action, conditional, reason) = decide_action(
                row.evidence.effective_state,
                within_capacity,
                input.strategy,
                input.competition_mode,
                row.refresh_urgency,
            );
            row.action = action;
            row.conditional = conditional;
            row.occupies_goalie_slot = within_capacity
                && row.action != FantasyGoaliePlanAction::Locked
                && !matches!(
                    row.evidence.effective_state,
                    FantasyGoalieStartState::ConfirmedBackup
                        | FantasyGoalieStartState::ReportedBackup
                );
            row.reason = reason;
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.player_key.cmp(&b.player_key))
    });

    let expected_remaining_appearances = rows
        .iter()
        .filter(|row| row.occupies_goalie_slot)
        .map(|row| row.start_probability)
        .sum::<f64>();
    let confirmed_remaining_appearances = rows
        .iter()
        .filter(|row| {
            row.occupies_goalie_slot
                && row.evidence.effective_state == FantasyGoalieStartState::ConfirmedStarting
        })
        .count();
    let expected_total_appearances =
        input.current_goalie_appearances + expected_remaining_appearances;
    let confirmed_floor_total_appearances =
        input.current_goalie_appearances + confirmed_remaining_appearances as f64;
    let minimum = f64::from(input.minimum_goalie_appearances);
    let minimum_shortfall = (minimum - expected_total_appearances).max(0.0);
    let minimum_at_risk =
        input.minimum_goalie_appearances > 0 && confirmed_floor_total_appearances < minimum;

    let goalie_by_key = input
        .goalies
        .iter()
        .map(|goalie| (goalie.player_key.as_str(), goalie))
        .collect::<BTreeMap<_, _>>();
    let mut stream_candidates = candidate_rows
        .into_iter()
        .filter_map(|(key, game_rows)| {
            let goalie = goalie_by_key.get(key.as_str())?;
            let mut appearance_gain = 0.0;
            let mut points_gain = 0.0;
            for candidate in &game_rows {
                if candidate.refresh_urgency == FantasyGoalieRefreshUrgency::Locked {
                    continue;
                }
                let selected = rows
                    .iter()
                    .filter(|row| row.date == candidate.date && row.occupies_goalie_slot)
                    .collect::<Vec<_>>();
                if selected.len() < usize::from(input.goalie_slots) {
                    appearance_gain += candidate.start_probability;
                    points_gain += candidate.projected_points;
                } else if let Some(weakest) = selected.into_iter().min_by(|a, b| {
                    decision_score(a, input.strategy, input.competition_mode)
                        .total_cmp(&decision_score(b, input.strategy, input.competition_mode))
                }) {
                    appearance_gain +=
                        (candidate.start_probability - weakest.start_probability).max(0.0);
                    points_gain += (candidate.projected_points - weakest.projected_points).max(0.0);
                }
            }
            let acquisition_eligible =
                goalie.acquisition_eligible && input.acquisitions_remaining > 0;
            let recommendation = if !goalie.acquisition_eligible {
                "Not currently a legal free-agent add.".to_owned()
            } else if input.acquisitions_remaining == 0 {
                "No acquisition remains this week; keep as an emergency watch.".to_owned()
            } else if appearance_gain > 0.0 {
                format!(
                    "Adds {:.2} expected usable appearance(s); confirm the starter before adding.",
                    appearance_gain
                )
            } else {
                "Schedule collides with stronger rostered options; do not spend a move now."
                    .to_owned()
            };
            Some(FantasyGoalieStreamCandidateRow {
                player_key: key,
                player: goalie.player.clone(),
                nhl_team: goalie.nhl_team.clone(),
                acquisition_eligible,
                game_dates: game_rows.iter().map(|row| row.date).collect(),
                opponents: game_rows.iter().map(|row| row.opponent.clone()).collect(),
                back_to_back_games: game_rows.iter().filter(|row| row.team_back_to_back).count(),
                confirmed_start_dates: game_rows
                    .iter()
                    .filter(|row| {
                        row.evidence.effective_state == FantasyGoalieStartState::ConfirmedStarting
                    })
                    .map(|row| row.date)
                    .collect(),
                reported_or_estimated_start_dates: game_rows
                    .iter()
                    .filter(|row| {
                        matches!(
                            row.evidence.effective_state,
                            FantasyGoalieStartState::ReportedStarting
                                | FantasyGoalieStartState::EstimatedStarting
                        )
                    })
                    .map(|row| row.date)
                    .collect(),
                next_game_start_utc: game_rows
                    .iter()
                    .filter_map(|row| row.game_start_utc)
                    .filter(|start| *start > input.evaluated_at)
                    .min(),
                next_refresh_utc: game_rows
                    .iter()
                    .filter_map(|row| row_refresh_checkpoint(row, input.evaluated_at))
                    .min(),
                next_safety_check_utc: game_rows
                    .iter()
                    .filter_map(|row| row_safety_checkpoint(row, input.evaluated_at))
                    .min(),
                expected_appearance_gain: appearance_gain,
                projected_points_gain: points_gain,
                poor_start_points: game_rows
                    .iter()
                    .map(|row| row.poor_start_points)
                    .fold(f64::INFINITY, f64::min),
                expected_save_percentage: mean_options(
                    game_rows.iter().map(|row| row.expected_save_percentage),
                ),
                expected_goals_against_average: mean_options(
                    game_rows
                        .iter()
                        .map(|row| row.expected_goals_against_average),
                ),
                poor_start_save_percentage: mean_options(
                    game_rows.iter().map(|row| row.poor_start_save_percentage),
                ),
                poor_start_goals_against_average: mean_options(
                    game_rows
                        .iter()
                        .map(|row| row.poor_start_goals_against_average),
                ),
                recommendation,
            })
        })
        .collect::<Vec<_>>();
    stream_candidates.sort_by(|a, b| {
        b.acquisition_eligible
            .cmp(&a.acquisition_eligible)
            .then_with(|| {
                stream_candidate_score(b, input.strategy, input.competition_mode, minimum_shortfall)
                    .total_cmp(&stream_candidate_score(
                        a,
                        input.strategy,
                        input.competition_mode,
                        minimum_shortfall,
                    ))
            })
            .then_with(|| a.player_key.cmp(&b.player_key))
    });
    stream_candidates.truncate(10);
    let focus_date = input
        .focus_date
        .unwrap_or_else(|| input.evaluated_at.date_naive());
    let next_required_refresh_utc = rows
        .iter()
        .filter_map(|row| row_refresh_checkpoint(row, input.evaluated_at))
        .chain(
            stream_candidates
                .iter()
                .filter(|candidate| candidate.acquisition_eligible)
                .filter_map(|candidate| candidate.next_refresh_utc),
        )
        .min();
    let next_game_lock_utc = rows
        .iter()
        .filter_map(|row| row.game_start_utc)
        .chain(
            stream_candidates
                .iter()
                .filter(|candidate| candidate.acquisition_eligible)
                .filter_map(|candidate| candidate.next_game_start_utc),
        )
        .filter(|start| *start > input.evaluated_at)
        .min();
    let next_safety_check_utc = rows
        .iter()
        .filter_map(|row| row_safety_checkpoint(row, input.evaluated_at))
        .chain(
            stream_candidates
                .iter()
                .filter(|candidate| candidate.acquisition_eligible)
                .filter_map(|candidate| candidate.next_safety_check_utc),
        )
        .min();
    let refreshes_due_now = rows
        .iter()
        .filter_map(|row| row_refresh_checkpoint(row, input.evaluated_at))
        .chain(
            stream_candidates
                .iter()
                .filter(|candidate| candidate.acquisition_eligible)
                .filter_map(|candidate| candidate.next_refresh_utc),
        )
        .filter(|refresh| *refresh <= input.evaluated_at)
        .count();
    let unresolved_rostered_goalies_on_focus_date = rows
        .iter()
        .filter(|row| row.date == focus_date && row.evidence.requires_refresh)
        .count();
    let safety_checks_due_now = rows
        .iter()
        .filter_map(|row| row_safety_checkpoint(row, input.evaluated_at))
        .chain(
            stream_candidates
                .iter()
                .filter(|candidate| candidate.acquisition_eligible)
                .filter_map(|candidate| candidate.next_safety_check_utc),
        )
        .filter(|check| *check <= input.evaluated_at)
        .count();
    let best_stream = stream_candidates.iter().find(|row| {
        row.acquisition_eligible
            && (row.expected_appearance_gain > 0.0 || row.projected_points_gain > 0.0)
    });
    let stream_appearance_gain = best_stream
        .map(|row| row.expected_appearance_gain)
        .unwrap_or_default();
    let stream_points_gain = best_stream
        .map(|row| row.projected_points_gain)
        .unwrap_or_default();
    let three_goalie_expected_total = expected_total_appearances + stream_appearance_gain;
    let portfolio = FantasyGoaliePortfolioComparison {
        rostered_goalies: input
            .goalies
            .iter()
            .filter(|goalie| goalie.rostered)
            .count(),
        acquisitions_remaining: input.acquisitions_remaining,
        best_third_goalie: best_stream.map(|row| row.player.clone()),
        expected_appearance_gain: stream_appearance_gain,
        projected_points_gain: stream_points_gain,
        two_goalie_expected_total: expected_total_appearances,
        three_goalie_expected_total,
        two_goalie_minimum_shortfall: minimum_shortfall,
        three_goalie_minimum_shortfall: (minimum - three_goalie_expected_total).max(0.0),
        recommendation: if let Some(candidate) = best_stream {
            if minimum_shortfall > 0.0 && three_goalie_expected_total >= minimum {
                format!(
                    "Add {} conditionally if the start is confirmed; it projects to rescue the goalie minimum.",
                    candidate.player
                )
            } else if stream_appearance_gain >= 0.75 {
                format!(
                    "{} is the best third-goalie option, but spend the move only after starter confirmation.",
                    candidate.player
                )
            } else {
                "Keep two goalies for now; the best available third-goalie gain is marginal."
                    .to_owned()
            }
        } else {
            "Keep the current goalie group; no legal positive stream is available.".to_owned()
        },
    };
    let recommendation = if minimum_shortfall > 0.0 {
        format!(
            "Goalie minimum is projected short by {:.1}; refresh unknown starts and review a legal stream.",
            minimum_shortfall
        )
    } else if minimum_at_risk {
        "Expected appearances clear the minimum, but the confirmed floor does not; preserve flexibility and refresh before lock."
            .to_owned()
    } else if input.strategy == FantasyMatchupStrategy::Floor {
        "The confirmed appearance floor is sufficient; avoid unnecessary ratio downside.".to_owned()
    } else {
        "Goalie appearance coverage is sufficient; use evidence state and matchup posture for each start."
            .to_owned()
    };

    Ok(FantasyGoaliePlanView {
        schema: FANTASY_GOALIE_PLAN_SCHEMA.to_owned(),
        competition_mode: input.competition_mode.label().to_owned(),
        league: input.league,
        team: input.team,
        week_start: input.week_start,
        week_end: input.week_end,
        focus_date: input.focus_date,
        strategy: input.strategy,
        goalie_slots: input.goalie_slots,
        minimum_goalie_appearances: input.minimum_goalie_appearances,
        current_goalie_appearances: input.current_goalie_appearances,
        expected_remaining_appearances,
        confirmed_remaining_appearances,
        expected_total_appearances,
        confirmed_floor_total_appearances,
        minimum_shortfall,
        minimum_at_risk,
        next_required_refresh_utc,
        next_safety_check_utc,
        next_game_lock_utc,
        refreshes_due_now,
        safety_checks_due_now,
        unresolved_rostered_goalies_on_focus_date,
        rows,
        stream_candidates,
        portfolio,
        recommendation,
        model_notes: vec![
            "confirmed, reported, estimated, and unknown starter evidence remain distinct".to_owned(),
            "missing or stale evidence uses historical workload only as a probability and always requests refresh".to_owned(),
            "poor-start stress subtracts 70% of per-start points, 0.035 SV%, and adds 1.25 GAA".to_owned(),
            "opponent offense is a disclosed relative index; back-to-back workload is discounted unless a start is confirmed".to_owned(),
        ],
        warnings: input.warnings,
    })
}

fn start_probability(
    state: FantasyGoalieStartState,
    historical: f64,
    team_back_to_back: bool,
) -> f64 {
    let workload = historical * if team_back_to_back { 0.65 } else { 1.0 };
    match state {
        FantasyGoalieStartState::ConfirmedStarting => 1.0,
        FantasyGoalieStartState::ReportedStarting => {
            0.85 * if team_back_to_back { 0.80 } else { 1.0 }
        }
        FantasyGoalieStartState::EstimatedStarting => workload.max(0.45),
        FantasyGoalieStartState::ConfirmedBackup => 0.0,
        FantasyGoalieStartState::ReportedBackup => workload.min(0.15),
        FantasyGoalieStartState::Unknown => workload,
    }
}

fn refresh_urgency(minutes_to_lock: Option<i64>) -> FantasyGoalieRefreshUrgency {
    match minutes_to_lock {
        Some(minutes) if minutes <= 0 => FantasyGoalieRefreshUrgency::Locked,
        Some(minutes) if minutes <= 60 => FantasyGoalieRefreshUrgency::RefreshNow,
        Some(minutes) if minutes <= 180 => FantasyGoalieRefreshUrgency::RefreshSoon,
        _ => FantasyGoalieRefreshUrgency::CheckLater,
    }
}

fn row_refresh_checkpoint(
    row: &FantasyGoaliePlanRow,
    evaluated_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if !row.evidence.requires_refresh || row.refresh_urgency == FantasyGoalieRefreshUrgency::Locked
    {
        return None;
    }
    match row.refresh_urgency {
        FantasyGoalieRefreshUrgency::CheckLater => row
            .game_start_utc
            .map(|start| start - chrono::Duration::hours(3)),
        FantasyGoalieRefreshUrgency::RefreshSoon | FantasyGoalieRefreshUrgency::RefreshNow => {
            Some(evaluated_at)
        }
        FantasyGoalieRefreshUrgency::Locked => None,
    }
}

fn row_safety_checkpoint(
    row: &FantasyGoaliePlanRow,
    evaluated_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if row.refresh_urgency == FantasyGoalieRefreshUrgency::Locked {
        return None;
    }
    if row.evidence.requires_refresh {
        return row_refresh_checkpoint(row, evaluated_at);
    }
    row.refresh_deadline_utc
        .map(|deadline| deadline.max(evaluated_at))
}

fn decision_score(
    row: &FantasyGoaliePlanRow,
    strategy: FantasyMatchupStrategy,
    mode: FantasyCompetitionMode,
) -> f64 {
    let evidence = match row.evidence.effective_state {
        FantasyGoalieStartState::ConfirmedStarting => 100.0,
        FantasyGoalieStartState::ReportedStarting => 80.0,
        FantasyGoalieStartState::EstimatedStarting => 60.0,
        FantasyGoalieStartState::Unknown => 40.0,
        FantasyGoalieStartState::ReportedBackup => 10.0,
        FantasyGoalieStartState::ConfirmedBackup => 0.0,
    };
    let objective = match mode {
        FantasyCompetitionMode::Points => match strategy {
            FantasyMatchupStrategy::Floor => row.poor_start_points,
            FantasyMatchupStrategy::Balanced => row.projected_points,
            FantasyMatchupStrategy::Upside => row.projected_points * 1.15,
        },
        FantasyCompetitionMode::Categories => {
            let (save_percentage, goals_against_average) =
                if strategy == FantasyMatchupStrategy::Floor {
                    (
                        row.poor_start_save_percentage,
                        row.poor_start_goals_against_average,
                    )
                } else {
                    (
                        row.expected_save_percentage,
                        row.expected_goals_against_average,
                    )
                };
            save_percentage.unwrap_or(0.0) * 100.0 - goals_against_average.unwrap_or(5.0) * 10.0
                + row.start_probability * 5.0
        }
    };
    let rest_penalty = if row.team_back_to_back { 3.0 } else { 0.0 };
    evidence + objective - rest_penalty
}

fn mean_options(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let values = values.flatten().collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn stream_candidate_score(
    row: &FantasyGoalieStreamCandidateRow,
    strategy: FantasyMatchupStrategy,
    mode: FantasyCompetitionMode,
    minimum_shortfall: f64,
) -> f64 {
    let coverage = if minimum_shortfall > 0.0 {
        row.expected_appearance_gain * 100.0
    } else {
        row.expected_appearance_gain * 10.0
    };
    let value = match mode {
        FantasyCompetitionMode::Points => match strategy {
            FantasyMatchupStrategy::Floor => row.poor_start_points,
            FantasyMatchupStrategy::Balanced => row.projected_points_gain,
            FantasyMatchupStrategy::Upside => row.projected_points_gain * 1.15,
        },
        FantasyCompetitionMode::Categories => {
            let (save_percentage, goals_against_average) =
                if strategy == FantasyMatchupStrategy::Floor {
                    (
                        row.poor_start_save_percentage,
                        row.poor_start_goals_against_average,
                    )
                } else {
                    (
                        row.expected_save_percentage,
                        row.expected_goals_against_average,
                    )
                };
            save_percentage.unwrap_or(0.0) * 100.0 - goals_against_average.unwrap_or(5.0) * 10.0
        }
    };
    coverage + value - row.back_to_back_games as f64 * 2.0
}

fn decide_action(
    state: FantasyGoalieStartState,
    within_capacity: bool,
    strategy: FantasyMatchupStrategy,
    mode: FantasyCompetitionMode,
    urgency: FantasyGoalieRefreshUrgency,
) -> (FantasyGoaliePlanAction, bool, String) {
    if urgency == FantasyGoalieRefreshUrgency::Locked {
        return (
            FantasyGoaliePlanAction::Locked,
            false,
            "game has locked; no lineup or add action remains available".to_owned(),
        );
    }
    if !within_capacity {
        return (
            FantasyGoaliePlanAction::Bench,
            false,
            "daily goalie slots are already occupied by stronger evidence/value".to_owned(),
        );
    }
    match state {
        FantasyGoalieStartState::ConfirmedStarting => (
            FantasyGoaliePlanAction::Start,
            false,
            if mode == FantasyCompetitionMode::Categories
                && strategy == FantasyMatchupStrategy::Floor
            {
                "confirmed starter; ratio downside is disclosed before lock"
            } else {
                "confirmed starter owns an available goalie slot"
            }
            .to_owned(),
        ),
        FantasyGoalieStartState::ReportedStarting => (
            FantasyGoaliePlanAction::Wait,
            true,
            "reported starter is not confirmed; refresh before lock".to_owned(),
        ),
        FantasyGoalieStartState::EstimatedStarting => (
            FantasyGoaliePlanAction::Wait,
            true,
            "estimated start remains probabilistic; refresh before lock".to_owned(),
        ),
        FantasyGoalieStartState::Unknown => (
            FantasyGoaliePlanAction::Refresh,
            true,
            "starter state is unknown or stale".to_owned(),
        ),
        FantasyGoalieStartState::ConfirmedBackup => (
            FantasyGoaliePlanAction::Bench,
            false,
            "confirmed backup is not projected to start".to_owned(),
        ),
        FantasyGoalieStartState::ReportedBackup => (
            FantasyGoaliePlanAction::Bench,
            true,
            "reported backup; retain refresh if the starter changes".to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn goalie(key: &str, dates: Vec<NaiveDate>) -> FantasyGoaliePlanPlayerInput {
        FantasyGoaliePlanPlayerInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: "NYR".to_owned(),
            rostered: true,
            acquisition_eligible: false,
            games: dates
                .into_iter()
                .map(|date| FantasyGoalieGameInput {
                    date,
                    start_time_utc: None,
                    opponent: "BOS".to_owned(),
                    home: true,
                    team_back_to_back: false,
                    opponent_offense_index: 1.0,
                })
                .collect(),
            projected_points_per_start: 5.0,
            historical_start_probability: 0.6,
            expected_save_percentage: Some(0.91),
            expected_goals_against_average: Some(2.7),
        }
    }

    #[test]
    fn stale_confirmed_observation_resolves_unknown_and_requests_refresh() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 9, 18, 0, 0).unwrap();
        let observation = FantasyGoalieStartObservation {
            player_key: "goalie".to_owned(),
            game_date: date,
            state: FantasyGoalieStartState::ConfirmedStarting,
            source: "team reporter".to_owned(),
            source_url: None,
            observed_at: now - chrono::Duration::hours(8),
            fetched_at: now - chrono::Duration::hours(8),
            detail: None,
        };
        let resolved = resolve_fantasy_goalie_start("goalie", date, &[observation], now, 360);
        assert_eq!(
            resolved.reported_state,
            FantasyGoalieStartState::ConfirmedStarting
        );
        assert_eq!(resolved.effective_state, FantasyGoalieStartState::Unknown);
        assert_eq!(resolved.freshness, FantasyObservationFreshness::Stale);
        assert!(resolved.requires_refresh);
    }

    #[test]
    fn plan_preserves_reported_state_and_enforces_daily_goalie_capacity() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 9, 18, 0, 0).unwrap();
        let observation = FantasyGoalieStartObservation {
            player_key: "reported".to_owned(),
            game_date: date,
            state: FantasyGoalieStartState::ReportedStarting,
            source: "beat reporter".to_owned(),
            source_url: None,
            observed_at: now - chrono::Duration::minutes(10),
            fetched_at: now - chrono::Duration::minutes(9),
            detail: None,
        };
        let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: None,
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Categories,
            goalie_slots: 1,
            minimum_goalie_appearances: 1,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 4,
            goalies: vec![
                goalie("reported", vec![date]),
                goalie("unknown", vec![date]),
            ],
            observations: vec![observation],
            warnings: Vec::new(),
        })
        .unwrap();
        let reported = view
            .rows
            .iter()
            .find(|row| row.player_key == "reported")
            .unwrap();
        let unknown = view
            .rows
            .iter()
            .find(|row| row.player_key == "unknown")
            .unwrap();
        assert_eq!(
            reported.evidence.effective_state,
            FantasyGoalieStartState::ReportedStarting
        );
        assert_eq!(reported.action, FantasyGoaliePlanAction::Wait);
        assert!(reported.conditional);
        assert!(reported.occupies_goalie_slot);
        assert_eq!(unknown.action, FantasyGoaliePlanAction::Bench);
        assert!(!unknown.occupies_goalie_slot);
    }

    #[test]
    fn minimum_risk_uses_confirmed_floor_separately_from_expected_starts() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 9, 12, 0, 0).unwrap();
        let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: None,
            strategy: FantasyMatchupStrategy::Floor,
            competition_mode: FantasyCompetitionMode::Categories,
            goalie_slots: 2,
            minimum_goalie_appearances: 2,
            current_goalie_appearances: 1.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 4,
            goalies: vec![goalie("one", vec![date]), goalie("two", vec![date])],
            observations: Vec::new(),
            warnings: Vec::new(),
        })
        .unwrap();
        assert!(view.expected_total_appearances >= 2.0);
        assert_eq!(view.confirmed_floor_total_appearances, 1.0);
        assert!(view.minimum_at_risk);
    }

    #[test]
    fn back_to_back_is_probabilistic_until_confirmation() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 16, 0, 0).unwrap();
        let mut input_goalie = goalie("one", vec![date]);
        input_goalie.games[0].team_back_to_back = true;
        let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: None,
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Points,
            goalie_slots: 2,
            minimum_goalie_appearances: 0,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 4,
            goalies: vec![input_goalie],
            observations: Vec::new(),
            warnings: Vec::new(),
        })
        .unwrap();
        assert!((view.rows[0].start_probability - 0.39).abs() < 0.001);
        assert!(view.rows[0].team_back_to_back);
    }

    #[test]
    fn locked_goalie_game_cannot_create_a_remaining_start_or_stream() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 22, 0, 0).unwrap();
        let mut input_goalie = goalie("one", vec![date]);
        input_goalie.games[0].start_time_utc = Some(now - chrono::Duration::minutes(1));
        let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: Some(date),
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Points,
            goalie_slots: 2,
            minimum_goalie_appearances: 0,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 4,
            goalies: vec![input_goalie],
            observations: Vec::new(),
            warnings: Vec::new(),
        })
        .unwrap();
        assert_eq!(view.rows[0].action, FantasyGoaliePlanAction::Locked);
        assert_eq!(
            view.rows[0].refresh_urgency,
            FantasyGoalieRefreshUrgency::Locked
        );
        assert!(!view.rows[0].occupies_goalie_slot);
        assert_eq!(view.expected_remaining_appearances, 0.0);
    }

    #[test]
    fn legal_stream_quantifies_third_goalie_minimum_gain() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 16, 0, 0).unwrap();
        let mut stream = goalie("stream", vec![date]);
        stream.rostered = false;
        stream.acquisition_eligible = true;
        let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: None,
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Categories,
            goalie_slots: 2,
            minimum_goalie_appearances: 1,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 1,
            goalies: vec![stream],
            observations: Vec::new(),
            warnings: Vec::new(),
        })
        .unwrap();
        assert_eq!(view.rows.len(), 0);
        assert_eq!(view.stream_candidates.len(), 1);
        assert_eq!(view.portfolio.best_third_goalie.as_deref(), Some("stream"));
        assert!((view.portfolio.expected_appearance_gain - 0.6).abs() < 0.001);
        assert!(view.portfolio.three_goalie_minimum_shortfall < 1.0);
    }

    #[test]
    fn category_floor_stream_ranks_ratio_quality_not_point_total() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 16, 0, 0).unwrap();
        let mut safe = goalie("safe", vec![date]);
        safe.rostered = false;
        safe.acquisition_eligible = true;
        safe.projected_points_per_start = 2.0;
        safe.expected_save_percentage = Some(0.93);
        safe.expected_goals_against_average = Some(2.0);
        let mut points_only = goalie("points", vec![date]);
        points_only.rostered = false;
        points_only.acquisition_eligible = true;
        points_only.projected_points_per_start = 12.0;
        points_only.expected_save_percentage = Some(0.88);
        points_only.expected_goals_against_average = Some(4.0);
        let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: None,
            strategy: FantasyMatchupStrategy::Floor,
            competition_mode: FantasyCompetitionMode::Categories,
            goalie_slots: 2,
            minimum_goalie_appearances: 0,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 1,
            goalies: vec![points_only, safe],
            observations: Vec::new(),
            warnings: Vec::new(),
        })
        .unwrap();
        assert_eq!(view.portfolio.best_third_goalie.as_deref(), Some("safe"));
    }

    #[test]
    fn latest_backup_report_reverses_an_earlier_confirmed_start() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 18, 0, 0).unwrap();
        let observation = |state, minutes_ago| FantasyGoalieStartObservation {
            player_key: "one".to_owned(),
            game_date: date,
            state,
            source: "team reporter".to_owned(),
            source_url: None,
            observed_at: now - chrono::Duration::minutes(minutes_ago),
            fetched_at: now - chrono::Duration::minutes(minutes_ago),
            detail: None,
        };
        let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: Some(date),
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Points,
            goalie_slots: 1,
            minimum_goalie_appearances: 1,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 1,
            goalies: vec![goalie("one", vec![date])],
            observations: vec![
                observation(FantasyGoalieStartState::ConfirmedStarting, 30),
                observation(FantasyGoalieStartState::ConfirmedBackup, 5),
            ],
            warnings: Vec::new(),
        })
        .unwrap();

        assert_eq!(
            view.rows[0].evidence.effective_state,
            FantasyGoalieStartState::ConfirmedBackup
        );
        assert_eq!(view.rows[0].action, FantasyGoaliePlanAction::Bench);
        assert_eq!(view.confirmed_remaining_appearances, 0);
        assert_eq!(view.expected_remaining_appearances, 0.0);
    }

    #[test]
    fn plan_exposes_exact_refresh_and_lock_checkpoints() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let start = Utc.with_ymd_and_hms(2026, 11, 11, 1, 0, 0).unwrap();
        let now = start - chrono::Duration::hours(5);
        let mut rostered = goalie("rostered", vec![date]);
        rostered.games[0].start_time_utc = Some(start);
        let mut stream = goalie("stream", vec![date]);
        stream.rostered = false;
        stream.acquisition_eligible = true;
        stream.games[0].start_time_utc = Some(start);
        let input = FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: Some(date),
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Points,
            goalie_slots: 1,
            minimum_goalie_appearances: 1,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 1,
            goalies: vec![rostered, stream],
            observations: Vec::new(),
            warnings: Vec::new(),
        };
        let early = build_fantasy_goalie_plan(input.clone()).unwrap();
        assert_eq!(
            early.next_required_refresh_utc,
            Some(start - chrono::Duration::hours(3))
        );
        assert_eq!(early.next_game_lock_utc, Some(start));
        assert_eq!(early.refreshes_due_now, 0);
        assert_eq!(early.unresolved_rostered_goalies_on_focus_date, 1);

        let urgent = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            evaluated_at: start - chrono::Duration::hours(2),
            ..input
        })
        .unwrap();
        assert_eq!(
            urgent.next_required_refresh_utc,
            Some(start - chrono::Duration::hours(2))
        );
        assert_eq!(urgent.refreshes_due_now, 2);
    }

    #[test]
    fn confirmed_starter_and_backup_receive_a_final_safety_check() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let start = Utc.with_ymd_and_hms(2026, 11, 11, 1, 0, 0).unwrap();
        let now = start - chrono::Duration::hours(5);
        let mut starter = goalie("starter", vec![date]);
        starter.games[0].start_time_utc = Some(start);
        let mut backup = goalie("backup", vec![date]);
        backup.games[0].start_time_utc = Some(start);
        let observation = |player_key: &str, state| FantasyGoalieStartObservation {
            player_key: player_key.to_owned(),
            game_date: date,
            state,
            source: "team reporter".to_owned(),
            source_url: None,
            observed_at: now,
            fetched_at: now,
            detail: None,
        };
        let input = FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: date,
            focus_date: Some(date),
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Points,
            goalie_slots: 2,
            minimum_goalie_appearances: 1,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 1,
            goalies: vec![starter, backup],
            observations: vec![
                observation("starter", FantasyGoalieStartState::ConfirmedStarting),
                observation("backup", FantasyGoalieStartState::ConfirmedBackup),
            ],
            warnings: Vec::new(),
        };
        let early = build_fantasy_goalie_plan(input.clone()).unwrap();
        assert_eq!(early.next_required_refresh_utc, None);
        assert_eq!(
            early.next_safety_check_utc,
            Some(start - chrono::Duration::minutes(30))
        );
        assert_eq!(early.safety_checks_due_now, 0);

        let late = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            evaluated_at: start - chrono::Duration::minutes(20),
            ..input
        })
        .unwrap();
        assert_eq!(
            late.next_safety_check_utc,
            Some(start - chrono::Duration::minutes(20))
        );
        assert_eq!(late.safety_checks_due_now, 2);
        assert_eq!(
            late.rows
                .iter()
                .find(|row| row.player_key == "starter")
                .unwrap()
                .action,
            FantasyGoaliePlanAction::Start
        );
        assert_eq!(
            late.rows
                .iter()
                .find(|row| row.player_key == "backup")
                .unwrap()
                .action,
            FantasyGoaliePlanAction::Bench
        );
    }
}
