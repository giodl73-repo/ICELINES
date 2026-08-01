//! Compatibility reader/lowering for `prospect_population_overlay.v1`.
//!
//! The legacy relationship remains visible and never masquerades as a newly
//! sourced legal-control event. Camp participation is lowered independently.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterVersion, CompatibilityProspectRelationshipFact, CompatibilityProspectRelationshipKind,
    ContentHash, EffectivePrecision, EffectiveTime, FactAssertion, FactAuthority, FactId,
    FactSubject, OrganizationId, ParticipationAuthority, ParticipationKind,
    PlayerParticipationFact, ProviderId, SourceContractError, SourceDisclosure,
    SourceDisclosureCode, SourceEvidence, SourceExclusion, SourceFact, SourceId, SourceUrl,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PROSPECT_POPULATION_OVERLAY_V1: &str = "prospect_population_overlay.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectPopulationOverlayV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub checked_at: String,
    pub candidates: Vec<ProspectPopulationCandidateV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectPopulationCandidateV1 {
    pub player_id: u32,
    pub display_name: String,
    pub team: String,
    pub position: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    pub source_url: String,
    #[serde(default)]
    pub relationship: ProspectPopulationRelationshipV1,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectPopulationRelationshipV1 {
    #[default]
    LegacyOrganizationalCandidate,
    OrganizationRights,
    NhlContract,
    AhlAssignment,
    DevelopmentCampParticipant,
    FreeAgentInvite,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProspectPopulationLoweringV1 {
    pub assertions: Vec<FactAssertion<SourceFact>>,
    pub exclusions: Vec<SourceExclusion>,
    pub disclosures: Vec<SourceDisclosure>,
}

pub fn lower_prospect_population_overlay_v1(
    overlay: ProspectPopulationOverlayV1,
    evaluation_season: Season,
    content_hash: ContentHash,
) -> Result<ProspectPopulationLoweringV1, SourceContractError> {
    if overlay
        .schema
        .as_deref()
        .is_some_and(|schema| schema != PROSPECT_POPULATION_OVERLAY_V1)
    {
        return Err(SourceContractError::UnsupportedSchema {
            found: overlay.schema.unwrap_or_default(),
        });
    }
    let checked_at = parse_checked_at(&overlay.checked_at)?;
    let mut player_ids = BTreeSet::new();
    let mut assertions = Vec::new();
    let mut exclusions = Vec::new();
    for candidate in overlay.candidates {
        let player_id = PlayerId::try_new(candidate.player_id)
            .map_err(|_| SourceContractError::ZeroPlayerId)?;
        if !player_ids.insert(candidate.player_id) {
            return Err(SourceContractError::DuplicateId {
                kind: "prospect_population_player",
                id: candidate.player_id.to_string(),
            });
        }
        if candidate.display_name.trim().is_empty() {
            return Err(SourceContractError::Empty("display_name"));
        }
        let organization = OrganizationId::try_new(candidate.team.trim().to_ascii_uppercase())?;
        let evidence = SourceEvidence::new(
            SourceId::try_new(format!("prospect-overlay:{}", candidate.player_id))?,
            SourceUrl::try_new(candidate.source_url)?,
            ProviderId::try_new("authored_compatibility_overlay")?,
            checked_at,
            content_hash.clone(),
            AdapterVersion::try_new("prospect_population_overlay_lowering.v1")?,
        );
        let effective = EffectiveTime::new(checked_at, None, EffectivePrecision::Day)?;
        match candidate.relationship {
            ProspectPopulationRelationshipV1::DevelopmentCampParticipant
            | ProspectPopulationRelationshipV1::FreeAgentInvite => {
                let authority = if candidate.relationship
                    == ProspectPopulationRelationshipV1::FreeAgentInvite
                {
                    ParticipationAuthority::FreeAgentInvite
                } else {
                    ParticipationAuthority::Unknown
                };
                assertions.push(FactAssertion::new(
                    FactId::try_new(format!("compat-camp:{}", candidate.player_id))?,
                    format!(
                        "player:{}:participation:{}:{}",
                        candidate.player_id,
                        organization.as_str(),
                        evaluation_season.0
                    ),
                    FactSubject::Player(player_id),
                    effective,
                    FactAuthority::Attendance,
                    SourceFact::PlayerParticipation(PlayerParticipationFact {
                        organization,
                        season: evaluation_season,
                        kind: ParticipationKind::DevelopmentCamp,
                        authority,
                    }),
                    vec![evidence],
                )?);
            }
            ProspectPopulationRelationshipV1::Unknown => {
                exclusions.push(SourceExclusion {
                    exclusion_id: format!("compat-unknown:{}", candidate.player_id),
                    stage: "relationship_lowering".to_owned(),
                    subject: Some(FactSubject::Player(player_id)),
                    reason_code: "unknown_legacy_relationship".to_owned(),
                    message: "The compatibility row does not establish control or participation."
                        .to_owned(),
                    source_ids: vec![evidence.source_id().clone()],
                });
            }
            relationship => {
                let relationship = match relationship {
                    ProspectPopulationRelationshipV1::LegacyOrganizationalCandidate => {
                        CompatibilityProspectRelationshipKind::LegacyOrganizationalCandidate
                    }
                    ProspectPopulationRelationshipV1::OrganizationRights => {
                        CompatibilityProspectRelationshipKind::OrganizationRights
                    }
                    ProspectPopulationRelationshipV1::NhlContract => {
                        CompatibilityProspectRelationshipKind::NhlContract
                    }
                    ProspectPopulationRelationshipV1::AhlAssignment => {
                        CompatibilityProspectRelationshipKind::AhlAssignment
                    }
                    _ => unreachable!("participation and unknown handled above"),
                };
                assertions.push(FactAssertion::new(
                    FactId::try_new(format!("compat-relationship:{}", candidate.player_id))?,
                    format!(
                        "player:{}:compat_relationship:{}",
                        candidate.player_id,
                        organization.as_str()
                    ),
                    FactSubject::Player(player_id),
                    effective,
                    FactAuthority::Compatibility,
                    SourceFact::CompatibilityProspectRelationship(
                        CompatibilityProspectRelationshipFact {
                            organization,
                            relationship,
                        },
                    ),
                    vec![evidence],
                )?);
            }
        }
    }
    Ok(ProspectPopulationLoweringV1 {
        assertions,
        exclusions,
        disclosures: vec![SourceDisclosure {
            code: SourceDisclosureCode::PartialPopulation,
            scope: "prospect_population_overlay.v1".to_owned(),
            message: "Legacy relationship rows retain compatibility classification; they are not newly inferred organization-control events.".to_owned(),
        }],
    })
}

fn parse_checked_at(value: &str) -> Result<DateTime<Utc>, SourceContractError> {
    if let Ok(value) = DateTime::parse_from_rfc3339(value) {
        return Ok(value.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| SourceContractError::Empty("checked_at must be YYYY-MM-DD or RFC 3339"))?;
    Utc.from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or(SourceContractError::Empty("checked_at is ambiguous"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash() -> ContentHash {
        ContentHash::try_new("a".repeat(64)).unwrap()
    }

    #[test]
    fn legacy_rows_round_trip_and_lower_only_to_compatibility_authority() {
        let overlay: ProspectPopulationOverlayV1 = serde_json::from_str(
            r#"{"checked_at":"2026-07-31","candidates":[{"player_id":1,"display_name":"Legacy Player","team":"SEA","position":"C","birth_date":null,"source_url":"https://example.com/player"}]}"#,
        )
        .unwrap();
        let decoded: ProspectPopulationOverlayV1 =
            serde_json::from_slice(&serde_json::to_vec(&overlay).unwrap()).unwrap();
        let result =
            lower_prospect_population_overlay_v1(decoded, Season(20_262_027), hash()).unwrap();
        assert_eq!(result.assertions.len(), 1);
        assert!(matches!(
            result.assertions[0].fact(),
            SourceFact::CompatibilityProspectRelationship(_)
        ));
    }

    #[test]
    fn development_camp_attendance_does_not_claim_control() {
        let overlay: ProspectPopulationOverlayV1 = serde_json::from_str(
            r#"{"schema":"prospect_population_overlay.v1","checked_at":"2026-07-31","candidates":[{"player_id":2,"display_name":"Camp Player","team":"NYR","position":"D","birth_date":null,"source_url":"https://example.com/camp","relationship":"development_camp_participant"}]}"#,
        )
        .unwrap();
        let result =
            lower_prospect_population_overlay_v1(overlay, Season(20_262_027), hash()).unwrap();
        let SourceFact::PlayerParticipation(participation) = result.assertions[0].fact() else {
            panic!("expected participation fact");
        };
        assert_eq!(participation.authority, ParticipationAuthority::Unknown);
    }

    #[test]
    fn unknown_relationship_becomes_a_typed_exclusion() {
        let overlay: ProspectPopulationOverlayV1 = serde_json::from_str(
            r#"{"schema":"prospect_population_overlay.v1","checked_at":"2026-07-31","candidates":[{"player_id":3,"display_name":"Unknown Player","team":"BOS","position":"G","birth_date":null,"source_url":"https://example.com/unknown","relationship":"unknown"}]}"#,
        )
        .unwrap();
        let result =
            lower_prospect_population_overlay_v1(overlay, Season(20_262_027), hash()).unwrap();
        assert!(result.assertions.is_empty());
        assert_eq!(
            result.exclusions[0].reason_code,
            "unknown_legacy_relationship"
        );
    }
}
