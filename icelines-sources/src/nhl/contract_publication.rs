//! Official NHL signing-article adapter.
//!
//! The common article layout supplies a displayed identity rather than a
//! canonical player ID. Contract observations therefore remain staged until
//! the identity review workflow accepts or sets that identity.

use super::club_publication::extract_json_string_property;
use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, ContractKind, EffectivePrecision, EffectiveTime, FactAuthority,
    FreshnessClass, OrganizationId, PlayerOrganizationEvent, ProposalId, ProviderId,
    ProviderIdentityProposal, ProviderPersonLocator, SourceEvidence, SourceFact, SourceId,
    SourceUrl, StagedAssertionId, StagedPlayerAssertion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedContractSigning {
    pub proposal_id: ProposalId,
    pub organization: OrganizationId,
    pub occurred_at: EffectiveTime,
    pub contract_kind: ContractKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractPublicationOutput {
    pub identity_proposal: ProviderIdentityProposal,
    pub signing: StagedContractSigning,
    pub staged_assertion: StagedPlayerAssertion,
}

#[derive(Debug, Clone)]
pub struct NhlArticleContractSigningAdapter {
    organization: OrganizationId,
    captured_at: DateTime<Utc>,
    source_url: SourceUrl,
}

impl NhlArticleContractSigningAdapter {
    pub fn new(
        organization: &str,
        captured_at: DateTime<Utc>,
        source_url: &str,
    ) -> Result<Self, String> {
        let organization = organization.trim().to_ascii_uppercase();
        if !(2..=4).contains(&organization.len())
            || !organization.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err("organization must be a 2-4 letter NHL abbreviation".to_owned());
        }
        Ok(Self {
            organization: OrganizationId::try_new(organization)
                .map_err(|error| error.to_string())?,
            captured_at,
            source_url: SourceUrl::try_new(source_url).map_err(|error| error.to_string())?,
        })
    }

    fn error(
        &self,
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
}

impl SourceAdapter for NhlArticleContractSigningAdapter {
    type Output = ContractPublicationOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!(
                "nhl-contract-publication:{}",
                self.organization.as_str()
            ))
            .expect("validated organization produces a source id"),
            provider: ProviderId::try_new("official_nhl_publication")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.article.contract_signing")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_contract_publication",
            supported_layouts: &["nhl_article_jsonld.signs_headline.v1"],
            required_identity_keys: &["headline", "datePublished", "displayed_name"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Transactional,
            historical_availability: HistoricalAvailability::ProviderArchive,
            absence_semantics: AbsenceSemantics::NotEvidence,
            output_fact_families: &["identity_proposal", "staged_contract_signing"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let document = std::str::from_utf8(input.bytes()).map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("contract publication is not UTF-8: {error}"),
            )
        })?;
        let headline = extract_json_string_property(document, "headline").map_err(|message| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                message,
            )
        })?;
        let displayed_name = headline
            .split_once(" signs ")
            .map(|(name, _)| name.trim())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::UnsupportedLayout,
                    "headline does not match the reviewed '<player> signs ...' layout".to_owned(),
                )
            })?;
        let date = extract_json_string_property(document, "datePublished")
            .ok()
            .and_then(|value| value.get(..10).map(str::to_owned))
            .and_then(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d").ok())
            .ok_or_else(|| {
                self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    "datePublished must begin with YYYY-MM-DD".to_owned(),
                )
            })?;
        let occurred_at = EffectiveTime::new(
            Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight is a valid time")),
            None,
            EffectivePrecision::Day,
        )
        .expect("single-ended effective time is valid");
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            self.source_url.clone(),
            descriptor.provider.clone(),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let proposal_id = ProposalId::try_new(format!(
            "contract-signing:{}:{}:{}",
            self.organization.as_str(),
            date.format("%Y-%m-%d"),
            normalized_key(displayed_name)
        ))
        .expect("validated signing fields produce a proposal id");
        let identity_proposal = ProviderIdentityProposal::new(
            proposal_id.clone(),
            ProviderPersonLocator::SourceRow {
                source_id: input.source_id().clone(),
                row_key: "contract-signing:1".to_owned(),
            },
            displayed_name,
            None,
            None,
            vec![evidence.clone()],
        )
        .map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                error.to_string(),
            )
        })?;
        let signing = StagedContractSigning {
            proposal_id: proposal_id.clone(),
            organization: self.organization.clone(),
            occurred_at: occurred_at.clone(),
            contract_kind: ContractKind::Unknown,
        };
        let staged_assertion = StagedPlayerAssertion::new(
            StagedAssertionId::try_new(format!("staged:{proposal_id}:contract-signing"))
                .expect("validated proposal id produces a staged assertion id"),
            format!("proposal:{proposal_id}:contract-signing"),
            proposal_id,
            occurred_at,
            FactAuthority::Contract,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
                with: self.organization.clone(),
                contract_kind: ContractKind::Unknown,
            }),
            vec![evidence],
        )
        .map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                error.to_string(),
            )
        })?;
        Ok(ContractPublicationOutput {
            identity_proposal,
            signing,
            staged_assertion,
        })
    }
}

fn normalized_key(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
