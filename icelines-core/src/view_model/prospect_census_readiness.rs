//! Compact, UI-neutral publication-readiness projection of a prospect census.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    ProspectCensusAuthorityGap, ProspectCensusAuthorityGapSummary, ProspectCensusCounts,
    ProspectCensusLossReason, ProspectCensusPublicationStatus, ProspectCensusView,
    ProspectPopulationAuthorityStatus, CANONICAL_TEAMS, PROSPECT_CENSUS_SCHEMA,
};

pub const PROSPECT_CENSUS_READINESS_SCHEMA: &str = "prospect_census_readiness_board.v1";
pub const PROSPECT_CENSUS_READINESS_METHOD: &str = "prospect_census_readiness_board.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusLossSummaryView {
    pub reason: ProspectCensusLossReason,
    pub players: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusReadinessTeamView {
    pub organization: String,
    pub population_authority_status: ProspectPopulationAuthorityStatus,
    pub publication_status: ProspectCensusPublicationStatus,
    pub requested_ranking_depth: usize,
    pub ranking_depth_shortfall: usize,
    pub counts: ProspectCensusCounts,
    #[serde(default)]
    pub authority_gaps: Vec<ProspectCensusAuthorityGap>,
    #[serde(default)]
    pub loss_summary: Vec<ProspectCensusLossSummaryView>,
    #[serde(default)]
    pub remediation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusReadinessBoardView {
    pub schema: String,
    pub method_version: String,
    pub evaluation_season: u32,
    pub effective_cutoff: String,
    pub knowledge_cutoff: String,
    pub source_package_fingerprint: String,
    pub source_census_fingerprint: String,
    pub organizations: usize,
    pub population_complete_organizations: usize,
    pub depth_complete_organizations: usize,
    pub published_organizations: usize,
    pub league_counts: ProspectCensusCounts,
    #[serde(default)]
    pub authority_gap_summary: Vec<ProspectCensusAuthorityGapSummary>,
    #[serde(default)]
    pub loss_summary: Vec<ProspectCensusLossSummaryView>,
    pub teams: Vec<ProspectCensusReadinessTeamView>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl ProspectCensusReadinessBoardView {
    pub fn team(&self, organization: &str) -> Option<&ProspectCensusReadinessTeamView> {
        self.teams
            .iter()
            .find(|row| row.organization == organization)
    }

    pub fn calculate_fingerprint(&self) -> Result<String, ProspectCensusReadinessError> {
        let mut canonical = self.clone();
        canonical.fingerprint.clear();
        canonical
            .teams
            .sort_by(|left, right| left.organization.cmp(&right.organization));
        canonical.authority_gap_summary.sort_by(|left, right| {
            left.source_family
                .cmp(&right.source_family)
                .then_with(|| left.state.cmp(&right.state))
        });
        canonical.loss_summary.sort_by_key(|row| row.reason);
        for team in &mut canonical.teams {
            team.authority_gaps.sort_by(|left, right| {
                left.source_family
                    .cmp(&right.source_family)
                    .then_with(|| left.state.cmp(&right.state))
            });
            team.loss_summary.sort_by_key(|row| row.reason);
            team.remediation.sort();
        }
        hash_json(&canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProspectCensusReadinessError {
    #[error("unsupported prospect census schema: {0}")]
    UnsupportedSchema(String),
    #[error("prospect census does not contain the canonical 32 organizations")]
    InvalidLeagueEnvelope,
    #[error("prospect census readiness totals do not reconcile")]
    TotalsDoNotReconcile,
    #[error("prospect census readiness JSON failed: {0}")]
    InvalidJson(String),
}

pub fn build_prospect_census_readiness(
    census: &ProspectCensusView,
) -> Result<ProspectCensusReadinessBoardView, ProspectCensusReadinessError> {
    if census.schema != PROSPECT_CENSUS_SCHEMA {
        return Err(ProspectCensusReadinessError::UnsupportedSchema(
            census.schema.clone(),
        ));
    }
    let expected = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| *team)
        .collect::<BTreeSet<_>>();
    let actual = census
        .organizations
        .iter()
        .map(|team| team.organization.as_str())
        .collect::<BTreeSet<_>>();
    if census.organizations.len() != CANONICAL_TEAMS.len() || actual != expected {
        return Err(ProspectCensusReadinessError::InvalidLeagueEnvelope);
    }

    let summed_counts =
        census
            .organizations
            .iter()
            .fold(ProspectCensusCounts::default(), |mut total, team| {
                add_counts(&mut total, &team.counts);
                total
            });
    let gap_summary = summarize_gaps(census);
    if summed_counts != census.league_counts
        || gap_summary != census.authority_gap_summary
        || census.losses.len()
            != census
                .organizations
                .iter()
                .map(|team| team.losses.len())
                .sum::<usize>()
    {
        return Err(ProspectCensusReadinessError::TotalsDoNotReconcile);
    }

    let source_census_fingerprint = hash_json(census)?;
    let population_complete_organizations = census
        .organizations
        .iter()
        .filter(|team| {
            team.population_authority_status == ProspectPopulationAuthorityStatus::Complete
        })
        .count();
    let depth_complete_organizations = census
        .organizations
        .iter()
        .filter(|team| team.ranking_depth_complete)
        .count();
    let published_organizations = census
        .organizations
        .iter()
        .filter(|team| team.publication_status == ProspectCensusPublicationStatus::Published)
        .count();
    let mut teams = census
        .organizations
        .iter()
        .map(|team| ProspectCensusReadinessTeamView {
            organization: team.organization.clone(),
            population_authority_status: team.population_authority_status,
            publication_status: team.publication_status,
            requested_ranking_depth: team.requested_ranking_depth,
            ranking_depth_shortfall: team.ranking_depth_shortfall,
            counts: team.counts.clone(),
            authority_gaps: team.authority_gaps.clone(),
            loss_summary: summarize_losses(team.losses.iter().map(|row| row.reason)),
            remediation: team.remediation.clone(),
        })
        .collect::<Vec<_>>();
    teams.sort_by(|left, right| left.organization.cmp(&right.organization));

    let mut board = ProspectCensusReadinessBoardView {
        schema: PROSPECT_CENSUS_READINESS_SCHEMA.to_owned(),
        method_version: PROSPECT_CENSUS_READINESS_METHOD.to_owned(),
        evaluation_season: census.evaluation_season,
        effective_cutoff: census.effective_cutoff.clone(),
        knowledge_cutoff: census.knowledge_cutoff.clone(),
        source_package_fingerprint: census.source_package_fingerprint.clone(),
        source_census_fingerprint,
        organizations: census.organizations.len(),
        population_complete_organizations,
        depth_complete_organizations,
        published_organizations,
        league_counts: census.league_counts.clone(),
        authority_gap_summary: gap_summary,
        loss_summary: summarize_losses(census.losses.iter().map(|row| row.reason)),
        teams,
        disclosures: vec![
            "Readiness is a compact projection of the sealed prospect census; it does not add, remove, or reclassify player evidence.".to_owned(),
            "Population completeness and requested ranking depth remain independent publication gates.".to_owned(),
            "Authority gaps identify missing source work; they are not permission to infer current organizational control.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    board.fingerprint = board.calculate_fingerprint()?;
    Ok(board)
}

fn summarize_losses(
    reasons: impl Iterator<Item = ProspectCensusLossReason>,
) -> Vec<ProspectCensusLossSummaryView> {
    let mut counts = BTreeMap::new();
    for reason in reasons {
        *counts.entry(reason).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .map(|(reason, players)| ProspectCensusLossSummaryView { reason, players })
        .collect()
}

fn summarize_gaps(census: &ProspectCensusView) -> Vec<ProspectCensusAuthorityGapSummary> {
    let mut counts = BTreeMap::new();
    for team in &census.organizations {
        for gap in &team.authority_gaps {
            *counts
                .entry((gap.source_family.clone(), gap.state))
                .or_insert(0usize) += 1;
        }
    }
    counts
        .into_iter()
        .map(
            |((source_family, state), organizations)| ProspectCensusAuthorityGapSummary {
                source_family,
                state,
                organizations,
            },
        )
        .collect()
}

fn add_counts(total: &mut ProspectCensusCounts, row: &ProspectCensusCounts) {
    total.discovered += row.discovered;
    total.canonical_identity += row.canonical_identity;
    total.controlled_relationship += row.controlled_relationship;
    total.prospect_eligible += row.prospect_eligible;
    total.career_evidence_usable += row.career_evidence_usable;
    total.study_built += row.study_built;
    total.ranked += row.ranked;
}

fn hash_json(value: &impl Serialize) -> Result<String, ProspectCensusReadinessError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ProspectCensusReadinessError::InvalidJson(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ProspectCensusAuthorityGapState, ProspectCensusFreshnessStatus,
        ProspectCensusOrganizationView, ProspectRankPublicationStatus,
        ProspectScorePublicationStatus,
    };

    fn census() -> ProspectCensusView {
        let gap = |source_family: &str| ProspectCensusAuthorityGap {
            source_family: source_family.to_owned(),
            state: ProspectCensusAuthorityGapState::Failed,
            reason: "requested source object was not acquired".to_owned(),
        };
        let organizations = CANONICAL_TEAMS
            .iter()
            .map(|(organization, _)| ProspectCensusOrganizationView {
                organization: (*organization).to_owned(),
                population_authority_status: ProspectPopulationAuthorityStatus::Incomplete,
                ranking_depth_complete: false,
                requested_ranking_depth: 10,
                ranking_depth_shortfall: 10,
                publication_status: ProspectCensusPublicationStatus::PopulationIncomplete,
                score_status: ProspectScorePublicationStatus::ScoreWithheld,
                rank_status: ProspectRankPublicationStatus::RankWithheld,
                counts: ProspectCensusCounts::default(),
                authority_gaps: vec![
                    gap("ahl_current_assignment"),
                    gap("nhl_club_camp_publication"),
                    gap("nhl_contract_publication"),
                ],
                losses: vec![],
                disclosures: vec![],
                remediation: vec!["Acquire missing authority.".to_owned()],
            })
            .collect::<Vec<_>>();
        ProspectCensusView {
            schema: PROSPECT_CENSUS_SCHEMA.to_owned(),
            evaluation_season: 20_262_027,
            effective_cutoff: "2026-07-31T12:00:00Z".to_owned(),
            knowledge_cutoff: "2026-07-31T12:00:00Z".to_owned(),
            freshness_status: ProspectCensusFreshnessStatus::Fresh,
            source_package_fingerprint: "a".repeat(64),
            reconciliation_policy_version: "reconcile.v1".to_owned(),
            eligibility_policy_version: "eligibility.v1".to_owned(),
            organizations,
            league_counts: ProspectCensusCounts::default(),
            authority_gap_summary: vec![
                ProspectCensusAuthorityGapSummary {
                    source_family: "ahl_current_assignment".to_owned(),
                    state: ProspectCensusAuthorityGapState::Failed,
                    organizations: 32,
                },
                ProspectCensusAuthorityGapSummary {
                    source_family: "nhl_club_camp_publication".to_owned(),
                    state: ProspectCensusAuthorityGapState::Failed,
                    organizations: 32,
                },
                ProspectCensusAuthorityGapSummary {
                    source_family: "nhl_contract_publication".to_owned(),
                    state: ProspectCensusAuthorityGapState::Failed,
                    organizations: 32,
                },
            ],
            dimensions: vec![],
            losses: vec![],
            disclosures: vec![],
        }
    }

    #[test]
    fn compact_board_reconciles_all_team_authority_gaps() {
        let board = build_prospect_census_readiness(&census()).unwrap();
        assert_eq!(board.organizations, 32);
        assert_eq!(board.population_complete_organizations, 0);
        assert_eq!(board.published_organizations, 0);
        assert_eq!(board.authority_gap_summary.len(), 3);
        assert!(board
            .authority_gap_summary
            .iter()
            .all(|row| row.organizations == 32));
        assert_eq!(board.team("NYR").unwrap().authority_gaps.len(), 3);
        assert_eq!(board.source_census_fingerprint.len(), 64);
        assert_eq!(board.calculate_fingerprint().unwrap(), board.fingerprint);
    }

    #[test]
    fn compact_board_refuses_partial_league_envelope() {
        let mut census = census();
        census.organizations.pop();
        assert_eq!(
            build_prospect_census_readiness(&census).unwrap_err(),
            ProspectCensusReadinessError::InvalidLeagueEnvelope
        );
    }

    #[test]
    fn compact_board_refuses_unreconciled_league_totals() {
        let mut census = census();
        census.league_counts.discovered = 1;
        assert_eq!(
            build_prospect_census_readiness(&census).unwrap_err(),
            ProspectCensusReadinessError::TotalsDoNotReconcile
        );
    }
}
