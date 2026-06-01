use crate::tui::app::App;
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{PlayerStreaksView, ViewContext, ViewWindow};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct PlayerStreaksScreenState {
    cache: Mutex<Option<PlayerStreaksCache>>,
}

#[derive(Debug, Clone)]
struct PlayerStreaksCache {
    key: PlayerStreaksCacheKey,
    view: PlayerStreaksView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayerStreaksCacheKey {
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
}

impl PlayerStreaksScreenState {
    fn view(
        &self,
        pid: PlayerId,
        player_name: &str,
        season: Season,
        season_type: SeasonType,
    ) -> anyhow::Result<PlayerStreaksView> {
        let key = PlayerStreaksCacheKey {
            pid,
            season,
            season_type,
        };
        if let Ok(cache) = self.cache.lock() {
            if let Some(cached) = cache.as_ref().filter(|cached| cached.key == key) {
                return Ok(cached.view.clone());
            }
        }

        let cfg = crate::config::Config::load()?;
        let data_root = cfg
            .cache_dir
            .parent()
            .unwrap_or(&cfg.cache_dir)
            .join("data");
        let store = icelines_fetch::datastore::DataStore::open(&data_root)?;
        let lines = icelines_fetch::streaks_provider::load_player_game_lines(&store, pid.0);
        let (shot_lines, play_by_play_source_loaded) =
            icelines_fetch::streaks_provider::load_player_shot_lines(&store, pid.0);
        let player_name = lines
            .first()
            .map(|line| line.player_name.clone())
            .or_else(|| shot_lines.first().map(|line| line.player_name.clone()))
            .unwrap_or_else(|| player_name.to_string());
        let context = ViewContext::new(ViewWindow::new(season, season_type));
        let view = PlayerStreaksView::from_game_and_shot_lines(
            context,
            pid.0,
            player_name,
            &lines,
            &shot_lines,
            play_by_play_source_loaded,
        );

        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(PlayerStreaksCache {
                key,
                view: view.clone(),
            });
        }
        Ok(view)
    }
}

pub fn render_by_id(f: &mut Frame, app: &App, area: Rect, pid: PlayerId) {
    let dim = Style::default().fg(Color::DarkGray);
    let hi = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let err = Style::default().fg(Color::Red);

    let Some(identity) = app.repo.identity(pid) else {
        f.render_widget(
            Paragraph::new(format!(
                "No player with NHL id {} in the active repository. Press Esc to go back.",
                pid.0
            ))
            .block(
                Block::default()
                    .title(" Player Streaks ")
                    .borders(Borders::ALL),
            )
            .style(err),
            area,
        );
        return;
    };

    let view = match app.player_streaks.view(
        pid,
        &identity.full_name,
        app.active_season_typed,
        app.active_type,
    ) {
        Ok(view) => view,
        Err(e) => {
            f.render_widget(
                Paragraph::new(format!("Could not load local streak inputs: {e}"))
                    .block(
                        Block::default()
                            .title(format!(" {} Streaks ", identity.full_name))
                            .borders(Borders::ALL),
                    )
                    .style(err),
                area,
            );
            return;
        }
    };

    let mut items = vec![
        ListItem::new(Line::styled(format!(" {}", view.player_name), hi)),
        ListItem::new(Line::from(format!(
            " Player streaks  -  {} cached game lines  -  Esc: player card",
            view.games_loaded
        ))),
        ListItem::new(Line::styled(
            " Source: cached boxscore + play-by-play rows; no streaks are inferred from season totals.",
            dim,
        )),
        ListItem::new(Line::from("")),
    ];

    if view.games_loaded == 0 {
        items.push(ListItem::new(Line::styled(
            " No cached boxscore/play-by-play game lines yet. Run `icelines fetch boxscore --date YYYY-MM-DD` and `icelines fetch play-by-play --date YYYY-MM-DD`.",
            dim,
        )));
    } else {
        items.push(ListItem::new(Line::styled(
            " metric        current   status     longest   longest window",
            dim,
        )));
        for row in &view.rows {
            items.push(ListItem::new(Line::from(format!(
                " {:<12} {:>7}   {:<8} {:>9}   {}",
                row.metric,
                row.current,
                row.current_status,
                row.longest,
                date_window(
                    row.longest_start_date.as_deref(),
                    row.longest_end_date.as_deref()
                )
            ))));
        }
    }

    f.render_widget(
        List::new(items).block(
            Block::default()
                .title(" Player Streaks ")
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn date_window(start: Option<&str>, end: Option<&str>) -> String {
    match (start, end) {
        (Some(start), Some(end)) if start == end => start.to_string(),
        (Some(start), Some(end)) => format!("{start} to {end}"),
        _ => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_streaks_screen_formats_date_window() {
        assert_eq!(
            date_window(Some("2025-10-01"), Some("2025-10-02")),
            "2025-10-01 to 2025-10-02"
        );
        assert_eq!(
            date_window(Some("2025-10-01"), Some("2025-10-01")),
            "2025-10-01"
        );
        assert_eq!(date_window(None, None), "-");
    }
}
