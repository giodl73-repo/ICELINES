//! Playoffs tab — list-style bracket and per-series detail.
//!
//! v1 rendering: rounds as horizontal sections, series as rows. Each row shows
//! seeding (when available), team abbrevs, current series score, and winner
//! marker once decided. ASCII bracket art is deferred to v2.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::playoffs::{playoff_year_for_season, PlayoffsState};
use icelines_fetch::nhl_api::{PlayoffBracket, PlayoffGameResult, PlayoffSeries};

// ── Bracket view ──────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(
        " Playoffs · {}  ·  ↑↓:series  ←→:round  Enter:detail  r:retry  y:season ",
        season_label(&app.active_season),
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Off-season / unparseable season → simple message
    let year = match playoff_year_for_season(&app.active_season) {
        Some(y) => y,
        None => {
            render_message(f, inner, "Selected season has no playoff data.");
            return;
        }
    };

    let state = {
        let map = app.playoffs_cache.lock().unwrap();
        map.get(&year).cloned().unwrap_or(PlayoffsState::Idle)
    };

    match state {
        PlayoffsState::Idle => render_message(f, inner, "Loading bracket…"),
        PlayoffsState::Loading => render_message(f, inner, "Fetching playoff bracket…"),
        PlayoffsState::Error(e) => render_error(f, inner, &e),
        PlayoffsState::Loaded(b) if b.is_empty() => {
            render_off_season(f, inner, &b, &app.active_season);
        }
        PlayoffsState::Loaded(b) => {
            render_bracket(f, inner, app, &b);
        }
    }
}

fn render_message(f: &mut Frame, area: Rect, msg: &str) {
    let dim = Style::default().fg(Color::DarkGray);
    f.render_widget(
        Paragraph::new(vec![Line::from(""), Line::styled(format!("  {msg}"), dim)]),
        area,
    );
}

fn render_error(f: &mut Frame, area: Rect, msg: &str) {
    let red = Style::default().fg(Color::Red);
    let dim = Style::default().fg(Color::DarkGray);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::styled("  Bracket data unavailable", red),
            Line::styled(format!("  ({msg})"), dim),
            Line::from(""),
            Line::styled("  Press r to retry  ·  y to switch seasons", dim),
        ]),
        area,
    );
}

fn render_off_season(f: &mut Frame, area: Rect, b: &PlayoffBracket, active_season: &str) {
    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    // Live API often returns the bracket payload with an empty `season`
    // field during the regular season. Fall back to the active season the
    // user picked so the message always carries a real label.
    let season_label = if b.season.is_empty() {
        active_season
    } else {
        b.season.as_str()
    };
    let pretty = self::season_label(season_label);
    let lines = vec![
        Line::from(""),
        Line::styled("  Playoffs not yet active for this season.", gold),
        Line::from(""),
        Line::styled(
            format!("  Season {pretty} — bracket data has not been published."),
            dim,
        ),
        Line::from(""),
        Line::styled(
            "  Once first-round matchups are set, this tab will populate automatically.",
            dim,
        ),
        Line::styled("  Press y to browse historical playoffs.", dim),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn render_bracket(f: &mut Frame, area: Rect, app: &App, b: &PlayoffBracket) {
    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let cyan = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let n_rounds = b.rounds.len();
    let active_round = app.playoffs_round.min(n_rounds.saturating_sub(1));

    // Header strip showing all round labels with the active one highlighted.
    let mut header_spans: Vec<ratatui::text::Span> = Vec::new();
    for (i, r) in b.rounds.iter().enumerate() {
        let active = i == active_round;
        let label = format!(" {} ", r.label);
        let style = if active { cyan } else { dim };
        header_spans.push(ratatui::text::Span::styled(label, style));
        if i + 1 < n_rounds {
            header_spans.push(ratatui::text::Span::styled("│", dim));
        }
    }
    let header = ratatui::text::Line::from(header_spans);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);
    lines.push(Line::styled(format!("  {}", "─".repeat(70)), dim));

    let round = &b.rounds[active_round];
    if round.series.is_empty() {
        lines.push(Line::styled(
            format!("  No series in {} yet.", round.label),
            dim,
        ));
    } else {
        let active_series = app
            .playoffs_series
            .min(round.series.len().saturating_sub(1));
        let mut last_conf: Option<String> = None;
        for (i, s) in round.series.iter().enumerate() {
            // Conference section header when it changes
            if s.conference != last_conf {
                if let Some(conf) = &s.conference {
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }
                    lines.push(Line::styled(
                        format!("  {} CONFERENCE", conf.to_uppercase()),
                        gold,
                    ));
                }
                last_conf = s.conference.clone();
            }

            let selected = i == active_series;
            let row = format_series_row(s);
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if s.is_complete() {
                Style::default().fg(Color::Green)
            } else if s.games_played() > 0 {
                Style::default().fg(Color::White)
            } else {
                dim
            };
            lines.push(Line::styled(row, style));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!(
            "  Round {} of {} · {} series · Enter for series detail",
            active_round + 1,
            n_rounds,
            round.series.len(),
        ),
        dim,
    ));

    f.render_widget(Paragraph::new(lines), area);
}

fn format_series_row(s: &PlayoffSeries) -> String {
    let top_rank = s.top_seed_rank.as_deref().unwrap_or("—");
    let bot_rank = s.bottom_seed_rank.as_deref().unwrap_or("—");
    let summary = s.summary();
    format!(
        "    ({:<3}) {:<3}  vs  ({:<3}) {:<3}    {}",
        top_rank, s.top_seed_abbrev, bot_rank, s.bottom_seed_abbrev, summary,
    )
}

// ── Series detail view ───────────────────────────────────────────────────────

pub fn render_series_detail(f: &mut Frame, app: &App, area: Rect, letter: &str) {
    let title = format!(" Series {letter} · Esc back ");
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let year = match playoff_year_for_season(&app.active_season) {
        Some(y) => y,
        None => {
            render_message(f, inner, "No playoff data for this season.");
            return;
        }
    };

    let state = {
        let map = app.playoffs_cache.lock().unwrap();
        map.get(&year).cloned().unwrap_or(PlayoffsState::Idle)
    };

    let series = match state {
        PlayoffsState::Loaded(b) => b.find_series(letter).cloned(),
        PlayoffsState::Error(e) => {
            render_error(f, inner, &e);
            return;
        }
        _ => {
            render_message(f, inner, "Loading bracket…");
            return;
        }
    };

    let Some(series) = series else {
        render_message(f, inner, &format!("Series {letter} not found in bracket."));
        return;
    };

    render_series_body(f, inner, &series);
}

fn render_series_body(f: &mut Frame, area: Rect, s: &PlayoffSeries) {
    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let green = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Header: full team names + summary
    let header = format!(
        "  {} ({})  vs  {} ({})",
        s.top_seed_name,
        s.top_seed_rank.as_deref().unwrap_or("—"),
        s.bottom_seed_name,
        s.bottom_seed_rank.as_deref().unwrap_or("—"),
    );
    lines.push(Line::styled(header, gold));
    if let Some(conf) = &s.conference {
        lines.push(Line::styled(format!("  {} Conference", conf), dim));
    }
    lines.push(Line::from(""));

    let summary_style = if s.is_complete() { green } else { gold };
    lines.push(Line::styled(format!("  {}", s.summary()), summary_style));
    lines.push(Line::styled(format!("  {}", "─".repeat(60)), dim));
    lines.push(Line::from(""));

    lines.push(Line::styled("  GAMES", gold));
    if s.games.is_empty() {
        // Live API path — no per-game data. Show the count + next-game hint.
        lines.push(Line::styled(
            format!("    {} game(s) played so far", s.games_played()),
            Style::default(),
        ));
        if !s.is_complete() && s.games_played() < 7 {
            let next_game = s.games_played() + 1;
            // A best-of-7 game is mandatory ("upcoming") iff its number is at
            // most 4 + min(top_wins, bot_wins) — the trailing team's win count
            // guarantees that many games must be played. Beyond that, games
            // are "(if needed)".
            let last_mandatory = 4 + s.top_seed_wins.min(s.bottom_seed_wins);
            let hint = if next_game <= last_mandatory {
                format!("    Game {next_game} upcoming")
            } else {
                format!("    Game {next_game} (if needed)")
            };
            lines.push(Line::styled(hint, dim));
        }
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "  Per-game logs available for bundled historical seasons.",
            dim,
        ));
    } else {
        // Bundled-data path — render the actual game log.
        for (i, g) in s.games.iter().enumerate() {
            let row = format_game_row(i + 1, g);
            let style = if g.home_score > g.away_score && g.home_abbrev == s.top_seed_abbrev
                || g.away_score > g.home_score && g.away_abbrev == s.top_seed_abbrev
            {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::styled(row, style));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled("  Esc to return to bracket", dim));

    f.render_widget(Paragraph::new(lines), area);
}

/// Format one row of the game log:
/// `  Game 7  1994-06-14   NYR 3 – 2 VAN   NYR wins 4-3`
fn format_game_row(num: usize, g: &PlayoffGameResult) -> String {
    format!(
        "    Game {num}  {date}   {home} {hs} – {as_} {away}   {after}",
        num = num,
        date = g.date,
        home = g.home_abbrev,
        hs = g.home_score,
        as_ = g.away_score,
        away = g.away_abbrev,
        after = g.series_after,
    )
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn season_label(season: &str) -> String {
    if season.len() == 8 {
        let yy_start = &season[2..4];
        let yy_end = &season[6..8];
        format!("{yy_start}-{yy_end}")
    } else {
        season.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_fetch::nhl_api::{PlayoffBracket, PlayoffRound, PlayoffSeries};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn fixture_series(
        letter: &str,
        top: &str,
        bot: &str,
        top_w: u8,
        bot_w: u8,
        conf: Option<&str>,
    ) -> PlayoffSeries {
        PlayoffSeries {
            letter: Some(letter.to_owned()),
            top_seed_abbrev: top.to_owned(),
            top_seed_name: top.to_owned(),
            top_seed_wins: top_w,
            top_seed_rank: Some("A1".to_owned()),
            bottom_seed_abbrev: bot.to_owned(),
            bottom_seed_name: bot.to_owned(),
            bottom_seed_wins: bot_w,
            bottom_seed_rank: Some("WC2".to_owned()),
            winner_abbrev: if top_w == 4 {
                Some(top.to_owned())
            } else if bot_w == 4 {
                Some(bot.to_owned())
            } else {
                None
            },
            conference: conf.map(str::to_owned),
            games: Vec::new(),
        }
    }

    fn seed(app: &mut App, rounds: Vec<PlayoffRound>) {
        let bracket = PlayoffBracket {
            season: app.active_season.clone(),
            current_round: None,
            rounds,
        };
        let year = playoff_year_for_season(&app.active_season).unwrap();
        app.playoffs_cache
            .lock()
            .unwrap()
            .insert(year, PlayoffsState::Loaded(bracket));
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

    fn render_to_text(app: &App) -> String {
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render(f, app, area);
        })
        .unwrap();
        buffer_text(term.backend().buffer())
    }

    fn render_detail_to_text(app: &App, letter: &str) -> String {
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render_series_detail(f, app, area, letter);
        })
        .unwrap();
        buffer_text(term.backend().buffer())
    }

    #[test]
    fn l0_render_playoffs_idle_shows_loading() {
        let app = App::new(false);
        let text = render_to_text(&app);
        assert!(
            text.contains("Loading"),
            "idle must show loading message, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_playoffs_off_season_when_empty() {
        let mut app = App::new(false);
        seed(&mut app, vec![]);
        let text = render_to_text(&app);
        assert!(
            text.contains("Playoffs not yet active"),
            "empty bracket must show off-season message, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_playoffs_error_shows_retry_hint() {
        let app = App::new(false);
        let year = playoff_year_for_season(&app.active_season).unwrap();
        app.playoffs_cache
            .lock()
            .unwrap()
            .insert(year, PlayoffsState::Error("connection refused".to_owned()));
        let text = render_to_text(&app);
        assert!(
            text.contains("unavailable"),
            "error path missing, got:\n{text}"
        );
        assert!(text.contains("retry"), "must hint retry, got:\n{text}");
    }

    #[test]
    fn l0_render_playoffs_loaded_shows_round_header_and_series() {
        let mut app = App::new(false);
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".to_owned(),
            series: vec![
                fixture_series("A", "FLA", "TBL", 4, 2, Some("Eastern")),
                fixture_series("B", "WSH", "NYR", 3, 3, Some("Eastern")),
                fixture_series("E", "EDM", "VAN", 4, 1, Some("Western")),
            ],
        };
        let r2 = PlayoffRound {
            round_number: 2,
            label: "Second Round".to_owned(),
            series: vec![fixture_series("I", "FLA", "WSH", 1, 0, Some("Eastern"))],
        };
        seed(&mut app, vec![r1, r2]);

        let text = render_to_text(&app);
        // Round labels
        assert!(text.contains("First Round"), "round 1 label missing");
        assert!(text.contains("Second Round"), "round 2 label missing");
        // Conference headers in active round (round 0 default)
        assert!(
            text.contains("EASTERN CONFERENCE"),
            "east header missing, got:\n{text}"
        );
        assert!(text.contains("WESTERN CONFERENCE"), "west header missing");
        // Team abbrevs
        assert!(text.contains("FLA") && text.contains("TBL"));
        assert!(text.contains("EDM") && text.contains("VAN"));
        // Summary phrasing for completed series
        assert!(
            text.contains("FLA wins"),
            "completed series summary missing, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_playoffs_active_round_changes_with_cursor() {
        let mut app = App::new(false);
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".to_owned(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2, Some("Eastern"))],
        };
        let r2 = PlayoffRound {
            round_number: 2,
            label: "Second Round".to_owned(),
            series: vec![fixture_series("I", "FLA", "WSH", 1, 0, Some("Eastern"))],
        };
        seed(&mut app, vec![r1, r2]);

        // Default (round 0): only round-1 series rendered as series rows
        let text0 = render_to_text(&app);
        assert!(
            text0.contains("Round 1 of 2"),
            "footer should show 'Round 1 of 2', got:\n{text0}"
        );

        // Move to round 2
        app.playoffs_round = 1;
        let text1 = render_to_text(&app);
        assert!(
            text1.contains("Round 2 of 2"),
            "footer should show 'Round 2 of 2', got:\n{text1}"
        );
    }

    #[test]
    fn l0_render_series_detail_shows_summary_and_games_played() {
        let mut app = App::new(false);
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".to_owned(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2, Some("Eastern"))],
        };
        seed(&mut app, vec![r1]);

        let text = render_detail_to_text(&app, "A");
        assert!(text.contains("FLA"), "team must appear");
        assert!(text.contains("TBL"), "team must appear");
        assert!(text.contains("Eastern"), "conference label must appear");
        assert!(
            text.contains("FLA wins"),
            "summary must show FLA wins, got:\n{text}"
        );
        assert!(
            text.contains("6 game(s) played"),
            "games_played count must appear, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_series_detail_unknown_letter_shows_message() {
        let mut app = App::new(false);
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".to_owned(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2, Some("Eastern"))],
        };
        seed(&mut app, vec![r1]);

        let text = render_detail_to_text(&app, "Z");
        assert!(
            text.contains("Series Z not found"),
            "unknown letter must show explicit message, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_series_detail_in_progress_hints_next_game() {
        let mut app = App::new(false);
        // 3-2 in progress → next is Game 6 upcoming
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".to_owned(),
            series: vec![fixture_series("A", "FLA", "TBL", 3, 2, Some("Eastern"))],
        };
        seed(&mut app, vec![r1]);

        let text = render_detail_to_text(&app, "A");
        assert!(
            text.contains("Game 6"),
            "in-progress series must hint next game, got:\n{text}"
        );
        // 3-2 means top_seed needs only 1 more → Game 6 is mandatory ("upcoming"), Game 7 conditional
        assert!(
            text.contains("upcoming"),
            "Game 6 should be 'upcoming' not 'if needed', got:\n{text}"
        );
    }

    // ── Phase 8c: bundled per-game render path ──────────────────────────────

    fn fixture_series_with_games(letter: &str) -> PlayoffSeries {
        use icelines_fetch::nhl_api::PlayoffGameResult;
        PlayoffSeries {
            letter: Some(letter.to_owned()),
            top_seed_abbrev: "NYR".to_owned(),
            top_seed_name: "New York Rangers".to_owned(),
            top_seed_wins: 4,
            top_seed_rank: Some("E1".to_owned()),
            bottom_seed_abbrev: "VAN".to_owned(),
            bottom_seed_name: "Vancouver Canucks".to_owned(),
            bottom_seed_wins: 3,
            bottom_seed_rank: Some("W7".to_owned()),
            winner_abbrev: Some("NYR".to_owned()),
            conference: None,
            games: vec![
                PlayoffGameResult {
                    date: "1994-05-31".to_owned(),
                    home_abbrev: "NYR".to_owned(),
                    away_abbrev: "VAN".to_owned(),
                    home_score: 2,
                    away_score: 3,
                    series_after: "VAN leads 1-0".to_owned(),
                    goals: vec![],
                },
                PlayoffGameResult {
                    date: "1994-06-14".to_owned(),
                    home_abbrev: "NYR".to_owned(),
                    away_abbrev: "VAN".to_owned(),
                    home_score: 3,
                    away_score: 2,
                    series_after: "NYR wins 4-3".to_owned(),
                    goals: vec![],
                },
            ],
        }
    }

    #[test]
    fn l0_render_series_detail_with_games_shows_game_log() {
        let mut app = App::new(false);
        let r4 = PlayoffRound {
            round_number: 4,
            label: "Stanley Cup Final".to_owned(),
            series: vec![fixture_series_with_games("CUP")],
        };
        seed(&mut app, vec![r4]);

        let text = render_detail_to_text(&app, "CUP");
        // Game-log rows render with date + score + series_after.
        assert!(
            text.contains("Game 1"),
            "first game row missing, got:\n{text}"
        );
        assert!(text.contains("1994-05-31"), "first game date missing");
        assert!(text.contains("VAN leads 1-0"), "first series_after missing");
        assert!(text.contains("Game 2"), "second game row missing");
        assert!(text.contains("1994-06-14"), "Cup-clinching date missing");
        assert!(
            text.contains("NYR wins 4-3"),
            "Cup-clinching series_after missing"
        );
        // The v2 placeholder / "X game(s) played so far" line is gone for this path.
        assert!(
            !text.contains("game(s) played so far"),
            "non-placeholder branch shouldn't show fallback count, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_series_detail_no_games_falls_back_to_count() {
        // Live API path — empty games → still shows count + next-game hint.
        let mut app = App::new(false);
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".to_owned(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2, Some("Eastern"))],
        };
        seed(&mut app, vec![r1]);

        let text = render_detail_to_text(&app, "A");
        assert!(
            text.contains("6 game(s) played"),
            "fallback count must appear when games empty, got:\n{text}"
        );
    }
}
