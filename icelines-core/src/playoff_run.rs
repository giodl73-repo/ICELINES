//! Phase Conn Smythe C.2 — per-player playoff-run aggregates.
//!
//! Pure data shape + a single composition helper. Aggregation
//! orchestration (walking the Boxscore manifest, filtering to
//! `game_type=3` games inside the playoff window) lives in
//! icelines-cli where DataStore is reachable. This module keeps the
//! schema + the per-row arithmetic primitives here so they're
//! testable without disk.
//!
//! Naming: "Playoff run" instead of "Cup run" because not every
//! season's runs reach the Cup. The Conn Smythe trophy is awarded
//! to the playoff MVP regardless of which round their team
//! exited — same scope here.

use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayoffRunSummary {
    pub player_id: PlayerId,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub sog: u32,
    pub hits: u32,
    pub blocks: u32,
    pub pim: u32,
    pub toi_seconds: u32,
    /// Populated only when this player appears in any GoalieLine
    /// across the run. `None` for skater-only runs.
    pub goalie_record: Option<GoalieRunRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoalieRunRecord {
    pub starts: u32,
    pub wins: u32,
    pub losses: u32,
    pub ot_losses: u32,
    pub saves: u32,
    pub shots_against: u32,
    pub goals_against: u32,
    /// Computed: 1 - GA / SA. `None` when no shots faced.
    pub save_pct: Option<f32>,
    /// Computed: GA / (TOI / 60). `None` when no TOI recorded.
    pub gaa: Option<f32>,
    pub shutouts: u32,
}

impl PlayoffRunSummary {
    pub fn new(pid: PlayerId) -> Self {
        Self {
            player_id: pid,
            ..Default::default()
        }
    }

    /// Add one game's skater stat line to the running totals.
    #[allow(clippy::too_many_arguments)] // mirrors the boxscore SkaterLine fields 1-to-1
    pub fn add_skater_game(
        &mut self,
        goals: u32,
        assists: u32,
        plus_minus: i32,
        sog: u32,
        hits: u32,
        blocks: u32,
        pim: u32,
        toi_seconds: u32,
    ) {
        self.games += 1;
        self.goals += goals;
        self.assists += assists;
        self.points += goals + assists;
        self.plus_minus += plus_minus;
        self.sog += sog;
        self.hits += hits;
        self.blocks += blocks;
        self.pim += pim;
        self.toi_seconds += toi_seconds;
    }

    /// Add one goalie appearance. `decision` ∈ {"W","L","OTL", None}.
    /// Goalie totals are kept in the `goalie_record` sub-struct so
    /// skater rollups stay clean for two-way (rare) cases.
    pub fn add_goalie_game(
        &mut self,
        saves: u32,
        shots_against: u32,
        decision: Option<&str>,
        toi_seconds: u32,
    ) {
        let record = self.goalie_record.get_or_insert_with(GoalieRunRecord::default);
        record.starts += 1;
        record.saves += saves;
        record.shots_against += shots_against;
        let ga = shots_against.saturating_sub(saves);
        record.goals_against += ga;
        match decision {
            Some("W") => record.wins += 1,
            Some("L") => record.losses += 1,
            Some("OTL") => record.ot_losses += 1,
            _ => {}
        }
        if ga == 0 && shots_against > 0 {
            record.shutouts += 1;
        }
        // Re-derive ratios from the running totals so the saved
        // values stay consistent with the inputs.
        if record.shots_against > 0 {
            record.save_pct = Some(
                (record.shots_against - record.goals_against) as f32
                    / record.shots_against as f32,
            );
        }
        let total_toi = self.toi_seconds + toi_seconds;
        if total_toi > 0 {
            record.gaa = Some(record.goals_against as f32 / (total_toi as f32 / 3600.0));
        }
        // `toi_seconds` on the parent struct accumulates regardless
        // of skater vs goalie so the run total is honest.
        self.toi_seconds += toi_seconds;
    }

    /// One-line narrative summary like
    /// "12G 18A 30P · +5 · 14 GP · 47 SOG"
    /// (or a goalie-style line when goalie_record is populated).
    pub fn summary_line(&self) -> String {
        match &self.goalie_record {
            Some(g) => {
                let sv = g.save_pct.unwrap_or(0.0);
                let gaa = g.gaa.unwrap_or(0.0);
                format!(
                    "{}-{}-{} · {} GP · SV%{:.3} · GAA {:.2}{}",
                    g.wins,
                    g.losses,
                    g.ot_losses,
                    g.starts,
                    sv,
                    gaa,
                    if g.shutouts > 0 {
                        format!(" · {} SO", g.shutouts)
                    } else {
                        String::new()
                    }
                )
            }
            None => format!(
                "{}G {}A {}P · {:+} · {} GP · {} SOG",
                self.goals, self.assists, self.points, self.plus_minus, self.games, self.sog
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_conn_smythe_c2_skater_aggregation_two_games() {
        let mut run = PlayoffRunSummary::new(PlayerId(8478402));
        run.add_skater_game(1, 2, 2, 4, 2, 0, 0, 1334);
        run.add_skater_game(2, 1, 1, 5, 3, 1, 2, 1422);
        assert_eq!(run.games, 2);
        assert_eq!(run.goals, 3);
        assert_eq!(run.assists, 3);
        assert_eq!(run.points, 6);
        assert_eq!(run.plus_minus, 3);
        assert_eq!(run.sog, 9);
        assert_eq!(run.hits, 5);
        assert_eq!(run.blocks, 1);
        assert_eq!(run.toi_seconds, 1334 + 1422);
        assert!(run.goalie_record.is_none(), "no goalie record for skater");
    }

    #[test]
    fn l0_conn_smythe_c2_goalie_aggregation_with_shutout() {
        let mut run = PlayoffRunSummary::new(PlayerId(8475670));
        run.add_goalie_game(32, 32, Some("W"), 3600); // shutout win
        run.add_goalie_game(28, 31, Some("L"), 3600); // 3 GA in 60min
        let g = run.goalie_record.as_ref().expect("goalie record");
        assert_eq!(g.starts, 2);
        assert_eq!(g.wins, 1);
        assert_eq!(g.losses, 1);
        assert_eq!(g.ot_losses, 0);
        assert_eq!(g.saves, 60);
        assert_eq!(g.shots_against, 63);
        assert_eq!(g.goals_against, 3);
        assert_eq!(g.shutouts, 1);
        let sv = g.save_pct.unwrap();
        assert!((sv - 60.0 / 63.0).abs() < 1e-6, "got {sv}");
        let gaa = g.gaa.unwrap();
        // 3 GA in 7200 secs (2h) = 3 / 2.0 hours = 1.5 GAA
        assert!((gaa - 1.5).abs() < 1e-6, "got {gaa}");
    }

    #[test]
    fn l0_conn_smythe_c2_summary_line_skater_vs_goalie() {
        let mut s = PlayoffRunSummary::new(PlayerId(8478402));
        s.add_skater_game(1, 2, 2, 4, 0, 0, 0, 1200);
        s.add_skater_game(2, 1, 1, 5, 0, 0, 0, 1300);
        let summary = s.summary_line();
        assert!(summary.contains("3G 3A 6P"), "got: {summary}");
        assert!(summary.contains("+3"));
        assert!(summary.contains("2 GP"));

        let mut g = PlayoffRunSummary::new(PlayerId(8475670));
        g.add_goalie_game(32, 32, Some("W"), 3600);
        g.add_goalie_game(28, 30, Some("OTL"), 3700);
        let summary = g.summary_line();
        assert!(summary.contains("1-0-1"), "W-L-OTL: {summary}");
        assert!(summary.contains("2 GP"));
        assert!(summary.contains("SV%"));
    }

    #[test]
    fn l0_conn_smythe_c2_empty_run_renders_zeros() {
        let run = PlayoffRunSummary::new(PlayerId(1));
        assert_eq!(run.games, 0);
        let s = run.summary_line();
        assert!(s.contains("0G 0A 0P"));
    }

    #[test]
    fn l0_conn_smythe_c2_serde_round_trip() {
        let mut s = PlayoffRunSummary::new(PlayerId(8478402));
        s.add_skater_game(1, 2, 2, 4, 2, 0, 0, 1200);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"player_id\":8478402"));
        let back: PlayoffRunSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.points, 3);
        assert_eq!(back.games, 1);
    }
}
