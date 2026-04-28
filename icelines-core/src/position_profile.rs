//! PositionProfile — aggregates per-game boxscore position data for a player.
//!
//! The profile is built from NHL gamecenter boxscore data, tracking how many
//! games a player appeared at each position in a given season.  Primary and
//! multi-eligible positions are derived from the appearance counts.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Position;

/// Aggregated position eligibility for one player in one season.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionProfile {
    pub player_id: u32,
    pub season: String,
    /// position → number of games the player appeared at that position
    pub appearances: HashMap<Position, u32>,
    /// position with the most appearances (ties broken alphabetically by abbreviation)
    pub primary: Position,
    /// positions where `count / total_games >= 0.20`; always includes `primary`
    pub multi_eligible: Vec<Position>,
    pub total_games: u32,
}

impl PositionProfile {
    /// Build a `PositionProfile` from raw appearance counts.
    ///
    /// Returns `None` if `total_games` is zero (no data to derive from).
    ///
    /// `appearances` maps `Position` → game count for that position.
    /// Any positions absent from the map are treated as zero appearances.
    pub fn build(
        player_id: u32,
        season: String,
        appearances: HashMap<Position, u32>,
    ) -> Option<Self> {
        let total_games: u32 = appearances.values().sum();
        if total_games == 0 {
            return None;
        }

        // Primary = argmax of appearances; ties broken alphabetically by abbreviation.
        let primary = appearances
            .iter()
            .max_by(|(pa, &ca), (pb, &cb)| {
                ca.cmp(&cb).then_with(|| {
                    // Lower abbreviation wins tie (alphabetical ascending → we want
                    // the *smallest* abbreviation, so reverse the ordering for max_by)
                    pb.abbreviation().cmp(pa.abbreviation())
                })
            })
            .map(|(&pos, _)| pos)
            .expect("appearances is non-empty because total_games > 0");

        // Multi-eligible: any position with count/total >= 0.20; primary always included.
        let threshold = 0.20_f64;
        let mut multi_eligible: Vec<Position> = appearances
            .iter()
            .filter(|(&pos, &count)| {
                pos == primary || (count as f64 / total_games as f64) >= threshold
            })
            .map(|(&pos, _)| pos)
            .collect();

        // Stable sort: alphabetical by abbreviation so the order is deterministic.
        multi_eligible.sort_by(|a, b| a.abbreviation().cmp(b.abbreviation()));
        // Deduplicate (primary may already appear via the filter).
        multi_eligible.dedup();

        Some(Self {
            player_id,
            season,
            appearances,
            primary,
            multi_eligible,
            total_games,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn appearances(pairs: &[(Position, u32)]) -> HashMap<Position, u32> {
        pairs.iter().cloned().collect()
    }

    /// L0: Draisaitl-style — C=45, L=25 → primary=C, multi=[C, LW]
    /// (25/70 ≈ 35.7% ≥ 20%)
    #[test]
    fn draisaitl_multi_eligible() {
        let app = appearances(&[(Position::Center, 45), (Position::LeftWing, 25)]);
        let profile = PositionProfile::build(8478402, "20252026".into(), app).unwrap();

        assert_eq!(profile.primary, Position::Center);
        assert_eq!(profile.total_games, 70);
        // multi should contain both C and LW
        assert!(profile.multi_eligible.contains(&Position::Center));
        assert!(profile.multi_eligible.contains(&Position::LeftWing));
        assert_eq!(profile.multi_eligible.len(), 2);
    }

    /// L0: Single position — L=78 → primary=LW, multi=[LW]
    #[test]
    fn mcmann_single_position() {
        let app = appearances(&[(Position::LeftWing, 78)]);
        let profile = PositionProfile::build(8481533, "20252026".into(), app).unwrap();

        assert_eq!(profile.primary, Position::LeftWing);
        assert_eq!(profile.multi_eligible, vec![Position::LeftWing]);
    }

    /// L0: Tie-break by abbreviation — C=40, R=40 → primary=C (C < RW alphabetically)
    #[test]
    fn tie_break_alpha() {
        let app = appearances(&[(Position::Center, 40), (Position::RightWing, 40)]);
        let profile = PositionProfile::build(9999999, "20252026".into(), app).unwrap();

        assert_eq!(profile.primary, Position::Center);
        // Both qualify at 50% ≥ 20%
        assert!(profile.multi_eligible.contains(&Position::Center));
        assert!(profile.multi_eligible.contains(&Position::RightWing));
    }

    /// L0: All zero appearances → build returns None
    #[test]
    fn zero_games_no_profile() {
        let app = appearances(&[(Position::Center, 0), (Position::LeftWing, 0)]);
        assert!(PositionProfile::build(1234567, "20252026".into(), app).is_none());
    }
}
