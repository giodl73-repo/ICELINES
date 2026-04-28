//! Stub screens for Tonight, Projections, Groups, Fetch.

use ratatui::{layout::Rect, text::Line, widgets::{Block, Borders, Paragraph}, Frame};

macro_rules! stub_screen {
    ($fn:ident, $title:literal, $body:expr) => {
        pub fn $fn(f: &mut Frame, area: Rect) {
            let block = Block::default().borders(Borders::ALL).title($title);
            let inner = block.inner(area);
            f.render_widget(block, area);
            f.render_widget(Paragraph::new($body as Vec<Line>), inner);
        }
    };
}

stub_screen!(render_tonight, " Tonight's Games ",
    vec![
        Line::from("  Today's NHL schedule"),
        Line::from(""),
        Line::from("  Run `icelines tonight` in terminal for live data."),
        Line::from("  Live schedule integration planned for Phase 4."),
    ]
);

stub_screen!(render_projections, " Rest-of-Season Projections ",
    vec![
        Line::from("  Pace · Regressed · Composite projections"),
        Line::from(""),
        Line::from("  Run `icelines project --team SEA` in terminal."),
        Line::from("  Interactive projection picker planned for Phase 4."),
    ]
);

stub_screen!(render_groups, " Player Groups ",
    vec![
        Line::from("  Manage watchlists and custom groups"),
        Line::from(""),
        Line::from("  Use `icelines group list` in terminal."),
        Line::from("  Interactive group browser planned for Phase 4."),
    ]
);

stub_screen!(render_fetch, " Fetch & Cache Status ",
    vec![
        Line::from("  Data pipeline status"),
        Line::from(""),
        Line::from("  Run `icelines snapshot list` to see cached snapshots."),
        Line::from("  Run `icelines fetch all` to refresh data."),
        Line::from("  Interactive fetch progress planned for Phase 4."),
    ]
);
