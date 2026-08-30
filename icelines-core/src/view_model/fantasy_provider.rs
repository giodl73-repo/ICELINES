use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const FANTASY_PROVIDER_STATUS_SCHEMA: &str = "fantasy_provider_status.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyProviderKind {
    Yahoo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyProviderAccessState {
    Unknown,
    PendingReview,
    Granted,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyProviderConnectionState {
    Disconnected,
    Connected,
    ReauthRequired,
    RemoteMissing,
    RelinkRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyProviderCapability {
    LeagueSettings,
    Teams,
    Rosters,
    PlayerEligibility,
    Ownership,
    DraftResults,
    Transactions,
    Standings,
    Scoreboard,
    Matchups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyProviderCapabilityState {
    Blocked,
    Unknown,
    Fresh,
    Stale,
    Partial,
    Inconsistent,
    Failed,
}

impl FantasyProviderCapabilityState {
    fn requires_freshness_contract(self) -> bool {
        matches!(self, Self::Fresh | Self::Stale)
    }

    fn requires_recovery(self) -> bool {
        !matches!(self, Self::Fresh)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyProviderCapabilityRow {
    pub capability: FantasyProviderCapability,
    pub required: bool,
    pub state: FantasyProviderCapabilityState,
    pub fetched_at: Option<String>,
    pub age_seconds: Option<u64>,
    pub freshness_policy_version: Option<String>,
    pub detail: String,
    pub recovery_command: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyProviderPlayerMappingCounts {
    pub total: usize,
    pub resolved: usize,
    pub ambiguous: usize,
    pub unresolved: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyProviderStatusView {
    pub schema: String,
    pub provider: FantasyProviderKind,
    pub access_state: FantasyProviderAccessState,
    pub connection_state: FantasyProviderConnectionState,
    pub remote_league_label: Option<String>,
    pub local_league_label: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub capabilities: Vec<FantasyProviderCapabilityRow>,
    pub player_mappings: FantasyProviderPlayerMappingCounts,
    pub warnings: Vec<String>,
    pub recovery_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FantasyProviderStatusError {
    #[error("unsupported fantasy provider status schema '{0}'")]
    UnsupportedSchema(String),
    #[error("Yahoo cannot be connected before Fantasy API access is granted")]
    ConnectedWithoutAccess,
    #[error("{0} must not be blank when present")]
    BlankOptionalField(&'static str),
    #[error("last_success_at requires last_attempt_at")]
    SuccessWithoutAttempt,
    #[error("duplicate fantasy provider capability '{0:?}'")]
    DuplicateCapability(FantasyProviderCapability),
    #[error("capability '{0:?}' requires fetched_at, age_seconds, and freshness_policy_version")]
    MissingFreshnessContract(FantasyProviderCapability),
    #[error("capability '{0:?}' requires a recovery command in state '{1:?}'")]
    MissingCapabilityRecovery(FantasyProviderCapability, FantasyProviderCapabilityState),
    #[error("player mapping counts do not reconcile: total={total}, resolved={resolved}, ambiguous={ambiguous}, unresolved={unresolved}")]
    PlayerMappingsDoNotReconcile {
        total: usize,
        resolved: usize,
        ambiguous: usize,
        unresolved: usize,
    },
    #[error("fantasy provider status requires a recovery command")]
    MissingRecoveryCommand,
    #[error("fantasy provider warning must not be blank")]
    BlankWarning,
}

impl FantasyProviderStatusView {
    pub fn validate(&self) -> Result<(), FantasyProviderStatusError> {
        if self.schema != FANTASY_PROVIDER_STATUS_SCHEMA {
            return Err(FantasyProviderStatusError::UnsupportedSchema(
                self.schema.clone(),
            ));
        }
        if self.connection_state == FantasyProviderConnectionState::Connected
            && self.access_state != FantasyProviderAccessState::Granted
        {
            return Err(FantasyProviderStatusError::ConnectedWithoutAccess);
        }

        validate_optional_text("remote_league_label", self.remote_league_label.as_deref())?;
        validate_optional_text("local_league_label", self.local_league_label.as_deref())?;
        validate_optional_text("last_attempt_at", self.last_attempt_at.as_deref())?;
        validate_optional_text("last_success_at", self.last_success_at.as_deref())?;
        if self.last_success_at.is_some() && self.last_attempt_at.is_none() {
            return Err(FantasyProviderStatusError::SuccessWithoutAttempt);
        }

        let mut capabilities = BTreeSet::new();
        for row in &self.capabilities {
            if !capabilities.insert(row.capability) {
                return Err(FantasyProviderStatusError::DuplicateCapability(
                    row.capability,
                ));
            }
            validate_optional_text("capability.fetched_at", row.fetched_at.as_deref())?;
            validate_optional_text(
                "capability.freshness_policy_version",
                row.freshness_policy_version.as_deref(),
            )?;
            validate_optional_text(
                "capability.recovery_command",
                row.recovery_command.as_deref(),
            )?;
            if row.state.requires_freshness_contract()
                && (row.fetched_at.is_none()
                    || row.age_seconds.is_none()
                    || row.freshness_policy_version.is_none())
            {
                return Err(FantasyProviderStatusError::MissingFreshnessContract(
                    row.capability,
                ));
            }
            if row.state.requires_recovery() && row.recovery_command.is_none() {
                return Err(FantasyProviderStatusError::MissingCapabilityRecovery(
                    row.capability,
                    row.state,
                ));
            }
            if row.detail.trim().is_empty() {
                return Err(FantasyProviderStatusError::BlankOptionalField(
                    "capability.detail",
                ));
            }
        }

        let mappings = self.player_mappings;
        if mappings.resolved + mappings.ambiguous + mappings.unresolved != mappings.total {
            return Err(FantasyProviderStatusError::PlayerMappingsDoNotReconcile {
                total: mappings.total,
                resolved: mappings.resolved,
                ambiguous: mappings.ambiguous,
                unresolved: mappings.unresolved,
            });
        }
        if self.recovery_command.trim().is_empty() {
            return Err(FantasyProviderStatusError::MissingRecoveryCommand);
        }
        if self
            .warnings
            .iter()
            .any(|warning| warning.trim().is_empty())
        {
            return Err(FantasyProviderStatusError::BlankWarning);
        }
        Ok(())
    }
}

fn validate_optional_text(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), FantasyProviderStatusError> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        return Err(FantasyProviderStatusError::BlankOptionalField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_review() -> FantasyProviderStatusView {
        FantasyProviderStatusView {
            schema: FANTASY_PROVIDER_STATUS_SCHEMA.to_owned(),
            provider: FantasyProviderKind::Yahoo,
            access_state: FantasyProviderAccessState::PendingReview,
            connection_state: FantasyProviderConnectionState::Disconnected,
            remote_league_label: None,
            local_league_label: Some("Felix's Five-Hole 2026-27".to_owned()),
            last_attempt_at: None,
            last_success_at: None,
            capabilities: vec![FantasyProviderCapabilityRow {
                capability: FantasyProviderCapability::Rosters,
                required: true,
                state: FantasyProviderCapabilityState::Blocked,
                fetched_at: None,
                age_seconds: None,
                freshness_policy_version: None,
                detail: "Yahoo Fantasy API access review is pending".to_owned(),
                recovery_command: Some("icelines fantasy yahoo status".to_owned()),
            }],
            player_mappings: FantasyProviderPlayerMappingCounts::default(),
            warnings: vec![
                "Generic Yahoo registration does not grant Fantasy API access".to_owned(),
            ],
            recovery_command: "icelines fantasy yahoo status".to_owned(),
        }
    }

    #[test]
    fn pending_access_is_valid_without_fabricating_sync_freshness() {
        assert_eq!(pending_review().validate(), Ok(()));
    }

    #[test]
    fn connected_state_requires_granted_access() {
        let mut view = pending_review();
        view.connection_state = FantasyProviderConnectionState::Connected;
        assert_eq!(
            view.validate(),
            Err(FantasyProviderStatusError::ConnectedWithoutAccess)
        );
    }

    #[test]
    fn fresh_capability_requires_timestamp_age_and_policy_version() {
        let mut view = pending_review();
        view.access_state = FantasyProviderAccessState::Granted;
        view.connection_state = FantasyProviderConnectionState::Connected;
        view.capabilities[0].state = FantasyProviderCapabilityState::Fresh;
        view.capabilities[0].recovery_command = None;
        assert_eq!(
            view.validate(),
            Err(FantasyProviderStatusError::MissingFreshnessContract(
                FantasyProviderCapability::Rosters
            ))
        );
    }

    #[test]
    fn duplicate_capabilities_are_rejected() {
        let mut view = pending_review();
        view.capabilities.push(view.capabilities[0].clone());
        assert_eq!(
            view.validate(),
            Err(FantasyProviderStatusError::DuplicateCapability(
                FantasyProviderCapability::Rosters
            ))
        );
    }

    #[test]
    fn mapping_counts_must_reconcile() {
        let mut view = pending_review();
        view.player_mappings = FantasyProviderPlayerMappingCounts {
            total: 3,
            resolved: 1,
            ambiguous: 1,
            unresolved: 0,
        };
        assert!(matches!(
            view.validate(),
            Err(FantasyProviderStatusError::PlayerMappingsDoNotReconcile { .. })
        ));
    }

    #[test]
    fn json_contract_contains_no_credential_fields() {
        let json = serde_json::to_string(&pending_review()).expect("status should serialize");
        assert!(!json.contains("client_id"));
        assert!(!json.contains("client_secret"));
        assert!(!json.contains("access_token"));
        assert!(!json.contains("refresh_token"));
    }
}
