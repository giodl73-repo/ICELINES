//! TUI screens: Tonight, Projections, Groups, Fetch+Install.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::app::App;

// ── Tonight ───────────────────────────────────────────────────────────────────

pub fn render_tonight(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Tonight's Games ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let cmd = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(""),
        Line::from("  Tonight's schedule is fetched live from the NHL API."),
        Line::from("  Run these in your terminal:"),
        Line::from(""),
        Line::styled("  icelines tonight", cmd),
        Line::styled("  → all games tonight with UTC start times", dim),
        Line::from(""),
        Line::styled("  icelines tonight --team EDM", cmd),
        Line::styled("  → games involving a specific team", dim),
        Line::from(""),
        Line::styled("  icelines schedule --days 7", cmd),
        Line::styled("  → upcoming schedule for the next week", dim),
        Line::from(""),
        Line::styled("  icelines schedule --team SEA --days 3", cmd),
        Line::styled("  → upcoming home/away games for one team", dim),
        Line::from(""),
        Line::from("  The NHL public API is free — no key required."),
    ];
    f.render_widget(Paragraph::new(lines), inner);
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
