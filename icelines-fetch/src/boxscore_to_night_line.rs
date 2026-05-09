//! Phase Foster +19 — boxscore body → `SkaterNightLine` / `GoalieNightLine`.
//!
//! Pure projection from the NHL `/v1/gamecenter/{id}/boxscore`
//! response into the per-night row schemas defined in
//! `icelines_core::favorites`. Foster.3 persisted the raw JSON
//! body; Foster +3's `DataStore::load_boxscore_raw` reads it back;
//! this module turns it into rendered content.
//!
//! The function takes a parsed `Boxscore` (from `nhl_api::parse_boxscore`)
//! rather than a JSON `Value` so the test surface is the strongly-
//! typed input shape and we don't duplicate the parse logic.
//!
//! Hits/blocks/PIM/takeaways/giveaways pass through `gate_finalized`
//! per SCOUT B2 — NHL API zero-defaults those fields mid-game and we
//! must not record "0 hits tonight" until the boxscore finalizes.

use icelines_core::entity::EntityRef;
use icelines_core::favorites::{GameResult, GameState, GoalieNightLine, HomeAway, SkaterNightLine};
use icelines_core::identity::PlayerId;
use icelines_core::model::TeamAbbr;

use crate::nhl_api::Boxscore;

/// Extract the favorited skater's per-night line from a parsed
/// boxscore. Returns `None` when the player isn't on either team's
/// roster for this game.
pub fn extract_skater_line(boxscore: &Boxscore, player_id: u32) -> Option<SkaterNightLine> {
    // Try home first, then away — the same loop body composes the
    // SkaterNightLine for either side.
    let (line, is_home) = boxscore
        .home_skaters
        .iter()
        .find(|s| s.player_id == player_id)
        .map(|s| (s, true))
        .or_else(|| {
            boxscore
                .away_skaters
                .iter()
                .find(|s| s.player_id == player_id)
                .map(|s| (s, false))
        })?;

    let game_state = parse_game_state(boxscore.game_state.as_deref());
    let team_abbr = if is_home {
        boxscore.home_abbrev.clone()
    } else {
        boxscore.away_abbrev.clone()
    };
    let opponent_abbr = if is_home {
        boxscore.away_abbrev.clone()
    } else {
        boxscore.home_abbrev.clone()
    };
    let team_score = if is_home {
        boxscore.home_score as u32
    } else {
        boxscore.away_score as u32
    };
    let opponent_score = if is_home {
        boxscore.away_score as u32
    } else {
        boxscore.home_score as u32
    };
    let mut result = SkaterNightLine::classify_result(team_score, opponent_score, game_state);
    // OT/SO loss override — when the boxscore says lastPeriodType is
    // OT or SO and our skater's team lost, upgrade Loss → OtLoss.
    if matches!(result, GameResult::Loss) {
        if let Some(p) = boxscore.last_period.as_deref() {
            if matches!(p, "OT" | "SO") {
                result = GameResult::OtLoss;
            }
        }
    }

    Some(SkaterNightLine {
        player: EntityRef::Player(PlayerId(line.player_id)),
        team: TeamAbbr(team_abbr),
        opponent: TeamAbbr(opponent_abbr),
        home_or_away: if is_home {
            HomeAway::Home
        } else {
            HomeAway::Away
        },
        team_score,
        opponent_score,
        result,
        goals: line.goals,
        assists: line.assists,
        points: line.goals + line.assists,
        plus_minus: line.plus_minus,
        shots: Some(line.sog),
        hits: SkaterNightLine::gate_finalized(line.hits, game_state),
        blocks: SkaterNightLine::gate_finalized(line.blocked_shots, game_state),
        pim: SkaterNightLine::gate_finalized(line.pim, game_state),
        takeaways: SkaterNightLine::gate_finalized(line.takeaways, game_state),
        giveaways: SkaterNightLine::gate_finalized(line.giveaways, game_state),
        toi_seconds: Some(line.toi_seconds),
        // Power-play splits aren't yet broken out by the
        // nhl_api::parse_boxscore path — TODO when SkaterLine grows
        // pp_goals / pp_assists / sh_goals fields. Until then the
        // dashboard renders 0 for these (unambiguous from the
        // Option<u32> fields above which use None to mean "unknown").
        power_play_goals: 0,
        power_play_assists: 0,
        shorthanded_goals: 0,
        game_state,
    })
}

/// Extract the favorited goalie's per-night line. Returns `None`
/// when the player isn't a goalie in this boxscore. Phase Foster +24:
/// prefers PID match when GoalieLine.player_id is non-zero, falls
/// back to name-substring (case-insensitive) for legacy data.
pub fn extract_goalie_line(
    boxscore: &Boxscore,
    player_id: u32,
    display_name: &str,
) -> Option<GoalieNightLine> {
    // Foster +24 — PID match wins. Name match is the fallback for
    // boxscores parsed before the playerId field landed in
    // GoalieLine, or for newly-favorited goalies whose PID didn't
    // resolve via the bundled bios.
    let g = boxscore
        .goalies
        .iter()
        .find(|g| g.player_id != 0 && g.player_id == player_id)
        .or_else(|| {
            let needle = display_name.trim().to_lowercase();
            if needle.is_empty() {
                return None;
            }
            boxscore
                .goalies
                .iter()
                .find(|g| g.player_name.to_lowercase().contains(&needle))
        })?;

    let game_state = parse_game_state(boxscore.game_state.as_deref());
    let is_home = g.team_abbrev == boxscore.home_abbrev;
    let opponent_abbr = if is_home {
        boxscore.away_abbrev.clone()
    } else {
        boxscore.home_abbrev.clone()
    };
    let team_score = if is_home {
        boxscore.home_score as u32
    } else {
        boxscore.away_score as u32
    };
    let opponent_score = if is_home {
        boxscore.away_score as u32
    } else {
        boxscore.home_score as u32
    };

    let saves = g.saves;
    let shots_against = g.shots;
    let goals_against = shots_against.saturating_sub(saves);
    let save_pct = GoalieNightLine::compute_save_pct(saves, shots_against);
    // We don't have TOI for goalies in this parse path — assume a
    // full 60 minutes for the GAA computation when the game is
    // finalized; otherwise pass 0.
    let assumed_toi_secs = if game_state.is_finalized() { 3600 } else { 0 };
    let gaa = GoalieNightLine::compute_gaa(goals_against, assumed_toi_secs);

    let decision = match g.decision.as_deref() {
        Some("W") => Some(icelines_core::favorites::Decision::Win),
        Some("L") => Some(icelines_core::favorites::Decision::Loss),
        Some("OTL") => Some(icelines_core::favorites::Decision::OtLoss),
        _ => None,
    };

    Some(GoalieNightLine {
        // Foster +24 — use the parsed player_id. Falls back to the
        // caller-supplied PID when the boxscore JSON didn't surface
        // one (legacy format or future API drift).
        player: EntityRef::Player(PlayerId(if g.player_id != 0 {
            g.player_id
        } else {
            player_id
        })),
        team: TeamAbbr(g.team_abbrev.clone()),
        opponent: TeamAbbr(opponent_abbr),
        home_or_away: if is_home {
            HomeAway::Home
        } else {
            HomeAway::Away
        },
        team_score,
        opponent_score,
        games_started: g.decision.is_some(), // proxy: only starters get decisions
        decision,
        saves,
        shots_against,
        goals_against,
        save_pct,
        gaa,
        toi_seconds: if game_state.is_finalized() {
            Some(assumed_toi_secs)
        } else {
            None
        },
        shutout: goals_against == 0 && shots_against > 0 && game_state.is_finalized(),
        game_state,
    })
}

fn parse_game_state(s: Option<&str>) -> GameState {
    match s {
        Some("FUT") => GameState::Fut,
        Some("PRE") => GameState::Pre,
        Some("LIVE") | Some("CRIT") => GameState::Live,
        Some("FINAL") => GameState::Final,
        Some("OFF") => GameState::Off,
        _ => GameState::Fut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nhl_api::{Boxscore, GoalieLine, SkaterLine};

    fn skater(id: u32, team: &str, goals: u32, assists: u32, pm: i32) -> SkaterLine {
        SkaterLine {
            player_id: id,
            player_name: format!("Player {id}"),
            team_abbrev: team.into(),
            position: "C".into(),
            toi_seconds: 1200,
            goals,
            assists,
            plus_minus: pm,
            sog: 4,
            hits: 2,
            blocked_shots: 1,
            takeaways: 1,
            giveaways: 0,
            pim: 0,
        }
    }

    fn goalie(name: &str, team: &str, saves: u32, shots: u32, dec: Option<&str>) -> GoalieLine {
        GoalieLine {
            player_id: 0,
            player_name: name.into(),
            team_abbrev: team.into(),
            saves,
            shots,
            decision: dec.map(str::to_owned),
        }
    }

    fn box_with_skater(home_skater: SkaterLine, state: &str, last: Option<&str>) -> Boxscore {
        Boxscore {
            game_id: 2025020342,
            away_abbrev: "CGY".into(),
            home_abbrev: "EDM".into(),
            away_score: 3,
            home_score: 7,
            game_state: Some(state.into()),
            last_period: last.map(str::to_owned),
            goals: vec![],
            goalies: vec![],
            away_skaters: vec![],
            home_skaters: vec![home_skater],
        }
    }

    #[test]
    fn l1_foster_plus19_skater_home_winner() {
        let line = skater(8478402, "EDM", 1, 2, 2);
        let bs = box_with_skater(line, "FINAL", Some("REG"));
        let nl = extract_skater_line(&bs, 8478402).expect("found");
        assert_eq!(nl.team.0, "EDM");
        assert_eq!(nl.opponent.0, "CGY");
        assert!(matches!(nl.home_or_away, HomeAway::Home));
        assert_eq!(nl.team_score, 7);
        assert_eq!(nl.opponent_score, 3);
        assert!(matches!(nl.result, GameResult::Win));
        assert_eq!(nl.goals, 1);
        assert_eq!(nl.assists, 2);
        assert_eq!(nl.points, 3);
        assert_eq!(nl.plus_minus, 2);
        // Final state → hits/blocks gate to Some.
        assert_eq!(nl.hits, Some(2));
        assert_eq!(nl.blocks, Some(1));
        assert!(matches!(nl.game_state, GameState::Final));
    }

    #[test]
    fn l1_foster_plus19_skater_in_progress_gates_hits_to_none() {
        let line = skater(8478402, "EDM", 0, 0, 0);
        // Mid-game; hits = 0 in the API but we must record None.
        let mut bs = box_with_skater(line, "LIVE", None);
        bs.home_skaters[0].hits = 0;
        let nl = extract_skater_line(&bs, 8478402).expect("found");
        assert!(matches!(nl.game_state, GameState::Live));
        assert_eq!(nl.hits, None, "mid-game zero must not be recorded");
        assert_eq!(nl.blocks, None);
        assert!(matches!(nl.result, GameResult::InProgress));
    }

    #[test]
    fn l1_foster_plus19_skater_ot_loss_promoted() {
        // Skater on the away team; home wins 4-3 in OT.
        let mut bs = Boxscore {
            game_id: 2025020342,
            away_abbrev: "CGY".into(),
            home_abbrev: "EDM".into(),
            away_score: 3,
            home_score: 4,
            game_state: Some("FINAL".into()),
            last_period: Some("OT".into()),
            goals: vec![],
            goalies: vec![],
            away_skaters: vec![skater(8470000, "CGY", 0, 1, -1)],
            home_skaters: vec![],
        };
        let nl = extract_skater_line(&bs, 8470000).expect("found");
        assert!(
            matches!(nl.result, GameResult::OtLoss),
            "OT loss must be promoted from plain Loss, got {:?}",
            nl.result
        );
        assert!(matches!(nl.home_or_away, HomeAway::Away));
        // Sanity: SO last-period also promotes to OtLoss.
        bs.last_period = Some("SO".into());
        let nl_so = extract_skater_line(&bs, 8470000).unwrap();
        assert!(matches!(nl_so.result, GameResult::OtLoss));
    }

    #[test]
    fn l1_foster_plus19_skater_not_in_either_roster_returns_none() {
        let bs = box_with_skater(skater(1, "EDM", 0, 0, 0), "FINAL", None);
        assert!(
            extract_skater_line(&bs, 999999).is_none(),
            "missing PID returns None"
        );
    }

    #[test]
    fn l1_foster_plus19_goalie_finalized_shutout() {
        let bs = Boxscore {
            game_id: 1,
            away_abbrev: "CGY".into(),
            home_abbrev: "EDM".into(),
            away_score: 0,
            home_score: 4,
            game_state: Some("FINAL".into()),
            last_period: Some("REG".into()),
            goals: vec![],
            goalies: vec![goalie("Stuart Skinner", "EDM", 32, 32, Some("W"))],
            away_skaters: vec![],
            home_skaters: vec![],
        };
        let gl = extract_goalie_line(&bs, 0, "Stuart Skinner").expect("found");
        assert_eq!(gl.saves, 32);
        assert_eq!(gl.shots_against, 32);
        assert_eq!(gl.goals_against, 0);
        assert!((gl.save_pct - 1.0).abs() < 1e-6);
        assert!(gl.shutout, "0 GA on >0 SA at FINAL = shutout");
        assert!(matches!(
            gl.decision,
            Some(icelines_core::favorites::Decision::Win)
        ));
        assert!(matches!(gl.home_or_away, HomeAway::Home));
    }

    #[test]
    fn l1_foster_plus19_goalie_partial_name_match() {
        let bs = Boxscore {
            game_id: 1,
            away_abbrev: "CGY".into(),
            home_abbrev: "EDM".into(),
            away_score: 1,
            home_score: 3,
            game_state: Some("FINAL".into()),
            last_period: Some("REG".into()),
            goals: vec![],
            goalies: vec![goalie("Stuart Skinner", "EDM", 30, 31, Some("W"))],
            away_skaters: vec![],
            home_skaters: vec![],
        };
        // Substring match — case-insensitive.
        let gl = extract_goalie_line(&bs, 0, "skinner").expect("partial match");
        assert_eq!(gl.saves, 30);
    }

    #[test]
    fn l1_foster_plus24_goalie_pid_match_beats_name() {
        // Two goalies with the same surname; PID match disambiguates.
        let mut g1 = goalie("Stuart Skinner", "EDM", 30, 31, Some("W"));
        g1.player_id = 8479973;
        let mut g2 = goalie("J.T. Skinner", "CGY", 18, 22, Some("L"));
        g2.player_id = 8475670;
        let bs = Boxscore {
            game_id: 1,
            away_abbrev: "CGY".into(),
            home_abbrev: "EDM".into(),
            away_score: 1,
            home_score: 3,
            game_state: Some("FINAL".into()),
            last_period: Some("REG".into()),
            goals: vec![],
            goalies: vec![g1, g2],
            away_skaters: vec![],
            home_skaters: vec![],
        };
        // Name "skinner" alone would hit Stuart first; ask for the
        // Calgary one by PID.
        let gl = extract_goalie_line(&bs, 8475670, "skinner").expect("found by PID");
        assert_eq!(gl.team.0, "CGY", "PID match wins over name match");
        assert_eq!(gl.saves, 18);
    }

    #[test]
    fn l1_foster_plus19_goalie_no_decision_relief() {
        let bs = Boxscore {
            game_id: 1,
            away_abbrev: "CGY".into(),
            home_abbrev: "EDM".into(),
            away_score: 5,
            home_score: 3,
            game_state: Some("FINAL".into()),
            last_period: Some("REG".into()),
            goals: vec![],
            goalies: vec![goalie("Calvin Pickard", "EDM", 8, 9, None)],
            away_skaters: vec![],
            home_skaters: vec![],
        };
        let gl = extract_goalie_line(&bs, 0, "Pickard").expect("found");
        assert!(gl.decision.is_none(), "relief = no decision");
        assert!(!gl.games_started, "no decision → not the starter");
    }
}
