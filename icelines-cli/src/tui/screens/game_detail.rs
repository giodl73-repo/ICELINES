//! Game detail screen — boxscore goals, goalies, series context.
//!
//! Renders against the `BoxscoreCache` populated by
//! `crate::tui::tonight::maybe_fetch_boxscore`. The series-results panel
//! (for playoff games) is derived from the games already in the active
//! `tonight_cache` entry, so no additional fetch is required.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::tonight::{lookup_boxscore, BoxscoreState};
use icelines_fetch::nhl_api::{Boxscore, Goal, ScheduledGame};

pub fn render(f: &mut Frame, app: &App, area: Rect, game_id: u64) {
    // Look up the schedule entry for this game (gives us series context).
    let game: Option<ScheduledGame> = {
        use crate::tui::tonight::{lookup, TonightState};
        let state = lookup(&app.tonight_cache, &app.scores_date);
        match state {
            TonightState::Loaded(games) => {
                games.iter().find(|g| g.game_id == game_id).cloned()
            }
            _ => None,
        }
    };

    let title = match &game {
        Some(g) => {
            let aw = g.away_score.unwrap_or(0);
            let hw = g.home_score.unwrap_or(0);
            let state = g.game_state.as_deref().unwrap_or("");
            let last  = g.last_period.as_deref().unwrap_or("");
            let suffix = match (state, last) {
                ("FINAL"|"OFF", "OT") => " · OT".to_owned(),
                ("FINAL"|"OFF", "SO") => " · SO".to_owned(),
                ("FINAL"|"OFF", _)    => " · Final".to_owned(),
                ("LIVE"|"CRIT", _)    => " · LIVE".to_owned(),
                _ => String::new(),
            };
            format!(
                " {} {aw} – {hw} {}{suffix} · Esc back ",
                g.away_abbrev, g.home_abbrev,
            )
        }
        None => format!(" Game {game_id} · Esc back "),
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let state = lookup_boxscore(&app.boxscore_cache, game_id);
    match state {
        BoxscoreState::Idle | BoxscoreState::Loading => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(
                        "  Fetching boxscore…",
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                inner,
            );
        }
        BoxscoreState::Error(e) => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(
                        format!("  Boxscore unavailable: {e}"),
                        Style::default().fg(Color::Red),
                    ),
                    Line::from(""),
                    Line::styled(
                        "  Press r to retry.",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                inner,
            );
        }
        BoxscoreState::Loaded(b) => {
            render_loaded(f, inner, &b, game.as_ref());
        }
    }
}

fn render_loaded(f: &mut Frame, area: Rect, b: &Boxscore, sched: Option<&ScheduledGame>) {
    let dim   = Style::default().fg(Color::DarkGray);
    let gold  = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Series context (playoffs only)
    if let Some(g) = sched {
        if g.is_playoff() {
            if let Some(label) = g.series_label() {
                lines.push(Line::styled(format!("  {label}"), gold));
            }
            if let (Some(aw), Some(hw)) = (g.away_wins, g.home_wins) {
                let ctx = match aw.cmp(&hw) {
                    std::cmp::Ordering::Greater =>
                        format!("  {} leads series {}-{}", g.away_abbrev, aw, hw),
                    std::cmp::Ordering::Less =>
                        format!("  {} leads series {}-{}", g.home_abbrev, hw, aw),
                    std::cmp::Ordering::Equal =>
                        format!("  Series tied {}-{}", aw, hw),
                };
                lines.push(Line::styled(ctx, dim));
            }
            lines.push(Line::from(""));
        }
    }

    // Goals
    lines.push(Line::styled("  GOALS", gold));
    if b.goals.is_empty() {
        lines.push(Line::styled("    (none recorded)", dim));
    } else {
        for goal in &b.goals {
            lines.push(Line::from(format_goal_row(goal)));
        }
    }
    lines.push(Line::from(""));

    // Goalies
    if !b.goalies.is_empty() {
        lines.push(Line::styled("  GOALTENDERS", gold));
        for gl in &b.goalies {
            let decision = gl.decision.as_deref().map(|d| format!(" ({d})")).unwrap_or_default();
            lines.push(Line::from(format!(
                "    {:<22} ({})  {} saves / {} shots{}",
                gl.player_name, gl.team_abbrev, gl.saves, gl.shots, decision,
            )));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::styled("  Esc to return to scores", dim));
    f.render_widget(Paragraph::new(lines), area);
}

fn format_goal_row(goal: &Goal) -> String {
    let period_label = match (goal.period, goal.period_type.as_str()) {
        (_, "OT") => "OT".to_owned(),
        (_, "SO") => "SO".to_owned(),
        (n, _)    => ordinal(n),
    };
    let assists = match (&goal.assist1_name, &goal.assist2_name) {
        (Some(a), Some(b)) => format!(" — {a}, {b}"),
        (Some(a), None)    => format!(" — {a}"),
        _ => String::new(),
    };
    format!(
        "    {:<3}  {:<6}  {:<22} ({}){assists}   {}-{}",
        period_label,
        goal.time_in_period,
        goal.scorer_name,
        goal.scorer_team,
        goal.away_score,
        goal.home_score,
    )
}

fn ordinal(n: u8) -> String {
    match n {
        1 => "1st".to_owned(),
        2 => "2nd".to_owned(),
        3 => "3rd".to_owned(),
        n => format!("{n}th"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::tui::tonight::{BoxscoreState, TonightState};
    use icelines_fetch::nhl_api::{Boxscore, Goal, GoalieLine, ScheduledGame};

    #[allow(clippy::too_many_arguments)]
    fn fixture_goal(period: u8, ptype: &str, time: &str, scorer: &str, team: &str,
                    a1: Option<&str>, a2: Option<&str>, aw: u8, hm: u8) -> Goal {
        Goal {
            period,
            period_type:    ptype.to_owned(),
            time_in_period: time.to_owned(),
            scorer_name:    scorer.to_owned(),
            scorer_team:    team.to_owned(),
            assist1_name:   a1.map(str::to_owned),
            assist2_name:   a2.map(str::to_owned),
            away_score:     aw,
            home_score:     hm,
        }
    }

    fn fixture_boxscore(game_id: u64) -> Boxscore {
        Boxscore {
            game_id,
            away_abbrev: "NYR".to_owned(),
            home_abbrev: "WSH".to_owned(),
            away_score:  2,
            home_score:  3,
            game_state:  Some("FINAL".to_owned()),
            last_period: Some("OT".to_owned()),
            goals: vec![
                fixture_goal(1, "REG", "08:14", "Ovechkin", "WSH", Some("Kuznetsov"), Some("Carlson"), 0, 1),
                fixture_goal(1, "REG", "17:55", "Zibanejad", "NYR", Some("Trocheck"),  Some("Fox"),       1, 1),
                fixture_goal(2, "REG", "11:44", "Panarin",   "NYR", Some("Trocheck"),  None,              2, 1),
                fixture_goal(3, "REG", "19:58", "Strome",    "WSH", Some("Backstrom"), Some("Jensen"),    2, 2),
                fixture_goal(4, "OT",  "03:22", "Wilson",    "WSH", Some("Ovechkin"),  None,              2, 3),
            ],
            goalies: vec![
                GoalieLine {
                    player_name: "Shesterkin".to_owned(),
                    team_abbrev: "NYR".to_owned(),
                    saves: 32, shots: 35,
                    decision: Some("L".to_owned()),
                },
                GoalieLine {
                    player_name: "Lindgren".to_owned(),
                    team_abbrev: "WSH".to_owned(),
                    saves: 28, shots: 30,
                    decision: Some("W".to_owned()),
                },
            ],
        }
    }

    fn fixture_playoff_game(game_id: u64) -> ScheduledGame {
        ScheduledGame {
            game_id,
            date:           "2026-04-28".to_owned(),
            game_type:      3,
            away_abbrev:    "NYR".to_owned(),
            away_name:      "New York Rangers".to_owned(),
            home_abbrev:    "WSH".to_owned(),
            home_name:      "Washington Capitals".to_owned(),
            start_time_utc: "2026-04-28T23:05:00Z".to_owned(),
            away_score:     Some(2),
            home_score:     Some(3),
            game_state:     Some("FINAL".to_owned()),
            last_period:    Some("OT".to_owned()),
            series_game:    Some("Game 5".to_owned()),
            away_wins:      Some(2),
            home_wins:      Some(3),
        }
    }

    fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn render_to_text(app: &App, game_id: u64) -> String {
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render(f, app, area, game_id);
        }).unwrap();
        buffer_text(term.backend().buffer())
    }

    #[test]
    fn l0_render_game_detail_idle_shows_fetching() {
        let app = App::new(false);
        let text = render_to_text(&app, 12345);
        assert!(text.contains("Fetching"), "idle state should show fetching, got:\n{text}");
    }

    #[test]
    fn l0_render_game_detail_loaded_shows_goals_and_goalies() {
        let app = App::new(false);
        app.boxscore_cache.lock().unwrap()
            .insert(12345, BoxscoreState::Loaded(fixture_boxscore(12345)));
        let text = render_to_text(&app, 12345);

        // Every scorer must appear
        for scorer in &["Ovechkin", "Zibanejad", "Panarin", "Strome", "Wilson"] {
            assert!(text.contains(scorer), "scorer {scorer} missing, got:\n{text}");
        }
        // OT label
        assert!(text.contains("OT"), "OT label must appear");
        // Goalies + saves/shots
        assert!(text.contains("Shesterkin"), "away goalie must appear");
        assert!(text.contains("Lindgren"),   "home goalie must appear");
        assert!(text.contains("32 saves") || text.contains("32"), "save count must appear");
        // Section headers
        assert!(text.contains("GOALS"), "GOALS header missing");
        assert!(text.contains("GOALTENDERS"), "GOALTENDERS header missing");
    }

    #[test]
    fn l0_render_game_detail_playoff_shows_series_context() {
        let app = App::new(false);
        // Seed schedule entry so series context is available
        app.tonight_cache.lock().unwrap()
            .insert(String::new(), TonightState::Loaded(vec![fixture_playoff_game(12345)]));
        // Seed boxscore
        app.boxscore_cache.lock().unwrap()
            .insert(12345, BoxscoreState::Loaded(fixture_boxscore(12345)));

        let text = render_to_text(&app, 12345);
        // Series label from schedule entry
        assert!(text.contains("Game 5"), "series game label must appear, got:\n{text}");
        assert!(text.contains("leads series") || text.contains("Series tied"),
            "series context line must appear, got:\n{text}");
    }

    #[test]
    fn l0_render_game_detail_error_shows_retry_hint() {
        let app = App::new(false);
        app.boxscore_cache.lock().unwrap()
            .insert(12345, BoxscoreState::Error("network down".to_owned()));
        let text = render_to_text(&app, 12345);
        assert!(text.contains("Boxscore unavailable"), "error message missing, got:\n{text}");
        assert!(text.contains("retry"), "retry hint missing");
    }

    #[test]
    fn l0_render_game_detail_empty_goals_shows_none_recorded() {
        let app = App::new(false);
        let mut bs = fixture_boxscore(12345);
        bs.goals.clear();
        bs.goalies.clear();
        app.boxscore_cache.lock().unwrap()
            .insert(12345, BoxscoreState::Loaded(bs));
        let text = render_to_text(&app, 12345);
        assert!(text.contains("none recorded"),
            "empty goals must show '(none recorded)', got:\n{text}");
    }

    #[test]
    fn l0_render_game_detail_title_shows_score_and_ot() {
        let app = App::new(false);
        app.tonight_cache.lock().unwrap()
            .insert(String::new(), TonightState::Loaded(vec![fixture_playoff_game(12345)]));
        app.boxscore_cache.lock().unwrap()
            .insert(12345, BoxscoreState::Loaded(fixture_boxscore(12345)));
        let text = render_to_text(&app, 12345);
        // Title shows "NYR 2 – 3 WSH · OT"
        assert!(text.contains("NYR") && text.contains("WSH"));
        assert!(text.contains("OT"));
    }
}
