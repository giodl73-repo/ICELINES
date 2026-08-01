//! Source-bound organization candidate population shared by training-camp and
//! prospect-ranking adapters.

use serde::{Deserialize, Serialize};

pub const PROSPECT_POPULATION_OVERLAY_SCHEMA: &str = "prospect_population_overlay.v1";
pub const PROSPECT_POPULATION_AUDIT_SCHEMA: &str = "prospect_population_audit.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectPopulationTeamAuditView {
    pub team: String,
    pub candidates: usize,
    pub ranking_eligible: usize,
    pub camp_only: usize,
    pub legacy_relationships: usize,
    pub unknown_relationships: usize,
    pub relationship_counts: std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectPopulationAuditView {
    pub schema: String,
    pub checked_at: String,
    pub candidates: usize,
    pub ranking_eligible: usize,
    pub camp_only: usize,
    pub legacy_relationships: usize,
    pub unknown_relationships: usize,
    pub fully_classified: bool,
    pub teams: Vec<ProspectPopulationTeamAuditView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectPopulationOverlay {
    /// Optional for compatibility with the original camp-candidate envelope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub checked_at: String,
    pub candidates: Vec<ProspectPopulationCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectPopulationCandidate {
    pub player_id: u32,
    pub display_name: String,
    pub team: String,
    pub position: String,
    pub birth_date: Option<String>,
    pub source_url: String,
    /// Relationship actually established by the cited source. Older overlays
    /// deserialize to the compatibility state and retain their prior behavior.
    #[serde(default)]
    pub relationship: ProspectPopulationRelationship,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectPopulationRelationship {
    /// Compatibility state for overlays authored before relationship typing.
    #[default]
    LegacyOrganizationalCandidate,
    OrganizationRights,
    NhlContract,
    AhlAssignment,
    DevelopmentCampParticipant,
    FreeAgentInvite,
    Unknown,
}

impl ProspectPopulationRelationship {
    pub fn supports_prospect_ranking(self) -> bool {
        matches!(
            self,
            Self::LegacyOrganizationalCandidate
                | Self::OrganizationRights
                | Self::NhlContract
                | Self::AhlAssignment
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::LegacyOrganizationalCandidate => "legacy_organizational_candidate",
            Self::OrganizationRights => "organization_rights",
            Self::NhlContract => "nhl_contract",
            Self::AhlAssignment => "ahl_assignment",
            Self::DevelopmentCampParticipant => "development_camp_participant",
            Self::FreeAgentInvite => "free_agent_invite",
            Self::Unknown => "unknown",
        }
    }
}

impl ProspectPopulationOverlay {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .schema
            .as_deref()
            .is_some_and(|schema| schema != PROSPECT_POPULATION_OVERLAY_SCHEMA)
        {
            return Err(format!(
                "unsupported prospect population overlay schema {}; expected {PROSPECT_POPULATION_OVERLAY_SCHEMA}",
                self.schema.as_deref().unwrap_or_default(),
            ));
        }
        if self.checked_at.trim().is_empty() {
            return Err("prospect population overlay checked_at must not be empty".to_owned());
        }
        if chrono::NaiveDate::parse_from_str(&self.checked_at, "%Y-%m-%d").is_err()
            && chrono::DateTime::parse_from_rfc3339(&self.checked_at).is_err()
        {
            return Err(
                "prospect population overlay checked_at must be YYYY-MM-DD or RFC 3339".to_owned(),
            );
        }
        let mut ids = std::collections::BTreeSet::new();
        for candidate in &self.candidates {
            if candidate.player_id == 0 {
                return Err("prospect population candidate player_id must be non-zero".to_owned());
            }
            if !ids.insert(candidate.player_id) {
                return Err(format!(
                    "prospect population overlay contains duplicate player {}",
                    candidate.player_id
                ));
            }
            let team = candidate.team.trim();
            if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                return Err(format!("invalid candidate overlay team {}", candidate.team));
            }
            if candidate.display_name.trim().is_empty() {
                return Err(format!(
                    "candidate {} has an empty display_name",
                    candidate.player_id
                ));
            }
            if candidate
                .birth_date
                .as_deref()
                .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err())
            {
                return Err(format!(
                    "candidate {} has invalid birth_date",
                    candidate.player_id
                ));
            }
            if !matches!(
                candidate.position.as_str(),
                "C" | "L" | "R" | "LW" | "RW" | "D" | "G"
            ) {
                return Err(format!(
                    "candidate {} has unsupported position {}",
                    candidate.player_id, candidate.position
                ));
            }
            if !(candidate.source_url.starts_with("https://")
                || candidate.source_url.starts_with("http://"))
            {
                return Err(format!(
                    "candidate {} requires an absolute http(s) source_url",
                    candidate.player_id
                ));
            }
        }
        Ok(())
    }

    pub fn audit(&self) -> Result<ProspectPopulationAuditView, String> {
        self.validate()?;
        let mut by_team =
            std::collections::BTreeMap::<String, Vec<&ProspectPopulationCandidate>>::new();
        for candidate in &self.candidates {
            by_team
                .entry(candidate.team.trim().to_ascii_uppercase())
                .or_default()
                .push(candidate);
        }
        let teams = by_team
            .into_iter()
            .map(|(team, candidates)| {
                let mut relationship_counts = std::collections::BTreeMap::new();
                for candidate in &candidates {
                    *relationship_counts
                        .entry(candidate.relationship.label().to_owned())
                        .or_insert(0) += 1;
                }
                let ranking_eligible = candidates
                    .iter()
                    .filter(|candidate| candidate.relationship.supports_prospect_ranking())
                    .count();
                let legacy_relationships = candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.relationship
                            == ProspectPopulationRelationship::LegacyOrganizationalCandidate
                    })
                    .count();
                let unknown_relationships = candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.relationship == ProspectPopulationRelationship::Unknown
                    })
                    .count();
                ProspectPopulationTeamAuditView {
                    team,
                    candidates: candidates.len(),
                    ranking_eligible,
                    camp_only: candidates.len() - ranking_eligible,
                    legacy_relationships,
                    unknown_relationships,
                    relationship_counts,
                }
            })
            .collect::<Vec<_>>();
        let ranking_eligible = teams.iter().map(|team| team.ranking_eligible).sum();
        let legacy_relationships = teams.iter().map(|team| team.legacy_relationships).sum();
        let unknown_relationships = teams.iter().map(|team| team.unknown_relationships).sum();
        Ok(ProspectPopulationAuditView {
            schema: PROSPECT_POPULATION_AUDIT_SCHEMA.to_owned(),
            checked_at: self.checked_at.clone(),
            candidates: self.candidates.len(),
            ranking_eligible,
            camp_only: self.candidates.len() - ranking_eligible,
            legacy_relationships,
            unknown_relationships,
            fully_classified: legacy_relationships == 0 && unknown_relationships == 0,
            teams,
            disclosures: vec![
                "Population relationships are source-authority facts layered onto canonical NHL player IDs; this audit does not resolve or mutate identity.".to_owned(),
                "Organization rights, NHL contracts, and AHL assignments support prospect ranking. Development-camp participants, free-agent invitees, and unknown relationships remain camp-only.".to_owned(),
                "Legacy organizational candidates retain compatibility behavior but prevent fully_classified authority until their relationship is explicitly sourced.".to_owned(),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ProspectPopulationOverlay, ProspectPopulationRelationship,
        PROSPECT_POPULATION_OVERLAY_SCHEMA,
    };

    #[test]
    fn legacy_overlay_defaults_without_claiming_a_new_relationship() {
        let overlay: ProspectPopulationOverlay = serde_json::from_str(
            r#"{"checked_at":"2026-07-31","candidates":[{"player_id":1,"display_name":"Camp Player","team":"SEA","position":"C","birth_date":"2005-01-01","source_url":"https://www.nhl.com/kraken"}]}"#,
        )
        .unwrap();
        overlay.validate().unwrap();
        assert_eq!(overlay.schema, None);
        assert_eq!(
            overlay.candidates[0].relationship,
            ProspectPopulationRelationship::LegacyOrganizationalCandidate
        );
    }

    #[test]
    fn invite_relationship_is_camp_only_and_schema_is_validated() {
        let mut overlay: ProspectPopulationOverlay = serde_json::from_str(
            r#"{"schema":"prospect_population_overlay.v1","checked_at":"2026-07-31","candidates":[{"player_id":1,"display_name":"Camp Invite","team":"SEA","position":"C","birth_date":null,"source_url":"https://www.nhl.com/kraken","relationship":"free_agent_invite"}]}"#,
        )
        .unwrap();
        overlay.validate().unwrap();
        assert!(!overlay.candidates[0]
            .relationship
            .supports_prospect_ranking());

        overlay.schema = Some("future.v2".to_owned());
        assert!(overlay
            .validate()
            .unwrap_err()
            .contains(PROSPECT_POPULATION_OVERLAY_SCHEMA));
    }

    #[test]
    fn audit_separates_ranking_camp_and_legacy_authority() {
        let overlay: ProspectPopulationOverlay = serde_json::from_str(
            r#"{"schema":"prospect_population_overlay.v1","checked_at":"2026-07-31","candidates":[{"player_id":1,"display_name":"Controlled Prospect","team":"SEA","position":"C","birth_date":"2005-01-01","source_url":"https://www.nhl.com/kraken","relationship":"organization_rights"},{"player_id":2,"display_name":"Camp Invite","team":"SEA","position":"D","birth_date":null,"source_url":"https://www.nhl.com/kraken","relationship":"free_agent_invite"},{"player_id":3,"display_name":"Legacy Prospect","team":"NYR","position":"G","birth_date":"2004-01-01","source_url":"https://www.nhl.com/rangers"}]}"#,
        )
        .unwrap();

        let audit = overlay.audit().unwrap();
        assert_eq!(audit.candidates, 3);
        assert_eq!(audit.ranking_eligible, 2);
        assert_eq!(audit.camp_only, 1);
        assert_eq!(audit.legacy_relationships, 1);
        assert!(!audit.fully_classified);
        assert_eq!(audit.teams.len(), 2);
        assert_eq!(audit.teams[1].team, "SEA");
        assert_eq!(audit.teams[1].camp_only, 1);
    }
}
