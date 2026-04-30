//! Goalies tab — league-wide goalie leaderboard. Phase G.3.
//!
//! Sort cycle (`s` key):  SV% ↓ → GAA ↑ → Wins ↓ → GP ↓ → Saves ↓ → SO ↓
//! Min-GP cycle (`m` key): 5 → 15 → 25 → 40 → 5
//! `Enter` opens a per-goalie detail card.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use icelines_core::model::Goalie;

/// Sort selectors. App stores the index; we map index → comparator here
/// so the cycle order is centralised.
pub const SORTS: &[GoalieSort] = &[
    GoalieSort::SvPctDesc,
    GoalieSort::GaaAsc,
    GoalieSort::WinsDesc,
    GoalieSort::GpDesc,
    GoalieSort::SavesDesc,
    GoalieSort::ShutoutsDesc,
];

#[derive(Clone, Copy)]
pub enum GoalieSort {
    SvPctDesc,
    GaaAsc,
    WinsDesc,
    GpDesc,
    SavesDesc,
    ShutoutsDesc,
}

impl GoalieSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::SvPctDesc    => "SV%",
            Self::GaaAsc       => "GAA",
            Self::WinsDesc     => "Wins",
            Self::GpDesc       => "GP",
            Self::SavesDesc    => "Saves",
            Self::ShutoutsDesc => "SO",
        }
    }
}

/// The min-GP cycle values exposed under the `m` key. Stops at sensible
/// NHL leaderboard thresholds rather than allowing arbitrary input.
pub const MIN_GP_CYCLE: &[u32] = &[5, 15, 25, 40];

/// Apply the App's sort selection to a vec of goalie references.
pub fn sort_goalies<'a>(
    goalies: &'a [Goalie],
    sort: GoalieSort,
    min_gp: u32,
) -> Vec<&'a Goalie> {
    let mut out: Vec<&Goalie> = goalies.iter()
        .filter(|g| g.qualified(min_gp))
        .collect();
    use std::cmp::Ordering;
    out.sort_by(|a, b| {
        let sa = a.stats.as_ref();
        let sb = b.stats.as_ref();
        let ord = match sort {
            GoalieSort::SvPctDesc => {
                let av = sa.and_then(|s| s.save_pct).unwrap_or(0.0);
                let bv = sb.and_then(|s| s.save_pct).unwrap_or(0.0);
                bv.partial_cmp(&av).unwrap_or(Ordering::Equal)
            }
            GoalieSort::GaaAsc => {
                let av = sa.and_then(|s| s.goals_against_average).unwrap_or(f32::INFINITY);
                let bv = sb.and_then(|s| s.goals_against_average).unwrap_or(f32::INFINITY);
                av.partial_cmp(&bv).unwrap_or(Ordering::Equal)
            }
            GoalieSort::WinsDesc =>
                sb.map(|s| s.wins).unwrap_or(0).cmp(&sa.map(|s| s.wins).unwrap_or(0)),
            GoalieSort::GpDesc =>
                sb.map(|s| s.games_played).unwrap_or(0).cmp(&sa.map(|s| s.games_played).unwrap_or(0)),
            GoalieSort::SavesDesc =>
                sb.map(|s| s.saves).unwrap_or(0).cmp(&sa.map(|s| s.saves).unwrap_or(0)),
            GoalieSort::ShutoutsDesc =>
                sb.map(|s| s.shutouts).unwrap_or(0).cmp(&sa.map(|s| s.shutouts).unwrap_or(0)),
        };
        // Tiebreaker: SV% desc to keep the leaderboard stable under ties.
        ord.then_with(|| {
            let av = sa.and_then(|s| s.save_pct).unwrap_or(0.0);
            let bv = sb.and_then(|s| s.save_pct).unwrap_or(0.0);
            bv.partial_cmp(&av).unwrap_or(Ordering::Equal)
        })
    });
    out
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let sort = SORTS.get(app.goalie_sort as usize).copied().unwrap_or(GoalieSort::SvPctDesc);
    let title = format!(
        " Goalies · sort: {} · min GP: {} · s:sort  m:min-gp  Enter:detail  Esc:back ",
        sort.label(), app.goalie_min_gp,
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.goalies.is_empty() {
        let dim = Style::default().fg(Color::DarkGray);
        f.render_widget(Paragraph::new(vec![
            Line::from(""),
            Line::styled("  No goalie data loaded yet.", dim),
            Line::from(""),
            Line::styled(
                "  Run `icelines fetch goalies` to populate, or wait for the loader.",
                dim,
            ),
        ]), inner);
        return;
    }

    let qualified = sort_goalies(&app.goalies, sort, app.goalie_min_gp);
    if qualified.is_empty() {
        let dim = Style::default().fg(Color::DarkGray);
        f.render_widget(Paragraph::new(vec![
            Line::from(""),
            Line::styled(
                format!("  No goalies have played at least {} games this season.", app.goalie_min_gp),
                dim,
            ),
            Line::from(""),
            Line::styled("  Press m to lower the minimum (5/15/25/40).", dim),
        ]), inner);
        return;
    }

    let dim   = Style::default().fg(Color::DarkGray);
    let gold  = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let cyan  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);

    // Header row + horizontal rule.
    let header_line = format!(
        "  {:<3}  {:<22} {:<5} {:<4}  {:<10}  {:<6}  {:<6}  {:<3}  {:<6}",
        "#", "Goalie", "Team", "GP", "W-L-OT", "SV%", "GAA", "SO", "Saves",
    );
    let mut items: Vec<ListItem> = Vec::new();
    items.push(ListItem::new(Line::styled(header_line, gold)));
    items.push(ListItem::new(Line::styled(format!("  {}", "─".repeat(80)), dim)));

    let selected_idx = app.goalie_selected.min(qualified.len().saturating_sub(1));
    for (rank, g) in qualified.iter().enumerate() {
        let stats = match g.stats.as_ref() {
            Some(s) => s,
            None    => continue,
        };
        let record = match stats.ot_losses {
            Some(otl) => format!("{}-{}-{}", stats.wins, stats.losses, otl),
            None      => format!("{}-{}",    stats.wins, stats.losses),
        };
        let sv_pct  = stats.save_pct.map(|v| format!("{:.3}", v))
            .unwrap_or_else(|| "—".to_owned());
        let gaa     = stats.goals_against_average.map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "—".to_owned());
        let row = format!(
            "  {:<3}  {:<22} {:<5} {:<4}  {:<10}  {:<6}  {:<6}  {:<3}  {:<6}",
            rank + 1,
            short_name(&g.full_name),
            g.team.as_str(),
            stats.games_played,
            record,
            sv_pct,
            gaa,
            stats.shutouts,
            stats.saves,
        );
        let style = if rank == selected_idx {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if rank < 3 {
            cyan       // top-3 highlighted
        } else {
            Style::default()
        };
        items.push(ListItem::new(Line::styled(row, style)));
    }

    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(format!("  {} qualified · ", qualified.len()), dim),
        Span::styled("s", cyan), Span::styled(":sort  ", dim),
        Span::styled("m", cyan), Span::styled(":min-gp  ", dim),
        Span::styled("Enter", cyan), Span::styled(":detail", dim),
    ])));
    f.render_widget(List::new(items), area);
}

/// Trim "Connor Hellebuyck" → "C. Hellebuyck" so the leaderboard fits in
/// the 22-col column width without wrapping.
fn short_name(full: &str) -> String {
    let mut parts = full.split_whitespace();
    match (parts.next(), parts.next_back()) {
        (Some(first), Some(last)) if first != last => {
            let initial = first.chars().next().unwrap_or('?');
            format!("{initial}. {last}")
        }
        (Some(only), _) => only.to_owned(),
        _               => full.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::model::{Goalie, GoalieBio, GoalieSeasonStats, TeamAbbr};

    fn fixture_goalie(id: u32, name: &str, team: &str, gp: u32, wins: u32,
                      sv_pct: f32, gaa: f32, so: u32) -> Goalie {
        Goalie {
            nhl_id:          id,
            full_name:       name.to_owned(),
            name_normalized: name.to_lowercase().replace(' ', "_"),
            team:            TeamAbbr(team.to_owned()),
            stats: Some(GoalieSeasonStats {
                games_played: gp, games_started: gp,
                wins, losses: gp.saturating_sub(wins).saturating_sub(2),
                ot_losses: Some(2), ties: None,
                shots_against: 30 * gp, goals_against: ((gaa as u32) * gp).max(0),
                saves: 28 * gp,
                save_pct: Some(sv_pct), goals_against_average: Some(gaa),
                shutouts: so, time_on_ice: gp * 3600,
            }),
            bio: GoalieBio {
                birth_date: None, birth_country: None, nationality_code: None,
                catches: Some("L".to_owned()),
                height_in_inches: None, weight_lbs: None,
                draft_year: None, draft_round: None, draft_overall: None,
                rookie_season: None,
            },
            headshot_url:   None,
            sweater_number: None,
        }
    }

    #[test]
    fn l0_sort_goalies_sv_pct_desc_default() {
        let pool = vec![
            fixture_goalie(1, "Backup", "WPG", 20, 8,  0.890, 3.20, 0),
            fixture_goalie(2, "Connor Hellebuyck", "WPG", 63, 47, 0.925, 2.00, 8),
            fixture_goalie(3, "Mid Tier", "BOS", 35, 18, 0.910, 2.50, 2),
        ];
        let sorted = sort_goalies(&pool, GoalieSort::SvPctDesc, 15);
        assert_eq!(sorted[0].full_name, "Connor Hellebuyck",
            "highest SV% should sort first");
        assert_eq!(sorted[1].full_name, "Mid Tier");
        assert_eq!(sorted[2].full_name, "Backup");
    }

    #[test]
    fn l0_sort_goalies_gaa_asc_low_is_best() {
        let pool = vec![
            fixture_goalie(1, "High GAA", "OTT", 30, 12, 0.900, 3.20, 1),
            fixture_goalie(2, "Low GAA",  "WPG", 30, 18, 0.920, 2.00, 5),
        ];
        let sorted = sort_goalies(&pool, GoalieSort::GaaAsc, 15);
        assert_eq!(sorted[0].full_name, "Low GAA",
            "GAA sort: smaller is better — low GAA first");
    }

    #[test]
    fn l0_sort_goalies_filters_by_min_gp() {
        // Backup with 5 GP should NOT appear when min_gp = 15.
        let pool = vec![
            fixture_goalie(1, "Backup",  "WPG", 5,  2,  0.999, 1.00, 0),
            fixture_goalie(2, "Starter", "WPG", 50, 28, 0.910, 2.50, 5),
        ];
        let sorted = sort_goalies(&pool, GoalieSort::SvPctDesc, 15);
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].full_name, "Starter");
    }

    #[test]
    fn l0_sort_goalies_lower_min_gp_includes_more() {
        let pool = vec![
            fixture_goalie(1, "Backup",  "WPG", 7,  3, 0.940, 2.00, 1),
            fixture_goalie(2, "Starter", "WPG", 50, 28, 0.910, 2.50, 5),
        ];
        let lo = sort_goalies(&pool, GoalieSort::SvPctDesc, 5);
        assert_eq!(lo.len(), 2, "min_gp=5 includes both");
        let hi = sort_goalies(&pool, GoalieSort::SvPctDesc, 15);
        assert_eq!(hi.len(), 1, "min_gp=15 excludes the backup");
    }

    #[test]
    fn l0_short_name_uses_initial() {
        assert_eq!(short_name("Connor Hellebuyck"), "C. Hellebuyck");
        assert_eq!(short_name("Igor"),              "Igor");
    }
}
