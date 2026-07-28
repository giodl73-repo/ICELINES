//! Sealed official standings authority for historical Window evaluation.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use icelines_core::{WindowOutcomeRow, CANONICAL_TEAMS};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::nhl_api::NhlStandingsRow;

pub const ORGANIZATION_WINDOW_STANDINGS_SCHEMA: &str = "organization_window_standings_snapshot.v1";
pub const ORGANIZATION_WINDOW_HISTORICAL_IDENTITY_VERSION: &str = "nhl_franchise_continuity_32.v1";
pub const NHL_STANDINGS_SOURCE_BASE: &str = "https://api-web.nhle.com/v1/standings";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowStandingRow {
    /// Stable Window organization identity.
    pub organization: String,
    /// Abbreviation reported by the NHL endpoint at the historical date.
    pub observed_team: String,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub overtime_losses: u32,
    pub points: u32,
    pub points_percentage: f64,
    pub goal_differential: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowStandingsSnapshot {
    pub schema: String,
    pub target_season: u32,
    pub effective_date: NaiveDate,
    pub captured_at: String,
    pub source_url: String,
    /// SHA-256 of the sorted, normalized standings projection used here.
    pub source_projection_fingerprint: String,
    pub organization_identity_version: String,
    pub rows: Vec<OrganizationWindowStandingRow>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl OrganizationWindowStandingsSnapshot {
    pub fn outcomes(&self) -> Vec<WindowOutcomeRow> {
        self.rows
            .iter()
            .map(|row| WindowOutcomeRow {
                organization: row.organization.clone(),
                target_value: row.points_percentage * 100.0,
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationWindowHistoryError {
    #[error("historical standings capture timestamp is empty")]
    EmptyCapturedAt,
    #[error("historical standings cohort does not match the canonical 32 organizations")]
    InvalidCohort,
    #[error("historical standings contains duplicate organization {0}")]
    DuplicateOrganization(String),
    #[error("historical standings row is invalid for {0}")]
    InvalidStanding(String),
    #[error("historical standings serialization failed: {0}")]
    Serialization(String),
}

pub fn build_organization_window_standings_snapshot(
    target_season: u32,
    effective_date: NaiveDate,
    captured_at: &str,
    standings: &[NhlStandingsRow],
) -> Result<OrganizationWindowStandingsSnapshot, OrganizationWindowHistoryError> {
    if captured_at.trim().is_empty() {
        return Err(OrganizationWindowHistoryError::EmptyCapturedAt);
    }
    let mut rows = standings
        .iter()
        .map(|standing| OrganizationWindowStandingRow {
            organization: historical_franchise_organization(&standing.team),
            observed_team: standing.team.to_ascii_uppercase(),
            games_played: standing.games_played,
            wins: standing.wins,
            losses: standing.losses,
            overtime_losses: standing.overtime_losses,
            points: standing.points,
            points_percentage: f64::from(standing.points_percentage),
            goal_differential: standing.goal_differential,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.organization.cmp(&right.organization));

    let mut organizations = BTreeSet::new();
    for row in &rows {
        if !organizations.insert(row.organization.as_str()) {
            return Err(OrganizationWindowHistoryError::DuplicateOrganization(
                row.organization.clone(),
            ));
        }
        let decisions = row.wins + row.losses + row.overtime_losses;
        let calculated_percentage = row.points as f64 / (row.games_played * 2) as f64;
        if row.games_played == 0
            || decisions != row.games_played
            || !row.points_percentage.is_finite()
            || !(0.0..=1.0).contains(&row.points_percentage)
            || row.points != row.wins * 2 + row.overtime_losses
            || (row.points_percentage - calculated_percentage).abs() > 0.000_01
        {
            return Err(OrganizationWindowHistoryError::InvalidStanding(
                row.observed_team.clone(),
            ));
        }
    }
    let expected = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| *team)
        .collect::<BTreeSet<_>>();
    if organizations != expected {
        return Err(OrganizationWindowHistoryError::InvalidCohort);
    }

    let source_projection_fingerprint = sha256_json(&rows)?;
    let mut snapshot = OrganizationWindowStandingsSnapshot {
        schema: ORGANIZATION_WINDOW_STANDINGS_SCHEMA.to_owned(),
        target_season,
        effective_date,
        captured_at: captured_at.to_owned(),
        source_url: format!("{NHL_STANDINGS_SOURCE_BASE}/{effective_date}"),
        source_projection_fingerprint,
        organization_identity_version: ORGANIZATION_WINDOW_HISTORICAL_IDENTITY_VERSION.to_owned(),
        rows,
        disclosures: vec![
            "Outcomes are final official regular-season standings point percentages normalized to 0..100.".to_owned(),
            "ARI and PHX observations map to the stable UTA franchise identity; observed_team preserves the historical abbreviation.".to_owned(),
            "The source projection fingerprint seals normalized parsed rows, not the byte-for-byte HTTP response.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    snapshot.fingerprint = snapshot_fingerprint(&snapshot)?;
    Ok(snapshot)
}

pub fn historical_franchise_organization(team: &str) -> String {
    match team.trim().to_ascii_uppercase().as_str() {
        "ARI" | "PHX" => "UTA".to_owned(),
        normalized => normalized.to_owned(),
    }
}

fn snapshot_fingerprint(
    snapshot: &OrganizationWindowStandingsSnapshot,
) -> Result<String, OrganizationWindowHistoryError> {
    let mut canonical = snapshot.clone();
    canonical.fingerprint.clear();
    canonical
        .rows
        .sort_by(|left, right| left.organization.cmp(&right.organization));
    canonical.disclosures.sort();
    sha256_json(&canonical)
}

fn sha256_json(value: &impl Serialize) -> Result<String, OrganizationWindowHistoryError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing(team: &str, points: u32) -> NhlStandingsRow {
        let wins = points / 2;
        let overtime_losses = points % 2;
        NhlStandingsRow {
            team: team.to_owned(),
            conference: None,
            division: None,
            games_played: 82,
            wins,
            losses: 82 - wins - overtime_losses,
            overtime_losses,
            points,
            points_percentage: points as f32 / 164.0,
            regulation_wins: None,
            goal_differential: 0,
            league_rank: None,
            conference_rank: None,
            division_rank: None,
            wild_card_rank: None,
        }
    }

    fn historical_cohort() -> Vec<NhlStandingsRow> {
        CANONICAL_TEAMS
            .iter()
            .enumerate()
            .map(|(index, (team, _))| {
                standing(if *team == "UTA" { "ARI" } else { team }, 70 + index as u32)
            })
            .collect()
    }

    #[test]
    fn l0_snapshot_seals_canonical_outcomes_and_preserves_historical_identity() {
        let rows = historical_cohort();
        let snapshot = build_organization_window_standings_snapshot(
            20232024,
            NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
            "2026-07-28T08:00:00Z",
            &rows,
        )
        .unwrap();
        assert_eq!(snapshot.rows.len(), 32);
        let utah = snapshot
            .rows
            .iter()
            .find(|row| row.organization == "UTA")
            .unwrap();
        assert_eq!(utah.observed_team, "ARI");
        assert_eq!(snapshot.outcomes().len(), 32);
        assert_eq!(snapshot.fingerprint.len(), 64);

        let mut reversed = rows;
        reversed.reverse();
        let same = build_organization_window_standings_snapshot(
            20232024,
            NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
            "2026-07-28T08:00:00Z",
            &reversed,
        )
        .unwrap();
        assert_eq!(snapshot.fingerprint, same.fingerprint);
    }

    #[test]
    fn l0_snapshot_rejects_incomplete_or_duplicate_franchise_cohorts() {
        let mut incomplete = historical_cohort();
        incomplete.pop();
        assert_eq!(
            build_organization_window_standings_snapshot(
                20232024,
                NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
                "2026-07-28T08:00:00Z",
                &incomplete,
            ),
            Err(OrganizationWindowHistoryError::InvalidCohort)
        );

        let mut duplicate = historical_cohort();
        duplicate.push(standing("UTA", 90));
        assert_eq!(
            build_organization_window_standings_snapshot(
                20232024,
                NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
                "2026-07-28T08:00:00Z",
                &duplicate,
            ),
            Err(OrganizationWindowHistoryError::DuplicateOrganization(
                "UTA".to_owned()
            ))
        );
    }
}
