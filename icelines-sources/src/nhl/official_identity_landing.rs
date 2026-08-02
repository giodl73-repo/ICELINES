use chrono::{DateTime, Utc};
use icelines_core::source_facts::{
    AdapterVersion, ContentHash, PlayerOrganizationEvent, ProviderId, SourceEvidence, SourceFact,
    SourceId, SourceUrl,
};
use icelines_core::OfficialIdentityDraftCoordinates;
use sha2::{Digest, Sha256};

use crate::adapter::{SourceAdapter, SourceInput};
use crate::nhl::player_landing::OfficialNhlDraftAdapter;

#[derive(Debug, Clone)]
pub struct OfficialIdentityLandingRecord {
    pub display_name: String,
    pub birth_date: Option<String>,
    pub draft: Option<OfficialIdentityDraftCoordinates>,
    pub evidence: SourceEvidence,
}

pub fn parse_official_identity_landing(
    player_id: u32,
    bytes: &[u8],
    captured_at: DateTime<Utc>,
    source_url: &str,
) -> Result<OfficialIdentityLandingRecord, String> {
    let content_hash = content_hash(bytes)?;
    let adapter =
        OfficialNhlDraftAdapter::new(player_id, captured_at).map_err(|error| error.to_string())?;
    let descriptor = adapter.descriptor();
    let assertion = adapter
        .parse(SourceInput::new(
            bytes,
            descriptor.source_id.clone(),
            content_hash.clone(),
        ))
        .map_err(|error| error.to_string())?;
    let draft = assertion.and_then(|assertion| match assertion.fact() {
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
            by,
            year,
            round,
            overall,
        }) => Some(OfficialIdentityDraftCoordinates {
            organization: by.as_str().to_owned(),
            year: *year,
            round: *round,
            overall: *overall,
        }),
        _ => None,
    });
    let raw: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let first = localized_default(raw.get("firstName"));
    let last = localized_default(raw.get("lastName"));
    let display_name = format!("{first} {last}").trim().to_owned();
    if display_name.is_empty() {
        return Err("landing has no display name".to_owned());
    }
    let evidence = SourceEvidence::new(
        SourceId::try_new(format!("nhl-player-landing:{player_id}"))
            .map_err(|error| error.to_string())?,
        SourceUrl::try_new(source_url).map_err(|error| error.to_string())?,
        ProviderId::try_new("official_nhl_api").map_err(|error| error.to_string())?,
        captured_at,
        content_hash,
        AdapterVersion::try_new("v1").map_err(|error| error.to_string())?,
    );
    Ok(OfficialIdentityLandingRecord {
        display_name,
        birth_date: raw
            .get("birthDate")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        draft,
        evidence,
    })
}

fn localized_default(value: Option<&serde_json::Value>) -> &str {
    value
        .and_then(|value| {
            value
                .get("default")
                .and_then(serde_json::Value::as_str)
                .or_else(|| value.as_str())
        })
        .unwrap_or("")
}

fn content_hash(bytes: &[u8]) -> Result<ContentHash, String> {
    ContentHash::try_new(format!("{:x}", Sha256::digest(bytes))).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_official_identity_landing;
    use chrono::{TimeZone, Utc};

    #[test]
    fn parses_identity_birth_date_draft_and_evidence() {
        let bytes = br#"{
            "playerId":8484786,
            "firstName":{"default":"Cole"},
            "lastName":{"default":"Beaudoin"},
            "birthDate":"2006-04-24",
            "draftDetails":{"year":2024,"teamAbbrev":"UTA","round":1,"pickInRound":24,"overallPick":24}
        }"#;
        let captured_at = Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap();
        let record = parse_official_identity_landing(
            8_484_786,
            bytes,
            captured_at,
            "https://api-web.nhle.com/v1/player/8484786/landing",
        )
        .expect("valid landing");
        assert_eq!(record.display_name, "Cole Beaudoin");
        assert_eq!(record.birth_date.as_deref(), Some("2006-04-24"));
        assert_eq!(record.draft.as_ref().map(|draft| draft.overall), Some(24));
        assert_eq!(record.evidence.provider().as_str(), "official_nhl_api");
    }

    #[test]
    fn rejects_missing_display_name() {
        let captured_at = Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap();
        assert!(parse_official_identity_landing(
            1,
            br#"{"birthDate":"2000-01-01"}"#,
            captured_at,
            "https://api-web.nhle.com/v1/player/1/landing",
        )
        .is_err());
    }
}
