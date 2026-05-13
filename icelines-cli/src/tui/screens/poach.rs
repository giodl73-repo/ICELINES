//! Fantasy poacher TUI board. Phase Selke.5.

use icelines_core::{
    model::Position,
    view_model::{
        AvailabilityState, PoachAvailabilityFilter, PoachBoardView, PoachCandidateKind, PoachQuery,
    },
};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashSet;

use crate::tui::app::App;
use crate::visual::{
    tui_header_style, tui_meta_style, tui_panel_block, tui_selected_style, tui_warning_style,
};

pub fn chrome() -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    ScreenChrome {
        title: "Poach - yahoo-standard - top adds".to_string(),
        keybinds: vec![
            KeyHint::new("up/down", "select"),
            KeyHint::new("Enter", "player card"),
            KeyHint::new("p", "poach filters"),
            KeyHint::new("w", "toggle watch"),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoachScreenState {
    pub positions: Vec<Position>,
    pub categories: Vec<String>,
    pub availability_filter: PoachAvailabilityFilter,
    pub candidate_kind: PoachCandidateKind,
    pub limit: u16,
}

impl Default for PoachScreenState {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            categories: Vec::new(),
            availability_filter: PoachAvailabilityFilter::Any,
            candidate_kind: PoachCandidateKind::All,
            limit: 30,
        }
    }
}

impl PoachScreenState {
    pub fn apply_to_query(&self, query: &mut PoachQuery) {
        query.positions = self.positions.clone();
        query.categories = self.categories.clone();
        query.availability_filter = self.availability_filter;
        query.candidate_kind = self.candidate_kind;
        query.limit = Some(self.limit);
        query.sort = Some("poach_score".to_string());
    }

    pub fn context_label(&self) -> String {
        let pos = if self.positions.is_empty() {
            "all-pos".to_string()
        } else {
            self.positions
                .iter()
                .map(|position| position.abbreviation())
                .collect::<Vec<_>>()
                .join("/")
        };
        let cats = if self.categories.is_empty() {
            "scheme-cats".to_string()
        } else {
            self.categories.join(",")
        };
        format!(
            "{} | {} | {} | {} | top {}",
            pos,
            cats,
            availability_filter_label(self.availability_filter),
            candidate_kind_label(self.candidate_kind),
            self.limit
        )
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = tui_panel_block(" Fantasy Poacher - adds, stashes, category fit ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let view = build_view(app);
    if let Some(empty) = &view.empty_state {
        let detail = empty.detail.as_deref().unwrap_or("");
        f.render_widget(
            Paragraph::new(format!("{}\n\n{}", empty.title, detail)).style(tui_warning_style()),
            inner,
        );
        return;
    }

    let watched = watchlist_members();
    let mut items = Vec::new();
    items.push(ListItem::new(Line::styled(
        format!("  {}", app.poach.context_label()),
        tui_meta_style(),
    )));
    items.push(ListItem::new(Line::styled(
        format!(
            "  {:<3} {:<2} {:<22} {:<4} {:<3} {:>5} {:<9} {:<11} {:<22} {}",
            "Rk", "W", "Player", "Team", "Pos", "Score", "Avail", "Conf", "Why", "Risk"
        ),
        tui_meta_style(),
    )));
    items.push(ListItem::new(Line::styled(
        format!("  {}", "-".repeat(92)),
        tui_meta_style(),
    )));

    for (idx, row) in view.rows.iter().enumerate() {
        let selected = idx == app.selected.min(view.rows.len().saturating_sub(1));
        let why = row
            .explanations
            .first()
            .map(|explanation| explanation.message.as_str())
            .unwrap_or("No explanation");
        let risk = row.risk_summary.as_deref().unwrap_or("-");
        let watch_mark = if app
            .repo
            .identity(row.player_id)
            .map(|identity| watched.contains(&identity.name_normalized))
            .unwrap_or(false)
        {
            "W"
        } else {
            "-"
        };
        let style = if selected {
            tui_selected_style()
        } else if row.risk_summary.is_some() {
            tui_warning_style()
        } else {
            tui_header_style()
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(
                "  {:<3} {:<2} {:<22} {:<4} {:<3} {:>5.1} {:<9} {:<11} {:<22} {}",
                idx + 1,
                watch_mark,
                truncate(&row.display_name, 22),
                row.team.as_str(),
                row.position.abbreviation(),
                row.score.final_score,
                availability_label(row.availability),
                format!("{:?}", row.confidence).to_ascii_lowercase(),
                truncate(why, 22),
                truncate(risk, 22),
            ),
            style,
        )])));
    }

    if view.context.completeness != icelines_core::Completeness::Complete {
        items.push(ListItem::new(Line::styled(
            "  Missing schedule/import/shift data is disclosed, not scored as negative evidence.",
            tui_meta_style(),
        )));
    }

    f.render_widget(List::new(items), inner);
}

pub fn selected_player_id(app: &App) -> Option<icelines_core::identity::PlayerId> {
    let view = build_view(app);
    view.rows
        .get(app.selected.min(view.rows.len().saturating_sub(1)))
        .map(|row| row.player_id)
}

pub fn selected_player(app: &App) -> Option<(String, String)> {
    selected_watch_target(app).map(|target| (target.normalized, target.display_name))
}

#[derive(Debug, Clone)]
pub struct WatchTarget {
    pub normalized: String,
    pub display_name: String,
    pub reason: String,
}

pub fn selected_watch_target(app: &App) -> Option<WatchTarget> {
    let view = build_view(app);
    view.rows
        .get(app.selected.min(view.rows.len().saturating_sub(1)))
        .and_then(|row| {
            let (normalized, display_name) = app
                .repo
                .identity(row.player_id)
                .map(|identity| (identity.name_normalized.clone(), identity.full_name.clone()))
                .or_else(|| {
                    Some((
                        row.display_name.to_ascii_lowercase(),
                        row.display_name.clone(),
                    ))
                })?;
            let explanation = row
                .explanations
                .first()
                .map(|explanation| explanation.message.as_str())
                .unwrap_or("Poach board candidate");
            let mut reason = format!(
                "Poach score {:.1}; confidence {:?}; {}",
                row.score.final_score, row.confidence, explanation
            );
            if let Some(risk) = &row.risk_summary {
                reason.push_str("; risk: ");
                reason.push_str(risk);
            }
            Some(WatchTarget {
                normalized,
                display_name,
                reason,
            })
        })
}

fn build_view(app: &App) -> PoachBoardView {
    let mut query = PoachQuery::new(app.active_season_typed, app.active_type, "yahoo-standard");
    app.poach.apply_to_query(&mut query);
    if let Some(rosters) = active_fantasy_rostered_player_keys() {
        query =
            query.with_imported_league_availability(rosters.all_rostered, rosters.user_rostered);
    }
    PoachBoardView::from_repository(&app.repo, query)
}

struct ActiveFantasyRosters {
    all_rostered: Vec<String>,
    user_rostered: Vec<String>,
}

fn active_fantasy_rostered_player_keys() -> Option<ActiveFantasyRosters> {
    let db = crate::fantasy_db::FantasyDb::open().ok()?;
    let league = db.get_active_league().ok()??;
    let user_team_id = db
        .get_user_team(&league.id)
        .ok()
        .flatten()
        .map(|team| team.id);
    let mut all_rostered = Vec::new();
    let mut user_rostered = Vec::new();
    for team in db.list_teams(&league.id).ok()? {
        let roster = db.list_roster(&team.id).ok()?;
        if Some(team.id.as_str()) == user_team_id.as_deref() {
            user_rostered.extend(roster.iter().cloned());
        }
        all_rostered.extend(roster);
    }
    Some(ActiveFantasyRosters {
        all_rostered,
        user_rostered,
    })
}

fn availability_label(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Unknown => "unknown",
        AvailabilityState::Available => "available",
        AvailabilityState::RosteredByUser => "my",
        AvailabilityState::ImportedAvailable => "free",
        AvailabilityState::ImportedRostered => "rostered",
        AvailabilityState::Watched => "watched",
    }
}

fn availability_filter_label(filter: PoachAvailabilityFilter) -> &'static str {
    match filter {
        PoachAvailabilityFilter::Any => "any",
        PoachAvailabilityFilter::Available => "available",
        PoachAvailabilityFilter::NotOnUserRoster => "not-my-roster",
        PoachAvailabilityFilter::Watched => "watched",
        PoachAvailabilityFilter::ImportedAvailable => "free",
        PoachAvailabilityFilter::Unknown => "unknown",
    }
}

fn candidate_kind_label(kind: PoachCandidateKind) -> &'static str {
    match kind {
        PoachCandidateKind::All => "all",
        PoachCandidateKind::Streamer => "streamers",
        PoachCandidateKind::Stash => "stashes",
        PoachCandidateKind::CategorySpecialist => "category",
        PoachCandidateKind::DeploymentRiser => "risers",
        PoachCandidateKind::GoalieStreamer => "goalie-streamers",
        PoachCandidateKind::WatchAlert => "watch-alerts",
    }
}

fn watchlist_members() -> HashSet<String> {
    crate::db::GroupDb::open()
        .ok()
        .and_then(|db| db.list_members("Watchlist").ok())
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut out = value.chars().take(max_chars - 3).collect::<String>();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_text(app: &App) -> String {
        let backend = TestBackend::new(120, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, app, f.area())).unwrap();
        let buf = term.backend().buffer();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn l0_poach_tui_empty_state_discloses_missing_source() {
        let app = App::new(true);
        let text = render_text(&app);

        assert!(text.contains("Fantasy Poacher"));
        assert!(text.contains("Missing poacher source data"));
    }

    #[test]
    fn l0_poach_tui_chrome_names_surface() {
        let chrome = super::chrome();

        assert!(chrome.title.contains("Poach"));
        assert!(chrome.keybinds.iter().any(|key| key.key == "p"));
        assert!(chrome.keybinds.iter().any(|key| key.key == "w"));
    }
}
