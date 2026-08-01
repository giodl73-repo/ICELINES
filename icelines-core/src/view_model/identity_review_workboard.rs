//! UI-neutral queue of unresolved provider identity proposals.

use crate::source_facts::SourceEvidence;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const IDENTITY_REVIEW_WORKBOARD_SCHEMA: &str = "identity_review_workboard.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityReviewContextInput {
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<IdentityReviewDraftCoordinates>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IdentityReviewDraftCoordinates {
    pub year: u16,
    pub round: u8,
    pub overall: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityReviewProposalInput {
    pub proposal_id: String,
    pub displayed_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_player_id: Option<u32>,
    pub providers: Vec<String>,
    pub evidence_urls: Vec<String>,
    pub evidence: Vec<SourceEvidence>,
    pub contexts: Vec<IdentityReviewContextInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityReviewWorkboardInput {
    pub evaluation_season: u32,
    pub source_package_id: String,
    pub source_package_fingerprint: String,
    pub effective_cutoff: String,
    pub knowledge_cutoff: String,
    pub proposals: Vec<IdentityReviewProposalInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityReviewFamilyCount {
    pub family: String,
    pub proposals: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityReviewWorkboardRow {
    pub rank: usize,
    pub proposal_id: String,
    pub displayed_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_player_id: Option<u32>,
    pub search_query: String,
    pub providers: Vec<String>,
    pub evidence_urls: Vec<String>,
    pub evidence: Vec<SourceEvidence>,
    pub contexts: Vec<IdentityReviewContextInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityReviewWorkboardView {
    pub schema: String,
    pub evaluation_season: u32,
    pub source_package_id: String,
    pub source_package_fingerprint: String,
    pub effective_cutoff: String,
    pub knowledge_cutoff: String,
    pub unresolved_count: usize,
    pub family_counts: Vec<IdentityReviewFamilyCount>,
    pub rows: Vec<IdentityReviewWorkboardRow>,
    pub disclosures: Vec<String>,
}

pub fn build_identity_review_workboard(
    input: IdentityReviewWorkboardInput,
) -> Result<IdentityReviewWorkboardView, String> {
    if input.evaluation_season == 0
        || input.source_package_id.trim().is_empty()
        || input.source_package_fingerprint.trim().is_empty()
        || input.effective_cutoff.trim().is_empty()
        || input.knowledge_cutoff.trim().is_empty()
    {
        return Err("identity review workboard metadata must be complete".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(input.proposals.len());
    let mut family_counts = BTreeMap::<String, usize>::new();
    for mut proposal in input.proposals {
        if proposal.proposal_id.trim().is_empty()
            || proposal.displayed_name.trim().is_empty()
            || proposal.providers.is_empty()
            || proposal.evidence_urls.is_empty()
            || proposal.evidence.is_empty()
            || proposal.contexts.is_empty()
            || !ids.insert(proposal.proposal_id.clone())
        {
            return Err("identity review proposals require unique IDs, identity labels, evidence, providers, and context".to_owned());
        }
        proposal.providers.sort();
        proposal.providers.dedup();
        proposal.evidence_urls.sort();
        proposal.evidence_urls.dedup();
        proposal.contexts.sort_by(|left, right| {
            (&left.family, &left.organization, &left.draft, &left.detail).cmp(&(
                &right.family,
                &right.organization,
                &right.draft,
                &right.detail,
            ))
        });
        proposal.contexts.dedup();
        for family in proposal
            .contexts
            .iter()
            .map(|context| context.family.clone())
            .collect::<BTreeSet<_>>()
        {
            *family_counts.entry(family).or_default() += 1;
        }
        rows.push(IdentityReviewWorkboardRow {
            rank: 0,
            search_query: proposal.displayed_name.clone(),
            proposal_id: proposal.proposal_id,
            displayed_name: proposal.displayed_name,
            birth_date: proposal.birth_date,
            proposed_player_id: proposal.proposed_player_id,
            providers: proposal.providers,
            evidence_urls: proposal.evidence_urls,
            evidence: proposal.evidence,
            contexts: proposal.contexts,
        });
    }
    rows.sort_by(|left, right| {
        let left_context = &left.contexts[0];
        let right_context = &right.contexts[0];
        (
            &left_context.family,
            &left_context.organization,
            &left.displayed_name,
            &left.proposal_id,
        )
            .cmp(&(
                &right_context.family,
                &right_context.organization,
                &right.displayed_name,
                &right.proposal_id,
            ))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    Ok(IdentityReviewWorkboardView {
        schema: IDENTITY_REVIEW_WORKBOARD_SCHEMA.to_owned(),
        evaluation_season: input.evaluation_season,
        source_package_id: input.source_package_id,
        source_package_fingerprint: input.source_package_fingerprint,
        effective_cutoff: input.effective_cutoff,
        knowledge_cutoff: input.knowledge_cutoff,
        unresolved_count: rows.len(),
        family_counts: family_counts
            .into_iter()
            .map(|(family, proposals)| IdentityReviewFamilyCount { family, proposals })
            .collect(),
        rows,
        disclosures: vec![
            "Rows are unresolved identity proposals, not canonical players.".to_owned(),
            "Rank is deterministic queue order and does not imply match confidence.".to_owned(),
            "Only a finalized evidence-backed identity_review_ledger.v1 may resolve a row."
                .to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_and_counts_unresolved_proposals_deterministically() {
        let view = build_identity_review_workboard(IdentityReviewWorkboardInput {
            evaluation_season: 20_262_027,
            source_package_id: "sources".to_owned(),
            source_package_fingerprint: "f".repeat(64),
            effective_cutoff: "2026-07-31T12:00:00Z".to_owned(),
            knowledge_cutoff: "2026-07-31T12:00:00Z".to_owned(),
            proposals: vec![IdentityReviewProposalInput {
                proposal_id: "draft-5".to_owned(),
                displayed_name: "Example Player".to_owned(),
                birth_date: None,
                proposed_player_id: None,
                providers: vec!["official_nhl".to_owned()],
                evidence_urls: vec!["https://example.test/draft".to_owned()],
                evidence: vec![SourceEvidence::new(
                    crate::source_facts::SourceId::try_new("draft").unwrap(),
                    crate::source_facts::SourceUrl::try_new("https://example.test/draft").unwrap(),
                    crate::source_facts::ProviderId::try_new("official_nhl").unwrap(),
                    chrono::Utc::now(),
                    crate::source_facts::ContentHash::try_new("a".repeat(64)).unwrap(),
                    crate::source_facts::AdapterVersion::try_new("v1").unwrap(),
                )],
                contexts: vec![IdentityReviewContextInput {
                    family: "draft".to_owned(),
                    organization: Some("NYR".to_owned()),
                    draft: Some(IdentityReviewDraftCoordinates {
                        year: 2026,
                        round: 1,
                        overall: 5,
                    }),
                    detail: "2026 round 1 overall 5".to_owned(),
                }],
            }],
        })
        .unwrap();
        assert_eq!(view.unresolved_count, 1);
        assert_eq!(view.family_counts[0].family, "draft");
        assert_eq!(view.rows[0].rank, 1);
    }
}
