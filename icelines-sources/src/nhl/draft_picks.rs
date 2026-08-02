//! Official NHL multi-year draft-picks ledger adapter.
//!
//! The league endpoint supplies names and selection details but no canonical
//! NHL player ID. Picks are staged behind identity review rather than joined
//! by name.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, TimeZone, Utc};
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, EffectivePrecision, EffectiveTime, FactAuthority, FreshnessClass,
    OrganizationId, PlayerOrganizationEvent, ProposalId, ProviderId, ProviderIdentityProposal,
    ProviderPersonLocator, SourceEvidence, SourceFact, SourceId, SourceUrl, StagedAssertionId,
    StagedPlayerAssertion,
};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedDraftSelection {
    pub proposal_id: ProposalId,
    pub organization: OrganizationId,
    pub year: u16,
    pub round: u8,
    pub overall: u16,
    pub position_code: String,
    pub occurred_at: EffectiveTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForfeitedDraftSlot {
    pub organization: OrganizationId,
    pub round: u8,
    pub overall: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftPicksOutput {
    pub identity_proposals: Vec<ProviderIdentityProposal>,
    pub selections: Vec<StagedDraftSelection>,
    pub staged_assertions: Vec<StagedPlayerAssertion>,
    pub forfeited_slots: Vec<ForfeitedDraftSlot>,
}

#[derive(Debug, Clone)]
pub struct OfficialNhlDraftPicksAdapter {
    year: u16,
    captured_at: DateTime<Utc>,
}

impl OfficialNhlDraftPicksAdapter {
    pub fn new(year: u16, captured_at: DateTime<Utc>) -> Result<Self, String> {
        if !(1979..=2200).contains(&year) {
            return Err("draft year is outside the supported endpoint range".to_owned());
        }
        Ok(Self { year, captured_at })
    }
}

impl SourceAdapter for OfficialNhlDraftPicksAdapter {
    type Output = DraftPicksOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!("nhl-draft-picks:{}", self.year))
                .expect("validated draft year produces a source id"),
            provider: ProviderId::try_new("official_nhl_api").expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.draft_picks.all")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_draft_picks",
            supported_layouts: &["nhl_draft_picks.all.v1"],
            required_identity_keys: &["draftYear", "overallPick", "displayed_name"],
            additive_field_policy: AdditiveFieldPolicy::IgnoreReviewed,
            freshness_class: FreshnessClass::Static,
            historical_availability: HistoricalAvailability::ProviderArchive,
            absence_semantics: AbsenceSemantics::AuthoritativeEmpty,
            output_fact_families: &["identity_proposal", "staged_draft_selection"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let ledger: DraftLedgerWire =
            serde_json::from_slice(input.bytes()).map_err(|error| AdapterError {
                source_id: input.source_id().clone(),
                adapter_id: descriptor.adapter_id.clone(),
                input_hash: input.content_hash().clone(),
                category: AdapterErrorCategory::UnsupportedLayout,
                disposition: AdapterDisposition::FatalSource,
                message: format!("invalid NHL draft-picks ledger: {error}"),
            })?;
        if ledger.draft_year != self.year || ledger.state != "over" {
            return Err(adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::SemanticValidation,
                "draft ledger must match the requested year and have terminal state `over`"
                    .to_owned(),
            ));
        }
        let (occurred_at, effective_precision) =
            if let Some(value) = ledger.broadcast_start_time_utc.as_deref() {
                (
                    DateTime::parse_from_rfc3339(value)
                        .map(|value| value.with_timezone(&Utc))
                        .map_err(|_| {
                            adapter_error(
                                &input,
                                &descriptor,
                                AdapterErrorCategory::MalformedRecord,
                                "broadcastStartTimeUTC must be RFC 3339 when present".to_owned(),
                            )
                        })?,
                    EffectivePrecision::Day,
                )
            } else {
                (
                    Utc.with_ymd_and_hms(i32::from(self.year), 7, 1, 0, 0, 0)
                        .single()
                        .expect("validated draft year has a representative UTC date"),
                    EffectivePrecision::Unknown,
                )
            };
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            SourceUrl::try_new(format!(
                "https://api-web.nhle.com/v1/draft/picks/{}/all",
                self.year
            ))
            .expect("official draft endpoint is valid"),
            descriptor.provider.clone(),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let mut overalls = BTreeSet::new();
        let mut identity_proposals = Vec::with_capacity(ledger.picks.len());
        let mut selections = Vec::with_capacity(ledger.picks.len());
        let mut staged_assertions = Vec::with_capacity(ledger.picks.len());
        let mut forfeited_slots = Vec::new();
        for pick in ledger.picks {
            if pick.round == 0
                || pick.pick_in_round == 0
                || pick.overall_pick == 0
                || !overalls.insert(pick.overall_pick)
            {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    format!("invalid or duplicate overall pick {}", pick.overall_pick),
                ));
            }
            let team = validate_team(&pick.team_abbrev).map_err(|message| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    message,
                )
            })?;
            if pick.first_name.is_none() && pick.last_name.default.eq_ignore_ascii_case("forfeited")
            {
                forfeited_slots.push(ForfeitedDraftSlot {
                    organization: OrganizationId::try_new(team)
                        .expect("validated team abbreviation is an organization id"),
                    round: pick.round,
                    overall: pick.overall_pick,
                });
                continue;
            }
            let first_name = pick.first_name.ok_or_else(|| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    format!("draft pick {} is missing firstName", pick.overall_pick),
                )
            })?;
            let position_code = pick.position_code.ok_or_else(|| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    format!("draft pick {} is missing positionCode", pick.overall_pick),
                )
            })?;
            let displayed_name = format!(
                "{} {}",
                first_name.default.trim(),
                pick.last_name.default.trim()
            )
            .trim()
            .to_owned();
            if displayed_name.is_empty() || position_code.trim().is_empty() {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    format!(
                        "draft pick {} is missing identity fields",
                        pick.overall_pick
                    ),
                ));
            }
            let proposal_id =
                ProposalId::try_new(format!("nhl-draft:{}:{}", self.year, pick.overall_pick))
                    .expect("validated draft fields produce a proposal id");
            identity_proposals.push(
                ProviderIdentityProposal::new(
                    proposal_id.clone(),
                    ProviderPersonLocator::SourceRow {
                        source_id: input.source_id().clone(),
                        row_key: format!("overall-pick:{}", pick.overall_pick),
                    },
                    displayed_name,
                    None,
                    None,
                    vec![evidence.clone()],
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
            let organization = OrganizationId::try_new(team)
                .expect("validated team abbreviation is an organization id");
            let selection_time = EffectiveTime::new(occurred_at, None, effective_precision)
                .expect("single-ended effective time is valid");
            selections.push(StagedDraftSelection {
                proposal_id: proposal_id.clone(),
                organization: organization.clone(),
                year: self.year,
                round: pick.round,
                overall: pick.overall_pick,
                position_code,
                occurred_at: selection_time.clone(),
            });
            staged_assertions.push(
                StagedPlayerAssertion::new(
                    StagedAssertionId::try_new(format!("staged:{proposal_id}:draft"))
                        .expect("validated proposal id produces a staged assertion id"),
                    format!("proposal:{proposal_id}:draft"),
                    proposal_id,
                    selection_time,
                    FactAuthority::Draft,
                    SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                        by: organization,
                        year: self.year,
                        round: pick.round,
                        overall: pick.overall_pick,
                    }),
                    vec![evidence.clone()],
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
        }
        Ok(DraftPicksOutput {
            identity_proposals,
            selections,
            staged_assertions,
            forfeited_slots,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftLedgerWire {
    #[serde(rename = "broadcastStartTimeUTC")]
    broadcast_start_time_utc: Option<String>,
    draft_year: u16,
    state: String,
    picks: Vec<DraftPickWire>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftPickWire {
    round: u8,
    pick_in_round: u16,
    overall_pick: u16,
    team_abbrev: String,
    first_name: Option<LocalizedDefault>,
    last_name: LocalizedDefault,
    position_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalizedDefault {
    default: String,
}

fn validate_team(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_uppercase();
    if !(2..=4).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("draft teamAbbrev must be a 2-4 letter code".to_owned());
    }
    Ok(value)
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
    use chrono::{Datelike, TimeZone};
    use icelines_core::source_facts::ContentHash;

    #[test]
    fn archived_terminal_ledger_without_broadcast_time_uses_unknown_precision() {
        let bytes = br#"{
            "draftYear":2022,
            "state":"over",
            "picks":[{
                "round":1,
                "pickInRound":1,
                "overallPick":1,
                "teamAbbrev":"MTL",
                "firstName":{"default":"Juraj"},
                "lastName":{"default":"Slafkovsky"},
                "positionCode":"LW"
            }]
        }"#;
        let adapter = OfficialNhlDraftPicksAdapter::new(
            2022,
            Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
                .single()
                .unwrap(),
        )
        .unwrap();
        let output = adapter
            .parse(SourceInput::new(
                bytes,
                adapter.descriptor().source_id,
                ContentHash::try_new("a".repeat(64)).unwrap(),
            ))
            .unwrap();

        assert_eq!(output.selections.len(), 1);
        assert_eq!(
            output.selections[0].occurred_at.precision,
            EffectivePrecision::Unknown
        );
        assert_eq!(output.selections[0].occurred_at.starts_at.year(), 2022);
    }
}
