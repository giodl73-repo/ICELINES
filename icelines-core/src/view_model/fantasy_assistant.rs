use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::name::normalize_name;

use super::fantasy_goalie_plan::{
    FantasyGoaliePlanAction, FantasyGoaliePlanView, FantasyGoalieRefreshUrgency,
    FantasyGoalieStartState,
};
use crate::model::Position;

pub const FANTASY_ASSISTANT_RULES_SCHEMA: &str = "fantasy_assistant_rules.v1";
pub const FANTASY_DAILY_LINEUP_SCHEMA: &str = "fantasy_daily_lineup.v1";
pub const FANTASY_TAKEN_IMPORT_SCHEMA: &str = "fantasy_taken_import.v1";
pub const FANTASY_DRAFT_BOARD_SCHEMA: &str = "fantasy_draft_board.v1";
pub const FANTASY_ELIGIBILITY_IMPORT_SCHEMA: &str = "fantasy_eligibility_import.v1";
pub const FANTASY_WEEK_BUDGET_SCHEMA: &str = "fantasy_week_budget.v1";
pub const FANTASY_WEEKLY_PICKUP_SCHEMA: &str = "fantasy_weekly_pickup.v1";
pub const FANTASY_INJURY_PLAN_SCHEMA: &str = "fantasy_injury_plan.v1";
pub const FANTASY_MORNING_BRIEFING_SCHEMA: &str = "fantasy_morning_briefing.v3";
const EXCEPTIONAL_RESERVE_MIN_VALUE: f64 = 6.0;
const EXCEPTIONAL_RESERVE_MIN_STARTS: f64 = 3.0;
pub const FANTASY_SLEEPER_BOARD_SCHEMA: &str = "fantasy_sleeper_board.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySleeperInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    pub current_gp: u32,
    pub current_fantasy_per_game: f64,
    pub prior_gp: u32,
    pub prior_player_existed: bool,
    pub prior_rate_available: bool,
    pub prior_fantasy_per_game: f64,
    pub current_shots_per_game: f64,
    pub prior_shots_per_game: f64,
    pub current_hits_per_game: Option<f64>,
    pub prior_hits_per_game: Option<f64>,
    pub current_blocks_per_game: Option<f64>,
    pub prior_blocks_per_game: Option<f64>,
    pub current_pp_points_per_game: f64,
    pub prior_pp_points_per_game: f64,
    /// Fraction of the team's games on quiet slates, from 0.0 through 1.0.
    pub quiet_slate_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasySleeperConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySleeperComponents {
    pub league_scoring_growth: f64,
    pub category_rate_growth: f64,
    pub power_play_growth: f64,
    pub quiet_slate_value: f64,
    pub position_flexibility: f64,
    pub newcomer_opportunity: f64,
    pub sample_risk_discount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySleeperRow {
    pub rank: usize,
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    pub current_gp: u32,
    pub current_fantasy_per_game: f64,
    pub prior_fantasy_per_game: f64,
    pub score: f64,
    pub confidence: FantasySleeperConfidence,
    pub components: FantasySleeperComponents,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySleeperBoardView {
    pub schema: String,
    pub scoring_scheme: String,
    pub stats_season: String,
    pub baseline_season: String,
    pub rows: Vec<FantasySleeperRow>,
    pub warnings: Vec<String>,
}

pub fn build_fantasy_sleeper_board(
    scoring_scheme: impl Into<String>,
    stats_season: impl Into<String>,
    baseline_season: impl Into<String>,
    inputs: Vec<FantasySleeperInput>,
    top: usize,
) -> FantasySleeperBoardView {
    let baseline_source_gaps = inputs
        .iter()
        .filter(|input| input.prior_player_existed && !input.prior_rate_available)
        .count();
    let mut rows = inputs
        .into_iter()
        .filter(|input| input.current_gp >= 10)
        .map(score_sleeper)
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| {
                b.current_fantasy_per_game
                    .total_cmp(&a.current_fantasy_per_game)
            })
            .then_with(|| a.player_key.cmp(&b.player_key))
    });
    rows.truncate(top);
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    let mut warnings = vec![
        "Sleeper scores are discovery signals, not rest-of-season projections; verify deployment and injury evidence before adding"
            .to_owned(),
        "Goalie sleeper scoring is deferred; this board evaluates skaters only".to_owned(),
    ];
    if baseline_source_gaps > 0 {
        warnings.push(format!(
            "{baseline_source_gaps} candidate(s) existed in the baseline stats source but lacked a complete identity/rate join; no growth or newcomer credit was assigned"
        ));
    }
    FantasySleeperBoardView {
        schema: FANTASY_SLEEPER_BOARD_SCHEMA.to_owned(),
        scoring_scheme: scoring_scheme.into(),
        stats_season: stats_season.into(),
        baseline_season: baseline_season.into(),
        rows,
        warnings,
    }
}

fn score_sleeper(input: FantasySleeperInput) -> FantasySleeperRow {
    let league_scoring_growth = if input.prior_rate_available && input.prior_gp >= 10 {
        ((input.current_fantasy_per_game - input.prior_fantasy_per_game).max(0.0) * 8.0)
            .clamp(0.0, 35.0)
    } else if !input.prior_player_existed {
        (input.current_fantasy_per_game.max(0.0) * 4.0).clamp(0.0, 20.0)
    } else {
        0.0
    };
    let rate_delta = |current: Option<f64>, prior: Option<f64>, weight: f64| match (current, prior)
    {
        (Some(current), Some(prior)) => (current - prior).max(0.0) * weight,
        _ => 0.0,
    };
    let category_rate_growth = if input.prior_rate_available {
        (((input.current_shots_per_game - input.prior_shots_per_game).max(0.0) * 4.0)
            + rate_delta(input.current_hits_per_game, input.prior_hits_per_game, 2.0)
            + rate_delta(
                input.current_blocks_per_game,
                input.prior_blocks_per_game,
                3.0,
            ))
        .clamp(0.0, 20.0)
    } else {
        0.0
    };
    let power_play_growth = if input.prior_rate_available {
        ((input.current_pp_points_per_game - input.prior_pp_points_per_game).max(0.0) * 30.0)
            .clamp(0.0, 15.0)
    } else {
        0.0
    };
    let quiet_slate_value = input.quiet_slate_rate.clamp(0.0, 1.0) * 15.0;
    let position_flexibility =
        (input.platform_positions.len().saturating_sub(1) as f64 * 2.5).clamp(0.0, 5.0);
    let newcomer_opportunity = if !input.prior_player_existed {
        10.0
    } else {
        0.0
    };
    let sample_risk_discount = if input.current_gp < 20 {
        (f64::from(20 - input.current_gp) / 20.0) * 20.0
    } else {
        0.0
    };
    let score = (league_scoring_growth
        + category_rate_growth
        + power_play_growth
        + quiet_slate_value
        + position_flexibility
        + newcomer_opportunity
        - sample_risk_discount)
        .clamp(0.0, 100.0);
    let confidence = match input.current_gp {
        60.. => FantasySleeperConfidence::High,
        30.. => FantasySleeperConfidence::Medium,
        _ => FantasySleeperConfidence::Low,
    };
    let mut reasons = Vec::new();
    if league_scoring_growth > 0.0 {
        reasons.push(format!(
            "league-scored rate rose from {:.2} to {:.2} per game",
            input.prior_fantasy_per_game, input.current_fantasy_per_game
        ));
    }
    if category_rate_growth > 0.0 {
        reasons.push("shots/hits/blocks rate growth".to_owned());
    }
    if power_play_growth > 0.0 {
        reasons.push("power-play production rate growth".to_owned());
    }
    if newcomer_opportunity > 0.0 {
        reasons.push("newcomer or prior-season small sample".to_owned());
    }
    if quiet_slate_value > 0.0 {
        reasons.push(format!(
            "{:.0}% of team games land on quiet slates",
            input.quiet_slate_rate.clamp(0.0, 1.0) * 100.0
        ));
    }
    if sample_risk_discount > 0.0 {
        reasons.push(format!(
            "small-sample discount: {:.1}",
            sample_risk_discount
        ));
    }
    FantasySleeperRow {
        rank: 0,
        player_key: input.player_key,
        player: input.player,
        nhl_team: input.nhl_team,
        platform_positions: input.platform_positions,
        current_gp: input.current_gp,
        current_fantasy_per_game: input.current_fantasy_per_game,
        prior_fantasy_per_game: input.prior_fantasy_per_game,
        score,
        confidence,
        components: FantasySleeperComponents {
            league_scoring_growth,
            category_rate_growth,
            power_play_growth,
            quiet_slate_value,
            position_flexibility,
            newcomer_opportunity,
            sample_risk_discount,
        },
        reasons,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyAcquisitionKind {
    FreeAgentAdd,
    WaiverClaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyAcquisitionInput {
    pub effective_at: DateTime<Utc>,
    pub kind: FantasyAcquisitionKind,
    pub counts_toward_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyWeekBudgetView {
    pub schema: String,
    pub timezone: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub acquisition_limit: u8,
    pub acquisitions_used: u8,
    pub acquisitions_remaining: u8,
    pub can_add: bool,
    #[serde(default)]
    pub injury_reserve: u8,
    #[serde(default)]
    pub injury_reserve_active: u8,
    #[serde(default)]
    pub proactive_acquisitions_remaining: u8,
    #[serde(default)]
    pub can_proactively_add: bool,
    #[serde(default)]
    pub injury_reserve_releases_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyWaiverWindow {
    pub player_key: String,
    pub dropped_at: DateTime<Utc>,
    pub clears_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyMarketStatus {
    FreeAgent,
    Waivers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyAcquisitionAvailability {
    pub player_key: String,
    pub status: FantasyMarketStatus,
    pub usable_at: DateTime<Utc>,
    pub usable_now: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyWeeklyMoveInput {
    pub add_player_key: String,
    pub add_player: String,
    pub drop_player_key: String,
    pub drop_player: String,
    pub availability: FantasyAcquisitionAvailability,
    pub incremental_usable_starts: f64,
    pub projected_points_from_incremental_starts: f64,
    pub category_gap_delta: f64,
    pub future_schedule_option_value: f64,
    pub dropped_player_rest_of_week_value: f64,
    pub waiver_reacquisition_cost: f64,
    pub pickup_budget_cost: f64,
    pub uncertainty_discount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyWeeklyMoveRow {
    pub rank: usize,
    pub add_player_key: String,
    pub add_player: String,
    pub drop_player_key: String,
    pub drop_player: String,
    pub incremental_usable_starts: f64,
    pub projected_value_delta: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyWeeklyPickupView {
    pub schema: String,
    pub budget: FantasyWeekBudgetView,
    pub rows: Vec<FantasyWeeklyMoveRow>,
    pub blocked_waiver_candidates: usize,
    pub warnings: Vec<String>,
}

pub fn build_fantasy_weekly_pickups(
    budget: FantasyWeekBudgetView,
    moves: Vec<FantasyWeeklyMoveInput>,
    top: usize,
) -> Result<FantasyWeeklyPickupView, String> {
    build_fantasy_weekly_pickups_with_reserve_override(budget, moves, top, false, true)
}

pub fn build_fantasy_weekly_pickups_with_reserve_override(
    budget: FantasyWeekBudgetView,
    moves: Vec<FantasyWeeklyMoveInput>,
    top: usize,
    allow_injury_reserve: bool,
    allow_exceptional_override: bool,
) -> Result<FantasyWeeklyPickupView, String> {
    if !budget.can_add {
        return Ok(FantasyWeeklyPickupView {
            schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
            budget,
            rows: Vec::new(),
            blocked_waiver_candidates: moves
                .iter()
                .filter(|candidate| !candidate.availability.usable_now)
                .count(),
            warnings: vec!["weekly acquisition limit reached; no add is legal".to_owned()],
        });
    }
    let reserve_only = !allow_injury_reserve && !budget.can_proactively_add;
    let blocked_waiver_candidates = moves
        .iter()
        .filter(|candidate| !candidate.availability.usable_now)
        .count();
    let mut rows = moves
        .into_iter()
        .filter(|candidate| candidate.availability.usable_now)
        .map(rank_weekly_move)
        .collect::<Result<Vec<_>, _>>()?;
    rows.sort_by(|a, b| {
        b.projected_value_delta
            .total_cmp(&a.projected_value_delta)
            .then_with(|| {
                b.incremental_usable_starts
                    .total_cmp(&a.incremental_usable_starts)
            })
            .then_with(|| a.add_player_key.cmp(&b.add_player_key))
            .then_with(|| a.drop_player_key.cmp(&b.drop_player_key))
    });
    let exceptional_override = reserve_only
        && allow_exceptional_override
        && rows.first().is_some_and(exceptional_reserve_move);
    if reserve_only && !exceptional_override {
        return Ok(FantasyWeeklyPickupView {
            schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
            budget,
            rows: Vec::new(),
            blocked_waiver_candidates,
            warnings: vec![
                "the remaining acquisition budget is reserved for an injury replacement".to_owned(),
            ],
        });
    }
    if exceptional_override {
        rows.retain(exceptional_reserve_move);
        for row in &mut rows {
            row.reasons.push(format!(
                "exceptional reserve override: at least {EXCEPTIONAL_RESERVE_MIN_STARTS:.1} starts and {EXCEPTIONAL_RESERVE_MIN_VALUE:.1} net value"
            ));
        }
    }
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    rows.truncate(top.max(1));
    let mut warnings = Vec::new();
    if exceptional_override {
        warnings.push(
            "exceptional healthy-roster value justifies reviewing the protected final acquisition"
                .to_owned(),
        );
    }
    if blocked_waiver_candidates > 0 {
        warnings.push(format!(
            "{blocked_waiver_candidates} add/drop candidate(s) are blocked by waivers"
        ));
    }
    if rows
        .first()
        .is_some_and(|row| row.projected_value_delta <= 0.0)
    {
        warnings.push("no evaluated move has a positive projected value delta".to_owned());
    }
    Ok(FantasyWeeklyPickupView {
        schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
        budget,
        rows,
        blocked_waiver_candidates,
        warnings,
    })
}

fn exceptional_reserve_move(row: &FantasyWeeklyMoveRow) -> bool {
    row.projected_value_delta >= EXCEPTIONAL_RESERVE_MIN_VALUE
        && row.incremental_usable_starts >= EXCEPTIONAL_RESERVE_MIN_STARTS
}

fn rank_weekly_move(candidate: FantasyWeeklyMoveInput) -> Result<FantasyWeeklyMoveRow, String> {
    for (label, value) in [
        (
            "incremental_usable_starts",
            candidate.incremental_usable_starts,
        ),
        (
            "projected_points_from_incremental_starts",
            candidate.projected_points_from_incremental_starts,
        ),
        ("category_gap_delta", candidate.category_gap_delta),
        (
            "future_schedule_option_value",
            candidate.future_schedule_option_value,
        ),
        (
            "dropped_player_rest_of_week_value",
            candidate.dropped_player_rest_of_week_value,
        ),
        (
            "waiver_reacquisition_cost",
            candidate.waiver_reacquisition_cost,
        ),
        ("pickup_budget_cost", candidate.pickup_budget_cost),
        ("uncertainty_discount", candidate.uncertainty_discount),
    ] {
        if !value.is_finite() {
            return Err(format!("{} has non-finite {label}", candidate.add_player));
        }
    }
    let projected_value_delta = candidate.projected_points_from_incremental_starts
        + candidate.category_gap_delta
        + candidate.future_schedule_option_value
        - candidate.dropped_player_rest_of_week_value
        - candidate.waiver_reacquisition_cost
        - candidate.pickup_budget_cost
        - candidate.uncertainty_discount;
    let mut reasons = vec![
        format!(
            "{:.1} incremental usable starts",
            candidate.incremental_usable_starts
        ),
        format!(
            "{:.1} projected points from those starts",
            candidate.projected_points_from_incremental_starts
        ),
        format!(
            "{:.1} rest-of-week value surrendered",
            candidate.dropped_player_rest_of_week_value
        ),
    ];
    if candidate.future_schedule_option_value != 0.0 {
        reasons.push(format!(
            "{:+.1} saved-playoff-calendar retention value",
            candidate.future_schedule_option_value
        ));
    }
    Ok(FantasyWeeklyMoveRow {
        rank: 0,
        add_player_key: candidate.add_player_key,
        add_player: candidate.add_player,
        drop_player_key: candidate.drop_player_key,
        drop_player: candidate.drop_player,
        incremental_usable_starts: candidate.incremental_usable_starts,
        projected_value_delta,
        reasons,
    })
}

pub fn build_fantasy_week_budget(
    now: DateTime<Utc>,
    timezone: &str,
    acquisition_limit: u8,
    acquisitions: &[FantasyAcquisitionInput],
) -> Result<FantasyWeekBudgetView, String> {
    let timezone_parsed = timezone
        .parse::<Tz>()
        .map_err(|_| format!("unsupported IANA timezone '{timezone}'"))?;
    let local_date = now.with_timezone(&timezone_parsed).date_naive();
    let week_start =
        local_date - Duration::days(local_date.weekday().num_days_from_monday() as i64);
    let week_end = week_start + Duration::days(6);
    let used = acquisitions
        .iter()
        .filter(|event| {
            let date = event
                .effective_at
                .with_timezone(&timezone_parsed)
                .date_naive();
            event.counts_toward_limit && date >= week_start && date <= week_end
        })
        .count()
        .min(u8::MAX as usize) as u8;
    let remaining = acquisition_limit.saturating_sub(used);
    Ok(FantasyWeekBudgetView {
        schema: FANTASY_WEEK_BUDGET_SCHEMA.to_owned(),
        timezone: timezone.to_owned(),
        week_start,
        week_end,
        acquisition_limit,
        acquisitions_used: used,
        acquisitions_remaining: remaining,
        can_add: remaining > 0,
        injury_reserve: 0,
        injury_reserve_active: 0,
        proactive_acquisitions_remaining: remaining,
        can_proactively_add: remaining > 0,
        injury_reserve_releases_on: None,
    })
}

pub fn apply_fantasy_pickup_reserve(
    mut budget: FantasyWeekBudgetView,
    evaluation_date: NaiveDate,
    injury_reserve: u8,
    release_weekday_from_monday: u8,
) -> Result<FantasyWeekBudgetView, String> {
    if !(budget.week_start..=budget.week_end).contains(&evaluation_date) {
        return Err(format!(
            "evaluation date {evaluation_date} is outside budget week {} through {}",
            budget.week_start, budget.week_end
        ));
    }
    if release_weekday_from_monday > 6 {
        return Err("injury reserve release weekday must be between 0 and 6".to_owned());
    }
    let reserve = injury_reserve.min(budget.acquisition_limit);
    let releases_on = budget.week_start + Duration::days(i64::from(release_weekday_from_monday));
    let active = if evaluation_date < releases_on {
        reserve.min(budget.acquisitions_remaining)
    } else {
        0
    };
    let proactive_remaining = budget.acquisitions_remaining.saturating_sub(active);
    budget.injury_reserve = reserve;
    budget.injury_reserve_active = active;
    budget.proactive_acquisitions_remaining = proactive_remaining;
    budget.can_proactively_add = proactive_remaining > 0;
    budget.injury_reserve_releases_on = Some(releases_on);
    Ok(budget)
}

pub fn fantasy_waiver_window(
    player_key: impl Into<String>,
    dropped_at: DateTime<Utc>,
    waiver_days: u8,
) -> FantasyWaiverWindow {
    FantasyWaiverWindow {
        player_key: player_key.into(),
        dropped_at,
        clears_at: dropped_at + Duration::days(i64::from(waiver_days)),
    }
}

pub fn fantasy_acquisition_availability(
    player_key: impl Into<String>,
    now: DateTime<Utc>,
    waiver: Option<&FantasyWaiverWindow>,
) -> FantasyAcquisitionAvailability {
    let player_key = player_key.into();
    match waiver.filter(|waiver| waiver.clears_at > now) {
        Some(waiver) => FantasyAcquisitionAvailability {
            player_key,
            status: FantasyMarketStatus::Waivers,
            usable_at: waiver.clears_at,
            usable_now: false,
        },
        None => FantasyAcquisitionAvailability {
            player_key,
            status: FantasyMarketStatus::FreeAgent,
            usable_at: now,
            usable_now: true,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyDraftIdentityInput {
    pub player_key: String,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyTakenResolutionStatus {
    Matched,
    Duplicate,
    Ambiguous,
    Unresolved,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyTakenPlayerRow {
    pub row_number: u32,
    pub supplied_name: String,
    pub normalized_name: Option<String>,
    pub matched_player_key: Option<String>,
    pub matched_player: Option<String>,
    pub status: FantasyTakenResolutionStatus,
    pub candidates: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyTakenImportView {
    pub schema: String,
    pub rows: Vec<FantasyTakenPlayerRow>,
    pub matched_player_keys: Vec<String>,
    pub matched: usize,
    pub duplicates: usize,
    pub ambiguous: usize,
    pub unresolved: usize,
    pub empty: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyEligibilityImportStatus {
    Imported,
    Duplicate,
    Ambiguous,
    Unresolved,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyEligibilityImportRow {
    pub row_number: u32,
    pub supplied_name: String,
    pub normalized_name: Option<String>,
    pub matched_player_key: Option<String>,
    pub positions: Vec<Position>,
    pub status: FantasyEligibilityImportStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyEligibilityImportView {
    pub schema: String,
    pub rows: Vec<FantasyEligibilityImportRow>,
    pub imported: usize,
    pub duplicates: usize,
    pub ambiguous: usize,
    pub unresolved: usize,
    pub invalid: usize,
}

pub fn import_fantasy_platform_eligibility(
    input: &str,
    identities: &[FantasyDraftIdentityInput],
) -> Result<FantasyEligibilityImportView, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(Cursor::new(input));
    let headers = reader
        .headers()
        .map_err(|error| format!("invalid eligibility CSV header: {error}"))?
        .clone();
    let name_column = headers
        .iter()
        .position(is_player_name_header)
        .ok_or_else(|| {
            "eligibility CSV requires Player, Player Name, Name, or Full Name".to_owned()
        })?;
    let position_column = headers
        .iter()
        .position(is_eligibility_header)
        .ok_or_else(|| {
            "eligibility CSV requires Position, Positions, Eligible Positions, or Pos".to_owned()
        })?;
    let mut identity_index = BTreeMap::<String, BTreeSet<usize>>::new();
    for (index, identity) in identities.iter().enumerate() {
        for name in std::iter::once(&identity.display_name).chain(identity.aliases.iter()) {
            let normalized = normalize_name(name);
            if !normalized.is_empty() {
                identity_index.entry(normalized).or_default().insert(index);
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|error| format!("invalid eligibility CSV: {error}"))?;
        let supplied_name = record.get(name_column).unwrap_or("").trim().to_owned();
        let normalized = normalize_name(&supplied_name);
        let positions_text = record.get(position_column).unwrap_or("");
        let parsed_positions = parse_platform_positions(positions_text);
        let (status, matched_player_key, positions, message) = if normalized.is_empty() {
            (
                FantasyEligibilityImportStatus::Invalid,
                None,
                Vec::new(),
                Some("player name is empty".to_owned()),
            )
        } else if !seen.insert(normalized.clone()) {
            (
                FantasyEligibilityImportStatus::Duplicate,
                None,
                parsed_positions.unwrap_or_default(),
                Some("duplicate player eligibility row".to_owned()),
            )
        } else if let Err(message) = parsed_positions {
            (
                FantasyEligibilityImportStatus::Invalid,
                None,
                Vec::new(),
                Some(message),
            )
        } else {
            let positions = parsed_positions.expect("validated positions");
            match identity_index.get(&normalized) {
                Some(matches) if matches.len() == 1 => {
                    let identity = &identities[*matches.iter().next().expect("one match")];
                    (
                        FantasyEligibilityImportStatus::Imported,
                        Some(identity.player_key.clone()),
                        positions,
                        None,
                    )
                }
                Some(_) => (
                    FantasyEligibilityImportStatus::Ambiguous,
                    None,
                    positions,
                    Some("name matches multiple canonical players".to_owned()),
                ),
                None => (
                    FantasyEligibilityImportStatus::Unresolved,
                    None,
                    positions,
                    Some("name did not match the current player pool".to_owned()),
                ),
            }
        };
        rows.push(FantasyEligibilityImportRow {
            row_number: (index + 2) as u32,
            supplied_name,
            normalized_name: (!normalized.is_empty()).then_some(normalized),
            matched_player_key,
            positions,
            status,
            message,
        });
    }
    Ok(FantasyEligibilityImportView {
        schema: FANTASY_ELIGIBILITY_IMPORT_SCHEMA.to_owned(),
        imported: count_eligibility_status(&rows, FantasyEligibilityImportStatus::Imported),
        duplicates: count_eligibility_status(&rows, FantasyEligibilityImportStatus::Duplicate),
        ambiguous: count_eligibility_status(&rows, FantasyEligibilityImportStatus::Ambiguous),
        unresolved: count_eligibility_status(&rows, FantasyEligibilityImportStatus::Unresolved),
        invalid: count_eligibility_status(&rows, FantasyEligibilityImportStatus::Invalid),
        rows,
    })
}

fn count_eligibility_status(
    rows: &[FantasyEligibilityImportRow],
    status: FantasyEligibilityImportStatus,
) -> usize {
    rows.iter().filter(|row| row.status == status).count()
}

fn is_eligibility_header(value: &str) -> bool {
    matches!(
        normalize_name(value).as_str(),
        "position" | "positions" | "eligible positions" | "position eligibility" | "pos"
    )
}

fn parse_platform_positions(value: &str) -> Result<Vec<Position>, String> {
    let mut positions = Vec::new();
    for token in value
        .to_uppercase()
        .split(|character: char| {
            character == ',' || character == '/' || character == ';' || character.is_whitespace()
        })
        .filter(|token| !token.is_empty())
    {
        let position = match token {
            "C" => Position::Center,
            "LW" | "L" => Position::LeftWing,
            "RW" | "R" => Position::RightWing,
            "D" => Position::Defense,
            "G" => Position::Goalie,
            other => return Err(format!("unsupported platform position '{other}'")),
        };
        if !positions.contains(&position) {
            positions.push(position);
        }
    }
    if positions.is_empty() {
        return Err("platform eligibility is empty".to_owned());
    }
    Ok(positions)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDraftCandidateInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    /// Full-season player value under the active fantasy scoring scheme.
    pub league_scored_quality: f64,
    /// Replacement-level value for this player's scarcest eligible position.
    pub replacement_level: f64,
    /// Starts newly usable after daily slot optimization, not raw scheduled games.
    pub incremental_usable_starts: f64,
    pub quiet_slate_games: f64,
    /// Exact-date collision rate against the current roster, from 0 through 1.
    pub schedule_collision_rate: f64,
    /// Legal active-start change over the configured fantasy playoff window.
    #[serde(default)]
    pub playoff_incremental_usable_starts: f64,
    /// Legal active-lineup value change over the configured playoff window.
    #[serde(default)]
    pub playoff_usable_value_delta: f64,
    /// Already-scaled deduction for injury, role, or evidence uncertainty.
    pub risk_penalty: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDraftValueComponents {
    pub league_scored_quality: f64,
    pub starter_gap_value: f64,
    pub positional_scarcity: f64,
    pub multi_position_flexibility: f64,
    pub incremental_usable_starts: f64,
    pub quiet_slate_value: f64,
    pub schedule_diversity: f64,
    pub collision_cost: f64,
    pub playoff_fit_value: f64,
    pub risk_penalty: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDraftCandidateRow {
    pub rank: usize,
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    pub draft_value: f64,
    pub fills_open_starter: bool,
    pub best_open_slot: Option<FantasyActiveSlotKind>,
    pub components: FantasyDraftValueComponents,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDraftPositionLeader {
    pub slot_kind: FantasyActiveSlotKind,
    pub player_key: String,
    pub player: String,
    pub draft_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDraftBoardView {
    pub schema: String,
    pub scoring_scheme: String,
    pub scoring_season: String,
    pub open_slots: Vec<FantasyActiveSlot>,
    pub available_players: usize,
    pub excluded_taken_players: usize,
    pub rows: Vec<FantasyDraftCandidateRow>,
    pub position_leaders: Vec<FantasyDraftPositionLeader>,
    pub fallback_pick: Option<FantasyDraftCandidateRow>,
    pub taken_import: FantasyTakenImportView,
    pub eligibility_import: Option<FantasyEligibilityImportView>,
    pub warnings: Vec<String>,
}

pub fn build_fantasy_draft_board(
    scoring_scheme: impl Into<String>,
    scoring_season: impl Into<String>,
    open_slots: Vec<FantasyActiveSlot>,
    candidates: Vec<FantasyDraftCandidateInput>,
    taken_import: FantasyTakenImportView,
    top: usize,
) -> Result<FantasyDraftBoardView, String> {
    let taken = taken_import
        .matched_player_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut warnings = Vec::new();
    if taken_import.ambiguous > 0 || taken_import.unresolved > 0 {
        warnings.push(format!(
            "{} ambiguous and {} unresolved taken-player row(s) remain available until resolved",
            taken_import.ambiguous, taken_import.unresolved
        ));
    }

    let available_players = candidates
        .iter()
        .filter(|candidate| !taken.contains(&candidate.player_key))
        .count();
    let excluded_taken_players = candidates.len().saturating_sub(available_players);
    let mut rows = candidates
        .into_iter()
        .filter(|candidate| !taken.contains(&candidate.player_key))
        .map(|candidate| rank_draft_candidate(candidate, &open_slots))
        .collect::<Result<Vec<_>, _>>()?;
    rows.sort_by(|a, b| {
        b.draft_value
            .total_cmp(&a.draft_value)
            .then_with(|| {
                b.components
                    .league_scored_quality
                    .total_cmp(&a.components.league_scored_quality)
            })
            .then_with(|| a.player_key.cmp(&b.player_key))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }

    let slot_kinds = [
        FantasyActiveSlotKind::Center,
        FantasyActiveSlotKind::LeftWing,
        FantasyActiveSlotKind::RightWing,
        FantasyActiveSlotKind::Defense,
        FantasyActiveSlotKind::Goalie,
        FantasyActiveSlotKind::Utility,
    ];
    let position_leaders = slot_kinds
        .into_iter()
        .filter_map(|slot_kind| {
            rows.iter()
                .find(|row| slot_kind.accepts(&row.platform_positions))
                .map(|row| FantasyDraftPositionLeader {
                    slot_kind,
                    player_key: row.player_key.clone(),
                    player: row.player.clone(),
                    draft_value: row.draft_value,
                })
        })
        .collect();
    let fallback_pick = rows.get(1).cloned();
    rows.truncate(top.max(1));

    Ok(FantasyDraftBoardView {
        schema: FANTASY_DRAFT_BOARD_SCHEMA.to_owned(),
        scoring_scheme: scoring_scheme.into(),
        scoring_season: scoring_season.into(),
        open_slots,
        available_players,
        excluded_taken_players,
        rows,
        position_leaders,
        fallback_pick,
        taken_import,
        eligibility_import: None,
        warnings,
    })
}

fn rank_draft_candidate(
    candidate: FantasyDraftCandidateInput,
    open_slots: &[FantasyActiveSlot],
) -> Result<FantasyDraftCandidateRow, String> {
    for (label, value) in [
        ("league_scored_quality", candidate.league_scored_quality),
        ("replacement_level", candidate.replacement_level),
        (
            "incremental_usable_starts",
            candidate.incremental_usable_starts,
        ),
        ("quiet_slate_games", candidate.quiet_slate_games),
        ("schedule_collision_rate", candidate.schedule_collision_rate),
        (
            "playoff_incremental_usable_starts",
            candidate.playoff_incremental_usable_starts,
        ),
        (
            "playoff_usable_value_delta",
            candidate.playoff_usable_value_delta,
        ),
        ("risk_penalty", candidate.risk_penalty),
    ] {
        if !value.is_finite() {
            return Err(format!("{} has non-finite {label}", candidate.player));
        }
    }
    if !(0.0..=1.0).contains(&candidate.schedule_collision_rate) {
        return Err(format!(
            "{} schedule_collision_rate must be between 0 and 1",
            candidate.player
        ));
    }

    let best_open_slot = open_slots
        .iter()
        .find(|slot| slot.kind.accepts(&candidate.platform_positions))
        .map(|slot| slot.kind);
    let fills_open_starter = best_open_slot.is_some();
    let starter_gap_value = if fills_open_starter { 12.0 } else { 0.0 };
    let positional_scarcity =
        (candidate.league_scored_quality - candidate.replacement_level).max(0.0) * 0.12;
    let distinct_positions =
        candidate
            .platform_positions
            .iter()
            .copied()
            .fold(Vec::new(), |mut positions, position| {
                if !positions.contains(&position) {
                    positions.push(position);
                }
                positions
            });
    let multi_position_flexibility = distinct_positions.len().saturating_sub(1) as f64 * 2.0;
    let incremental_usable_starts = candidate.incremental_usable_starts * 0.25;
    let quiet_slate_value = candidate.quiet_slate_games * 0.75;
    let schedule_diversity = (1.0 - candidate.schedule_collision_rate) * 4.0;
    let collision_cost = candidate.schedule_collision_rate * 6.0;
    let playoff_fit_value = (candidate.playoff_incremental_usable_starts * 1.5
        + candidate.playoff_usable_value_delta * 0.25)
        .clamp(-8.0, 12.0);
    let risk_penalty = candidate.risk_penalty.max(0.0);
    let components = FantasyDraftValueComponents {
        league_scored_quality: candidate.league_scored_quality,
        starter_gap_value,
        positional_scarcity,
        multi_position_flexibility,
        incremental_usable_starts,
        quiet_slate_value,
        schedule_diversity,
        collision_cost,
        playoff_fit_value,
        risk_penalty,
    };
    let draft_value = components.league_scored_quality
        + components.starter_gap_value
        + components.positional_scarcity
        + components.multi_position_flexibility
        + components.incremental_usable_starts
        + components.quiet_slate_value
        + components.schedule_diversity
        + components.playoff_fit_value
        - components.collision_cost
        - components.risk_penalty;
    let mut reasons = vec![format!(
        "league-scored quality {:.1}",
        candidate.league_scored_quality
    )];
    if let Some(slot) = best_open_slot {
        reasons.push(format!("fills open {} starter slot", slot.label()));
    }
    if distinct_positions.len() > 1 {
        reasons.push(format!("{}-position eligibility", distinct_positions.len()));
    }
    if candidate.quiet_slate_games > 0.0 {
        reasons.push(format!(
            "{:.1} quiet-slate games",
            candidate.quiet_slate_games
        ));
    }
    reasons.push(format!(
        "{:.0}% exact-date roster collision",
        candidate.schedule_collision_rate * 100.0
    ));
    if candidate.playoff_incremental_usable_starts != 0.0
        || candidate.playoff_usable_value_delta != 0.0
    {
        reasons.push(format!(
            "playoff fit {:+.1} usable starts / {:+.1} active value",
            candidate.playoff_incremental_usable_starts, candidate.playoff_usable_value_delta
        ));
    }

    Ok(FantasyDraftCandidateRow {
        rank: 0,
        player_key: candidate.player_key,
        player: candidate.player,
        nhl_team: candidate.nhl_team,
        platform_positions: distinct_positions,
        draft_value,
        fills_open_starter,
        best_open_slot,
        components,
        reasons,
    })
}

/// Parse pasted newline text or a CSV with a common player-name column, then
/// reconcile exact normalized names and aliases without silently excluding an
/// ambiguous or unknown player.
pub fn import_fantasy_taken_players(
    input: &str,
    identities: &[FantasyDraftIdentityInput],
) -> Result<FantasyTakenImportView, String> {
    let supplied = parse_taken_names(input)?;
    let mut identity_index = BTreeMap::<String, BTreeSet<usize>>::new();
    for (index, identity) in identities.iter().enumerate() {
        for name in std::iter::once(&identity.display_name).chain(identity.aliases.iter()) {
            let normalized = normalize_name(name);
            if !normalized.is_empty() {
                identity_index.entry(normalized).or_default().insert(index);
            }
        }
    }

    let mut matched_keys = BTreeSet::new();
    let mut seen_supplied = BTreeSet::new();
    let mut rows = Vec::with_capacity(supplied.len());
    for (row_number, supplied_name) in supplied {
        let normalized = normalize_name(&supplied_name);
        let (status, matched_index, candidates, message) = if normalized.is_empty() {
            (
                FantasyTakenResolutionStatus::Empty,
                None,
                Vec::new(),
                Some("player name is empty".to_owned()),
            )
        } else if !seen_supplied.insert(normalized.clone()) {
            (
                FantasyTakenResolutionStatus::Duplicate,
                None,
                Vec::new(),
                Some("duplicate taken-player row".to_owned()),
            )
        } else {
            match identity_index.get(&normalized) {
                Some(matches) if matches.len() == 1 => {
                    let index = *matches.iter().next().expect("one identity match");
                    let key = identities[index].player_key.clone();
                    if matched_keys.insert(key) {
                        (
                            FantasyTakenResolutionStatus::Matched,
                            Some(index),
                            Vec::new(),
                            None,
                        )
                    } else {
                        (
                            FantasyTakenResolutionStatus::Duplicate,
                            None,
                            Vec::new(),
                            Some("alias duplicates an already matched player".to_owned()),
                        )
                    }
                }
                Some(matches) => (
                    FantasyTakenResolutionStatus::Ambiguous,
                    None,
                    matches
                        .iter()
                        .map(|index| identities[*index].display_name.clone())
                        .collect(),
                    Some("name matches multiple canonical players".to_owned()),
                ),
                None => (
                    FantasyTakenResolutionStatus::Unresolved,
                    None,
                    Vec::new(),
                    Some("name did not match the current player pool".to_owned()),
                ),
            }
        };
        let matched = matched_index.map(|index| &identities[index]);
        rows.push(FantasyTakenPlayerRow {
            row_number,
            supplied_name,
            normalized_name: (!normalized.is_empty()).then_some(normalized),
            matched_player_key: matched.map(|identity| identity.player_key.clone()),
            matched_player: matched.map(|identity| identity.display_name.clone()),
            status,
            candidates,
            message,
        });
    }

    Ok(FantasyTakenImportView {
        schema: FANTASY_TAKEN_IMPORT_SCHEMA.to_owned(),
        matched_player_keys: matched_keys.into_iter().collect(),
        matched: count_taken_status(&rows, FantasyTakenResolutionStatus::Matched),
        duplicates: count_taken_status(&rows, FantasyTakenResolutionStatus::Duplicate),
        ambiguous: count_taken_status(&rows, FantasyTakenResolutionStatus::Ambiguous),
        unresolved: count_taken_status(&rows, FantasyTakenResolutionStatus::Unresolved),
        empty: count_taken_status(&rows, FantasyTakenResolutionStatus::Empty),
        rows,
    })
}

fn count_taken_status(
    rows: &[FantasyTakenPlayerRow],
    status: FantasyTakenResolutionStatus,
) -> usize {
    rows.iter().filter(|row| row.status == status).count()
}

fn parse_taken_names(input: &str) -> Result<Vec<(u32, String)>, String> {
    let first_nonempty = input
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let header_cells = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(Cursor::new(first_nonempty.as_bytes()))
        .records()
        .next()
        .transpose()
        .map_err(|error| format!("invalid taken-player header: {error}"))?
        .unwrap_or_default();
    let player_column = header_cells.iter().position(is_player_name_header);
    if let Some(player_column) = player_column {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(Cursor::new(input));
        return reader
            .records()
            .enumerate()
            .map(|(index, record)| {
                let record =
                    record.map_err(|error| format!("invalid taken-player CSV: {error}"))?;
                Ok((
                    (index + 2) as u32,
                    record.get(player_column).unwrap_or("").trim().to_owned(),
                ))
            })
            .collect();
    }
    Ok(input
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| ((index + 1) as u32, line.trim().to_owned()))
        .collect())
}

fn is_player_name_header(value: &str) -> bool {
    matches!(
        normalize_name(value).as_str(),
        "player" | "player name" | "name" | "full name"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyActiveSlotKind {
    Center,
    LeftWing,
    RightWing,
    Defense,
    Utility,
    Goalie,
}

impl FantasyActiveSlotKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Center => "C",
            Self::LeftWing => "LW",
            Self::RightWing => "RW",
            Self::Defense => "D",
            Self::Utility => "UTIL",
            Self::Goalie => "G",
        }
    }

    pub fn accepts(self, positions: &[Position]) -> bool {
        positions.iter().copied().any(|position| match self {
            Self::Center => position == Position::Center,
            Self::LeftWing => position == Position::LeftWing,
            Self::RightWing => position == Position::RightWing,
            Self::Defense => position == Position::Defense,
            Self::Utility => position != Position::Goalie,
            Self::Goalie => position == Position::Goalie,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyAssistantRules {
    pub schema: String,
    pub active_slots: BTreeMap<FantasyActiveSlotKind, u8>,
    pub bench_slots: u8,
    pub ir_slots: u8,
    pub ir_plus_slots: u8,
    pub weekly_acquisition_limit: u8,
    #[serde(default = "default_injury_pickup_reserve")]
    pub injury_pickup_reserve: u8,
    #[serde(default = "default_injury_reserve_release_weekday")]
    pub injury_reserve_release_weekday: u8,
    pub waiver_days: u8,
    pub waiver_claim_counts_as_acquisition: bool,
    #[serde(default = "default_free_agent_same_day")]
    pub free_agent_same_day: bool,
    pub timezone: String,
    pub morning_time: String,
    /// Monday starting the league's first fantasy playoff round.
    #[serde(default)]
    pub playoff_start: Option<NaiveDate>,
    #[serde(default = "default_playoff_rounds")]
    pub playoff_rounds: u8,
}

impl FantasyAssistantRules {
    pub fn configured_2026() -> Self {
        Self {
            schema: FANTASY_ASSISTANT_RULES_SCHEMA.to_owned(),
            active_slots: BTreeMap::from([
                (FantasyActiveSlotKind::Center, 2),
                (FantasyActiveSlotKind::LeftWing, 2),
                (FantasyActiveSlotKind::RightWing, 2),
                (FantasyActiveSlotKind::Defense, 3),
                (FantasyActiveSlotKind::Utility, 1),
                (FantasyActiveSlotKind::Goalie, 2),
            ]),
            bench_slots: 4,
            ir_slots: 2,
            ir_plus_slots: 2,
            weekly_acquisition_limit: 4,
            injury_pickup_reserve: default_injury_pickup_reserve(),
            injury_reserve_release_weekday: default_injury_reserve_release_weekday(),
            waiver_days: 2,
            waiver_claim_counts_as_acquisition: true,
            free_agent_same_day: default_free_agent_same_day(),
            timezone: "America/Los_Angeles".to_owned(),
            morning_time: "07:00".to_owned(),
            playoff_start: None,
            playoff_rounds: default_playoff_rounds(),
        }
    }

    pub fn active_slot_count(&self) -> usize {
        self.active_slots
            .values()
            .map(|count| *count as usize)
            .sum()
    }

    pub fn standard_roster_capacity(&self) -> usize {
        self.active_slot_count() + self.bench_slots as usize
    }

    pub fn total_capacity_with_reserve(&self) -> usize {
        self.standard_roster_capacity() + self.ir_slots as usize + self.ir_plus_slots as usize
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != FANTASY_ASSISTANT_RULES_SCHEMA {
            return Err(format!(
                "unsupported fantasy assistant rules schema '{}'",
                self.schema
            ));
        }
        if self.active_slot_count() == 0 {
            return Err("fantasy assistant rules require at least one active slot".to_owned());
        }
        if self.weekly_acquisition_limit == 0 {
            return Err("weekly acquisition limit must be at least one".to_owned());
        }
        if self.injury_pickup_reserve > self.weekly_acquisition_limit {
            return Err("injury pickup reserve cannot exceed the weekly limit".to_owned());
        }
        if self.injury_reserve_release_weekday > 6 {
            return Err("injury reserve release weekday must be between 0 and 6".to_owned());
        }
        if self.timezone.trim().is_empty() || self.morning_time.trim().is_empty() {
            return Err("timezone and morning time are required".to_owned());
        }
        if !(1..=4).contains(&self.playoff_rounds) {
            return Err("fantasy playoff rounds must be between one and four".to_owned());
        }
        if self
            .playoff_start
            .is_some_and(|date| date.weekday() != chrono::Weekday::Mon)
        {
            return Err("fantasy playoff start must be a Monday".to_owned());
        }
        Ok(())
    }

    pub(crate) fn expanded_active_slots(&self) -> Vec<FantasyActiveSlot> {
        let order = [
            FantasyActiveSlotKind::Center,
            FantasyActiveSlotKind::LeftWing,
            FantasyActiveSlotKind::RightWing,
            FantasyActiveSlotKind::Defense,
            FantasyActiveSlotKind::Utility,
            FantasyActiveSlotKind::Goalie,
        ];
        order
            .into_iter()
            .flat_map(|kind| {
                (1..=self.active_slots.get(&kind).copied().unwrap_or(0)).map(move |number| {
                    FantasyActiveSlot {
                        slot_id: format!("{}{number}", kind.label()),
                        kind,
                    }
                })
            })
            .collect()
    }
}

const fn default_injury_pickup_reserve() -> u8 {
    1
}

const fn default_injury_reserve_release_weekday() -> u8 {
    5
}

const fn default_free_agent_same_day() -> bool {
    true
}

const fn default_playoff_rounds() -> u8 {
    3
}

impl Default for FantasyAssistantRules {
    fn default() -> Self {
        Self::configured_2026()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyPlayerAvailabilityStatus {
    Healthy,
    DayToDay,
    GameTimeDecision,
    Out,
    InjuredReserve,
    LongTermInjuredReserve,
    Suspended,
    Personal,
    Unknown,
}

impl FantasyPlayerAvailabilityStatus {
    fn strict_ir_eligible(self) -> bool {
        matches!(self, Self::InjuredReserve | Self::LongTermInjuredReserve)
    }

    fn ir_plus_eligible(self) -> bool {
        matches!(
            self,
            Self::DayToDay
                | Self::GameTimeDecision
                | Self::Out
                | Self::InjuredReserve
                | Self::LongTermInjuredReserve
        )
    }

    pub(crate) fn expected_available(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyObservationConfidence {
    Confirmed,
    Reported,
    Estimated,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyObservationFreshness {
    Fresh,
    Stale,
    FutureDated,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyStatusObservation {
    pub player_key: String,
    pub status: FantasyPlayerAvailabilityStatus,
    pub source: String,
    pub source_url: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
    pub confidence: FantasyObservationConfidence,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyResolvedPlayerStatus {
    pub player_key: String,
    pub reported_status: FantasyPlayerAvailabilityStatus,
    /// Status safe to consume in lineup optimization. Stale, future, and
    /// missing evidence resolve to Unknown.
    pub effective_status: FantasyPlayerAvailabilityStatus,
    pub freshness: FantasyObservationFreshness,
    pub confidence: FantasyObservationConfidence,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub observed_at: Option<DateTime<Utc>>,
    pub age_minutes: Option<i64>,
    pub requires_pregame_refresh: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyInjuryPlanView {
    pub schema: String,
    pub date: NaiveDate,
    pub lineup: FantasyDailyLineupView,
    pub statuses: Vec<FantasyResolvedPlayerStatus>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyMorningActionKind {
    MoveToIr,
    MoveToIrPlus,
    RefreshStatus,
    Start,
    RefreshGoalie,
    StartGoalie,
    BenchGoalie,
    GoalieLocked,
    GoalieStreamReview,
    GoalieFallback,
    PickupReview,
    SleeperWatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyMorningAction {
    pub priority: u8,
    pub kind: FantasyMorningActionKind,
    pub player_key: Option<String>,
    pub player: Option<String>,
    pub message: String,
    pub conditional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMorningBriefingView {
    pub schema: String,
    pub date: NaiveDate,
    pub generated_at: DateTime<Utc>,
    /// The instant used for freshness, lock, waiver, and recommendation decisions.
    pub evaluated_at: DateTime<Utc>,
    pub timezone: String,
    pub injury_plan: FantasyInjuryPlanView,
    pub goalie_plan: Option<FantasyGoaliePlanView>,
    pub budget: FantasyWeekBudgetView,
    pub pickup_plan: Option<FantasyWeeklyPickupView>,
    pub sleeper_plan: Option<FantasySleeperBoardView>,
    pub next_goalie_refresh_utc: Option<DateTime<Utc>>,
    pub next_goalie_safety_check_utc: Option<DateTime<Utc>>,
    pub next_goalie_lock_utc: Option<DateTime<Utc>>,
    pub goalie_refreshes_due_now: usize,
    pub goalie_safety_checks_due_now: usize,
    pub actions: Vec<FantasyMorningAction>,
    /// Hash of decision-bearing fields only. It deliberately excludes
    /// `generated_at`, status ages, and warning prose.
    pub material_fingerprint: String,
    pub suppressed_unchanged: bool,
    pub warnings: Vec<String>,
}

pub fn build_fantasy_morning_briefing(
    generated_at: DateTime<Utc>,
    evaluated_at: DateTime<Utc>,
    timezone: impl Into<String>,
    injury_plan: FantasyInjuryPlanView,
    goalie_plan: Option<FantasyGoaliePlanView>,
    budget: FantasyWeekBudgetView,
    pickup_plan: Option<FantasyWeeklyPickupView>,
    sleeper_plan: Option<FantasySleeperBoardView>,
) -> FantasyMorningBriefingView {
    let timezone = timezone.into();
    let mut actions = Vec::new();
    let mut goalie_stream_primary = None::<(String, String)>;
    for row in &injury_plan.lineup.injured_reserve {
        actions.push(FantasyMorningAction {
            priority: 10,
            kind: FantasyMorningActionKind::MoveToIr,
            player_key: Some(row.player_key.clone()),
            player: Some(row.player.clone()),
            message: format!("Move {} to {}", row.player, row.reserve_slot),
            conditional: false,
        });
    }
    for row in &injury_plan.lineup.injured_reserve_plus {
        actions.push(FantasyMorningAction {
            priority: 20,
            kind: FantasyMorningActionKind::MoveToIrPlus,
            player_key: Some(row.player_key.clone()),
            player: Some(row.player.clone()),
            message: format!("Move {} to {}", row.player, row.reserve_slot),
            conditional: false,
        });
    }
    for status in injury_plan
        .statuses
        .iter()
        .filter(|status| status.requires_pregame_refresh)
    {
        actions.push(FantasyMorningAction {
            priority: 30,
            kind: FantasyMorningActionKind::RefreshStatus,
            player_key: Some(status.player_key.clone()),
            player: None,
            message: format!(
                "Refresh availability for {} before lineup lock",
                status.player_key
            ),
            conditional: true,
        });
    }
    if let Some(plan) = &goalie_plan {
        for row in plan.rows.iter().filter(|row| row.date == injury_plan.date) {
            if row.evidence.effective_state.is_confirmed()
                && row
                    .refresh_deadline_utc
                    .is_some_and(|deadline| deadline <= evaluated_at)
                && row.game_start_utc.is_some_and(|start| start > evaluated_at)
            {
                actions.push(FantasyMorningAction {
                    priority: 24,
                    kind: FantasyMorningActionKind::RefreshGoalie,
                    player_key: Some(row.player_key.clone()),
                    player: Some(row.player.clone()),
                    message: format!(
                        "Final safety check now: verify {} is still {:?} before the {} lock",
                        row.player,
                        row.evidence.effective_state,
                        row.game_start_utc
                            .expect("future game start checked")
                            .to_rfc3339()
                    ),
                    conditional: true,
                });
            }
            match (row.evidence.effective_state, row.action) {
                (_, FantasyGoaliePlanAction::Locked) => {
                    actions.push(FantasyMorningAction {
                        priority: 37,
                        kind: FantasyMorningActionKind::GoalieLocked,
                        player_key: Some(row.player_key.clone()),
                        player: Some(row.player.clone()),
                        message: format!(
                            "{} vs {} is locked; no lineup change is available",
                            row.player, row.opponent
                        ),
                        conditional: false,
                    });
                }
                (FantasyGoalieStartState::ConfirmedStarting, FantasyGoaliePlanAction::Start) => {
                    actions.push(FantasyMorningAction {
                        priority: 35,
                        kind: FantasyMorningActionKind::StartGoalie,
                        player_key: Some(row.player_key.clone()),
                        player: Some(row.player.clone()),
                        message: format!(
                            "Confirmed today: start {} vs {} in a goalie slot",
                            row.player, row.opponent
                        ),
                        conditional: false,
                    });
                }
                (FantasyGoalieStartState::ConfirmedBackup, _)
                | (FantasyGoalieStartState::ReportedBackup, _) => {
                    actions.push(FantasyMorningAction {
                        priority: 36,
                        kind: FantasyMorningActionKind::BenchGoalie,
                        player_key: Some(row.player_key.clone()),
                        player: Some(row.player.clone()),
                        message: format!(
                            "Do not start {} vs {}: current evidence says backup",
                            row.player, row.opponent
                        ),
                        conditional: row.evidence.effective_state
                            != FantasyGoalieStartState::ConfirmedBackup,
                    });
                }
                _ => {
                    let timing = match row.refresh_urgency {
                        FantasyGoalieRefreshUrgency::RefreshNow => "check now",
                        FantasyGoalieRefreshUrgency::RefreshSoon => "refresh within three hours",
                        FantasyGoalieRefreshUrgency::CheckLater => "check again closer to lock",
                        FantasyGoalieRefreshUrgency::Locked => "game locked",
                    };
                    actions.push(FantasyMorningAction {
                        priority: 25,
                        kind: FantasyMorningActionKind::RefreshGoalie,
                        player_key: Some(row.player_key.clone()),
                        player: Some(row.player.clone()),
                        message: format!(
                            "Refresh today's starter evidence for {} vs {}: {} ({:?}, {:.0}% workload probability; lock {})",
                            row.player,
                            row.opponent,
                            timing,
                            row.evidence.effective_state,
                            row.start_probability * 100.0,
                            row.game_start_utc
                                .map(|time| time.to_rfc3339())
                                .unwrap_or_else(|| "time unavailable".to_owned())
                        ),
                        conditional: true,
                    });
                }
            }
        }
        let mut today_streams = plan
            .stream_candidates
            .iter()
            .filter(|candidate| {
                candidate.acquisition_eligible
                    && candidate.game_dates.contains(&injury_plan.date)
                    && candidate.expected_appearance_gain > 0.0
                    && candidate
                        .next_game_start_utc
                        .is_none_or(|start| start > evaluated_at)
            })
            .collect::<Vec<_>>();
        today_streams.sort_by(|a, b| {
            b.confirmed_start_dates
                .contains(&injury_plan.date)
                .cmp(&a.confirmed_start_dates.contains(&injury_plan.date))
                .then_with(|| {
                    b.reported_or_estimated_start_dates
                        .contains(&injury_plan.date)
                        .cmp(
                            &a.reported_or_estimated_start_dates
                                .contains(&injury_plan.date),
                        )
                })
                .then_with(|| {
                    b.expected_appearance_gain
                        .total_cmp(&a.expected_appearance_gain)
                })
                .then_with(|| a.player_key.cmp(&b.player_key))
        });
        let mut today_streams = today_streams.into_iter();
        if let Some(primary) = today_streams
            .next()
            .filter(|candidate| plan.minimum_at_risk || candidate.expected_appearance_gain >= 0.75)
        {
            goalie_stream_primary = Some((primary.player_key.clone(), primary.player.clone()));
            let mut message = if primary.confirmed_start_dates.contains(&injury_plan.date)
                && primary
                    .next_safety_check_utc
                    .is_some_and(|check| check <= evaluated_at)
            {
                format!(
                    "Final verification due now: confirm {} remains the starter, then add if still free (+{:.2} expected usable starts)",
                    primary.player, primary.expected_appearance_gain
                )
            } else if primary.confirmed_start_dates.contains(&injury_plan.date) {
                format!(
                    "Fresh confirmed goalie stream: add {} if still free (+{:.2} expected usable starts)",
                    primary.player, primary.expected_appearance_gain
                )
            } else {
                format!(
                    "Conditional goalie stream: add {} only after a same-day start confirmation (+{:.2} expected usable starts)",
                    primary.player, primary.expected_appearance_gain
                )
            };
            if budget.proactive_acquisitions_remaining == 1 {
                message.push_str("; this uses the final proactive acquisition");
            }
            append_goalie_stream_pairing(&mut message, &primary.player_key, pickup_plan.as_ref());
            actions.push(FantasyMorningAction {
                priority: 45,
                kind: FantasyMorningActionKind::GoalieStreamReview,
                player_key: Some(primary.player_key.clone()),
                player: Some(primary.player.clone()),
                message,
                conditional: true,
            });
            if let Some(fallback) = today_streams.next() {
                let mut message = if fallback.confirmed_start_dates.contains(&injury_plan.date) {
                    format!(
                        "Confirmed fallback if {} is claimed: add {} if still free",
                        primary.player, fallback.player
                    )
                } else {
                    format!(
                        "Conditional fallback if {} is claimed or unconfirmed: recheck {} before lock; do not add without confirmation",
                        primary.player, fallback.player
                    )
                };
                append_goalie_stream_pairing(
                    &mut message,
                    &fallback.player_key,
                    pickup_plan.as_ref(),
                );
                actions.push(FantasyMorningAction {
                    priority: 46,
                    kind: FantasyMorningActionKind::GoalieFallback,
                    player_key: Some(fallback.player_key.clone()),
                    player: Some(fallback.player.clone()),
                    message,
                    conditional: true,
                });
            }
        }
    }
    for row in injury_plan.lineup.active.iter().filter(|row| {
        row.has_game
            && row.status == FantasyPlayerAvailabilityStatus::Healthy
            && (goalie_plan.is_none() || row.slot_kind != FantasyActiveSlotKind::Goalie)
    }) {
        actions.push(FantasyMorningAction {
            priority: 40,
            kind: FantasyMorningActionKind::Start,
            player_key: Some(row.player_key.clone()),
            player: Some(row.player.clone()),
            message: format!("Start {} in {}", row.player, row.slot_id),
            conditional: false,
        });
    }
    let top_pickup = pickup_plan
        .as_ref()
        .and_then(|plan| plan.rows.first())
        .filter(|row| row.projected_value_delta > 0.0 || row.incremental_usable_starts > 0.0);
    let exceptional_reserve_override = pickup_plan.as_ref().is_some_and(|plan| {
        plan.warnings
            .iter()
            .any(|warning| warning.contains("exceptional healthy-roster value"))
    });
    if let Some(row) = top_pickup {
        let duplicates_goalie_stream = goalie_stream_primary
            .as_ref()
            .is_some_and(|(player_key, _)| player_key == &row.add_player_key);
        if !duplicates_goalie_stream {
            let message = if exceptional_reserve_override {
                format!(
                    "Exceptional-value reserve override: after confirming the roster is healthy, add {} and drop {} ({:+.1} usable starts, {:+.2} projected value; final move would be spent)",
                    row.add_player,
                    row.drop_player,
                    row.incremental_usable_starts,
                    row.projected_value_delta
                )
            } else if budget.proactive_acquisitions_remaining == 1 {
                if let Some((_, goalie)) = &goalie_stream_primary {
                    format!(
                        "Alternative to the {} goalie stream—choose only one use of the final proactive acquisition: add {} and drop {} ({:+.1} usable starts, {:+.2} projected value)",
                        goalie,
                        row.add_player,
                        row.drop_player,
                        row.incremental_usable_starts,
                        row.projected_value_delta
                    )
                } else {
                    format!(
                        "After verifying availability, add {} and drop {} ({:+.1} usable starts, {:+.2} projected value; final proactive acquisition)",
                        row.add_player,
                        row.drop_player,
                        row.incremental_usable_starts,
                        row.projected_value_delta
                    )
                }
            } else {
                format!(
                    "After verifying availability, add {} and drop {} ({:+.1} usable starts, {:+.2} projected value; {} adds remain)",
                    row.add_player,
                    row.drop_player,
                    row.incremental_usable_starts,
                    row.projected_value_delta,
                    budget.acquisitions_remaining
                )
            };
            actions.push(FantasyMorningAction {
                priority: 50,
                kind: FantasyMorningActionKind::PickupReview,
                player_key: Some(row.add_player_key.clone()),
                player: Some(row.add_player.clone()),
                message,
                conditional: true,
            });
        }
    } else if budget.can_add && !budget.can_proactively_add {
        actions.push(FantasyMorningAction {
            priority: 50,
            kind: FantasyMorningActionKind::PickupReview,
            player_key: None,
            player: None,
            message: format!(
                "Protect the final {} acquisition(s) for an injury until {}; {} hard-limit move(s) remain",
                budget.injury_reserve_active,
                budget
                    .injury_reserve_releases_on
                    .map(|date| date.to_string())
                    .unwrap_or_else(|| "the reserve window".to_owned()),
                budget.acquisitions_remaining
            ),
            conditional: false,
        });
    } else if budget.can_add && pickup_plan.is_none() {
        actions.push(FantasyMorningAction {
            priority: 50,
            kind: FantasyMorningActionKind::PickupReview,
            player_key: None,
            player: None,
            message: format!(
                "Review weekly pickups ({} of {} adds remain)",
                budget.acquisitions_remaining, budget.acquisition_limit
            ),
            conditional: true,
        });
    }
    if let Some(row) = sleeper_plan
        .as_ref()
        .and_then(|plan| plan.rows.first())
        .filter(|row| row.score >= 15.0)
    {
        let supports_pickup =
            top_pickup.is_some_and(|pickup| pickup.add_player_key == row.player_key);
        actions.push(FantasyMorningAction {
            priority: if supports_pickup { 49 } else { 55 },
            kind: FantasyMorningActionKind::SleeperWatch,
            player_key: Some(row.player_key.clone()),
            player: Some(row.player.clone()),
            message: if supports_pickup {
                format!(
                    "Sleeper evidence supports pickup candidate {} ({:.1} sleeper score)",
                    row.player, row.score
                )
            } else {
                format!(
                    "Watch sleeper {} ({}, {:.1} score): {}",
                    row.player,
                    row.nhl_team,
                    row.score,
                    row.reasons
                        .first()
                        .map(String::as_str)
                        .unwrap_or("rising rates")
                )
            },
            conditional: true,
        });
    }
    actions.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.player_key.cmp(&b.player_key))
            .then_with(|| a.message.cmp(&b.message))
    });

    let mut status_material = injury_plan
        .statuses
        .iter()
        .map(|status| {
            (
                status.player_key.as_str(),
                status.reported_status,
                status.effective_status,
                status.freshness,
                status.confidence,
                status.requires_pregame_refresh,
            )
        })
        .collect::<Vec<_>>();
    status_material.sort_by(|a, b| a.0.cmp(b.0));
    let budget_material = (
        budget.week_start,
        budget.week_end,
        budget.acquisition_limit,
        budget.acquisitions_used,
        budget.acquisitions_remaining,
        budget.can_add,
        budget.injury_reserve_active,
        budget.proactive_acquisitions_remaining,
        budget.can_proactively_add,
        budget.injury_reserve_releases_on,
    );
    let material = serde_json::to_vec(&(
        injury_plan.date,
        &timezone,
        &actions,
        status_material,
        goalie_plan.as_ref().map(|plan| {
            (
                plan.minimum_at_risk,
                plan.minimum_shortfall,
                &plan.rows,
                &plan.stream_candidates,
                &plan.portfolio,
            )
        }),
        &injury_plan.lineup.missing_active_slots,
        injury_plan.lineup.usable_starts,
        budget_material,
    ))
    .expect("morning briefing material fields are serializable");
    let material_fingerprint = format!("{:x}", Sha256::digest(material));
    let mut warnings = injury_plan.warnings.clone();
    if let Some(plan) = &pickup_plan {
        warnings.extend(plan.warnings.iter().cloned());
        if budget.can_add && top_pickup.is_none() {
            warnings.push(
                "No positive legal add/drop move was found for the remaining week".to_owned(),
            );
        }
    }
    if let Some(plan) = &sleeper_plan {
        warnings.extend(plan.warnings.iter().cloned());
    }
    if let Some(plan) = &goalie_plan {
        warnings.extend(plan.warnings.iter().cloned());
    }
    warnings.push("Briefing is advisory and does not mutate the fantasy platform".to_owned());

    let next_goalie_refresh_utc = goalie_plan
        .as_ref()
        .and_then(|plan| plan.next_required_refresh_utc);
    let next_goalie_safety_check_utc = goalie_plan
        .as_ref()
        .and_then(|plan| plan.next_safety_check_utc);
    let next_goalie_lock_utc = goalie_plan
        .as_ref()
        .and_then(|plan| plan.next_game_lock_utc);
    let goalie_refreshes_due_now = goalie_plan
        .as_ref()
        .map(|plan| plan.refreshes_due_now)
        .unwrap_or_default();
    let goalie_safety_checks_due_now = goalie_plan
        .as_ref()
        .map(|plan| plan.safety_checks_due_now)
        .unwrap_or_default();

    FantasyMorningBriefingView {
        schema: FANTASY_MORNING_BRIEFING_SCHEMA.to_owned(),
        date: injury_plan.date,
        generated_at,
        evaluated_at,
        timezone,
        injury_plan,
        goalie_plan,
        budget,
        pickup_plan,
        sleeper_plan,
        next_goalie_refresh_utc,
        next_goalie_safety_check_utc,
        next_goalie_lock_utc,
        goalie_refreshes_due_now,
        goalie_safety_checks_due_now,
        actions,
        material_fingerprint,
        suppressed_unchanged: false,
        warnings,
    }
}

fn append_goalie_stream_pairing(
    message: &mut String,
    player_key: &str,
    pickup_plan: Option<&FantasyWeeklyPickupView>,
) {
    if let Some(row) = pickup_plan.and_then(|plan| {
        plan.rows
            .iter()
            .find(|row| row.add_player_key == player_key)
    }) {
        message.push_str(&format!(
            "; weekly optimizer pairs the add with dropping {} ({:+.2} projected value)",
            row.drop_player, row.projected_value_delta
        ));
    } else {
        message.push_str(
            "; no legal drop pairing is available—verify an open roster spot before executing",
        );
    }
}

pub fn resolve_fantasy_player_status(
    player_key: impl Into<String>,
    observations: &[FantasyStatusObservation],
    now: DateTime<Utc>,
    max_age_minutes: i64,
) -> FantasyResolvedPlayerStatus {
    let player_key = player_key.into();
    let latest = observations
        .iter()
        .filter(|observation| observation.player_key == player_key)
        .max_by(|a, b| {
            a.observed_at
                .cmp(&b.observed_at)
                .then_with(|| a.fetched_at.cmp(&b.fetched_at))
                .then_with(|| a.source.cmp(&b.source))
        });
    let Some(observation) = latest else {
        return FantasyResolvedPlayerStatus {
            player_key,
            reported_status: FantasyPlayerAvailabilityStatus::Unknown,
            effective_status: FantasyPlayerAvailabilityStatus::Unknown,
            freshness: FantasyObservationFreshness::Missing,
            confidence: FantasyObservationConfidence::Unknown,
            source: None,
            source_url: None,
            observed_at: None,
            age_minutes: None,
            requires_pregame_refresh: true,
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
    let effective_status = if freshness == FantasyObservationFreshness::Fresh {
        observation.status
    } else {
        FantasyPlayerAvailabilityStatus::Unknown
    };
    let requires_pregame_refresh = freshness != FantasyObservationFreshness::Fresh
        || matches!(
            observation.status,
            FantasyPlayerAvailabilityStatus::DayToDay
                | FantasyPlayerAvailabilityStatus::GameTimeDecision
                | FantasyPlayerAvailabilityStatus::Unknown
        )
        || observation.confidence != FantasyObservationConfidence::Confirmed;
    FantasyResolvedPlayerStatus {
        player_key,
        reported_status: observation.status,
        effective_status,
        freshness,
        confidence: observation.confidence,
        source: Some(observation.source.clone()),
        source_url: observation.source_url.clone(),
        observed_at: Some(observation.observed_at),
        age_minutes: Some(age_minutes),
        requires_pregame_refresh,
        detail: observation.detail.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyLineupPlayerInput {
    pub player_key: String,
    pub display_name: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    pub projected_value: f64,
    pub has_game: bool,
    pub status: FantasyPlayerAvailabilityStatus,
    /// Exact active slot id such as `LW1` when that player's game has locked.
    /// `None` with `locked=true` means the player is locked outside an active slot.
    pub locked_slot: Option<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyActiveSlot {
    pub slot_id: String,
    pub kind: FantasyActiveSlotKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyLineupAssignmentRow {
    pub slot_id: String,
    pub slot_kind: FantasyActiveSlotKind,
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    pub projected_value: f64,
    pub has_game: bool,
    pub status: FantasyPlayerAvailabilityStatus,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyReserveAssignmentRow {
    pub reserve_slot: String,
    pub player_key: String,
    pub player: String,
    pub status: FantasyPlayerAvailabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyBenchAssignmentRow {
    pub bench_slot: String,
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub platform_positions: Vec<Position>,
    pub projected_value: f64,
    pub has_game: bool,
    pub status: FantasyPlayerAvailabilityStatus,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDailyLineupView {
    pub schema: String,
    pub rules: FantasyAssistantRules,
    pub active: Vec<FantasyLineupAssignmentRow>,
    pub bench: Vec<String>,
    #[serde(default)]
    pub bench_assignments: Vec<FantasyBenchAssignmentRow>,
    pub injured_reserve: Vec<FantasyReserveAssignmentRow>,
    pub injured_reserve_plus: Vec<FantasyReserveAssignmentRow>,
    pub overflow: Vec<String>,
    pub missing_active_slots: Vec<FantasyActiveSlot>,
    pub projected_active_value: f64,
    pub usable_starts: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct AssignmentState {
    score: f64,
    assignments: Vec<(usize, usize)>,
}

pub fn build_fantasy_daily_lineup(
    rules: FantasyAssistantRules,
    mut players: Vec<FantasyLineupPlayerInput>,
) -> Result<FantasyDailyLineupView, String> {
    rules.validate()?;
    players.sort_by(|a, b| a.player_key.cmp(&b.player_key));
    let mut warnings = Vec::new();

    let mut ir = Vec::new();
    let mut ir_plus = Vec::new();
    let mut reserved_keys = BTreeSet::new();
    for player in players
        .iter()
        .filter(|player| player.status.strict_ir_eligible())
    {
        if ir.len() < rules.ir_slots as usize {
            ir.push(reserve_row("IR", ir.len() + 1, player));
            reserved_keys.insert(player.player_key.clone());
        }
    }
    for player in &players {
        if reserved_keys.contains(&player.player_key) || !player.status.ir_plus_eligible() {
            continue;
        }
        if ir_plus.len() < rules.ir_plus_slots as usize {
            ir_plus.push(reserve_row("IR+", ir_plus.len() + 1, player));
            reserved_keys.insert(player.player_key.clone());
        }
    }

    let mut standard = players
        .into_iter()
        .filter(|player| !reserved_keys.contains(&player.player_key))
        .collect::<Vec<_>>();
    standard.sort_by(|a, b| {
        b.projected_value
            .total_cmp(&a.projected_value)
            .then_with(|| a.player_key.cmp(&b.player_key))
    });
    let overflow = if standard.len() > rules.standard_roster_capacity() {
        standard
            .drain(rules.standard_roster_capacity()..)
            .map(|player| player.display_name)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if !overflow.is_empty() {
        warnings.push(format!(
            "{} player(s) exceed the {}-player standard roster capacity",
            overflow.len(),
            rules.standard_roster_capacity()
        ));
    }

    let slots = rules.expanded_active_slots();
    let slot_by_id = slots
        .iter()
        .enumerate()
        .map(|(index, slot)| (slot.slot_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut occupied_slots = BTreeSet::new();
    let mut assigned_players = BTreeSet::new();
    let mut active = Vec::new();

    for player in standard.iter().filter(|player| player.locked) {
        let Some(slot_id) = player.locked_slot.as_ref() else {
            assigned_players.insert(player.player_key.clone());
            continue;
        };
        let Some(slot_index) = slot_by_id.get(slot_id).copied() else {
            warnings.push(format!(
                "{} is locked to unknown slot {}",
                player.display_name, slot_id
            ));
            assigned_players.insert(player.player_key.clone());
            continue;
        };
        let slot = &slots[slot_index];
        if occupied_slots.contains(&slot_index) || !slot.kind.accepts(&player.platform_positions) {
            warnings.push(format!(
                "{} has an invalid locked assignment to {}",
                player.display_name, slot_id
            ));
            assigned_players.insert(player.player_key.clone());
            continue;
        }
        occupied_slots.insert(slot_index);
        assigned_players.insert(player.player_key.clone());
        active.push(active_row(slot, player));
    }

    let available_slots = slots
        .iter()
        .enumerate()
        .filter(|(index, _)| !occupied_slots.contains(index))
        .map(|(_, slot)| slot.clone())
        .collect::<Vec<_>>();
    let candidates = standard
        .iter()
        .filter(|player| !assigned_players.contains(&player.player_key) && !player.locked)
        .collect::<Vec<_>>();
    let matched = maximum_weight_assignment(&candidates, &available_slots);
    for (player_index, slot_index) in matched {
        let player = candidates[player_index];
        let slot = &available_slots[slot_index];
        assigned_players.insert(player.player_key.clone());
        active.push(active_row(slot, player));
    }

    active.sort_by(|a, b| {
        let a_index = slot_by_id.get(&a.slot_id).copied().unwrap_or(usize::MAX);
        let b_index = slot_by_id.get(&b.slot_id).copied().unwrap_or(usize::MAX);
        a_index.cmp(&b_index)
    });
    let bench_players = standard
        .iter()
        .filter(|player| !active.iter().any(|row| row.player_key == player.player_key))
        .collect::<Vec<_>>();
    let bench = bench_players
        .iter()
        .map(|player| player.display_name.clone())
        .collect::<Vec<_>>();
    let bench_assignments = bench_players
        .iter()
        .enumerate()
        .map(|(index, player)| FantasyBenchAssignmentRow {
            bench_slot: format!("BN{}", index + 1),
            player_key: player.player_key.clone(),
            player: player.display_name.clone(),
            nhl_team: player.nhl_team.clone(),
            platform_positions: player.platform_positions.clone(),
            projected_value: player.projected_value,
            has_game: player.has_game,
            status: player.status,
            locked: player.locked,
        })
        .collect::<Vec<_>>();
    if bench.len() > rules.bench_slots as usize {
        warnings.push(format!(
            "{} players are outside active slots; only {} bench slots are configured",
            bench.len(),
            rules.bench_slots
        ));
    }

    let active_slot_ids = active
        .iter()
        .map(|row| row.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let missing_active_slots = slots
        .into_iter()
        .filter(|slot| !active_slot_ids.contains(slot.slot_id.as_str()))
        .collect::<Vec<_>>();
    let projected_active_value = active
        .iter()
        .filter(|row| row.has_game && row.status.expected_available())
        .map(|row| row.projected_value)
        .sum();
    let usable_starts = active
        .iter()
        .filter(|row| {
            row.has_game && row.status.expected_available() && row.projected_value.is_finite()
        })
        .count();

    for player in standard
        .iter()
        .filter(|player| player.has_game && !player.status.expected_available())
    {
        warnings.push(format!(
            "{} has a game but status {:?} is not treated as available",
            player.display_name, player.status
        ));
    }

    Ok(FantasyDailyLineupView {
        schema: FANTASY_DAILY_LINEUP_SCHEMA.to_owned(),
        rules,
        active,
        bench,
        bench_assignments,
        injured_reserve: ir,
        injured_reserve_plus: ir_plus,
        overflow,
        missing_active_slots,
        projected_active_value,
        usable_starts,
        warnings,
    })
}

fn reserve_row(
    prefix: &str,
    number: usize,
    player: &FantasyLineupPlayerInput,
) -> FantasyReserveAssignmentRow {
    FantasyReserveAssignmentRow {
        reserve_slot: format!("{prefix}{number}"),
        player_key: player.player_key.clone(),
        player: player.display_name.clone(),
        status: player.status,
    }
}

fn active_row(
    slot: &FantasyActiveSlot,
    player: &FantasyLineupPlayerInput,
) -> FantasyLineupAssignmentRow {
    FantasyLineupAssignmentRow {
        slot_id: slot.slot_id.clone(),
        slot_kind: slot.kind,
        player_key: player.player_key.clone(),
        player: player.display_name.clone(),
        nhl_team: player.nhl_team.clone(),
        platform_positions: player.platform_positions.clone(),
        projected_value: player.projected_value,
        has_game: player.has_game,
        status: player.status,
        locked: player.locked,
    }
}

fn maximum_weight_assignment(
    players: &[&FantasyLineupPlayerInput],
    slots: &[FantasyActiveSlot],
) -> Vec<(usize, usize)> {
    if slots.is_empty() || players.is_empty() {
        return Vec::new();
    }
    let mut states = BTreeMap::<u32, AssignmentState>::new();
    states.insert(
        0,
        AssignmentState {
            score: 0.0,
            assignments: Vec::new(),
        },
    );
    for (player_index, player) in players.iter().enumerate() {
        let snapshot = states.clone();
        for (mask, state) in snapshot {
            for (slot_index, slot) in slots.iter().enumerate() {
                let bit = 1u32 << slot_index;
                if mask & bit != 0 || !slot.kind.accepts(&player.platform_positions) {
                    continue;
                }
                let mut next = state.clone();
                next.score += if player.has_game && player.status.expected_available() {
                    player.projected_value
                } else {
                    0.0
                };
                next.assignments.push((player_index, slot_index));
                let next_mask = mask | bit;
                let should_replace = states.get(&next_mask).is_none_or(|current| {
                    next.score > current.score
                        || (next.score == current.score
                            && next.assignments.as_slice() < current.assignments.as_slice())
                });
                if should_replace {
                    states.insert(next_mask, next);
                }
            }
        }
    }
    states
        .into_iter()
        .max_by(|(a_mask, a), (b_mask, b)| {
            a_mask
                .count_ones()
                .cmp(&b_mask.count_ones())
                .then_with(|| a.score.total_cmp(&b.score))
                .then_with(|| b.assignments.cmp(&a.assignments))
        })
        .map_or_else(Vec::new, |(_, state)| state.assignments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn identities() -> Vec<FantasyDraftIdentityInput> {
        vec![
            FantasyDraftIdentityInput {
                player_key: "mcdavid".to_owned(),
                display_name: "Connor McDavid".to_owned(),
                aliases: vec!["C. McDavid".to_owned()],
            },
            FantasyDraftIdentityInput {
                player_key: "darren-raddysh".to_owned(),
                display_name: "Darren Raddysh".to_owned(),
                aliases: Vec::new(),
            },
        ]
    }

    fn empty_taken() -> FantasyTakenImportView {
        import_fantasy_taken_players("", &[]).unwrap()
    }

    fn draft_candidate(
        key: &str,
        positions: Vec<Position>,
        quality: f64,
    ) -> FantasyDraftCandidateInput {
        FantasyDraftCandidateInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: "NYR".to_owned(),
            platform_positions: positions,
            league_scored_quality: quality,
            replacement_level: 50.0,
            incremental_usable_starts: 1.0,
            quiet_slate_games: 1.0,
            schedule_collision_rate: 0.5,
            playoff_incremental_usable_starts: 0.0,
            playoff_usable_value_delta: 0.0,
            risk_penalty: 0.0,
        }
    }

    fn player(key: &str, positions: Vec<Position>, value: f64) -> FantasyLineupPlayerInput {
        FantasyLineupPlayerInput {
            player_key: key.to_owned(),
            display_name: key.to_owned(),
            nhl_team: "NYR".to_owned(),
            platform_positions: positions,
            projected_value: value,
            has_game: true,
            status: FantasyPlayerAvailabilityStatus::Healthy,
            locked_slot: None,
            locked: false,
        }
    }

    #[test]
    fn configured_rules_match_league_contract() {
        let rules = FantasyAssistantRules::configured_2026();
        assert_eq!(rules.active_slot_count(), 12);
        assert_eq!(rules.standard_roster_capacity(), 16);
        assert_eq!(rules.total_capacity_with_reserve(), 20);
        assert_eq!(rules.weekly_acquisition_limit, 4);
        assert_eq!(rules.waiver_days, 2);
        assert_eq!(rules.playoff_start, None);
        assert_eq!(rules.playoff_rounds, 3);
        assert!(rules.validate().is_ok());
    }

    #[test]
    fn playoff_calendar_requires_monday_and_one_to_four_rounds() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.playoff_start = Some(NaiveDate::from_ymd_opt(2027, 3, 16).unwrap());
        assert_eq!(
            rules.validate().unwrap_err(),
            "fantasy playoff start must be a Monday"
        );
        rules.playoff_start = Some(NaiveDate::from_ymd_opt(2027, 3, 15).unwrap());
        rules.playoff_rounds = 5;
        assert_eq!(
            rules.validate().unwrap_err(),
            "fantasy playoff rounds must be between one and four"
        );
    }

    #[test]
    fn utility_accepts_skater_and_rejects_goalie() {
        assert!(FantasyActiveSlotKind::Utility.accepts(&[Position::Center]));
        assert!(FantasyActiveSlotKind::Utility.accepts(&[Position::Defense]));
        assert!(!FantasyActiveSlotKind::Utility.accepts(&[Position::Goalie]));
    }

    #[test]
    fn multi_position_player_is_assigned_once_and_preserves_wing_gap() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([
            (FantasyActiveSlotKind::Center, 1),
            (FantasyActiveSlotKind::LeftWing, 1),
        ]);
        rules.bench_slots = 1;
        let view = build_fantasy_daily_lineup(
            rules,
            vec![
                player("flex", vec![Position::Center, Position::LeftWing], 8.0),
                player("center", vec![Position::Center], 7.0),
            ],
        )
        .unwrap();
        assert_eq!(view.active.len(), 2);
        assert_eq!(
            view.active
                .iter()
                .filter(|row| row.player_key == "flex")
                .count(),
            1
        );
        assert_eq!(
            view.active
                .iter()
                .find(|row| row.slot_kind == FantasyActiveSlotKind::LeftWing)
                .unwrap()
                .player_key,
            "flex"
        );
    }

    #[test]
    fn third_goalie_can_use_bench_but_not_utility() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([
            (FantasyActiveSlotKind::Goalie, 2),
            (FantasyActiveSlotKind::Utility, 1),
        ]);
        rules.bench_slots = 1;
        let view = build_fantasy_daily_lineup(
            rules,
            vec![
                player("g1", vec![Position::Goalie], 8.0),
                player("g2", vec![Position::Goalie], 7.0),
                player("g3", vec![Position::Goalie], 6.0),
                player("d1", vec![Position::Defense], 5.0),
            ],
        )
        .unwrap();
        assert_eq!(
            view.active
                .iter()
                .filter(|row| row.slot_kind == FantasyActiveSlotKind::Goalie)
                .count(),
            2
        );
        assert_eq!(
            view.active
                .iter()
                .find(|row| row.slot_kind == FantasyActiveSlotKind::Utility)
                .unwrap()
                .player_key,
            "d1"
        );
        assert!(view.bench.contains(&"g3".to_owned()));
    }

    #[test]
    fn strict_ir_is_filled_before_ir_plus() {
        let mut ir_player = player("ir", vec![Position::Defense], 4.0);
        ir_player.status = FantasyPlayerAvailabilityStatus::InjuredReserve;
        let mut dtd_player = player("dtd", vec![Position::Center], 5.0);
        dtd_player.status = FantasyPlayerAvailabilityStatus::DayToDay;
        let view = build_fantasy_daily_lineup(
            FantasyAssistantRules::configured_2026(),
            vec![ir_player, dtd_player],
        )
        .unwrap();
        assert_eq!(view.injured_reserve[0].player_key, "ir");
        assert_eq!(view.injured_reserve_plus[0].player_key, "dtd");
    }

    #[test]
    fn locked_active_player_cannot_be_displaced() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(FantasyActiveSlotKind::Center, 1)]);
        rules.bench_slots = 1;
        let mut locked = player("locked", vec![Position::Center], 1.0);
        locked.locked = true;
        locked.locked_slot = Some("C1".to_owned());
        let view = build_fantasy_daily_lineup(
            rules,
            vec![locked, player("better", vec![Position::Center], 10.0)],
        )
        .unwrap();
        assert_eq!(view.active[0].player_key, "locked");
        assert!(view.bench.contains(&"better".to_owned()));
    }

    #[test]
    fn unavailable_active_player_does_not_count_as_a_usable_start() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(FantasyActiveSlotKind::Center, 1)]);
        rules.bench_slots = 0;
        rules.ir_slots = 0;
        rules.ir_plus_slots = 0;
        let mut unavailable = player("out", vec![Position::Center], 10.0);
        unavailable.status = FantasyPlayerAvailabilityStatus::Out;

        let view = build_fantasy_daily_lineup(rules, vec![unavailable]).unwrap();

        assert_eq!(view.active[0].player_key, "out");
        assert_eq!(view.usable_starts, 0);
        assert_eq!(view.projected_active_value, 0.0);
        assert!(view.warnings.iter().any(|warning| warning.contains("out")));
    }

    #[test]
    fn unknown_status_is_not_treated_as_confirmed_available() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(FantasyActiveSlotKind::Center, 1)]);
        rules.bench_slots = 0;
        let mut unknown = player("unknown", vec![Position::Center], 10.0);
        unknown.status = FantasyPlayerAvailabilityStatus::Unknown;
        let view = build_fantasy_daily_lineup(rules, vec![unknown]).unwrap();

        assert_eq!(view.usable_starts, 0);
        assert_eq!(view.projected_active_value, 0.0);
    }

    #[test]
    fn stale_status_resolves_unknown_and_requires_refresh() {
        let now = Utc.with_ymd_and_hms(2026, 10, 5, 14, 0, 0).unwrap();
        let observation = FantasyStatusObservation {
            player_key: "player".to_owned(),
            status: FantasyPlayerAvailabilityStatus::Healthy,
            source: "league-export".to_owned(),
            source_url: None,
            observed_at: now - Duration::hours(7),
            fetched_at: now - Duration::hours(6),
            confidence: FantasyObservationConfidence::Confirmed,
            detail: None,
        };
        let resolved = resolve_fantasy_player_status("player", &[observation], now, 360);

        assert_eq!(resolved.freshness, FantasyObservationFreshness::Stale);
        assert_eq!(
            resolved.effective_status,
            FantasyPlayerAvailabilityStatus::Unknown
        );
        assert!(resolved.requires_pregame_refresh);
    }

    #[test]
    fn fresh_confirmed_healthy_status_is_actionable() {
        let now = Utc.with_ymd_and_hms(2026, 10, 5, 14, 0, 0).unwrap();
        let observation = FantasyStatusObservation {
            player_key: "player".to_owned(),
            status: FantasyPlayerAvailabilityStatus::Healthy,
            source: "league-export".to_owned(),
            source_url: Some("https://example.test/status".to_owned()),
            observed_at: now - Duration::minutes(15),
            fetched_at: now - Duration::minutes(10),
            confidence: FantasyObservationConfidence::Confirmed,
            detail: None,
        };
        let resolved = resolve_fantasy_player_status("player", &[observation], now, 360);

        assert_eq!(resolved.freshness, FantasyObservationFreshness::Fresh);
        assert_eq!(
            resolved.effective_status,
            FantasyPlayerAvailabilityStatus::Healthy
        );
        assert!(!resolved.requires_pregame_refresh);
    }

    #[test]
    fn taken_newline_import_matches_aliases_and_preserves_unresolved_rows() {
        let view = import_fantasy_taken_players(
            "Connor McDavid\nC. McDavid\nMystery Player\nDarren Raddysh\n",
            &identities(),
        )
        .unwrap();

        assert_eq!(view.matched_player_keys, vec!["darren-raddysh", "mcdavid"]);
        assert_eq!(view.matched, 2);
        assert_eq!(view.duplicates, 1);
        assert_eq!(view.unresolved, 1);
        assert_eq!(view.rows.len(), 4);
    }

    #[test]
    fn taken_csv_import_accepts_common_quoted_player_column() {
        let input = "Rank,Player Name,Team\n1,\"McDavid, Connor\",EDM\n2,Darren Raddysh,TBL\n";
        let pool = vec![
            FantasyDraftIdentityInput {
                player_key: "mcdavid".to_owned(),
                display_name: "Connor McDavid".to_owned(),
                aliases: vec!["McDavid, Connor".to_owned()],
            },
            identities().into_iter().nth(1).unwrap(),
        ];
        let view = import_fantasy_taken_players(input, &pool).unwrap();

        assert_eq!(view.matched, 2);
        assert_eq!(view.rows[0].row_number, 2);
        assert_eq!(view.rows[0].matched_player_key.as_deref(), Some("mcdavid"));
    }

    #[test]
    fn taken_import_never_collapses_ambiguous_canonical_names() {
        let pool = vec![
            FantasyDraftIdentityInput {
                player_key: "one".to_owned(),
                display_name: "Alex Smith".to_owned(),
                aliases: Vec::new(),
            },
            FantasyDraftIdentityInput {
                player_key: "two".to_owned(),
                display_name: "Alex Smith".to_owned(),
                aliases: Vec::new(),
            },
        ];
        let view = import_fantasy_taken_players("Alex Smith", &pool).unwrap();

        assert_eq!(view.ambiguous, 1);
        assert!(view.matched_player_keys.is_empty());
        assert_eq!(view.rows[0].candidates.len(), 2);
    }

    #[test]
    fn eligibility_csv_preserves_common_multi_position_combinations() {
        let input = "Player Name,Eligible Positions\nConnor McDavid,C/LW\nDarren Raddysh,\"D\"\n";
        let view = import_fantasy_platform_eligibility(input, &identities()).unwrap();

        assert_eq!(view.imported, 2);
        assert_eq!(
            view.rows[0].positions,
            vec![Position::Center, Position::LeftWing]
        );
        assert_eq!(view.rows[1].positions, vec![Position::Defense]);
    }

    #[test]
    fn eligibility_csv_retains_unresolved_and_invalid_rows() {
        let input = "Name,Position\nMystery Player,LW/RW\nConnor McDavid,W\n";
        let view = import_fantasy_platform_eligibility(input, &identities()).unwrap();

        assert_eq!(view.unresolved, 1);
        assert_eq!(view.invalid, 1);
        assert!(view.rows.iter().all(|row| row.matched_player_key.is_none()));
    }

    #[test]
    fn weekly_budget_uses_pacific_monday_sunday_and_hard_fence() {
        let now = Utc.with_ymd_and_hms(2026, 11, 5, 18, 0, 0).unwrap();
        let acquisitions = [1, 2, 3, 4]
            .into_iter()
            .map(|day| FantasyAcquisitionInput {
                effective_at: Utc.with_ymd_and_hms(2026, 11, day, 18, 0, 0).unwrap(),
                kind: FantasyAcquisitionKind::FreeAgentAdd,
                counts_toward_limit: true,
            })
            .collect::<Vec<_>>();
        let view = build_fantasy_week_budget(now, "America/Los_Angeles", 4, &acquisitions).unwrap();

        assert_eq!(
            view.week_start,
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );
        assert_eq!(view.week_end, NaiveDate::from_ymd_opt(2026, 11, 8).unwrap());
        assert_eq!(view.acquisitions_used, 3);
        assert_eq!(view.acquisitions_remaining, 1);
        assert!(view.can_add);

        let mut exhausted = acquisitions;
        exhausted.push(FantasyAcquisitionInput {
            effective_at: Utc.with_ymd_and_hms(2026, 11, 5, 20, 0, 0).unwrap(),
            kind: FantasyAcquisitionKind::WaiverClaim,
            counts_toward_limit: true,
        });
        let view = build_fantasy_week_budget(now, "America/Los_Angeles", 4, &exhausted).unwrap();
        assert_eq!(view.acquisitions_used, 4);
        assert!(!view.can_add);
    }

    #[test]
    fn pacific_week_boundary_respects_utc_date_and_dst() {
        let sunday_late_pacific = Utc.with_ymd_and_hms(2026, 3, 9, 6, 30, 0).unwrap();
        let monday_pacific = Utc.with_ymd_and_hms(2026, 3, 9, 8, 30, 0).unwrap();

        let sunday =
            build_fantasy_week_budget(sunday_late_pacific, "America/Los_Angeles", 4, &[]).unwrap();
        let monday =
            build_fantasy_week_budget(monday_pacific, "America/Los_Angeles", 4, &[]).unwrap();

        assert_eq!(
            sunday.week_start,
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap()
        );
        assert_eq!(
            monday.week_start,
            NaiveDate::from_ymd_opt(2026, 3, 9).unwrap()
        );
    }

    #[test]
    fn dropped_player_is_unavailable_for_exact_two_day_window() {
        let dropped_at = Utc.with_ymd_and_hms(2026, 10, 5, 16, 0, 0).unwrap();
        let waiver = fantasy_waiver_window("player", dropped_at, 2);
        assert_eq!(
            waiver.clears_at,
            Utc.with_ymd_and_hms(2026, 10, 7, 16, 0, 0).unwrap()
        );
        let before = fantasy_acquisition_availability(
            "player",
            waiver.clears_at - Duration::seconds(1),
            Some(&waiver),
        );
        let at_clear = fantasy_acquisition_availability("player", waiver.clears_at, Some(&waiver));
        assert_eq!(before.status, FantasyMarketStatus::Waivers);
        assert!(!before.usable_now);
        assert_eq!(at_clear.status, FantasyMarketStatus::FreeAgent);
        assert!(at_clear.usable_now);
    }

    fn weekly_move(add: &str, drop: &str, points: f64, available: bool) -> FantasyWeeklyMoveInput {
        let now = Utc.with_ymd_and_hms(2026, 10, 5, 16, 0, 0).unwrap();
        FantasyWeeklyMoveInput {
            add_player_key: add.to_owned(),
            add_player: add.to_owned(),
            drop_player_key: drop.to_owned(),
            drop_player: drop.to_owned(),
            availability: FantasyAcquisitionAvailability {
                player_key: add.to_owned(),
                status: if available {
                    FantasyMarketStatus::FreeAgent
                } else {
                    FantasyMarketStatus::Waivers
                },
                usable_at: if available {
                    now
                } else {
                    now + Duration::days(2)
                },
                usable_now: available,
            },
            incremental_usable_starts: 2.0,
            projected_points_from_incremental_starts: points,
            category_gap_delta: 0.0,
            future_schedule_option_value: 0.0,
            dropped_player_rest_of_week_value: 1.0,
            waiver_reacquisition_cost: 0.5,
            pickup_budget_cost: 0.25,
            uncertainty_discount: 0.25,
        }
    }

    #[test]
    fn friday_injury_reserve_blocks_proactive_move_but_allows_explicit_override() {
        let thursday = Utc.with_ymd_and_hms(2026, 11, 5, 18, 0, 0).unwrap();
        let acquisitions = [2, 3, 4]
            .into_iter()
            .map(|day| FantasyAcquisitionInput {
                effective_at: Utc.with_ymd_and_hms(2026, 11, day, 18, 0, 0).unwrap(),
                kind: FantasyAcquisitionKind::FreeAgentAdd,
                counts_toward_limit: true,
            })
            .collect::<Vec<_>>();
        let budget =
            build_fantasy_week_budget(thursday, "America/Los_Angeles", 4, &acquisitions).unwrap();
        let budget = apply_fantasy_pickup_reserve(
            budget,
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            1,
            5,
        )
        .unwrap();
        assert!(budget.can_add);
        assert!(!budget.can_proactively_add);
        assert_eq!(budget.proactive_acquisitions_remaining, 0);
        assert_eq!(budget.injury_reserve_active, 1);

        let protected = build_fantasy_weekly_pickups(
            budget.clone(),
            vec![weekly_move("add", "drop", 7.0, true)],
            5,
        )
        .unwrap();
        assert!(protected.rows.is_empty());
        assert!(protected.warnings[0].contains("reserved for an injury"));

        let injury_override = build_fantasy_weekly_pickups_with_reserve_override(
            budget,
            vec![weekly_move("add", "drop", 7.0, true)],
            5,
            true,
            false,
        )
        .unwrap();
        assert_eq!(injury_override.rows.len(), 1);
    }

    #[test]
    fn exceptional_healthy_move_can_override_reserve_but_uncertainty_blocks_it() {
        let thursday = Utc.with_ymd_and_hms(2026, 11, 5, 18, 0, 0).unwrap();
        let acquisitions = [2, 3, 4]
            .into_iter()
            .map(|day| FantasyAcquisitionInput {
                effective_at: Utc.with_ymd_and_hms(2026, 11, day, 18, 0, 0).unwrap(),
                kind: FantasyAcquisitionKind::FreeAgentAdd,
                counts_toward_limit: true,
            })
            .collect::<Vec<_>>();
        let budget = apply_fantasy_pickup_reserve(
            build_fantasy_week_budget(thursday, "America/Los_Angeles", 4, &acquisitions).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            1,
            5,
        )
        .unwrap();
        let mut candidate = weekly_move("exceptional", "drop", 8.0, true);
        candidate.incremental_usable_starts = 3.0;

        let healthy = build_fantasy_weekly_pickups_with_reserve_override(
            budget.clone(),
            vec![candidate.clone()],
            5,
            false,
            true,
        )
        .unwrap();
        assert_eq!(healthy.rows.len(), 1);
        assert!(healthy.warnings[0].contains("exceptional healthy-roster value"));

        let uncertain = build_fantasy_weekly_pickups_with_reserve_override(
            budget,
            vec![candidate],
            5,
            false,
            false,
        )
        .unwrap();
        assert!(uncertain.rows.is_empty());
        assert!(uncertain.warnings[0].contains("reserved for an injury"));
    }

    #[test]
    fn saturday_releases_unused_injury_reserve() {
        let saturday = Utc.with_ymd_and_hms(2026, 11, 7, 18, 0, 0).unwrap();
        let budget = build_fantasy_week_budget(saturday, "America/Los_Angeles", 4, &[]).unwrap();
        let budget = apply_fantasy_pickup_reserve(
            budget,
            NaiveDate::from_ymd_opt(2026, 11, 7).unwrap(),
            1,
            5,
        )
        .unwrap();
        assert_eq!(budget.injury_reserve_active, 0);
        assert_eq!(budget.proactive_acquisitions_remaining, 4);
        assert!(budget.can_proactively_add);
    }

    #[test]
    fn legacy_saved_rules_receive_pickup_reserve_defaults() {
        let mut value = serde_json::to_value(FantasyAssistantRules::configured_2026()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("injury_pickup_reserve");
        object.remove("injury_reserve_release_weekday");
        object.remove("playoff_start");
        object.remove("playoff_rounds");

        let rules: FantasyAssistantRules = serde_json::from_value(value).unwrap();
        assert_eq!(rules.injury_pickup_reserve, 1);
        assert_eq!(rules.injury_reserve_release_weekday, 5);
        assert_eq!(rules.playoff_start, None);
        assert_eq!(rules.playoff_rounds, 3);
        rules.validate().unwrap();
    }

    #[test]
    fn weekly_pickups_rank_incremental_value_and_filter_waivers() {
        let budget = build_fantasy_week_budget(
            Utc.with_ymd_and_hms(2026, 10, 5, 16, 0, 0).unwrap(),
            "America/Los_Angeles",
            4,
            &[],
        )
        .unwrap();
        let mut best = weekly_move("best", "bench", 8.0, true);
        best.future_schedule_option_value = 2.0;
        let view = build_fantasy_weekly_pickups(
            budget,
            vec![
                best,
                weekly_move("less", "bench", 5.0, true),
                weekly_move("waiver", "bench", 20.0, false),
            ],
            10,
        )
        .unwrap();

        assert_eq!(view.rows[0].add_player_key, "best");
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.blocked_waiver_candidates, 1);
        assert!(view.rows[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("saved-playoff-calendar")));
    }

    #[test]
    fn weekly_pickups_refuse_all_moves_when_budget_is_exhausted() {
        let now = Utc.with_ymd_and_hms(2026, 10, 8, 16, 0, 0).unwrap();
        let acquisitions = (0..4)
            .map(|_| FantasyAcquisitionInput {
                effective_at: now,
                kind: FantasyAcquisitionKind::FreeAgentAdd,
                counts_toward_limit: true,
            })
            .collect::<Vec<_>>();
        let budget =
            build_fantasy_week_budget(now, "America/Los_Angeles", 4, &acquisitions).unwrap();
        let view =
            build_fantasy_weekly_pickups(budget, vec![weekly_move("best", "bench", 8.0, true)], 10)
                .unwrap();

        assert!(view.rows.is_empty());
        assert!(!view.budget.can_add);
        assert!(view.warnings[0].contains("limit reached"));
    }

    #[test]
    fn draft_board_excludes_only_resolved_taken_players() {
        let pool = identities();
        let taken = import_fantasy_taken_players("Connor McDavid\nUnknown Person", &pool).unwrap();
        let board = build_fantasy_draft_board(
            "league-test",
            "20252026",
            Vec::new(),
            vec![
                draft_candidate("mcdavid", vec![Position::Center], 200.0),
                draft_candidate("darren-raddysh", vec![Position::Defense], 80.0),
            ],
            taken,
            10,
        )
        .unwrap();

        assert_eq!(board.excluded_taken_players, 1);
        assert_eq!(board.rows.len(), 1);
        assert_eq!(board.rows[0].player_key, "darren-raddysh");
        assert_eq!(board.taken_import.unresolved, 1);
        assert_eq!(board.warnings.len(), 1);
    }

    #[test]
    fn elite_quality_remains_dominant_over_calendar_fit() {
        let mut elite = draft_candidate("elite", vec![Position::Center], 200.0);
        elite.incremental_usable_starts = 0.0;
        elite.quiet_slate_games = 0.0;
        elite.schedule_collision_rate = 1.0;
        let mut calendar = draft_candidate("calendar", vec![Position::LeftWing], 120.0);
        calendar.incremental_usable_starts = 4.0;
        calendar.quiet_slate_games = 10.0;
        calendar.schedule_collision_rate = 0.0;
        calendar.playoff_incremental_usable_starts = 20.0;
        calendar.playoff_usable_value_delta = 100.0;
        let board = build_fantasy_draft_board(
            "league-test",
            "20252026",
            vec![FantasyActiveSlot {
                slot_id: "LW1".to_owned(),
                kind: FantasyActiveSlotKind::LeftWing,
            }],
            vec![calendar, elite],
            empty_taken(),
            10,
        )
        .unwrap();

        assert_eq!(board.rows[0].player_key, "elite");
        assert_eq!(board.rows[1].components.playoff_fit_value, 12.0);
        assert_eq!(
            board.rows[1].best_open_slot,
            Some(FantasyActiveSlotKind::LeftWing)
        );
    }

    #[test]
    fn draft_board_rewards_open_slot_and_multi_position_option_value() {
        let single = draft_candidate("single", vec![Position::Center], 100.0);
        let flexible = draft_candidate(
            "flexible",
            vec![Position::Center, Position::LeftWing],
            100.0,
        );
        let board = build_fantasy_draft_board(
            "league-test",
            "20252026",
            vec![FantasyActiveSlot {
                slot_id: "LW1".to_owned(),
                kind: FantasyActiveSlotKind::LeftWing,
            }],
            vec![single, flexible],
            empty_taken(),
            10,
        )
        .unwrap();

        assert_eq!(board.rows[0].player_key, "flexible");
        assert!(board.rows[0].fills_open_starter);
        assert_eq!(board.rows[0].components.multi_position_flexibility, 2.0);
    }

    #[test]
    fn morning_briefing_orders_safe_actions_and_has_stable_material_fingerprint() {
        let now = Utc.with_ymd_and_hms(2026, 10, 8, 14, 0, 0).unwrap();
        let rules = FantasyAssistantRules::configured_2026();
        let mut healthy = player("healthy", vec![Position::Center], 8.0);
        healthy.display_name = "Healthy Starter".to_owned();
        let mut injured = player("injured", vec![Position::Defense], 7.0);
        injured.display_name = "Injured Defender".to_owned();
        injured.status = FantasyPlayerAvailabilityStatus::InjuredReserve;
        let mut unknown = player("unknown", vec![Position::RightWing], 6.0);
        unknown.status = FantasyPlayerAvailabilityStatus::Unknown;
        let lineup = build_fantasy_daily_lineup(rules, vec![healthy, injured, unknown]).unwrap();
        let plan = FantasyInjuryPlanView {
            schema: FANTASY_INJURY_PLAN_SCHEMA.to_owned(),
            date: now.date_naive(),
            lineup,
            statuses: vec![FantasyResolvedPlayerStatus {
                player_key: "unknown".to_owned(),
                reported_status: FantasyPlayerAvailabilityStatus::Unknown,
                effective_status: FantasyPlayerAvailabilityStatus::Unknown,
                freshness: FantasyObservationFreshness::Missing,
                confidence: FantasyObservationConfidence::Unknown,
                source: None,
                source_url: None,
                observed_at: None,
                age_minutes: None,
                requires_pregame_refresh: true,
                detail: None,
            }],
            warnings: Vec::new(),
        };
        let budget = build_fantasy_week_budget(now, "America/Los_Angeles", 4, &[]).unwrap();
        let pickup_plan = FantasyWeeklyPickupView {
            schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
            budget: budget.clone(),
            rows: vec![FantasyWeeklyMoveRow {
                rank: 1,
                add_player_key: "available_defender".to_owned(),
                add_player: "Available Defender".to_owned(),
                drop_player_key: "bench_defender".to_owned(),
                drop_player: "Bench Defender".to_owned(),
                incremental_usable_starts: 2.0,
                projected_value_delta: 3.5,
                reasons: vec!["adds 2.0 usable starts".to_owned()],
            }],
            blocked_waiver_candidates: 0,
            warnings: Vec::new(),
        };
        let sleeper_plan = FantasySleeperBoardView {
            schema: FANTASY_SLEEPER_BOARD_SCHEMA.to_owned(),
            scoring_scheme: "league".to_owned(),
            stats_season: "20252026".to_owned(),
            baseline_season: "20242025".to_owned(),
            rows: vec![FantasySleeperRow {
                rank: 1,
                player_key: "available_defender".to_owned(),
                player: "Available Defender".to_owned(),
                nhl_team: "TBL".to_owned(),
                platform_positions: vec![Position::Defense],
                current_gp: 60,
                current_fantasy_per_game: 3.5,
                prior_fantasy_per_game: 1.5,
                score: 30.0,
                confidence: FantasySleeperConfidence::High,
                components: FantasySleeperComponents {
                    league_scoring_growth: 16.0,
                    category_rate_growth: 5.0,
                    power_play_growth: 4.0,
                    quiet_slate_value: 5.0,
                    position_flexibility: 0.0,
                    newcomer_opportunity: 0.0,
                    sample_risk_discount: 0.0,
                },
                reasons: vec!["league-scored rate growth".to_owned()],
            }],
            warnings: Vec::new(),
        };
        let first = build_fantasy_morning_briefing(
            now,
            now,
            "America/Los_Angeles",
            plan.clone(),
            None,
            budget.clone(),
            Some(pickup_plan.clone()),
            Some(sleeper_plan.clone()),
        );
        let second = build_fantasy_morning_briefing(
            now + Duration::minutes(10),
            now,
            "America/Los_Angeles",
            plan,
            None,
            budget,
            Some(pickup_plan),
            Some(sleeper_plan),
        );

        assert_eq!(first.actions[0].kind, FantasyMorningActionKind::MoveToIr);
        assert_eq!(
            first.actions[1].kind,
            FantasyMorningActionKind::RefreshStatus
        );
        assert!(first.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::Start
                && action.player_key.as_deref() == Some("healthy")
        }));
        assert!(!first.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::Start
                && action.player_key.as_deref() == Some("unknown")
        }));
        let pickup = first
            .actions
            .iter()
            .find(|action| action.kind == FantasyMorningActionKind::PickupReview)
            .expect("positive top pickup becomes a morning action");
        assert_eq!(pickup.player_key.as_deref(), Some("available_defender"));
        assert!(pickup.message.contains("+2.0 usable starts"));
        let sleeper = first
            .actions
            .iter()
            .find(|action| action.kind == FantasyMorningActionKind::SleeperWatch)
            .expect("top sleeper becomes a morning watch action");
        assert!(sleeper.message.contains("supports pickup candidate"));
        assert_eq!(first.material_fingerprint, second.material_fingerprint);
        assert_ne!(first.generated_at, second.generated_at);
    }

    #[test]
    fn morning_briefing_explicitly_protects_last_move_when_reserve_is_active() {
        let now = Utc.with_ymd_and_hms(2026, 11, 5, 18, 0, 0).unwrap();
        let rules = FantasyAssistantRules::configured_2026();
        let lineup = build_fantasy_daily_lineup(rules, Vec::new()).unwrap();
        let injury_plan = FantasyInjuryPlanView {
            schema: FANTASY_INJURY_PLAN_SCHEMA.to_owned(),
            date: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            lineup,
            statuses: Vec::new(),
            warnings: Vec::new(),
        };
        let acquisitions = [2, 3, 4]
            .into_iter()
            .map(|day| FantasyAcquisitionInput {
                effective_at: Utc.with_ymd_and_hms(2026, 11, day, 18, 0, 0).unwrap(),
                kind: FantasyAcquisitionKind::FreeAgentAdd,
                counts_toward_limit: true,
            })
            .collect::<Vec<_>>();
        let budget =
            build_fantasy_week_budget(now, "America/Los_Angeles", 4, &acquisitions).unwrap();
        let budget = apply_fantasy_pickup_reserve(
            budget,
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            1,
            5,
        )
        .unwrap();
        let pickup_plan = FantasyWeeklyPickupView {
            schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
            budget: budget.clone(),
            rows: Vec::new(),
            blocked_waiver_candidates: 0,
            warnings: vec![
                "the remaining acquisition budget is reserved for an injury replacement".to_owned(),
            ],
        };

        let briefing = build_fantasy_morning_briefing(
            now,
            now,
            "America/Los_Angeles",
            injury_plan,
            None,
            budget,
            Some(pickup_plan),
            None,
        );
        let action = briefing
            .actions
            .iter()
            .find(|action| action.kind == FantasyMorningActionKind::PickupReview)
            .unwrap();
        assert!(action.message.contains("Protect the final 1 acquisition"));
        assert!(!action.conditional);
    }

    #[test]
    fn morning_goalie_actions_distinguish_reported_roster_and_confirmed_stream() {
        use crate::view_model::{
            build_fantasy_goalie_plan, FantasyCompetitionMode, FantasyGoalieGameInput,
            FantasyGoaliePlanInput, FantasyGoaliePlanPlayerInput, FantasyGoalieStartObservation,
            FantasyGoalieStartState, FantasyMatchupStrategy,
        };

        let date = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let next_date = date.succ_opt().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 5, 15, 0, 0).unwrap();
        let game = FantasyGoalieGameInput {
            date,
            start_time_utc: Some(Utc.with_ymd_and_hms(2026, 11, 6, 2, 0, 0).unwrap()),
            opponent: "BOS".to_owned(),
            home: true,
            team_back_to_back: false,
            opponent_offense_index: 1.0,
        };
        let goalie = |key: &str, rostered: bool| FantasyGoaliePlanPlayerInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: "NYR".to_owned(),
            rostered,
            acquisition_eligible: !rostered,
            games: vec![game.clone()],
            projected_points_per_start: 5.0,
            historical_start_probability: 0.6,
            expected_save_percentage: Some(0.91),
            expected_goals_against_average: Some(2.7),
        };
        let reported = FantasyGoalieStartObservation {
            player_key: "rostered".to_owned(),
            game_date: date,
            state: FantasyGoalieStartState::ReportedStarting,
            source: "beat reporter".to_owned(),
            source_url: None,
            observed_at: now - Duration::minutes(10),
            fetched_at: now - Duration::minutes(9),
            detail: None,
        };
        let confirmed_stream = FantasyGoalieStartObservation {
            player_key: "stream_two".to_owned(),
            game_date: date,
            state: FantasyGoalieStartState::ConfirmedStarting,
            source: "team reporter".to_owned(),
            source_url: None,
            observed_at: now - Duration::minutes(5),
            fetched_at: now - Duration::minutes(4),
            detail: None,
        };
        let mut higher_volume_unconfirmed_stream = goalie("stream_one", false);
        higher_volume_unconfirmed_stream
            .games
            .push(FantasyGoalieGameInput {
                date: next_date,
                start_time_utc: Some(Utc.with_ymd_and_hms(2026, 11, 7, 2, 0, 0).unwrap()),
                opponent: "NJD".to_owned(),
                home: false,
                team_back_to_back: true,
                opponent_offense_index: 1.0,
            });
        let plan_goalies = vec![
            goalie("rostered", true),
            higher_volume_unconfirmed_stream,
            goalie("stream_two", false),
        ];
        let goalie_plan = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: next_date,
            focus_date: Some(date),
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Categories,
            goalie_slots: 2,
            minimum_goalie_appearances: 2,
            current_goalie_appearances: 0.0,
            evaluated_at: now,
            max_age_minutes: 360,
            acquisitions_remaining: 1,
            goalies: plan_goalies.clone(),
            observations: vec![reported.clone(), confirmed_stream.clone()],
            warnings: Vec::new(),
        })
        .unwrap();
        let mut morning_rules = FantasyAssistantRules::configured_2026();
        morning_rules.active_slots = BTreeMap::from([(FantasyActiveSlotKind::Goalie, 1)]);
        morning_rules.bench_slots = 0;
        let injury_plan = FantasyInjuryPlanView {
            schema: FANTASY_INJURY_PLAN_SCHEMA.to_owned(),
            date,
            lineup: build_fantasy_daily_lineup(
                morning_rules,
                vec![player("rostered", vec![Position::Goalie], 5.0)],
            )
            .unwrap(),
            statuses: Vec::new(),
            warnings: Vec::new(),
        };
        let budget = build_fantasy_week_budget(now, "America/Los_Angeles", 4, &[]).unwrap();
        let briefing = build_fantasy_morning_briefing(
            now + Duration::days(1),
            now,
            "America/Los_Angeles",
            injury_plan.clone(),
            Some(goalie_plan.clone()),
            budget.clone(),
            None,
            None,
        );

        assert!(briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::RefreshGoalie
                && action.player_key.as_deref() == Some("rostered")
                && action.conditional
        }));
        assert!(!briefing
            .actions
            .iter()
            .any(|action| action.kind == FantasyMorningActionKind::StartGoalie));
        assert!(!briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::Start
                && action.player_key.as_deref() == Some("rostered")
        }));
        assert!(briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::GoalieStreamReview
                && action.player_key.as_deref() == Some("stream_two")
                && action.message.contains("Fresh confirmed goalie stream")
                && action.message.contains("no legal drop pairing")
        }));
        assert!(briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::GoalieFallback
                && action.player_key.as_deref() == Some("stream_one")
                && action.message.contains("no legal drop pairing")
        }));
        assert_eq!(
            briefing.next_goalie_refresh_utc,
            Some(Utc.with_ymd_and_hms(2026, 11, 5, 23, 0, 0).unwrap())
        );
        assert_eq!(
            briefing.next_goalie_lock_utc,
            Some(Utc.with_ymd_and_hms(2026, 11, 6, 2, 0, 0).unwrap())
        );
        assert_eq!(briefing.goalie_refreshes_due_now, 0);
        assert_eq!(
            briefing.next_goalie_safety_check_utc,
            Some(Utc.with_ymd_and_hms(2026, 11, 5, 23, 0, 0).unwrap())
        );
        assert_eq!(briefing.goalie_safety_checks_due_now, 0);

        let acquisition_history = (0..3)
            .map(|_| FantasyAcquisitionInput {
                effective_at: now,
                kind: FantasyAcquisitionKind::FreeAgentAdd,
                counts_toward_limit: true,
            })
            .collect::<Vec<_>>();
        let final_move_budget =
            build_fantasy_week_budget(now, "America/Los_Angeles", 4, &acquisition_history).unwrap();
        let pickup_row = |key: &str, player_name: &str| FantasyWeeklyMoveRow {
            rank: 1,
            add_player_key: key.to_owned(),
            add_player: player_name.to_owned(),
            drop_player_key: "bench".to_owned(),
            drop_player: "Bench Player".to_owned(),
            incremental_usable_starts: 2.0,
            projected_value_delta: 3.5,
            reasons: vec!["adds two usable starts".to_owned()],
        };
        let mut primary_pairing = pickup_row("stream_two", "stream_two");
        primary_pairing.rank = 2;
        primary_pairing.drop_player = "Primary Goalie Cut".to_owned();
        let mut fallback_pairing = pickup_row("stream_one", "stream_one");
        fallback_pairing.rank = 3;
        fallback_pairing.drop_player = "Fallback Goalie Cut".to_owned();
        let conflicting_pickups = FantasyWeeklyPickupView {
            schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
            budget: final_move_budget.clone(),
            rows: vec![
                pickup_row("skater", "Skater Pickup"),
                primary_pairing,
                fallback_pairing,
            ],
            blocked_waiver_candidates: 0,
            warnings: Vec::new(),
        };
        let conflict_briefing = build_fantasy_morning_briefing(
            now,
            now,
            "America/Los_Angeles",
            injury_plan.clone(),
            Some(goalie_plan.clone()),
            final_move_budget.clone(),
            Some(conflicting_pickups),
            None,
        );
        assert!(conflict_briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::GoalieStreamReview
                && action.message.contains("final proactive acquisition")
                && action.message.contains("Primary Goalie Cut")
        }));
        assert!(conflict_briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::GoalieFallback
                && action.message.contains("Fallback Goalie Cut")
        }));
        assert!(conflict_briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::PickupReview
                && action.message.contains("choose only one")
                && action.message.contains("stream_two")
        }));

        let two_move_budget =
            build_fantasy_week_budget(now, "America/Los_Angeles", 4, &acquisition_history[..2])
                .unwrap();
        let multi_move_pickups = FantasyWeeklyPickupView {
            schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
            budget: two_move_budget.clone(),
            rows: vec![pickup_row("skater", "Skater Pickup")],
            blocked_waiver_candidates: 0,
            warnings: Vec::new(),
        };
        let multi_move = build_fantasy_morning_briefing(
            now,
            now,
            "America/Los_Angeles",
            injury_plan.clone(),
            Some(goalie_plan.clone()),
            two_move_budget,
            Some(multi_move_pickups),
            None,
        );
        assert!(multi_move.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::PickupReview
                && action.message.starts_with("After verifying availability")
                && !action.message.contains("choose only one")
        }));

        let duplicate_pickups = FantasyWeeklyPickupView {
            schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_owned(),
            budget: final_move_budget.clone(),
            rows: vec![pickup_row("stream_two", "stream_two")],
            blocked_waiver_candidates: 0,
            warnings: Vec::new(),
        };
        let deduplicated = build_fantasy_morning_briefing(
            now,
            now,
            "America/Los_Angeles",
            injury_plan.clone(),
            Some(goalie_plan.clone()),
            final_move_budget,
            Some(duplicate_pickups),
            None,
        );
        assert!(!deduplicated.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::PickupReview
                && action.player_key.as_deref() == Some("stream_two")
        }));
        assert!(deduplicated.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::GoalieStreamReview
                && action.player_key.as_deref() == Some("stream_two")
                && action.message.contains("weekly optimizer pairs the add")
        }));

        let late = Utc.with_ymd_and_hms(2026, 11, 6, 1, 40, 0).unwrap();
        let late_confirmation = |player_key: &str| FantasyGoalieStartObservation {
            player_key: player_key.to_owned(),
            game_date: date,
            state: FantasyGoalieStartState::ConfirmedStarting,
            source: "team reporter".to_owned(),
            source_url: None,
            observed_at: late - Duration::minutes(5),
            fetched_at: late - Duration::minutes(4),
            detail: None,
        };
        let late_plan = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
            league: "League".to_owned(),
            team: "Dawgs".to_owned(),
            week_start: date,
            week_end: next_date,
            focus_date: Some(date),
            strategy: FantasyMatchupStrategy::Balanced,
            competition_mode: FantasyCompetitionMode::Categories,
            goalie_slots: 2,
            minimum_goalie_appearances: 2,
            current_goalie_appearances: 0.0,
            evaluated_at: late,
            max_age_minutes: 360,
            acquisitions_remaining: 1,
            goalies: plan_goalies,
            observations: vec![
                late_confirmation("rostered"),
                late_confirmation("stream_two"),
            ],
            warnings: Vec::new(),
        })
        .unwrap();
        let late_briefing = build_fantasy_morning_briefing(
            late,
            late,
            "America/Los_Angeles",
            injury_plan,
            Some(late_plan),
            budget,
            None,
            None,
        );
        assert!(late_briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::RefreshGoalie
                && action.player_key.as_deref() == Some("rostered")
                && action.message.contains("Final safety check now")
        }));
        assert!(late_briefing.actions.iter().any(|action| {
            action.kind == FantasyMorningActionKind::GoalieStreamReview
                && action.player_key.as_deref() == Some("stream_two")
                && action.message.contains("Final verification due now")
        }));
        assert!(late_briefing.goalie_safety_checks_due_now >= 2);
    }

    #[test]
    fn sleeper_board_rewards_rate_growth_and_exposes_sample_risk() {
        let input = |key: &str, gp: u32, current: f64, prior: f64| FantasySleeperInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: "TBL".to_owned(),
            platform_positions: vec![Position::Defense],
            current_gp: gp,
            current_fantasy_per_game: current,
            prior_gp: 60,
            prior_player_existed: true,
            prior_rate_available: true,
            prior_fantasy_per_game: prior,
            current_shots_per_game: 2.0,
            prior_shots_per_game: 1.0,
            current_hits_per_game: Some(1.5),
            prior_hits_per_game: Some(1.0),
            current_blocks_per_game: Some(1.7),
            prior_blocks_per_game: Some(1.0),
            current_pp_points_per_game: 0.30,
            prior_pp_points_per_game: 0.10,
            quiet_slate_rate: 0.25,
        };
        let board = build_fantasy_sleeper_board(
            "league",
            "20252026",
            "20242025",
            vec![
                input("breakout", 55, 4.0, 2.0),
                input("steady", 70, 2.5, 2.4),
                input("too_small", 9, 8.0, 1.0),
                input("risky", 12, 4.0, 2.0),
            ],
            10,
        );

        assert_eq!(board.rows[0].player_key, "breakout");
        assert!(!board.rows.iter().any(|row| row.player_key == "too_small"));
        let risky = board
            .rows
            .iter()
            .find(|row| row.player_key == "risky")
            .unwrap();
        assert!(risky.components.sample_risk_discount > 0.0);
        assert_eq!(risky.confidence, FantasySleeperConfidence::Low);
    }
}
