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
    use crate::tui::tonight::TonightState;
    use icelines_fetch::nhl_api::ScheduledGame;

    let state = app.tonight_cache.lock().unwrap().clone();

    let title = match &state {
        TonightState::Loading => " Scores — fetching… ",
        TonightState::Error(_) => " Scores — fetch failed · r: retry ",
        _ => " Scores — r:refresh  ↑↓:select  Esc:back ",
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

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
}

fn render_scores_list(f: &mut Frame, app: &App, area: Rect, games: &[icelines_fetch::nhl_api::ScheduledGame]) {
    if games.is_empty() {
        f.render_widget(Paragraph::new(vec![
            Line::from(""),
            Line::styled("  No games scheduled today.", Style::default().fg(Color::DarkGray)),
        ]), area);
        return;
    }

    // Detect if any game is a playoff game
    let has_playoffs = games.iter().any(|g| g.is_playoff());
    let has_regular  = games.iter().any(|g| !g.is_playoff());

    let section_label = match (has_playoffs, has_regular) {
        (true,  false) => "  PLAYOFFS",
        (false, true)  => "  REGULAR SEASON",
        _              => "  TONIGHT",
    };

    let dim   = Style::default().fg(Color::DarkGray);
    let gold  = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let cyan  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);

    let mut items: Vec<ratatui::widgets::ListItem> = vec![
        ratatui::widgets::ListItem::new(Line::styled(section_label, gold)),
        ratatui::widgets::ListItem::new(Line::styled(format!("  {}", "─".repeat(60)), dim)),
    ];

    for (i, game) in games.iter().enumerate() {
        let utc  = game.start_time_utc.get(11..16).unwrap_or("?");
        let et   = fmt_et(utc);
        let selected = i == app.scores_selected;

        // Build the main game line
        let series_info = if game.is_playoff() {
            game.series_label()
                .unwrap_or_else(|| format!("Game {}", game.series_game.as_deref().unwrap_or("?")))
        } else {
            String::new()
        };

        let game_line = if series_info.is_empty() {
            format!("  {:<5}  {:>4} @ {:<4}  {}", et, game.away_abbrev, game.home_abbrev, " ".repeat(30))
        } else {
            format!("  {:<5}  {:>4} @ {:<4}  {}", et, game.away_abbrev, game.home_abbrev, series_info)
        };

        let style = if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if game.is_playoff() {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        items.push(ratatui::widgets::ListItem::new(Line::styled(game_line, style)));

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
                let ctx_style = if selected { style } else { dim };
                items.push(ratatui::widgets::ListItem::new(Line::styled(ctx, ctx_style)));
            }
        }
    }

    items.push(ratatui::widgets::ListItem::new(Line::from("")));
    items.push(ratatui::widgets::ListItem::new(Line::styled(
        "  Times shown in ET  ·  data from NHL public API", dim
    )));

    f.render_widget(ratatui::widgets::List::new(items), area);
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

// ── Schedule (stub) ───────────────────────────────────────────────────────────

pub fn render_schedule_stub(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Schedule ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let cmd = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(""),
        Line::from("  Full season schedule — coming in v2."),
        Line::from(""),
        Line::styled("  In the meantime, use the CLI:", dim),
        Line::from(""),
        Line::styled("  icelines tonight", cmd),
        Line::styled("  icelines schedule --days 7", cmd),
        Line::styled("  icelines schedule --team SEA --days 14", cmd),
        Line::from(""),
        Line::styled("  Planned: team filter, matchup search (NYR WSH), date nav.", dim),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

// ── Playoffs (stub) ───────────────────────────────────────────────────────────

pub fn render_playoffs_stub(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Playoffs ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let hi  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(""),
        Line::styled("  Playoff bracket + series tracker — coming in v2.", hi),
        Line::from(""),
        Line::styled("  Will include:", dim),
        Line::styled("    · Live bracket with round-by-round progression", dim),
        Line::styled("    · Series detail: game log, leading scorers", dim),
        Line::styled("    · Historical Stanley Cup campaigns (time-travel with y)", dim),
        Line::styled("    · Projected playoff picture during regular season", dim),
    ];
    f.render_widget(Paragraph::new(lines), inner);
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
