#![allow(dead_code)]
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use icelines_core::model::Player;
use icelines_core::cross_team::WebFitClass;

/// Render a single player cell (name + pace stats + fit label).
pub fn player_cell_text(p: &Player, fit: Option<WebFitClass>) -> Vec<Span<'static>> {
    let name = p.full_name.chars().take(20).collect::<String>();
    let (ppg, proj) = match p.pace_score {
        Some(s) => (format!("{:.2}", s.pace_82 / 82.0), format!("{:.0}", s.pace_82)),
        None    => ("—".to_owned(), "—".to_owned()),
    };

    let color = match fit {
        Some(WebFitClass::Elite)   => Color::Green,
        Some(WebFitClass::Solid)   => Color::Yellow,
        Some(WebFitClass::Buried)  => Color::Blue,
        Some(WebFitClass::Stretch) => Color::Red,
        None                       => Color::White,
    };

    vec![
        Span::styled(format!("{name:<20}"), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Span::raw(format!(" {ppg:>5} → {proj:>4}")),
    ]
}

/// Render the help overlay content.
pub fn help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from("  IceLines TUI — Key Bindings"),
        Line::from("  ─────────────────────────────"),
        Line::from("  q / Ctrl+C   Quit"),
        Line::from("  ?            Help (this screen)"),
        Line::from("  /            Search players"),
        Line::from("  Tab          Cycle main screens"),
        Line::from("  ↑/↓  j/k     Move cursor"),
        Line::from("  Enter        Select / drill down"),
        Line::from("  Esc          Go back"),
        Line::from("  r            Refresh current view"),
        Line::from(""),
        Line::from("  Screens: Home · Tonight · Projections · Groups · Fetch"),
        Line::from("  Enter on team → Team lineup card"),
        Line::from("  Enter on player → Player profile"),
    ]
}
