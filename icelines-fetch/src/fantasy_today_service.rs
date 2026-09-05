//! Read-only local assembly for the league-aware fantasy daily cockpit.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context as _;
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use icelines_core::{
    apply_fantasy_pickup_reserve, build_fantasy_bench_coverage, build_fantasy_daily_lineup,
    build_fantasy_goalie_plan, build_fantasy_matchup_strategy, build_fantasy_morning_briefing,
    build_fantasy_today, build_fantasy_today_v2, build_fantasy_week_budget,
    build_fantasy_weekly_pickups_with_reserve_override, fantasy_acquisition_availability,
    goalie_scheme_stats_from_view, model::Season, name::normalize_name,
    resolve_fantasy_player_status, scheme::compute_goalie_fantasy_score, score_fantasy_roster,
    season_stats::SeasonType, FantasyAcquisitionInput, FantasyActiveSlotKind,
    FantasyAssistantRules, FantasyBenchCoverageInput, FantasyBenchCoveragePlayerInput,
    FantasyCompetitionMode, FantasyDailyTransactionCandidate, FantasyGoalieGameInput,
    FantasyGoaliePlanInput, FantasyGoaliePlanPlayerInput, FantasyInjuryPlanView,
    FantasyLineupPlayerInput, FantasyMatchupPointsSnapshotInput, FantasyMatchupStrategy,
    FantasyMatchupStrategyInput, FantasyMatchupStrategyPlayerInput,
    FantasyMatchupStrategyTeamInput, FantasyPickupSequenceContext, FantasyPickupSequenceInput,
    FantasyPickupSequencePlayerInput, FantasyPickupSequenceView, FantasyPickupTransitionInput,
    FantasyPlatformSnapshot, FantasyPlayerAvailabilityStatus, FantasyTodayContext,
    FantasyTodayEvidenceRow, FantasyTodayInput, FantasyTodayMatchupInput, FantasyTodayReadinessRow,
    FantasyTodayState, FantasyTodayV2View, FantasyWeeklyMoveInput, Scheme,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyTodayCandidatePolicy {
    pub candidate_limit: usize,
    pub max_elapsed_ms: u64,
}

impl Default for FantasyTodayCandidatePolicy {
    fn default() -> Self {
        Self {
            candidate_limit: 12,
            max_elapsed_ms: 250,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FantasyTodayAssemblyRequest {
    pub database_path: PathBuf,
    pub data_root: PathBuf,
    pub snapshots_root: PathBuf,
    pub schemes_root: PathBuf,
    pub league: Option<String>,
    pub team: Option<String>,
    pub stats_season: String,
    pub season_type: SeasonType,
    pub schedule_season: u32,
    pub evaluated_at_utc: DateTime<Utc>,
    pub local_date: Option<NaiveDate>,
    pub status_max_age_minutes: i64,
    pub current_goalie_appearances: f64,
    pub candidate_policy: FantasyTodayCandidatePolicy,
    pub week_plan_policy: FantasyWeekPlanPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyWeekPlanPolicy {
    pub candidate_limit: usize,
    pub max_moves: u8,
    pub beam_width: usize,
    pub alternative_limit: usize,
}

impl Default for FantasyWeekPlanPolicy {
    fn default() -> Self {
        Self {
            candidate_limit: 8,
            max_moves: 2,
            beam_width: 12,
            alternative_limit: 3,
        }
    }
}

impl FantasyTodayAssemblyRequest {
    pub fn from_default_paths(
        league: Option<String>,
        team: Option<String>,
        stats_season: String,
        schedule_season: u32,
        evaluated_at_utc: DateTime<Utc>,
    ) -> Result<Self, FantasyTodayAssemblyError> {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .ok_or(FantasyTodayAssemblyError::HomeUnavailable)?;
        let root = home.join(".icelines");
        Ok(Self {
            database_path: root.join("icelines.db"),
            data_root: std::env::var_os("ICELINES_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("data")),
            snapshots_root: root.join("snapshots"),
            schemes_root: root.join("schemes"),
            league,
            team,
            stats_season,
            season_type: SeasonType::Regular,
            schedule_season,
            evaluated_at_utc,
            local_date: None,
            status_max_age_minutes: 180,
            current_goalie_appearances: 0.0,
            candidate_policy: FantasyTodayCandidatePolicy::default(),
            week_plan_policy: FantasyWeekPlanPolicy::default(),
        })
    }
}

#[derive(Debug, Error)]
pub enum FantasyTodayAssemblyError {
    #[error("cannot determine the IceLines user-data directory")]
    HomeUnavailable,
    #[error("fantasy database does not exist at {0}; run `icelines fantasy league-create` first")]
    DatabaseMissing(String),
    #[error("league '{0}' was not found; run `icelines fantasy league-list`")]
    LeagueMissing(String),
    #[error("no active fantasy league; run `icelines fantasy league-use <name>`")]
    ActiveLeagueMissing,
    #[error("team '{0}' was not found in the selected league")]
    TeamMissing(String),
    #[error("no user team is marked; run `icelines fantasy team-use <name>`")]
    UserTeamMissing,
    #[error("fantasy assistant rules are missing for league '{0}'; run `icelines fantasy assistant-setup`")]
    RulesMissing(String),
    #[error("required local cache is missing or incomplete: {0}")]
    CacheMissing(String),
    #[error("unsupported IANA timezone '{0}'")]
    InvalidTimezone(String),
    #[error("invalid assembly request: {0}")]
    InvalidRequest(String),
    #[error("daily fantasy evidence unavailable: {0}")]
    Evidence(String),
}

impl FantasyTodayAssemblyError {
    pub fn recovery_command(&self) -> Option<&'static str> {
        match self {
            Self::DatabaseMissing(_) => Some("icelines fantasy league-create"),
            Self::LeagueMissing(_) | Self::ActiveLeagueMissing => {
                Some("icelines fantasy league-list")
            }
            Self::TeamMissing(_) | Self::UserTeamMissing => Some("icelines fantasy team-list"),
            Self::RulesMissing(_) => Some("icelines fantasy assistant-setup"),
            Self::CacheMissing(_) => Some("icelines fantasy schedule-edge --refresh"),
            Self::Evidence(_) => Some("icelines fantasy readiness"),
            Self::HomeUnavailable | Self::InvalidTimezone(_) | Self::InvalidRequest(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasySavedMatchupRejectionReason {
    MissingMatchup,
    FutureSnapshot,
    StaleSnapshot,
    WrongWeek,
    WrongTeams,
    MissingThroughDate,
    ThroughDateOutsideWeek,
    ThroughDateAfterEvaluation,
    NonFiniteOrNegativePoints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasySavedMatchupRejection {
    pub captured_at: DateTime<Utc>,
    pub reason: FantasySavedMatchupRejectionReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySavedPointsMatchup {
    pub points: FantasyMatchupPointsSnapshotInput,
    pub captured_at: DateTime<Utc>,
    pub platform: String,
    pub source_url: Option<String>,
    pub user_goalie_appearances: Option<u8>,
    pub opponent_goalie_appearances: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySavedMatchupSelection {
    pub selected: Option<FantasySavedPointsMatchup>,
    pub rejections: Vec<FantasySavedMatchupRejection>,
}

#[allow(clippy::too_many_arguments)]
pub fn select_saved_points_matchup(
    snapshots: impl IntoIterator<Item = FantasyPlatformSnapshot>,
    user_team: &str,
    opponent: &str,
    week_start: NaiveDate,
    week_end: NaiveDate,
    evaluated_at_utc: DateTime<Utc>,
    local_date: NaiveDate,
    max_age_minutes: i64,
) -> FantasySavedMatchupSelection {
    let mut snapshots = snapshots.into_iter().collect::<Vec<_>>();
    snapshots.sort_by_key(|snapshot| std::cmp::Reverse(snapshot.captured_at));
    let mut rejections = Vec::new();
    for snapshot in snapshots {
        let reject = |reason| FantasySavedMatchupRejection {
            captured_at: snapshot.captured_at,
            reason,
        };
        if snapshot.captured_at > evaluated_at_utc {
            rejections.push(reject(FantasySavedMatchupRejectionReason::FutureSnapshot));
            continue;
        }
        if evaluated_at_utc - snapshot.captured_at > Duration::minutes(max_age_minutes) {
            rejections.push(reject(FantasySavedMatchupRejectionReason::StaleSnapshot));
            continue;
        }
        let Some(matchup) = snapshot.matchup.as_ref() else {
            rejections.push(reject(FantasySavedMatchupRejectionReason::MissingMatchup));
            continue;
        };
        if matchup.week_start != week_start {
            rejections.push(reject(FantasySavedMatchupRejectionReason::WrongWeek));
            continue;
        }
        let orientation = if matchup.team.eq_ignore_ascii_case(user_team)
            && matchup.opponent.eq_ignore_ascii_case(opponent)
        {
            Some((
                matchup.team_points,
                matchup.opponent_points,
                matchup.team_goalie_appearances,
                matchup.opponent_goalie_appearances,
            ))
        } else if matchup.opponent.eq_ignore_ascii_case(user_team)
            && matchup.team.eq_ignore_ascii_case(opponent)
        {
            Some((
                matchup.opponent_points,
                matchup.team_points,
                matchup.opponent_goalie_appearances,
                matchup.team_goalie_appearances,
            ))
        } else {
            None
        };
        let Some((user_points, opponent_points, user_goalies, opponent_goalies)) = orientation
        else {
            rejections.push(reject(FantasySavedMatchupRejectionReason::WrongTeams));
            continue;
        };
        let Some(through_date) = matchup.through else {
            rejections.push(reject(
                FantasySavedMatchupRejectionReason::MissingThroughDate,
            ));
            continue;
        };
        if through_date < week_start || through_date > week_end {
            rejections.push(reject(
                FantasySavedMatchupRejectionReason::ThroughDateOutsideWeek,
            ));
            continue;
        }
        if through_date > local_date {
            rejections.push(reject(
                FantasySavedMatchupRejectionReason::ThroughDateAfterEvaluation,
            ));
            continue;
        }
        if !user_points.is_finite()
            || !opponent_points.is_finite()
            || user_points < 0.0
            || opponent_points < 0.0
        {
            rejections.push(reject(
                FantasySavedMatchupRejectionReason::NonFiniteOrNegativePoints,
            ));
            continue;
        }
        let source = format!(
            "{} snapshot captured {}",
            snapshot.platform, snapshot.captured_at
        );
        return FantasySavedMatchupSelection {
            selected: Some(FantasySavedPointsMatchup {
                points: FantasyMatchupPointsSnapshotInput {
                    through_date,
                    user_points,
                    opponent_points,
                    source,
                },
                captured_at: snapshot.captured_at,
                platform: snapshot.platform,
                source_url: snapshot.source_url,
                user_goalie_appearances: user_goalies,
                opponent_goalie_appearances: opponent_goalies,
            }),
            rejections,
        };
    }
    FantasySavedMatchupSelection {
        selected: None,
        rejections,
    }
}

/// Assemble the complete daily contract from local, immutable evidence.
///
/// The implementation is introduced in the next build-green slice; keeping
/// the public boundary here lets snapshot-selection tests land independently.
pub fn assemble_fantasy_today(
    request: FantasyTodayAssemblyRequest,
) -> Result<FantasyTodayV2View, FantasyTodayAssemblyError> {
    if request.status_max_age_minutes < 0 {
        return Err(FantasyTodayAssemblyError::InvalidRequest(
            "status_max_age_minutes cannot be negative".to_owned(),
        ));
    }
    if !request.current_goalie_appearances.is_finite() || request.current_goalie_appearances < 0.0 {
        return Err(FantasyTodayAssemblyError::InvalidRequest(
            "current_goalie_appearances must be finite and non-negative".to_owned(),
        ));
    }
    if !request.database_path.is_file() {
        return Err(FantasyTodayAssemblyError::DatabaseMissing(
            request.database_path.display().to_string(),
        ));
    }
    let league = request.league.clone();
    let team = request.team.clone();
    assemble_fantasy_today_inner(request).map_err(|error| {
        let message = error.to_string();
        if message.starts_with("no active fantasy league") {
            FantasyTodayAssemblyError::ActiveLeagueMissing
        } else if message.starts_with("no user team is marked") {
            FantasyTodayAssemblyError::UserTeamMissing
        } else if message.starts_with("fantasy assistant rules are missing") {
            FantasyTodayAssemblyError::RulesMissing(league.unwrap_or_default())
        } else if message.starts_with("load cached NHL schedule")
            || message.starts_with("cached NHL schedule is empty")
        {
            FantasyTodayAssemblyError::CacheMissing(message)
        } else if message.starts_with("unsupported IANA timezone") {
            FantasyTodayAssemblyError::InvalidTimezone(message)
        } else if message.starts_with("league '") {
            FantasyTodayAssemblyError::LeagueMissing(league.unwrap_or_default())
        } else if message.starts_with("team '") {
            FantasyTodayAssemblyError::TeamMissing(team.unwrap_or_default())
        } else {
            FantasyTodayAssemblyError::Evidence(message)
        }
    })
}

pub fn competition_mode_supports_saved_matchup(mode: FantasyCompetitionMode) -> bool {
    mode == FantasyCompetitionMode::Points
}

fn assemble_fantasy_today_inner(
    request: FantasyTodayAssemblyRequest,
) -> anyhow::Result<FantasyTodayV2View> {
    if request.status_max_age_minutes < 0 {
        anyhow::bail!("status_max_age_minutes cannot be negative");
    }
    if !request.current_goalie_appearances.is_finite() || request.current_goalie_appearances < 0.0 {
        anyhow::bail!("current_goalie_appearances must be finite and non-negative");
    }
    let stats_season = Season(
        request
            .stats_season
            .parse::<u32>()
            .with_context(|| format!("invalid stats season '{}'", request.stats_season))?,
    );
    if !request.database_path.is_file() {
        anyhow::bail!(
            "fantasy database does not exist at {}",
            request.database_path.display()
        );
    }
    let db =
        crate::fantasy_db::FantasyDb::open_existing_read_only_path(request.database_path.clone())?;
    let league = if let Some(name) = &request.league {
        db.list_leagues()?
            .into_iter()
            .find(|row| row.name == *name || row.id == *name)
            .with_context(|| format!("league '{name}' was not found"))?
    } else {
        db.get_active_league()?
            .context("no active fantasy league")?
    };
    let team = if let Some(name) = &request.team {
        db.list_teams(&league.id)?
            .into_iter()
            .find(|row| row.name == *name || row.id == *name)
            .with_context(|| format!("team '{name}' was not found in {}", league.name))?
    } else {
        db.get_user_team(&league.id)?
            .context("no user team is marked")?
    };
    let rules = db.get_assistant_rules(&league.id)?.with_context(|| {
        format!(
            "fantasy assistant rules are missing for league '{}'",
            league.name
        )
    })?;
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let local_date = request.local_date.unwrap_or_else(|| {
        request
            .evaluated_at_utc
            .with_timezone(&timezone)
            .date_naive()
    });
    let evaluated_local_date = request
        .evaluated_at_utc
        .with_timezone(&timezone)
        .date_naive();
    if local_date != evaluated_local_date {
        anyhow::bail!(
            "local date {local_date} does not match evaluation date {evaluated_local_date} in {}",
            rules.timezone
        );
    }
    let week_start =
        local_date - Duration::days(i64::from(local_date.weekday().num_days_from_monday()));
    let week_end = week_start + Duration::days(6);

    let data_store = crate::datastore::DataStore::open(&request.data_root)?;
    let schedule = data_store
        .load_schedule(Season(request.schedule_season))
        .with_context(|| {
            format!(
                "load cached NHL schedule for {}; run `icelines fantasy schedule-edge --season {} --refresh`",
                request.schedule_season, request.schedule_season
            )
        })?;
    if schedule.is_empty() {
        anyhow::bail!("cached NHL schedule is empty");
    }

    let snapshot_store = crate::snapshot::SnapshotStore::new(&request.snapshots_root);
    let outcome =
        crate::stats_loader::load_into_repo(stats_season, request.season_type, &snapshot_store)?;
    let skaters = outcome
        .repo
        .skaters(stats_season, request.season_type)
        .collect::<Vec<_>>();
    let goalies = outcome
        .repo
        .goalies(stats_season, request.season_type)
        .collect::<Vec<_>>();
    let scheme = load_scheme(&league.scheme, &request.schemes_root)?;
    let (current_teams, current_rosters_ready, current_rosters_detail) =
        match load_current_player_team_map(&snapshot_store, Season(request.schedule_season)) {
            Ok(teams) => (
                teams,
                true,
                format!("cached current rosters loaded for {}", request.schedule_season),
            ),
            Err(error) => (
                HashMap::new(),
                false,
                format!(
                    "current-roster cache is incomplete; historical stat-team fallback is provisional: {error}"
                ),
            ),
        };
    let team_dates = schedule_team_dates(&schedule);
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    let views = skaters
        .iter()
        .chain(&goalies)
        .map(|view| (view.identity.name_normalized.clone(), view))
        .collect::<HashMap<_, _>>();
    let all_scores = score_fantasy_roster(
        &views.keys().cloned().collect::<Vec<_>>(),
        &skaters,
        &goalies,
        &scheme,
    )
    .into_iter()
    .map(|row| {
        (
            row.player.identity.name_normalized.clone(),
            f64::from(row.score),
        )
    })
    .collect::<HashMap<_, _>>();

    let observations = db.list_latest_status_observations(&league.id)?;
    let roster = db.list_roster(&team.id)?;
    let statuses = roster
        .iter()
        .map(|key| {
            resolve_fantasy_player_status(
                key.clone(),
                &observations,
                request.evaluated_at_utc,
                request.status_max_age_minutes,
            )
        })
        .collect::<Vec<_>>();
    let status_by_key = statuses
        .iter()
        .map(|row| (row.player_key.clone(), row.effective_status))
        .collect::<HashMap<_, _>>();
    let mut unresolved = Vec::new();
    let lineup_players = roster
        .iter()
        .filter_map(|key| {
            let Some(view) = views.get(key).copied() else {
                unresolved.push(key.clone());
                return None;
            };
            let nhl_team = current_teams
                .get(key)
                .cloned()
                .unwrap_or_else(|| view.team_display().to_owned());
            Some(FantasyLineupPlayerInput {
                player_key: key.clone(),
                display_name: view.full_name().to_owned(),
                nhl_team: nhl_team.clone(),
                platform_positions: eligibility
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| vec![view.position()]),
                projected_value: value_per_game(*view, &all_scores),
                has_game: team_dates
                    .get(&nhl_team)
                    .is_some_and(|dates| dates.contains(&local_date)),
                status: status_by_key
                    .get(key)
                    .copied()
                    .unwrap_or(FantasyPlayerAvailabilityStatus::Unknown),
                locked_slot: None,
                locked: false,
            })
        })
        .collect::<Vec<_>>();
    let lineup =
        build_fantasy_daily_lineup(rules.clone(), lineup_players).map_err(anyhow::Error::msg)?;
    let status_refreshes = statuses
        .iter()
        .filter(|row| row.requires_pregame_refresh)
        .count();
    let mut injury_warnings = lineup.warnings.clone();
    if status_refreshes > 0 {
        injury_warnings.push(format!(
            "{status_refreshes} roster status observation(s) require refresh"
        ));
    }
    if !unresolved.is_empty() {
        injury_warnings.push(format!(
            "{} roster player(s) were absent from the {} stats pool",
            unresolved.len(),
            request.stats_season
        ));
    }
    let injury_plan = FantasyInjuryPlanView {
        schema: icelines_core::view_model::FANTASY_INJURY_PLAN_SCHEMA.to_owned(),
        date: local_date,
        lineup,
        statuses,
        warnings: injury_warnings,
    };

    let budget = load_week_budget(&db, &league.id, &rules, request.evaluated_at_utc)?;
    let (transaction_candidate, candidate_state, candidate_recovery_command) =
        build_bounded_transaction_candidate(
            &db,
            &league.id,
            &roster,
            &rules,
            &budget,
            &skaters,
            &goalies,
            &all_scores,
            &eligibility,
            &current_teams,
            &schedule,
            local_date,
            week_end,
            request.evaluated_at_utc,
            &request.candidate_policy,
        )?;
    let matchups = db.list_matchups(&league.id, week_start)?;
    let opponent_name = matchups.iter().find_map(|row| {
        if row.home_team == team.name {
            row.away_team.clone()
        } else if row.away_team.as_deref() == Some(team.name.as_str()) {
            Some(row.home_team.clone())
        } else {
            None
        }
    });
    let opponent = opponent_name
        .as_ref()
        .and_then(|name| db.get_team_by_name(&league.id, name).ok().flatten());
    let competition = db.get_competition_rules(&league.id)?;
    let snapshots = db.list_platform_snapshots(&league.id, 50)?;
    let saved_matchup = opponent.as_ref().map(|opponent| {
        select_saved_points_matchup(
            snapshots,
            &team.name,
            &opponent.name,
            week_start,
            week_end,
            request.evaluated_at_utc,
            local_date,
            request.status_max_age_minutes,
        )
    });
    let current_goalie_appearances = if request.current_goalie_appearances > 0.0 {
        request.current_goalie_appearances
    } else {
        saved_matchup
            .as_ref()
            .and_then(|selection| selection.selected.as_ref())
            .and_then(|selected| selected.user_goalie_appearances)
            .map(f64::from)
            .unwrap_or(0.0)
    };
    let goalie_plan = build_goalie_plan(
        &db,
        &league,
        &team,
        &rules,
        &scheme,
        &skaters,
        &goalies,
        &current_teams,
        &schedule,
        week_start,
        week_end,
        local_date,
        current_goalie_appearances,
        request.status_max_age_minutes,
        request.evaluated_at_utc,
        budget.proactive_acquisitions_remaining,
    )?;
    let generated_at = Utc::now();
    let morning = build_fantasy_morning_briefing(
        generated_at,
        request.evaluated_at_utc,
        rules.timezone.clone(),
        injury_plan,
        Some(goalie_plan),
        budget.clone(),
        None,
        None,
    );
    let bench_coverage = build_bench_coverage(&team.name, &morning, &schedule)?;

    let matchup_input = if competition.mode == FantasyCompetitionMode::Points {
        opponent
            .as_ref()
            .map(|opponent| {
                build_points_matchup(
                    &db,
                    &league.name,
                    &league.scheme,
                    &team,
                    opponent,
                    &rules,
                    &views,
                    &all_scores,
                    &eligibility,
                    &current_teams,
                    &team_dates,
                    &observations,
                    week_start,
                    week_end,
                    request.evaluated_at_utc,
                    request.status_max_age_minutes,
                    saved_matchup
                        .as_ref()
                        .and_then(|selection| selection.selected.as_ref())
                        .map(|selected| selected.points.clone()),
                )
            })
            .transpose()?
            .map(|view| FantasyTodayMatchupInput::Points(Box::new(view)))
    } else {
        None
    };

    let mut readiness = vec![
        FantasyTodayReadinessRow {
            workflow: "schedule".to_owned(),
            state: FantasyTodayState::Ready,
            reason_code: None,
            message: format!(
                "cached official NHL schedule loaded for {}",
                request.schedule_season
            ),
            recovery_command: Some(format!(
                "icelines fantasy schedule-edge --season {} --refresh",
                request.schedule_season
            )),
        },
        FantasyTodayReadinessRow {
            workflow: "rules_roster".to_owned(),
            state: FantasyTodayState::Ready,
            reason_code: None,
            message: "saved league rules, user team, roster, and eligibility loaded read-only"
                .to_owned(),
            recovery_command: Some("icelines fantasy snapshot-yahoo".to_owned()),
        },
        FantasyTodayReadinessRow {
            workflow: "player_rates".to_owned(),
            state: FantasyTodayState::Ready,
            reason_code: None,
            message: format!(
                "sealed player-rate sample loaded for {}",
                request.stats_season
            ),
            recovery_command: Some(format!(
                "icelines fetch all --season {}",
                request.stats_season
            )),
        },
        FantasyTodayReadinessRow {
            workflow: "current_rosters".to_owned(),
            state: if current_rosters_ready {
                FantasyTodayState::Ready
            } else {
                FantasyTodayState::Provisional
            },
            reason_code: (!current_rosters_ready)
                .then(|| "current_roster_cache_incomplete".to_owned()),
            message: current_rosters_detail.clone(),
            recovery_command: (!current_rosters_ready).then(|| {
                format!(
                    "icelines fetch rosters --season {}",
                    request.schedule_season
                )
            }),
        },
        FantasyTodayReadinessRow {
            workflow: "player_status".to_owned(),
            state: if status_refreshes == 0 {
                FantasyTodayState::Ready
            } else {
                FantasyTodayState::Provisional
            },
            reason_code: (status_refreshes > 0).then(|| "status_refresh_required".to_owned()),
            message: if status_refreshes == 0 {
                "saved status evidence is current for displayed decisions".to_owned()
            } else {
                format!("{status_refreshes} roster status observation(s) require refresh")
            },
            recovery_command: (status_refreshes > 0)
                .then(|| "icelines fantasy status-show".to_owned()),
        },
    ];
    let matchup_state = if competition.mode == FantasyCompetitionMode::Categories {
        FantasyTodayReadinessRow {
            workflow: "matchup".to_owned(),
            state: FantasyTodayState::Provisional,
            reason_code: Some("category_components_unavailable".to_owned()),
            message: "saved provider snapshots do not contain per-category matchup values"
                .to_owned(),
            recovery_command: Some(
                "icelines fantasy matchup-plan --category-snapshot <path>".to_owned(),
            ),
        }
    } else if opponent.is_none() {
        FantasyTodayReadinessRow {
            workflow: "matchup".to_owned(),
            state: FantasyTodayState::Provisional,
            reason_code: Some("opponent_unavailable".to_owned()),
            message: format!("no saved opponent for {} in week {}", team.name, week_start),
            recovery_command: Some("icelines fantasy matchup-set".to_owned()),
        }
    } else if saved_matchup
        .as_ref()
        .is_some_and(|selection| selection.selected.is_some())
    {
        FantasyTodayReadinessRow {
            workflow: "matchup".to_owned(),
            state: FantasyTodayState::Ready,
            reason_code: None,
            message: "newest coherent saved points matchup composed".to_owned(),
            recovery_command: Some("icelines fantasy snapshot-show".to_owned()),
        }
    } else {
        FantasyTodayReadinessRow {
            workflow: "matchup".to_owned(),
            state: FantasyTodayState::Provisional,
            reason_code: Some("saved_matchup_unavailable".to_owned()),
            message:
                "opponent projection is available but no coherent saved point totals were found"
                    .to_owned(),
            recovery_command: Some("icelines fantasy snapshot-yahoo".to_owned()),
        }
    };
    readiness.push(matchup_state);
    let mut evidence = vec![
        FantasyTodayEvidenceRow {
            source_family: "nhl_schedule_cache".to_owned(),
            authority_scope: "NHL schedule and game timing".to_owned(),
            state: FantasyTodayState::Ready,
            observed_at: None,
            fetched_at: None,
            detail: format!(
                "cached regular-season schedule for {}",
                request.schedule_season
            ),
            recovery_command: Some(format!(
                "icelines fantasy schedule-edge --season {} --refresh",
                request.schedule_season
            )),
        },
        FantasyTodayEvidenceRow {
            source_family: "fantasy_local_state".to_owned(),
            authority_scope: "league rules, roster, eligibility, and transaction ledger".to_owned(),
            state: FantasyTodayState::Ready,
            observed_at: Some(request.evaluated_at_utc.to_rfc3339()),
            fetched_at: None,
            detail: "opened the existing fantasy database immutable/read-only".to_owned(),
            recovery_command: Some("icelines fantasy snapshot-show".to_owned()),
        },
        FantasyTodayEvidenceRow {
            source_family: "sealed_stats".to_owned(),
            authority_scope: "player scoring rates".to_owned(),
            state: FantasyTodayState::Ready,
            observed_at: None,
            fetched_at: None,
            detail: format!("selected NHL stats sample {}", request.stats_season),
            recovery_command: Some(format!(
                "icelines fetch all --season {}",
                request.stats_season
            )),
        },
        FantasyTodayEvidenceRow {
            source_family: "nhl_roster_cache".to_owned(),
            authority_scope: "current player-to-team schedule joins".to_owned(),
            state: if current_rosters_ready {
                FantasyTodayState::Ready
            } else {
                FantasyTodayState::Provisional
            },
            observed_at: None,
            fetched_at: None,
            detail: current_rosters_detail,
            recovery_command: (!current_rosters_ready).then(|| {
                format!(
                    "icelines fetch rosters --season {}",
                    request.schedule_season
                )
            }),
        },
    ];
    if let Some(selection) = &saved_matchup {
        if let Some(selected) = &selection.selected {
            let rejected_reasons = selection
                .rejections
                .iter()
                .filter_map(|row| {
                    serde_json::to_value(&row.reason)
                        .ok()
                        .and_then(|value| value.as_str().map(str::to_owned))
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ");
            evidence.push(FantasyTodayEvidenceRow {
                source_family: selected.platform.clone(),
                authority_scope: "fantasy matchup totals and goalie appearances".to_owned(),
                state: FantasyTodayState::Ready,
                observed_at: Some(selected.points.through_date.to_string()),
                fetched_at: Some(selected.captured_at.to_rfc3339()),
                detail: format!(
                    "selected coherent snapshot; {} newer/inapplicable snapshot(s) rejected{}",
                    selection.rejections.len(),
                    if rejected_reasons.is_empty() {
                        String::new()
                    } else {
                        format!(" ({rejected_reasons})")
                    }
                ),
                recovery_command: Some("icelines fantasy snapshot-show".to_owned()),
            });
        }
    }
    let week_plan = build_week_plan(
        &db,
        &league,
        &team,
        &rules,
        &budget,
        &skaters,
        &goalies,
        &all_scores,
        &eligibility,
        &current_teams,
        &schedule,
        &observations,
        &roster,
        &competition,
        week_start,
        week_end,
        local_date,
        request.evaluated_at_utc,
        request.status_max_age_minutes,
        &request.stats_season,
        request.season_type,
        &request.week_plan_policy,
        &readiness,
        &evidence,
    )?;
    let today = build_fantasy_today(FantasyTodayInput {
        context: FantasyTodayContext {
            league_id: league.id,
            league_name: league.name,
            fantasy_team_id: team.id,
            fantasy_team_name: team.name,
            stats_season: request.stats_season,
            season_type: request.season_type,
            competition_mode: competition.mode.label().to_owned(),
            date: local_date,
            week_start,
            week_end,
            timezone: rules.timezone,
            generated_at,
            evaluated_at: request.evaluated_at_utc,
        },
        morning,
        matchup: matchup_input,
        bench_coverage,
        provider_status: None,
        readiness,
        evidence: std::mem::take(&mut evidence),
    });
    let mut view = build_fantasy_today_v2(
        today,
        transaction_candidate,
        candidate_state,
        candidate_recovery_command,
    );
    view.week_plan = Some(week_plan);
    Ok(view)
}

#[allow(clippy::too_many_arguments)]
fn build_week_plan(
    db: &crate::fantasy_db::FantasyDb,
    league: &crate::fantasy_db::LeagueRow,
    team: &crate::fantasy_db::TeamRow,
    rules: &FantasyAssistantRules,
    budget: &icelines_core::FantasyWeekBudgetView,
    skaters: &[icelines_core::stats_repository::PlayerView<'_>],
    goalies: &[icelines_core::stats_repository::PlayerView<'_>],
    scores: &HashMap<String, f64>,
    eligibility: &HashMap<String, Vec<icelines_core::model::Position>>,
    current_teams: &HashMap<String, String>,
    schedule: &[crate::nhl_api::ScheduledGame],
    observations: &[icelines_core::FantasyStatusObservation],
    roster: &[String],
    competition: &icelines_core::FantasyCompetitionRules,
    week_start: NaiveDate,
    week_end: NaiveDate,
    local_date: NaiveDate,
    evaluated_at: DateTime<Utc>,
    status_max_age_minutes: i64,
    stats_season: &str,
    season_type: SeasonType,
    policy: &FantasyWeekPlanPolicy,
    readiness: &[FantasyTodayReadinessRow],
    evidence: &[FantasyTodayEvidenceRow],
) -> anyhow::Result<FantasyPickupSequenceView> {
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let all_rostered = db
        .list_teams(&league.id)?
        .into_iter()
        .map(|row| db.list_roster(&row.id))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let mut pool = skaters
        .iter()
        .chain(goalies)
        .map(|view| {
            let key = view.identity.name_normalized.clone();
            DailyCandidatePlayer {
                key: key.clone(),
                player: view.full_name().to_owned(),
                team: current_teams
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| view.team_display().to_owned()),
                positions: eligibility
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| vec![view.position()]),
                quality: scores.get(&key).copied().unwrap_or_default(),
                games_played: view.gp(),
            }
        })
        .collect::<Vec<_>>();
    pool.sort_by(|a, b| a.key.cmp(&b.key));
    pool.dedup_by(|a, b| a.key == b.key);
    let pool_by_key = pool
        .iter()
        .map(|player| (player.key.clone(), player))
        .collect::<HashMap<_, _>>();
    let modeled_roster = roster
        .iter()
        .filter(|key| pool_by_key.contains_key(*key))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut available = pool
        .iter()
        .filter(|player| !all_rostered.contains(&player.key))
        .collect::<Vec<_>>();
    available.sort_by(|a, b| {
        player_rate(b)
            .total_cmp(&player_rate(a))
            .then_with(|| a.key.cmp(&b.key))
    });
    available.truncate(policy.candidate_limit);
    let included_keys = modeled_roster
        .iter()
        .cloned()
        .chain(available.iter().map(|player| player.key.clone()))
        .collect::<BTreeSet<_>>();
    let mut usable_at = HashMap::new();
    let mut players = Vec::new();
    for key in &included_keys {
        let player = pool_by_key[key];
        let waiver = db.get_waiver(&league.id, key)?;
        let available_at = waiver.map_or(evaluated_at, |row| row.clears_at);
        usable_at.insert(key.clone(), available_at);
        let status = resolve_fantasy_player_status(
            key.clone(),
            observations,
            evaluated_at,
            status_max_age_minutes,
        )
        .effective_status;
        players.push(FantasyPickupSequencePlayerInput {
            player_key: key.clone(),
            nhl_player_id: None,
            display_name: player.player.clone(),
            nhl_team: player.team.clone(),
            platform_positions: player.positions.clone(),
            projected_per_game: Some(player_rate(player)),
            game_dates: usable_game_dates(schedule, &player.team, available_at),
            status,
            initially_rostered: modeled_roster.contains(key),
            droppable: true,
            usable_at: available_at,
            drop_lock_at: None,
        });
    }
    let open_roster_slot = roster.len() < rules.standard_roster_capacity();
    let mut transitions = Vec::new();
    let mut ordinal = 0u32;
    for offset in 0..=6 {
        let date = week_start + Duration::days(offset);
        if date < local_date {
            continue;
        }
        let morning = timezone
            .from_local_datetime(&date.and_hms_opt(7, 0, 0).expect("valid local morning"))
            .single()
            .ok_or_else(|| anyhow::anyhow!("07:00 local time is ambiguous on {date}"))?
            .with_timezone(&Utc)
            .max(evaluated_at);
        for candidate in &available {
            let effective_at = morning.max(usable_at[&candidate.key]);
            if effective_at.with_timezone(&timezone).date_naive() != date {
                continue;
            }
            let drops = if open_roster_slot {
                std::iter::once(None)
                    .chain(
                        included_keys
                            .iter()
                            .filter(|key| *key != &candidate.key)
                            .map(Some),
                    )
                    .collect::<Vec<_>>()
            } else {
                included_keys
                    .iter()
                    .filter(|key| *key != &candidate.key)
                    .map(Some)
                    .collect::<Vec<_>>()
            };
            for drop in drops {
                if drop.is_some_and(|key| {
                    team_locked_on(schedule, &pool_by_key[key].team, date, effective_at)
                }) {
                    continue;
                }
                ordinal = ordinal.saturating_add(1);
                transitions.push(FantasyPickupTransitionInput {
                    transition_id: format!(
                        "{date}:{}:{}",
                        candidate.key,
                        drop.cloned().unwrap_or_else(|| "open".to_owned())
                    ),
                    ordinal,
                    effective_at,
                    local_date: date,
                    add_player_key: candidate.key.clone(),
                    drop_player_key: drop.cloned(),
                    matchup_points_delta: 0.0,
                    future_schedule_option_value: 0.0,
                    waiver_reacquisition_cost: drop
                        .map(|key| player_rate(pool_by_key[key]) * 0.10)
                        .unwrap_or_default(),
                    acquisition_budget_cost: if budget.acquisitions_remaining <= 1 {
                        1.0
                    } else {
                        0.25
                    },
                    uncertainty_discount: 0.0,
                    conditional_reason: None,
                });
            }
        }
    }
    crate::fantasy_week_plan_service::assemble_fantasy_week_plan(FantasyPickupSequenceInput {
        context: FantasyPickupSequenceContext {
            league_id: league.id.clone(),
            league_name: league.name.clone(),
            fantasy_team_id: team.id.clone(),
            fantasy_team_name: team.name.clone(),
            stats_season: stats_season.to_owned(),
            season_type,
            competition_mode: competition.mode.label().to_owned(),
            week_start,
            week_end,
            timezone: rules.timezone.clone(),
            generated_at: Utc::now(),
            evaluated_at,
        },
        rules: rules.clone(),
        budget: budget.clone(),
        players,
        transitions,
        max_moves: policy.max_moves,
        beam_width: policy.beam_width,
        alternative_limit: policy.alternative_limit,
        readiness: readiness.to_vec(),
        evidence: evidence.to_vec(),
    })
    .map_err(Into::into)
}

fn player_rate(player: &DailyCandidatePlayer) -> f64 {
    if player.games_played == 0 {
        0.0
    } else {
        player.quality / f64::from(player.games_played)
    }
}

fn usable_game_dates(
    schedule: &[crate::nhl_api::ScheduledGame],
    team: &str,
    usable_at: DateTime<Utc>,
) -> BTreeSet<NaiveDate> {
    schedule
        .iter()
        .filter(|game| {
            game.game_type == 2 && (game.home_abbrev == team || game.away_abbrev == team)
        })
        .filter_map(|game| {
            let start = DateTime::parse_from_rfc3339(&game.start_time_utc)
                .ok()?
                .with_timezone(&Utc);
            (start > usable_at)
                .then(|| NaiveDate::parse_from_str(&game.date, "%Y-%m-%d").ok())
                .flatten()
        })
        .collect()
}

fn team_locked_on(
    schedule: &[crate::nhl_api::ScheduledGame],
    team: &str,
    date: NaiveDate,
    effective_at: DateTime<Utc>,
) -> bool {
    schedule
        .iter()
        .filter(|game| {
            game.game_type == 2
                && game.date == date.to_string()
                && (game.home_abbrev == team || game.away_abbrev == team)
        })
        .filter_map(|game| DateTime::parse_from_rfc3339(&game.start_time_utc).ok())
        .any(|start| start.with_timezone(&Utc) <= effective_at)
}

fn load_scheme(name: &str, schemes_root: &Path) -> anyhow::Result<Scheme> {
    let path = schemes_root.join(format!("{name}.toml"));
    if path.is_file() {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read fantasy scheme {}", path.display()))?;
        return toml::from_str(&text)
            .with_context(|| format!("parse fantasy scheme {}", path.display()));
    }
    Scheme::builtin_named(name).with_context(|| format!("fantasy scheme '{name}' was not found"))
}

fn load_current_player_team_map(
    store: &crate::snapshot::SnapshotStore,
    season: Season,
) -> anyhow::Result<HashMap<String, String>> {
    let mut player_team = HashMap::new();
    for (team, _) in icelines_core::CANONICAL_TEAMS {
        let roster = store.read_tier_file_any_for_season::<crate::schema::RosterResponse>(
            &crate::snapshot::SnapshotTier::Rosters,
            &format!("{team}.json"),
            &season.as_str(),
        )?;
        for player in roster
            .forwards
            .iter()
            .chain(&roster.defensemen)
            .chain(&roster.goalies)
        {
            player_team.insert(
                normalize_name(&format!(
                    "{} {}",
                    player.first_name.as_str(),
                    player.last_name.as_str()
                )),
                (*team).to_owned(),
            );
        }
    }
    Ok(player_team)
}

fn schedule_team_dates(
    schedule: &[crate::nhl_api::ScheduledGame],
) -> HashMap<String, BTreeSet<NaiveDate>> {
    let mut dates = HashMap::<String, BTreeSet<NaiveDate>>::new();
    for game in schedule.iter().filter(|game| game.game_type == 2) {
        let Ok(date) = NaiveDate::parse_from_str(&game.date, "%Y-%m-%d") else {
            continue;
        };
        dates
            .entry(game.away_abbrev.clone())
            .or_default()
            .insert(date);
        dates
            .entry(game.home_abbrev.clone())
            .or_default()
            .insert(date);
    }
    dates
}

fn schedule_team_dates_after(
    schedule: &[crate::nhl_api::ScheduledGame],
    evaluated_at: DateTime<Utc>,
) -> HashMap<String, BTreeSet<NaiveDate>> {
    let mut dates = HashMap::<String, BTreeSet<NaiveDate>>::new();
    for game in schedule.iter().filter(|game| game.game_type == 2) {
        let Ok(date) = NaiveDate::parse_from_str(&game.date, "%Y-%m-%d") else {
            continue;
        };
        let unlocked = DateTime::parse_from_rfc3339(&game.start_time_utc)
            .ok()
            .is_some_and(|start| start.with_timezone(&Utc) > evaluated_at);
        if !unlocked {
            continue;
        }
        dates
            .entry(game.away_abbrev.clone())
            .or_default()
            .insert(date);
        dates
            .entry(game.home_abbrev.clone())
            .or_default()
            .insert(date);
    }
    dates
}

#[derive(Debug, Clone)]
struct DailyCandidatePlayer {
    key: String,
    player: String,
    team: String,
    positions: Vec<icelines_core::model::Position>,
    quality: f64,
    games_played: u32,
}

#[derive(Debug, Default)]
struct DailyRosterProjection {
    usable_starts: usize,
    player_values: HashMap<String, f64>,
}

#[allow(clippy::too_many_arguments)]
fn build_bounded_transaction_candidate(
    db: &crate::fantasy_db::FantasyDb,
    league_id: &str,
    roster: &[String],
    rules: &FantasyAssistantRules,
    budget: &icelines_core::FantasyWeekBudgetView,
    skaters: &[icelines_core::stats_repository::PlayerView<'_>],
    goalies: &[icelines_core::stats_repository::PlayerView<'_>],
    scores: &HashMap<String, f64>,
    eligibility: &HashMap<String, Vec<icelines_core::model::Position>>,
    current_teams: &HashMap<String, String>,
    schedule: &[crate::nhl_api::ScheduledGame],
    local_date: NaiveDate,
    week_end: NaiveDate,
    evaluated_at: DateTime<Utc>,
    policy: &FantasyTodayCandidatePolicy,
) -> anyhow::Result<(
    Option<FantasyDailyTransactionCandidate>,
    FantasyTodayState,
    Option<String>,
)> {
    if policy.candidate_limit == 0 || policy.max_elapsed_ms == 0 {
        return Ok((
            None,
            FantasyTodayState::Provisional,
            Some("icelines fantasy pickups --top 5".to_owned()),
        ));
    }
    if !budget.can_add {
        return Ok((None, FantasyTodayState::Ready, None));
    }

    let candidate_team_dates = schedule_team_dates_after(schedule, evaluated_at);
    let locked_teams_today = schedule
        .iter()
        .filter(|game| game.game_type == 2 && game.date == local_date.to_string())
        .filter(|game| {
            DateTime::parse_from_rfc3339(&game.start_time_utc)
                .ok()
                .is_some_and(|start| start.with_timezone(&Utc) <= evaluated_at)
        })
        .flat_map(|game| [game.away_abbrev.clone(), game.home_abbrev.clone()])
        .collect::<BTreeSet<_>>();

    let all_rostered = db
        .list_teams(league_id)?
        .into_iter()
        .map(|team| db.list_roster(&team.id))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let mut pool = skaters
        .iter()
        .chain(goalies)
        .map(|view| {
            let key = view.identity.name_normalized.clone();
            DailyCandidatePlayer {
                key: key.clone(),
                player: view.full_name().to_owned(),
                team: current_teams
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| view.team_display().to_owned()),
                positions: eligibility
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| vec![view.position()]),
                quality: scores.get(&key).copied().unwrap_or_default(),
                games_played: view.gp(),
            }
        })
        .collect::<Vec<_>>();
    pool.sort_by(|a, b| a.key.cmp(&b.key));
    pool.dedup_by(|a, b| a.key == b.key);
    let pool_by_key = pool
        .iter()
        .map(|player| (player.key.clone(), player))
        .collect::<HashMap<_, _>>();
    let modeled_roster = roster
        .iter()
        .filter(|key| pool_by_key.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let droppable_roster = modeled_roster
        .iter()
        .filter(|key| {
            pool_by_key.get(*key).is_some_and(|player| {
                player.games_played > 0 && !locked_teams_today.contains(&player.team)
            })
        })
        .collect::<Vec<_>>();
    let baseline = weekly_roster_projection(
        &modeled_roster,
        &pool_by_key,
        &candidate_team_dates,
        local_date,
        week_end,
        rules,
    );
    let baseline_value = baseline.player_values.values().sum::<f64>();
    let mut available = pool
        .iter()
        .filter(|player| !all_rostered.contains(&player.key))
        .collect::<Vec<_>>();
    available.sort_by(|a, b| {
        b.quality
            .total_cmp(&a.quality)
            .then_with(|| a.key.cmp(&b.key))
    });
    let available_count = available.len();
    available.truncate(policy.candidate_limit);
    let candidates_considered = available.len();
    let open_roster_slot = roster.len() < rules.standard_roster_capacity();
    // Bound the combinatorial add/drop evaluation itself. Pool construction
    // and the one baseline lineup pass are shared setup, not candidate search.
    let started = Instant::now();
    let mut moves = Vec::new();
    let mut availabilities = HashMap::new();
    let mut timed_out = false;

    'candidates: for candidate in available {
        let waiver = db.get_waiver(league_id, &candidate.key)?;
        let availability =
            fantasy_acquisition_availability(candidate.key.clone(), evaluated_at, waiver.as_ref());
        availabilities.insert(candidate.key.clone(), availability.clone());
        let drops = if open_roster_slot {
            std::iter::once(None)
                .chain(droppable_roster.iter().copied().map(Some))
                .collect::<Vec<_>>()
        } else {
            droppable_roster
                .iter()
                .copied()
                .map(Some)
                .collect::<Vec<_>>()
        };
        for drop_key in drops {
            if started.elapsed().as_millis() >= u128::from(policy.max_elapsed_ms) {
                timed_out = true;
                break 'candidates;
            }
            let drop_value = drop_key
                .and_then(|key| baseline.player_values.get(key).copied())
                .unwrap_or_default();
            let mut after_roster = modeled_roster.clone();
            if let Some(drop_key) = drop_key {
                after_roster.retain(|key| key != drop_key);
            }
            after_roster.push(candidate.key.clone());
            let after = weekly_roster_projection(
                &after_roster,
                &pool_by_key,
                &candidate_team_dates,
                local_date,
                week_end,
                rules,
            );
            let after_value = after.player_values.values().sum::<f64>();
            moves.push(FantasyWeeklyMoveInput {
                add_player_key: candidate.key.clone(),
                add_player: candidate.player.clone(),
                drop_player_key: drop_key.cloned().unwrap_or_default(),
                drop_player: drop_key
                    .and_then(|key| pool_by_key.get(key).map(|player| player.player.clone()))
                    .unwrap_or_else(|| "Open roster slot".to_owned()),
                availability: availability.clone(),
                incremental_usable_starts: after.usable_starts as f64
                    - baseline.usable_starts as f64,
                // The existing pickup scorer subtracts `drop_value` below.
                // Adding it here makes the resulting net component equal the
                // complete active-lineup delta, including displaced starters.
                projected_points_from_incremental_starts: after_value - baseline_value + drop_value,
                category_gap_delta: 0.0,
                future_schedule_option_value: 0.0,
                dropped_player_rest_of_week_value: drop_value,
                waiver_reacquisition_cost: drop_value * 0.10,
                pickup_budget_cost: if budget.acquisitions_remaining <= 1 {
                    1.0
                } else {
                    0.25
                },
                uncertainty_discount: 0.0,
            });
        }
    }
    let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let pickup =
        build_fantasy_weekly_pickups_with_reserve_override(budget.clone(), moves, 1, false, true)
            .map_err(anyhow::Error::msg)?;
    let candidate = pickup
        .rows
        .into_iter()
        .next()
        .filter(|row| row.projected_value_delta > 0.0)
        .map(|row| {
            let availability = availabilities.get(&row.add_player_key);
            FantasyDailyTransactionCandidate {
                add_player_key: row.add_player_key,
                add_player: row.add_player,
                drop_player_key: row.drop_player_key,
                drop_player: row.drop_player,
                modeled_value_delta: row.projected_value_delta,
                incremental_usable_starts: row.incremental_usable_starts,
                legal_at_evaluation: availability.is_some_and(|value| value.usable_now),
                waiver_clears_at: availability
                    .filter(|value| value.status == icelines_core::FantasyMarketStatus::Waivers)
                    .map(|value| value.usable_at),
                acquisition_cost: 1,
                acquisitions_remaining_before: budget.acquisitions_remaining,
                candidates_considered,
                candidate_limit: policy.candidate_limit,
                truncated: timed_out || available_count > candidates_considered,
                elapsed_ms,
                evidence_observed_at: None,
                reasons: row
                    .reasons
                    .into_iter()
                    .chain(std::iter::once(
                        "daily bounded screen uses remaining scheduled games; run the deep pickup command for exhaustive lineup optimization"
                            .to_owned(),
                    ))
                    .collect(),
            }
        });
    Ok((
        candidate,
        if timed_out {
            FantasyTodayState::Provisional
        } else {
            FantasyTodayState::Ready
        },
        timed_out.then(|| "icelines fantasy pickups --top 5".to_owned()),
    ))
}

fn weekly_roster_projection(
    roster: &[String],
    pool: &HashMap<String, &DailyCandidatePlayer>,
    team_dates: &HashMap<String, BTreeSet<NaiveDate>>,
    start: NaiveDate,
    end: NaiveDate,
    rules: &FantasyAssistantRules,
) -> DailyRosterProjection {
    let slots = rules
        .active_slots
        .iter()
        .flat_map(|(slot, count)| std::iter::repeat_n(*slot, usize::from(*count)))
        .collect::<Vec<_>>();
    let mut projection = DailyRosterProjection::default();
    for offset in 0..=(end - start).num_days() {
        let date = start + Duration::days(offset);
        let mut players = roster
            .iter()
            .filter_map(|key| pool.get(key).copied())
            .filter(|player| {
                team_dates
                    .get(&player.team)
                    .is_some_and(|dates| dates.contains(&date))
            })
            .collect::<Vec<_>>();
        players.sort_by(|a, b| {
            candidate_value_per_game(b)
                .total_cmp(&candidate_value_per_game(a))
                .then_with(|| a.key.cmp(&b.key))
        });
        let mut matched = vec![None; slots.len()];
        for player_index in 0..players.len() {
            let mut seen = vec![false; slots.len()];
            let _ = assign_candidate_slot(player_index, &players, &slots, &mut seen, &mut matched);
        }
        let selected = matched.into_iter().flatten().collect::<BTreeSet<_>>();
        projection.usable_starts += selected.len();
        for player_index in selected {
            let player = players[player_index];
            *projection
                .player_values
                .entry(player.key.clone())
                .or_default() += candidate_value_per_game(player);
        }
    }
    projection
}

fn candidate_value_per_game(player: &DailyCandidatePlayer) -> f64 {
    if player.games_played == 0 {
        0.0
    } else {
        player.quality / f64::from(player.games_played)
    }
}

fn assign_candidate_slot(
    player_index: usize,
    players: &[&DailyCandidatePlayer],
    slots: &[FantasyActiveSlotKind],
    seen: &mut [bool],
    matched: &mut [Option<usize>],
) -> bool {
    for (slot_index, slot) in slots.iter().enumerate() {
        if seen[slot_index] || !slot.accepts(&players[player_index].positions) {
            continue;
        }
        seen[slot_index] = true;
        if matched[slot_index].is_none()
            || assign_candidate_slot(
                matched[slot_index].expect("checked occupied slot"),
                players,
                slots,
                seen,
                matched,
            )
        {
            matched[slot_index] = Some(player_index);
            return true;
        }
    }
    false
}

fn value_per_game(
    view: icelines_core::stats_repository::PlayerView<'_>,
    scores: &HashMap<String, f64>,
) -> f64 {
    if view.gp() == 0 {
        0.0
    } else {
        scores
            .get(view.name_normalized())
            .copied()
            .unwrap_or_default()
            / f64::from(view.gp())
    }
}

fn load_week_budget(
    db: &crate::fantasy_db::FantasyDb,
    league_id: &str,
    rules: &FantasyAssistantRules,
    now: DateTime<Utc>,
) -> anyhow::Result<icelines_core::FantasyWeekBudgetView> {
    let acquisitions = db
        .list_acquisitions(league_id, now - Duration::days(8), now + Duration::days(8))?
        .into_iter()
        .map(|row| FantasyAcquisitionInput {
            effective_at: row.effective_at,
            kind: row.kind,
            counts_toward_limit: row.counts_toward_limit,
        })
        .collect::<Vec<_>>();
    let budget = build_fantasy_week_budget(
        now,
        &rules.timezone,
        rules.weekly_acquisition_limit,
        &acquisitions,
    )
    .map_err(anyhow::Error::msg)?;
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    apply_fantasy_pickup_reserve(
        budget,
        now.with_timezone(&timezone).date_naive(),
        rules.injury_pickup_reserve,
        rules.injury_reserve_release_weekday,
    )
    .map_err(anyhow::Error::msg)
}

fn build_bench_coverage(
    fantasy_team: &str,
    morning: &icelines_core::FantasyMorningBriefingView,
    schedule: &[crate::nhl_api::ScheduledGame],
) -> anyhow::Result<Option<icelines_core::FantasyBenchCoverageView>> {
    let lineup = &morning.injury_plan.lineup;
    let players = lineup
        .active
        .iter()
        .map(|row| FantasyBenchCoveragePlayerInput {
            player_key: row.player_key.clone(),
            player: row.player.clone(),
            nhl_team: row.nhl_team.clone(),
            positions: row.platform_positions.clone(),
            projected_value_per_game: row.projected_value,
        })
        .chain(
            lineup
                .bench_assignments
                .iter()
                .map(|row| FantasyBenchCoveragePlayerInput {
                    player_key: row.player_key.clone(),
                    player: row.player.clone(),
                    nhl_team: row.nhl_team.clone(),
                    positions: row.platform_positions.clone(),
                    projected_value_per_game: row.projected_value,
                }),
        )
        .collect::<Vec<_>>();
    if players.is_empty() {
        return Ok(None);
    }
    let games = schedule
        .iter()
        .filter(|game| game.game_type == 2)
        .map(|game| {
            Ok(icelines_core::FantasyScheduleGameInput {
                game_id: game.game_id,
                date: NaiveDate::parse_from_str(&game.date, "%Y-%m-%d")?,
                away_team: game.away_abbrev.clone(),
                home_team: game.home_abbrev.clone(),
            })
        })
        .collect::<Result<Vec<_>, chrono::ParseError>>()?;
    build_fantasy_bench_coverage(FantasyBenchCoverageInput {
        fantasy_team: fantasy_team.to_owned(),
        start: morning.budget.week_start,
        end: morning.budget.week_end,
        off_night_max_games: 4,
        rules: lineup.rules.clone(),
        players,
        games,
    })
    .map(Some)
    .map_err(anyhow::Error::msg)
}

#[allow(clippy::too_many_arguments)]
fn build_points_matchup(
    db: &crate::fantasy_db::FantasyDb,
    league_name: &str,
    scheme_name: &str,
    user: &crate::fantasy_db::TeamRow,
    opponent: &crate::fantasy_db::TeamRow,
    rules: &FantasyAssistantRules,
    views: &HashMap<String, &icelines_core::stats_repository::PlayerView<'_>>,
    scores: &HashMap<String, f64>,
    eligibility: &HashMap<String, Vec<icelines_core::model::Position>>,
    current_teams: &HashMap<String, String>,
    team_dates: &HashMap<String, BTreeSet<NaiveDate>>,
    observations: &[icelines_core::FantasyStatusObservation],
    week_start: NaiveDate,
    week_end: NaiveDate,
    evaluated_at: DateTime<Utc>,
    max_age_minutes: i64,
    current_points: Option<FantasyMatchupPointsSnapshotInput>,
) -> anyhow::Result<icelines_core::FantasyMatchupStrategyView> {
    let team_input = |team: &crate::fantasy_db::TeamRow| -> anyhow::Result<_> {
        let players = db
            .list_roster(&team.id)?
            .into_iter()
            .filter_map(|key| {
                let view = views.get(&key).copied()?;
                let nhl_team = current_teams
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| view.team_display().to_owned());
                let status = resolve_fantasy_player_status(
                    key.clone(),
                    observations,
                    evaluated_at,
                    max_age_minutes,
                );
                Some(FantasyMatchupStrategyPlayerInput {
                    player_key: key.clone(),
                    player: view.full_name().to_owned(),
                    nhl_team: nhl_team.clone(),
                    positions: eligibility
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| vec![view.position()]),
                    projected_value_per_game: value_per_game(*view, scores),
                    game_dates: team_dates.get(&nhl_team).cloned().unwrap_or_default(),
                    status: status.effective_status,
                })
            })
            .collect();
        Ok(FantasyMatchupStrategyTeamInput {
            team: team.name.clone(),
            players,
        })
    };
    build_fantasy_matchup_strategy(FantasyMatchupStrategyInput {
        league: league_name.to_owned(),
        scoring_scheme: scheme_name.to_owned(),
        week_start,
        week_end,
        strategy: FantasyMatchupStrategy::Balanced,
        rules: rules.clone(),
        user: team_input(user)?,
        opponent: team_input(opponent)?,
        current_points,
        largest_legal_swing: None,
        warnings: Vec::new(),
    })
    .map_err(anyhow::Error::msg)
}

#[allow(clippy::too_many_arguments)]
fn build_goalie_plan(
    db: &crate::fantasy_db::FantasyDb,
    league: &crate::fantasy_db::LeagueRow,
    team: &crate::fantasy_db::TeamRow,
    rules: &FantasyAssistantRules,
    scheme: &Scheme,
    skaters: &[icelines_core::stats_repository::PlayerView<'_>],
    goalies: &[icelines_core::stats_repository::PlayerView<'_>],
    current_teams: &HashMap<String, String>,
    schedule: &[crate::nhl_api::ScheduledGame],
    week_start: NaiveDate,
    week_end: NaiveDate,
    focus_date: NaiveDate,
    current_appearances: f64,
    max_age_minutes: i64,
    evaluated_at: DateTime<Utc>,
    acquisitions_remaining: u8,
) -> anyhow::Result<icelines_core::FantasyGoaliePlanView> {
    let roster = db
        .list_roster(&team.id)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let all_rostered = db
        .list_teams(&league.id)?
        .into_iter()
        .map(|row| db.list_roster(&row.id))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let mut offense = HashMap::<String, f64>::new();
    for view in skaters.iter().filter(|view| view.gp() > 0) {
        let team = current_teams
            .get(view.name_normalized())
            .cloned()
            .unwrap_or_else(|| view.team_display().to_owned());
        *offense.entry(team).or_default() +=
            f64::from(view.stats.totals.goals) / f64::from(view.gp());
    }
    let average = if offense.is_empty() {
        1.0
    } else {
        offense.values().sum::<f64>() / offense.len() as f64
    };
    let offense_index = offense
        .into_iter()
        .map(|(team, value)| (team, (value / average.max(f64::EPSILON)).clamp(0.75, 1.25)))
        .collect::<HashMap<_, _>>();
    let mut inputs = Vec::new();
    for view in goalies {
        let Some(stats) = view.stats.goalie.as_ref() else {
            continue;
        };
        let Some(score_stats) = goalie_scheme_stats_from_view(view) else {
            continue;
        };
        let key = view.name_normalized().to_owned();
        let nhl_team = current_teams
            .get(&key)
            .cloned()
            .unwrap_or_else(|| view.team_display().to_owned());
        let games =
            goalie_schedule_contexts(schedule, &nhl_team, week_start, week_end, &offense_index);
        if games.is_empty() {
            continue;
        }
        let rostered = roster.contains(&key);
        let owned_elsewhere = all_rostered.contains(&key) && !rostered;
        let score_per_start = compute_goalie_fantasy_score(&score_stats, &scheme.goalie, view.gp())
            .map(|score| {
                if stats.games_started == 0 {
                    0.0
                } else {
                    f64::from(score.total) / f64::from(stats.games_started)
                }
            })
            .unwrap_or_default();
        inputs.push(FantasyGoaliePlanPlayerInput {
            player_key: key,
            player: view.full_name().to_owned(),
            nhl_team,
            rostered,
            acquisition_eligible: !rostered && !owned_elsewhere,
            games,
            projected_points_per_start: score_per_start,
            historical_start_probability: (f64::from(stats.games_started) / 82.0).clamp(0.0, 1.0),
            expected_save_percentage: stats.save_pct.map(f64::from),
            expected_goals_against_average: stats.goals_against_average.map(f64::from),
        });
    }
    let competition = db.get_competition_rules(&league.id)?;
    build_fantasy_goalie_plan(FantasyGoaliePlanInput {
        league: league.name.clone(),
        team: team.name.clone(),
        week_start,
        week_end,
        focus_date: Some(focus_date),
        strategy: FantasyMatchupStrategy::Balanced,
        competition_mode: competition.mode,
        goalie_slots: rules
            .active_slots
            .get(&FantasyActiveSlotKind::Goalie)
            .copied()
            .unwrap_or(0),
        minimum_goalie_appearances: competition.minimum_goalie_appearances,
        current_goalie_appearances: current_appearances,
        evaluated_at,
        max_age_minutes,
        acquisitions_remaining,
        goalies: inputs,
        observations: db.list_latest_goalie_start_observations(&league.id, week_start, week_end)?,
        warnings: Vec::new(),
    })
    .map_err(anyhow::Error::msg)
}

fn goalie_schedule_contexts(
    games: &[crate::nhl_api::ScheduledGame],
    team: &str,
    week_start: NaiveDate,
    week_end: NaiveDate,
    offense_index: &HashMap<String, f64>,
) -> Vec<FantasyGoalieGameInput> {
    let team_dates = games
        .iter()
        .filter(|game| {
            game.game_type == 2 && (game.home_abbrev == team || game.away_abbrev == team)
        })
        .filter_map(|game| NaiveDate::parse_from_str(&game.date, "%Y-%m-%d").ok())
        .collect::<BTreeSet<_>>();
    let mut rows = games
        .iter()
        .filter(|game| {
            game.game_type == 2 && (game.home_abbrev == team || game.away_abbrev == team)
        })
        .filter_map(|game| {
            let date = NaiveDate::parse_from_str(&game.date, "%Y-%m-%d").ok()?;
            if date < week_start || date > week_end {
                return None;
            }
            let home = game.home_abbrev == team;
            let opponent = if home {
                game.away_abbrev.clone()
            } else {
                game.home_abbrev.clone()
            };
            Some(FantasyGoalieGameInput {
                date,
                start_time_utc: DateTime::parse_from_rfc3339(&game.start_time_utc)
                    .ok()
                    .map(|value| value.with_timezone(&Utc)),
                opponent_offense_index: offense_index.get(&opponent).copied().unwrap_or(1.0),
                opponent,
                home,
                team_back_to_back: team_dates.contains(&(date - Duration::days(1))),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.date);
    rows
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Timelike};
    use icelines_core::{
        FantasyPlatformMatchupSnapshot, FantasyPlatformSnapshot, FANTASY_PLATFORM_SNAPSHOT_SCHEMA,
    };

    use super::*;

    fn snapshot(captured_hour: u32, week_start: NaiveDate) -> FantasyPlatformSnapshot {
        FantasyPlatformSnapshot {
            schema: FANTASY_PLATFORM_SNAPSHOT_SCHEMA.to_owned(),
            platform: "yahoo".to_owned(),
            captured_at: Utc
                .with_ymd_and_hms(2026, 9, 8, captured_hour, 0, 0)
                .unwrap(),
            source_url: Some("https://example.invalid/private".to_owned()),
            standings: Vec::new(),
            matchup: Some(FantasyPlatformMatchupSnapshot {
                week_start,
                team: "Team".to_owned(),
                opponent: "Rival".to_owned(),
                team_points: 30.0,
                opponent_points: 25.0,
                through: Some(week_start + chrono::Duration::days(1)),
                team_goalie_appearances: Some(1),
                opponent_goalie_appearances: Some(2),
            }),
            statuses: Vec::new(),
        }
    }

    #[test]
    fn selects_newest_coherent_snapshot_and_rejects_future_state() {
        let week = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap();
        let selection = select_saved_points_matchup(
            vec![snapshot(13, week), snapshot(11, week)],
            "Team",
            "Rival",
            week,
            week + chrono::Duration::days(6),
            now,
            NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
            180,
        );
        assert_eq!(selection.selected.unwrap().captured_at.hour(), 11);
        assert_eq!(
            selection.rejections[0].reason,
            FantasySavedMatchupRejectionReason::FutureSnapshot
        );
    }

    #[test]
    fn reverse_orientation_preserves_user_point_and_goalie_axes() {
        let week = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let mut value = snapshot(11, week);
        let matchup = value.matchup.as_mut().unwrap();
        matchup.team = "Rival".to_owned();
        matchup.opponent = "Team".to_owned();
        matchup.team_points = 25.0;
        matchup.opponent_points = 30.0;
        matchup.team_goalie_appearances = Some(2);
        matchup.opponent_goalie_appearances = Some(1);
        let selected = select_saved_points_matchup(
            vec![value],
            "Team",
            "Rival",
            week,
            week + chrono::Duration::days(6),
            Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
            180,
        )
        .selected
        .unwrap();
        assert_eq!(selected.points.user_points, 30.0);
        assert_eq!(selected.user_goalie_appearances, Some(1));
    }

    #[test]
    fn rejects_wrong_team_axes_instead_of_composing_unrelated_points() {
        let week = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let mut value = snapshot(11, week);
        value.matchup.as_mut().unwrap().team = "Someone Else".to_owned();
        let selection = select_saved_points_matchup(
            vec![value],
            "Team",
            "Rival",
            week,
            week + chrono::Duration::days(6),
            Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
            180,
        );

        assert!(selection.selected.is_none());
        assert_eq!(
            selection.rejections[0].reason,
            FantasySavedMatchupRejectionReason::WrongTeams
        );
    }

    #[test]
    fn stale_and_partial_saved_matchups_are_rejected_explicitly() {
        let week = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap();
        let stale = snapshot(8, week);
        let mut partial = snapshot(11, week);
        partial.matchup.as_mut().unwrap().through = None;

        let selection = select_saved_points_matchup(
            vec![stale, partial],
            "Team",
            "Rival",
            week,
            week + Duration::days(6),
            now,
            NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
            180,
        );

        assert!(selection.selected.is_none());
        assert!(selection
            .rejections
            .iter()
            .any(|row| row.reason == FantasySavedMatchupRejectionReason::StaleSnapshot));
        assert!(selection
            .rejections
            .iter()
            .any(|row| { row.reason == FantasySavedMatchupRejectionReason::MissingThroughDate }));
    }

    #[test]
    fn missing_database_is_typed_and_does_not_create_state() {
        let temp = tempfile::tempdir().unwrap();
        let database_path = temp.path().join("missing.db");
        let request = FantasyTodayAssemblyRequest {
            database_path: database_path.clone(),
            data_root: temp.path().join("data"),
            snapshots_root: temp.path().join("snapshots"),
            schemes_root: temp.path().join("schemes"),
            league: None,
            team: None,
            stats_season: "20252026".to_owned(),
            season_type: SeasonType::Regular,
            schedule_season: 20262027,
            evaluated_at_utc: Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap(),
            local_date: None,
            status_max_age_minutes: 180,
            current_goalie_appearances: 0.0,
            candidate_policy: FantasyTodayCandidatePolicy::default(),
            week_plan_policy: FantasyWeekPlanPolicy::default(),
        };

        assert!(matches!(
            assemble_fantasy_today(request),
            Err(FantasyTodayAssemblyError::DatabaseMissing(_))
        ));
        assert!(!database_path.exists());
        assert!(!database_path.with_extension("db-wal").exists());
        assert!(!database_path.with_extension("db-shm").exists());
    }

    #[test]
    fn missing_rules_and_schedule_cache_have_typed_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let database_path = temp.path().join("icelines.db");
        let db = crate::fantasy_db::FantasyDb::open_path(database_path.clone()).unwrap();
        let league_id = db
            .create_league("Fixture League", "yahoo-standard")
            .unwrap();
        db.create_team(&league_id, "Fixture Team", "Fixture Owner")
            .unwrap();
        db.set_active_league("Fixture League").unwrap();
        assert!(db.set_user_team(&league_id, "Fixture Team").unwrap());
        drop(db);

        let request = FantasyTodayAssemblyRequest {
            database_path: database_path.clone(),
            data_root: temp.path().join("data"),
            snapshots_root: temp.path().join("snapshots"),
            schemes_root: temp.path().join("schemes"),
            league: None,
            team: None,
            stats_season: "20252026".to_owned(),
            season_type: SeasonType::Regular,
            schedule_season: 20262027,
            evaluated_at_utc: Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap(),
            local_date: None,
            status_max_age_minutes: 180,
            current_goalie_appearances: 0.0,
            candidate_policy: FantasyTodayCandidatePolicy::default(),
            week_plan_policy: FantasyWeekPlanPolicy::default(),
        };

        let mut missing_league = request.clone();
        missing_league.league = Some("Missing League".to_owned());
        assert!(matches!(
            assemble_fantasy_today(missing_league),
            Err(FantasyTodayAssemblyError::LeagueMissing(_))
        ));

        let mut missing_team = request.clone();
        missing_team.league = Some("Fixture League".to_owned());
        missing_team.team = Some("Missing Team".to_owned());
        assert!(matches!(
            assemble_fantasy_today(missing_team),
            Err(FantasyTodayAssemblyError::TeamMissing(_))
        ));

        let error = assemble_fantasy_today(request.clone()).unwrap_err();
        assert!(matches!(error, FantasyTodayAssemblyError::RulesMissing(_)));
        assert_eq!(
            error.recovery_command(),
            Some("icelines fantasy assistant-setup")
        );

        let db = crate::fantasy_db::FantasyDb::open_path(database_path).unwrap();
        let mut invalid_timezone_rules = FantasyAssistantRules::configured_2026();
        invalid_timezone_rules.timezone = "Mars/Olympus".to_owned();
        db.set_assistant_rules(&league_id, &invalid_timezone_rules)
            .unwrap();
        drop(db);
        assert!(matches!(
            assemble_fantasy_today(request.clone()),
            Err(FantasyTodayAssemblyError::InvalidTimezone(_))
        ));

        let db = crate::fantasy_db::FantasyDb::open_path(request.database_path.clone()).unwrap();
        db.set_assistant_rules(&league_id, &FantasyAssistantRules::configured_2026())
            .unwrap();
        drop(db);
        let error = assemble_fantasy_today(request).unwrap_err();
        assert!(matches!(error, FantasyTodayAssemblyError::CacheMissing(_)));
        assert_eq!(
            error.recovery_command(),
            Some("icelines fantasy schedule-edge --refresh")
        );
    }

    #[test]
    fn categories_never_claim_saved_points_matchup_support() {
        assert!(!competition_mode_supports_saved_matchup(
            FantasyCompetitionMode::Categories
        ));
        assert!(competition_mode_supports_saved_matchup(
            FantasyCompetitionMode::Points
        ));
    }

    #[test]
    fn usable_start_matching_distinguishes_collision_from_quiet_night() {
        use icelines_core::model::Position;

        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let incumbent = DailyCandidatePlayer {
            key: "incumbent".to_owned(),
            player: "Incumbent".to_owned(),
            team: "AAA".to_owned(),
            positions: vec![Position::Defense],
            quality: 10.0,
            games_played: 10,
        };
        let candidate = DailyCandidatePlayer {
            key: "candidate".to_owned(),
            player: "Candidate".to_owned(),
            team: "BBB".to_owned(),
            positions: vec![Position::Defense],
            quality: 10.0,
            games_played: 10,
        };
        let players = HashMap::from([
            (incumbent.key.clone(), &incumbent),
            (candidate.key.clone(), &candidate),
        ]);
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots =
            std::collections::BTreeMap::from([(FantasyActiveSlotKind::Defense, 1)]);
        let roster = vec![incumbent.key.clone(), candidate.key.clone()];
        let collision = HashMap::from([
            ("AAA".to_owned(), BTreeSet::from([date])),
            ("BBB".to_owned(), BTreeSet::from([date])),
        ]);
        let quiet = HashMap::from([
            ("AAA".to_owned(), BTreeSet::from([date])),
            ("BBB".to_owned(), BTreeSet::from([date + Duration::days(1)])),
        ]);

        assert_eq!(
            weekly_roster_projection(&roster, &players, &collision, date, date, &rules)
                .usable_starts,
            1
        );
        assert_eq!(
            weekly_roster_projection(
                &roster,
                &players,
                &quiet,
                date,
                date + Duration::days(1),
                &rules,
            )
            .usable_starts,
            2
        );
    }
}
