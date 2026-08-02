//! Leakage-safe adapter for shift-aligned pair/trio outcome evidence.
//!
//! This layer deliberately accepts an already declared expected-goal baseline.
//! IceLines can therefore swap source/model implementations without changing
//! the core chemistry contract, while still preserving the source seals and
//! refusing evidence observed after the game forecast boundary.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use icelines_core::{
    LineChemistryEvidenceInput, LineChemistryEvidenceKind, CANONICAL_TEAMS,
    LINE_CHEMISTRY_EVIDENCE_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SHIFT_ADJUSTED_UNIT_OUTCOME_SCHEMA: &str = "shift_adjusted_unit_outcome.v1";
pub const SHIFT_ADJUSTED_CHEMISTRY_ADAPTER_SCHEMA: &str = "shift_adjusted_chemistry_adapter.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShiftAdjustedUnitOutcomeInput {
    pub schema: String,
    pub team: String,
    /// Exactly two or three stable player IDs; ordering is immaterial.
    pub player_ids: Vec<u32>,
    pub evidence_cutoff_at: DateTime<Utc>,
    pub shared_games: u32,
    pub shared_minutes: f64,
    /// Shift-aligned on-ice expected-goal share, 0 through 1.
    pub observed_xg_share: f64,
    /// Expected share from the declared individual/opponent/deployment baseline.
    pub baseline_xg_share: f64,
    pub deployment_affinity: Option<f64>,
    pub outcome_source_fingerprint: String,
    pub baseline_source_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShiftAdjustedChemistryAdapterView {
    pub schema: String,
    pub team: String,
    pub forecast_at: DateTime<Utc>,
    pub evidence: Vec<LineChemistryEvidenceInput>,
    pub disclosures: Vec<String>,
    pub source_fingerprints: Vec<String>,
    pub fingerprint: String,
}

pub fn build_shift_adjusted_chemistry_evidence(
    team: &str,
    forecast_at: DateTime<Utc>,
    rows: Vec<ShiftAdjustedUnitOutcomeInput>,
) -> Result<ShiftAdjustedChemistryAdapterView, String> {
    let team = team.trim().to_ascii_uppercase();
    if !canonical_team(&team) || rows.is_empty() {
        return Err("shift-adjusted chemistry requires a canonical team and evidence rows".into());
    }
    let mut keys = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut evidence = Vec::with_capacity(rows.len());
    for row in rows {
        let mut player_ids = row.player_ids.clone();
        player_ids.sort_unstable();
        if row.schema != SHIFT_ADJUSTED_UNIT_OUTCOME_SCHEMA
            || !row.team.eq_ignore_ascii_case(&team)
            || row.evidence_cutoff_at > forecast_at
            || !(2..=3).contains(&player_ids.len())
            || player_ids.contains(&0)
            || player_ids.windows(2).any(|ids| ids[0] == ids[1])
            || !keys.insert(player_ids.clone())
            || row.shared_games == 0
            || !row.shared_minutes.is_finite()
            || row.shared_minutes <= 0.0
            || !valid_share(row.observed_xg_share)
            || !valid_share(row.baseline_xg_share)
            || row
                .deployment_affinity
                .is_some_and(|value| !valid_share(value))
            || !valid_fingerprint(&row.outcome_source_fingerprint)
            || !valid_fingerprint(&row.baseline_source_fingerprint)
        {
            return Err(
                "shift-adjusted chemistry rejects invalid, duplicate, cross-team, or future evidence"
                    .into(),
            );
        }
        sources.insert(row.outcome_source_fingerprint.clone());
        sources.insert(row.baseline_source_fingerprint.clone());
        let mut canonical_row = row.clone();
        canonical_row.player_ids = player_ids.clone();
        let source_fingerprint = fingerprint(&canonical_row)?;
        evidence.push(LineChemistryEvidenceInput {
            schema: LINE_CHEMISTRY_EVIDENCE_SCHEMA.to_owned(),
            player_ids,
            team: team.clone(),
            evidence_cutoff_at: row.evidence_cutoff_at,
            shared_games: row.shared_games,
            shared_minutes: row.shared_minutes,
            // Map a full 0..1 share delta onto the core -1..1 residual range.
            performance_residual: Some(
                ((row.observed_xg_share - row.baseline_xg_share) * 2.0).clamp(-1.0, 1.0),
            ),
            deployment_affinity: row.deployment_affinity,
            kind: LineChemistryEvidenceKind::ShiftAdjustedOutcome,
            source_fingerprint,
        });
    }
    evidence.sort_by(|left, right| left.player_ids.cmp(&right.player_ids));
    let mut view = ShiftAdjustedChemistryAdapterView {
        schema: SHIFT_ADJUSTED_CHEMISTRY_ADAPTER_SCHEMA.to_owned(),
        team,
        forecast_at,
        evidence,
        disclosures: vec![
            "Residual is observed shift-aligned xG share minus the declared individual/opponent/deployment baseline; it is not raw shared ice.".to_owned(),
            "Pair and trio reliability shrinkage is applied later by the core Matchup model from shared games and minutes.".to_owned(),
            "Rows after the forecast boundary are rejected to preserve chronological replay.".to_owned(),
        ],
        source_fingerprints: sources.into_iter().collect(),
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

fn valid_share(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn canonical_team(team: &str) -> bool {
    CANONICAL_TEAMS.iter().any(|(abbr, _)| *abbr == team)
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn seal(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn row() -> ShiftAdjustedUnitOutcomeInput {
        ShiftAdjustedUnitOutcomeInput {
            schema: SHIFT_ADJUSTED_UNIT_OUTCOME_SCHEMA.to_owned(),
            team: "NYR".to_owned(),
            player_ids: vec![93, 10],
            evidence_cutoff_at: Utc.with_ymd_and_hms(2026, 4, 18, 20, 0, 0).unwrap(),
            shared_games: 11,
            shared_minutes: 108.0,
            observed_xg_share: 0.57,
            baseline_xg_share: 0.52,
            deployment_affinity: Some(0.42),
            outcome_source_fingerprint: seal('a'),
            baseline_source_fingerprint: seal('b'),
        }
    }

    #[test]
    fn adjusted_outcome_preserves_sample_and_builds_residual() {
        let forecast_at = Utc.with_ymd_and_hms(2026, 4, 19, 12, 0, 0).unwrap();
        let view = build_shift_adjusted_chemistry_evidence("nyr", forecast_at, vec![row()])
            .expect("valid shift-adjusted evidence");
        assert_eq!(view.evidence.len(), 1);
        assert_eq!(view.evidence[0].player_ids, vec![10, 93]);
        assert_eq!(view.evidence[0].shared_games, 11);
        assert!((view.evidence[0].performance_residual.unwrap() - 0.1).abs() < 1e-9);
        assert_eq!(
            view.evidence[0].kind,
            LineChemistryEvidenceKind::ShiftAdjustedOutcome
        );
    }

    #[test]
    fn future_or_duplicate_evidence_is_rejected() {
        let forecast_at = Utc.with_ymd_and_hms(2026, 4, 18, 12, 0, 0).unwrap();
        assert!(build_shift_adjusted_chemistry_evidence("NYR", forecast_at, vec![row()]).is_err());

        let forecast_at = Utc.with_ymd_and_hms(2026, 4, 19, 12, 0, 0).unwrap();
        assert!(
            build_shift_adjusted_chemistry_evidence("NYR", forecast_at, vec![row(), row()])
                .is_err()
        );
    }
}
