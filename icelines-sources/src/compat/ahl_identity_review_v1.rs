//! Compatibility adapter for finalized `ahl_identity_review_decisions.v1`.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use crate::ahl::roster_stats::AhlRosterStatsOutput;
use chrono::{DateTime, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, DecisionId, FreshnessClass, IdentityReviewAction,
    IdentityReviewDecision, ProposalId, ProviderId, SourceEvidence, SourceId, SourceUrl,
};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const AHL_IDENTITY_REVIEW_DECISIONS_V1: &str = "ahl_identity_review_decisions.v1";

#[derive(Debug, Clone)]
pub struct AhlIdentityReviewV1Adapter {
    season: u32,
    ahl_team: String,
    captured_at: DateTime<Utc>,
    source_url: SourceUrl,
    proposals: BTreeMap<String, ProposalId>,
}

impl AhlIdentityReviewV1Adapter {
    pub fn new(
        season: u32,
        ahl_team: &str,
        captured_at: DateTime<Utc>,
        source_url: &str,
        roster: &AhlRosterStatsOutput,
    ) -> Result<Self, String> {
        let ahl_team = ahl_team.trim().to_ascii_uppercase();
        if !(2..=4).contains(&ahl_team.len())
            || !ahl_team.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err("AHL team must be a 2-4 letter code".to_owned());
        }
        let expected_club = format!("AHL:{ahl_team}");
        let mut proposals = BTreeMap::new();
        for observation in &roster.roster_observations {
            if observation.season.0 != season || observation.ahl_club.as_str() != expected_club {
                continue;
            }
            if proposals
                .insert(
                    observation.provider_player_id.clone(),
                    observation.proposal_id.clone(),
                )
                .is_some()
            {
                return Err(format!(
                    "duplicate staged AHL provider identity {}",
                    observation.provider_player_id
                ));
            }
        }
        if proposals.is_empty() {
            return Err(format!(
                "no staged roster identities found for {ahl_team} in {season}"
            ));
        }
        Ok(Self {
            season,
            ahl_team,
            captured_at,
            source_url: SourceUrl::try_new(source_url).map_err(|error| error.to_string())?,
            proposals,
        })
    }
}

impl SourceAdapter for AhlIdentityReviewV1Adapter {
    type Output = Vec<IdentityReviewDecision>;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!(
                "ahl-identity-review:{}:{}",
                self.season, self.ahl_team
            ))
            .expect("validated review fields produce a source id"),
            provider: ProviderId::try_new("icelines_review_registry")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("compat.ahl_identity_review_decisions")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "reviewed_ahl_identity_decisions",
            supported_layouts: &[AHL_IDENTITY_REVIEW_DECISIONS_V1],
            required_identity_keys: &["provider_player_id", "nhl_player_id"],
            additive_field_policy: AdditiveFieldPolicy::IgnoreReviewed,
            freshness_class: FreshnessClass::Static,
            historical_availability: HistoricalAvailability::CallerSuppliedArchive,
            absence_semantics: AbsenceSemantics::NotEvidence,
            output_fact_families: &["identity_review_decision"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let review: ReviewEnvelope =
            serde_json::from_slice(input.bytes()).map_err(|error| AdapterError {
                source_id: input.source_id().clone(),
                adapter_id: descriptor.adapter_id.clone(),
                input_hash: input.content_hash().clone(),
                category: AdapterErrorCategory::UnsupportedLayout,
                disposition: AdapterDisposition::FatalSource,
                message: format!("invalid AHL identity review: {error}"),
            })?;
        if review.schema != AHL_IDENTITY_REVIEW_DECISIONS_V1
            || review.season != self.season
            || review.ahl_team.trim().to_ascii_uppercase() != self.ahl_team
            || review.draft
        {
            return Err(error(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                "AHL review must be finalized and match the staged team-season".to_owned(),
            ));
        }
        let reviewer = review
            .reviewer
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    "finalized AHL review requires reviewer".to_owned(),
                )
            })?;
        let reviewed_at = review
            .reviewed_at
            .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
            .map(|value| value.with_timezone(&Utc))
            .ok_or_else(|| {
                error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    "finalized AHL review requires RFC 3339 reviewed_at".to_owned(),
                )
            })?;
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            self.source_url.clone(),
            descriptor.provider.clone(),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let mut decisions = Vec::with_capacity(review.decisions.len());
        for row in review.decisions {
            let proposal_id = self.proposals.get(&row.provider_player_id).ok_or_else(|| {
                error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    format!(
                        "AHL review references provider player {} outside the staged roster",
                        row.provider_player_id
                    ),
                )
            })?;
            let (action, player_id) = match row.action {
                LegacyReviewAction::Reject => {
                    if row.nhl_player_id.is_some() {
                        return Err(error(
                            &input,
                            &descriptor,
                            AdapterErrorCategory::SemanticValidation,
                            "rejected AHL identity must not retain an NHL player id".to_owned(),
                        ));
                    }
                    (IdentityReviewAction::Reject, None)
                }
                LegacyReviewAction::AcceptProposal | LegacyReviewAction::SetIdentity => {
                    let player_id = row
                        .nhl_player_id
                        .and_then(|value| PlayerId::try_new(value).ok())
                        .ok_or_else(|| {
                            error(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::SemanticValidation,
                                "accepted AHL identity requires a non-zero NHL player id"
                                    .to_owned(),
                            )
                        })?;
                    // The staged roster proposal has no canonical ID. Legacy
                    // accept-proposal therefore lowers explicitly to set-identity.
                    (IdentityReviewAction::SetIdentity, Some(player_id))
                }
            };
            decisions.push(
                IdentityReviewDecision::new(
                    DecisionId::try_new(format!(
                        "ahl-review:{}:{}:{}",
                        self.season, self.ahl_team, row.provider_player_id
                    ))
                    .expect("validated AHL fields produce a decision id"),
                    proposal_id.clone(),
                    action,
                    player_id,
                    reviewer.clone(),
                    reviewed_at,
                    row.note,
                    vec![evidence.clone()],
                )
                .map_err(|failure| {
                    error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        failure.to_string(),
                    )
                })?,
            );
        }
        Ok(decisions)
    }
}

#[derive(Debug, Deserialize)]
struct ReviewEnvelope {
    schema: String,
    season: u32,
    ahl_team: String,
    draft: bool,
    #[serde(default)]
    reviewer: Option<String>,
    #[serde(default)]
    reviewed_at: Option<String>,
    decisions: Vec<ReviewRow>,
}

#[derive(Debug, Deserialize)]
struct ReviewRow {
    provider_player_id: String,
    action: LegacyReviewAction,
    #[serde(default)]
    nhl_player_id: Option<u32>,
    note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyReviewAction {
    AcceptProposal,
    SetIdentity,
    Reject,
}

fn error(
    input: &SourceInput<'_>,
    descriptor: &SourceDescriptor,
    category: AdapterErrorCategory,
    message: String,
) -> AdapterError {
    AdapterError {
        source_id: input.source_id().clone(),
        adapter_id: descriptor.adapter_id.clone(),
        input_hash: input.content_hash().clone(),
        category,
        disposition: AdapterDisposition::FatalSource,
        message,
    }
}
