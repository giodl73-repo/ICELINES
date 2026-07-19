use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

/// 32 NHL teams in default ranking order (updated by icelines build).
pub const RANKED_TEAMS: &[&str] = &[
    "COL", "TBL", "VGK", "DAL", "EDM", "PIT", "MTL", "MIN", "OTT", "FLA", "SJS", "BUF", "ANA",
    "CAR", "UTA", "BOS", "WSH", "DET", "CBJ", "TOR", "NYI", "NYR", "PHI", "NJD", "STL", "LAK",
    "SEA", "NSH", "CHI", "WPG", "CGY", "VAN",
];

const TEAM_NAMES: &[(&str, &str)] = &[
    ("COL", "Colorado Avalanche"),
    ("TBL", "Tampa Bay Lightning"),
    ("VGK", "Vegas Golden Knights"),
    ("DAL", "Dallas Stars"),
    ("EDM", "Edmonton Oilers"),
    ("PIT", "Pittsburgh Penguins"),
    ("MTL", "Montréal Canadiens"),
    ("MIN", "Minnesota Wild"),
    ("OTT", "Ottawa Senators"),
    ("FLA", "Florida Panthers"),
    ("SJS", "San Jose Sharks"),
    ("BUF", "Buffalo Sabres"),
    ("ANA", "Anaheim Ducks"),
    ("CAR", "Carolina Hurricanes"),
    ("UTA", "Utah Hockey Club"),
    ("BOS", "Boston Bruins"),
    ("WSH", "Washington Capitals"),
    ("DET", "Detroit Red Wings"),
    ("CBJ", "Columbus Blue Jackets"),
    ("TOR", "Toronto Maple Leafs"),
    ("NYI", "New York Islanders"),
    ("NYR", "New York Rangers"),
    ("PHI", "Philadelphia Flyers"),
    ("NJD", "New Jersey Devils"),
    ("STL", "St. Louis Blues"),
    ("LAK", "Los Angeles Kings"),
    ("SEA", "Seattle Kraken"),
    ("NSH", "Nashville Predators"),
    ("CHI", "Chicago Blackhawks"),
    ("WPG", "Winnipeg Jets"),
    ("CGY", "Calgary Flames"),
    ("VAN", "Vancouver Canucks"),
];

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_col(f, app, chunks[0], 0, 16, 1);
    render_col(f, app, chunks[1], 16, 32, 17);
}

fn render_col(f: &mut Frame, app: &App, area: Rect, from: usize, to: usize, rank_start: usize) {
    let items: Vec<ListItem> = RANKED_TEAMS[from..to.min(RANKED_TEAMS.len())]
        .iter()
        .enumerate()
        .map(|(i, abbrev)| {
            let rank = rank_start + i;
            let name = TEAM_NAMES
                .iter()
                .find(|(a, _)| a == abbrev)
                .map(|(_, n)| *n)
                .unwrap_or(abbrev);
            let rank_color = if rank <= 5 {
                Color::Green
            } else if rank <= 10 {
                Color::Cyan
            } else if rank >= 28 {
                Color::Red
            } else {
                Color::White
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("#{rank:<3}"), Style::default().fg(rank_color)),
                Span::styled(
                    format!(" {abbrev:<5}"),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" {name}")),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if app.selected >= from && app.selected < to {
        state.select(Some(app.selected - from));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Center Ice — League Tracker "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn l0_ranked_teams_has_thirty_two_unique_entries() {
        assert_eq!(
            RANKED_TEAMS.len(),
            32,
            "RANKED_TEAMS must list every NHL franchise"
        );
        let set: HashSet<&str> = RANKED_TEAMS.iter().copied().collect();
        assert_eq!(set.len(), 32, "RANKED_TEAMS must contain unique abbrevs");
    }

    #[test]
    fn l0_ranked_teams_match_canonical_nhl_abbrevs() {
        // The Home → Team flow uses RANKED_TEAMS abbrevs to filter
        // app.players by `p.team.as_str() == abbrev`. If the abbrev
        // is wrong, the team page renders empty (regression: TB/SJ
        // instead of TBL/SJS).
        let canonical: HashSet<&str> = icelines_fetch::ALL_NHL_TEAMS.iter().copied().collect();
        for &t in RANKED_TEAMS {
            assert!(
                canonical.contains(t),
                "RANKED_TEAMS entry '{t}' is not a valid NHL API abbrev — \
                 use the canonical teams::ALL_NHL_TEAMS form (e.g. TBL not TB)",
            );
        }
    }

    #[test]
    fn l0_team_names_keys_match_ranked_teams_set() {
        let ranked: HashSet<&str> = RANKED_TEAMS.iter().copied().collect();
        let names: HashSet<&str> = TEAM_NAMES.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            ranked, names,
            "TEAM_NAMES must cover exactly the same abbrevs as RANKED_TEAMS",
        );
    }

    #[test]
    fn l1_every_ranked_team_has_players_in_bundled_bios() {
        // The strongest catch: every Home entry must produce ≥1 player
        // in the current-season bundled bios. A regression that spelled
        // 'TB' instead of 'TBL' would surface here as a clear failure
        // pointing at the offending abbrev.
        let bios = icelines_fetch::get_bundled_bios(icelines_core::CURRENT_SEASON_STR)
            .expect("25-26 bios must be bundled");
        let teams_in_bios: HashSet<String> = bios
            .iter()
            .filter_map(|b| b.current_team_abbrev.clone())
            .collect();

        for &t in RANKED_TEAMS {
            assert!(
                teams_in_bios.contains(t),
                "RANKED_TEAMS entry '{t}' has zero players in bundled bios — \
                 Home → Team would render empty for this team",
            );
        }
    }
}
