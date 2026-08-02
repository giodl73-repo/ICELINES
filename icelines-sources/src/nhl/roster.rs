//! Official NHL club-roster adapter.
//!
//! Roster publication is assignment evidence only. It does not prove which
//! organization owns a player's rights or which contract the player signed.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, ClubRef, EffectivePrecision, EffectiveTime, FactAssertion,
    FactAuthority, FactId, FactSubject, FreshnessClass, OrganizationId, PlayerOrganizationEvent,
    ProviderId, SourceEvidence, SourceFact, SourceId, SourceUrl,
};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct OfficialNhlRosterAdapter {
    team: String,
    organization: OrganizationId,
    club: ClubRef,
    observed_at: DateTime<Utc>,
}

impl OfficialNhlRosterAdapter {
    pub fn new(team: &str, observed_at: DateTime<Utc>) -> Result<Self, String> {
        let team = team.trim().to_ascii_uppercase();
        if !(2..=4).contains(&team.len()) || !team.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err("team must be a 2-4 letter NHL abbreviation".to_owned());
        }
        Ok(Self {
            organization: OrganizationId::try_new(team.clone())
                .map_err(|error| error.to_string())?,
            club: ClubRef::try_new(format!("NHL:{team}")).map_err(|error| error.to_string())?,
            team,
            observed_at,
        })
    }
}

impl SourceAdapter for OfficialNhlRosterAdapter {
    type Output = Vec<FactAssertion<SourceFact>>;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!("nhl-roster:{}", self.team))
                .expect("validated team produces a valid source id"),
            provider: ProviderId::try_new("official_nhl_api").expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.club_roster.assignment")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_club_roster",
            supported_layouts: &["nhl_club_roster.v1"],
            required_identity_keys: &["id"],
            additive_field_policy: AdditiveFieldPolicy::IgnoreReviewed,
            freshness_class: FreshnessClass::Roster,
            historical_availability: HistoricalAvailability::CallerSuppliedArchive,
            absence_semantics: AbsenceSemantics::AuthoritativeEmpty,
            output_fact_families: &["player_organization.assigned"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let roster: RosterResponse = serde_json::from_slice(input.bytes()).map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                AdapterDisposition::FatalSource,
                format!("official roster layout is invalid: {error}"),
            )
        })?;
        let mut seen = BTreeSet::new();
        let mut assertions = Vec::new();
        for (group, player) in roster.players() {
            if player.id == 0 {
                return Err(self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    AdapterDisposition::QuarantinedRecord,
                    format!("{group} roster contains player id 0"),
                ));
            }
            if !seen.insert(player.id) {
                return Err(self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    AdapterDisposition::QuarantinedRecord,
                    format!(
                        "player {} occurs more than once in roster groups",
                        player.id
                    ),
                ));
            }
            if !group.accepts(&player.position_code) {
                return Err(self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    AdapterDisposition::QuarantinedRecord,
                    format!(
                        "player {} position {} conflicts with {group} roster group",
                        player.id, player.position_code
                    ),
                ));
            }
            let player_id = PlayerId::try_new(player.id).expect("zero player id handled above");
            let evidence = SourceEvidence::new(
                input.source_id().clone(),
                SourceUrl::try_new(format!(
                    "https://api-web.nhle.com/v1/roster/{}/current",
                    self.team
                ))
                .expect("official NHL roster URL is valid"),
                ProviderId::try_new("official_nhl_api").expect("static provider id is valid"),
                self.observed_at,
                input.content_hash().clone(),
                descriptor.adapter_version.clone(),
            );
            let effective = EffectiveTime::new(self.observed_at, None, EffectivePrecision::Instant)
                .expect("single-ended effective time is valid");
            let assertion = FactAssertion::new(
                FactId::try_new(format!(
                    "nhl-roster:{}:{}:{}",
                    self.team,
                    player.id,
                    self.observed_at.timestamp()
                ))
                .expect("roster fact id is valid"),
                format!("player:{}:assignment:NHL:{}", player.id, self.team),
                FactSubject::Player(player_id),
                effective,
                FactAuthority::Assignment,
                SourceFact::PlayerOrganization(PlayerOrganizationEvent::Assigned {
                    by: self.organization.clone(),
                    to: self.club.clone(),
                }),
                vec![evidence],
            )
            .map_err(|error| {
                self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::SemanticValidation,
                    AdapterDisposition::QuarantinedRecord,
                    error.to_string(),
                )
            })?;
            assertions.push(assertion);
        }
        assertions.sort_by_key(|assertion| assertion.fact_id().as_str().to_owned());
        Ok(assertions)
    }
}

impl OfficialNhlRosterAdapter {
    fn error(
        &self,
        input: &SourceInput<'_>,
        descriptor: &SourceDescriptor,
        category: AdapterErrorCategory,
        disposition: AdapterDisposition,
        message: String,
    ) -> AdapterError {
        AdapterError {
            source_id: input.source_id().clone(),
            adapter_id: descriptor.adapter_id.clone(),
            input_hash: input.content_hash().clone(),
            category,
            disposition,
            message,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RosterResponse {
    forwards: Vec<RosterPlayer>,
    defensemen: Vec<RosterPlayer>,
    goalies: Vec<RosterPlayer>,
}

impl RosterResponse {
    fn players(&self) -> impl Iterator<Item = (RosterGroup, &RosterPlayer)> {
        self.forwards
            .iter()
            .map(|player| (RosterGroup::Forward, player))
            .chain(
                self.defensemen
                    .iter()
                    .map(|player| (RosterGroup::Defense, player)),
            )
            .chain(
                self.goalies
                    .iter()
                    .map(|player| (RosterGroup::Goalie, player)),
            )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RosterPlayer {
    id: u32,
    position_code: String,
}

#[derive(Debug, Clone, Copy)]
enum RosterGroup {
    Forward,
    Defense,
    Goalie,
}

impl RosterGroup {
    fn accepts(self, position: &str) -> bool {
        match self {
            Self::Forward => matches!(position, "C" | "L" | "R"),
            Self::Defense => position == "D",
            Self::Goalie => position == "G",
        }
    }
}

impl std::fmt::Display for RosterGroup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Forward => "forward",
            Self::Defense => "defense",
            Self::Goalie => "goalie",
        })
    }
}
