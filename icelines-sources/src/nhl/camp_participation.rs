//! Canonical camp-participation ledger adapter.
//!
//! This boundary admits provider- or review-resolved NHL player identities.
//! Participation is attendance evidence only and never establishes contract
//! rights, roster assignment, or organization control.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, EffectivePrecision, EffectiveTime, FactAssertion, FactAuthority,
    FactId, FactSubject, FreshnessClass, OrganizationId, ParticipationAuthority, ParticipationKind,
    PlayerParticipationFact, ProviderId, SourceEvidence, SourceFact, SourceId, SourceUrl,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const CAMP_PARTICIPATION_LEDGER_V1: &str = "camp_participation_ledger.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct CampParticipationLedgerOutput {
    pub season: Season,
    pub captured_at: DateTime<Utc>,
    pub coverage_source_url: SourceUrl,
    pub provider: ProviderId,
    pub records_by_organization: BTreeMap<OrganizationId, usize>,
    pub facts: Vec<FactAssertion<SourceFact>>,
}

#[derive(Debug, Clone, Default)]
pub struct CampParticipationLedgerV1Adapter;

impl SourceAdapter for CampParticipationLedgerV1Adapter {
    type Output = CampParticipationLedgerOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new("camp-participation-ledger")
                .expect("static source id is valid"),
            provider: ProviderId::try_new("declared_camp_participation_provider")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.camp_participation_ledger")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "canonical_camp_participation",
            supported_layouts: &[CAMP_PARTICIPATION_LEDGER_V1],
            required_identity_keys: &["player_id", "organization"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Roster,
            historical_availability: HistoricalAvailability::CallerSuppliedArchive,
            absence_semantics: AbsenceSemantics::AuthoritativeEmpty,
            output_fact_families: &["player_participation"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let ledger: CampParticipationLedgerWire =
            serde_json::from_slice(input.bytes()).map_err(|error| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::UnsupportedLayout,
                    format!("invalid camp-participation ledger: {error}"),
                )
            })?;
        if ledger.schema != CAMP_PARTICIPATION_LEDGER_V1 || ledger.season == 0 {
            return Err(adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("expected non-zero season and schema {CAMP_PARTICIPATION_LEDGER_V1}"),
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
                    "camp coverage rows must be unique and terminal".to_owned(),
                ));
            }
        }
        if records_by_organization.is_empty() {
            return Err(adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                "camp-participation ledger requires organization coverage".to_owned(),
            ));
        }
        let mut row_keys = BTreeSet::new();
        let mut actual_counts = BTreeMap::<OrganizationId, usize>::new();
        let mut facts = Vec::with_capacity(ledger.participants.len());
        for row in ledger.participants {
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
            let kind_key = participation_kind_key(row.kind);
            if !row_keys.insert((player_id, organization.clone(), kind_key))
                || !records_by_organization.contains_key(&organization)
            {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    "camp rows must have unique player/team/kind keys and covered organizations"
                        .to_owned(),
                ));
            }
            let occurred_at = parse_time(&row.occurred_at).map_err(|message| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    message,
                )
            })?;
            if occurred_at > captured_at {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    format!(
                        "camp participation for player {} is after capture",
                        player_id.0
                    ),
                ));
            }
            let evidence = SourceEvidence::new(
                input.source_id().clone(),
                SourceUrl::try_new(row.source_url).map_err(|error| {
                    adapter_error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::MalformedRecord,
                        error.to_string(),
                    )
                })?,
                provider.clone(),
                captured_at,
                input.content_hash().clone(),
                descriptor.adapter_version.clone(),
            );
            facts.push(
                FactAssertion::new(
                    FactId::try_new(format!(
                        "camp-participation:{}:{}:{}:{}",
                        ledger.season,
                        organization.as_str(),
                        player_id.0,
                        kind_key
                    ))
                    .expect("validated camp fields produce a fact id"),
                    format!(
                        "player:{}:participation:{}:{}:{}",
                        player_id.0,
                        organization.as_str(),
                        ledger.season,
                        kind_key
                    ),
                    FactSubject::Player(player_id),
                    EffectiveTime::new(occurred_at, None, EffectivePrecision::Day)
                        .expect("single-ended effective time is valid"),
                    FactAuthority::Attendance,
                    SourceFact::PlayerParticipation(PlayerParticipationFact {
                        organization: organization.clone(),
                        season: Season(ledger.season),
                        kind: row.kind,
                        authority: row.authority,
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
                "camp row counts do not reconcile with terminal organization coverage".to_owned(),
            ));
        }
        Ok(CampParticipationLedgerOutput {
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
struct CampParticipationLedgerWire {
    schema: String,
    season: u32,
    provider: String,
    captured_at: String,
    source_url: String,
    coverage: Vec<CampCoverageWire>,
    participants: Vec<CampParticipantWire>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampCoverageWire {
    organization: String,
    terminal: bool,
    records: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampParticipantWire {
    player_id: u32,
    organization: String,
    kind: ParticipationKind,
    authority: ParticipationAuthority,
    occurred_at: String,
    source_url: String,
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| "camp-participation timestamps must be RFC 3339".to_owned())
}

fn participation_kind_key(kind: ParticipationKind) -> &'static str {
    match kind {
        ParticipationKind::DevelopmentCamp => "development_camp",
        ParticipationKind::RookieCamp => "rookie_camp",
        ParticipationKind::TrainingCamp => "training_camp",
        ParticipationKind::ProspectTournament => "prospect_tournament",
    }
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
    fn terminal_counts_emit_attendance_without_control() {
        let bytes = br#"{
          "schema":"camp_participation_ledger.v1",
          "season":20262027,
          "provider":"fixture_camp_registry",
          "captured_at":"2026-07-31T12:00:00Z",
          "source_url":"https://example.test/camps",
          "coverage":[
            {"organization":"NYR","terminal":true,"records":1},
            {"organization":"SEA","terminal":true,"records":0}
          ],
          "participants":[{
            "player_id":8480004,
            "organization":"NYR",
            "kind":"development_camp",
            "authority":"controlled_player",
            "occurred_at":"2026-07-03T00:00:00Z",
            "source_url":"https://example.test/camps/nyr"
          }]
        }"#;
        let adapter = CampParticipationLedgerV1Adapter;
        let output = adapter
            .parse(SourceInput::new(
                bytes,
                adapter.descriptor().source_id,
                ContentHash::try_new("c".repeat(64)).unwrap(),
            ))
            .unwrap();
        assert_eq!(output.facts.len(), 1);
        assert!(matches!(
            output.facts[0].fact(),
            SourceFact::PlayerParticipation(PlayerParticipationFact {
                kind: ParticipationKind::DevelopmentCamp,
                ..
            })
        ));
    }

    #[test]
    fn duplicate_or_mismatched_rows_fail_closed() {
        let bytes = br#"{
          "schema":"camp_participation_ledger.v1",
          "season":20262027,
          "provider":"fixture_camp_registry",
          "captured_at":"2026-07-31T12:00:00Z",
          "source_url":"https://example.test/camps",
          "coverage":[{"organization":"NYR","terminal":true,"records":2}],
          "participants":[{
            "player_id":8480004,"organization":"NYR","kind":"development_camp",
            "authority":"controlled_player","occurred_at":"2026-07-03T00:00:00Z",
            "source_url":"https://example.test/camps/nyr"
          }]
        }"#;
        let adapter = CampParticipationLedgerV1Adapter;
        assert!(adapter
            .parse(SourceInput::new(
                bytes,
                adapter.descriptor().source_id,
                ContentHash::try_new("d".repeat(64)).unwrap(),
            ))
            .is_err());
    }
}
