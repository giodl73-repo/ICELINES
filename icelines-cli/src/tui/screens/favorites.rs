//! Phase Foster.2 — Favorites tab renderer.
//!
//! Reads the user's `Favorites` group from the SQLite db and lays
//! out a header + member list + a one-line empty-state nudge when
//! the group is empty. Per-night stat lines + boxscore-driven event
//! rows wire in once Foster.3+ orchestration lands; the tab itself
//! is here today so users have a place to land.

use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, _app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    render_header(f, chunks[0]);

    // Read members lazily on each render — group sizes are small and
    // SQLite open is fast. Future versions can cache this.
    let members = match crate::db::GroupDb::open() {
        Ok(db) => db.list_members_with_kind("Favorites").unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    if members.is_empty() {
        render_empty_state(f, chunks[1]);
    } else {
        render_member_list(f, chunks[1], &members);
    }
}

fn render_header(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Favorites ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let lines = vec![
        Line::from(Span::styled(
            "Your favorited players + teams.",
            Style::default().fg(Color::Cyan),
        )),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let hint = Style::default().fg(Color::Yellow);
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  No favorites yet.", hint)),
        Line::from(""),
        Line::from(Span::styled(
            "  Press `g` on any player or team to add it",
            dim,
        )),
        Line::from(Span::styled(
            "  (lowercase g = group picker, lowercase f = instant Favorites add).",
            dim,
        )),
        Line::from(""),
        Line::from(Span::styled("  Or run from the CLI:", dim)),
        Line::from(Span::styled(
            "    icelines group add Favorites \"Connor McDavid\"",
            dim,
        )),
        Line::from(Span::styled(
            "    icelines group add Favorites EDM",
            dim,
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Per-night stat lines + box scores ship in a follow-up;",
            dim,
        )),
        Line::from(Span::styled(
            "  this tab is here so favorites land somewhere visible.",
            dim,
        )),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_member_list(
    f: &mut Frame,
    area: Rect,
    members: &[(String, crate::db::MemberKind)],
) {
    let player_count = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Player))
        .count();
    let team_count = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Team))
        .count();
    let title = format!(
        " Favorites — {player_count} player(s), {team_count} team(s) "
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Players first, then teams, alphabetical within each.
    let mut players: Vec<&str> = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Player))
        .map(|(k, _)| k.as_str())
        .collect();
    players.sort_unstable();
    let mut teams: Vec<&str> = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Team))
        .map(|(k, _)| k.as_str())
        .collect();
    teams.sort_unstable();

    let mut items: Vec<ListItem> = Vec::with_capacity(members.len() + 4);
    if !players.is_empty() {
        items.push(ListItem::new(Span::styled(
            "PLAYERS",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        for p in players {
            items.push(ListItem::new(format!("  · {p}")));
        }
    }
    if !teams.is_empty() {
        if !items.is_empty() {
            items.push(ListItem::new(""));
        }
        items.push(ListItem::new(Span::styled(
            "TEAMS",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        for t in teams {
            items.push(ListItem::new(format!("  · {t}")));
        }
    }
    items.push(ListItem::new(""));
    items.push(ListItem::new(Span::styled(
        "  Tonight's stat lines wire in via `icelines fetch boxscore`",
        Style::default().fg(Color::DarkGray),
    )));
    items.push(ListItem::new(Span::styled(
        "  (Foster.3+ orchestration — coming soon).",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(List::new(items), inner);
}
