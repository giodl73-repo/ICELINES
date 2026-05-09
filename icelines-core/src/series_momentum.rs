//! Phase Conn Smythe C.1 — playoff series momentum schemas.
//!
//! Pure data shapes describing the live state of a playoff series:
//! who's leading, how many games remain, OT count, last result,
//! home advantage. The builder that turns a `PlayoffSeries` (from
//! `icelines-fetch`) into a `SeriesMomentum` lives in icelines-cli
//! (mirrors how `FavoritesView` / `compute_favorites_view` split:
//! schema here, orchestration where the data primitives live).
//!
//! "Momentum" is intentionally just narrative state, not a stat
//! prediction. The view answers "what happened, who's winning,
//! what's next" — not "who will win." The latter is xWin% modeling
//! work explicitly deferred from Conn Smythe.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::identity::GameId;
use crate::model::{Season, TeamAbbr};

/// Live state of a single playoff series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesMomentum {
    pub series_letter: String,
    pub season: Season,
    pub round: u8,
    pub round_label: String,
    pub top_seed_abbrev: TeamAbbr,
    pub bottom_seed_abbrev: TeamAbbr,
    pub top_seed_wins: u8,
    pub bottom_seed_wins: u8,
    pub games_played: u8,
    /// Best-of-7 series — `7 - games_played` clamped at 0 once the
    /// series concludes. NHL has used best-of-7 for every round
    /// since 1939.
    pub games_remaining: u8,
    pub leader: SeriesLeader,
    pub last_result: Option<SeriesGameResult>,
    /// How many games in the series went to OT. Tracked because OT
    /// counts are a strong narrative signal — "BOS won 3 of 5 in OT"
    /// reads differently than "BOS won 3 of 5 in regulation".
    pub ot_games: u8,
    /// True iff the next scheduled game is at the higher-seeded team
    /// (top seed). Used for the "next game" banner.
    pub home_advantage: bool,
    /// True once one side has reached 4 wins.
    pub series_complete: bool,
    pub winner_abbrev: Option<TeamAbbr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesLeader {
    Top,
    Bottom,
    Tied,
}

/// One game's result within a series — the "last_result" the
/// momentum view surfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesGameResult {
    pub game_id: GameId,
    pub date: NaiveDate,
    pub winner: TeamAbbr,
    /// `(winner_score, loser_score)`. Loser is implied — the team in
    /// the series that isn't the winner.
    pub winner_score: u32,
    pub loser_score: u32,
    /// True when the game ended in OT or SO. Combined for renderer
    /// brevity ("OT") — caller can split via `last_period_type`
    /// when the boxscore is available.
    pub ot: bool,
}

impl SeriesMomentum {
    /// Compute the leader given top + bottom seed wins.
    pub fn leader_from(top_wins: u8, bottom_wins: u8) -> SeriesLeader {
        if top_wins > bottom_wins {
            SeriesLeader::Top
        } else if bottom_wins > top_wins {
            SeriesLeader::Bottom
        } else {
            SeriesLeader::Tied
        }
    }

    /// Best-of-7 → games-remaining = `7 - games_played`, floored at 0.
    pub fn games_remaining_for(top_wins: u8, bottom_wins: u8) -> u8 {
        let played = top_wins + bottom_wins;
        if top_wins == 4 || bottom_wins == 4 {
            0
        } else {
            7u8.saturating_sub(played)
        }
    }

    /// True iff one side has reached 4 wins.
    pub fn is_complete_for(top_wins: u8, bottom_wins: u8) -> bool {
        top_wins == 4 || bottom_wins == 4
    }

    /// The next-game home-advantage rule for a 2-2-1-1-1 best-of-7
    /// (NHL's standard format since 2014):
    /// - Games 1, 2, 5, 7 at the higher seed (top seed by series
    ///   convention)
    /// - Games 3, 4, 6 at the lower seed
    ///
    /// Returns `true` when the next game (game `played + 1`) is at
    /// the top seed. Returns `false` once the series is complete
    /// (no next game).
    pub fn home_advantage_for(top_wins: u8, bottom_wins: u8) -> bool {
        if Self::is_complete_for(top_wins, bottom_wins) {
            return false;
        }
        let next_game = top_wins + bottom_wins + 1;
        matches!(next_game, 1 | 2 | 5 | 7)
    }

    /// Render a one-line summary like "EDM leads 2-1 · 1 OT" or
    /// "Tied 2-2 · series begins" / "BOS wins 4-2".
    pub fn summary_line(&self) -> String {
        let ot_suffix = if self.ot_games > 0 {
            format!(" · {} OT", self.ot_games)
        } else {
            String::new()
        };
        if self.series_complete {
            let winner = self
                .winner_abbrev
                .as_ref()
                .map(|w| w.0.as_str())
                .unwrap_or("?");
            return format!(
                "{} wins {}-{}{}",
                winner,
                self.top_seed_wins.max(self.bottom_seed_wins),
                self.top_seed_wins.min(self.bottom_seed_wins),
                ot_suffix,
            );
        }
        match self.leader {
            SeriesLeader::Top => format!(
                "{} leads {}-{}{}",
                self.top_seed_abbrev.0, self.top_seed_wins, self.bottom_seed_wins, ot_suffix
            ),
            SeriesLeader::Bottom => format!(
                "{} leads {}-{}{}",
                self.bottom_seed_abbrev.0, self.bottom_seed_wins, self.top_seed_wins, ot_suffix
            ),
            SeriesLeader::Tied => {
                if self.games_played == 0 {
                    format!(
                        "{} vs {} · series begins",
                        self.top_seed_abbrev.0, self.bottom_seed_abbrev.0
                    )
                } else {
                    format!(
                        "Tied {}-{}{}",
                        self.top_seed_wins, self.bottom_seed_wins, ot_suffix
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_momentum(
        top_wins: u8,
        bottom_wins: u8,
        ot_games: u8,
        complete: bool,
    ) -> SeriesMomentum {
        let played = top_wins + bottom_wins;
        SeriesMomentum {
            series_letter: "A".into(),
            season: Season(20252026),
            round: 1,
            round_label: "First Round".into(),
            top_seed_abbrev: TeamAbbr("EDM".into()),
            bottom_seed_abbrev: TeamAbbr("LAK".into()),
            top_seed_wins: top_wins,
            bottom_seed_wins: bottom_wins,
            games_played: played,
            games_remaining: SeriesMomentum::games_remaining_for(top_wins, bottom_wins),
            leader: SeriesMomentum::leader_from(top_wins, bottom_wins),
            last_result: None,
            ot_games,
            home_advantage: SeriesMomentum::home_advantage_for(top_wins, bottom_wins),
            series_complete: complete,
            winner_abbrev: if complete {
                Some(TeamAbbr(if top_wins == 4 { "EDM" } else { "LAK" }.into()))
            } else {
                None
            },
        }
    }

    #[test]
    fn l0_conn_smythe_c1_leader_from_truth_table() {
        assert_eq!(SeriesMomentum::leader_from(0, 0), SeriesLeader::Tied);
        assert_eq!(SeriesMomentum::leader_from(2, 1), SeriesLeader::Top);
        assert_eq!(SeriesMomentum::leader_from(1, 3), SeriesLeader::Bottom);
        assert_eq!(SeriesMomentum::leader_from(2, 2), SeriesLeader::Tied);
    }

    #[test]
    fn l0_conn_smythe_c1_games_remaining_truth_table() {
        // 0-0 → 7 remaining
        assert_eq!(SeriesMomentum::games_remaining_for(0, 0), 7);
        // 2-1 → 4 remaining (3 played)
        assert_eq!(SeriesMomentum::games_remaining_for(2, 1), 4);
        // 4-2 → series over, 0 remaining
        assert_eq!(SeriesMomentum::games_remaining_for(4, 2), 0);
        // 4-3 → series over even though 7 games played
        assert_eq!(SeriesMomentum::games_remaining_for(4, 3), 0);
    }

    #[test]
    fn l0_conn_smythe_c1_home_advantage_2211_format() {
        // 2-2-1-1-1 NHL format:
        //   Game 1 (0-0 played): top seed home → true
        //   Game 2 (1-0 or 0-1): top seed home → true
        //   Game 3 (2-0 or 0-2 or 1-1): bottom seed home → false
        //   Game 4: bottom seed → false
        //   Game 5: top → true
        //   Game 6: bottom → false
        //   Game 7: top → true
        assert!(SeriesMomentum::home_advantage_for(0, 0), "G1 at top");
        assert!(SeriesMomentum::home_advantage_for(1, 0), "G2 at top");
        assert!(!SeriesMomentum::home_advantage_for(2, 0), "G3 at bottom");
        assert!(!SeriesMomentum::home_advantage_for(2, 1), "G4 at bottom");
        assert!(SeriesMomentum::home_advantage_for(2, 2), "G5 at top");
        assert!(!SeriesMomentum::home_advantage_for(3, 2), "G6 at bottom");
        assert!(SeriesMomentum::home_advantage_for(3, 3), "G7 at top");
        // Series over → no next game.
        assert!(!SeriesMomentum::home_advantage_for(4, 2));
    }

    #[test]
    fn l0_conn_smythe_c1_summary_line_in_progress() {
        let m = fixture_momentum(2, 1, 1, false);
        assert_eq!(m.summary_line(), "EDM leads 2-1 · 1 OT");

        let m2 = fixture_momentum(0, 2, 0, false);
        assert_eq!(m2.summary_line(), "LAK leads 2-0");

        let m3 = fixture_momentum(0, 0, 0, false);
        assert_eq!(m3.summary_line(), "EDM vs LAK · series begins");

        let m4 = fixture_momentum(2, 2, 2, false);
        assert_eq!(m4.summary_line(), "Tied 2-2 · 2 OT");
    }

    #[test]
    fn l0_conn_smythe_c1_summary_line_complete() {
        let m = fixture_momentum(4, 2, 1, true);
        assert_eq!(m.summary_line(), "EDM wins 4-2 · 1 OT");

        let m2 = fixture_momentum(3, 4, 0, true);
        assert_eq!(m2.summary_line(), "LAK wins 4-3");
    }

    #[test]
    fn l0_conn_smythe_c1_serde_round_trip() {
        let m = fixture_momentum(2, 1, 1, false);
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("\"top_seed_wins\":2"));
        assert!(s.contains("\"leader\":\"top\""));
        let back: SeriesMomentum = serde_json::from_str(&s).unwrap();
        assert_eq!(back.top_seed_wins, 2);
        assert_eq!(back.leader, SeriesLeader::Top);
    }
}
