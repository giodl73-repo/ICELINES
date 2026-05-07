//! Phase Conn Smythe C.3 — live in-progress game detail schema.
//!
//! Pure data describing one game's current scoreboard state. The
//! existing `nhl_api::Boxscore` shape is the natural input —
//! `LiveGameDetail` is the trimmed renderer-friendly view that
//! powers the new web `/game/:id` route + the JSON envelope.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::favorites::GameState;
use crate::identity::GameId;
use crate::model::TeamAbbr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveGameDetail {
    pub game_id: GameId,
    pub date: NaiveDate,
    pub away: TeamAbbr,
    pub home: TeamAbbr,
    pub away_score: u32,
    pub home_score: u32,
    /// 1, 2, 3, OT (4), SO (5). 0 means pre-game.
    pub period: u8,
    pub period_label: String,
    pub state: GameState,
    pub goal_summary: Vec<GoalSummary>,
    /// (away_starter, home_starter) — `None` slots when the API
    /// doesn't expose the starting goalie or there's no record.
    pub starting_goalies: (Option<String>, Option<String>),
    /// Best-effort heuristic — fires when a team has scored ≥2 in a
    /// period AND the scoring goalie is no longer in the boxscore's
    /// per-team goalies list. A future refinement can read the
    /// API's pulled-goalie indicator directly.
    pub goalie_pulled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSummary {
    pub period: u8,
    pub time_in_period: String,
    pub team: TeamAbbr,
    pub scorer: String,
}

impl LiveGameDetail {
    /// Map NHL period codes to a human label.
    pub fn period_label_for(period: u8, state: GameState) -> String {
        match (period, state) {
            (0, _) => "Pre-game".into(),
            (1, _) => "1st".into(),
            (2, _) => "2nd".into(),
            (3, _) => "3rd".into(),
            (4, GameState::Live | GameState::Pre) => "OT".into(),
            (4, _) => "OT".into(),
            (5, _) => "SO".into(),
            (n, _) => format!("Period {n}"),
        }
    }

    /// True iff the score is a regulation tie (any state). Used by
    /// downstream renderers that highlight one-goal vs blowout
    /// games differently.
    pub fn is_one_goal_game(&self) -> bool {
        self.away_score.abs_diff(self.home_score) <= 1
    }

    /// One-line summary like "BOS 3 – 2 EDM · 2nd · 2 goals (BOS 1, EDM 1)"
    pub fn summary_line(&self) -> String {
        format!(
            "{} {} – {} {} · {} · {} goal(s)",
            self.away.0,
            self.away_score,
            self.home_score,
            self.home.0,
            self.period_label,
            self.goal_summary.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(period: u8, state: GameState, away: u32, home: u32) -> LiveGameDetail {
        LiveGameDetail {
            game_id: GameId(2025020342),
            date: NaiveDate::from_ymd_opt(2026, 5, 6).unwrap(),
            away: TeamAbbr("BOS".into()),
            home: TeamAbbr("EDM".into()),
            away_score: away,
            home_score: home,
            period,
            period_label: LiveGameDetail::period_label_for(period, state),
            state,
            goal_summary: Vec::new(),
            starting_goalies: (None, None),
            goalie_pulled: false,
        }
    }

    #[test]
    fn l0_conn_smythe_c3_period_label_truth_table() {
        assert_eq!(
            LiveGameDetail::period_label_for(0, GameState::Pre),
            "Pre-game"
        );
        assert_eq!(LiveGameDetail::period_label_for(1, GameState::Live), "1st");
        assert_eq!(LiveGameDetail::period_label_for(2, GameState::Live), "2nd");
        assert_eq!(LiveGameDetail::period_label_for(3, GameState::Live), "3rd");
        assert_eq!(LiveGameDetail::period_label_for(4, GameState::Live), "OT");
        assert_eq!(LiveGameDetail::period_label_for(5, GameState::Final), "SO");
        assert_eq!(
            LiveGameDetail::period_label_for(7, GameState::Live),
            "Period 7"
        );
    }

    #[test]
    fn l0_conn_smythe_c3_is_one_goal_game() {
        assert!(fixture(2, GameState::Live, 1, 0).is_one_goal_game());
        assert!(fixture(2, GameState::Live, 3, 4).is_one_goal_game());
        assert!(fixture(2, GameState::Live, 2, 2).is_one_goal_game());
        assert!(!fixture(2, GameState::Live, 5, 1).is_one_goal_game());
    }

    #[test]
    fn l0_conn_smythe_c3_summary_line() {
        let mut g = fixture(2, GameState::Live, 3, 2);
        g.goal_summary.push(GoalSummary {
            period: 1,
            time_in_period: "12:34".into(),
            team: TeamAbbr("BOS".into()),
            scorer: "Marchand".into(),
        });
        let s = g.summary_line();
        assert!(s.contains("BOS 3 – 2 EDM"), "got: {s}");
        assert!(s.contains("2nd"));
        assert!(s.contains("1 goal"));
    }

    #[test]
    fn l0_conn_smythe_c3_serde_round_trip() {
        let g = fixture(3, GameState::Final, 4, 3);
        let s = serde_json::to_string(&g).unwrap();
        let back: LiveGameDetail = serde_json::from_str(&s).unwrap();
        assert_eq!(back.away_score, 4);
        assert_eq!(back.period, 3);
    }
}
