use crate::tui::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect, abbrev: &str) {
    let block = Block::default().borders(Borders::ALL).title(format!(
        " {} — Roster  ·  g: add to group  ·  Enter: player card  ·  Esc: back ",
        abbrev
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Hart.5c.6 Phase B-1.3 — skater roster via team_views (last-stint
    // index). Per spec D10: team page = current roster, so last-stint.
    // Mid-season trades show only on the player's current team here;
    // historical "played for both" lives on the player page.
    let team_abbr = icelines_core::model::TeamAbbr(abbrev.to_owned());
    let team_views = app.team_views(&team_abbr);

    if team_views.is_empty() {
        let msg = vec![
            Line::from(format!("  {} — Lineup Card", abbrev)),
            Line::from(""),
            Line::from("  Run `icelines fetch all` to load roster data."),
            Line::from(""),
            Line::from("  4×3 forward grid + 3×2 defense pairs will appear here"),
            Line::from("  with fit colors: ★ elite  ~ solid  ↑ buried  ↓ stretch"),
        ];
        f.render_widget(Paragraph::new(msg), inner);
        return;
    }

    let mut lines: Vec<Line> = vec![
        Line::from(format!(
            "  {} players  ·  ↑↓ select  ·  Enter: open player card",
            team_views.len()
        )),
        Line::from(""),
        Line::from(format!(
            "  {:<22} {:<4}  {:>6}  {:>7}",
            "Player", "Pos", "PPG", "Pts/82"
        )),
        Line::from(format!("  {}", "─".repeat(46))),
    ];

    for (i, v) in team_views.iter().enumerate() {
        let p82 = v.pace_82();
        let ppg = p82
            .map(|p| format!("{:.3}", p / 82.0))
            .unwrap_or_else(|| "—".to_owned());
        let proj = p82
            .map(|p| format!("{:.1}", p))
            .unwrap_or_else(|| "—".to_owned());
        let name = v.full_name().chars().take(22).collect::<String>();

        let text = format!(
            "  {:<22} {:<4}  {:>6}  {:>7}",
            name,
            v.position().abbreviation(),
            ppg,
            proj
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

    // ── Goalie strip (Phase G.4) — Hart.5c.6 Phase B-3 view-based ─────────
    // Filter goalie_views to this team and order by GP desc (starter
    // first). Mirrors `collect_team_goalies` semantics.
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
                v.gp(), // post-Hart canonical GP source
                sv_pct,
                record,
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);

    // Group picker overlay — shown when user presses g on a team roster row
    if app.group_picker_open {
        super::player::render_group_picker(f, app, area);
    }
}

/// Pick out the goalies on `abbrev` and order them by GP descending so
/// View-based goalie collection for a team. Filters `goalie_views`
/// to the given team abbrev and orders by `view.gp()` descending
/// (starter first). Used by Screen::Team and Screen::DepthTeam goalie
/// strips.
pub(crate) fn collect_team_goalie_views<'a, 'v: 'a>(
    goalie_views: &'a [icelines_core::stats_repository::PlayerView<'v>],
    abbrev: &str,
) -> Vec<&'a icelines_core::stats_repository::PlayerView<'v>> {
    let mut out: Vec<&icelines_core::stats_repository::PlayerView<'v>> = goalie_views
        .iter()
        .filter(|v| v.team_display() == abbrev)
        .collect();
    out.sort_by(|a, b| b.gp().cmp(&a.gp()));
    out
}
