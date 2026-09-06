//! Read-only assembly for the cross-workflow fantasy readiness dashboard.

use std::collections::HashMap;

use icelines_core::{
    build_fantasy_readiness, FantasyReadinessBuildInput, FantasyReadinessCheckInput,
    FantasyReadinessRequirement, FantasyReadinessView, FantasyReadinessWorkflow,
    FantasyReadinessWorkflowInput, FantasyTodayEvidenceRow, FantasyTodayReadinessRow,
    FantasyTodayState,
};

use crate::fantasy_db::FantasyDb;
use crate::fantasy_today_service::{
    assemble_fantasy_today, FantasyTodayAssemblyError, FantasyTodayAssemblyRequest,
};

#[derive(Debug, Clone)]
pub struct FantasyReadinessAssemblyRequest {
    pub today: FantasyTodayAssemblyRequest,
    pub workflow: Option<FantasyReadinessWorkflow>,
}

pub fn assemble_fantasy_readiness(
    request: FantasyReadinessAssemblyRequest,
) -> Result<FantasyReadinessView, String> {
    let stats_season = request.today.stats_season.clone();
    let evaluated_at = request.today.evaluated_at_utc;
    let selected = request
        .workflow
        .map_or_else(|| FantasyReadinessWorkflow::ALL.to_vec(), |row| vec![row]);
    let today = match assemble_fantasy_today(request.today.clone()) {
        Ok(view) => view,
        Err(error) => return blocked_root_view(stats_season, evaluated_at, selected, error),
    };
    let readiness = today
        .today
        .readiness
        .iter()
        .map(|row| (row.workflow.as_str(), row))
        .collect::<HashMap<_, _>>();
    let evidence = today
        .today
        .evidence
        .iter()
        .map(|row| (row.source_family.as_str(), row))
        .collect::<HashMap<_, _>>();
    let supplemental = supplemental_checks(&request, &today)?;
    let workflows = selected
        .into_iter()
        .map(|workflow| FantasyReadinessWorkflowInput {
            workflow,
            checks: workflow_checks(workflow)
                .into_iter()
                .map(|(id, requirement)| {
                    check_from_sources(id, requirement, &readiness, &evidence, &supplemental)
                })
                .collect(),
        })
        .collect();
    build_fantasy_readiness(FantasyReadinessBuildInput {
        league_id: Some(today.today.context.league_id.clone()),
        league_name: Some(today.today.context.league_name.clone()),
        fantasy_team_id: Some(today.today.context.fantasy_team_id.clone()),
        fantasy_team_name: Some(today.today.context.fantasy_team_name.clone()),
        stats_season,
        evaluated_at,
        workflows,
        warnings: today.today.warnings.clone(),
    })
}

fn supplemental_checks(
    request: &FantasyReadinessAssemblyRequest,
    today: &icelines_core::FantasyTodayV2View,
) -> Result<HashMap<&'static str, FantasyReadinessCheckInput>, String> {
    let mut checks = HashMap::new();
    checks.insert(
        "pickup_budget",
        FantasyReadinessCheckInput {
            check_id: "pickup_budget".to_owned(),
            requirement: FantasyReadinessRequirement::Required,
            state: FantasyTodayState::Ready,
            reason_code: None,
            message: format!(
                "weekly acquisition budget loaded: {} of {} move(s) remain",
                today.today.acquisitions.remaining, today.today.acquisitions.limit
            ),
            recovery_command: Some("icelines fantasy weekly-budget".to_owned()),
            source_family: Some("fantasy_local_state".to_owned()),
            observed_at: Some(request.today.evaluated_at_utc.to_rfc3339()),
            fetched_at: None,
        },
    );
    let goalie = today.today.goalies.as_ref();
    let goalie_refreshes =
        goalie.map_or(0, |row| row.refreshes_due_now + row.safety_checks_due_now);
    checks.insert(
        "goalie_evidence",
        FantasyReadinessCheckInput {
            check_id: "goalie_evidence".to_owned(),
            requirement: FantasyReadinessRequirement::Required,
            state: if goalie.is_none() {
                FantasyTodayState::Blocked
            } else if goalie_refreshes > 0 {
                FantasyTodayState::Provisional
            } else {
                FantasyTodayState::Ready
            },
            reason_code: if goalie.is_none() {
                Some("goalie_plan_unavailable".to_owned())
            } else if goalie_refreshes > 0 {
                Some("goalie_refresh_required".to_owned())
            } else {
                None
            },
            message: goalie.map_or_else(
                || "goalie plan could not be assembled".to_owned(),
                |row| {
                    format!(
                        "goalie evidence loaded; {} refresh/checkpoint(s) due now",
                        row.refreshes_due_now + row.safety_checks_due_now
                    )
                },
            ),
            recovery_command: Some("icelines fantasy goalie-start-show".to_owned()),
            source_family: Some("fantasy_goalie_observations".to_owned()),
            observed_at: None,
            fetched_at: None,
        },
    );

    let db = FantasyDb::open_existing_read_only_path(request.today.database_path.clone())
        .map_err(|error| error.to_string())?;
    let decisions = db
        .list_decisions(&today.today.context.league_id, 1, false)
        .map_err(|error| error.to_string())?;
    checks.insert(
        "decision_journal",
        FantasyReadinessCheckInput {
            check_id: "decision_journal".to_owned(),
            requirement: FantasyReadinessRequirement::Required,
            state: if decisions.is_empty() {
                FantasyTodayState::Provisional
            } else {
                FantasyTodayState::Ready
            },
            reason_code: decisions
                .is_empty()
                .then(|| "no_frozen_decisions".to_owned()),
            message: if decisions.is_empty() {
                "no frozen decisions are available for retrospective review".to_owned()
            } else {
                "at least one immutable fantasy decision is available for review".to_owned()
            },
            recovery_command: Some("icelines fantasy decision-record".to_owned()),
            source_family: Some("fantasy_decision_journal".to_owned()),
            observed_at: decisions.first().map(|row| row.recorded_at.clone()),
            fetched_at: None,
        },
    );
    let teams = db
        .list_teams(&today.today.context.league_id)
        .map_err(|error| error.to_string())?;
    let complete_teams = teams
        .iter()
        .filter(|team| {
            db.list_roster(&team.id)
                .is_ok_and(|roster| !roster.is_empty())
        })
        .count();
    let trade_ready = teams.len() >= 2 && complete_teams == teams.len();
    checks.insert(
        "trade_inventory",
        FantasyReadinessCheckInput {
            check_id: "trade_inventory".to_owned(),
            requirement: FantasyReadinessRequirement::Required,
            state: if trade_ready {
                FantasyTodayState::Ready
            } else {
                FantasyTodayState::Blocked
            },
            reason_code: (!trade_ready).then(|| "league_rosters_incomplete".to_owned()),
            message: format!(
                "{complete_teams} of {} saved team roster(s) contain players",
                teams.len()
            ),
            recovery_command: Some("icelines fantasy sync-yahoo --help".to_owned()),
            source_family: Some("fantasy_local_state".to_owned()),
            observed_at: Some(request.today.evaluated_at_utc.to_rfc3339()),
            fetched_at: None,
        },
    );
    Ok(checks)
}

fn workflow_checks(
    workflow: FantasyReadinessWorkflow,
) -> Vec<(&'static str, FantasyReadinessRequirement)> {
    use FantasyReadinessRequirement::{Optional, Required};
    match workflow {
        FantasyReadinessWorkflow::Draft => vec![
            ("rules_roster", Required),
            ("player_rates", Required),
            ("current_rosters", Optional),
        ],
        FantasyReadinessWorkflow::Today => vec![
            ("schedule", Required),
            ("rules_roster", Required),
            ("player_rates", Required),
            ("current_rosters", Optional),
            ("player_status", Optional),
            ("matchup", Optional),
        ],
        FantasyReadinessWorkflow::Matchup => vec![
            ("schedule", Required),
            ("rules_roster", Required),
            ("player_rates", Required),
            ("current_rosters", Required),
            ("player_status", Optional),
            ("matchup", Required),
        ],
        FantasyReadinessWorkflow::WeekPlan => vec![
            ("schedule", Required),
            ("rules_roster", Required),
            ("player_rates", Required),
            ("current_rosters", Required),
            ("player_status", Optional),
            ("pickup_budget", Required),
            ("matchup", Optional),
        ],
        FantasyReadinessWorkflow::Goalie => vec![
            ("schedule", Required),
            ("rules_roster", Required),
            ("player_rates", Required),
            ("current_rosters", Required),
            ("player_status", Optional),
            ("goalie_evidence", Required),
        ],
        FantasyReadinessWorkflow::Trade => vec![
            ("rules_roster", Required),
            ("player_rates", Required),
            ("trade_inventory", Required),
        ],
        FantasyReadinessWorkflow::DecisionReview => {
            vec![("rules_roster", Required), ("decision_journal", Required)]
        }
    }
}

fn check_from_sources(
    id: &'static str,
    requirement: FantasyReadinessRequirement,
    readiness: &HashMap<&str, &FantasyTodayReadinessRow>,
    evidence: &HashMap<&str, &FantasyTodayEvidenceRow>,
    supplemental: &HashMap<&'static str, FantasyReadinessCheckInput>,
) -> FantasyReadinessCheckInput {
    if let Some(row) = supplemental.get(id) {
        let mut row = row.clone();
        row.requirement = requirement;
        return row;
    }
    let Some(row) = readiness.get(id) else {
        return FantasyReadinessCheckInput {
            check_id: id.to_owned(),
            requirement,
            state: FantasyTodayState::Blocked,
            reason_code: Some("readiness_signal_missing".to_owned()),
            message: format!("the shared daily contract did not emit the '{id}' check"),
            recovery_command: Some("icelines fantasy today".to_owned()),
            source_family: None,
            observed_at: None,
            fetched_at: None,
        };
    };
    let source_id = match id {
        "schedule" => Some("nhl_schedule_cache"),
        "rules_roster" => Some("fantasy_local_state"),
        "player_rates" => Some("sealed_stats"),
        "current_rosters" => Some("nhl_roster_cache"),
        _ => None,
    };
    let source = source_id.and_then(|source_id| evidence.get(source_id).copied());
    FantasyReadinessCheckInput {
        check_id: id.to_owned(),
        requirement,
        state: row.state,
        reason_code: row.reason_code.clone(),
        message: row.message.clone(),
        recovery_command: row.recovery_command.clone().or_else(|| {
            (row.state != FantasyTodayState::Ready).then(|| "icelines fantasy today".to_owned())
        }),
        source_family: source.map(|row| row.source_family.clone()),
        observed_at: source.and_then(|row| row.observed_at.clone()),
        fetched_at: source.and_then(|row| row.fetched_at.clone()),
    }
}

fn blocked_root_view(
    stats_season: String,
    evaluated_at: chrono::DateTime<chrono::Utc>,
    workflows: Vec<FantasyReadinessWorkflow>,
    error: FantasyTodayAssemblyError,
) -> Result<FantasyReadinessView, String> {
    let recovery = error
        .recovery_command()
        .unwrap_or("icelines fantasy readiness --help")
        .to_owned();
    let message = error.to_string();
    build_fantasy_readiness(FantasyReadinessBuildInput {
        league_id: None,
        league_name: None,
        fantasy_team_id: None,
        fantasy_team_name: None,
        stats_season,
        evaluated_at,
        workflows: workflows
            .into_iter()
            .map(|workflow| FantasyReadinessWorkflowInput {
                workflow,
                checks: vec![FantasyReadinessCheckInput {
                    check_id: "root_evidence".to_owned(),
                    requirement: FantasyReadinessRequirement::Required,
                    state: FantasyTodayState::Blocked,
                    reason_code: Some("root_evidence_unavailable".to_owned()),
                    message: message.clone(),
                    recovery_command: Some(recovery.clone()),
                    source_family: None,
                    observed_at: None,
                    fetched_at: None,
                }],
            })
            .collect(),
        warnings: vec![message],
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone as _, Utc};

    use super::*;

    #[test]
    fn missing_database_returns_typed_blocked_view_without_creating_it() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("missing.db");
        let mut today = FantasyTodayAssemblyRequest::from_default_paths(
            None,
            None,
            "20252026".to_owned(),
            20262027,
            Utc.with_ymd_and_hms(2026, 9, 5, 18, 0, 0).unwrap(),
        )
        .unwrap();
        today.database_path = database.clone();
        today.data_root = temp.path().join("data");
        today.snapshots_root = temp.path().join("snapshots");
        today.schemes_root = temp.path().join("schemes");
        let view = assemble_fantasy_readiness(FantasyReadinessAssemblyRequest {
            today,
            workflow: Some(FantasyReadinessWorkflow::Matchup),
        })
        .unwrap();
        assert_eq!(view.schema, icelines_core::FANTASY_READINESS_SCHEMA);
        assert_eq!(view.state, FantasyTodayState::Blocked);
        assert_eq!(view.workflows.len(), 1);
        assert_eq!(
            view.workflows[0].workflow,
            FantasyReadinessWorkflow::Matchup
        );
        assert!(!database.exists());
    }
}
