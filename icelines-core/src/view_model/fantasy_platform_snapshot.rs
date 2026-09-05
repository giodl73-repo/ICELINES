use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};

use super::fantasy_assistant::FantasyPlayerAvailabilityStatus;

pub const FANTASY_PLATFORM_SNAPSHOT_SCHEMA: &str = "fantasy_platform_snapshot.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlatformStandingRow {
    pub rank: u16,
    pub team: String,
    #[serde(default)]
    pub wins: Option<u16>,
    #[serde(default)]
    pub losses: Option<u16>,
    #[serde(default)]
    pub ties: Option<u16>,
    #[serde(default)]
    pub points_for: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlatformMatchupSnapshot {
    pub week_start: NaiveDate,
    pub team: String,
    pub opponent: String,
    pub team_points: f64,
    pub opponent_points: f64,
    #[serde(default)]
    pub through: Option<NaiveDate>,
    #[serde(default)]
    pub team_goalie_appearances: Option<u8>,
    #[serde(default)]
    pub opponent_goalie_appearances: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyPlatformPlayerStatusRow {
    pub player: String,
    pub status: FantasyPlayerAvailabilityStatus,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlatformSnapshot {
    pub schema: String,
    #[serde(default = "default_platform")]
    pub platform: String,
    pub captured_at: DateTime<Utc>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub standings: Vec<FantasyPlatformStandingRow>,
    #[serde(default)]
    pub matchup: Option<FantasyPlatformMatchupSnapshot>,
    #[serde(default)]
    pub statuses: Vec<FantasyPlatformPlayerStatusRow>,
}

fn default_platform() -> String {
    "yahoo".to_owned()
}

impl FantasyPlatformSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != FANTASY_PLATFORM_SNAPSHOT_SCHEMA {
            return Err(format!(
                "unsupported snapshot schema '{}'; expected {FANTASY_PLATFORM_SNAPSHOT_SCHEMA}",
                self.schema
            ));
        }
        if self.platform.trim().is_empty() {
            return Err("snapshot platform is required".to_owned());
        }
        let mut ranks = BTreeSet::new();
        let mut teams = BTreeSet::new();
        for row in &self.standings {
            if row.rank == 0 || row.team.trim().is_empty() {
                return Err("every standing requires a positive rank and team".to_owned());
            }
            if !ranks.insert(row.rank) {
                return Err(format!("duplicate standings rank {}", row.rank));
            }
            if !teams.insert(row.team.trim().to_ascii_lowercase()) {
                return Err(format!("duplicate standings team '{}'", row.team));
            }
            if row.points_for.is_some_and(|value| !value.is_finite()) {
                return Err(format!("non-finite points_for for '{}'", row.team));
            }
        }
        if let Some(matchup) = &self.matchup {
            if matchup.week_start.weekday() != Weekday::Mon {
                return Err("matchup week_start must be a Monday".to_owned());
            }
            if matchup.team.trim().is_empty()
                || matchup.opponent.trim().is_empty()
                || matchup.team.eq_ignore_ascii_case(&matchup.opponent)
            {
                return Err("matchup requires two distinct team names".to_owned());
            }
            if !matchup.team_points.is_finite() || !matchup.opponent_points.is_finite() {
                return Err("matchup points must be finite".to_owned());
            }
            if matchup
                .through
                .is_some_and(|through| through < matchup.week_start)
            {
                return Err("matchup through date cannot precede week_start".to_owned());
            }
        }
        let mut status_players = BTreeSet::new();
        for row in &self.statuses {
            if row.player.trim().is_empty() {
                return Err("status player is required".to_owned());
            }
            if !status_players.insert(row.player.trim().to_ascii_lowercase()) {
                return Err(format!("duplicate status player '{}'", row.player));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlatformStandingDeltaRow {
    pub team: String,
    pub rank: u16,
    pub previous_rank: Option<u16>,
    /// Positive means the team moved upward in the standings.
    pub rank_change: Option<i16>,
    pub points_for: Option<f64>,
    pub points_for_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlatformSnapshotView {
    pub schema: String,
    pub snapshot: FantasyPlatformSnapshot,
    pub previous_captured_at: Option<DateTime<Utc>>,
    pub standings: Vec<FantasyPlatformStandingDeltaRow>,
}

pub fn build_fantasy_platform_snapshot_view(
    snapshot: FantasyPlatformSnapshot,
    previous: Option<&FantasyPlatformSnapshot>,
) -> Result<FantasyPlatformSnapshotView, String> {
    snapshot.validate()?;
    if let Some(previous) = previous {
        previous.validate()?;
    }
    let previous_by_team = previous
        .map(|snapshot| {
            snapshot
                .standings
                .iter()
                .map(|row| (row.team.trim().to_ascii_lowercase(), row))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let standings = snapshot
        .standings
        .iter()
        .map(|row| {
            let prior = previous_by_team.get(&row.team.trim().to_ascii_lowercase());
            FantasyPlatformStandingDeltaRow {
                team: row.team.clone(),
                rank: row.rank,
                previous_rank: prior.map(|prior| prior.rank),
                rank_change: prior.map(|prior| prior.rank as i16 - row.rank as i16),
                points_for: row.points_for,
                points_for_change: row
                    .points_for
                    .zip(prior.and_then(|prior| prior.points_for))
                    .map(|(current, prior)| current - prior),
            }
        })
        .collect();
    Ok(FantasyPlatformSnapshotView {
        schema: FANTASY_PLATFORM_SNAPSHOT_SCHEMA.to_owned(),
        snapshot,
        previous_captured_at: previous.map(|snapshot| snapshot.captured_at),
        standings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(rank: u16, points: f64) -> FantasyPlatformSnapshot {
        FantasyPlatformSnapshot {
            schema: FANTASY_PLATFORM_SNAPSHOT_SCHEMA.to_owned(),
            platform: "yahoo".to_owned(),
            captured_at: DateTime::parse_from_rfc3339("2026-10-10T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            source_url: None,
            standings: vec![FantasyPlatformStandingRow {
                rank,
                team: "Felix's Five-Hole".to_owned(),
                wins: Some(1),
                losses: Some(0),
                ties: Some(0),
                points_for: Some(points),
            }],
            matchup: None,
            statuses: Vec::new(),
        }
    }

    #[test]
    fn snapshot_view_keeps_observed_rank_and_points_deltas() {
        let prior = snapshot(7, 100.0);
        let current = snapshot(5, 125.5);
        let view = build_fantasy_platform_snapshot_view(current, Some(&prior)).unwrap();
        assert_eq!(view.standings[0].rank_change, Some(2));
        assert_eq!(view.standings[0].points_for_change, Some(25.5));
    }

    #[test]
    fn snapshot_rejects_duplicate_standings_teams() {
        let mut invalid = snapshot(1, 1.0);
        invalid.standings.push(FantasyPlatformStandingRow {
            rank: 2,
            team: "felix's five-hole".to_owned(),
            wins: None,
            losses: None,
            ties: None,
            points_for: None,
        });
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn snapshot_rejects_non_monday_matchup_start() {
        let mut invalid = snapshot(1, 1.0);
        invalid.matchup = Some(FantasyPlatformMatchupSnapshot {
            week_start: NaiveDate::from_ymd_opt(2026, 10, 6).unwrap(),
            team: "Felix's Five-Hole".to_owned(),
            opponent: "PENSylvania".to_owned(),
            team_points: 10.0,
            opponent_points: 9.0,
            through: None,
            team_goalie_appearances: None,
            opponent_goalie_appearances: None,
        });
        assert_eq!(
            invalid.validate().unwrap_err(),
            "matchup week_start must be a Monday"
        );
    }
}
