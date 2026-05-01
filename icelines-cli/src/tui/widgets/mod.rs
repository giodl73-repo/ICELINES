use ratatui::text::Line;

/// Render the help overlay content.
pub fn help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from("  IceLines TUI — Key Bindings"),
        Line::from("  ─────────────────────────────"),
        Line::from("  q / Ctrl+C   Quit"),
        Line::from("  ?            Help (this screen)"),
        Line::from("  /            Search (players or schedule team / matchup)"),
        Line::from("  Tab          Cycle main screens"),
        Line::from("  1–6          Jump directly to a tab"),
        Line::from("  ↑/↓  j/k     Move cursor"),
        Line::from("  ←/→          Sub-view  ·  on Schedule: previous / next week"),
        Line::from("  Enter        Select / drill down"),
        Line::from("  Esc          Go back / clear search"),
        Line::from("  r            Refresh / retry current view"),
        Line::from("  d            Toggle league depth chart  (s: scoring on Depth)"),
        Line::from("  t            On Schedule: jump to today's week"),
        Line::from("  y            Season picker (time-travel)"),
        Line::from("  F            Admin overlay"),
        Line::from(""),
        Line::from("  Tabs: League · Stats · Scores · Schedule · Groups · Playoffs"),
        Line::from("  Enter on team → Team lineup card"),
        Line::from("  Enter on player → Player profile"),
        Line::from("  /SEA → team filter   ·   /NYR WSH → matchup filter"),
    ]
}
