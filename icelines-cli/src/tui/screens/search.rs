use crate::tui::app::App;
use icelines_core::name::normalize_name;
use icelines_core::stats_repository::PlayerView;
use ratatui::{
    layout::Rect,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Cap on results shown — same limit pre and post migration.
const MAX_RESULTS: usize = 20;

/// Filter + sort the views for the Search screen. Hart.5c.6 Phase B-1
/// pulls this into a pure function so the sort order is testable and
/// the render path is just a formatter.
///
/// Sort order per the 5c.6 v0.7 spec snapshot-determinism table:
///   pace_82 desc (None last), full_name asc as tiebreak.
///
/// Empty query → top MAX_RESULTS by pace.
/// Non-empty query → matches by `name_normalized.contains(q)`, then
/// the same sort, capped at MAX_RESULTS.
pub fn search_results<'a>(views: &'a [PlayerView<'a>], query: &str) -> Vec<&'a PlayerView<'a>> {
    let q = normalize_name(query);
    let mut filtered: Vec<&PlayerView<'a>> = if q.is_empty() {
        views.iter().collect()
    } else {
        views
            .iter()
            .filter(|v| v.identity.name_normalized.contains(&q))
            .collect()
    };
    // pace_82 desc with None LAST; full_name asc tiebreak.
    filtered.sort_by(|a, b| {
        let pa = a.pace_82();
        let pb = b.pace_82();
        // None goes after Some — invert the natural Option ordering.
        match (pa, pb) {
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
            (None, None) => std::cmp::Ordering::Equal,
        }
        .then_with(|| a.full_name().cmp(b.full_name()))
    });
    filtered.truncate(MAX_RESULTS);
    filtered
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search input box
    let input = Paragraph::new(format!("/ {}_", app.search_query)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search Players "),
    );
    f.render_widget(input, chunks[0]);

    // Hart.5c.6 Phase B-1: collect views once per frame, filter+sort
    // via the pure helper. `views` lives only inside this function
    // frame — no struct field, no lifetime infection.
    let views = app.views();
    let results = search_results(&views, &app.search_query);

    let items: Vec<ListItem> = results
        .iter()
        .map(|v| {
            let ppg = v
                .pace_82()
                .map(|p| format!("{:.2}", p / 82.0))
                .unwrap_or_else(|| "—".to_owned());
            ListItem::new(Line::from(format!(
                "  {:<22} {:<5} {:<4}  {} pts/gp",
                v.full_name().chars().take(22).collect::<String>(),
                v.team_display(),
                v.position().abbreviation(),
                ppg,
            )))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected.min(results.len().saturating_sub(1))));

    let label = if app.search_query.trim().is_empty() {
        format!(" Top {} players ", results.len())
    } else {
        format!(" {} matches for '{}' ", results.len(), app.search_query)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(label))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[1], &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::fixtures;
    use icelines_core::stats_repository::StatsRepository;

    fn build_repo(seeds: &[(u32, &str, &str, f64)]) -> StatsRepository {
        // (player_id, full_name, team, pace_82)
        let mut r = StatsRepository::new();
        for &(id, name, team, pace) in seeds {
            // Use the production normalizer so diacritic stripping is
            // exercised in tests (Slafkovský → "slafkovsky").
            let normalized = icelines_core::name::normalize_name(name);
            let identity = fixtures::identity(id).name(name, &normalized).build();
            // Hand-build StatsFixture for arbitrary pace; the default
            // fixture hardcodes pace_82=93.7.
            let mut stats = fixtures::stats(id, 20242025, team).build();
            if let Some(ref mut ps) = stats.totals.pace_score {
                ps.pace_82 = pace;
            }
            r.upsert_identity(identity).unwrap();
            r.upsert_stats(stats).unwrap();
        }
        r
    }

    #[test]
    fn l0_search_empty_query_returns_top_by_pace_desc() {
        let repo = build_repo(&[
            (1, "Alice Anderson", "EDM", 60.0),
            (2, "Bob Brown", "TOR", 90.0),
            (3, "Cara Carter", "BOS", 75.0),
        ]);
        let views: Vec<_> = repo
            .skaters(
                icelines_core::model::Season(20242025),
                icelines_core::season_stats::SeasonType::Regular,
            )
            .collect();
        let results = search_results(&views, "");

        // Top 3 by pace_82 desc: Bob (90) > Cara (75) > Alice (60).
        let names: Vec<&str> = results.iter().map(|v| v.full_name()).collect();
        assert_eq!(names, vec!["Bob Brown", "Cara Carter", "Alice Anderson"]);
    }

    #[test]
    fn l0_search_nonempty_query_filters_by_name_normalized() {
        let repo = build_repo(&[
            (1, "Connor McDavid", "EDM", 138.0),
            (2, "Nikita Kucherov", "TBL", 140.0),
            (3, "Auston Matthews", "TOR", 110.0),
        ]);
        let views: Vec<_> = repo
            .skaters(
                icelines_core::model::Season(20242025),
                icelines_core::season_stats::SeasonType::Regular,
            )
            .collect();
        let results = search_results(&views, "mcdavid");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].full_name(), "Connor McDavid");
    }

    #[test]
    fn l0_search_caps_at_max_results() {
        let mut seeds: Vec<(u32, &str, &str, f64)> = Vec::new();
        // 25 players named "Player N" with descending pace; expect
        // exactly MAX_RESULTS (20) returned.
        let names: Vec<String> = (0..25).map(|i| format!("Player {i:02}")).collect();
        for (i, name) in names.iter().enumerate() {
            seeds.push((1000 + i as u32, name.as_str(), "EDM", 100.0 - i as f64));
        }
        let repo = build_repo(&seeds);
        let views: Vec<_> = repo
            .skaters(
                icelines_core::model::Season(20242025),
                icelines_core::season_stats::SeasonType::Regular,
            )
            .collect();
        let results = search_results(&views, "");
        assert_eq!(results.len(), MAX_RESULTS);
    }

    #[test]
    fn l0_search_full_name_tiebreak_when_pace_equal() {
        let repo = build_repo(&[
            (1, "Bravo", "EDM", 80.0),
            (2, "Alpha", "EDM", 80.0),
            (3, "Charlie", "EDM", 80.0),
        ]);
        let views: Vec<_> = repo
            .skaters(
                icelines_core::model::Season(20242025),
                icelines_core::season_stats::SeasonType::Regular,
            )
            .collect();
        let results = search_results(&views, "");
        // All three have identical pace; full_name asc tiebreak.
        let names: Vec<&str> = results.iter().map(|v| v.full_name()).collect();
        assert_eq!(names, vec!["Alpha", "Bravo", "Charlie"]);
    }

    #[test]
    fn l0_search_normalizes_query_for_diacritic_match() {
        // "Slafkovsky" (no accent) should match "Slafkovský" (accented)
        // because name_normalized strips diacritics and the query is
        // normalize_name'd before contains().
        let repo = build_repo(&[(1, "Juraj Slafkovský", "MTL", 50.0)]);
        let views: Vec<_> = repo
            .skaters(
                icelines_core::model::Season(20242025),
                icelines_core::season_stats::SeasonType::Regular,
            )
            .collect();
        let results = search_results(&views, "slafkovsky");
        assert_eq!(results.len(), 1);
    }
}
