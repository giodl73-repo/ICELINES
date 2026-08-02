//! Provider-neutral finalized identity-review ledger.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, ContentHash, DecisionId, FreshnessClass, IdentityReviewAction,
    IdentityReviewDecision, ProposalId, ProviderId, SourceEvidence, SourceId, SourceUrl,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const IDENTITY_REVIEW_LEDGER_V1: &str = "identity_review_ledger.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct IdentityReviewLedgerOutput {
    pub season: Season,
    pub reviewed_at: DateTime<Utc>,
    pub registry_url: SourceUrl,
    pub provider: ProviderId,
    pub decisions: Vec<IdentityReviewDecision>,
}

#[derive(Debug, Clone, Default)]
pub struct IdentityReviewLedgerV1Adapter;

impl SourceAdapter for IdentityReviewLedgerV1Adapter {
    type Output = IdentityReviewLedgerOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new("identity-review-ledger")
                .expect("static source id is valid"),
            provider: ProviderId::try_new("declared_identity_review_registry")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("identity.review_ledger")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "finalized_identity_review",
            supported_layouts: &[IDENTITY_REVIEW_LEDGER_V1],
            required_identity_keys: &["proposal_id", "action"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Static,
            historical_availability: HistoricalAvailability::CallerSuppliedArchive,
            absence_semantics: AbsenceSemantics::NotEvidence,
            output_fact_families: &["identity_review_decision"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let ledger: IdentityReviewLedgerDocument =
            serde_json::from_slice(input.bytes()).map_err(|error| {
                fail(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::UnsupportedLayout,
                    format!("invalid identity-review ledger: {error}"),
                )
            })?;
        if ledger.schema != IDENTITY_REVIEW_LEDGER_V1 || ledger.season == 0 {
            return Err(fail(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("expected non-zero season and schema {IDENTITY_REVIEW_LEDGER_V1}"),
            ));
        }
        let reviewed_at = DateTime::parse_from_rfc3339(&ledger.reviewed_at)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| {
                fail(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    "reviewed_at must be RFC 3339".to_owned(),
                )
            })?;
        let provider = ProviderId::try_new(ledger.provider).map_err(|error| {
            fail(
                &input,
                &descriptor,
                AdapterErrorCategory::MalformedRecord,
                error.to_string(),
            )
        })?;
        let registry_url = SourceUrl::try_new(ledger.registry_url).map_err(|error| {
            fail(
                &input,
                &descriptor,
                AdapterErrorCategory::MalformedRecord,
                error.to_string(),
            )
        })?;
        if ledger.reviewer.trim().is_empty() || ledger.decisions.is_empty() {
            return Err(fail(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                "reviewer and decisions must not be empty".to_owned(),
            ));
        }
        let mut decision_ids = BTreeSet::new();
        let mut proposal_ids = BTreeSet::new();
        let mut decisions = Vec::with_capacity(ledger.decisions.len());
        for row in ledger.decisions {
            let decision_id = DecisionId::try_new(row.decision_id).map_err(|error| {
                fail(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    error.to_string(),
                )
            })?;
            let proposal_id = ProposalId::try_new(row.proposal_id).map_err(|error| {
                fail(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    error.to_string(),
                )
            })?;
            if !decision_ids.insert(decision_id.clone())
                || !proposal_ids.insert(proposal_id.clone())
            {
                return Err(fail(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    "decision IDs and proposal IDs must be unique".to_owned(),
                ));
            }
            let evidence = row
                .evidence
                .into_iter()
                .map(|item| {
                    let captured_at = DateTime::parse_from_rfc3339(&item.captured_at)
                        .map(|value| value.with_timezone(&Utc))
                        .map_err(|_| {
                            fail(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                "evidence captured_at must be RFC 3339".to_owned(),
                            )
                        })?;
                    if captured_at > reviewed_at {
                        return Err(fail(
                            &input,
                            &descriptor,
                            AdapterErrorCategory::SemanticValidation,
                            "identity evidence cannot be captured after review".to_owned(),
                        ));
                    }
                    Ok(SourceEvidence::new(
                        SourceId::try_new(item.source_id).map_err(|error| {
                            fail(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                error.to_string(),
                            )
                        })?,
                        SourceUrl::try_new(item.source_url).map_err(|error| {
                            fail(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                error.to_string(),
                            )
                        })?,
                        ProviderId::try_new(item.provider).map_err(|error| {
                            fail(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                error.to_string(),
                            )
                        })?,
                        captured_at,
                        ContentHash::try_new(item.content_sha256).map_err(|error| {
                            fail(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                error.to_string(),
                            )
                        })?,
                        AdapterVersion::try_new(item.adapter_version).map_err(|error| {
                            fail(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                error.to_string(),
                            )
                        })?,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if evidence.is_empty() {
                return Err(fail(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    "every identity decision requires evidence".to_owned(),
                ));
            }
            decisions.push(
                IdentityReviewDecision::new(
                    decision_id,
                    proposal_id,
                    row.action,
                    row.player_id
                        .map(PlayerId::try_new)
                        .transpose()
                        .map_err(|error| {
                            fail(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                error.to_string(),
                            )
                        })?,
                    &ledger.reviewer,
                    reviewed_at,
                    row.rationale,
                    evidence,
                )
                .map_err(|error| {
                    fail(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
        }
        Ok(IdentityReviewLedgerOutput {
            season: Season(ledger.season),
            reviewed_at,
            registry_url,
            provider,
            decisions,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityReviewLedgerDocument {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub registry_url: String,
    pub reviewer: String,
    pub reviewed_at: String,
    pub decisions: Vec<IdentityReviewLedgerRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityReviewLedgerRow {
    pub decision_id: String,
    pub proposal_id: String,
    pub action: IdentityReviewAction,
    pub player_id: Option<u32>,
    pub rationale: String,
    pub evidence: Vec<IdentityReviewEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityReviewEvidence {
    pub source_id: String,
    pub source_url: String,
    pub provider: String,
    pub captured_at: String,
    pub content_sha256: String,
    pub adapter_version: String,
}

fn fail(
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

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::source_facts::ContentHash;

    #[test]
    fn parses_finalized_set_and_reject_decisions() {
        let bytes = br#"{"schema":"identity_review_ledger.v1","season":20262027,"provider":"fixture_registry","registry_url":"https://example.test/reviews","reviewer":"fixture-reviewer","reviewed_at":"2026-07-31T12:00:00Z","decisions":[{"decision_id":"draft-2026-5","proposal_id":"nhl-draft:2026:5","action":"set_identity","player_id":8480001,"rationale":"Official landing and draft coordinates agree.","evidence":[{"source_id":"landing-8480001","source_url":"https://api-web.nhle.com/v1/player/8480001/landing","provider":"official_nhl","captured_at":"2026-07-31T11:00:00Z","content_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","adapter_version":"v1"}]},{"decision_id":"reject-row","proposal_id":"camp-row","action":"reject","player_id":null,"rationale":"Publication row is staff, not a player.","evidence":[{"source_id":"camp-row","source_url":"https://example.test/camp","provider":"official_club","captured_at":"2026-07-31T11:00:00Z","content_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","adapter_version":"v1"}]}]}"#;
        let adapter = IdentityReviewLedgerV1Adapter;
        let output = adapter
            .parse(SourceInput::new(
                bytes,
                adapter.descriptor().source_id,
                ContentHash::try_new("e".repeat(64)).unwrap(),
            ))
            .unwrap();
        assert_eq!(output.decisions.len(), 2);
        assert_eq!(
            output.decisions[0].canonical_player_id(),
            Some(PlayerId(8_480_001))
        );
        assert_eq!(output.decisions[1].action(), IdentityReviewAction::Reject);
    }

    #[test]
    fn duplicate_proposal_decisions_fail_closed() {
        let bytes = br#"{"schema":"identity_review_ledger.v1","season":20262027,"provider":"fixture_registry","registry_url":"https://example.test/reviews","reviewer":"fixture-reviewer","reviewed_at":"2026-07-31T12:00:00Z","decisions":[{"decision_id":"one","proposal_id":"same","action":"reject","player_id":null,"rationale":"First review.","evidence":[{"source_id":"one","source_url":"https://example.test/one","provider":"fixture","captured_at":"2026-07-31T11:00:00Z","content_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","adapter_version":"v1"}]},{"decision_id":"two","proposal_id":"same","action":"reject","player_id":null,"rationale":"Second review.","evidence":[{"source_id":"two","source_url":"https://example.test/two","provider":"fixture","captured_at":"2026-07-31T11:00:00Z","content_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","adapter_version":"v1"}]}]}"#;
        let adapter = IdentityReviewLedgerV1Adapter;
        assert!(adapter
            .parse(SourceInput::new(
                bytes,
                adapter.descriptor().source_id,
                ContentHash::try_new("f".repeat(64)).unwrap()
            ))
            .is_err());
    }
}
