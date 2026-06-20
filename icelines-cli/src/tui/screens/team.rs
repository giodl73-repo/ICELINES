use crate::tui::app::App;
#[cfg(test)]
use crate::tui::filter_state::COUNTRY_CYCLE;
use crate::tui::filter_state::{ForcedColumns, RosterFilterState};
#[cfg(test)]
pub type TeamPosFilter = crate::tui::filter_state::PosFilter;
use crate::visual::{tui_header_style, tui_meta_style, tui_panel_block, tui_selected_style};
use ratatui::{layout::Rect, text::Line, widgets::Paragraph, Frame};

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

/// Phase Adams.12 — Team screen state. `country_filter` is
/// `None` for "all countries"; `Some(code)` matches the bio's
/// `nationality_code`. `force_hits_column` shows the Hits
/// column regardless of sort key (so the user can sort by
/// Points but still see Hits).
#[derive(Debug, Clone, Default)]
pub struct TeamScreenState {
    pub sort: TeamSort,
    pub filters: RosterFilterState,
}

impl TeamScreenState {
    /// Phase Adams.12 — cycle the country filter through
    /// COUNTRY_CYCLE plus the "all" position. Order: None →
    /// CAN → USA → … → SVK → None.
    pub fn cycle_country(&mut self) {
        self.filters.cycle_country();
    }

    /// Pretty label for the chrome title — `All` when no
    /// filter, otherwise the 3-letter ISO code.
    pub fn country_label(&self) -> &str {
        self.filters.country_label()
    }

    pub fn hits_column_forced(&self) -> bool {
        self.filters.forced_columns.contains(ForcedColumns::HITS)
    }
}

// ── Phase Masterton.1 / Adams.10 — chrome accessor ───────────────────────────

pub fn chrome(state: &TeamScreenState) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    let title = format!(
        "Team · sort={} · pos={} · country={} · hits={}",
        state.sort.label(),
        state.filters.pos_filter.label(),
        state.country_label(),
        if state.hits_column_forced() {
            "on"
        } else {
            "off"
        }
    );
    let keybinds = vec![
        KeyHint::new("s", "cycle sort"),
        KeyHint::new("p", "cycle pos"),
        KeyHint::new("c", "cycle country"),
        KeyHint::new("h", "toggle hits col"),
        KeyHint::new("↑↓", "select"),
        KeyHint::new("Enter", "open card"),
        KeyHint::new("g", "add to group"),
    ];
    ScreenChrome { title, keybinds }
}

// ── Render ───────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App, area: Rect, abbrev: &str) {
    let block = tui_panel_block(format!(
        " {} — Roster  ·  s: sort ({})  ·  p: pos ({})  ·  c: country ({})  ·  h: hits ({})  ·  Enter: open ",
        abbrev,
        app.team.sort.label(),
        app.team.filters.pos_filter.label(),
        app.team.country_label(),
        if app.team.hits_column_forced() {
            "on"
        } else {
            "off"
        },
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

    // Apply position + country filter, then sort. Filter first
    // so the sort doesn't waste cycles on dropped rows.
    let mut filtered: Vec<&icelines_core::stats_repository::PlayerView<'_>> = team_views
        .iter()
        .filter(|v| app.team.filters.matches_view(v))
        .collect();
    sort_team_views(&mut filtered, app.team.sort);

    // Hits column is shown when sort=Hits OR force_hits_column.
    let show_hits = app.team.hits_column_forced() || matches!(app.team.sort, TeamSort::Hits);
    let show_goals = matches!(app.team.sort, TeamSort::Goals);
    let mut header_extra = String::new();
    if show_goals {
        header_extra.push_str("    G");
    }
    if show_hits {
        header_extra.push_str("  Hits");
    }
    let max_p82 = filtered
        .iter()
        .filter_map(|v| v.pace_82())
        .filter(|p| p.is_finite() && *p > 0.0)
        .fold(0.0, f64::max);
    let mut lines: Vec<Line> = vec![
        Line::from(format!(
            "  {} of {} players  ·  sort: {}  ·  pos: {}  ·  country: {}  ·  s/p/c/h cycle",
            filtered.len(),
            team_views.len(),
            app.team.sort.label(),
            app.team.filters.pos_filter.label(),
            app.team.country_label(),
        )),
        Line::from(""),
        Line::from(format!(
            "  {:<20} {:<4}  {:>6}  {:>7} {:<4}{}",
            "Player", "Pos", "PPG", "Pts/82", "bar", header_extra
        )),
        Line::from(format!(
            "  {}",
            "─".repeat(51 + header_extra.chars().count())
        )),
    ];

    for (i, v) in filtered.iter().enumerate() {
        let p82 = v.pace_82();
        let ppg = p82
            .map(|p| format!("{:.3}", p / 82.0))
            .unwrap_or_else(|| "—".to_owned());
        let proj = p82
            .map(|p| format!("{:.1}", p))
            .unwrap_or_else(|| "—".to_owned());
        let bar = pace_bar(p82, max_p82);
        let name = v.full_name().chars().take(20).collect::<String>();
        let mut extra = String::new();
        if show_goals {
            extra.push_str(&format!("  {:>4}", v.goals()));
        }
        if show_hits {
            extra.push_str(&format!(
                "  {:>4}",
                v.hits()
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| "—".to_owned())
            ));
        }

        let text = format!(
            "  {:<20} {:<4}  {:>6}  {:>7} {:<4}{}",
            name,
            v.position().abbreviation(),
            ppg,
            proj,
            bar,
            extra,
        );

        let style = if i == app.selected {
            tui_selected_style()
        } else {
            tui_meta_style()
        };
        lines.push(Line::styled(text, style));
    }

    // Goalie strip (unchanged from pre-Adams.10).
    let goalie_views = app.goalie_views();
    let team_goalies = collect_team_goalie_views(&goalie_views, abbrev);
    if !team_goalies.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::styled("  GOALTENDING", tui_header_style()));
        lines.push(Line::styled(
            format!(
                "  {:<22} {:<4}  {:>6}  {:>7}",
                "Goalie", "GP", "SV%", "Record"
            ),
            tui_meta_style(),
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
            views.sort_by_key(|view| std::cmp::Reverse(view.goals()));
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

fn pace_bar(pace_82: Option<f64>, max_pace_82: f64) -> String {
    let Some(value) = pace_82 else {
        return String::new();
    };
    if !value.is_finite() || value <= 0.0 || !max_pace_82.is_finite() || max_pace_82 <= 0.0 {
        return String::new();
    }

    let filled = ((value / max_pace_82) * 4.0).ceil() as usize;
    "#".repeat(filled.clamp(1, 4))
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
    fn l0_team_pace_bar_scales_and_skips_missing_values() {
        assert_eq!(pace_bar(None, 80.0), "");
        assert_eq!(pace_bar(Some(0.0), 80.0), "");
        assert_eq!(pace_bar(Some(f64::NAN), 80.0), "");
        assert_eq!(pace_bar(Some(20.0), 80.0), "#");
        assert_eq!(pace_bar(Some(40.0), 80.0), "##");
        assert_eq!(pace_bar(Some(80.0), 80.0), "####");
        assert_eq!(pace_bar(Some(80.0), 0.0), "");
    }

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
        assert_eq!(s.filters.pos_filter, TeamPosFilter::All);
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
        assert!(keys.contains(&"c"), "Adams.12: country keybind exposed");
        assert!(keys.contains(&"h"), "Adams.12: hits-toggle keybind exposed");
    }

    #[test]
    fn l0_team_chrome_title_reflects_state() {
        let mut s = TeamScreenState {
            sort: TeamSort::Hits,
            filters: RosterFilterState::default(),
        };
        s.filters.pos_filter = TeamPosFilter::Forwards;
        s.filters.country_filter = Some(crate::tui::filter_state::CountryCode::CAN);
        s.filters.forced_columns.toggle(ForcedColumns::HITS);
        let c = chrome(&s);
        assert!(c.title.contains("sort=Hits"));
        assert!(c.title.contains("pos=F"));
        assert!(c.title.contains("country=CAN"));
        assert!(c.title.contains("hits=on"));
    }

    // ── Phase Adams.12 — country cycle + hits-column toggle ────────────────

    #[test]
    fn l0_team_country_cycles_through_all_codes() {
        let mut s = TeamScreenState::default();
        assert_eq!(s.filters.country_filter, None);
        s.cycle_country();
        assert_eq!(
            s.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::CAN)
        );
        s.cycle_country();
        assert_eq!(
            s.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::USA)
        );
        s.cycle_country();
        assert_eq!(
            s.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::SWE)
        );
        // Step through the rest.
        for _ in 0..(COUNTRY_CYCLE.len() - 3) {
            s.cycle_country();
        }
        // After cycling through all of COUNTRY_CYCLE, returns to None.
        s.cycle_country();
        assert_eq!(
            s.filters.country_filter, None,
            "cycle_country must wrap None → CAN → … → SVK → None"
        );
    }

    #[test]
    fn l0_team_country_label_is_all_when_none() {
        let s = TeamScreenState::default();
        assert_eq!(s.country_label(), "All");
    }

    #[test]
    fn l0_team_country_label_uses_iso_code_when_set() {
        let mut s = TeamScreenState::default();
        s.filters.country_filter = Some(crate::tui::filter_state::CountryCode::FIN);
        assert_eq!(s.country_label(), "FIN");
    }

    #[test]
    fn l0_team_default_force_hits_column_is_false() {
        let s = TeamScreenState::default();
        assert!(!s.hits_column_forced());
    }

    #[test]
    fn l0_team_country_cycle_includes_canonical_codes() {
        // The cycle must include CAN, USA, SWE, FIN — the four
        // countries that account for ~85% of NHL roster.
        for code in &[
            crate::tui::filter_state::CountryCode::CAN,
            crate::tui::filter_state::CountryCode::USA,
            crate::tui::filter_state::CountryCode::SWE,
            crate::tui::filter_state::CountryCode::FIN,
        ] {
            assert!(
                COUNTRY_CYCLE.contains(code),
                "COUNTRY_CYCLE must include {code:?}"
            );
        }
    }
}
