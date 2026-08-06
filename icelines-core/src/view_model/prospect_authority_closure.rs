//! Machine-actionable closure plan for prospect census authority gaps.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    ProspectCensusAuthorityGapState, ProspectCensusReadinessBoardView, CANONICAL_TEAMS,
    PROSPECT_CENSUS_READINESS_SCHEMA,
};

pub const PROSPECT_AUTHORITY_CLOSURE_SCHEMA: &str = "prospect_authority_closure_board.v1";
pub const PROSPECT_AUTHORITY_CLOSURE_METHOD: &str = "prospect_authority_closure_board.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectAuthorityClosureGate {
    PopulationAuthority,
    OrganizationalControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectAuthorityClosureDisposition {
    Acquire,
    ResolveQuarantine,
    CompletePagination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectAuthorityClosureFamilySummary {
    pub source_family: String,
    pub cells: usize,
    pub organizations: usize,
    pub gate: ProspectAuthorityClosureGate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_artifact_schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingestion_option: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectAuthorityClosureCell {
    pub organization: String,
    pub source_family: String,
    pub state: ProspectCensusAuthorityGapState,
    pub gate: ProspectAuthorityClosureGate,
    pub disposition: ProspectAuthorityClosureDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_artifact_schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingestion_option: Option<String>,
    pub reason: String,
    pub remediation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectAuthorityClosureBoardView {
    pub schema: String,
    pub method_version: String,
    pub evaluation_season: u32,
    pub effective_cutoff: String,
    pub knowledge_cutoff: String,
    pub source_readiness_fingerprint: String,
    pub organizations: usize,
    pub affected_organizations: usize,
    pub cells: usize,
    pub control_blocking_cells: usize,
    pub population_blocking_cells: usize,
    pub family_summary: Vec<ProspectAuthorityClosureFamilySummary>,
    pub closure_cells: Vec<ProspectAuthorityClosureCell>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl ProspectAuthorityClosureBoardView {
    pub fn cells_for_team(&self, organization: &str) -> Vec<&ProspectAuthorityClosureCell> {
        self.closure_cells
            .iter()
            .filter(|row| row.organization == organization)
            .collect()
    }

    pub fn calculate_fingerprint(&self) -> Result<String, ProspectAuthorityClosureError> {
        let mut canonical = self.clone();
        canonical.fingerprint.clear();
        canonical.family_summary.sort_by(|left, right| {
            left.source_family
                .cmp(&right.source_family)
                .then_with(|| left.gate.cmp(&right.gate))
        });
        canonical.closure_cells.sort_by(|left, right| {
            left.organization
                .cmp(&right.organization)
                .then_with(|| left.source_family.cmp(&right.source_family))
                .then_with(|| left.state.cmp(&right.state))
        });
        hash_json(&canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProspectAuthorityClosureError {
    #[error("unsupported prospect census readiness schema: {0}")]
    UnsupportedSchema(String),
    #[error("prospect census readiness fingerprint does not match its contents")]
    InvalidSourceFingerprint,
    #[error("prospect authority closure requires the canonical 32-team envelope")]
    InvalidLeagueEnvelope,
    #[error("prospect authority closure summaries do not reconcile")]
    TotalsDoNotReconcile,
    #[error("prospect authority closure JSON failed: {0}")]
    InvalidJson(String),
}

pub fn build_prospect_authority_closure(
    readiness: &ProspectCensusReadinessBoardView,
) -> Result<ProspectAuthorityClosureBoardView, ProspectAuthorityClosureError> {
    if readiness.schema != PROSPECT_CENSUS_READINESS_SCHEMA {
        return Err(ProspectAuthorityClosureError::UnsupportedSchema(
            readiness.schema.clone(),
        ));
    }
    if readiness
        .calculate_fingerprint()
        .map_err(|error| ProspectAuthorityClosureError::InvalidJson(error.to_string()))?
        != readiness.fingerprint
    {
        return Err(ProspectAuthorityClosureError::InvalidSourceFingerprint);
    }
    let expected = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| *team)
        .collect::<BTreeSet<_>>();
    let actual = readiness
        .teams
        .iter()
        .map(|team| team.organization.as_str())
        .collect::<BTreeSet<_>>();
    if readiness.organizations != CANONICAL_TEAMS.len()
        || readiness.teams.len() != CANONICAL_TEAMS.len()
        || actual != expected
    {
        return Err(ProspectAuthorityClosureError::InvalidLeagueEnvelope);
    }

    let mut closure_cells = readiness
        .teams
        .iter()
        .flat_map(|team| {
            team.authority_gaps.iter().map(|gap| {
                let boundary = boundary_for(&gap.source_family);
                ProspectAuthorityClosureCell {
                    organization: team.organization.clone(),
                    source_family: gap.source_family.clone(),
                    state: gap.state,
                    gate: boundary.gate,
                    disposition: disposition_for(gap.state),
                    required_artifact_schema: boundary.artifact.map(str::to_owned),
                    ingestion_option: boundary.option.map(str::to_owned),
                    reason: gap.reason.clone(),
                    remediation: remediation_for(
                        &gap.source_family,
                        gap.state,
                        boundary.artifact,
                        boundary.option,
                    ),
                }
            })
        })
        .collect::<Vec<_>>();
    closure_cells.sort_by(|left, right| {
        left.organization
            .cmp(&right.organization)
            .then_with(|| left.source_family.cmp(&right.source_family))
            .then_with(|| left.state.cmp(&right.state))
    });

    let family_summary = summarize_families(&closure_cells);
    let derived_gap_summary = closure_cells.iter().fold(
        BTreeMap::<(String, ProspectCensusAuthorityGapState), usize>::new(),
        |mut counts, row| {
            *counts
                .entry((row.source_family.clone(), row.state))
                .or_default() += 1;
            counts
        },
    );
    let supplied_gap_summary = readiness.authority_gap_summary.iter().fold(
        BTreeMap::<(String, ProspectCensusAuthorityGapState), usize>::new(),
        |mut counts, row| {
            counts.insert((row.source_family.clone(), row.state), row.organizations);
            counts
        },
    );
    let cells = closure_cells.len();
    let affected_organizations = closure_cells
        .iter()
        .map(|row| row.organization.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let control_blocking_cells = closure_cells
        .iter()
        .filter(|row| row.gate == ProspectAuthorityClosureGate::OrganizationalControl)
        .count();
    let population_blocking_cells = cells - control_blocking_cells;
    if derived_gap_summary != supplied_gap_summary
        || family_summary.iter().map(|row| row.cells).sum::<usize>() != cells
        || readiness
            .authority_gap_summary
            .iter()
            .map(|row| row.organizations)
            .sum::<usize>()
            != cells
    {
        return Err(ProspectAuthorityClosureError::TotalsDoNotReconcile);
    }

    let mut board = ProspectAuthorityClosureBoardView {
        schema: PROSPECT_AUTHORITY_CLOSURE_SCHEMA.to_owned(),
        method_version: PROSPECT_AUTHORITY_CLOSURE_METHOD.to_owned(),
        evaluation_season: readiness.evaluation_season,
        effective_cutoff: readiness.effective_cutoff.clone(),
        knowledge_cutoff: readiness.knowledge_cutoff.clone(),
        source_readiness_fingerprint: readiness.fingerprint.clone(),
        organizations: readiness.organizations,
        affected_organizations,
        cells,
        control_blocking_cells,
        population_blocking_cells,
        family_summary,
        closure_cells,
        disclosures: vec![
            "Closure recipes name required authority boundaries; they do not acquire data or approve evidence.".to_owned(),
            "Contract control is independent from roster, affiliate, draft, and camp observations.".to_owned(),
            "Camp participation and AHL assignment can close population cells but cannot establish organizational control.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    board.fingerprint = board.calculate_fingerprint()?;
    Ok(board)
}

#[derive(Clone, Copy)]
struct Boundary {
    gate: ProspectAuthorityClosureGate,
    artifact: Option<&'static str>,
    option: Option<&'static str>,
}

fn boundary_for(source_family: &str) -> Boundary {
    match source_family {
        "nhl_contract_publication" => Boundary {
            gate: ProspectAuthorityClosureGate::OrganizationalControl,
            artifact: Some("contract_control_ledger.v1"),
            option: Some("--contract-control-ledger"),
        },
        "nhl_club_camp_publication" => Boundary {
            gate: ProspectAuthorityClosureGate::PopulationAuthority,
            artifact: Some("camp_participation_ledger.v1"),
            option: Some("--camp-participation-ledger"),
        },
        "ahl_current_assignment" => Boundary {
            gate: ProspectAuthorityClosureGate::PopulationAuthority,
            artifact: Some("ahl_roster_stats.v1"),
            option: Some("--ahl-roster-snapshot"),
        },
        _ => Boundary {
            gate: ProspectAuthorityClosureGate::PopulationAuthority,
            artifact: None,
            option: None,
        },
    }
}

fn disposition_for(state: ProspectCensusAuthorityGapState) -> ProspectAuthorityClosureDisposition {
    match state {
        ProspectCensusAuthorityGapState::Failed => ProspectAuthorityClosureDisposition::Acquire,
        ProspectCensusAuthorityGapState::Quarantined => {
            ProspectAuthorityClosureDisposition::ResolveQuarantine
        }
        ProspectCensusAuthorityGapState::IncompletePagination => {
            ProspectAuthorityClosureDisposition::CompletePagination
        }
    }
}

fn remediation_for(
    family: &str,
    state: ProspectCensusAuthorityGapState,
    artifact: Option<&str>,
    option: Option<&str>,
) -> String {
    let action = match state {
        ProspectCensusAuthorityGapState::Failed => "acquire and validate",
        ProspectCensusAuthorityGapState::Quarantined => "resolve quarantine and revalidate",
        ProspectCensusAuthorityGapState::IncompletePagination => {
            "complete terminal pagination and revalidate"
        }
    };
    match (artifact, option) {
        (Some(artifact), Some(option)) => {
            format!("{action} {artifact}, then ingest it with {option}")
        }
        _ => format!("{action} the {family} source family through an approved adapter"),
    }
}

fn summarize_families(
    cells: &[ProspectAuthorityClosureCell],
) -> Vec<ProspectAuthorityClosureFamilySummary> {
    let mut grouped = BTreeMap::<String, Vec<&ProspectAuthorityClosureCell>>::new();
    for cell in cells {
        grouped
            .entry(cell.source_family.clone())
            .or_default()
            .push(cell);
    }
    grouped
        .into_iter()
        .map(|(source_family, rows)| {
            let first = rows[0];
            ProspectAuthorityClosureFamilySummary {
                source_family,
                cells: rows.len(),
                organizations: rows
                    .iter()
                    .map(|row| row.organization.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                gate: first.gate,
                required_artifact_schema: first.required_artifact_schema.clone(),
                ingestion_option: first.ingestion_option.clone(),
            }
        })
        .collect()
}

fn hash_json(value: &impl Serialize) -> Result<String, ProspectAuthorityClosureError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ProspectAuthorityClosureError::InvalidJson(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readiness() -> ProspectCensusReadinessBoardView {
        serde_json::from_str(include_str!(
            "../../../examples/prospect-census-readiness-2026-27.json"
        ))
        .unwrap()
    }

    #[test]
    fn real_readiness_becomes_exact_96_cell_closure_plan() {
        let board = build_prospect_authority_closure(&readiness()).unwrap();
        assert_eq!(board.organizations, 32);
        assert_eq!(board.affected_organizations, 32);
        assert_eq!(board.cells, 96);
        assert_eq!(board.control_blocking_cells, 32);
        assert_eq!(board.population_blocking_cells, 64);
        assert_eq!(board.family_summary.len(), 3);
        assert_eq!(board.cells_for_team("NYR").len(), 3);
        assert_eq!(board.cells_for_team("SEA").len(), 3);
        let contract = board
            .family_summary
            .iter()
            .find(|row| row.source_family == "nhl_contract_publication")
            .unwrap();
        assert_eq!(
            contract.required_artifact_schema.as_deref(),
            Some("contract_control_ledger.v1")
        );
        assert_eq!(board.calculate_fingerprint().unwrap(), board.fingerprint);
    }

    #[test]
    fn closure_refuses_tampered_readiness_fingerprint() {
        let mut source = readiness();
        source.population_complete_organizations = 1;
        assert_eq!(
            build_prospect_authority_closure(&source).unwrap_err(),
            ProspectAuthorityClosureError::InvalidSourceFingerprint
        );
    }

    #[test]
    fn closure_refuses_refingerprinted_unreconciled_gap_summary() {
        let mut source = readiness();
        source.authority_gap_summary[0].organizations = 31;
        source.fingerprint = source.calculate_fingerprint().unwrap();
        assert_eq!(
            build_prospect_authority_closure(&source).unwrap_err(),
            ProspectAuthorityClosureError::TotalsDoNotReconcile
        );
    }
}
