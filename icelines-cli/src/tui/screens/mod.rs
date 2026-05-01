pub mod home;
pub mod team;
pub mod player;
pub mod search;
pub mod misc;
pub mod queries;
pub mod comps;
pub mod depth;
pub mod schedule;
pub mod playoffs;
pub mod game_detail;
pub mod goalies;
pub mod transactions;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use crate::tui::app::{App, Screen};
use crate::tui::widgets::help_lines;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_nav(f, app, chunks[0]);

    match &app.screen {
        Screen::Home            => home::render(f, app, chunks[1]),
        Screen::Team(abbrev)    => team::render(f, app, chunks[1], abbrev),
        Screen::Player(idx)     => player::render(f, app, chunks[1], *idx),
        Screen::Search          => search::render(f, app, chunks[1]),
        Screen::Queries         => queries::render(f, app, chunks[1]),
        Screen::Tonight         => misc::render_tonight(f, app, chunks[1]),
        Screen::Projections     => misc::render_projections(f, app, chunks[1]),
        Screen::Groups              => misc::render_groups(f, app, chunks[1]),
        Screen::GroupDetail(name)   => misc::render_group_members(f, app, chunks[1], name),
        Screen::Fetch               => misc::render_fetch(f, app, chunks[1]),
        Screen::Help                => home::render(f, app, chunks[1]),
        Screen::Comps(idx)          => comps::render(f, app, chunks[1], *idx),
        Screen::CompsById(pid)      => comps::render_by_id(f, app, chunks[1], *pid),
        Screen::Depth               => depth::render_league(f, app, chunks[1]),
        Screen::DepthTeam(abbrev)   => depth::render_team(f, app, chunks[1], abbrev),
        Screen::Schedule                  => schedule::render(f, app, chunks[1]),
        Screen::ScheduleTeam(team)        => schedule::render_team_schedule(f, app, chunks[1], team),
        Screen::ScheduleMatchup(t1, t2)   => schedule::render_matchup(f, app, chunks[1], t1, t2),
        Screen::Playoffs                  => playoffs::render(f, app, chunks[1]),
        Screen::SeriesDetail(letter)      => playoffs::render_series_detail(f, app, chunks[1], letter),
        Screen::GameDetail(game_id)       => game_detail::render(f, app, chunks[1], *game_id),
        Screen::Goalies                   => goalies::render(f, app, chunks[1]),
        Screen::GoalieDetail(idx)         => goalies::render_detail(f, app, chunks[1], *idx),
        Screen::Transactions              => transactions::render(f, app, chunks[1]),
    }

    f.render_widget(
        Paragraph::new(app.status.as_str()).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

    // Group picker overlay — shown on any player-list screen when g is pressed.
    // Rendered at top level so it floats over the current screen.
    // (player.rs and team.rs also call this, but those handle it locally.
    //  This catches Projections, Search, Queries, GroupDetail.)
    if app.group_picker_open {
        // Skip if player/team screen — they render the overlay themselves
        let handled_locally = matches!(app.screen,
            Screen::Player(_) | Screen::Team(_)
        );
        if !handled_locally {
            player::render_group_picker(f, app, area);
        }
    }

    if app.show_help {
        let popup = centered_rect(62, 65, area);
        f.render_widget(Clear, popup);
        let block = Block::default()
            .title(" Help — any key to close ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(popup);
        f.render_widget(block, popup);
        f.render_widget(Paragraph::new(help_lines()), inner);
    }

    if app.show_admin {
        let popup = centered_rect(44, 50, area);
        f.render_widget(Clear, popup);
        let block = Block::default()
            .title(" Admin — Esc to close ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));
        let inner = block.inner(popup);
        f.render_widget(block, popup);
        misc::render_admin(f, app, inner);
    }

    if app.show_season_picker {
        misc::render_season_picker(f, app, area);
    }
}

fn tab_for_screen(screen: &Screen) -> usize {
    match screen {
        Screen::Home | Screen::Team(_) | Screen::Player(_)
        | Screen::Comps(_) | Screen::CompsById(_)                => 0, // League
        Screen::Depth | Screen::DepthTeam(_)                     => 1, // Depth
        Screen::Queries | Screen::Projections | Screen::Search   => 2, // Stats (default: Queries)
        Screen::Goalies | Screen::GoalieDetail(_)                => 3, // Goalies
        Screen::Tonight | Screen::GameDetail(_)                  => 4, // Scores
        Screen::Schedule | Screen::ScheduleTeam(_)
            | Screen::ScheduleMatchup(..)                        => 5, // Schedule
        Screen::Transactions                                     => 6, // Transactions
        Screen::Playoffs | Screen::SeriesDetail(_)               => 7, // Playoffs
        // Groups is not a tab (Phase T+1): reachable via `g` from anywhere.
        _                                                        => 99,// no tab (Fetch, Help, Groups)
    }
}

fn render_nav(f: &mut Frame, app: &App, area: Rect) {
    let tab_labels = [
        "League", "Depth", "Stats", "Goalies", "Scores",
        "Schedule", "Transactions", "Playoffs",
    ];
    let active_tab = tab_for_screen(&app.screen);

    let mut spans: Vec<Span> = Vec::new();
    for (i, label) in tab_labels.iter().enumerate() {
        let active = i == active_tab;
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {label} "), style));
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
    }

    // Season indicator — shown when a historical season is active
    if app.active_season != icelines_core::CURRENT_SEASON_STR {
        let label = crate::tui::screens::misc::PICKER_SEASONS.iter()
            .find(|(id, _, _)| *id == app.active_season.as_str())
            .map(|(_, l, _)| *l)
            .unwrap_or(app.active_season.as_str());
        spans.push(Span::styled(
            format!("  [{}] ", label.split_whitespace().next().unwrap_or(label)),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    let hint = if app.show_admin {
        "  Esc:close admin"
    } else if app.show_season_picker {
        "  Esc:cancel picker"
    } else {
        "  g:groups  y:season  F:admin  Tab:cycle  ?:help  q:quit"
    };
    spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn centered_rect(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let popup_h = r.height * pct_y / 100;
    let popup_w = r.width  * pct_x / 100;
    Rect::new(
        r.x + (r.width  - popup_w) / 2,
        r.y + (r.height - popup_h) / 2,
        popup_w,
        popup_h,
    )
}
