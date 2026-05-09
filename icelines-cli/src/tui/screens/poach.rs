//! Fantasy poacher TUI board. Phase Selke.5.

use icelines_core::view_model::{PoachBoardView, PoachQuery};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashSet;

use crate::tui::app::App;

pub fn chrome() -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    ScreenChrome {
        title: "Poach - yahoo-standard - top adds".to_string(),
        keybinds: vec![
            KeyHint::new("up/down", "select"),
            KeyHint::new("Enter", "player card"),
            KeyHint::new("w", "toggle watch"),
        ],
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Fantasy Poacher - adds, stashes, category fit ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let view = build_view(app);
    if let Some(empty) = &view.empty_state {
        let detail = empty.detail.as_deref().unwrap_or("");
        f.render_widget(
            Paragraph::new(format!("{}\n\n{}", empty.title, detail))
                .style(Style::default().fg(Color::Yellow)),
            inner,
        );
        return;
    }

    let watched = watchlist_members();
    let mut items = Vec::new();
    items.push(ListItem::new(Line::styled(
        format!(
            "  {:<3} {:<2} {:<24} {:<4} {:<3} {:>5} {:<11} {:<24} {}",
            "Rk", "W", "Player", "Team", "Pos", "Score", "Conf", "Why", "Risk"
        ),
        Style::default().fg(Color::DarkGray),
    )));
    items.push(ListItem::new(Line::styled(
        format!("  {}", "-".repeat(92)),
        Style::default().fg(Color::DarkGray),
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
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if row.risk_summary.is_some() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(
                "  {:<3} {:<2} {:<24} {:<4} {:<3} {:>5.1} {:<11} {:<24} {}",
                idx + 1,
                watch_mark,
                truncate(&row.display_name, 24),
                row.team.as_str(),
                row.position.abbreviation(),
                row.score.final_score,
                format!("{:?}", row.confidence).to_ascii_lowercase(),
                truncate(why, 24),
                truncate(risk, 24),
            ),
            style,
        )])));
    }

    if view.context.completeness != icelines_core::Completeness::Complete {
        items.push(ListItem::new(Line::styled(
            "  Missing schedule/import/shift data is disclosed, not scored as negative evidence.",
            Style::default().fg(Color::DarkGray),
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
    query.limit = Some(30);
    query.sort = Some("poach_score".to_string());
    PoachBoardView::from_repository(&app.repo, query)
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
        assert!(chrome.keybinds.iter().any(|key| key.key == "w"));
    }
}
