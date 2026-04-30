//! TUI screens: Tonight/Scores, Projections, Groups, Fetch+Install, Schedule, Playoffs, Admin.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::app::App;

// ── Scores / Tonight ─────────────────────────────────────────────────────────

pub fn render_tonight(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tonight::{lookup, TonightState};

    // Reserve a 3-line strip at the bottom for the d-key date picker overlay.
    let bottom_h: u16 = if app.scores_picker_open { 3 } else { 0 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_h)])
        .split(area);
    let main_area = chunks[0];

    let state = lookup(&app.tonight_cache, &app.scores_date);
    let date_label = scores_date_label(&app.scores_date);
    let updated = scores_updated_indicator(app);

    let title = match &state {
        TonightState::Loading  => format!(" Scores · {date_label} · fetching…{updated} "),
        TonightState::Error(_) => format!(" Scores · {date_label} · fetch failed · r:retry{updated} "),
        _ => format!(" Scores · {date_label} ·  ←→:date  d:jump  t:today  Enter:detail{updated} "),
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(main_area);
    f.render_widget(block, main_area);

    let dim = Style::default().fg(Color::DarkGray);

    match state {
        TonightState::Idle => {
            f.render_widget(Paragraph::new(vec![
                Line::from(""),
                Line::styled("  Loading schedule…", dim),
            ]), inner);
        }
        TonightState::Loading => {
            f.render_widget(Paragraph::new(vec![
                Line::from(""),
                Line::styled("  Fetching NHL schedule…", Style::default().fg(Color::Cyan)),
            ]), inner);
        }
        TonightState::Error(e) => {
            f.render_widget(Paragraph::new(vec![
                Line::from(""),
                Line::styled(format!("  Error: {e}"), Style::default().fg(Color::Red)),
                Line::from(""),
                Line::styled("  Press r to retry.", dim),
                Line::from(""),
                Line::styled("  Or use the CLI: icelines tonight", dim),
            ]), inner);
        }
        TonightState::Loaded(games) => {
            render_scores_list(f, app, inner, &games);
        }
    }

    if bottom_h > 0 {
        render_scores_date_picker(f, app, chunks[1]);
    }
}

fn render_scores_date_picker(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Go to date — Enter applies, Esc cancels ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(err) = &app.scores_picker_err {
        f.render_widget(
            Paragraph::new(Line::styled(
                format!("  ⚠ {err}"),
                Style::default().fg(Color::Red),
            )),
            inner,
        );
        return;
    }

    let cursor = "█";
    let prompt = format!("  Go to: {}{cursor}", app.scores_picker_input);
    f.render_widget(Paragraph::new(prompt), inner);
}

fn render_scores_list(f: &mut Frame, app: &App, area: Rect, games: &[icelines_fetch::nhl_api::ScheduledGame]) {
    let date_label = scores_date_label(&app.scores_date);
    let dim_style  = Style::default().fg(Color::DarkGray);

    // The NHL `/v1/schedule/now` endpoint returns the whole "gameWeek"
    // (up to 7 days) — so we filter games to the user's selected date.
    // Empty `scores_date` means "today" — but timezone matters: a user
    // in Pacific time looking at the Scores tab at 8 PM PT is at
    // 2026-04-29 locally but 2026-04-30 UTC, while the NHL day grouping
    // can use either depending on the game's time. Accept BOTH local
    // and UTC "today" so the user always sees tonight's games.
    let target_dates: Vec<String> = if app.scores_date.is_empty() {
        let local = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        let utc   = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        if local == utc { vec![local] } else { vec![local, utc] }
    } else {
        vec![app.scores_date.clone()]
    };
    let filtered: Vec<&icelines_fetch::nhl_api::ScheduledGame> = games.iter()
        .filter(|g| target_dates.iter().any(|d| *d == g.date))
        .collect();

    if filtered.is_empty() {
        // Diagnostic hint: when the gameWeek isn't empty but our filter
        // matched nothing, it's almost certainly a timezone or
        // navigation mismatch. Surface the first available date so the
        // user can hit ←/→ to find it instead of staring at "no games".
        let hint = if !games.is_empty() && app.scores_date.is_empty() {
            let earliest = games.iter()
                .map(|g| g.date.as_str())
                .filter(|d| !d.is_empty())
                .min()
                .unwrap_or("");
            format!("  Schedule has {} game(s) starting {} — press → to view.",
                games.len(), earliest)
        } else if app.scores_date.is_empty() {
            "  No games scheduled today.".to_owned()
        } else {
            format!("  No games scheduled for {date_label}.")
        };
        f.render_widget(Paragraph::new(vec![
            Line::from(""),
            Line::styled(hint, dim_style),
            Line::from(""),
            Line::styled("  ←/→ navigate days  ·  d jump to date  ·  Esc back", dim_style),
        ]), area);
        return;
    }

    // Detect if any game is a playoff game
    let has_playoffs = filtered.iter().any(|g| g.is_playoff());
    let has_regular  = filtered.iter().any(|g| !g.is_playoff());

    let section_label = match (has_playoffs, has_regular) {
        (true,  false) => "  PLAYOFFS",
        (false, true)  => "  REGULAR SEASON",
        _              => "  TONIGHT",
    };

    let dim   = Style::default().fg(Color::DarkGray);
    let gold  = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let mut items: Vec<ratatui::widgets::ListItem> = vec![
        ratatui::widgets::ListItem::new(Line::styled(section_label, gold)),
        ratatui::widgets::ListItem::new(Line::styled(format!("  {}", "─".repeat(60)), dim)),
    ];

    use ratatui::text::Span;
    for (i, game) in filtered.iter().enumerate() {
        let utc  = game.start_time_utc.get(11..16).unwrap_or("?");
        let et   = fmt_et(utc);
        let selected = i == app.scores_selected;

        // The series tag is the game number ("Game 5") for playoff games.
        // For regular-season games it's empty.
        let series_tag = if game.is_playoff() {
            game.series_game.clone().unwrap_or_else(|| "Playoffs".to_owned())
        } else {
            String::new()
        };

        // Score block — the part the user wants to "pop".
        // Final/live games: `away_score – home_score` with the winner's
        //   number bolded + cyan, loser dim. The em-dash separator is
        //   greyed so the numbers carry the eye.
        // Future games: time-of-day in cyan.
        let row_base = if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if game.is_playoff() {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let label_dim = if selected { row_base } else { dim };
        let accent    = if selected { row_base } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        };

        let mut spans: Vec<Span<'static>> = Vec::new();
        // Indent + matchup column ("  MTL @ TBL  ").
        spans.push(Span::styled("  ".to_owned(), label_dim));
        spans.push(Span::styled(format!("{:>4} ", game.away_abbrev), label_dim));
        spans.push(Span::styled("@ ".to_owned(), if selected { row_base } else { dim }));
        spans.push(Span::styled(format!("{:<4}", game.home_abbrev), label_dim));
        // Series tag (game number) — distinct, dim.
        if !series_tag.is_empty() {
            spans.push(Span::styled(format!("  {series_tag}"), label_dim));
        }
        // Padding to push the score block to a fixed right-side column.
        let prefix_width = 2 + 5 + 2 + 4 + if series_tag.is_empty() { 0 } else { 2 + series_tag.chars().count() };
        let target_col   = 36usize;  // score column starts here, regardless of prefix
        let pad = target_col.saturating_sub(prefix_width);
        if pad > 0 { spans.push(Span::raw(" ".repeat(pad))); }

        // Score / time block.
        if game.is_final() || game.is_live() {
            let aw = game.away_score.unwrap_or(0);
            let hw = game.home_score.unwrap_or(0);
            let (away_style, home_style) = match aw.cmp(&hw) {
                std::cmp::Ordering::Greater => (accent,    label_dim),
                std::cmp::Ordering::Less    => (label_dim, accent),
                std::cmp::Ordering::Equal   => (label_dim, label_dim),
            };
            spans.push(Span::styled(format!("{aw:>2}"), away_style));
            spans.push(Span::styled(" – ".to_owned(),
                if selected { row_base } else { dim }));
            spans.push(Span::styled(format!("{hw}"), home_style));
            // Final / LIVE tag at the far right.
            let tag = if game.is_live() { "LIVE" }
                      else { match game.last_period.as_deref() {
                          Some("OT") => "Final/OT",
                          Some("SO") => "Final/SO",
                          _          => "Final",
                      }};
            let tag_style = if game.is_live() {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                label_dim
            };
            spans.push(Span::styled(format!("  {tag}"), tag_style));
        } else {
            // Scheduled / not-yet-started — show the start time in accent.
            spans.push(Span::styled(et.clone(), accent));
        }
        items.push(ratatui::widgets::ListItem::new(Line::from(spans)));

        // Series context line for playoff games
        if game.is_playoff() {
            if let (Some(aw), Some(hw)) = (game.away_wins, game.home_wins) {
                let ctx = match aw.cmp(&hw) {
                    std::cmp::Ordering::Greater =>
                        format!("         {} leads series {}-{}", game.away_abbrev, aw, hw),
                    std::cmp::Ordering::Less =>
                        format!("         {} leads series {}-{}", game.home_abbrev, hw, aw),
                    std::cmp::Ordering::Equal =>
                        format!("         Series tied {}-{}", aw, hw),
                };
                let ctx_style = if selected { row_base } else { dim };
                items.push(ratatui::widgets::ListItem::new(Line::styled(ctx, ctx_style)));
            }
        }
    }

    items.push(ratatui::widgets::ListItem::new(Line::from("")));
    // Date arrows + jump hint at the bottom — mirrors the spec's footer
    let anchor = if app.scores_date.is_empty() {
        crate::tui::schedule::today_iso()
    } else {
        app.scores_date.clone()
    };
    let prev_d = crate::tui::schedule::add_days(&anchor, -1).unwrap_or_default();
    let next_d = crate::tui::schedule::add_days(&anchor, 1).unwrap_or_default();
    items.push(ratatui::widgets::ListItem::new(Line::styled(
        format!("  ←  {prev_d}    {next_d}  →"),
        dim,
    )));
    items.push(ratatui::widgets::ListItem::new(Line::styled(
        "  Times shown in ET  ·  data from NHL public API", dim
    )));

    f.render_widget(ratatui::widgets::List::new(items), area);
}

/// "Updated Xs ago" indicator string, prefixed with " · " so it can be
/// concatenated into a title segment. Returns an empty string when no
/// auto-refresh has happened yet (e.g. on past dates).
fn scores_updated_indicator(app: &App) -> String {
    let last = match app.last_auto_refresh {
        Some(t) => t,
        None    => return String::new(),
    };
    let elapsed = std::time::Instant::now().duration_since(last).as_secs();
    format!("  ·  Updated {}s ago", elapsed)
}

/// Format the active scores date for the title bar — "Today" for the empty
/// sentinel, "Mon Apr 28, 2026" for an explicit date.
fn scores_date_label(date_key: &str) -> String {
    if date_key.is_empty() {
        return "Today".to_owned();
    }
    use chrono::NaiveDate;
    if let Ok(d) = NaiveDate::parse_from_str(date_key, "%Y-%m-%d") {
        d.format("%a %b %-d, %Y").to_string()
    } else {
        date_key.to_owned()
    }
}

/// Convert "HH:MM" UTC to "H:MM AM/PM ET" (EDT = UTC-4).
fn fmt_et(utc_hhmm: &str) -> String {
    let parts: Vec<&str> = utc_hhmm.splitn(2, ':').collect();
    if let [h, m] = parts.as_slice() {
        if let (Ok(h), Ok(m)) = (h.parse::<u32>(), m.parse::<u32>()) {
            let et_h = (h + 24 - 4) % 24;
            let period = if et_h < 12 { "AM" } else { "PM" };
            let display = match et_h % 12 { 0 => 12, n => n };
            return format!("{display}:{m:02} {period}");
        }
    }
    format!("{utc_hhmm} UTC")
}

// ── Projections ───────────────────────────────────────────────────────────────

pub fn render_projections(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Top Projections (pts/82)  ↑↓ scroll · Enter: player card ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.players.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from("  Loading player data…"),
            Line::from("  Run `icelines fetch all` if this persists."),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let mut sorted: Vec<_> = app.players.iter().filter(|p| p.pace_score.is_some()).collect();
    sorted.sort_by(|a, b| {
        let sa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
        let sb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let visible = inner.height.saturating_sub(3) as usize;
    let offset  = app.selected.saturating_sub(visible / 2).min(sorted.len().saturating_sub(visible));

    let dim = Style::default().fg(Color::DarkGray);
    let mut lines = vec![
        Line::styled(
            format!("  {:<4} {:<22} {:<5} {:<4} {:>6}  {:>7}  {:>5}", "Rank","Player","Team","Pos","PPG","Pts/82","GP"),
            dim,
        ),
        Line::styled(format!("  {}", "─".repeat(58)), dim),
    ];

    for (i, p) in sorted.iter().skip(offset).take(visible).enumerate() {
        let global_rank = offset + i + 1;
        let ppg  = p.pace_score.map(|s| format!("{:.3}", s.pace_82 / 82.0)).unwrap_or_else(|| "—".to_owned());
        let proj = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
        let gp   = p.pace_score.map(|s| s.gp.to_string()).unwrap_or_else(|| "—".to_owned());
        let name = p.full_name.chars().take(22).collect::<String>();

        let style = if offset + i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if global_rank <= 5 {
            Style::default().fg(Color::Green)
        } else if global_rank <= 20 {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::styled(
            format!("  {:<4} {:<22} {:<5} {:<4} {:>6}  {:>7}  {:>5}", global_rank, name, p.team.as_str(), p.position.abbreviation(), ppg, proj, gp),
            style,
        ));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Groups ────────────────────────────────────────────────────────────────────

/// Which group is "open" for member viewing — stored in app.selected
/// When selected >= group count, we're in group-list view (default)
pub fn render_groups(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Groups — ↑↓ select · Enter view members · Esc back ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Load groups synchronously — GroupDb reads are fast (local SQLite)
    let groups = crate::db::GroupDb::open()
        .ok()
        .and_then(|db| db.list_groups().ok())
        .unwrap_or_default();

    if groups.is_empty() {
        let dim = Style::default().fg(Color::DarkGray);
        let cmd = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        let lines = vec![
            Line::from(""),
            Line::from("  No groups yet. Create one in your terminal:"),
            Line::from(""),
            Line::styled("  icelines group create \"My Watchlist\"", cmd),
            Line::styled("  icelines group add \"My Watchlist\" \"McDavid\"", cmd),
            Line::styled("  icelines group show \"My Watchlist\"", cmd),
            Line::from(""),
            Line::styled("  Groups persist in ~/.icelines/icelines.db", dim),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let mut lines = vec![
        Line::styled(
            format!("  {:<24} {:>7}  {}", "Group", "Members", "Description"),
            Style::default().fg(Color::DarkGray),
        ),
        Line::styled(format!("  {}", "─".repeat(60)), Style::default().fg(Color::DarkGray)),
    ];

    for (i, g) in groups.iter().enumerate() {
        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let desc = g.description.chars().take(28).collect::<String>();
        lines.push(Line::styled(
            format!("  {:<24} {:>7}  {}", g.name, g.member_count, desc),
            style,
        ));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  Enter: view members  ·  manage via icelines group <cmd>",
        Style::default().fg(Color::DarkGray),
    ));

    f.render_widget(Paragraph::new(lines), inner);
}

/// Show members of the selected group (called from app.rs when Enter is pressed on Groups).
pub fn render_group_members(f: &mut Frame, app: &App, area: Rect, group_name: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — Esc to go back ", group_name));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let members = crate::db::GroupDb::open()
        .ok()
        .and_then(|db| db.list_members(group_name).ok())
        .unwrap_or_default();

    if members.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from("  This group is empty."),
            Line::from(""),
            Line::styled(
                format!("  icelines group add \"{}\" \"McDavid\"", group_name),
                Style::default().fg(Color::Cyan),
            ),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let dim = Style::default().fg(Color::DarkGray);
    let mut lines = vec![
        Line::styled(format!("  {:<24} {:<5} {:<4} {:>8}", "Player", "Team", "Pos", "Pts/82"), dim),
        Line::styled(format!("  {}", "─".repeat(46)), dim),
    ];

    for (i, norm) in members.iter().enumerate() {
        let player = app.players.iter().find(|p| p.name_normalized.contains(norm.as_str()));
        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let row = match player {
            Some(p) => {
                let proj = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
                let name = p.full_name.chars().take(24).collect::<String>();
                format!("  {:<24} {:<5} {:<4} {:>8}", name, p.team.as_str(), p.position.abbreviation(), proj)
            }
            None => format!("  {}  (not in current data)", norm),
        };
        lines.push(Line::styled(row, style));
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(format!("  {} member(s)", members.len()), dim));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Fetch + Install ───────────────────────────────────────────────────────────

/// Season list for the picker overlay: (season_id, display_label, is_lockout).
/// Includes the 2004-05 lockout as an unselectable entry.
pub const PICKER_SEASONS: &[(&str, &str, bool)] = &[
    ("20252026","2025-26  (current)",               false),
    ("20242025","2024-25",                          false),
    ("20232024","2023-24",                          false),
    ("20222023","2022-23",                          false),
    ("20212022","2021-22",                          false),
    ("20202021","2020-21  (COVID bubble)",          false),
    ("20192020","2019-20",                          false),
    ("20182019","2018-19",                          false),
    ("20172018","2017-18",                          false),
    ("20162017","2016-17",                          false),
    ("20152016","2015-16  (McDavid rookie)",        false),
    ("20142015","2014-15",                          false),
    ("20132014","2013-14",                          false),
    ("20122013","2012-13  (lockout-shortened)",     false),
    ("20112012","2011-12",                          false),
    ("20102011","2010-11",                          false),
    ("20092010","2009-10",                          false),
    ("20082009","2008-09",                          false),
    ("20072008","2007-08",                          false),
    ("20062007","2006-07",                          false),
    ("20052006","2005-06  (Ovechkin/Crosby)",       false),
    ("20042005","✗ 2004-05  LOCKOUT — no season",   true),
    ("20032004","2003-04",                          false),
    ("20022003","2002-03",                          false),
    ("20012002","2001-02",                          false),
    ("20002001","2000-01",                          false),
    ("19992000","1999-2000",                        false),
    ("19981999","1998-99",                          false),
    ("19971998","1997-98",                          false),
    ("19961997","1996-97",                          false),
    ("19951996","1995-96",                          false),
    ("19941995","1994-95  (lockout-shortened)",     false),
    ("19931994","1993-94",                          false),
    ("19921993","1992-93",                          false),
    ("19911992","1991-92",                          false),
    ("19901991","1990-91",                          false),
    ("19891990","1989-90",                          false),
    ("19881989","1988-89",                          false),
    ("19871988","1987-88  (Gretzky to LA)",         false),
];

/// All 38 seasons newest-first (mirrors AVAILABLE_SEASONS in data.rs).
pub const ALL_SEASONS: &[(&str, &str)] = &[
    ("20252026","2025-26 Current"),
    ("20242025","2024-25"),
    ("20232024","2023-24"),
    ("20222023","2022-23"),
    ("20212022","2021-22"),
    ("20202021","2020-21 (COVID bubble)"),
    ("20192020","2019-20"),
    ("20182019","2018-19"),
    ("20172018","2017-18"),
    ("20162017","2016-17"),
    ("20152016","2015-16 (McDavid rookie)"),
    ("20142015","2014-15"),
    ("20132014","2013-14"),
    ("20122013","2012-13 (lockout-shortened)"),
    ("20112012","2011-12"),
    ("20102011","2010-11"),
    ("20092010","2009-10"),
    ("20082009","2008-09"),
    ("20072008","2007-08"),
    ("20062007","2006-07"),
    ("20052006","2005-06 (Ovechkin/Crosby rookies)"),
    ("20032004","2003-04"),
    ("20022003","2002-03"),
    ("20012002","2001-02"),
    ("20002001","2000-01"),
    ("19992000","1999-2000"),
    ("19981999","1998-99"),
    ("19971998","1997-98"),
    ("19961997","1996-97"),
    ("19951996","1995-96"),
    ("19941995","1994-95 (lockout-shortened)"),
    ("19931994","1993-94"),
    ("19921993","1992-93"),
    ("19911992","1991-92"),
    ("19901991","1990-91"),
    ("19891990","1989-90"),
    ("19881989","1988-89"),
    ("19871988","1987-88 (Gretzky to LA)"),
];

fn season_installed(season: &str) -> bool {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let bundle_path = std::path::Path::new(&home)
        .join(".icelines/seasons")
        .join(season)
        .join(format!("bundle-{season}/bios.json"));
    bundle_path.exists()
}

pub fn render_fetch(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::loader::InstallPhase;

    // Split: top section (fetch commands) | bottom section (season list)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    // ── Top: Fetch commands ──────────────────────────────────────────────────
    let fetch_block = Block::default()
        .borders(Borders::ALL)
        .title(" Fetch Commands ");
    let fetch_inner = fetch_block.inner(chunks[0]);
    f.render_widget(fetch_block, chunks[0]);

    let player_status = if app.players.is_empty() {
        "loading…".to_owned()
    } else {
        format!("{} players loaded", app.players.len())
    };

    let cmd = Style::default().fg(Color::Cyan);
    let dim = Style::default().fg(Color::DarkGray);
    let fetch_lines = vec![
        Line::styled(format!("  Data: {}", player_status), dim),
        Line::from(""),
        Line::styled("  icelines fetch all", cmd),
        Line::styled("  → rosters + stats from NHL API (~5 min, run in terminal)", dim),
        Line::styled("  icelines fetch realtime  |  icelines fetch money-puck", cmd),
    ];
    f.render_widget(Paragraph::new(fetch_lines), fetch_inner);

    // ── Bottom: Season install list ──────────────────────────────────────────
    let install_title = match app.install_state.phase() {
        InstallPhase::Downloading(ref s) => format!(" Installing {}… ⠋ ", s),
        InstallPhase::Done(ref s, kb)    => format!(" ✓ {} installed ({} KB) — ↑↓ for more · i to install ", s, kb),
        InstallPhase::Error(_, _)        => " Install failed — see status bar ".to_owned(),
        InstallPhase::Idle               => " Season History — ↑↓ select · i to install ".to_owned(),
    };
    let install_block = Block::default()
        .borders(Borders::ALL)
        .title(install_title);
    let install_inner = install_block.inner(chunks[1]);
    f.render_widget(install_block, chunks[1]);

    let visible = install_inner.height as usize;
    let offset  = app.selected.saturating_sub(visible / 2).min(ALL_SEASONS.len().saturating_sub(visible));

    let installing_id = match app.install_state.phase() {
        InstallPhase::Downloading(ref s) => Some(s.clone()),
        _ => None,
    };
    let just_done_id = match app.install_state.phase() {
        InstallPhase::Done(ref s, _) => Some(s.clone()),
        _ => None,
    };

    let spinner_frames = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
    let spinner = spinner_frames[(app.tick / 2 % 10) as usize];

    let items: Vec<ListItem> = ALL_SEASONS.iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, (season_id, label))| {
            let installed = season_installed(season_id)
                || just_done_id.as_deref() == Some(season_id);
            let is_installing = installing_id.as_deref() == Some(season_id);

            let (marker, marker_style) = if is_installing {
                (spinner, Style::default().fg(Color::Yellow))
            } else if installed {
                ("✓", Style::default().fg(Color::Green))
            } else {
                ("○", Style::default().fg(Color::DarkGray))
            };

            let row_style = if i == app.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if is_installing {
                Style::default().fg(Color::Yellow)
            } else if installed {
                Style::default()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let hint = if i == app.selected && !installed && !is_installing {
                "  ← press i to install"
            } else if is_installing {
                "  ← downloading…"
            } else {
                ""
            };

            let text = format!("  {} {:<10}  {}{}", marker, season_id, label, hint);
            ListItem::new(Line::styled(text, if is_installing { marker_style } else { row_style }))
        })
        .collect();

    f.render_widget(List::new(items), install_inner);
}

// ── Season picker overlay ─────────────────────────────────────────────────────

pub fn render_season_picker(f: &mut Frame, app: &App, area: Rect) {
    use icelines_fetch::bundled::{is_installed, BUNDLED_SEASONS};

    let popup_h = (area.height * 70 / 100).min(44);
    let popup_w = (area.width  * 50 / 100).min(52).max(44);
    let popup = Rect::new(
        area.x + (area.width  - popup_w) / 2,
        area.y + (area.height - popup_h) / 2,
        popup_w, popup_h,
    );
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = ratatui::widgets::Block::default()
        .borders(Borders::ALL)
        .title(" Select Season — ↑↓ · Enter · i:install · Esc:cancel ")
        .style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let visible = inner.height as usize;
    let total   = PICKER_SEASONS.len();
    let offset  = app.picker_selected.saturating_sub(visible / 2).min(total.saturating_sub(visible));

    let items: Vec<ListItem> = PICKER_SEASONS.iter().enumerate()
        .skip(offset).take(visible)
        .map(|(i, (season_id, label, is_lockout))| {
            let is_current  = *season_id == app.active_season.as_str();
            let is_bundled  = BUNDLED_SEASONS.contains(season_id);
            let installed   = is_bundled || is_installed(season_id);
            let selected    = i == app.picker_selected;

            let prefix = if is_current  { "▶ " }
                         else if installed { "✓ " }
                         else             { "  " };

            let (style, suffix) = if selected {
                (Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD), "")
            } else if *is_lockout {
                (Style::default().fg(Color::DarkGray), "")
            } else if !installed {
                (Style::default().fg(Color::DarkGray), "  [not installed]")
            } else {
                (Style::default(), "")
            };

            ListItem::new(ratatui::text::Line::styled(
                format!(" {prefix}{label}{suffix}"),
                style,
            ))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}


// ── Admin overlay ─────────────────────────────────────────────────────────────

pub fn render_admin(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::loader::InstallPhase;

    let dim  = Style::default().fg(Color::DarkGray);
    let cmd  = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let hi   = Style::default().fg(Color::White);

    let phase_line = match app.install_state.phase() {
        InstallPhase::Idle =>
            Line::styled("  No install in progress.", dim),
        InstallPhase::Downloading(ref s) =>
            Line::styled(format!("  Installing {s}…"), Style::default().fg(Color::Cyan)),
        InstallPhase::Done(ref s, kb) =>
            Line::styled(format!("  ✓ {s} installed ({kb} KB)"), Style::default().fg(Color::Green)),
        InstallPhase::Error(_, ref msg) =>
            Line::styled(format!("  ✗ Failed: {msg}"), Style::default().fg(Color::Red)),
    };

    let lines = vec![
        Line::from(""),
        Line::styled("  Admin commands (run in terminal):", hi),
        Line::from(""),
        Line::styled("  icelines fetch all", cmd),
        Line::styled("    → refresh all NHL data", dim),
        Line::from(""),
        Line::styled("  icelines data list", cmd),
        Line::styled("    → show installed seasons", dim),
        Line::from(""),
        Line::styled("  icelines data install 20032004", cmd),
        Line::styled("    → install a historical season", dim),
        Line::from(""),
        Line::styled("  ─────────────────────────────", dim),
        phase_line,
        Line::from(""),
        Line::styled("  Esc to close", dim),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    //! L0 render tests for the admin overlay (Phase 8a.2).
    use super::*;
    use crate::tui::app::App;
    use crate::tui::loader::InstallPhase;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

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

    fn render_admin_to_text(app: &App) -> String {
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render_admin(f, app, area);
        }).unwrap();
        buffer_text(term.backend().buffer())
    }

    #[test]
    fn l0_render_admin_idle_phase_shows_no_install() {
        let app = App::new(false);
        // Default phase is Idle
        let text = render_admin_to_text(&app);
        assert!(text.contains("No install in progress"),
            "Idle phase must show 'No install in progress', got:\n{text}");
        // Canonical CLI commands listed
        assert!(text.contains("icelines fetch all"), "fetch hint missing");
        assert!(text.contains("icelines data list"), "list hint missing");
    }

    #[test]
    fn l0_render_admin_downloading_phase_shows_spinner() {
        let app = App::new(false);
        app.install_state.force_phase(InstallPhase::Downloading("19931994".to_owned()));
        let text = render_admin_to_text(&app);
        assert!(text.contains("Installing 19931994"),
            "Downloading phase must show season being installed, got:\n{text}");
    }

    #[test]
    fn l0_render_admin_error_phase_shows_red() {
        let app = App::new(false);
        app.install_state.force_phase(InstallPhase::Error(
            "19931994".to_owned(),
            "connection refused".to_owned(),
        ));
        let text = render_admin_to_text(&app);
        // TestBackend captures characters but not ANSI colors. Verify the
        // textual content of the error branch reaches the buffer.
        assert!(text.contains("Failed"),
            "Error phase must show 'Failed:' prefix, got:\n{text}");
        assert!(text.contains("connection refused"),
            "Error phase must include the error message, got:\n{text}");
    }

    #[test]
    fn l0_render_admin_done_phase_shows_check_and_size() {
        let app = App::new(false);
        app.install_state.force_phase(InstallPhase::Done("19931994".to_owned(), 4321));
        let text = render_admin_to_text(&app);
        assert!(text.contains("19931994") && text.contains("4321"),
            "Done phase must show season + KB, got:\n{text}");
    }

    // ── Scores auto-refresh indicator (Phase 8b) ─────────────────────────────

    fn render_tonight_to_text(app: &App) -> String {
        let backend = TestBackend::new(120, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render_tonight(f, app, area);
        }).unwrap();
        buffer_text(term.backend().buffer())
    }

    #[test]
    fn l0_render_scores_shows_updated_indicator_when_armed() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Tonight;
        app.scores_date.clear();
        // Force the indicator to read ~14s by setting last_auto_refresh in the past.
        let past = std::time::Instant::now() - std::time::Duration::from_secs(14);
        app.last_auto_refresh = Some(past);

        let text = render_tonight_to_text(&app);
        // Look for "Updated " followed by a digit + "s ago"
        let has_marker = text.contains("Updated") && text.contains("s ago");
        assert!(has_marker, "auto-refresh indicator missing, got:\n{text}");
    }

    #[test]
    fn l0_render_scores_hides_updated_indicator_on_past_date() {
        let app = App::new(false);
        let mut app = app;
        app.screen = crate::tui::app::Screen::Tonight;
        app.scores_date = "2026-01-15".to_owned();
        // Past dates leave last_auto_refresh as None — no indicator.
        app.last_auto_refresh = None;
        let text = render_tonight_to_text(&app);
        assert!(!text.contains("Updated "), "indicator must not render on past dates");
    }

    // ── Scores tab date filtering (regression: 2026-04-29) ──────────────────
    //
    // `/v1/schedule/now` returns the whole "gameWeek" — up to 7 days of
    // future games. Without filtering, every day's games rendered together
    // as if they were all tonight's. These tests pin the per-date filter
    // so the regression doesn't come back.

    use icelines_fetch::nhl_api::ScheduledGame;
    use crate::tui::tonight::TonightState;

    fn fixture_game(date: &str, away: &str, home: &str, hour_utc: u8) -> ScheduledGame {
        ScheduledGame {
            game_id: 2025030100 + (hour_utc as u64),
            date: date.to_owned(),
            game_type: 3,
            away_abbrev: away.to_owned(),
            away_name:   format!("Away {away}"),
            home_abbrev: home.to_owned(),
            home_name:   format!("Home {home}"),
            start_time_utc: format!("{date}T{hour_utc:02}:00:00Z"),
            away_score: None,
            home_score: None,
            game_state: Some("FUT".to_owned()),
            last_period: None,
            series_game: Some("Game 1".to_owned()),
            away_wins: Some(0),
            home_wins: Some(0),
        }
    }

    fn render_with_games(scores_date: &str, games: Vec<ScheduledGame>) -> String {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Tonight;
        app.scores_date = scores_date.to_owned();
        app.tonight_cache.lock().unwrap()
            .insert(scores_date.to_owned(), TonightState::Loaded(games));
        render_tonight_to_text(&app)
    }

    #[test]
    fn l0_scores_filters_to_selected_date_only() {
        // Three games across three different days. With scores_date set
        // to the middle date, only the middle game renders.
        let games = vec![
            fixture_game("2026-04-27", "MTL", "TBL", 23),
            fixture_game("2026-04-28", "PIT", "PHI", 23),
            fixture_game("2026-04-29", "BUF", "BOS", 23),
        ];
        let text = render_with_games("2026-04-28", games);
        assert!(text.contains("PIT") && text.contains("PHI"),
            "selected-date game must render, got:\n{text}");
        assert!(!text.contains("MTL"),
            "earlier-date game must NOT render, got:\n{text}");
        assert!(!text.contains("BUF"),
            "later-date game must NOT render, got:\n{text}");
    }

    #[test]
    fn l0_scores_today_filter_when_scores_date_empty() {
        // Empty scores_date == "today" — populate the cache under the
        // empty-string key (the canonical TonightCache key for "now")
        // and verify only games with today's date render.
        let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        let two_weeks_ago = (chrono::Local::now().date_naive()
            - chrono::Duration::days(14))
            .format("%Y-%m-%d").to_string();
        let games = vec![
            fixture_game(&two_weeks_ago, "MTL", "TBL", 23),
            fixture_game(&today,         "PIT", "PHI", 23),
        ];
        let text = render_with_games("", games);
        assert!(text.contains("PIT"),
            "today's game must render, got:\n{text}");
        assert!(!text.contains("MTL"),
            "two-weeks-ago game must NOT render under empty scores_date, got:\n{text}");
    }

    #[test]
    fn l0_scores_filter_accepts_local_or_utc_today() {
        // Timezone tolerance: when the user is mid-evening Pacific, their
        // local date and the UTC date can disagree. Both should match —
        // the NHL day-wrapper grouping uses either depending on the
        // game's time slot. This test pins the relaxation: a game on
        // today-in-UTC (which could be tomorrow-in-local) renders OK.
        let utc_today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let games = vec![fixture_game(&utc_today, "MTL", "TBL", 23)];
        let text = render_with_games("", games);
        assert!(text.contains("MTL"),
            "UTC-today game must render even when local is yesterday, got:\n{text}");
    }

    #[test]
    fn l0_scores_empty_filter_shows_diagnostic_hint() {
        // When the API returns games but none match today's date in
        // either timezone, show a helpful hint with the earliest
        // upcoming date instead of "No games scheduled today" — the
        // user can then hit → to navigate to the games.
        let future = (chrono::Local::now().date_naive()
            + chrono::Duration::days(3))
            .format("%Y-%m-%d").to_string();
        let games = vec![
            fixture_game(&future, "PIT", "PHI", 23),
            fixture_game(&future, "BUF", "BOS", 23),
        ];
        let text = render_with_games("", games);
        assert!(text.contains("Schedule has 2 game"),
            "diagnostic count missing, got:\n{text}");
        assert!(text.contains(&future),
            "diagnostic should name the earliest available date, got:\n{text}");
        assert!(text.contains("press →"),
            "diagnostic should prompt for navigation, got:\n{text}");
    }

    #[test]
    fn l0_scores_renders_game_label_when_series_game_present() {
        // Regression for the "Game ?" placeholder bug: when the parser
        // populates series_game with a real label like "Game 1", the
        // Scores tab must surface that label, not a question mark.
        let mut g = fixture_game("2026-04-28", "WSH", "NYR", 23);
        g.series_game = Some("Game 4".to_owned());
        let text = render_with_games("2026-04-28", vec![g]);
        assert!(text.contains("Game 4"),
            "series label must render verbatim, got:\n{text}");
        assert!(!text.contains("Game ?"),
            "no question-mark placeholder when label is present, got:\n{text}");
    }

    #[test]
    fn l0_scores_final_game_renders_score_with_em_dash() {
        // Final games: score block is `aw – hw` with an em-dash
        // separator. The user's Scores tab now surfaces the score
        // distinctly from the matchup column.
        let mut g = fixture_game("2026-04-28", "MTL", "TBL", 23);
        g.away_score  = Some(3);
        g.home_score  = Some(2);
        g.game_state  = Some("OFF".to_owned());
        g.last_period = Some("REG".to_owned());
        let text = render_with_games("2026-04-28", vec![g]);
        // Score numbers AND em-dash separator both visible.
        assert!(text.contains(" 3 – 2"),
            "score should render as `3 – 2`, got:\n{text}");
        // Final tag at the right.
        assert!(text.contains("Final"),
            "Final tag should appear for completed games, got:\n{text}");
    }

    #[test]
    fn l0_scores_live_game_shows_LIVE_tag() {
        let mut g = fixture_game("2026-04-28", "MTL", "TBL", 23);
        g.away_score = Some(2);
        g.home_score = Some(1);
        g.game_state = Some("LIVE".to_owned());
        let text = render_with_games("2026-04-28", vec![g]);
        assert!(text.contains("LIVE"),
            "LIVE tag must render for in-progress games, got:\n{text}");
        assert!(text.contains("2 – 1"),
            "live games show their running score, got:\n{text}");
    }

    #[test]
    fn l0_scores_future_game_shows_start_time() {
        // Pre-game (game_state = "FUT") games show the ET start time
        // instead of a score. Time appears in the score column.
        let g = fixture_game("2026-04-28", "MTL", "TBL", 23);
        let text = render_with_games("2026-04-28", vec![g]);
        // 23:00 UTC = 7:00 PM ET (during DST).
        assert!(text.contains("7:00 PM") || text.contains("8:00 PM"),
            "future games show ET start time, got:\n{text}");
        assert!(!text.contains("Final"),
            "future games should NOT render the Final tag, got:\n{text}");
    }
}
