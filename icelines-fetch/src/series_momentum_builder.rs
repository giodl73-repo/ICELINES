//! Phase Conn Smythe C.1 — projection from `PlayoffSeries` →
//! `SeriesMomentum`.
//!
//! Lives here (not icelines-core) because `PlayoffSeries` is the
//! input type and that's an icelines-fetch shape. The output schema
//! lives in `icelines_core::series_momentum`.

use chrono::NaiveDate;
use icelines_core::identity::GameId;
use icelines_core::model::{Season, TeamAbbr};
#[cfg_attr(not(test), allow(unused_imports))]
use icelines_core::series_momentum::{SeriesGameResult, SeriesLeader, SeriesMomentum};

use crate::nhl_api::{PlayoffGameResult, PlayoffRound, PlayoffSeries};

/// Project one `PlayoffSeries` into a `SeriesMomentum`. Reads
/// `top_seed_wins` / `bottom_seed_wins` for the series state and
/// walks `series.games` for OT counts + the last result. When the
/// API doesn't supply per-game logs (live current-season bracket)
/// the OT count is 0 and `last_result` is `None`.
pub fn compute_series_momentum(
    season: Season,
    round: &PlayoffRound,
    series: &PlayoffSeries,
) -> SeriesMomentum {
    let top_wins = series.top_seed_wins;
    let bottom_wins = series.bottom_seed_wins;
    let played = top_wins + bottom_wins;
    let leader = SeriesMomentum::leader_from(top_wins, bottom_wins);
    let games_remaining = SeriesMomentum::games_remaining_for(top_wins, bottom_wins);
    let series_complete = SeriesMomentum::is_complete_for(top_wins, bottom_wins);
    let home_advantage = SeriesMomentum::home_advantage_for(top_wins, bottom_wins);

    let ot_games = series
        .games
        .iter()
        .filter(|g| series_after_indicates_ot(&g.series_after))
        .count() as u8;

    let last_result = series.games.last().and_then(build_last_result);

    let winner_abbrev = series
        .winner_abbrev
        .as_ref()
        .map(|w| TeamAbbr(w.clone()));

    SeriesMomentum {
        series_letter: series.letter.clone().unwrap_or_else(|| "?".into()),
        season,
        round: round.round_number,
        round_label: round.label.clone(),
        top_seed_abbrev: TeamAbbr(series.top_seed_abbrev.clone()),
        bottom_seed_abbrev: TeamAbbr(series.bottom_seed_abbrev.clone()),
        top_seed_wins: top_wins,
        bottom_seed_wins: bottom_wins,
        games_played: played,
        games_remaining,
        leader,
        last_result,
        ot_games,
        home_advantage,
        series_complete,
        winner_abbrev,
    }
}

/// Per-game OT detection from the bundled v1 shape, which doesn't
/// expose `lastPeriodType` directly. Returns `true` when any
/// telltale OT marker shows up in the `series_after` summary string
/// (e.g. "FLA 2-1 · OT"). Best-effort; a richer source would parse
/// the actual `gameOutcome.lastPeriodType` field.
fn series_after_indicates_ot(s: &str) -> bool {
    let upper = s.to_uppercase();
    upper.contains("OT") || upper.contains("SO") || upper.contains("OVERTIME")
}

fn build_last_result(g: &PlayoffGameResult) -> Option<SeriesGameResult> {
    let date = NaiveDate::parse_from_str(&g.date, "%Y-%m-%d").ok()?;
    let (winner, w_score, l_score) = if g.home_score > g.away_score {
        (
            TeamAbbr(g.home_abbrev.clone()),
            g.home_score as u32,
            g.away_score as u32,
        )
    } else if g.away_score > g.home_score {
        (
            TeamAbbr(g.away_abbrev.clone()),
            g.away_score as u32,
            g.home_score as u32,
        )
    } else {
        // Tied games shouldn't appear in playoff logs (NHL plays
        // unlimited OT until someone scores), but be defensive.
        return None;
    };
    Some(SeriesGameResult {
        // The bundled `PlayoffGameResult` doesn't carry the NHL
        // game_id; we surface 0 as a sentinel and let downstream
        // consumers (live game detail) substitute when they have it.
        game_id: GameId(0),
        date,
        winner,
        winner_score: w_score,
        loser_score: l_score,
        ot: series_after_indicates_ot(&g.series_after),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_round() -> PlayoffRound {
        PlayoffRound {
            round_number: 1,
            label: "First Round".into(),
            series: vec![],
        }
    }

    fn fixture_series(
        letter: &str,
        top: &str,
        bot: &str,
        top_wins: u8,
        bot_wins: u8,
        games: Vec<PlayoffGameResult>,
    ) -> PlayoffSeries {
        PlayoffSeries {
            letter: Some(letter.into()),
            top_seed_abbrev: top.into(),
            top_seed_name: top.into(),
            top_seed_wins: top_wins,
            top_seed_rank: None,
            bottom_seed_abbrev: bot.into(),
            bottom_seed_name: bot.into(),
            bottom_seed_wins: bot_wins,
            bottom_seed_rank: None,
            winner_abbrev: if top_wins == 4 {
                Some(top.into())
            } else if bot_wins == 4 {
                Some(bot.into())
            } else {
                None
            },
            conference: None,
            games,
        }
    }

    fn game(date: &str, home: &str, away: &str, hs: u8, as_: u8, suffix: &str) -> PlayoffGameResult {
        PlayoffGameResult {
            date: date.into(),
            home_abbrev: home.into(),
            away_abbrev: away.into(),
            home_score: hs,
            away_score: as_,
            series_after: suffix.into(),
            goals: vec![],
        }
    }

    #[test]
    fn l0_conn_smythe_c1_builder_empty_series() {
        let s = fixture_series("A", "EDM", "LAK", 0, 0, vec![]);
        let m = compute_series_momentum(Season(20252026), &fixture_round(), &s);
        assert_eq!(m.top_seed_wins, 0);
        assert_eq!(m.bottom_seed_wins, 0);
        assert_eq!(m.games_played, 0);
        assert_eq!(m.games_remaining, 7);
        assert_eq!(m.leader, SeriesLeader::Tied);
        assert_eq!(m.ot_games, 0);
        assert!(!m.series_complete);
        assert!(m.last_result.is_none());
    }

    #[test]
    fn l0_conn_smythe_c1_builder_top_seed_leads_with_one_ot() {
        let games = vec![
            game("2026-04-21", "EDM", "LAK", 4, 2, "EDM 1-0"),
            game("2026-04-23", "EDM", "LAK", 3, 4, "tied 1-1 · OT"),
            game("2026-04-25", "LAK", "EDM", 2, 3, "EDM 2-1"),
        ];
        let s = fixture_series("A", "EDM", "LAK", 2, 1, games);
        let m = compute_series_momentum(Season(20252026), &fixture_round(), &s);
        assert_eq!(m.leader, SeriesLeader::Top);
        assert_eq!(m.games_played, 3);
        assert_eq!(m.games_remaining, 4);
        assert_eq!(m.ot_games, 1, "G2 marked OT in series_after");
        let last = m.last_result.expect("G3 last result");
        assert_eq!(last.winner.0, "EDM");
        assert_eq!(last.winner_score, 3);
        assert_eq!(last.loser_score, 2);
        assert!(!last.ot);
    }

    #[test]
    fn l0_conn_smythe_c1_builder_complete_series_records_winner() {
        let games = vec![
            game("2026-04-21", "EDM", "LAK", 4, 2, "EDM 1-0"),
            game("2026-04-23", "EDM", "LAK", 5, 1, "EDM 2-0"),
            game("2026-04-25", "LAK", "EDM", 1, 3, "EDM 3-0"),
            game("2026-04-27", "LAK", "EDM", 2, 4, "EDM wins 4-0"),
        ];
        let s = fixture_series("A", "EDM", "LAK", 4, 0, games);
        let m = compute_series_momentum(Season(20252026), &fixture_round(), &s);
        assert!(m.series_complete);
        assert_eq!(m.games_remaining, 0);
        assert_eq!(m.winner_abbrev.unwrap().0, "EDM");
    }

    #[test]
    fn l0_conn_smythe_c1_builder_no_games_log_falls_back_gracefully() {
        // Live current-season bracket: wins counts populated, games[] empty.
        let s = fixture_series("A", "EDM", "LAK", 2, 1, vec![]);
        let m = compute_series_momentum(Season(20252026), &fixture_round(), &s);
        assert_eq!(m.leader, SeriesLeader::Top);
        assert_eq!(m.ot_games, 0, "no games log → 0 OT detected");
        assert!(m.last_result.is_none());
    }

    #[test]
    fn l0_conn_smythe_c1_builder_ot_marker_case_insensitive() {
        let games = vec![
            game("2026-04-21", "A", "B", 3, 2, "1-0 ot"),
            game("2026-04-23", "A", "B", 2, 1, "2-0 OVERTIME"),
            game("2026-04-25", "B", "A", 4, 3, "tied 2-2 · so"),
        ];
        let s = fixture_series("Z", "A", "B", 2, 1, games);
        let m = compute_series_momentum(Season(20252026), &fixture_round(), &s);
        assert_eq!(m.ot_games, 3, "all three OT/SO/OVERTIME markers count");
    }
}
