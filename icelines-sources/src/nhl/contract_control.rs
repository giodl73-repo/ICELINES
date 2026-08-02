//! Canonical current contract-control ledger adapter.
//!
//! Providers may be licensed, official-publication aggregators, or reviewed
//! local imports. The common contract requires canonical player IDs, explicit
//! organization coverage, terminal enumeration, and row-level evidence.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, ContractKind, EffectivePrecision, EffectiveTime, FactAssertion,
    FactAuthority, FactId, FactSubject, FreshnessClass, OrganizationId, PlayerOrganizationEvent,
    ProviderId, SourceEvidence, SourceFact, SourceId, SourceUrl,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const CONTRACT_CONTROL_LEDGER_V1: &str = "contract_control_ledger.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct ContractControlLedgerOutput {
    pub season: Season,
    pub captured_at: DateTime<Utc>,
    pub coverage_source_url: SourceUrl,
    pub provider: ProviderId,
    pub records_by_organization: BTreeMap<OrganizationId, usize>,
    pub facts: Vec<FactAssertion<SourceFact>>,
}

#[derive(Debug, Clone, Default)]
pub struct ContractControlLedgerV1Adapter;

impl SourceAdapter for ContractControlLedgerV1Adapter {
    type Output = ContractControlLedgerOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new("contract-control-ledger")
                .expect("static source id is valid"),
            provider: ProviderId::try_new("declared_contract_control_provider")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.contract_control_ledger")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "canonical_current_contract_control",
            supported_layouts: &[CONTRACT_CONTROL_LEDGER_V1],
            required_identity_keys: &["player_id", "organization"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Transactional,
            historical_availability: HistoricalAvailability::CallerSuppliedArchive,
            absence_semantics: AbsenceSemantics::AuthoritativeEmpty,
            output_fact_families: &["player_organization.contract_signed"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let ledger: ContractControlLedgerWire =
            serde_json::from_slice(input.bytes()).map_err(|error| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::UnsupportedLayout,
                    format!("invalid contract-control ledger: {error}"),
                )
            })?;
        if ledger.schema != CONTRACT_CONTROL_LEDGER_V1 || ledger.season == 0 {
            return Err(adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("expected non-zero season and schema {CONTRACT_CONTROL_LEDGER_V1}"),
            ));
        }
        let captured_at = parse_time(&ledger.captured_at).map_err(|message| {
            adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::MalformedRecord,
                message,
            )
        })?;
        let provider = ProviderId::try_new(ledger.provider).map_err(|error| {
            adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::MalformedRecord,
                error.to_string(),
            )
        })?;
        let coverage_source_url = SourceUrl::try_new(ledger.source_url).map_err(|error| {
            adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::MalformedRecord,
                error.to_string(),
            )
        })?;
        let mut records_by_organization = BTreeMap::new();
        for row in ledger.coverage {
            let organization = OrganizationId::try_new(row.organization).map_err(|error| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    error.to_string(),
                )
            })?;
            if !row.terminal
                || records_by_organization
                    .insert(organization, row.records)
                    .is_some()
            {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    "contract coverage rows must be unique and terminal".to_owned(),
                ));
            }
        }
        if records_by_organization.is_empty() {
            return Err(adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                "contract-control ledger requires organization coverage".to_owned(),
            ));
        }
        let mut player_ids = BTreeSet::new();
        let mut actual_counts = BTreeMap::<OrganizationId, usize>::new();
        let mut facts = Vec::with_capacity(ledger.contracts.len());
        for row in ledger.contracts {
            let player_id = PlayerId::try_new(row.player_id).map_err(|error| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    error.to_string(),
                )
            })?;
            let organization = OrganizationId::try_new(row.organization).map_err(|error| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    error.to_string(),
                )
            })?;
            if !player_ids.insert(player_id) || !records_by_organization.contains_key(&organization)
            {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    "contract rows must have unique players and covered organizations".to_owned(),
                ));
            }
            let effective_at = parse_time(&row.effective_at).map_err(|message| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    message,
                )
            })?;
            if effective_at > captured_at {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    format!("contract for player {} starts after capture", player_id.0),
                ));
            }
            let source_url = SourceUrl::try_new(row.source_url).map_err(|error| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    error.to_string(),
                )
            })?;
            let evidence = SourceEvidence::new(
                input.source_id().clone(),
                source_url,
                provider.clone(),
                captured_at,
                input.content_hash().clone(),
                descriptor.adapter_version.clone(),
            );
            facts.push(
                FactAssertion::new(
                    FactId::try_new(format!(
                        "contract-control:{}:{}:{}",
                        ledger.season,
                        organization.as_str(),
                        player_id.0
                    ))
                    .expect("validated contract fields produce a fact id"),
                    format!("player:{}:current_contract", player_id.0),
                    FactSubject::Player(player_id),
                    EffectiveTime::new(effective_at, None, EffectivePrecision::Day)
                        .expect("single-ended effective time is valid"),
                    FactAuthority::Contract,
                    SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
                        with: organization.clone(),
                        contract_kind: row.contract_kind,
                    }),
                    vec![evidence],
                )
                .map_err(|error| {
                    adapter_error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
            *actual_counts.entry(organization).or_default() += 1;
        }
        if records_by_organization
            .iter()
            .any(|(organization, expected)| {
                actual_counts.get(organization).copied().unwrap_or_default() != *expected
            })
        {
            return Err(adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                "contract row counts do not reconcile with terminal organization coverage"
                    .to_owned(),
            ));
        }
        Ok(ContractControlLedgerOutput {
            season: Season(ledger.season),
            captured_at,
            coverage_source_url,
            provider,
            records_by_organization,
            facts,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractControlLedgerWire {
    schema: String,
    season: u32,
    provider: String,
    captured_at: String,
    source_url: String,
    coverage: Vec<ContractCoverageWire>,
    contracts: Vec<ContractRowWire>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractCoverageWire {
    organization: String,
    terminal: bool,
    records: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractRowWire {
    player_id: u32,
    organization: String,
    contract_kind: ContractKind,
    effective_at: String,
    source_url: String,
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| "contract-control timestamps must be RFC 3339".to_owned())
}

fn adapter_error(
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
    fn terminal_counts_and_contract_facts_reconcile() {
        let bytes = br#"{
          "schema":"contract_control_ledger.v1",
          "season":20262027,
          "provider":"fixture_contract_registry",
          "captured_at":"2026-07-31T12:00:00Z",
          "source_url":"https://example.test/contracts",
          "coverage":[
            {"organization":"NYR","terminal":true,"records":1},
            {"organization":"SEA","terminal":true,"records":0}
          ],
          "contracts":[{
            "player_id":8480001,
            "organization":"NYR",
            "contract_kind":"entry_level",
            "effective_at":"2026-07-01T00:00:00Z",
            "source_url":"https://example.test/contracts/8480001"
          }]
        }"#;
        let adapter = ContractControlLedgerV1Adapter;
        let output = adapter
            .parse(SourceInput::new(
                bytes,
                adapter.descriptor().source_id,
                ContentHash::try_new("a".repeat(64)).unwrap(),
            ))
            .unwrap();

        assert_eq!(output.facts.len(), 1);
        assert_eq!(output.records_by_organization.len(), 2);
        assert!(matches!(
            output.facts[0].fact(),
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
                contract_kind: ContractKind::EntryLevel,
                ..
            })
        ));
    }

    #[test]
    fn nonterminal_or_mismatched_coverage_fails_closed() {
        let bytes = br#"{
          "schema":"contract_control_ledger.v1",
          "season":20262027,
          "provider":"fixture_contract_registry",
          "captured_at":"2026-07-31T12:00:00Z",
          "source_url":"https://example.test/contracts",
          "coverage":[{"organization":"NYR","terminal":true,"records":2}],
          "contracts":[{
            "player_id":8480001,"organization":"NYR","contract_kind":"unknown",
            "effective_at":"2026-07-01T00:00:00Z",
            "source_url":"https://example.test/contracts/8480001"
          }]
        }"#;
        let adapter = ContractControlLedgerV1Adapter;
        assert!(adapter
            .parse(SourceInput::new(
                bytes,
                adapter.descriptor().source_id,
                ContentHash::try_new("b".repeat(64)).unwrap(),
            ))
            .is_err());
    }
}
