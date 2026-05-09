use crate::tui::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// ── Phase Adams.10 — Team screen state ───────────────────────────────────────

/// Sort key for the Team screen roster table. Cycled with `s`.
/// Age sort intentionally omitted in Adams.10 — the bio's
/// birth_date isn't on the canonical PlayerIdentity (lives on
/// the snapshot path); deferred to Adams.10b.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TeamSort {
    /// Default: Pts/82 (pace), descending. Mirrors the legacy
    /// behavior pre-Adams.10.
    #[default]
    Pace,
    Name,
    Position,
    Goals,
    Hits,
}

impl TeamSort {
    pub const ALL: &'static [TeamSort] = &[
        TeamSort::Pace,
        TeamSort::Name,
        TeamSort::Position,
        TeamSort::Goals,
        TeamSort::Hits,
    ];
    pub fn label(self) -> &'static str {
        match self {
            TeamSort::Pace => "Pts/82",
            TeamSort::Name => "Name",
            TeamSort::Position => "Pos",
            TeamSort::Goals => "G",
            TeamSort::Hits => "Hits",
        }
    }
    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

/// Position-class filter for the Team screen. Cycled with `p`.
/// `All` shows everyone (default). `F` = forwards (C/LW/RW/F).
/// `D` = defensemen (LD/RD/D). `C/LW/RW/LD/RD` = single position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TeamPosFilter {
    #[default]
    All,
    Forwards,
    Defense,
    C,
    LW,
    RW,
    LD,
    RD,
}

impl TeamPosFilter {
    pub const ALL: &'static [TeamPosFilter] = &[
        TeamPosFilter::All,
        TeamPosFilter::Forwards,
        TeamPosFilter::Defense,
        TeamPosFilter::C,
        TeamPosFilter::LW,
        TeamPosFilter::RW,
        TeamPosFilter::LD,
        TeamPosFilter::RD,
    ];
    pub fn label(self) -> &'static str {
        match self {
            TeamPosFilter::All => "All",
            TeamPosFilter::Forwards => "F",
            TeamPosFilter::Defense => "D",
            TeamPosFilter::C => "C",
            TeamPosFilter::LW => "LW",
            TeamPosFilter::RW => "RW",
            TeamPosFilter::LD => "LD",
            TeamPosFilter::RD => "RD",
        }
    }
    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
    /// Does `pos_abbrev` (e.g. "C", "LW", "LD", "G") satisfy this filter?
    pub fn matches(self, pos_abbrev: &str) -> bool {
        match self {
            TeamPosFilter::All => true,
            TeamPosFilter::Forwards => matches!(pos_abbrev, "C" | "LW" | "RW" | "F"),
            TeamPosFilter::Defense => matches!(pos_abbrev, "LD" | "RD" | "D"),
            TeamPosFilter::C => pos_abbrev == "C",
            TeamPosFilter::LW => pos_abbrev == "LW",
            TeamPosFilter::RW => pos_abbrev == "RW",
            TeamPosFilter::LD => pos_abbrev == "LD",
            TeamPosFilter::RD => pos_abbrev == "RD",
        }
    }
}

/// Country-code filter — deferred. Adams.10 ships sort +
/// position filter; country (e.g. `country=CAN`) is Adams.10b.
/// Cursor lives on `App::selected` like the legacy team flow.
#[derive(Debug, Clone, Default)]
pub struct TeamScreenState {
    pub sort: TeamSort,
    pub pos_filter: TeamPosFilter,
}

// ── Phase Masterton.1 / Adams.10 — chrome accessor ───────────────────────────

pub fn chrome(state: &TeamScreenState) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    let title = format!(
        "Team · sort={} · pos={}",
        state.sort.label(),
        state.pos_filter.label()
    );
    let keybinds = vec![
        KeyHint::new("s", "cycle sort"),
        KeyHint::new("p", "cycle pos"),
        KeyHint::new("↑↓", "select"),
        KeyHint::new("Enter", "open card"),
        KeyHint::new("g", "add to group"),
    ];
    ScreenChrome { title, keybinds }
}

// ── Render ───────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App, area: Rect, abbrev: &str) {
    let block = Block::default().borders(Borders::ALL).title(format!(
        " {} — Roster  ·  s: sort ({})  ·  p: pos ({})  ·  Enter: open  ·  g: group ",
        abbrev,
        app.team.sort.label(),
        app.team.pos_filter.label(),
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let team_abbr = icelines_core::model::TeamAbbr(abbrev.to_owned());
    let team_views = app.team_views(&team_abbr);

    if team_views.is_empty() {
        let msg = vec![
            Line::from(format!("  {} — Lineup Card", abbrev)),
            Line::from(""),
            Line::from("  Run `icelines fetch all` to load roster data."),
        ];
        f.render_widget(Paragraph::new(msg), inner);
        return;
    }

    // Apply filter then sort. Filter first so the sort doesn't
    // waste cycles on dropped rows.
    let mut filtered: Vec<&icelines_core::stats_repository::PlayerView<'_>> = team_views
        .iter()
        .filter(|v| {
            app.team
                .pos_filter
                .matches(v.position().abbreviation())
        })
        .collect();
    sort_team_views(&mut filtered, app.team.sort);

    let header_extra = match app.team.sort {
        TeamSort::Hits => "  Hits",
        TeamSort::Goals => "    G",
        _ => "",
    };
    let mut lines: Vec<Line> = vec![
        Line::from(format!(
            "  {} of {} players  ·  sort: {}  ·  pos: {}  ·  s/p cycle  ·  ↑↓ select  ·  Enter open",
            filtered.len(),
            team_views.len(),
            app.team.sort.label(),
            app.team.pos_filter.label(),
        )),
        Line::from(""),
        Line::from(format!(
            "  {:<22} {:<4}  {:>6}  {:>7}{}",
            "Player", "Pos", "PPG", "Pts/82", header_extra
        )),
        Line::from(format!("  {}", "─".repeat(46 + header_extra.chars().count()))),
    ];

    for (i, v) in filtered.iter().enumerate() {
        let p82 = v.pace_82();
        let ppg = p82
            .map(|p| format!("{:.3}", p / 82.0))
            .unwrap_or_else(|| "—".to_owned());
        let proj = p82
            .map(|p| format!("{:.1}", p))
            .unwrap_or_else(|| "—".to_owned());
        let name = v.full_name().chars().take(22).collect::<String>();
        let extra = match app.team.sort {
            TeamSort::Hits => format!(
                "  {:>4}",
                v.hits()
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| "—".to_owned())
            ),
            TeamSort::Goals => format!("  {:>4}", v.goals()),
            _ => String::new(),
        };

        let text = format!(
            "  {:<22} {:<4}  {:>6}  {:>7}{}",
            name,
            v.position().abbreviation(),
            ppg,
            proj,
            extra,
        );

        let style = if i == app.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::styled(text, style));
    }

    // Goalie strip (unchanged from pre-Adams.10).
    let goalie_views = app.goalie_views();
    let team_goalies = collect_team_goalie_views(&goalie_views, abbrev);
    if !team_goalies.is_empty() {
        let dim = Style::default().fg(Color::DarkGray);
        let gold = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        lines.push(Line::from(""));
        lines.push(Line::styled("  GOALTENDING", gold));
        lines.push(Line::styled(
            format!(
                "  {:<22} {:<4}  {:>6}  {:>7}",
                "Goalie", "GP", "SV%", "Record"
            ),
            dim,
        ));
        for v in &team_goalies {
            let stats = match v.stats.goalie.as_ref() {
                Some(s) => s,
                None => {
                    lines.push(Line::from(format!(
                        "  {:<22} {:<4}  {:>6}  {:>7}",
                        v.full_name().chars().take(22).collect::<String>(),
                        "—",
                        "—",
                        "—",
                    )));
                    continue;
                }
            };
            let sv_pct = stats
                .save_pct
                .map(|x| format!("{:.3}", x))
                .unwrap_or_else(|| "—".to_owned());
            let record = match stats.ot_losses {
                Some(otl) => format!("{}-{}-{}", stats.wins, stats.losses, otl),
                None => format!("{}-{}", stats.wins, stats.losses),
            };
            lines.push(Line::from(format!(
                "  {:<22} {:<4}  {:>6}  {:>7}",
                v.full_name().chars().take(22).collect::<String>(),
                v.gp(),
                sv_pct,
                record,
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);

    if app.group_picker.open {
        super::player::render_group_picker(f, app, area);
    }
}

/// Phase Adams.10 — sort `views` in place by the configured key.
/// Pure; called both from render and from L0 tests.
pub fn sort_team_views(
    views: &mut [&icelines_core::stats_repository::PlayerView<'_>],
    sort: TeamSort,
) {
    use std::cmp::Ordering;
    match sort {
        TeamSort::Pace => {
            // Pts/82 desc; None goes last.
            views.sort_by(|a, b| match (a.pace_82(), b.pace_82()) {
                (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            });
        }
        TeamSort::Name => {
            views.sort_by(|a, b| a.full_name().cmp(b.full_name()));
        }
        TeamSort::Position => {
            views.sort_by(|a, b| {
                position_order(a.position().abbreviation())
                    .cmp(&position_order(b.position().abbreviation()))
                    .then_with(|| {
                        b.pace_82()
                            .partial_cmp(&a.pace_82())
                            .unwrap_or(Ordering::Equal)
                    })
            });
        }
        TeamSort::Goals => {
            views.sort_by(|a, b| b.goals().cmp(&a.goals()));
        }
        TeamSort::Hits => {
            // None hits last.
            views.sort_by(|a, b| match (a.hits(), b.hits()) {
                (Some(x), Some(y)) => y.cmp(&x),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            });
        }
    }
}

/// Position canonical order for sorting: C → LW → RW → F → LD → RD → D → G.
fn position_order(p: &str) -> u8 {
    match p {
        "C" => 0,
        "LW" => 1,
        "RW" => 2,
        "F" => 3,
        "LD" => 4,
        "RD" => 5,
        "D" => 6,
        "G" => 7,
        _ => 8,
    }
}

/// View-based goalie collection for a team.
pub(crate) fn collect_team_goalie_views<'a, 'v: 'a>(
    goalie_views: &'a [icelines_core::stats_repository::PlayerView<'v>],
    abbrev: &str,
) -> Vec<&'a icelines_core::stats_repository::PlayerView<'v>> {
    let mut out: Vec<&icelines_core::stats_repository::PlayerView<'v>> = goalie_views
        .iter()
        .filter(|v| v.team_display() == abbrev)
        .collect();
    out.sort_by_key(|v| std::cmp::Reverse(v.gp()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_team_sort_cycles_through_all() {
        let mut s = TeamSort::default();
        let mut seen = vec![s];
        for _ in 0..TeamSort::ALL.len() {
            s = s.next();
            seen.push(s);
        }
        // Cycles back to the start after ALL.len() steps.
        assert_eq!(seen.first(), seen.last());
    }

    #[test]
    fn l0_team_pos_filter_cycles_through_all() {
        let mut p = TeamPosFilter::default();
        let mut seen = vec![p];
        for _ in 0..TeamPosFilter::ALL.len() {
            p = p.next();
            seen.push(p);
        }
        assert_eq!(seen.first(), seen.last());
    }

    #[test]
    fn l0_team_pos_filter_forwards_matches_c_lw_rw() {
        let f = TeamPosFilter::Forwards;
        assert!(f.matches("C"));
        assert!(f.matches("LW"));
        assert!(f.matches("RW"));
        assert!(f.matches("F"));
        assert!(!f.matches("LD"));
        assert!(!f.matches("RD"));
        assert!(!f.matches("D"));
        assert!(!f.matches("G"));
    }

    #[test]
    fn l0_team_pos_filter_defense_matches_ld_rd_d() {
        let f = TeamPosFilter::Defense;
        assert!(f.matches("LD"));
        assert!(f.matches("RD"));
        assert!(f.matches("D"));
        assert!(!f.matches("C"));
        assert!(!f.matches("LW"));
        assert!(!f.matches("G"));
    }

    #[test]
    fn l0_team_pos_filter_lw_only_matches_lw() {
        let f = TeamPosFilter::LW;
        assert!(f.matches("LW"));
        assert!(!f.matches("C"));
        assert!(!f.matches("RW"));
        assert!(!f.matches("LD"));
    }

    #[test]
    fn l0_team_pos_filter_all_matches_everything() {
        let f = TeamPosFilter::All;
        for p in &["C", "LW", "RW", "F", "LD", "RD", "D", "G"] {
            assert!(f.matches(p), "All must accept {p}");
        }
    }

    #[test]
    fn l0_team_state_default_is_pace_all() {
        let s = TeamScreenState::default();
        assert_eq!(s.sort, TeamSort::Pace);
        assert_eq!(s.pos_filter, TeamPosFilter::All);
    }

    #[test]
    fn l0_team_chrome_advertises_sort_and_pos() {
        let s = TeamScreenState::default();
        let c = chrome(&s);
        assert!(c.title.contains("sort=Pts/82"));
        assert!(c.title.contains("pos=All"));
        let keys: Vec<&'static str> = c.keybinds.iter().map(|k| k.key).collect();
        assert!(keys.contains(&"s"));
        assert!(keys.contains(&"p"));
    }

    #[test]
    fn l0_team_chrome_title_reflects_state() {
        let s = TeamScreenState {
            sort: TeamSort::Hits,
            pos_filter: TeamPosFilter::Forwards,
        };
        let c = chrome(&s);
        assert!(c.title.contains("sort=Hits"));
        assert!(c.title.contains("pos=F"));
    }
}
