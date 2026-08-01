//! UI-neutral results of official identity evidence acquisition.

use crate::source_facts::SourceEvidence;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const OFFICIAL_IDENTITY_CANDIDATE_BOARD_SCHEMA: &str = "official_identity_candidate_board.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialIdentityDraftCoordinates {
    pub organization: String,
    pub year: u16,
    pub round: u8,
    pub overall: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficialIdentityCandidateStatus {
    ExactCoordinateMatch,
    AmbiguousCoordinateMatch,
    NoExactName,
    LandingMissing,
    CoordinateMismatch,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialIdentityCandidateView {
    pub player_id: u32,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<OfficialIdentityDraftCoordinates>,
    pub search_evidence: SourceEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing_evidence: Option<SourceEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialIdentityCandidateRow {
    pub rank: usize,
    pub proposal_id: String,
    pub displayed_name: String,
    pub search_query: String,
    pub expected_draft: OfficialIdentityDraftCoordinates,
    pub proposal_evidence: Vec<SourceEvidence>,
    pub status: OfficialIdentityCandidateStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eligible_player_id: Option<u32>,
    pub candidates: Vec<OfficialIdentityCandidateView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialIdentityCandidateStatusCount {
    pub status: OfficialIdentityCandidateStatus,
    pub proposals: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialIdentityCandidateBoardView {
    pub schema: String,
    pub evaluation_season: u32,
    pub source_package_id: String,
    pub source_package_fingerprint: String,
    pub effective_cutoff: String,
    /// Knowledge boundary of the source package that produced the workboard.
    pub knowledge_cutoff: String,
    /// Latest capture admitted to this evidence artifact. This can be newer
    /// than the base package and therefore applies only to a subsequent seal.
    pub evidence_cutoff: String,
    pub evaluated_count: usize,
    pub eligible_count: usize,
    pub status_counts: Vec<OfficialIdentityCandidateStatusCount>,
    pub rows: Vec<OfficialIdentityCandidateRow>,
    pub disclosures: Vec<String>,
}

pub fn build_official_identity_candidate_board(
    evaluation_season: u32,
    source_package_id: String,
    source_package_fingerprint: String,
    effective_cutoff: String,
    knowledge_cutoff: String,
    evidence_cutoff: String,
    mut rows: Vec<OfficialIdentityCandidateRow>,
) -> Result<OfficialIdentityCandidateBoardView, String> {
    if evaluation_season == 0
        || source_package_id.trim().is_empty()
        || source_package_fingerprint.len() != 64
        || !source_package_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || effective_cutoff.trim().is_empty()
        || knowledge_cutoff.trim().is_empty()
        || evidence_cutoff.trim().is_empty()
    {
        return Err("official identity candidate board metadata is incomplete".to_owned());
    }
    let evidence_cutoff = DateTime::parse_from_rfc3339(&evidence_cutoff)
        .map_err(|_| "official identity evidence cutoff must be RFC 3339".to_owned())?;
    rows.sort_by_key(|row| row.rank);
    let mut prior_rank = 0;
    let mut status_counts = BTreeMap::new();
    for row in &rows {
        if row.rank == 0
            || row.rank <= prior_rank
            || row.proposal_id.trim().is_empty()
            || row.proposal_evidence.is_empty()
        {
            return Err(
                "official identity candidate rows require unique increasing ranks and proposal evidence"
                    .to_owned(),
            );
        }
        let evidence_is_future = row
            .proposal_evidence
            .iter()
            .chain(row.candidates.iter().flat_map(|candidate| {
                std::iter::once(&candidate.search_evidence).chain(candidate.landing_evidence.iter())
            }))
            .any(|evidence| evidence.captured_at() > evidence_cutoff);
        if evidence_is_future {
            return Err(format!(
                "proposal {} contains evidence after the artifact cutoff",
                row.proposal_id
            ));
        }
        prior_rank = row.rank;
        let exact = row
            .candidates
            .iter()
            .filter(|candidate| candidate.draft.as_ref() == Some(&row.expected_draft))
            .collect::<Vec<_>>();
        let valid_eligible = row.status == OfficialIdentityCandidateStatus::ExactCoordinateMatch
            && exact.len() == 1
            && row.eligible_player_id == Some(exact[0].player_id);
        if (row.eligible_player_id.is_some()
            || row.status == OfficialIdentityCandidateStatus::ExactCoordinateMatch)
            && !valid_eligible
        {
            return Err(format!(
                "proposal {} is eligible only after one exact draft-coordinate match",
                row.proposal_id
            ));
        }
        *status_counts.entry(row.status).or_insert(0usize) += 1;
    }
    let eligible_count = rows
        .iter()
        .filter(|row| row.eligible_player_id.is_some())
        .count();
    Ok(OfficialIdentityCandidateBoardView {
        schema: OFFICIAL_IDENTITY_CANDIDATE_BOARD_SCHEMA.to_owned(),
        evaluation_season,
        source_package_id,
        source_package_fingerprint,
        effective_cutoff,
        knowledge_cutoff,
        evidence_cutoff: evidence_cutoff.to_rfc3339(),
        evaluated_count: rows.len(),
        eligible_count,
        status_counts: status_counts
            .into_iter()
            .map(|(status, proposals)| OfficialIdentityCandidateStatusCount { status, proposals })
            .collect(),
        rows,
        disclosures: vec![
            "Candidates come only from exact normalized-name results in the official NHL player search.".to_owned(),
            "Eligibility requires exactly one official player landing with matching draft organization, year, round, and overall pick.".to_owned(),
            "This artifact records evidence candidates; it does not mutate canonical identity or source-package authority.".to_owned(),
            "Evidence newer than the base package knowledge cutoff is eligible only for a subsequent source-package seal.".to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_facts::{AdapterVersion, ContentHash, ProviderId, SourceId, SourceUrl};
    use chrono::{TimeZone, Utc};

    fn evidence(url: &str, hash: char) -> SourceEvidence {
        SourceEvidence::new(
            SourceId::try_new("official-test").unwrap(),
            SourceUrl::try_new(url).unwrap(),
            ProviderId::try_new("official_nhl_api").unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap(),
            ContentHash::try_new(hash.to_string().repeat(64)).unwrap(),
            AdapterVersion::try_new("v1").unwrap(),
        )
    }

    #[test]
    fn rejects_eligibility_without_one_exact_coordinate_match() {
        let coordinates = OfficialIdentityDraftCoordinates {
            organization: "NYR".to_owned(),
            year: 2026,
            round: 1,
            overall: 5,
        };
        let error = build_official_identity_candidate_board(
            20_262_027,
            "package".to_owned(),
            "a".repeat(64),
            "2026-07-31T00:00:00Z".to_owned(),
            "2026-07-31T00:00:00Z".to_owned(),
            "2026-07-31T00:00:00Z".to_owned(),
            vec![OfficialIdentityCandidateRow {
                rank: 1,
                proposal_id: "proposal".to_owned(),
                displayed_name: "Player".to_owned(),
                search_query: "Player".to_owned(),
                expected_draft: coordinates.clone(),
                proposal_evidence: vec![evidence("https://example.test/draft", 'd')],
                status: OfficialIdentityCandidateStatus::ExactCoordinateMatch,
                eligible_player_id: Some(1),
                candidates: vec![OfficialIdentityCandidateView {
                    player_id: 1,
                    display_name: "Player".to_owned(),
                    birth_date: None,
                    draft: None,
                    search_evidence: evidence("https://example.test/search", 'b'),
                    landing_evidence: Some(evidence("https://example.test/landing", 'c')),
                }],
                errors: vec![],
            }],
        )
        .unwrap_err();
        assert!(error.contains("exact draft-coordinate"));
    }
}
