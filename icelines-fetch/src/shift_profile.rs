//! ShiftProfile — linemate co-occurrence analysis from boxscore data.
//!
//! Builds a per-player profile tracking which forwards appear together across
//! games, providing linemate relationships for the `icelines mates` command.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Public data structures ────────────────────────────────────────────────────

/// Per-player shift and linemate summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftProfile {
    pub player_id: u32,
    /// Number of games included in the analysis.
    pub games_analyzed: u32,
    /// Average even-strength TOI in seconds per game (integer).
    pub avg_ev_toi_seconds_per_game: u32,
    /// Top linemates by shared appearances (at most 5).
    pub top_linemates: Vec<LinematePair>,
    /// Fraction of shifts that began in the offensive zone (reserved for future API).
    pub zone_start_pct: Option<f32>,
}

/// A single linemate pairing entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinematePair {
    pub partner_id: u32,
    /// Number of games in which both players appeared on the same team.
    pub shared_shifts: u32,
    /// shared_shifts / games_analyzed (fraction of target's games with this partner).
    pub co_ice_pct: f32,
}

// ── BoxscoreData (simplified input type) ─────────────────────────────────────

/// Simplified boxscore holding only the fields needed for linemate analysis.
#[derive(Debug, Clone)]
pub struct BoxscoreData {
    pub game_id: u64,
    pub home_team: String,
    pub away_team: String,
    pub players: Vec<BoxscorePlayerEntry>,
}

/// Per-player entry inside a boxscore.
#[derive(Debug, Clone)]
pub struct BoxscorePlayerEntry {
    pub player_id: u32,
    /// Team abbreviation (e.g. "NYR").
    pub team: String,
    /// NHL position code: "C", "L", "R", "D", "G".
    pub position: String,
    /// Even-strength TOI in seconds (parsed from "MM:SS").
    pub toi_secs: u32,
    pub shifts: u32,
}

// ── TOI parsing helper ────────────────────────────────────────────────────────

/// Parse a `"MM:SS"` TOI string to total seconds.
/// Returns 0 on any parse error (no panics in library code).
pub fn parse_toi_mmss(s: &str) -> u32 {
    let mut parts = s.splitn(2, ':');
    let mins: u32 = parts.next().and_then(|m| m.parse().ok()).unwrap_or(0);
    let secs: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    mins * 60 + secs
}

// ── Profile builder ───────────────────────────────────────────────────────────

/// Build a `ShiftProfile` for `player_id` from a slice of boxscores.
///
/// Returns `None` if the player appeared in zero games.
///
/// Algorithm:
/// 1. For each boxscore, find the target player's entry (by player_id).
/// 2. Collect all other forwards on the same team in that game.
/// 3. Accumulate total TOI seconds and count teammate co-appearances.
/// 4. Derive avg_ev_toi_seconds_per_game and top 5 linemates.
pub fn build_profile_from_boxscores(
    player_id: u32,
    boxscores: &[BoxscoreData],
) -> Option<ShiftProfile> {
    let mut games_analyzed: u32 = 0;
    let mut total_toi_secs: u64 = 0;
    // partner_id → number of shared games
    let mut shared_game_counts: HashMap<u32, u32> = HashMap::new();

    for boxscore in boxscores {
        // Find the target player in this game.
        let target = boxscore.players.iter().find(|p| p.player_id == player_id);

        let target = match target {
            Some(t) => t,
            None => continue, // player did not appear in this game
        };

        games_analyzed += 1;
        total_toi_secs += u64::from(target.toi_secs);

        let target_team = target.team.clone();

        // Find all other forwards on the same team.
        for entry in &boxscore.players {
            if entry.player_id == player_id {
                continue;
            }
            if entry.team != target_team {
                continue;
            }
            if !is_forward(&entry.position) {
                continue;
            }
            *shared_game_counts.entry(entry.player_id).or_insert(0) += 1;
        }
    }

    if games_analyzed == 0 {
        return None;
    }

    let avg_ev_toi_seconds_per_game = (total_toi_secs / u64::from(games_analyzed)) as u32;

    // Build top-5 linemates sorted by shared_shifts descending.
    let mut linemate_vec: Vec<(u32, u32)> = shared_game_counts.into_iter().collect();
    linemate_vec.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let top_linemates: Vec<LinematePair> = linemate_vec
        .into_iter()
        .take(5)
        .map(|(partner_id, shared_shifts)| LinematePair {
            partner_id,
            shared_shifts,
            co_ice_pct: shared_shifts as f32 / games_analyzed as f32,
        })
        .collect();

    Some(ShiftProfile {
        player_id,
        games_analyzed,
        avg_ev_toi_seconds_per_game,
        top_linemates,
        zone_start_pct: None,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_forward(position: &str) -> bool {
    matches!(position, "C" | "L" | "R")
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(
        player_id: u32,
        team: &str,
        position: &str,
        toi_mmss: &str,
    ) -> BoxscorePlayerEntry {
        BoxscorePlayerEntry {
            player_id,
            team: team.to_owned(),
            position: position.to_owned(),
            toi_secs: parse_toi_mmss(toi_mmss),
            shifts: 0,
        }
    }

    /// L0: single boxscore with 3 players — target played with 2 others.
    #[test]
    fn l0_profile_from_single_boxscore() {
        let boxscore = BoxscoreData {
            game_id: 2025020001,
            home_team: "NYR".to_owned(),
            away_team: "BOS".to_owned(),
            players: vec![
                make_entry(10, "NYR", "C", "18:30"), // target
                make_entry(11, "NYR", "L", "15:00"), // forward teammate A
                make_entry(12, "NYR", "R", "12:00"), // forward teammate B
                make_entry(13, "NYR", "D", "22:00"), // defenseman — excluded
                make_entry(20, "BOS", "C", "17:00"), // opponent — excluded
            ],
        };

        let profile = build_profile_from_boxscores(10, &[boxscore]).unwrap();

        assert_eq!(profile.player_id, 10);
        assert_eq!(profile.games_analyzed, 1);
        assert_eq!(profile.avg_ev_toi_seconds_per_game, 18 * 60 + 30);

        // Both forward teammates should appear.
        assert_eq!(profile.top_linemates.len(), 2);

        let ids: Vec<u32> = profile
            .top_linemates
            .iter()
            .map(|lm| lm.partner_id)
            .collect();
        assert!(ids.contains(&11));
        assert!(ids.contains(&12));

        // co_ice_pct = 1 shared game / 1 game analyzed = 1.0
        for lm in &profile.top_linemates {
            assert!((lm.co_ice_pct - 1.0).abs() < f32::EPSILON);
            assert_eq!(lm.shared_shifts, 1);
        }
    }

    /// L0: player not in any boxscore → None.
    #[test]
    fn l0_profile_gp_zero_returns_none() {
        let boxscore = BoxscoreData {
            game_id: 2025020002,
            home_team: "EDM".to_owned(),
            away_team: "COL".to_owned(),
            players: vec![
                make_entry(99, "EDM", "C", "20:00"),
                make_entry(88, "EDM", "L", "18:00"),
            ],
        };

        // player_id 42 does not appear in any boxscore
        let result = build_profile_from_boxscores(42, &[boxscore]);
        assert!(result.is_none());
    }

    /// Extra: parse_toi_mmss handles malformed input without panicking.
    #[test]
    fn l0_parse_toi_bad_input_returns_zero() {
        assert_eq!(parse_toi_mmss(""), 0);
        assert_eq!(parse_toi_mmss("abc"), 0);
        assert_eq!(parse_toi_mmss("20:30"), 20 * 60 + 30);
        assert_eq!(parse_toi_mmss("0:00"), 0);
    }
}
