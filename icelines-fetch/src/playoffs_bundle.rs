//! Historical playoffs bundle (Phase 8c).
//!
//! Each season ships an optional `playoffs.json` next to its `bios.json` /
//! `stats.json`. The file captures the bracket — rounds, series, and per-game
//! results — frozen at the end of the season. The live NHL API
//! (`/v1/playoff-bracket/{year}`) does not include per-game game logs, so
//! historical brackets are pre-built and bundled.
//!
//! Resolution order (Phase 8c):
//! 1. `playoffs.json` from the bundled binary or installed bundle
//! 2. Live API `/v1/playoff-bracket/{year}` (current season only)
//! 3. Empty bracket / error message in the UI
//!
//! See `design/specs/playoffs.md` for the full schema and the fallback
//! hierarchy resolved from the WIRE blocker.

use serde::{Deserialize, Serialize};

use crate::nhl_api::{PlayoffBracket, PlayoffGameResult, PlayoffGoal, PlayoffRound, PlayoffSeries};

// ── Bundle JSON shape ─────────────────────────────────────────────────────────

/// Top-level shape of `playoffs.json`. Hand-authored historical bundles use
/// `snake_case` keys to read naturally as JSON. Conversion to the rendered
/// `PlayoffBracket` happens via `to_bracket`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayoffsBundle {
    pub season:      String,                  // "19931994"
    pub champion:    Option<String>,          // team abbrev that won the Cup
    pub conn_smythe: Option<String>,          // playoff MVP
    pub rounds:      Vec<PlayoffsBundleRound>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayoffsBundleRound {
    pub round:  u8,                            // 1..=4
    #[serde(default)]
    pub label:  Option<String>,                // "First Round" — derived if omitted
    pub series: Vec<PlayoffsBundleSeries>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayoffsBundleSeries {
    pub top_seed:        String,               // abbrev — required
    pub bottom_seed:     String,               // abbrev — required
    pub winner:          Option<String>,       // abbrev — None = incomplete
    /// Per-game count actually played. Used to derive wins when not directly
    /// provided in `top_wins` / `bottom_wins`.
    #[serde(default)]
    pub games:           Option<u8>,
    #[serde(default)]
    pub top_wins:         Option<u8>,
    #[serde(default)]
    pub bottom_wins:      Option<u8>,
    #[serde(default)]
    pub top_seed_name:    Option<String>,      // "New York Rangers"
    #[serde(default)]
    pub bottom_seed_name: Option<String>,
    #[serde(default)]
    pub top_seed_rank:    Option<String>,      // "A1", "WC2", or freeform "1"
    #[serde(default)]
    pub bottom_seed_rank: Option<String>,
    #[serde(default)]
    pub conference:       Option<String>,      // "Eastern" | "Western"
    #[serde(default)]
    pub letter:           Option<String>,      // stable key — auto-assigned if absent
    #[serde(default)]
    pub results:          Vec<PlayoffsBundleGame>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayoffsBundleGame {
    pub date:         String,                  // "1994-06-14"
    pub home:         String,                  // abbrev
    pub away:         String,                  // abbrev
    pub home_score:   u8,
    pub away_score:   u8,
    /// e.g. "NYR 1-0", "tied 2-2". If absent, computed at render time.
    #[serde(default)]
    pub series_after: Option<String>,
    #[serde(default)]
    pub goals:        Vec<PlayoffsBundleGoal>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayoffsBundleGoal {
    pub scorer: String,
    pub team:   String,
}

// ── Conversion ────────────────────────────────────────────────────────────────

impl PlayoffsBundle {
    /// Convert into the renderable `PlayoffBracket` shape used by the TUI.
    /// Wins are inferred from per-game results when not declared explicitly.
    /// Series letters are auto-assigned ("A", "B", …) in declaration order
    /// across the whole bracket when missing.
    pub fn to_bracket(&self) -> PlayoffBracket {
        let mut next_letter = b'A';
        let mut rounds = Vec::with_capacity(self.rounds.len());
        for r in &self.rounds {
            let label = r.label.clone().unwrap_or_else(|| default_round_label(r.round));
            let mut series_out = Vec::with_capacity(r.series.len());
            for s in &r.series {
                let letter = s.letter.clone().unwrap_or_else(|| {
                    let c = next_letter as char;
                    next_letter += 1;
                    c.to_string()
                });
                series_out.push(bundle_series_to_playoff_series(s, letter));
            }
            rounds.push(PlayoffRound {
                round_number: r.round,
                label,
                series: series_out,
            });
        }
        rounds.sort_by_key(|r| r.round_number);
        PlayoffBracket {
            season:        self.season.clone(),
            current_round: rounds.iter().map(|r| r.round_number).max(),
            rounds,
        }
    }
}

fn default_round_label(round: u8) -> String {
    match round {
        1 => "First Round".to_owned(),
        2 => "Second Round".to_owned(),
        3 => "Conference Final".to_owned(),
        4 => "Stanley Cup Final".to_owned(),
        n => format!("Round {n}"),
    }
}

fn bundle_series_to_playoff_series(s: &PlayoffsBundleSeries, letter: String) -> PlayoffSeries {
    // Compute wins. Priority: explicit top_wins/bottom_wins, then count from results,
    // then fall back to (4, games-4) when only `games` and `winner` are set.
    let (top_wins, bot_wins) = derive_wins(s);
    let games = s.results.iter().enumerate().map(|(i, g)| {
        let series_after = g.series_after.clone().unwrap_or_else(|| {
            // Recompute from the running tally up to and including this game.
            let mut t = 0u8;
            let mut b = 0u8;
            for prior in s.results.iter().take(i + 1) {
                let (top, bot) = side_winners(prior, &s.top_seed, &s.bottom_seed);
                t += top as u8;
                b += bot as u8;
            }
            format_series_after(&s.top_seed, &s.bottom_seed, t, b)
        });
        PlayoffGameResult {
            date:         g.date.clone(),
            home_abbrev:  g.home.clone(),
            away_abbrev:  g.away.clone(),
            home_score:   g.home_score,
            away_score:   g.away_score,
            series_after,
            goals: g.goals.iter().map(|gl| PlayoffGoal {
                scorer: gl.scorer.clone(),
                team:   gl.team.clone(),
            }).collect(),
        }
    }).collect();
    PlayoffSeries {
        letter:             Some(letter),
        top_seed_abbrev:    s.top_seed.clone(),
        top_seed_name:      s.top_seed_name.clone().unwrap_or_else(|| s.top_seed.clone()),
        top_seed_wins:      top_wins,
        top_seed_rank:      s.top_seed_rank.clone(),
        bottom_seed_abbrev: s.bottom_seed.clone(),
        bottom_seed_name:   s.bottom_seed_name.clone().unwrap_or_else(|| s.bottom_seed.clone()),
        bottom_seed_wins:   bot_wins,
        bottom_seed_rank:   s.bottom_seed_rank.clone(),
        winner_abbrev:      s.winner.clone(),
        conference:         s.conference.clone(),
        games,
    }
}

fn derive_wins(s: &PlayoffsBundleSeries) -> (u8, u8) {
    if let (Some(t), Some(b)) = (s.top_wins, s.bottom_wins) {
        return (t, b);
    }
    if !s.results.is_empty() {
        let mut t = 0u8;
        let mut b = 0u8;
        for g in &s.results {
            let (top, bot) = side_winners(g, &s.top_seed, &s.bottom_seed);
            t += top as u8;
            b += bot as u8;
        }
        return (t, b);
    }
    // No game log + no explicit win counts — fall back to {games, winner}.
    if let (Some(games), Some(w)) = (s.games, s.winner.as_deref()) {
        let losing = games.saturating_sub(4);
        if w == s.top_seed {
            return (4, losing);
        } else if w == s.bottom_seed {
            return (losing, 4);
        }
    }
    (0, 0)
}

fn side_winners(g: &PlayoffsBundleGame, top: &str, bot: &str) -> (bool, bool) {
    let winner = if g.home_score > g.away_score {
        &g.home
    } else {
        &g.away
    };
    (winner == top, winner == bot)
}

fn format_series_after(top: &str, bot: &str, t: u8, b: u8) -> String {
    if t == 4 {
        format!("{top} wins {t}-{b}")
    } else if b == 4 {
        format!("{bot} wins {b}-{t}")
    } else if t > b {
        format!("{top} leads {t}-{b}")
    } else if b > t {
        format!("{bot} leads {b}-{t}")
    } else {
        format!("tied {t}-{b}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_two_games_nyr_3_0() -> PlayoffsBundleSeries {
        PlayoffsBundleSeries {
            top_seed:         "NYR".to_owned(),
            bottom_seed:      "NYI".to_owned(),
            winner:           None,
            games:            None,
            top_wins:         None,
            bottom_wins:      None,
            top_seed_name:    Some("New York Rangers".to_owned()),
            bottom_seed_name: Some("New York Islanders".to_owned()),
            top_seed_rank:    Some("1".to_owned()),
            bottom_seed_rank: Some("8".to_owned()),
            conference:       Some("Eastern".to_owned()),
            letter:           None,
            results: vec![
                PlayoffsBundleGame {
                    date: "1994-04-17".to_owned(),
                    home: "NYR".to_owned(), away: "NYI".to_owned(),
                    home_score: 6, away_score: 0,
                    series_after: None, goals: vec![],
                },
                PlayoffsBundleGame {
                    date: "1994-04-18".to_owned(),
                    home: "NYR".to_owned(), away: "NYI".to_owned(),
                    home_score: 6, away_score: 0,
                    series_after: None, goals: vec![],
                },
            ],
        }
    }

    #[test]
    fn l0_bundle_roundtrip_json() {
        let json = r#"{
          "season": "19931994",
          "champion": "NYR",
          "conn_smythe": "Brian Leetch",
          "rounds": [
            {"round": 1, "series": [
              {"top_seed": "NYR", "bottom_seed": "NYI",
               "winner": "NYR", "games": 4, "results": []}
            ]}
          ]
        }"#;
        let b: PlayoffsBundle = serde_json::from_str(json).expect("parses");
        assert_eq!(b.season, "19931994");
        assert_eq!(b.champion.as_deref(), Some("NYR"));
        assert_eq!(b.rounds.len(), 1);
        assert_eq!(b.rounds[0].series.len(), 1);
    }

    #[test]
    fn l0_to_bracket_assigns_letters_in_declaration_order() {
        let bundle = PlayoffsBundle {
            season: "19931994".to_owned(),
            champion: None, conn_smythe: None,
            rounds: vec![PlayoffsBundleRound {
                round: 1, label: None,
                series: vec![
                    PlayoffsBundleSeries {
                        top_seed: "NYR".to_owned(), bottom_seed: "NYI".to_owned(),
                        winner: Some("NYR".to_owned()), games: Some(4),
                        top_wins: None, bottom_wins: None,
                        top_seed_name: None, bottom_seed_name: None,
                        top_seed_rank: None, bottom_seed_rank: None,
                        conference: None, letter: None, results: vec![],
                    },
                    PlayoffsBundleSeries {
                        top_seed: "NJD".to_owned(), bottom_seed: "BUF".to_owned(),
                        winner: Some("NJD".to_owned()), games: Some(7),
                        top_wins: None, bottom_wins: None,
                        top_seed_name: None, bottom_seed_name: None,
                        top_seed_rank: None, bottom_seed_rank: None,
                        conference: None, letter: None, results: vec![],
                    },
                ],
            }],
        };
        let br = bundle.to_bracket();
        assert_eq!(br.rounds[0].series[0].letter.as_deref(), Some("A"));
        assert_eq!(br.rounds[0].series[1].letter.as_deref(), Some("B"));
    }

    #[test]
    fn l0_derive_wins_from_results() {
        let s = fixture_two_games_nyr_3_0();
        let (t, b) = derive_wins(&s);
        assert_eq!((t, b), (2, 0), "two NYR wins should give (2,0)");
    }

    #[test]
    fn l0_derive_wins_falls_back_to_games_winner() {
        let s = PlayoffsBundleSeries {
            top_seed: "NYR".to_owned(), bottom_seed: "VAN".to_owned(),
            winner: Some("NYR".to_owned()), games: Some(7),
            top_wins: None, bottom_wins: None,
            top_seed_name: None, bottom_seed_name: None,
            top_seed_rank: None, bottom_seed_rank: None,
            conference: None, letter: None, results: vec![],
        };
        assert_eq!(derive_wins(&s), (4, 3));
    }

    #[test]
    fn l0_derive_wins_explicit_overrides_results() {
        let mut s = fixture_two_games_nyr_3_0();
        s.top_wins = Some(4);
        s.bottom_wins = Some(2);
        // Explicit values win, even when results would say (2,0).
        assert_eq!(derive_wins(&s), (4, 2));
    }

    #[test]
    fn l0_to_bracket_recomputes_series_after_when_missing() {
        let s = fixture_two_games_nyr_3_0();
        let bundle = PlayoffsBundle {
            season: "19931994".to_owned(), champion: None, conn_smythe: None,
            rounds: vec![PlayoffsBundleRound {
                round: 1, label: None, series: vec![s],
            }],
        };
        let br = bundle.to_bracket();
        let sr = &br.rounds[0].series[0];
        assert_eq!(sr.games.len(), 2);
        assert_eq!(sr.games[0].series_after, "NYR leads 1-0");
        assert_eq!(sr.games[1].series_after, "NYR leads 2-0");
    }

    #[test]
    fn l0_to_bracket_default_round_labels() {
        let bundle = PlayoffsBundle {
            season: "19931994".to_owned(), champion: None, conn_smythe: None,
            rounds: vec![
                PlayoffsBundleRound { round: 1, label: None, series: vec![] },
                PlayoffsBundleRound { round: 2, label: None, series: vec![] },
                PlayoffsBundleRound { round: 3, label: None, series: vec![] },
                PlayoffsBundleRound { round: 4, label: None, series: vec![] },
            ],
        };
        let br = bundle.to_bracket();
        assert_eq!(br.rounds[0].label, "First Round");
        assert_eq!(br.rounds[1].label, "Second Round");
        assert_eq!(br.rounds[2].label, "Conference Final");
        assert_eq!(br.rounds[3].label, "Stanley Cup Final");
    }
}
