use crate::tui::app::App;
use icelines_core::identity::PlayerId;
use icelines_core::{PlayerAwardRow, PlayerAwardsView};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render_by_id(f: &mut Frame, app: &App, area: Rect, pid: PlayerId) {
    let hi = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let err = Style::default().fg(Color::Red);

    let Some(identity) = app.repo.identity(pid) else {
        f.render_widget(
            Paragraph::new(format!(
                "No player with NHL id {} in the active repository. Press Esc to go back.",
                pid.0
            ))
            .block(
                Block::default()
                    .title(" Trophy Case ")
                    .borders(Borders::ALL),
            )
            .style(err),
            area,
        );
        return;
    };

    let store = icelines_fetch::career_landing::load_local_awards_store();
    let Some(view) = store.get(pid.0) else {
        let lines = vec![
            Line::styled(format!(" {}", identity.full_name), hi),
            Line::from(" Awards / Trophy Case"),
            Line::styled(
                format!(
                    " No local awards cache yet. Run: icelines awards \"{}\"",
                    identity.full_name
                ),
                dim,
            ),
            Line::styled(
                format!(
                    " Web: /player/{}/awards  |  API: /api/v1/player/{}/awards",
                    pid.0, pid.0
                ),
                dim,
            ),
        ];
        f.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .title(" Trophy Case ")
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    };

    render_view(f, area, view);
}

fn render_view(f: &mut Frame, area: Rect, view: &PlayerAwardsView) {
    let hi = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let mut items = vec![
        ListItem::new(Line::styled(format!(" {}", view.player_name), hi)),
        ListItem::new(Line::from(format!(
            " Trophy Case  -  {} trophies, {} trophy seasons",
            view.trophy_count(),
            view.season_count()
        ))),
        ListItem::new(Line::styled(
            " Source: NHL landing awards[]  -  Esc: player card",
            dim,
        )),
        ListItem::new(Line::from("")),
    ];

    if view.awards.is_empty() {
        items.push(ListItem::new(Line::styled(
            " No NHL awards found for this player.",
            dim,
        )));
    } else {
        for award in &view.awards {
            push_award(&mut items, award);
        }
    }

    f.render_widget(
        List::new(items).block(
            Block::default()
                .title(" Trophy Case ")
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn push_award(items: &mut Vec<ListItem<'static>>, award: &PlayerAwardRow) {
    items.push(ListItem::new(Line::styled(
        format!(" {}", award.trophy),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    if award.seasons.is_empty() {
        items.push(ListItem::new(Line::styled(
            "   no season rows",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for season in &award.seasons {
            items.push(ListItem::new(Line::from(format!(
                "   {} {}  GP {}  {}-{}-{}",
                season.season.0,
                game_type_label(season.game_type_id),
                opt_u32(season.games_played),
                opt_u32(season.goals),
                opt_u32(season.assists),
                opt_u32(season.points)
            ))));
        }
    }
    items.push(ListItem::new(Line::from("")));
}

fn opt_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn game_type_label(game_type_id: u8) -> &'static str {
    match game_type_id {
        2 => "regular",
        3 => "playoffs",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_awards_screen_labels_game_types() {
        assert_eq!(game_type_label(2), "regular");
        assert_eq!(game_type_label(3), "playoffs");
        assert_eq!(game_type_label(7), "other");
    }
}
