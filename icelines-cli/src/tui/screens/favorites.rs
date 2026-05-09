//! Phase Foster.2 + Foster +21 — Favorites tab renderer.
//!
//! Reads the user's `Favorites` group + the day's slate + persisted
//! boxscore JSON, builds a `FavoritesView` via the shared builder,
//! and renders one row per favorited player + team with real
//! G/A/P/SOG/+/-/TOI / SV/SA/GAA. Empty group still shows an
//! instructional card. Boxscore not on disk → DNP row with reason.

use crate::tui::app::App;
use icelines_core::favorites::{
    FavoritesView, GameResult, GoalieNightLine, HomeAway, PlayerNightRow, SkaterNightLine,
    TeamNightRow,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

// ── Phase Adams.11 — chrome accessor ─────────────────────────────────────────

pub fn chrome() -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::KeyHint;
    crate::tui::chrome::ScreenChrome {
        title: "Favorites".to_owned(),
        keybinds: vec![
            KeyHint::new("g", "manage groups"),
            KeyHint::new("Enter", "open card"),
            KeyHint::new(":fav add", "from cmdbar"),
        ],
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
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
    } else if let Some(view) = build_view(app) {
        render_view(f, chunks[1], &view);
    } else {
        render_member_list(f, chunks[1], &members);
    }
}

/// Build a populated `FavoritesView` if we can. Phase Foster +22:
/// reads today's slate from the TUI's existing `tonight_cache` so
/// favorites auto-populate without requiring `icelines fetch boxscore`
/// upfront. If the cache is cold (user hasn't opened the Scores tab
/// yet), fires `maybe_fetch` in the background and renders with the
/// empty slate — next render after the fetch lands will populate.
fn build_view(app: &App) -> Option<FavoritesView> {
    let db = crate::db::GroupDb::open().ok()?;
    let date = chrono::Utc::now().date_naive();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)?;
    let data_root = home.join(".icelines").join("data");

    // Pull today's slate from the existing tonight_cache. Favorites
    // tab piggybacks on the Scores tab's fetch state — if the user
    // visits Favorites first, we kick off the fetch here so by the
    // next render the slate is populated.
    use crate::tui::tonight::{lookup, maybe_fetch, TonightState, TODAY_KEY};
    let slate: Vec<icelines_fetch::nhl_api::ScheduledGame> = {
        match lookup(&app.tonight.cache, TODAY_KEY) {
            TonightState::Loaded(games) => {
                let today_str = date.format("%Y-%m-%d").to_string();
                games.into_iter().filter(|g| g.date == today_str).collect()
            }
            TonightState::Idle => {
                maybe_fetch(app.tonight.cache.clone(), TODAY_KEY.to_string());
                Vec::new()
            }
            // Loading / Error → no slate this render; renderer
            // shows DNP rows. Status bar (chunks[2]) already
            // surfaces network errors.
            _ => Vec::new(),
        }
    };

    crate::favorites_view::compute_favorites_view(
        &db,
        "Favorites",
        date,
        icelines_core::timeframe::Timeframe::Day,
        &slate,
        &data_root,
    )
    .ok()
}

fn render_view(f: &mut Frame, area: Rect, view: &FavoritesView) {
    let block = Block::default().borders(Borders::ALL).title(format!(
        " Favorites — {} ({} player(s), {} team(s)) ",
        view.date.format("%Y-%m-%d (%a)"),
        view.players.len(),
        view.teams.len(),
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut items: Vec<ListItem> = Vec::with_capacity(view.players.len() + view.teams.len() + 4);
    if !view.players.is_empty() {
        items.push(ListItem::new(Span::styled(
            "PLAYERS",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for row in &view.players {
            items.push(ListItem::new(format_player_row_line(row)));
        }
    }
    if !view.teams.is_empty() {
        if !items.is_empty() {
            items.push(ListItem::new(""));
        }
        items.push(ListItem::new(Span::styled(
            "TEAMS",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for row in &view.teams {
            items.push(ListItem::new(format_team_row_line(row)));
        }
    }
    items.push(ListItem::new(""));
    items.push(ListItem::new(Span::styled(
        "  Refresh tonight's lines: `icelines fetch boxscore`",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(List::new(items), inner);
}

fn format_player_row_line(row: &PlayerNightRow) -> Line<'_> {
    let dim = Style::default().fg(Color::DarkGray);
    let win = Style::default().fg(Color::Green);
    let loss = Style::default().fg(Color::Red);
    match row {
        PlayerNightRow::Skater(s) => {
            let result_style = match s.result {
                GameResult::Win => win,
                GameResult::Loss | GameResult::OtLoss => loss,
                _ => Style::default(),
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{}", s.player),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::raw(matchup_str(s)),
                Span::raw(" "),
                Span::styled(
                    format!("{}-{}", s.team_score, s.opponent_score),
                    result_style,
                ),
                Span::raw(format!(
                    "  {}G {}A {}P  {:+}  TOI {}  ",
                    s.goals,
                    s.assists,
                    s.points,
                    s.plus_minus,
                    s.toi_seconds
                        .map(|t| format!("{}:{:02}", t / 60, t % 60))
                        .unwrap_or_else(|| "—".into()),
                )),
                Span::styled(
                    format!(
                        "{} SOG · {} hits · {} blk",
                        s.shots.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
                        s.hits.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
                        s.blocks
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "—".into()),
                    ),
                    dim,
                ),
            ])
        }
        PlayerNightRow::Goalie(g) => Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}", g.player),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  {} {}-{}  {}/{} SV · SV%{:.3} · GAA {:.2}{}",
                goalie_matchup(g),
                g.team_score,
                g.opponent_score,
                g.saves,
                g.shots_against,
                g.save_pct,
                g.gaa,
                if g.shutout { " · SO" } else { "" },
            )),
        ]),
        PlayerNightRow::DidNotPlay { player, reason } => Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{player}"), dim),
            Span::raw("  "),
            Span::styled(format!("DNP — {reason:?}"), dim),
        ]),
    }
}

fn matchup_str(s: &SkaterNightLine) -> String {
    match s.home_or_away {
        HomeAway::Home => format!("{} vs {}", s.team.0, s.opponent.0),
        HomeAway::Away => format!("{} @ {}", s.team.0, s.opponent.0),
    }
}

fn goalie_matchup(g: &GoalieNightLine) -> String {
    match g.home_or_away {
        HomeAway::Home => format!("{} vs {}", g.team.0, g.opponent.0),
        HomeAway::Away => format!("{} @ {}", g.team.0, g.opponent.0),
    }
}

fn format_team_row_line(t: &TeamNightRow) -> Line<'_> {
    let dim = Style::default().fg(Color::DarkGray);
    let bold = Style::default().add_modifier(Modifier::BOLD);
    if t.on_bye {
        return Line::from(vec![
            Span::raw("  "),
            Span::styled(t.team_abbr.0.clone(), bold),
            Span::raw("  "),
            Span::styled("bye", dim),
        ]);
    }
    let opp = t
        .opponent
        .as_ref()
        .map(|o| o.0.as_str())
        .unwrap_or("—")
        .to_string();
    let result = match t.result {
        Some(GameResult::Win) => "W",
        Some(GameResult::Loss) => "L",
        Some(GameResult::OtLoss) => "OTL",
        Some(GameResult::InProgress) => "LIVE",
        None => "—",
    };
    Line::from(vec![
        Span::raw("  "),
        Span::styled(t.team_abbr.0.clone(), bold),
        Span::raw("  "),
        Span::raw(if t.score.is_empty() {
            "—".to_string()
        } else {
            t.score.clone()
        }),
        Span::raw(format!("  {result} vs {opp}")),
    ])
}

fn render_header(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Favorites ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let lines = vec![Line::from(Span::styled(
        "Your favorited players + teams.",
        Style::default().fg(Color::Cyan),
    ))];
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
        Line::from(Span::styled("    icelines group add Favorites EDM", dim)),
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

fn render_member_list(f: &mut Frame, area: Rect, members: &[(String, crate::db::MemberKind)]) {
    let player_count = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Player))
        .count();
    let team_count = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Team))
        .count();
    let title = format!(" Favorites — {player_count} player(s), {team_count} team(s) ");
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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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
