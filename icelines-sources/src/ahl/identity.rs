use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA: &str = "ahl_canonical_identity_catalog.v1";

#[derive(Debug, Error)]
pub enum AhlIdentityError {
    #[error("AHL identity schema changed: {0}")]
    Schema(String),
    #[error("invalid AHL identity data: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlCanonicalIdentityCatalog {
    pub schema: String,
    pub checked_at: String,
    pub candidates: Vec<AhlCanonicalIdentityCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlCanonicalIdentityCandidate {
    pub nhl_player_id: u32,
    pub display_name: String,
    #[serde(default)]
    pub birth_date: Option<String>,
    pub evidence_urls: Vec<String>,
}

pub fn normalize_ahl_identity_name(name: &str) -> String {
    let normalized = icelines_core::normalize_name(name);
    let mut identity = String::with_capacity(normalized.len());
    let mut pending_boundary = false;
    for character in normalized.chars() {
        if character.is_alphanumeric() {
            if pending_boundary && !identity.is_empty() {
                identity.push(' ');
            }
            identity.push(character);
            pending_boundary = false;
        } else if character.is_whitespace() || character == '-' {
            pending_boundary = true;
        }
    }
    identity
}

pub fn normalized_ahl_identity_surname(name: &str) -> Option<String> {
    normalize_ahl_identity_name(name)
        .split_whitespace()
        .last()
        .map(str::to_owned)
}

pub fn ahl_identity_search_name_variants(name: &str) -> Vec<String> {
    let straight_apostrophes = name
        .chars()
        .map(|character| match character {
            '‘' | '’' => '\'',
            _ => character,
        })
        .collect::<String>();
    if straight_apostrophes == name {
        vec![name.to_owned()]
    } else {
        vec![name.to_owned(), straight_apostrophes]
    }
}

pub fn parse_official_nhl_search_candidates(
    expected_name: &str,
    source_url: &str,
    bytes: &[u8],
) -> Result<Vec<AhlCanonicalIdentityCandidate>, AhlIdentityError> {
    let expected = normalize_ahl_identity_name(expected_name);
    parse_search_candidates_matching(expected_name, source_url, bytes, |name| {
        normalize_ahl_identity_name(name) == expected
    })
}

pub fn parse_official_nhl_search_candidates_by_surname(
    expected_name: &str,
    source_url: &str,
    bytes: &[u8],
) -> Result<Vec<AhlCanonicalIdentityCandidate>, AhlIdentityError> {
    let expected_surname = normalized_ahl_identity_surname(expected_name).ok_or_else(|| {
        AhlIdentityError::Validation(format!("cannot derive surname from `{expected_name}`"))
    })?;
    parse_search_candidates_matching(expected_name, source_url, bytes, |name| {
        normalized_ahl_identity_surname(name).as_deref() == Some(expected_surname.as_str())
    })
}

/// Discover candidates for a draft-coordinate review. Exact normalized names
/// remain preferred. When the provider uses a nickname, parenthetical name,
/// or alternate transliteration, retain same-surname results so the caller can
/// require an immutable draft-coordinate match from each official landing.
///
/// This function is discovery-only: surname agreement must never establish a
/// canonical identity without the separate coordinate gate.
pub fn parse_official_nhl_draft_search_candidates(
    expected_name: &str,
    source_url: &str,
    bytes: &[u8],
) -> Result<Vec<AhlCanonicalIdentityCandidate>, AhlIdentityError> {
    let exact = parse_official_nhl_search_candidates(expected_name, source_url, bytes)?;
    if exact.is_empty() {
        parse_official_nhl_search_candidates_by_surname(expected_name, source_url, bytes)
    } else {
        Ok(exact)
    }
}

fn parse_search_candidates_matching(
    expected_name: &str,
    source_url: &str,
    bytes: &[u8],
    matches: impl Fn(&str) -> bool,
) -> Result<Vec<AhlCanonicalIdentityCandidate>, AhlIdentityError> {
    let rows: Vec<serde_json::Value> = serde_json::from_slice(bytes).map_err(|error| {
        AhlIdentityError::Schema(format!("invalid NHL player-search JSON: {error}"))
    })?;
    let mut candidates = Vec::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        let Some(name) = row.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if !matches(name) {
            continue;
        }
        let player_id = row
            .get("playerId")
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            })
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value != 0)
            .ok_or_else(|| {
                AhlIdentityError::Schema(format!(
                    "matching NHL player-search result for `{expected_name}` has no valid playerId"
                ))
            })?;
        if ids.insert(player_id) {
            candidates.push(AhlCanonicalIdentityCandidate {
                nhl_player_id: player_id,
                display_name: name.to_owned(),
                birth_date: None,
                evidence_urls: vec![source_url.to_owned()],
            });
        }
    }
    candidates.sort_by_key(|candidate| candidate.nhl_player_id);
    Ok(candidates)
}

pub fn enrich_official_nhl_landing_candidate(
    candidate: &AhlCanonicalIdentityCandidate,
    source_url: &str,
    bytes: &[u8],
) -> Result<AhlCanonicalIdentityCandidate, AhlIdentityError> {
    let row: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| AhlIdentityError::Schema(format!("invalid NHL landing JSON: {error}")))?;
    let player_id = row
        .get("playerId")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let first_name = localized_default(row.get("firstName"));
    let last_name = localized_default(row.get("lastName"));
    let display_name = format!("{first_name} {last_name}").trim().to_owned();
    if player_id != Some(candidate.nhl_player_id)
        || display_name.is_empty()
        || normalize_ahl_identity_name(&display_name)
            != normalize_ahl_identity_name(&candidate.display_name)
    {
        return Err(AhlIdentityError::Validation(format!(
            "NHL landing identity conflicts with search proposal {} ({})",
            candidate.nhl_player_id, candidate.display_name
        )));
    }
    let birth_date = row
        .get("birthDate")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if birth_date
        .as_deref()
        .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err())
    {
        return Err(AhlIdentityError::Schema(format!(
            "NHL landing identity {} has an invalid birthDate",
            candidate.nhl_player_id
        )));
    }
    let mut evidence_urls = candidate.evidence_urls.clone();
    evidence_urls.push(source_url.to_owned());
    evidence_urls.sort();
    evidence_urls.dedup();
    Ok(AhlCanonicalIdentityCandidate {
        nhl_player_id: candidate.nhl_player_id,
        display_name,
        birth_date,
        evidence_urls,
    })
}

pub fn merge_ahl_canonical_identity_catalogs(
    checked_at: impl Into<String>,
    catalogs: &[AhlCanonicalIdentityCatalog],
) -> Result<AhlCanonicalIdentityCatalog, AhlIdentityError> {
    let checked_at = checked_at.into();
    let mut merged = BTreeMap::<u32, AhlCanonicalIdentityCandidate>::new();
    let mut evidence_by_player = BTreeMap::<u32, BTreeSet<String>>::new();
    for catalog in catalogs {
        validate_catalog_authority(catalog)?;
        for candidate in &catalog.candidates {
            validate_candidate(candidate)?;
            evidence_by_player
                .entry(candidate.nhl_player_id)
                .or_default()
                .extend(candidate.evidence_urls.iter().cloned());
            match merged.entry(candidate.nhl_player_id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(candidate.clone());
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    let current = entry.get_mut();
                    if normalize_ahl_identity_name(&current.display_name)
                        != normalize_ahl_identity_name(&candidate.display_name)
                        || matches!(
                            (&current.birth_date, &candidate.birth_date),
                            (Some(left), Some(right)) if left != right
                        )
                    {
                        return Err(AhlIdentityError::Validation(format!(
                            "canonical NHL identity sources conflict for player {}",
                            candidate.nhl_player_id
                        )));
                    }
                    if current.birth_date.is_none() {
                        current.birth_date.clone_from(&candidate.birth_date);
                    }
                }
            }
        }
    }
    for (player_id, candidate) in &mut merged {
        candidate.evidence_urls = evidence_by_player
            .remove(player_id)
            .unwrap_or_default()
            .into_iter()
            .collect();
    }
    let catalog = AhlCanonicalIdentityCatalog {
        schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
        checked_at,
        candidates: merged.into_values().collect(),
    };
    validate_catalog(&catalog)?;
    Ok(catalog)
}

fn localized_default(value: Option<&serde_json::Value>) -> &str {
    value
        .and_then(|value| {
            value
                .as_str()
                .or_else(|| value.get("default").and_then(serde_json::Value::as_str))
        })
        .unwrap_or("")
}

fn validate_catalog(catalog: &AhlCanonicalIdentityCatalog) -> Result<(), AhlIdentityError> {
    validate_catalog_authority(catalog)?;
    let mut ids = BTreeSet::new();
    for candidate in &catalog.candidates {
        validate_candidate(candidate)?;
        if !ids.insert(candidate.nhl_player_id) {
            return Err(AhlIdentityError::Validation(
                "canonical NHL identity catalog contains invalid or duplicate candidates"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_catalog_authority(
    catalog: &AhlCanonicalIdentityCatalog,
) -> Result<(), AhlIdentityError> {
    if catalog.schema != AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA
        || catalog.checked_at.trim().is_empty()
    {
        return Err(AhlIdentityError::Validation(
            "invalid canonical NHL identity catalog authority".to_owned(),
        ));
    }
    Ok(())
}

fn validate_candidate(candidate: &AhlCanonicalIdentityCandidate) -> Result<(), AhlIdentityError> {
    if candidate.nhl_player_id == 0
        || normalize_ahl_identity_name(&candidate.display_name).is_empty()
        || candidate.evidence_urls.is_empty()
        || candidate
            .evidence_urls
            .iter()
            .any(|url| !absolute_http_url(url))
        || candidate
            .birth_date
            .as_deref()
            .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err())
    {
        return Err(AhlIdentityError::Validation(
            "canonical NHL identity catalog contains invalid or duplicate candidates".to_owned(),
        ));
    }
    Ok(())
}

fn absolute_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use super::{
        enrich_official_nhl_landing_candidate, parse_official_nhl_draft_search_candidates,
        parse_official_nhl_search_candidates,
    };

    #[test]
    fn search_and_landing_keep_provider_identity_separate_until_corroborated() {
        let candidates = parse_official_nhl_search_candidates(
            "Cole Beaudoin",
            "https://search.d3.nhle.com/example",
            br#"[{"name":"Cole Beaudoin","playerId":"8484786"}]"#,
        )
        .expect("search candidates");
        assert_eq!(candidates[0].birth_date, None);
        let enriched = enrich_official_nhl_landing_candidate(
            &candidates[0],
            "https://api-web.nhle.com/v1/player/8484786/landing",
            br#"{"playerId":8484786,"firstName":{"default":"Cole"},"lastName":{"default":"Beaudoin"},"birthDate":"2006-04-24"}"#,
        )
        .expect("corroborated landing");
        assert_eq!(enriched.birth_date.as_deref(), Some("2006-04-24"));
        assert_eq!(enriched.evidence_urls.len(), 2);
    }

    #[test]
    fn draft_discovery_falls_back_to_surname_without_approving_identity() {
        let candidates = parse_official_nhl_draft_search_candidates(
            "Jeffrey (JP) Hurlbert",
            "https://search.d3.nhle.com/example",
            br#"[{"name":"JP Hurlbert","playerId":"8486001"},{"name":"Alex Hurlbert","playerId":"8486002"},{"name":"JP Different","playerId":"8486003"}]"#,
        )
        .expect("surname discoveries");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].nhl_player_id, 8_486_001);
        assert_eq!(candidates[1].nhl_player_id, 8_486_002);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.birth_date.is_none()));
    }

    #[test]
    fn draft_discovery_prefers_exact_name_over_other_surname_results() {
        let candidates = parse_official_nhl_draft_search_candidates(
            "Sam Poulin",
            "https://search.d3.nhle.com/example",
            br#"[{"name":"Sam Poulin","playerId":"8481001"},{"name":"Samuel Poulin","playerId":"8481002"}]"#,
        )
        .expect("exact discovery");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].nhl_player_id, 8_481_001);
    }
}
