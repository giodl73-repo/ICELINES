//! Deterministic progress delta between two prospect authority closure boards.

use std::collections::{BTreeMap, BTreeSet};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    validate_prospect_authority_closure_board, ProspectAuthorityClosureBoardView,
    ProspectAuthorityClosureError, ProspectAuthorityClosureGate, ProspectCensusAuthorityGapState,
};

pub const PROSPECT_AUTHORITY_PROGRESS_SCHEMA: &str = "prospect_authority_progress.v1";
pub const PROSPECT_AUTHORITY_PROGRESS_METHOD: &str = "prospect_authority_progress.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectAuthorityProgressChangeKind {
    Closed,
    Opened,
    StateChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectAuthorityProgressChange {
    pub organization: String,
    pub source_family: String,
    pub gate: ProspectAuthorityClosureGate,
    pub kind: ProspectAuthorityProgressChangeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_state: Option<ProspectCensusAuthorityGapState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_state: Option<ProspectCensusAuthorityGapState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectAuthorityProgressView {
    pub schema: String,
    pub method_version: String,
    pub evaluation_season: u32,
    pub prior_knowledge_cutoff: String,
    pub current_knowledge_cutoff: String,
    pub prior_closure_fingerprint: String,
    pub current_closure_fingerprint: String,
    pub prior_cells: usize,
    pub current_cells: usize,
    pub closed_cells: usize,
    pub opened_cells: usize,
    pub state_changed_cells: usize,
    pub persisting_cells: usize,
    pub control_cells_closed: usize,
    pub population_cells_closed: usize,
    pub closure_basis_points: u16,
    pub changes: Vec<ProspectAuthorityProgressChange>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl ProspectAuthorityProgressView {
    pub fn calculate_fingerprint(&self) -> Result<String, ProspectAuthorityProgressError> {
        let mut canonical = self.clone();
        canonical.fingerprint.clear();
        canonical.changes.sort_by(|left, right| {
            left.organization
                .cmp(&right.organization)
                .then_with(|| left.source_family.cmp(&right.source_family))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        hash_json(&canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProspectAuthorityProgressError {
    #[error("unsupported prospect authority closure schema: {0}")]
    UnsupportedSchema(String),
    #[error("prospect authority closure fingerprint does not match its contents")]
    InvalidSourceFingerprint,
    #[error("authority progress requires matching evaluation seasons")]
    SeasonMismatch,
    #[error("current authority knowledge cutoff precedes the prior cutoff")]
    ReversedKnowledgeCutoff,
    #[error("authority closure board contains duplicate team/family cells")]
    DuplicateCell,
    #[error("authority closure board totals do not reconcile")]
    InvalidBoardTotals,
    #[error("authority gate changed for the same team/family cell")]
    GateChanged,
    #[error("prospect authority progress JSON failed: {0}")]
    InvalidJson(String),
    #[error("prospect authority cutoff is invalid: {0}")]
    InvalidCutoff(String),
}

pub fn build_prospect_authority_progress(
    prior: &ProspectAuthorityClosureBoardView,
    current: &ProspectAuthorityClosureBoardView,
) -> Result<ProspectAuthorityProgressView, ProspectAuthorityProgressError> {
    validate_prospect_authority_closure_board(prior).map_err(map_closure_validation_error)?;
    validate_prospect_authority_closure_board(current).map_err(map_closure_validation_error)?;
    if prior.evaluation_season != current.evaluation_season {
        return Err(ProspectAuthorityProgressError::SeasonMismatch);
    }
    let prior_cutoff = DateTime::parse_from_rfc3339(&prior.knowledge_cutoff)
        .map_err(|error| ProspectAuthorityProgressError::InvalidCutoff(error.to_string()))?;
    let current_cutoff = DateTime::parse_from_rfc3339(&current.knowledge_cutoff)
        .map_err(|error| ProspectAuthorityProgressError::InvalidCutoff(error.to_string()))?;
    if current_cutoff < prior_cutoff {
        return Err(ProspectAuthorityProgressError::ReversedKnowledgeCutoff);
    }

    let prior_cells = index_cells(prior)?;
    let current_cells = index_cells(current)?;
    let keys = prior_cells
        .keys()
        .chain(current_cells.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    let mut persisting_cells = 0usize;
    for key in keys {
        match (prior_cells.get(&key), current_cells.get(&key)) {
            (Some(prior_cell), None) => changes.push(ProspectAuthorityProgressChange {
                organization: key.0,
                source_family: key.1,
                gate: prior_cell.gate,
                kind: ProspectAuthorityProgressChangeKind::Closed,
                prior_state: Some(prior_cell.state),
                current_state: None,
            }),
            (None, Some(current_cell)) => changes.push(ProspectAuthorityProgressChange {
                organization: key.0,
                source_family: key.1,
                gate: current_cell.gate,
                kind: ProspectAuthorityProgressChangeKind::Opened,
                prior_state: None,
                current_state: Some(current_cell.state),
            }),
            (Some(prior_cell), Some(current_cell)) => {
                if prior_cell.gate != current_cell.gate {
                    return Err(ProspectAuthorityProgressError::GateChanged);
                }
                persisting_cells += 1;
                if prior_cell.state != current_cell.state {
                    changes.push(ProspectAuthorityProgressChange {
                        organization: key.0,
                        source_family: key.1,
                        gate: current_cell.gate,
                        kind: ProspectAuthorityProgressChangeKind::StateChanged,
                        prior_state: Some(prior_cell.state),
                        current_state: Some(current_cell.state),
                    });
                }
            }
            (None, None) => unreachable!("union key must occur in at least one board"),
        }
    }
    let closed_cells = changes
        .iter()
        .filter(|row| row.kind == ProspectAuthorityProgressChangeKind::Closed)
        .count();
    let opened_cells = changes
        .iter()
        .filter(|row| row.kind == ProspectAuthorityProgressChangeKind::Opened)
        .count();
    let state_changed_cells = changes
        .iter()
        .filter(|row| row.kind == ProspectAuthorityProgressChangeKind::StateChanged)
        .count();
    let control_cells_closed = changes
        .iter()
        .filter(|row| {
            row.kind == ProspectAuthorityProgressChangeKind::Closed
                && row.gate == ProspectAuthorityClosureGate::OrganizationalControl
        })
        .count();
    let population_cells_closed = closed_cells - control_cells_closed;
    let rounded_basis_points = (closed_cells as u128 * 10_000 + (prior.cells / 2) as u128)
        .checked_div(prior.cells as u128)
        .unwrap_or(0);
    let closure_basis_points = u16::try_from(rounded_basis_points)
        .expect("closed authority cells cannot exceed prior cells");

    let mut view = ProspectAuthorityProgressView {
        schema: PROSPECT_AUTHORITY_PROGRESS_SCHEMA.to_owned(),
        method_version: PROSPECT_AUTHORITY_PROGRESS_METHOD.to_owned(),
        evaluation_season: prior.evaluation_season,
        prior_knowledge_cutoff: prior.knowledge_cutoff.clone(),
        current_knowledge_cutoff: current.knowledge_cutoff.clone(),
        prior_closure_fingerprint: prior.fingerprint.clone(),
        current_closure_fingerprint: current.fingerprint.clone(),
        prior_cells: prior.cells,
        current_cells: current.cells,
        closed_cells,
        opened_cells,
        state_changed_cells,
        persisting_cells,
        control_cells_closed,
        population_cells_closed,
        closure_basis_points,
        changes,
        disclosures: vec![
            "A closed cell is absent from the later sealed closure board; this delta does not independently approve source evidence.".to_owned(),
            "Opened cells are regressions or newly required authority and remain separate from state changes on persisting cells.".to_owned(),
            "Closure percentage uses the prior board as its denominator and does not imply census publication readiness.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    view.fingerprint = view.calculate_fingerprint()?;
    Ok(view)
}

type CellKey = (String, String);

fn index_cells(
    board: &ProspectAuthorityClosureBoardView,
) -> Result<BTreeMap<CellKey, &crate::ProspectAuthorityClosureCell>, ProspectAuthorityProgressError>
{
    let mut cells = BTreeMap::new();
    for cell in &board.closure_cells {
        if cells
            .insert(
                (cell.organization.clone(), cell.source_family.clone()),
                cell,
            )
            .is_some()
        {
            return Err(ProspectAuthorityProgressError::DuplicateCell);
        }
    }
    Ok(cells)
}

fn map_closure_validation_error(
    error: ProspectAuthorityClosureError,
) -> ProspectAuthorityProgressError {
    match error {
        ProspectAuthorityClosureError::UnsupportedBoardSchema(schema) => {
            ProspectAuthorityProgressError::UnsupportedSchema(schema)
        }
        ProspectAuthorityClosureError::InvalidBoardFingerprint => {
            ProspectAuthorityProgressError::InvalidSourceFingerprint
        }
        ProspectAuthorityClosureError::DuplicateCell => {
            ProspectAuthorityProgressError::DuplicateCell
        }
        ProspectAuthorityClosureError::InvalidJson(error) => {
            ProspectAuthorityProgressError::InvalidJson(error)
        }
        ProspectAuthorityClosureError::InvalidCutoff(error) => {
            ProspectAuthorityProgressError::InvalidCutoff(error)
        }
        _ => ProspectAuthorityProgressError::InvalidBoardTotals,
    }
}

fn hash_json(value: &impl Serialize) -> Result<String, ProspectAuthorityProgressError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ProspectAuthorityProgressError::InvalidJson(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(path: &str) -> ProspectAuthorityClosureBoardView {
        serde_json::from_str(path).unwrap()
    }

    #[test]
    fn real_ahl_replay_closes_exactly_32_population_cells() {
        let prior = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27-pre-ahl.json"
        ));
        let current = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27.json"
        ));
        let progress = build_prospect_authority_progress(&prior, &current).unwrap();
        assert_eq!(progress.prior_cells, 96);
        assert_eq!(progress.current_cells, 64);
        assert_eq!(progress.closed_cells, 32);
        assert_eq!(progress.opened_cells, 0);
        assert_eq!(progress.state_changed_cells, 0);
        assert_eq!(progress.persisting_cells, 64);
        assert_eq!(progress.control_cells_closed, 0);
        assert_eq!(progress.population_cells_closed, 32);
        assert_eq!(progress.closure_basis_points, 3333);
        assert!(progress.changes.iter().all(|row| {
            row.kind == ProspectAuthorityProgressChangeKind::Closed
                && row.source_family == "ahl_current_assignment"
        }));
        assert_eq!(
            progress.calculate_fingerprint().unwrap(),
            progress.fingerprint
        );
    }

    #[test]
    fn progress_refuses_reversed_cutoffs() {
        let prior = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27.json"
        ));
        let current = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27-pre-ahl.json"
        ));
        assert_eq!(
            build_prospect_authority_progress(&prior, &current).unwrap_err(),
            ProspectAuthorityProgressError::ReversedKnowledgeCutoff
        );
    }

    #[test]
    fn progress_refuses_a_fingerprint_valid_board_with_unreconciled_totals() {
        let prior = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27-pre-ahl.json"
        ));
        let mut current = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27.json"
        ));
        current.cells += 1;
        current.fingerprint = current.calculate_fingerprint().unwrap();

        assert_eq!(
            build_prospect_authority_progress(&prior, &current).unwrap_err(),
            ProspectAuthorityProgressError::InvalidBoardTotals
        );
    }

    #[test]
    fn progress_refuses_duplicate_family_summaries_that_mask_a_missing_family() {
        let prior = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27-pre-ahl.json"
        ));
        let mut current = board(include_str!(
            "../../../examples/prospect-authority-closure-2026-27.json"
        ));
        current.family_summary[1] = current.family_summary[0].clone();
        current.fingerprint = current.calculate_fingerprint().unwrap();

        assert_eq!(
            build_prospect_authority_progress(&prior, &current).unwrap_err(),
            ProspectAuthorityProgressError::InvalidBoardTotals
        );
    }
}
