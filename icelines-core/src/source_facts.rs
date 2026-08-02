//! Source-neutral, immutable evidence and fact-package contracts.
//!
//! These records live beside `StatsRepository`. They do not mutate player
//! identity, season statistics, organization state, or product scores merely
//! because a provider emitted a row.

use crate::identity::{GameId, PlayerId};
use crate::model::{Season, TeamAbbr};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

pub const SOURCE_PACKAGE_SCHEMA: &str = "icelines_source_package.v1";
pub const SOURCE_PACKAGE_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/icelines_source_package.v1.schema.json");

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceContractError {
    #[error("{0} must not be empty")]
    Empty(&'static str),
    #[error("content hash must be 64 lowercase hexadecimal characters")]
    InvalidContentHash,
    #[error("source URL must be an absolute http(s) URL")]
    InvalidSourceUrl,
    #[error("canonical player id must be non-zero")]
    ZeroPlayerId,
    #[error("fact evidence must not be empty")]
    EmptyEvidence,
    #[error("effective interval ends before it starts")]
    InvalidEffectiveInterval,
    #[error("duplicate {kind} id {id}")]
    DuplicateId { kind: &'static str, id: String },
    #[error("fact {fact_id} occurs after package effective cutoff")]
    FactAfterEffectiveCutoff { fact_id: String },
    #[error("evidence {source_id} was captured after package knowledge cutoff")]
    EvidenceAfterKnowledgeCutoff { source_id: String },
    #[error("identity decision {decision_id} was reviewed after package knowledge cutoff")]
    DecisionAfterKnowledgeCutoff { decision_id: String },
    #[error("identity action requires a canonical player id")]
    MissingDecisionPlayer,
    #[error("rejected identity decision must not assign a canonical player id")]
    RejectedDecisionHasPlayer,
    #[error("run-manifest complete flag disagrees with object outcomes")]
    InvalidRunCompleteness,
    #[error("coverage invariant failed: {0}")]
    InvalidCoverage(String),
    #[error("source package schema {found} is unsupported")]
    UnsupportedSchema { found: String },
    #[error("source package fingerprint mismatch")]
    FingerprintMismatch,
    #[error("identity decision {decision_id} references unknown proposal {proposal_id}")]
    UnknownDecisionProposal {
        decision_id: String,
        proposal_id: String,
    },
    #[error("staged assertion {assertion_id} references unknown proposal {proposal_id}")]
    UnknownStagedProposal {
        assertion_id: String,
        proposal_id: String,
    },
    #[error("conflict {conflict_id} must reference at least two known facts")]
    InvalidConflict { conflict_id: String },
}

macro_rules! validated_string {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn try_new(value: impl Into<String>) -> Result<Self, SourceContractError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(SourceContractError::Empty($label));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_new(value).map_err(serde::de::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

validated_string!(SourceId, "source_id");
validated_string!(ProviderId, "provider");
validated_string!(AdapterId, "adapter_id");
validated_string!(AdapterVersion, "adapter_version");
validated_string!(PolicyVersion, "policy_version");
validated_string!(FactId, "fact_id");
validated_string!(StagedAssertionId, "staged_assertion_id");
validated_string!(ProposalId, "proposal_id");
validated_string!(DecisionId, "decision_id");
validated_string!(PackageId, "package_id");
validated_string!(OrganizationId, "organization_id");
validated_string!(ClubRef, "club_ref");
validated_string!(LeagueCode, "league_code");

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn try_new(value: impl Into<String>) -> Result<Self, SourceContractError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SourceContractError::InvalidContentHash);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SourceUrl(String);

impl SourceUrl {
    pub fn try_new(value: impl Into<String>) -> Result<Self, SourceContractError> {
        let value = value.into();
        if !(value.starts_with("https://") || value.starts_with("http://")) {
            return Err(SourceContractError::InvalidSourceUrl);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SourceUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEvidence {
    source_id: SourceId,
    source_url: SourceUrl,
    provider: ProviderId,
    captured_at: DateTime<Utc>,
    content_sha256: ContentHash,
    adapter_version: AdapterVersion,
}

impl SourceEvidence {
    pub fn new(
        source_id: SourceId,
        source_url: SourceUrl,
        provider: ProviderId,
        captured_at: DateTime<Utc>,
        content_sha256: ContentHash,
        adapter_version: AdapterVersion,
    ) -> Self {
        Self {
            source_id,
            source_url,
            provider,
            captured_at,
            content_sha256,
            adapter_version,
        }
    }

    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    pub fn source_url(&self) -> &SourceUrl {
        &self.source_url
    }

    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    pub fn captured_at(&self) -> DateTime<Utc> {
        self.captured_at
    }

    pub fn content_sha256(&self) -> &ContentHash {
        &self.content_sha256
    }

    pub fn adapter_version(&self) -> &AdapterVersion {
        &self.adapter_version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectivePrecision {
    Instant,
    Day,
    Season,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveTime {
    pub starts_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<DateTime<Utc>>,
    pub precision: EffectivePrecision,
}

impl EffectiveTime {
    pub fn new(
        starts_at: DateTime<Utc>,
        ends_at: Option<DateTime<Utc>>,
        precision: EffectivePrecision,
    ) -> Result<Self, SourceContractError> {
        if ends_at.is_some_and(|end| end < starts_at) {
            return Err(SourceContractError::InvalidEffectiveInterval);
        }
        Ok(Self {
            starts_at,
            ends_at,
            precision,
        })
    }

    fn validate(&self) -> Result<(), SourceContractError> {
        if self.ends_at.is_some_and(|end| end < self.starts_at) {
            return Err(SourceContractError::InvalidEffectiveInterval);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum FactSubject {
    Player(PlayerId),
    Organization(OrganizationId),
    Team(TeamAbbr),
    Game(GameId),
    League(LeagueCode),
}

impl FactSubject {
    fn validate(&self) -> Result<(), SourceContractError> {
        if matches!(self, Self::Player(PlayerId(0))) {
            return Err(SourceContractError::ZeroPlayerId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactAuthority {
    Draft,
    Attendance,
    Contract,
    LegalControl,
    Assignment,
    Context,
    Compatibility,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FactAssertion<T> {
    fact_id: FactId,
    semantic_key: String,
    subject: FactSubject,
    occurred_at: EffectiveTime,
    authority: FactAuthority,
    fact: T,
    evidence: Vec<SourceEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    supersedes: Vec<FactId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    retracts: Vec<FactId>,
}

#[derive(Deserialize)]
struct FactAssertionWire<T> {
    fact_id: FactId,
    semantic_key: String,
    subject: FactSubject,
    occurred_at: EffectiveTime,
    authority: FactAuthority,
    fact: T,
    evidence: Vec<SourceEvidence>,
    #[serde(default)]
    supersedes: Vec<FactId>,
    #[serde(default)]
    retracts: Vec<FactId>,
}

impl<T> FactAssertion<T> {
    pub fn new(
        fact_id: FactId,
        semantic_key: impl Into<String>,
        subject: FactSubject,
        occurred_at: EffectiveTime,
        authority: FactAuthority,
        fact: T,
        evidence: Vec<SourceEvidence>,
    ) -> Result<Self, SourceContractError> {
        let assertion = Self {
            fact_id,
            semantic_key: semantic_key.into(),
            subject,
            occurred_at,
            authority,
            fact,
            evidence,
            supersedes: Vec::new(),
            retracts: Vec::new(),
        };
        assertion.validate()?;
        Ok(assertion)
    }

    pub fn with_corrections(mut self, supersedes: Vec<FactId>, retracts: Vec<FactId>) -> Self {
        self.supersedes = supersedes;
        self.retracts = retracts;
        self
    }

    pub fn fact_id(&self) -> &FactId {
        &self.fact_id
    }

    pub fn subject(&self) -> &FactSubject {
        &self.subject
    }

    pub fn occurred_at(&self) -> &EffectiveTime {
        &self.occurred_at
    }

    pub fn authority(&self) -> FactAuthority {
        self.authority
    }

    pub fn fact(&self) -> &T {
        &self.fact
    }

    pub fn evidence(&self) -> &[SourceEvidence] {
        &self.evidence
    }

    pub fn supersedes(&self) -> &[FactId] {
        &self.supersedes
    }

    pub fn retracts(&self) -> &[FactId] {
        &self.retracts
    }

    fn validate(&self) -> Result<(), SourceContractError> {
        if self.semantic_key.trim().is_empty() {
            return Err(SourceContractError::Empty("semantic_key"));
        }
        self.subject.validate()?;
        self.occurred_at.validate()?;
        if self.evidence.is_empty() {
            return Err(SourceContractError::EmptyEvidence);
        }
        Ok(())
    }
}

impl<T> TryFrom<FactAssertionWire<T>> for FactAssertion<T> {
    type Error = SourceContractError;

    fn try_from(wire: FactAssertionWire<T>) -> Result<Self, Self::Error> {
        let assertion = Self {
            fact_id: wire.fact_id,
            semantic_key: wire.semantic_key,
            subject: wire.subject,
            occurred_at: wire.occurred_at,
            authority: wire.authority,
            fact: wire.fact,
            evidence: wire.evidence,
            supersedes: wire.supersedes,
            retracts: wire.retracts,
        };
        assertion.validate()?;
        Ok(assertion)
    }
}

impl<'de, T> Deserialize<'de> for FactAssertion<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        FactAssertionWire::<T>::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    EntryLevel,
    StandardPlayer,
    AmateurTryout,
    ProfessionalTryout,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum PlayerOrganizationEvent {
    Drafted {
        by: OrganizationId,
        year: u16,
        round: u8,
        overall: u16,
    },
    ContractSigned {
        with: OrganizationId,
        contract_kind: ContractKind,
    },
    RightsTransferred {
        from: OrganizationId,
        to: OrganizationId,
    },
    RightsExpired {
        organization: OrganizationId,
    },
    Assigned {
        by: OrganizationId,
        to: ClubRef,
    },
    Rostered {
        at: ClubRef,
    },
    /// Observed on a minor-league club whose published NHL affiliate is known.
    /// This establishes discovery and assignment context, never NHL control.
    AffiliateRostered {
        affiliate: OrganizationId,
        at: ClubRef,
    },
    Recalled {
        by: OrganizationId,
        from: ClubRef,
        to: ClubRef,
    },
    Loaned {
        by: OrganizationId,
        to: ClubRef,
    },
    Released {
        by: OrganizationId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipationKind {
    DevelopmentCamp,
    RookieCamp,
    TrainingCamp,
    ProspectTournament,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipationAuthority {
    ControlledPlayer,
    FreeAgentInvite,
    Tryout,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerParticipationFact {
    pub organization: OrganizationId,
    pub season: Season,
    pub kind: ParticipationKind,
    pub authority: ParticipationAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", content = "value", rename_all = "snake_case")]
pub enum SourceFact {
    PlayerOrganization(PlayerOrganizationEvent),
    PlayerParticipation(PlayerParticipationFact),
    CompatibilityProspectRelationship(CompatibilityProspectRelationshipFact),
}

/// A player-scoped hockey fact that cannot receive a canonical `FactSubject`
/// until its provider identity proposal is reviewed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StagedPlayerAssertion {
    assertion_id: StagedAssertionId,
    semantic_key: String,
    proposal_id: ProposalId,
    occurred_at: EffectiveTime,
    authority: FactAuthority,
    fact: SourceFact,
    evidence: Vec<SourceEvidence>,
}

#[derive(Deserialize)]
struct StagedPlayerAssertionWire {
    assertion_id: StagedAssertionId,
    semantic_key: String,
    proposal_id: ProposalId,
    occurred_at: EffectiveTime,
    authority: FactAuthority,
    fact: SourceFact,
    evidence: Vec<SourceEvidence>,
}

impl StagedPlayerAssertion {
    pub fn new(
        assertion_id: StagedAssertionId,
        semantic_key: impl Into<String>,
        proposal_id: ProposalId,
        occurred_at: EffectiveTime,
        authority: FactAuthority,
        fact: SourceFact,
        evidence: Vec<SourceEvidence>,
    ) -> Result<Self, SourceContractError> {
        let assertion = Self {
            assertion_id,
            semantic_key: semantic_key.into(),
            proposal_id,
            occurred_at,
            authority,
            fact,
            evidence,
        };
        assertion.validate()?;
        Ok(assertion)
    }

    pub fn assertion_id(&self) -> &StagedAssertionId {
        &self.assertion_id
    }

    pub fn proposal_id(&self) -> &ProposalId {
        &self.proposal_id
    }

    pub fn occurred_at(&self) -> &EffectiveTime {
        &self.occurred_at
    }

    pub fn semantic_key(&self) -> &str {
        &self.semantic_key
    }

    pub fn authority(&self) -> FactAuthority {
        self.authority
    }

    pub fn fact(&self) -> &SourceFact {
        &self.fact
    }

    pub fn evidence(&self) -> &[SourceEvidence] {
        &self.evidence
    }

    fn validate(&self) -> Result<(), SourceContractError> {
        if self.semantic_key.trim().is_empty() {
            return Err(SourceContractError::Empty("semantic_key"));
        }
        self.occurred_at.validate()?;
        if self.evidence.is_empty() {
            return Err(SourceContractError::EmptyEvidence);
        }
        Ok(())
    }
}

impl TryFrom<StagedPlayerAssertionWire> for StagedPlayerAssertion {
    type Error = SourceContractError;

    fn try_from(wire: StagedPlayerAssertionWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.assertion_id,
            wire.semantic_key,
            wire.proposal_id,
            wire.occurred_at,
            wire.authority,
            wire.fact,
            wire.evidence,
        )
    }
}

impl<'de> Deserialize<'de> for StagedPlayerAssertion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        StagedPlayerAssertionWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityProspectRelationshipKind {
    LegacyOrganizationalCandidate,
    OrganizationRights,
    NhlContract,
    AhlAssignment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityProspectRelationshipFact {
    pub organization: OrganizationId,
    pub relationship: CompatibilityProspectRelationshipKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "locator", rename_all = "snake_case")]
pub enum ProviderPersonLocator {
    StableId {
        provider: ProviderId,
        provider_player_id: String,
    },
    SourceRow {
        source_id: SourceId,
        row_key: String,
    },
}

impl ProviderPersonLocator {
    fn validate(&self) -> Result<(), SourceContractError> {
        let value = match self {
            Self::StableId {
                provider_player_id, ..
            } => provider_player_id,
            Self::SourceRow { row_key, .. } => row_key,
        };
        if value.trim().is_empty() {
            return Err(SourceContractError::Empty("provider_person_locator"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderIdentityProposal {
    proposal_id: ProposalId,
    locator: ProviderPersonLocator,
    displayed_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    birth_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proposed_player_id: Option<PlayerId>,
    evidence: Vec<SourceEvidence>,
}

impl ProviderIdentityProposal {
    pub fn new(
        proposal_id: ProposalId,
        locator: ProviderPersonLocator,
        displayed_name: impl Into<String>,
        birth_date: Option<String>,
        proposed_player_id: Option<PlayerId>,
        evidence: Vec<SourceEvidence>,
    ) -> Result<Self, SourceContractError> {
        let displayed_name = displayed_name.into();
        locator.validate()?;
        if displayed_name.trim().is_empty() {
            return Err(SourceContractError::Empty("displayed_name"));
        }
        if proposed_player_id == Some(PlayerId(0)) {
            return Err(SourceContractError::ZeroPlayerId);
        }
        if evidence.is_empty() {
            return Err(SourceContractError::EmptyEvidence);
        }
        Ok(Self {
            proposal_id,
            locator,
            displayed_name,
            birth_date,
            proposed_player_id,
            evidence,
        })
    }

    pub fn proposal_id(&self) -> &ProposalId {
        &self.proposal_id
    }

    pub fn displayed_name(&self) -> &str {
        &self.displayed_name
    }

    pub fn birth_date(&self) -> Option<&str> {
        self.birth_date.as_deref()
    }

    pub fn locator(&self) -> &ProviderPersonLocator {
        &self.locator
    }

    pub fn proposed_player_id(&self) -> Option<PlayerId> {
        self.proposed_player_id
    }

    pub fn evidence(&self) -> &[SourceEvidence] {
        &self.evidence
    }

    fn validate(&self) -> Result<(), SourceContractError> {
        self.locator.validate()?;
        Self::new(
            self.proposal_id.clone(),
            self.locator.clone(),
            self.displayed_name.clone(),
            self.birth_date.clone(),
            self.proposed_player_id,
            self.evidence.clone(),
        )
        .map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityReviewAction {
    AcceptProposal,
    SetIdentity,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityReviewDecision {
    decision_id: DecisionId,
    proposal_id: ProposalId,
    action: IdentityReviewAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    canonical_player_id: Option<PlayerId>,
    reviewer: String,
    reviewed_at: DateTime<Utc>,
    rationale: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence: Vec<SourceEvidence>,
}

impl IdentityReviewDecision {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        decision_id: DecisionId,
        proposal_id: ProposalId,
        action: IdentityReviewAction,
        canonical_player_id: Option<PlayerId>,
        reviewer: impl Into<String>,
        reviewed_at: DateTime<Utc>,
        rationale: impl Into<String>,
        evidence: Vec<SourceEvidence>,
    ) -> Result<Self, SourceContractError> {
        let decision = Self {
            decision_id,
            proposal_id,
            action,
            canonical_player_id,
            reviewer: reviewer.into(),
            reviewed_at,
            rationale: rationale.into(),
            evidence,
        };
        decision.validate()?;
        Ok(decision)
    }

    pub fn decision_id(&self) -> &DecisionId {
        &self.decision_id
    }

    pub fn proposal_id(&self) -> &ProposalId {
        &self.proposal_id
    }

    pub fn action(&self) -> IdentityReviewAction {
        self.action
    }

    pub fn canonical_player_id(&self) -> Option<PlayerId> {
        self.canonical_player_id
    }

    pub fn reviewed_at(&self) -> DateTime<Utc> {
        self.reviewed_at
    }

    fn validate(&self) -> Result<(), SourceContractError> {
        if self.reviewer.trim().is_empty() {
            return Err(SourceContractError::Empty("reviewer"));
        }
        if self.rationale.trim().is_empty() {
            return Err(SourceContractError::Empty("rationale"));
        }
        if self.canonical_player_id == Some(PlayerId(0)) {
            return Err(SourceContractError::ZeroPlayerId);
        }
        match (self.action, self.canonical_player_id) {
            (IdentityReviewAction::Reject, Some(_)) => {
                Err(SourceContractError::RejectedDecisionHasPlayer)
            }
            (IdentityReviewAction::Reject, None) => Ok(()),
            (_, None) => Err(SourceContractError::MissingDecisionPlayer),
            (_, Some(_)) => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessClass {
    Static,
    Transactional,
    Roster,
    Live,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessStatus {
    Fresh,
    Stale,
    Static,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFreshness {
    pub class: FreshnessClass,
    pub captured_at: DateTime<Utc>,
    pub evaluated_at: DateTime<Utc>,
    pub status: FreshnessStatus,
    pub policy_version: PolicyVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInputRecord {
    pub evidence: SourceEvidence,
    pub freshness: SourceFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SourceObjectState {
    Acquired { records: usize },
    NotApplicable { reason: String },
    Failed { reason: String },
    Quarantined { reason: String },
    IncompletePagination,
}

impl SourceObjectState {
    fn is_complete(&self) -> bool {
        matches!(self, Self::Acquired { .. } | Self::NotApplicable { .. })
    }

    fn validate(&self) -> Result<(), SourceContractError> {
        let reason = match self {
            Self::NotApplicable { reason }
            | Self::Failed { reason }
            | Self::Quarantined { reason } => Some(reason),
            Self::Acquired { .. } | Self::IncompletePagination => None,
        };
        if reason.is_some_and(|reason| reason.trim().is_empty()) {
            return Err(SourceContractError::Empty("source_object_reason"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObjectOutcome {
    pub object_id: String,
    pub source_family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<OrganizationId>,
    pub terminal_pagination: bool,
    pub state: SourceObjectState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRunManifest {
    pub requested_scope: String,
    pub source_catalog_version: String,
    pub objects: Vec<SourceObjectOutcome>,
    pub complete: bool,
}

impl SourceRunManifest {
    pub fn validate(&self) -> Result<(), SourceContractError> {
        if self.requested_scope.trim().is_empty() {
            return Err(SourceContractError::Empty("requested_scope"));
        }
        let actual_complete = self.objects.iter().all(|object| {
            object.state.is_complete()
                && (matches!(object.state, SourceObjectState::NotApplicable { .. })
                    || object.terminal_pagination)
        });
        if self.complete != actual_complete {
            return Err(SourceContractError::InvalidRunCompleteness);
        }
        for object in &self.objects {
            if object.object_id.trim().is_empty() {
                return Err(SourceContractError::Empty("source_object_id"));
            }
            if object.source_family.trim().is_empty() {
                return Err(SourceContractError::Empty("source_family"));
            }
            object.state.validate()?;
        }
        unique_ids(
            self.objects.iter().map(|object| object.object_id.as_str()),
            "source_object",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SourceCoverageBucket {
    pub source_family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<OrganizationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_group: Option<String>,
    pub expected: usize,
    pub acquired: usize,
    pub parsed: usize,
    pub quarantined: usize,
    pub resolved: usize,
    pub conflicted: usize,
    pub excluded: usize,
}

impl SourceCoverageBucket {
    fn validate(&self) -> Result<(), SourceContractError> {
        if self.source_family.trim().is_empty() {
            return Err(SourceContractError::Empty("source_family"));
        }
        if self.acquired > self.expected
            || self.parsed + self.quarantined > self.acquired
            || self.resolved + self.conflicted + self.excluded > self.parsed
        {
            return Err(SourceContractError::InvalidCoverage(format!(
                "{} counts do not reconcile",
                self.source_family
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDisclosureCode {
    PartialPopulation,
    MissingSourceFamily,
    StaleSource,
    UnresolvedIdentity,
    ConflictingControl,
    ParticipationOnly,
    HistoricalCutoff,
    RightsPolicy,
    UnsupportedLeague,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDisclosure {
    pub code: SourceDisclosureCode,
    pub scope: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceConflict {
    pub conflict_id: String,
    pub semantic_key: String,
    pub fact_ids: Vec<FactId>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceExclusion {
    pub exclusion_id: String,
    pub stage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<FactSubject>,
    pub reason_code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<SourceId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourcePackage {
    pub schema: String,
    pub package_id: PackageId,
    pub evaluation_season: Season,
    pub effective_cutoff: DateTime<Utc>,
    pub knowledge_cutoff: DateTime<Utc>,
    pub adapter_registry_version: AdapterVersion,
    pub reconciliation_policy_version: PolicyVersion,
    pub review_registry_fingerprint: ContentHash,
    pub run_manifest: SourceRunManifest,
    pub inputs: Vec<SourceInputRecord>,
    pub fact_assertions: Vec<FactAssertion<SourceFact>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub staged_player_assertions: Vec<StagedPlayerAssertion>,
    pub identity_proposals: Vec<ProviderIdentityProposal>,
    pub identity_review_decisions: Vec<IdentityReviewDecision>,
    pub conflicts: Vec<SourceConflict>,
    pub exclusions: Vec<SourceExclusion>,
    pub coverage: Vec<SourceCoverageBucket>,
    pub disclosures: Vec<SourceDisclosure>,
    pub fingerprint: ContentHash,
}

impl SourcePackage {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        package_id: PackageId,
        evaluation_season: Season,
        effective_cutoff: DateTime<Utc>,
        knowledge_cutoff: DateTime<Utc>,
        adapter_registry_version: AdapterVersion,
        reconciliation_policy_version: PolicyVersion,
        review_registry_fingerprint: ContentHash,
        run_manifest: SourceRunManifest,
        inputs: Vec<SourceInputRecord>,
        fact_assertions: Vec<FactAssertion<SourceFact>>,
        identity_proposals: Vec<ProviderIdentityProposal>,
        identity_review_decisions: Vec<IdentityReviewDecision>,
        conflicts: Vec<SourceConflict>,
        exclusions: Vec<SourceExclusion>,
        coverage: Vec<SourceCoverageBucket>,
        disclosures: Vec<SourceDisclosure>,
    ) -> Result<Self, SourceContractError> {
        let mut package = Self {
            schema: SOURCE_PACKAGE_SCHEMA.to_owned(),
            package_id,
            evaluation_season,
            effective_cutoff,
            knowledge_cutoff,
            adapter_registry_version,
            reconciliation_policy_version,
            review_registry_fingerprint,
            run_manifest,
            inputs,
            fact_assertions,
            staged_player_assertions: Vec::new(),
            identity_proposals,
            identity_review_decisions,
            conflicts,
            exclusions,
            coverage,
            disclosures,
            fingerprint: ContentHash::try_new("0".repeat(64))?,
        };
        package.canonicalize();
        package.validate_without_fingerprint()?;
        package.fingerprint = package.compute_fingerprint()?;
        Ok(package)
    }

    /// Adds unresolved player facts while preserving the original `seal` API
    /// for source packages that contain only canonical assertions.
    pub fn with_staged_player_assertions(
        mut self,
        staged_player_assertions: Vec<StagedPlayerAssertion>,
    ) -> Result<Self, SourceContractError> {
        self.staged_player_assertions = staged_player_assertions;
        self.fingerprint = ContentHash::try_new("0".repeat(64))?;
        self.canonicalize();
        self.validate_without_fingerprint()?;
        self.fingerprint = self.compute_fingerprint()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), SourceContractError> {
        self.validate_without_fingerprint()?;
        if self.compute_fingerprint()? != self.fingerprint {
            return Err(SourceContractError::FingerprintMismatch);
        }
        Ok(())
    }

    fn validate_without_fingerprint(&self) -> Result<(), SourceContractError> {
        if self.schema != SOURCE_PACKAGE_SCHEMA {
            return Err(SourceContractError::UnsupportedSchema {
                found: self.schema.clone(),
            });
        }
        self.run_manifest.validate()?;
        unique_ids(
            self.fact_assertions
                .iter()
                .map(|assertion| assertion.fact_id.as_str()),
            "fact",
        )?;
        unique_ids(
            self.staged_player_assertions
                .iter()
                .map(|assertion| assertion.assertion_id.as_str()),
            "staged_assertion",
        )?;
        unique_ids(
            self.identity_proposals
                .iter()
                .map(|proposal| proposal.proposal_id.as_str()),
            "proposal",
        )?;
        unique_ids(
            self.identity_review_decisions
                .iter()
                .map(|decision| decision.decision_id.as_str()),
            "decision",
        )?;
        unique_ids(
            self.identity_review_decisions
                .iter()
                .map(|decision| decision.proposal_id.as_str()),
            "decision_proposal",
        )?;
        unique_ids(
            self.conflicts
                .iter()
                .map(|conflict| conflict.conflict_id.as_str()),
            "conflict",
        )?;
        unique_ids(
            self.exclusions
                .iter()
                .map(|exclusion| exclusion.exclusion_id.as_str()),
            "exclusion",
        )?;
        let proposal_ids = self
            .identity_proposals
            .iter()
            .map(|proposal| proposal.proposal_id.as_str())
            .collect::<BTreeSet<_>>();
        for decision in &self.identity_review_decisions {
            if !proposal_ids.contains(decision.proposal_id.as_str()) {
                return Err(SourceContractError::UnknownDecisionProposal {
                    decision_id: decision.decision_id.to_string(),
                    proposal_id: decision.proposal_id.to_string(),
                });
            }
        }
        for assertion in &self.staged_player_assertions {
            if !proposal_ids.contains(assertion.proposal_id.as_str()) {
                return Err(SourceContractError::UnknownStagedProposal {
                    assertion_id: assertion.assertion_id.to_string(),
                    proposal_id: assertion.proposal_id.to_string(),
                });
            }
        }
        let fact_ids = self
            .fact_assertions
            .iter()
            .map(|fact| fact.fact_id.as_str())
            .collect::<BTreeSet<_>>();
        for conflict in &self.conflicts {
            if conflict.semantic_key.trim().is_empty()
                || conflict.reason.trim().is_empty()
                || conflict.fact_ids.len() < 2
                || conflict
                    .fact_ids
                    .iter()
                    .any(|fact_id| !fact_ids.contains(fact_id.as_str()))
            {
                return Err(SourceContractError::InvalidConflict {
                    conflict_id: conflict.conflict_id.clone(),
                });
            }
        }
        for exclusion in &self.exclusions {
            if exclusion.stage.trim().is_empty()
                || exclusion.reason_code.trim().is_empty()
                || exclusion.message.trim().is_empty()
            {
                return Err(SourceContractError::Empty("source_exclusion_field"));
            }
            if let Some(subject) = &exclusion.subject {
                subject.validate()?;
            }
        }
        for input in &self.inputs {
            if input.evidence.captured_at > self.knowledge_cutoff {
                return Err(SourceContractError::EvidenceAfterKnowledgeCutoff {
                    source_id: input.evidence.source_id.to_string(),
                });
            }
            if input.freshness.captured_at != input.evidence.captured_at {
                return Err(SourceContractError::InvalidCoverage(
                    "input evidence and freshness capture time differ".to_owned(),
                ));
            }
            if input.freshness.evaluated_at != self.knowledge_cutoff {
                return Err(SourceContractError::InvalidCoverage(
                    "freshness must be evaluated at the package knowledge cutoff".to_owned(),
                ));
            }
        }
        for assertion in &self.fact_assertions {
            assertion.validate()?;
            if assertion.occurred_at.starts_at > self.effective_cutoff {
                return Err(SourceContractError::FactAfterEffectiveCutoff {
                    fact_id: assertion.fact_id.to_string(),
                });
            }
            for evidence in &assertion.evidence {
                if evidence.captured_at > self.knowledge_cutoff {
                    return Err(SourceContractError::EvidenceAfterKnowledgeCutoff {
                        source_id: evidence.source_id.to_string(),
                    });
                }
            }
        }
        for assertion in &self.staged_player_assertions {
            assertion.validate()?;
            if assertion.occurred_at.starts_at > self.effective_cutoff {
                return Err(SourceContractError::FactAfterEffectiveCutoff {
                    fact_id: assertion.assertion_id.to_string(),
                });
            }
            for evidence in &assertion.evidence {
                if evidence.captured_at > self.knowledge_cutoff {
                    return Err(SourceContractError::EvidenceAfterKnowledgeCutoff {
                        source_id: evidence.source_id.to_string(),
                    });
                }
            }
        }
        for proposal in &self.identity_proposals {
            proposal.validate()?;
        }
        for decision in &self.identity_review_decisions {
            decision.validate()?;
            if decision.reviewed_at > self.knowledge_cutoff {
                return Err(SourceContractError::DecisionAfterKnowledgeCutoff {
                    decision_id: decision.decision_id.to_string(),
                });
            }
        }
        for bucket in &self.coverage {
            bucket.validate()?;
        }
        for disclosure in &self.disclosures {
            if disclosure.scope.trim().is_empty() || disclosure.message.trim().is_empty() {
                return Err(SourceContractError::Empty("source_disclosure_field"));
            }
        }
        Ok(())
    }

    fn canonicalize(&mut self) {
        self.inputs.sort_by(|left, right| {
            left.evidence
                .source_id
                .cmp(&right.evidence.source_id)
                .then_with(|| {
                    left.evidence
                        .content_sha256
                        .cmp(&right.evidence.content_sha256)
                })
        });
        self.fact_assertions
            .sort_by(|left, right| left.fact_id.cmp(&right.fact_id));
        self.staged_player_assertions
            .sort_by(|left, right| left.assertion_id.cmp(&right.assertion_id));
        self.identity_proposals
            .sort_by(|left, right| left.proposal_id.cmp(&right.proposal_id));
        self.identity_review_decisions
            .sort_by(|left, right| left.decision_id.cmp(&right.decision_id));
        self.conflicts
            .sort_by(|left, right| left.conflict_id.cmp(&right.conflict_id));
        self.exclusions
            .sort_by(|left, right| left.exclusion_id.cmp(&right.exclusion_id));
        self.coverage.sort_by(|left, right| {
            left.source_family
                .cmp(&right.source_family)
                .then_with(|| left.organization.cmp(&right.organization))
                .then_with(|| left.player_class.cmp(&right.player_class))
                .then_with(|| left.position_group.cmp(&right.position_group))
        });
        self.disclosures.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| left.scope.cmp(&right.scope))
                .then_with(|| left.message.cmp(&right.message))
        });
    }

    fn compute_fingerprint(&self) -> Result<ContentHash, SourceContractError> {
        let mut canonical = self.clone();
        canonical.canonicalize();
        canonical.fingerprint = ContentHash::try_new("0".repeat(64))?;
        let bytes = serde_json::to_vec(&canonical).map_err(|_| {
            SourceContractError::InvalidCoverage("package serialization failed".to_owned())
        })?;
        ContentHash::try_new(format!("{:x}", Sha256::digest(bytes)))
    }
}

fn unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    kind: &'static str,
) -> Result<(), SourceContractError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(SourceContractError::DuplicateId {
                kind,
                id: id.to_owned(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn at(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, day, 12, 0, 0)
            .single()
            .unwrap()
    }

    fn hash(byte: char) -> ContentHash {
        ContentHash::try_new(byte.to_string().repeat(64)).unwrap()
    }

    fn evidence(id: &str, captured_at: DateTime<Utc>, byte: char) -> SourceEvidence {
        SourceEvidence::new(
            SourceId::try_new(id).unwrap(),
            SourceUrl::try_new(format!("https://example.com/{id}")).unwrap(),
            ProviderId::try_new("fixture").unwrap(),
            captured_at,
            hash(byte),
            AdapterVersion::try_new("fixture.v1").unwrap(),
        )
    }

    fn drafted_fact(id: &str, occurred_at: DateTime<Utc>) -> FactAssertion<SourceFact> {
        FactAssertion::new(
            FactId::try_new(id).unwrap(),
            format!("player:8484144:draft:{id}"),
            FactSubject::Player(PlayerId::try_new(8_484_144).unwrap()),
            EffectiveTime::new(occurred_at, None, EffectivePrecision::Day).unwrap(),
            FactAuthority::Draft,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                by: OrganizationId::try_new("CHI").unwrap(),
                year: 2023,
                round: 1,
                overall: 1,
            }),
            vec![evidence("draft", at(20), 'a')],
        )
        .unwrap()
    }

    fn run_manifest(state: SourceObjectState, complete: bool) -> SourceRunManifest {
        SourceRunManifest {
            requested_scope: "2026-27 all-32 prospect census".to_owned(),
            source_catalog_version: "catalog.v1".to_owned(),
            objects: vec![SourceObjectOutcome {
                object_id: "draft-ledger".to_owned(),
                source_family: "nhl_draft".to_owned(),
                organization: None,
                terminal_pagination: true,
                state,
            }],
            complete,
        }
    }

    fn package_with(
        inputs: Vec<SourceInputRecord>,
        facts: Vec<FactAssertion<SourceFact>>,
    ) -> SourcePackage {
        package_with_proposals(inputs, facts, Vec::new())
    }

    fn package_with_proposals(
        inputs: Vec<SourceInputRecord>,
        facts: Vec<FactAssertion<SourceFact>>,
        proposals: Vec<ProviderIdentityProposal>,
    ) -> SourcePackage {
        SourcePackage::seal(
            PackageId::try_new("sources-2026-27").unwrap(),
            Season(20_262_027),
            at(31),
            at(31),
            AdapterVersion::try_new("registry.v1").unwrap(),
            PolicyVersion::try_new("reconcile.v1").unwrap(),
            hash('f'),
            run_manifest(
                SourceObjectState::Acquired {
                    records: facts.len(),
                },
                true,
            ),
            inputs,
            facts,
            proposals,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![SourceCoverageBucket {
                source_family: "nhl_draft".to_owned(),
                expected: 1,
                acquired: 1,
                parsed: 1,
                resolved: 1,
                ..SourceCoverageBucket::default()
            }],
            vec![SourceDisclosure {
                code: SourceDisclosureCode::HistoricalCutoff,
                scope: "league".to_owned(),
                message: "Facts are bounded by both package cutoffs.".to_owned(),
            }],
        )
        .unwrap()
    }

    fn camp_staged_assertion(proposal_id: &str) -> StagedPlayerAssertion {
        StagedPlayerAssertion::new(
            StagedAssertionId::try_new("staged-camp-1").unwrap(),
            "proposal:camp-player:training-camp",
            ProposalId::try_new(proposal_id).unwrap(),
            EffectiveTime::new(at(20), None, EffectivePrecision::Day).unwrap(),
            FactAuthority::Attendance,
            SourceFact::PlayerParticipation(PlayerParticipationFact {
                organization: OrganizationId::try_new("SEA").unwrap(),
                season: Season(20_262_027),
                kind: ParticipationKind::TrainingCamp,
                authority: ParticipationAuthority::Unknown,
            }),
            vec![evidence("camp", at(20), 'b')],
        )
        .unwrap()
    }

    #[test]
    fn source_assertion_rejects_zero_player_and_empty_evidence_on_deserialize() {
        let invalid_zero = serde_json::json!({
            "fact_id": "fact-1",
            "semantic_key": "player:0:camp",
            "subject": {"kind": "player", "id": 0},
            "occurred_at": {"starts_at": at(1), "precision": "day"},
            "authority": "attendance",
            "fact": {"family": "player_participation", "value": {
                "organization": "SEA", "season": 20262027,
                "kind": "training_camp", "authority": "unknown"
            }},
            "evidence": [{
                "source_id": "camp", "source_url": "https://example.com/camp",
                "provider": "fixture", "captured_at": at(1),
                "content_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "adapter_version": "v1"
            }]
        });
        assert!(serde_json::from_value::<FactAssertion<SourceFact>>(invalid_zero).is_err());

        let empty = FactAssertion::new(
            FactId::try_new("fact-2").unwrap(),
            "player:1:camp",
            FactSubject::Player(PlayerId::try_new(1).unwrap()),
            EffectiveTime::new(at(1), None, EffectivePrecision::Day).unwrap(),
            FactAuthority::Attendance,
            SourceFact::PlayerParticipation(PlayerParticipationFact {
                organization: OrganizationId::try_new("SEA").unwrap(),
                season: Season(20_262_027),
                kind: ParticipationKind::TrainingCamp,
                authority: ParticipationAuthority::Unknown,
            }),
            Vec::new(),
        );
        assert_eq!(empty.unwrap_err(), SourceContractError::EmptyEvidence);
    }

    #[test]
    fn identity_proposal_and_review_decision_are_separate_append_only_records() {
        let proposal = ProviderIdentityProposal::new(
            ProposalId::try_new("proposal-1").unwrap(),
            ProviderPersonLocator::SourceRow {
                source_id: SourceId::try_new("camp").unwrap(),
                row_key: "row-12".to_owned(),
            },
            "Camp Player",
            None,
            None,
            vec![evidence("camp", at(20), 'b')],
        )
        .unwrap();
        let decision = IdentityReviewDecision::new(
            DecisionId::try_new("decision-1").unwrap(),
            proposal.proposal_id().clone(),
            IdentityReviewAction::SetIdentity,
            Some(PlayerId::try_new(8_480_001).unwrap()),
            "reviewer@example.com",
            at(21),
            "Official birth date and club roster agree.",
            Vec::new(),
        )
        .unwrap();
        assert_eq!(decision.decision_id().as_str(), "decision-1");
        assert_eq!(proposal.proposal_id().as_str(), "proposal-1");
    }

    #[test]
    fn package_rejects_two_decisions_for_one_proposal() {
        let proposal = ProviderIdentityProposal::new(
            ProposalId::try_new("proposal-1").unwrap(),
            ProviderPersonLocator::SourceRow {
                source_id: SourceId::try_new("camp").unwrap(),
                row_key: "row-12".to_owned(),
            },
            "Camp Player",
            None,
            None,
            vec![evidence("camp", at(20), 'b')],
        )
        .unwrap();
        let decision = |id: &str, player_id| {
            IdentityReviewDecision::new(
                DecisionId::try_new(id).unwrap(),
                ProposalId::try_new("proposal-1").unwrap(),
                IdentityReviewAction::SetIdentity,
                Some(PlayerId::try_new(player_id).unwrap()),
                "reviewer@example.com",
                at(21),
                "Reviewed identity evidence.",
                vec![evidence(id, at(21), 'c')],
            )
            .unwrap()
        };
        let mut package = package_with_proposals(Vec::new(), Vec::new(), vec![proposal]);
        package.identity_review_decisions = vec![
            decision("decision-1", 8_480_001),
            decision("decision-2", 8_480_002),
        ];

        assert!(matches!(
            package.validate(),
            Err(SourceContractError::DuplicateId {
                kind: "decision_proposal",
                ..
            })
        ));
    }

    #[test]
    fn sealed_package_preserves_the_fact_waiting_on_identity_review() {
        let proposal = ProviderIdentityProposal::new(
            ProposalId::try_new("proposal-1").unwrap(),
            ProviderPersonLocator::SourceRow {
                source_id: SourceId::try_new("camp").unwrap(),
                row_key: "row-12".to_owned(),
            },
            "Camp Player",
            None,
            None,
            vec![evidence("camp", at(20), 'b')],
        )
        .unwrap();
        let package = package_with_proposals(Vec::new(), Vec::new(), vec![proposal])
            .with_staged_player_assertions(vec![camp_staged_assertion("proposal-1")])
            .unwrap();
        let decoded: SourcePackage = serde_json::from_slice(&serde_json::to_vec(&package).unwrap())
            .expect("staged package should round-trip");

        decoded.validate().unwrap();
        assert_eq!(decoded.staged_player_assertions.len(), 1);
        assert!(matches!(
            decoded.staged_player_assertions[0].fact(),
            SourceFact::PlayerParticipation(_)
        ));
    }

    #[test]
    fn staged_assertion_cannot_reference_a_missing_identity_proposal() {
        let error = package_with(Vec::new(), Vec::new())
            .with_staged_player_assertions(vec![camp_staged_assertion("missing-proposal")])
            .unwrap_err();
        assert!(matches!(
            error,
            SourceContractError::UnknownStagedProposal { .. }
        ));
    }

    #[test]
    fn run_manifest_distinguishes_authoritative_empty_from_failed_acquisition() {
        run_manifest(SourceObjectState::Acquired { records: 0 }, true)
            .validate()
            .unwrap();
        run_manifest(
            SourceObjectState::Failed {
                reason: "network unavailable".to_owned(),
            },
            false,
        )
        .validate()
        .unwrap();
        assert_eq!(
            run_manifest(SourceObjectState::IncompletePagination, true)
                .validate()
                .unwrap_err(),
            SourceContractError::InvalidRunCompleteness
        );
    }

    #[test]
    fn package_fingerprint_is_order_invariant_and_round_trips() {
        let first_input = SourceInputRecord {
            evidence: evidence("b-source", at(20), 'b'),
            freshness: SourceFreshness {
                class: FreshnessClass::Static,
                captured_at: at(20),
                evaluated_at: at(31),
                status: FreshnessStatus::Static,
                policy_version: PolicyVersion::try_new("freshness.v1").unwrap(),
            },
        };
        let second_input = SourceInputRecord {
            evidence: evidence("a-source", at(20), 'a'),
            freshness: SourceFreshness {
                class: FreshnessClass::Static,
                captured_at: at(20),
                evaluated_at: at(31),
                status: FreshnessStatus::Static,
                policy_version: PolicyVersion::try_new("freshness.v1").unwrap(),
            },
        };
        let first_fact = drafted_fact("fact-b", at(1));
        let second_fact = drafted_fact("fact-a", at(2));
        let first = package_with(
            vec![first_input.clone(), second_input.clone()],
            vec![first_fact.clone(), second_fact.clone()],
        );
        let second = package_with(
            vec![second_input, first_input],
            vec![second_fact, first_fact],
        );
        assert_eq!(first.fingerprint, second.fingerprint);
        let decoded: SourcePackage =
            serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
        decoded.validate().unwrap();
    }

    #[test]
    fn package_enforces_effective_and_knowledge_cutoffs_independently() {
        let late_fact = drafted_fact("late-fact", at(31) + chrono::Duration::hours(1));
        let result = SourcePackage::seal(
            PackageId::try_new("late-fact-package").unwrap(),
            Season(20_262_027),
            at(31),
            at(31),
            AdapterVersion::try_new("registry.v1").unwrap(),
            PolicyVersion::try_new("reconcile.v1").unwrap(),
            hash('f'),
            run_manifest(SourceObjectState::Acquired { records: 1 }, true),
            Vec::new(),
            vec![late_fact],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert!(matches!(
            result,
            Err(SourceContractError::FactAfterEffectiveCutoff { .. })
        ));

        let late_evidence = evidence("future-capture", at(31) + chrono::Duration::hours(1), 'c');
        let fact = FactAssertion::new(
            FactId::try_new("known-fact").unwrap(),
            "player:1:release:SEA",
            FactSubject::Player(PlayerId::try_new(1).unwrap()),
            EffectiveTime::new(at(1), None, EffectivePrecision::Day).unwrap(),
            FactAuthority::LegalControl,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Released {
                by: OrganizationId::try_new("SEA").unwrap(),
            }),
            vec![late_evidence],
        )
        .unwrap();
        let result = SourcePackage::seal(
            PackageId::try_new("late-knowledge-package").unwrap(),
            Season(20_262_027),
            at(31),
            at(31),
            AdapterVersion::try_new("registry.v1").unwrap(),
            PolicyVersion::try_new("reconcile.v1").unwrap(),
            hash('f'),
            run_manifest(SourceObjectState::Acquired { records: 1 }, true),
            Vec::new(),
            vec![fact],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert!(matches!(
            result,
            Err(SourceContractError::EvidenceAfterKnowledgeCutoff { .. })
        ));
    }

    #[test]
    fn source_package_json_schema_is_embedded_and_parseable() {
        let schema: serde_json::Value = serde_json::from_str(SOURCE_PACKAGE_JSON_SCHEMA).unwrap();
        assert_eq!(
            schema["properties"]["schema"]["const"],
            SOURCE_PACKAGE_SCHEMA
        );
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "exclusions"));
    }
}
