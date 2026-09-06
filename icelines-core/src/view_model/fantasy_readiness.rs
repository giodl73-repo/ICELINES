//! Cross-workflow fantasy evidence readiness.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::fantasy_today::FantasyTodayState;

pub const FANTASY_READINESS_SCHEMA: &str = "fantasy_readiness.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyReadinessWorkflow {
    Draft,
    Today,
    Matchup,
    WeekPlan,
    Goalie,
    Trade,
    DecisionReview,
}

impl FantasyReadinessWorkflow {
    pub const ALL: [Self; 7] = [
        Self::Draft,
        Self::Today,
        Self::Matchup,
        Self::WeekPlan,
        Self::Goalie,
        Self::Trade,
        Self::DecisionReview,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Today => "today",
            Self::Matchup => "matchup",
            Self::WeekPlan => "week_plan",
            Self::Goalie => "goalie",
            Self::Trade => "trade",
            Self::DecisionReview => "decision_review",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyReadinessRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyReadinessCheckInput {
    pub check_id: String,
    pub requirement: FantasyReadinessRequirement,
    pub state: FantasyTodayState,
    pub reason_code: Option<String>,
    pub message: String,
    pub recovery_command: Option<String>,
    pub source_family: Option<String>,
    pub observed_at: Option<String>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyReadinessWorkflowInput {
    pub workflow: FantasyReadinessWorkflow,
    pub checks: Vec<FantasyReadinessCheckInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyReadinessBuildInput {
    pub league_id: Option<String>,
    pub league_name: Option<String>,
    pub fantasy_team_id: Option<String>,
    pub fantasy_team_name: Option<String>,
    pub stats_season: String,
    pub evaluated_at: DateTime<Utc>,
    pub workflows: Vec<FantasyReadinessWorkflowInput>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyReadinessCheckRow {
    pub check_id: String,
    pub requirement: FantasyReadinessRequirement,
    pub state: FantasyTodayState,
    pub reason_code: Option<String>,
    pub message: String,
    pub recovery_command: Option<String>,
    pub source_family: Option<String>,
    pub observed_at: Option<String>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyReadinessWorkflowRow {
    pub workflow: FantasyReadinessWorkflow,
    pub state: FantasyTodayState,
    pub ready_checks: usize,
    pub total_checks: usize,
    pub checks: Vec<FantasyReadinessCheckRow>,
    pub recovery_commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyReadinessView {
    pub schema: String,
    pub league_id: Option<String>,
    pub league_name: Option<String>,
    pub fantasy_team_id: Option<String>,
    pub fantasy_team_name: Option<String>,
    pub stats_season: String,
    pub evaluated_at: DateTime<Utc>,
    pub state: FantasyTodayState,
    pub ready_workflows: usize,
    pub provisional_workflows: usize,
    pub blocked_workflows: usize,
    pub workflows: Vec<FantasyReadinessWorkflowRow>,
    pub warnings: Vec<String>,
    pub material_fingerprint: String,
}

pub fn build_fantasy_readiness(
    input: FantasyReadinessBuildInput,
) -> Result<FantasyReadinessView, String> {
    if input.stats_season.trim().is_empty() {
        return Err("stats season is required".to_owned());
    }
    if input.workflows.is_empty() {
        return Err("at least one readiness workflow is required".to_owned());
    }
    let mut workflow_ids = BTreeSet::new();
    let mut workflows = Vec::with_capacity(input.workflows.len());
    for workflow in input.workflows {
        if !workflow_ids.insert(workflow.workflow) {
            return Err(format!(
                "duplicate readiness workflow '{}'",
                workflow.workflow.as_str()
            ));
        }
        if workflow.checks.is_empty() {
            return Err(format!(
                "readiness workflow '{}' has no checks",
                workflow.workflow.as_str()
            ));
        }
        let mut check_ids = BTreeSet::new();
        let mut checks = Vec::with_capacity(workflow.checks.len());
        for check in workflow.checks {
            if check.check_id.trim().is_empty() || check.message.trim().is_empty() {
                return Err("readiness check ID and message are required".to_owned());
            }
            if !check_ids.insert(check.check_id.clone()) {
                return Err(format!(
                    "duplicate readiness check '{}' in workflow '{}'",
                    check.check_id,
                    workflow.workflow.as_str()
                ));
            }
            if check.state != FantasyTodayState::Ready
                && check.recovery_command.as_deref().is_none_or(str::is_empty)
            {
                return Err(format!(
                    "non-ready check '{}' requires a recovery command",
                    check.check_id
                ));
            }
            checks.push(FantasyReadinessCheckRow {
                check_id: check.check_id,
                requirement: check.requirement,
                state: check.state,
                reason_code: check.reason_code,
                message: check.message,
                recovery_command: check.recovery_command,
                source_family: check.source_family,
                observed_at: check.observed_at,
                fetched_at: check.fetched_at,
            });
        }
        let state = workflow_state(&checks);
        let recovery_commands = checks
            .iter()
            .filter(|row| row.state != FantasyTodayState::Ready)
            .filter_map(|row| row.recovery_command.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        workflows.push(FantasyReadinessWorkflowRow {
            workflow: workflow.workflow,
            state,
            ready_checks: checks
                .iter()
                .filter(|row| row.state == FantasyTodayState::Ready)
                .count(),
            total_checks: checks.len(),
            checks,
            recovery_commands,
        });
    }
    workflows.sort_by_key(|row| row.workflow);
    let ready_workflows = workflows
        .iter()
        .filter(|row| row.state == FantasyTodayState::Ready)
        .count();
    let provisional_workflows = workflows
        .iter()
        .filter(|row| row.state == FantasyTodayState::Provisional)
        .count();
    let blocked_workflows = workflows
        .iter()
        .filter(|row| row.state == FantasyTodayState::Blocked)
        .count();
    let state = if blocked_workflows > 0 {
        FantasyTodayState::Blocked
    } else if provisional_workflows > 0 {
        FantasyTodayState::Provisional
    } else {
        FantasyTodayState::Ready
    };
    let mut view = FantasyReadinessView {
        schema: FANTASY_READINESS_SCHEMA.to_owned(),
        league_id: input.league_id,
        league_name: input.league_name,
        fantasy_team_id: input.fantasy_team_id,
        fantasy_team_name: input.fantasy_team_name,
        stats_season: input.stats_season,
        evaluated_at: input.evaluated_at,
        state,
        ready_workflows,
        provisional_workflows,
        blocked_workflows,
        workflows,
        warnings: input.warnings,
        material_fingerprint: String::new(),
    };
    view.material_fingerprint = readiness_fingerprint(&view)?;
    Ok(view)
}

fn workflow_state(checks: &[FantasyReadinessCheckRow]) -> FantasyTodayState {
    if checks.iter().any(|row| {
        row.requirement == FantasyReadinessRequirement::Required
            && row.state == FantasyTodayState::Blocked
    }) {
        FantasyTodayState::Blocked
    } else if checks
        .iter()
        .any(|row| row.state != FantasyTodayState::Ready)
    {
        FantasyTodayState::Provisional
    } else {
        FantasyTodayState::Ready
    }
}

fn readiness_fingerprint(view: &FantasyReadinessView) -> Result<String, String> {
    let mut material = view.clone();
    material.material_fingerprint.clear();
    let bytes = serde_json::to_vec(&material).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    fn check(
        id: &str,
        requirement: FantasyReadinessRequirement,
        state: FantasyTodayState,
    ) -> FantasyReadinessCheckInput {
        FantasyReadinessCheckInput {
            check_id: id.to_owned(),
            requirement,
            state,
            reason_code: (state != FantasyTodayState::Ready).then(|| "missing".to_owned()),
            message: format!("{id} state"),
            recovery_command: (state != FantasyTodayState::Ready)
                .then(|| format!("icelines fantasy repair-{id}")),
            source_family: None,
            observed_at: None,
            fetched_at: None,
        }
    }

    fn input(checks: Vec<FantasyReadinessCheckInput>) -> FantasyReadinessBuildInput {
        FantasyReadinessBuildInput {
            league_id: Some("league".to_owned()),
            league_name: Some("League".to_owned()),
            fantasy_team_id: Some("team".to_owned()),
            fantasy_team_name: Some("Team".to_owned()),
            stats_season: "20252026".to_owned(),
            evaluated_at: Utc.with_ymd_and_hms(2026, 9, 5, 18, 0, 0).unwrap(),
            workflows: vec![FantasyReadinessWorkflowInput {
                workflow: FantasyReadinessWorkflow::Today,
                checks,
            }],
            warnings: Vec::new(),
        }
    }

    #[test]
    fn required_blocker_blocks_but_optional_blocker_is_provisional() {
        let blocked = build_fantasy_readiness(input(vec![check(
            "schedule",
            FantasyReadinessRequirement::Required,
            FantasyTodayState::Blocked,
        )]))
        .unwrap();
        assert_eq!(blocked.state, FantasyTodayState::Blocked);

        let provisional = build_fantasy_readiness(input(vec![check(
            "status",
            FantasyReadinessRequirement::Optional,
            FantasyTodayState::Blocked,
        )]))
        .unwrap();
        assert_eq!(provisional.state, FantasyTodayState::Provisional);
    }

    #[test]
    fn duplicate_checks_and_missing_recovery_fail_closed() {
        assert!(build_fantasy_readiness(input(vec![
            check(
                "schedule",
                FantasyReadinessRequirement::Required,
                FantasyTodayState::Ready,
            ),
            check(
                "schedule",
                FantasyReadinessRequirement::Required,
                FantasyTodayState::Ready,
            ),
        ]))
        .is_err());
        let mut missing = check(
            "schedule",
            FantasyReadinessRequirement::Required,
            FantasyTodayState::Provisional,
        );
        missing.recovery_command = None;
        assert!(build_fantasy_readiness(input(vec![missing])).is_err());
    }

    #[test]
    fn fingerprint_is_stable_and_changes_with_material_state() {
        let ready = build_fantasy_readiness(input(vec![check(
            "schedule",
            FantasyReadinessRequirement::Required,
            FantasyTodayState::Ready,
        )]))
        .unwrap();
        let again = build_fantasy_readiness(input(vec![check(
            "schedule",
            FantasyReadinessRequirement::Required,
            FantasyTodayState::Ready,
        )]))
        .unwrap();
        assert_eq!(ready.material_fingerprint, again.material_fingerprint);
        let provisional = build_fantasy_readiness(input(vec![check(
            "schedule",
            FantasyReadinessRequirement::Required,
            FantasyTodayState::Provisional,
        )]))
        .unwrap();
        assert_ne!(ready.material_fingerprint, provisional.material_fingerprint);
    }
}
