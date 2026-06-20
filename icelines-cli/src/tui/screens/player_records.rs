use crate::tui::app::App;
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{PlayerRecordsView, RecordsOpponentRow, ViewContext, ViewWindow};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerRecordsMetric {
    TeamsScoredAgainst,
    GoaliesScoredAgainst,
    FightOpponents,
}

impl PlayerRecordsMetric {
    const ALL: &'static [Self] = &[
        Self::TeamsScoredAgainst,
        Self::GoaliesScoredAgainst,
        Self::FightOpponents,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => "NHL teams scored against",
            Self::GoaliesScoredAgainst => "NHL goalies scored against",
            Self::FightOpponents => "Fight opponents",
        }
    }

    fn subject_label(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => "team",
            Self::GoaliesScoredAgainst => "goalie",
            Self::FightOpponents => "opponent",
        }
    }

    fn empty_hint(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => {
                "No local boxscore goal rows yet. Run `icelines fetch boxscore --date YYYY-MM-DD`."
            }
            Self::GoaliesScoredAgainst => {
                "No local play-by-play goalie rows yet. Run `icelines fetch play-by-play --date YYYY-MM-DD`."
            }
            Self::FightOpponents => {
                "No local play-by-play fighting rows yet. Run `icelines fetch play-by-play --date YYYY-MM-DD`."
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct PlayerRecordsScreenState {
    cache: Mutex<Option<PlayerRecordsCache>>,
}

#[derive(Debug, Clone)]
struct PlayerRecordsCache {
    key: PlayerRecordsCacheKey,
    sections: Vec<PlayerRecordSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayerRecordsCacheKey {
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
}

#[derive(Debug, Clone)]
struct PlayerRecordSection {
    metric: PlayerRecordsMetric,
    view: PlayerRecordsView,
}

impl PlayerRecordsScreenState {
    fn sections(
        &self,
        pid: PlayerId,
        player_name: &str,
        season: Season,
        season_type: SeasonType,
    ) -> anyhow::Result<Vec<PlayerRecordSection>> {
        let key = PlayerRecordsCacheKey {
            pid,
            season,
            season_type,
        };
        if let Ok(cache) = self.cache.lock() {
            if let Some(cached) = cache.as_ref().filter(|cached| cached.key == key) {
                return Ok(cached.sections.clone());
            }
        }

        let sections = build_sections(pid, player_name, season, season_type)?;
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(PlayerRecordsCache {
                key,
                sections: sections.clone(),
            });
        }
        Ok(sections)
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
                    .title(" Player Records ")
                    .borders(Borders::ALL),
            )
            .style(err),
            area,
        );
        return;
    };

    let sections = match app.player_records.sections(
        pid,
        &identity.full_name,
        app.active_season_typed,
        app.active_type,
    ) {
        Ok(sections) => sections,
        Err(e) => {
            f.render_widget(
                Paragraph::new(format!("Could not load local records data: {e}"))
                    .block(
                        Block::default()
                            .title(format!(" {} Records ", identity.full_name))
                            .borders(Borders::ALL),
                    )
                    .style(err),
                area,
            );
            return;
        }
    };

    let panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    let header = vec![
        Line::styled(format!(" {}", identity.full_name), hi),
        Line::from(format!(
            " Player records  -  Esc: player card  -  web: /records/player/{}?metric=...",
            pid.0
        )),
        Line::styled(
            " Totals count distinct opponents from local boxscore/play-by-play records.",
            dim,
        ),
    ];
    f.render_widget(
        Paragraph::new(header).block(Block::default().borders(Borders::ALL)),
        panels[0],
    );

    for (idx, section) in sections.iter().enumerate() {
        render_section(f, panels[idx + 1], section);
    }
}

fn build_sections(
    pid: PlayerId,
    player_name: &str,
    season: Season,
    season_type: SeasonType,
) -> anyhow::Result<Vec<PlayerRecordSection>> {
    let context = ViewContext::new(ViewWindow::new(season, season_type));
    let boxscore_goals =
        icelines_fetch::records_provider::load_goal_record_inputs_from_default_store()?;
    let play_by_play_goals =
        icelines_fetch::records_provider::load_play_by_play_goal_record_inputs_from_default_store(
        )?;
    let fights = icelines_fetch::records_provider::load_fight_record_inputs_from_default_store()?;

    Ok(PlayerRecordsMetric::ALL
        .iter()
        .copied()
        .map(|metric| {
            let view = match metric {
                PlayerRecordsMetric::TeamsScoredAgainst => PlayerRecordsView::teams_scored_against(
                    context.clone(),
                    pid.0,
                    player_name,
                    &boxscore_goals,
                ),
                PlayerRecordsMetric::GoaliesScoredAgainst => {
                    PlayerRecordsView::goalies_scored_against(
                        context.clone(),
                        pid.0,
                        player_name,
                        &play_by_play_goals,
                    )
                }
                PlayerRecordsMetric::FightOpponents => {
                    PlayerRecordsView::fight_opponents(context.clone(), pid.0, player_name, &fights)
                }
            };
            PlayerRecordSection { metric, view }
        })
        .collect())
}

fn render_section(f: &mut Frame, area: Rect, section: &PlayerRecordSection) {
    let title = format!(
        " {}: {} total ",
        section.metric.title(),
        section.view.rows.len()
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if section.view.rows.is_empty() {
        f.render_widget(
            Paragraph::new(section.metric.empty_hint()).style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let max_rows = inner.height.saturating_sub(1) as usize;
    let max_count = section
        .view
        .rows
        .iter()
        .map(|row| row.count)
        .max()
        .unwrap_or(0);
    let mut items = Vec::with_capacity(max_rows.saturating_add(1));
    items.push(ListItem::new(Line::styled(
        format!(
            " {:<24} {:>5} {:<4} {:<10}  {:<10}",
            section.metric.subject_label(),
            "count",
            "bar",
            "first",
            "last"
        ),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )));
    for row in section.view.rows.iter().take(max_rows) {
        items.push(ListItem::new(row_line(row, max_count)));
    }
    if section.view.rows.len() > max_rows {
        items.push(ListItem::new(Line::styled(
            format!(" ... {} more", section.view.rows.len() - max_rows),
            Style::default().fg(Color::DarkGray),
        )));
    }

    f.render_widget(List::new(items), inner);
}

fn row_line(row: &RecordsOpponentRow, max_count: u32) -> Line<'static> {
    let first = row.first_date.as_deref().unwrap_or("-");
    let last = row.last_date.as_deref().unwrap_or("-");
    Line::from(format!(
        " {:<24} {:>5} {:<4} {:<10}  {:<10}",
        truncate(&row.label, 24),
        row.count,
        count_bar(row.count, max_count),
        first,
        last
    ))
}

fn count_bar(count: u32, max_count: u32) -> String {
    if count == 0 || max_count == 0 {
        return String::new();
    }
    let bars = ((usize::try_from(count).unwrap_or(usize::MAX) * 4)
        .div_ceil(usize::try_from(max_count).unwrap_or(usize::MAX)))
    .max(1);
    "#".repeat(bars)
}

fn truncate(s: &str, width: usize) -> String {
    let count = s.chars().count();
    if count <= width {
        return s.to_string();
    }
    let keep = width.saturating_sub(1);
    let mut out = s.chars().take(keep).collect::<String>();
    out.push('.');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_records_metrics_cover_expected_sections() {
        let titles = PlayerRecordsMetric::ALL
            .iter()
            .map(|metric| metric.title())
            .collect::<Vec<_>>();
        assert_eq!(
            titles,
            vec![
                "NHL teams scored against",
                "NHL goalies scored against",
                "Fight opponents"
            ]
        );
    }

    #[test]
    fn truncate_preserves_short_labels_and_marks_long_labels() {
        assert_eq!(truncate("EDM", 5), "EDM");
        assert_eq!(truncate("Very Long Goalie Name", 10), "Very Long.");
    }

    #[test]
    fn l0_player_records_count_bar_scales_and_skips_zero_counts() {
        assert_eq!(count_bar(0, 4), "");
        assert_eq!(count_bar(1, 4), "#");
        assert_eq!(count_bar(2, 4), "##");
        assert_eq!(count_bar(4, 4), "####");
        assert_eq!(count_bar(4, 0), "");
    }
}
