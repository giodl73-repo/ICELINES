//! Phase Foster.2 + Foster +21 — Favorites tab renderer.
//!
//! Reads the user's `Favorites` group + the day's slate + persisted
//! boxscore JSON, builds a `FavoritesView` via the shared builder,
//! and renders one row per favorited player + team with real
//! G/A/P/SOG/+/-/TOI / SV/SA/GAA. Empty group still shows an
//! instructional card. Boxscore not on disk → DNP row with reason.

use crate::tui::app::App;
use icelines_core::entity::EntityRef;
use icelines_core::favorites::{
    FavoritesView, GameResult, GoalieNightLine, HomeAway, PlayerNightRow, SkaterNightLine,
    TeamNightRow,
};
use icelines_core::identity::PlayerId;
use icelines_core::stats_repository::PlayerView;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FavoritesSort {
    #[default]
    RecentlyAdded,
    Name,
    Kind,
}

impl FavoritesSort {
    pub const ALL: &'static [FavoritesSort] = &[
        FavoritesSort::RecentlyAdded,
        FavoritesSort::Name,
        FavoritesSort::Kind,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FavoritesSort::RecentlyAdded => "Recent",
            FavoritesSort::Name => "Name",
            FavoritesSort::Kind => "Kind",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Default)]
pub struct FavoritesScreenState {
    pub sort: FavoritesSort,
    pub filters: crate::tui::filter_state::RosterFilterState,
}

// ── Phase Adams.11 — chrome accessor ─────────────────────────────────────────

pub fn chrome(state: &FavoritesScreenState) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::KeyHint;
    crate::tui::chrome::ScreenChrome {
        title: format!(
            "Favorites - sort={} - pos={} - country={}",
            state.sort.label(),
            state.filters.pos_filter.label(),
            state.filters.country_label()
        ),
        keybinds: vec![
            KeyHint::new("s", "sort"),
            KeyHint::new("p", "cycle pos"),
            KeyHint::new("n", "cycle nation"),
            KeyHint::new("f", "free filter"),
            KeyHint::new("g", "manage groups"),
            KeyHint::new("Enter", "open card"),
            KeyHint::new("fav add", "cmdbar"),
        ],
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    render_header(f, chunks[0]);

    let db = match crate::db::GroupDb::open() {
        Ok(db) => db,
        Err(err) => {
            render_error_state(f, chunks[1], "Could not open Favorites", &err.to_string());
            return;
        }
    };
    let members = match db.list_members_with_kind("Favorites") {
        Ok(members) => members,
        Err(err) => {
            render_error_state(f, chunks[1], "Could not read Favorites", &err.to_string());
            return;
        }
    };

    if members.is_empty() {
        render_empty_state(f, chunks[1]);
        return;
    }

    match build_view(app, &db) {
        Ok(mut view) => {
            apply_favorites_state(&mut view, app);
            render_view(f, chunks[1], &view);
        }
        Err(BuildViewError::NoHome) => {
            render_error_state(
                f,
                chunks[1],
                "Could not locate IceLines data",
                "HOME/USERPROFILE is not set, so Favorites cannot find ~/.icelines/data.",
            );
        }
        Err(BuildViewError::Compute(err)) => {
            render_error_state(f, chunks[1], "Could not build Favorites", &err.to_string());
        }
    }
}

/// Build a populated `FavoritesView` if we can. Phase Foster +22:
/// reads today's slate from the TUI's existing `tonight_cache` so
/// favorites auto-populate without requiring `icelines fetch boxscore`
/// upfront. If the cache is cold (user hasn't opened the Scores tab
/// yet), fires `maybe_fetch` in the background and renders with the
/// empty slate — next render after the fetch lands will populate.
#[derive(Debug)]
enum BuildViewError {
    NoHome,
    Compute(anyhow::Error),
}

fn build_view(app: &App, db: &crate::db::GroupDb) -> Result<FavoritesView, BuildViewError> {
    let date = chrono::Utc::now().date_naive();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or(BuildViewError::NoHome)?;
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
        db,
        "Favorites",
        date,
        icelines_core::timeframe::Timeframe::Day,
        &slate,
        &data_root,
    )
    .map_err(BuildViewError::Compute)
}

fn apply_favorites_state(view: &mut FavoritesView, app: &App) {
    let views = active_player_views(app);
    if filters_active(&app.favorites.filters) {
        view.players
            .retain(|row| player_row_matches_filters(row, &app.favorites.filters, &views));
    }
    sort_player_rows(&mut view.players, app.favorites.sort);
}

fn active_player_views(app: &App) -> Vec<PlayerView<'_>> {
    let mut views = app.views();
    views.extend(app.goalie_views());
    views
}

fn filters_active(filters: &crate::tui::filter_state::RosterFilterState) -> bool {
    filters.pos_filter != crate::tui::filter_state::PosFilter::All
        || filters.country_filter.is_some()
        || filters.min_gp > 0
}

fn player_row_matches_filters(
    row: &PlayerNightRow,
    filters: &crate::tui::filter_state::RosterFilterState,
    views: &[PlayerView<'_>],
) -> bool {
    row_player_id(row)
        .and_then(|pid| views.iter().find(|view| view.id() == pid))
        .map(|view| filters.matches_view(view))
        .unwrap_or(false)
}

fn row_player_id(row: &PlayerNightRow) -> Option<PlayerId> {
    match row {
        PlayerNightRow::Skater(line) => entity_player_id(&line.player),
        PlayerNightRow::Goalie(line) => entity_player_id(&line.player),
        PlayerNightRow::DidNotPlay { player, .. } => entity_player_id(player),
    }
}

fn entity_player_id(entity: &EntityRef) -> Option<PlayerId> {
    match entity {
        EntityRef::Player(pid) => Some(*pid),
        _ => None,
    }
}

fn sort_player_rows(rows: &mut [PlayerNightRow], sort: FavoritesSort) {
    match sort {
        FavoritesSort::RecentlyAdded => {}
        FavoritesSort::Name => rows.sort_by_key(player_row_name),
        FavoritesSort::Kind => {
            rows.sort_by_key(|row| (player_row_kind_rank(row), player_row_name(row)))
        }
    }
}

fn player_row_kind_rank(row: &PlayerNightRow) -> u8 {
    match row {
        PlayerNightRow::Skater(_) => 0,
        PlayerNightRow::Goalie(_) => 1,
        PlayerNightRow::DidNotPlay { .. } => 2,
    }
}

fn player_row_name(row: &PlayerNightRow) -> String {
    match row {
        PlayerNightRow::Skater(line) => line.player.to_string(),
        PlayerNightRow::Goalie(line) => line.player.to_string(),
        PlayerNightRow::DidNotPlay { player, .. } => player.to_string(),
    }
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
            "  Per-night stat lines appear after schedule/boxscore data is available;",
            dim,
        )),
        Line::from(Span::styled(
            "  run `icelines fetch boxscore` or open Scores first to warm the cache.",
            dim,
        )),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_error_state(f: &mut Frame, area: Rect, title: &str, detail: &str) {
    let block = Block::default().borders(Borders::ALL).title(" Favorites ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let error = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {title}"), error)),
        Line::from(""),
        Line::from(Span::styled(format!("  {detail}"), dim)),
        Line::from(""),
        Line::from(Span::styled(
            "  This is different from an empty Favorites group; fix the storage error and reopen the tab.",
            dim,
        )),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_messier_favorites_sort_cycles() {
        let mut sort = FavoritesSort::default();
        assert_eq!(sort, FavoritesSort::RecentlyAdded);
        sort = sort.next();
        assert_eq!(sort, FavoritesSort::Name);
        sort = sort.next();
        assert_eq!(sort, FavoritesSort::Kind);
        sort = sort.next();
        assert_eq!(sort, FavoritesSort::RecentlyAdded);
    }

    #[test]
    fn l0_messier_favorites_chrome_advertises_filters() {
        let state = FavoritesScreenState::default();
        let chrome = chrome(&state);
        assert!(chrome.title.contains("sort=Recent"));
        assert!(chrome.title.contains("pos=All"));
        assert!(chrome.title.contains("country=All"));
        let keys: Vec<&str> = chrome.keybinds.iter().map(|k| k.key).collect();
        assert!(keys.contains(&"s"));
        assert!(keys.contains(&"p"));
        assert!(keys.contains(&"n"));
        assert!(keys.contains(&"f"));
        assert!(keys.contains(&"fav add"));
        assert!(!keys.contains(&":fav add"));
    }

    #[test]
    fn l0_audit_error_state_distinct_from_empty_state_text() {
        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut terminal = ratatui::Terminal::new(backend).expect("terminal");

        terminal
            .draw(|f| render_error_state(f, f.area(), "Could not read Favorites", "disk full"))
            .expect("draw");

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("Could not read Favorites"));
        assert!(text.contains("disk full"));
        assert!(!text.contains("No favorites yet."));
    }

    #[test]
    fn l0_audit_empty_state_remains_instructional() {
        let backend = ratatui::backend::TestBackend::new(80, 16);
        let mut terminal = ratatui::Terminal::new(backend).expect("terminal");

        terminal
            .draw(|f| render_empty_state(f, f.area()))
            .expect("draw");

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("No favorites yet."));
        assert!(!text.contains("Could not read Favorites"));
    }

    #[test]
    fn l0_messier_favorites_player_rows_filter_by_active_views() {
        use icelines_core::fixtures;
        use icelines_core::model::{Position, Season};
        use icelines_core::season_stats::SeasonType;

        let repo = fixtures::test_repo_with(
            fixtures::identity(1).build(),
            fixtures::stats(1, 20242025, "EDM")
                .position(Position::LeftWing)
                .build(),
        );
        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let mut filters = crate::tui::filter_state::RosterFilterState::default();
        filters.pos_filter = crate::tui::filter_state::PosFilter::LW;
        filters.country_filter = Some(crate::tui::filter_state::CountryCode::CAN);
        let row = PlayerNightRow::DidNotPlay {
            player: EntityRef::Player(PlayerId(1)),
            reason: icelines_core::favorites::DnpReason::TeamBye,
        };

        assert!(player_row_matches_filters(&row, &filters, &views));

        filters.pos_filter = crate::tui::filter_state::PosFilter::Defense;
        assert!(!player_row_matches_filters(&row, &filters, &views));
    }
}
