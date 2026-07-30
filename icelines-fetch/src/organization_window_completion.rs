//! Machine-readable closeout status for The Window's two independent product
//! gates: confirmed production sources and the preregistered future holdout.

use chrono::DateTime;
use icelines_core::{
    validate_organization_window_source_coverage, OrganizationWindowSourceCoverageView,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{OrganizationWindowFutureHoldoutRegistration, OrganizationWindowFutureHoldoutResult};

pub const ORGANIZATION_WINDOW_COMPLETION_STATUS_SCHEMA: &str =
    "organization_window_completion_status.v1";
pub const ORGANIZATION_WINDOW_COMPLETION_STATUS_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/organization_window_completion_status.v1.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationWindowCompletionState {
    EvaluationComplete,
    ProductionRankedAwaitingHoldout,
    HoldoutEligibleUnscored,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationWindowHoldoutGateState {
    WaitingUntilEligible,
    EligibleUnscored,
    Scored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationWindowIncompleteProfileView {
    pub profile_key: String,
    pub method_version: String,
    pub missing_organizations: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowSourceGateView {
    pub passed: bool,
    pub complete_required_profiles: usize,
    pub required_profiles: usize,
    pub rank_eligible_organizations: usize,
    pub expected_organizations: usize,
    pub carry_forward_observations: usize,
    pub incomplete_required_profiles: Vec<OrganizationWindowIncompleteProfileView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowHoldoutGateView {
    pub state: OrganizationWindowHoldoutGateState,
    pub eligible: bool,
    pub outcome_not_before: chrono::NaiveDate,
    pub result_present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_passed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowCompletionStatusView {
    pub schema: String,
    pub evaluated_at: String,
    pub state: OrganizationWindowCompletionState,
    pub project_complete: bool,
    pub source_gate: OrganizationWindowSourceGateView,
    pub holdout_gate: OrganizationWindowHoldoutGateView,
    pub source_audit: OrganizationWindowSourceCoverageView,
    pub holdout_registration: OrganizationWindowFutureHoldoutRegistration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holdout_result: Option<OrganizationWindowFutureHoldoutResult>,
    pub next_actions: Vec<String>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationWindowCompletionError {
    #[error("invalid Window completion evidence: {0}")]
    InvalidEvidence(String),
    #[error("Window completion status fingerprint mismatch")]
    FingerprintMismatch,
}

pub fn build_organization_window_completion_status(
    source_audit: &OrganizationWindowSourceCoverageView,
    holdout_registration: &OrganizationWindowFutureHoldoutRegistration,
    holdout_result: Option<&OrganizationWindowFutureHoldoutResult>,
    evaluated_at: impl Into<String>,
) -> Result<OrganizationWindowCompletionStatusView, OrganizationWindowCompletionError> {
    validate_organization_window_source_coverage(source_audit)
        .map_err(|error| OrganizationWindowCompletionError::InvalidEvidence(error.to_string()))?;
    holdout_registration
        .validate()
        .map_err(|error| OrganizationWindowCompletionError::InvalidEvidence(error.to_string()))?;
    if source_audit.season != holdout_registration.target_season {
        return Err(OrganizationWindowCompletionError::InvalidEvidence(format!(
            "source-audit season {} does not match holdout target season {}",
            source_audit.season, holdout_registration.target_season
        )));
    }

    let evaluated_at = evaluated_at.into();
    let evaluated = DateTime::parse_from_rfc3339(&evaluated_at).map_err(|_| {
        OrganizationWindowCompletionError::InvalidEvidence(
            "evaluated_at must be an RFC 3339 timestamp".to_owned(),
        )
    })?;
    let registered =
        DateTime::parse_from_rfc3339(&holdout_registration.registered_at).map_err(|_| {
            OrganizationWindowCompletionError::InvalidEvidence(
                "holdout registration timestamp must be RFC 3339".to_owned(),
            )
        })?;
    if source_audit.as_of > evaluated.date_naive() || registered > evaluated {
        return Err(OrganizationWindowCompletionError::InvalidEvidence(
            "completion evidence cannot postdate evaluated_at".to_owned(),
        ));
    }
    if let Some(result) = holdout_result {
        result.validate().map_err(|error| {
            OrganizationWindowCompletionError::InvalidEvidence(error.to_string())
        })?;
        let scored_at = DateTime::parse_from_rfc3339(&result.scored_at).map_err(|_| {
            OrganizationWindowCompletionError::InvalidEvidence(
                "holdout result scored_at must be RFC 3339".to_owned(),
            )
        })?;
        if result.registration != *holdout_registration || scored_at > evaluated {
            return Err(OrganizationWindowCompletionError::InvalidEvidence(
                "holdout result must bind the exact registration and cannot postdate evaluated_at"
                    .to_owned(),
            ));
        }
    }

    let (state, project_complete, source_gate, holdout_gate, next_actions) = derive_status(
        source_audit,
        holdout_registration,
        holdout_result,
        evaluated.date_naive(),
    );
    let mut status = OrganizationWindowCompletionStatusView {
        schema: ORGANIZATION_WINDOW_COMPLETION_STATUS_SCHEMA.to_owned(),
        evaluated_at,
        state,
        project_complete,
        source_gate,
        holdout_gate,
        source_audit: source_audit.clone(),
        holdout_registration: holdout_registration.clone(),
        holdout_result: holdout_result.cloned(),
        next_actions,
        disclosures: vec![
            "Project completion requires both confirmed production-ranked sources and one valid score of the exact preregistered future holdout.".to_owned(),
            "A scored holdout completes the evidence lifecycle whether its frozen acceptance rule passes or fails; acceptance controls predictive claims, not whether the result is retained.".to_owned(),
            "Organization health, production rank eligibility, and predictive calibration remain separate claims.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    status.fingerprint = completion_fingerprint(&status)?;
    Ok(status)
}

impl OrganizationWindowCompletionStatusView {
    pub fn validate(&self) -> Result<(), OrganizationWindowCompletionError> {
        let rebuilt = build_organization_window_completion_status(
            &self.source_audit,
            &self.holdout_registration,
            self.holdout_result.as_ref(),
            self.evaluated_at.clone(),
        )?;
        if self.schema != ORGANIZATION_WINDOW_COMPLETION_STATUS_SCHEMA
            || self.state != rebuilt.state
            || self.project_complete != rebuilt.project_complete
            || self.source_gate != rebuilt.source_gate
            || self.holdout_gate != rebuilt.holdout_gate
            || self.next_actions != rebuilt.next_actions
            || self.disclosures != rebuilt.disclosures
            || self.fingerprint != completion_fingerprint(self)?
        {
            return Err(OrganizationWindowCompletionError::FingerprintMismatch);
        }
        Ok(())
    }
}

fn derive_status(
    source: &OrganizationWindowSourceCoverageView,
    registration: &OrganizationWindowFutureHoldoutRegistration,
    result: Option<&OrganizationWindowFutureHoldoutResult>,
    evaluated_date: chrono::NaiveDate,
) -> (
    OrganizationWindowCompletionState,
    bool,
    OrganizationWindowSourceGateView,
    OrganizationWindowHoldoutGateView,
    Vec<String>,
) {
    let mut incomplete_required_profiles = source
        .profiles
        .iter()
        .filter(|profile| profile.required && !profile.complete)
        .map(|profile| OrganizationWindowIncompleteProfileView {
            profile_key: profile.profile_key.clone(),
            method_version: profile.method_version.clone(),
            missing_organizations: profile.missing_organizations.len(),
        })
        .collect::<Vec<_>>();
    incomplete_required_profiles.sort_by(|left, right| {
        left.profile_key
            .cmp(&right.profile_key)
            .then_with(|| left.method_version.cmp(&right.method_version))
    });
    let source_gate = OrganizationWindowSourceGateView {
        passed: source.production_ranked,
        complete_required_profiles: source.complete_required_profiles,
        required_profiles: source.required_profiles,
        rank_eligible_organizations: source.rank_eligible_organizations,
        expected_organizations: source.expected_organizations,
        carry_forward_observations: source.carry_forward_observations,
        incomplete_required_profiles,
    };
    let eligible = evaluated_date >= registration.outcome_not_before;
    let holdout_state = if result.is_some() {
        OrganizationWindowHoldoutGateState::Scored
    } else if eligible {
        OrganizationWindowHoldoutGateState::EligibleUnscored
    } else {
        OrganizationWindowHoldoutGateState::WaitingUntilEligible
    };
    let holdout_gate = OrganizationWindowHoldoutGateView {
        state: holdout_state,
        eligible,
        outcome_not_before: registration.outcome_not_before,
        result_present: result.is_some(),
        acceptance_passed: result.map(|result| result.acceptance_passed),
    };
    let project_complete = source_gate.passed && result.is_some();
    let state = if project_complete {
        OrganizationWindowCompletionState::Complete
    } else if !source_gate.passed {
        OrganizationWindowCompletionState::EvaluationComplete
    } else if eligible {
        OrganizationWindowCompletionState::HoldoutEligibleUnscored
    } else {
        OrganizationWindowCompletionState::ProductionRankedAwaitingHoldout
    };
    let mut next_actions = Vec::new();
    if !source_gate.passed {
        next_actions.push(format!(
            "Refresh confirmed target-season authorities to reach {0}/{0} complete required profiles, {1}/{1} rank-eligible organizations, and zero carry-forward observations (current: {2}/{0} profiles, {3}/{1} organizations, {4} carry-forward observations).",
            source_gate.required_profiles,
            source_gate.expected_organizations,
            source_gate.complete_required_profiles,
            source_gate.rank_eligible_organizations,
            source_gate.carry_forward_observations
        ));
    }
    if result.is_none() {
        if eligible {
            next_actions.push(
                "Score the exact preregistered holdout once with final target-season standings and retain the result regardless of acceptance outcome."
                    .to_owned(),
            );
        } else {
            next_actions.push(format!(
                "Do not score the preregistered holdout before {}.",
                registration.outcome_not_before
            ));
        }
    }
    (
        state,
        project_complete,
        source_gate,
        holdout_gate,
        next_actions,
    )
}

fn completion_fingerprint(
    status: &OrganizationWindowCompletionStatusView,
) -> Result<String, OrganizationWindowCompletionError> {
    let mut canonical = status.clone();
    canonical.fingerprint.clear();
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| OrganizationWindowCompletionError::InvalidEvidence(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_audit() -> OrganizationWindowSourceCoverageView {
        serde_json::from_str(include_str!(
            "../../examples/organization-window-source-audit-partial-2026-07-28.json"
        ))
        .unwrap()
    }

    fn registration() -> OrganizationWindowFutureHoldoutRegistration {
        serde_json::from_str(include_str!(
            "../../examples/window-history/future-holdout-2025-26-to-2026-27-registration.json"
        ))
        .unwrap()
    }

    #[test]
    fn current_evidence_is_evaluation_complete_and_names_both_next_actions() {
        let status = build_organization_window_completion_status(
            &source_audit(),
            &registration(),
            None,
            "2026-07-30T12:00:00Z",
        )
        .unwrap();
        assert_eq!(
            status.state,
            OrganizationWindowCompletionState::EvaluationComplete
        );
        assert!(!status.project_complete);
        assert_eq!(status.source_gate.complete_required_profiles, 14);
        assert_eq!(status.source_gate.incomplete_required_profiles.len(), 2);
        assert_eq!(
            status.holdout_gate.state,
            OrganizationWindowHoldoutGateState::WaitingUntilEligible
        );
        assert_eq!(status.next_actions.len(), 2);
        assert_eq!(
            status.next_actions[0],
            "Refresh confirmed target-season authorities to reach 16/16 complete required profiles, 32/32 rank-eligible organizations, and zero carry-forward observations (current: 14/16 profiles, 0/32 organizations, 0 carry-forward observations)."
        );
        status.validate().unwrap();
    }

    #[test]
    fn eligible_date_does_not_fabricate_a_scored_holdout() {
        let mut source = source_audit();
        for profile in &mut source.profiles {
            profile.organizations_with_observation = 32;
            profile.organizations_with_value = 32;
            profile.missing_organizations.clear();
            profile.complete = true;
        }
        source.complete_required_profiles = source.required_profiles;
        source.rank_eligible_organizations = 32;
        source.carry_forward_observations = 0;
        source.production_ranked = true;
        let status = build_organization_window_completion_status(
            &source,
            &registration(),
            None,
            "2027-04-11T12:00:00Z",
        )
        .unwrap();
        assert_eq!(
            status.state,
            OrganizationWindowCompletionState::HoldoutEligibleUnscored
        );
        assert!(!status.project_complete);
        assert_eq!(status.next_actions.len(), 1);
    }

    #[test]
    fn tampered_status_is_rejected() {
        let mut status = build_organization_window_completion_status(
            &source_audit(),
            &registration(),
            None,
            "2026-07-30T12:00:00Z",
        )
        .unwrap();
        status.project_complete = true;
        assert!(status.validate().is_err());
    }

    #[test]
    fn completion_status_rejects_evidence_from_the_future() {
        let error = build_organization_window_completion_status(
            &source_audit(),
            &registration(),
            None,
            "2026-07-28T12:00:00Z",
        )
        .unwrap_err();
        assert!(error.to_string().contains("cannot postdate"));
    }

    #[test]
    fn completion_status_schema_is_valid_json() {
        let schema: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_COMPLETION_STATUS_JSON_SCHEMA).unwrap();
        assert_eq!(
            schema["$id"],
            "https://icelines.app/schemas/organization_window_completion_status.v1.schema.json"
        );
    }
}
