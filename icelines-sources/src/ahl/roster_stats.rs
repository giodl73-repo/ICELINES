//! Compatibility adapter for IceLines' reviewed official AHL roster snapshot.
//!
//! AHL provider IDs remain provider-scoped. The adapter stages club-roster
//! observations and identity proposals; canonical assignment facts require a
//! separate reviewed identity decision.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, Utc};
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, ClubRef, EffectivePrecision, EffectiveTime, FactAuthority,
    FreshnessClass, OrganizationId, PlayerOrganizationEvent, ProposalId, ProviderId,
    ProviderIdentityProposal, ProviderPersonLocator, SourceEvidence, SourceFact, SourceId,
    SourceUrl, StagedAssertionId, StagedPlayerAssertion,
};
use serde::Deserialize;
use std::collections::BTreeSet;

pub const AHL_ROSTER_STATS_V1: &str = "ahl_roster_stats.v1";
pub const AHL_PROVIDER: &str = "ahl_hockeytech_statview";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedAhlRosterObservation {
    pub proposal_id: ProposalId,
    pub provider_player_id: String,
    pub season: Season,
    pub ahl_club: ClubRef,
    pub nhl_affiliate: Option<OrganizationId>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AhlRosterStatsOutput {
    pub identity_proposals: Vec<ProviderIdentityProposal>,
    pub roster_observations: Vec<StagedAhlRosterObservation>,
    pub staged_assertions: Vec<StagedPlayerAssertion>,
}

#[derive(Debug, Clone, Default)]
pub struct AhlRosterStatsV1Adapter;

impl SourceAdapter for AhlRosterStatsV1Adapter {
    type Output = AhlRosterStatsOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new("ahl-roster-stats").expect("static source id is valid"),
            provider: ProviderId::try_new(AHL_PROVIDER).expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("ahl.roster_stats.compatibility")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_ahl_roster_snapshot",
            supported_layouts: &[AHL_ROSTER_STATS_V1],
            required_identity_keys: &["provider", "provider_player_id"],
            additive_field_policy: AdditiveFieldPolicy::IgnoreReviewed,
            freshness_class: FreshnessClass::Roster,
            historical_availability: HistoricalAvailability::CallerSuppliedArchive,
            absence_semantics: AbsenceSemantics::AuthoritativeEmpty,
            output_fact_families: &["identity_proposal", "staged_ahl_rostered"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let snapshot: AhlRosterStatsWire =
            serde_json::from_slice(input.bytes()).map_err(|error| AdapterError {
                source_id: input.source_id().clone(),
                adapter_id: descriptor.adapter_id.clone(),
                input_hash: input.content_hash().clone(),
                category: AdapterErrorCategory::UnsupportedLayout,
                disposition: AdapterDisposition::FatalSource,
                message: format!("invalid AHL roster snapshot: {error}"),
            })?;
        if snapshot.schema != AHL_ROSTER_STATS_V1 || snapshot.provider != AHL_PROVIDER {
            return Err(adapter_error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("expected schema {AHL_ROSTER_STATS_V1} and provider {AHL_PROVIDER}"),
            ));
        }
        let observed_at = DateTime::parse_from_rfc3339(&snapshot.fetched_at)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    "AHL fetched_at must be RFC 3339".to_owned(),
                )
            })?;
        let source_url = SourceUrl::try_new(snapshot.roster_source_url).map_err(|error| {
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
            descriptor.provider.clone(),
            observed_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let season = Season(snapshot.season);
        let mut team_codes = BTreeSet::new();
        let mut identity_proposals = Vec::new();
        let mut roster_observations = Vec::new();
        let mut staged_assertions = Vec::new();
        for team in snapshot.teams {
            let team_code = validate_code(&team.team_code, "AHL team code").map_err(|message| {
                adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    message,
                )
            })?;
            if !team_codes.insert(team_code.clone()) {
                return Err(adapter_error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    format!("duplicate AHL team code {team_code}"),
                ));
            }
            let nhl_affiliate = team
                .nhl_affiliate
                .map(|value| validate_code(&value, "NHL affiliate"))
                .transpose()
                .map_err(|message| {
                    adapter_error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::MalformedRecord,
                        message,
                    )
                })?
                .map(OrganizationId::try_new)
                .transpose()
                .map_err(|error| {
                    adapter_error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::MalformedRecord,
                        error.to_string(),
                    )
                })?;
            let ahl_club = ClubRef::try_new(format!("AHL:{team_code}"))
                .expect("validated team code produces a club reference");
            let mut provider_ids = BTreeSet::new();
            for player in team.roster {
                if player.provider != AHL_PROVIDER
                    || player.provider_player_id.trim().is_empty()
                    || player.name.trim().is_empty()
                {
                    return Err(adapter_error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::MalformedRecord,
                        format!("invalid AHL roster identity in {team_code}"),
                    ));
                }
                let provider_player_id = player.provider_player_id.clone();
                if !provider_ids.insert(provider_player_id.clone()) {
                    return Err(adapter_error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        format!(
                            "duplicate AHL provider player {} in {team_code}",
                            player.provider_player_id
                        ),
                    ));
                }
                let proposal_id = ProposalId::try_new(format!(
                    "ahl-roster:{}:{}:{}",
                    snapshot.season, team_code, player.provider_player_id
                ))
                .expect("validated AHL fields produce a proposal id");
                identity_proposals.push(
                    ProviderIdentityProposal::new(
                        proposal_id.clone(),
                        ProviderPersonLocator::StableId {
                            provider: descriptor.provider.clone(),
                            provider_player_id: provider_player_id.clone(),
                        },
                        player.name,
                        nonempty(player.birthdate),
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
                roster_observations.push(StagedAhlRosterObservation {
                    proposal_id: proposal_id.clone(),
                    provider_player_id,
                    season,
                    ahl_club: ahl_club.clone(),
                    nhl_affiliate: nhl_affiliate.clone(),
                    observed_at,
                });
                staged_assertions.push(
                    StagedPlayerAssertion::new(
                        StagedAssertionId::try_new(format!("staged:{proposal_id}:ahl-rostered"))
                            .expect("validated proposal id produces a staged assertion id"),
                        format!("proposal:{proposal_id}:ahl-rostered"),
                        proposal_id,
                        EffectiveTime::new(observed_at, None, EffectivePrecision::Instant)
                            .expect("single-ended effective time is valid"),
                        FactAuthority::Assignment,
                        SourceFact::PlayerOrganization(match &nhl_affiliate {
                            Some(affiliate) => PlayerOrganizationEvent::AffiliateRostered {
                                affiliate: affiliate.clone(),
                                at: ahl_club.clone(),
                            },
                            None => PlayerOrganizationEvent::Rostered {
                                at: ahl_club.clone(),
                            },
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
        }
        Ok(AhlRosterStatsOutput {
            identity_proposals,
            roster_observations,
            staged_assertions,
        })
    }
}

#[derive(Debug, Deserialize)]
struct AhlRosterStatsWire {
    schema: String,
    season: u32,
    provider: String,
    fetched_at: String,
    roster_source_url: String,
    teams: Vec<AhlTeamWire>,
}

#[derive(Debug, Deserialize)]
struct AhlTeamWire {
    team_code: String,
    #[serde(default)]
    nhl_affiliate: Option<String>,
    roster: Vec<AhlPlayerWire>,
}

#[derive(Debug, Deserialize)]
struct AhlPlayerWire {
    provider: String,
    provider_player_id: String,
    name: String,
    #[serde(default)]
    birthdate: String,
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

fn validate_code(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_uppercase();
    if !(2..=4).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err(format!("{label} must be a 2-4 letter uppercase code"));
    }
    Ok(value)
}

fn nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}
