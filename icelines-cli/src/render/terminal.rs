#![allow(dead_code)]

use comfy_table::{Cell, Color, Table};
use icelines_core::stats_repository::PlayerView;
use icelines_core::view_model::{DepthGoalieSlot, DepthPlayerSlot};
use icelines_core::{
    classify_fit, DepthChart, DepthChartSlot, FitClass, MetricValue, TeamDepthView,
};
use owo_colors::OwoColorize;

// ── Color helpers ─────────────────────────────────────────────────────────────

/// Map a `FitClass` to a comfy-table `Color` for table cells.
fn fit_color(fc: FitClass) -> Color {
    match fc {
        FitClass::Elite => Color::Green,
        FitClass::Solid => Color::Yellow,
        FitClass::Buried => Color::Blue,
        FitClass::Stretch => Color::Red,
    }
}

/// Format a depth-chart slot cell value.
///
/// When `no_color` is false the cell foreground color is set via comfy-table.
/// When `no_color` is true the FitClass label is prepended as plain text
/// (e.g. "[Elite] McDavid 140.0").
fn slot_cell(slot: &DepthChartSlot, no_color: bool) -> Cell {
    let pace_str = slot.pace_82.map(|p| format!(" {p:.1}")).unwrap_or_default();

    if no_color {
        let fc = slot.pace_82.map(|p| classify_fit(p, slot.position));
        let prefix = fc.map(|f| format!("[{}] ", f.label())).unwrap_or_default();
        Cell::new(format!("{}{}{}", prefix, slot.full_name, pace_str))
    } else {
        let fc = slot.pace_82.map(|p| classify_fit(p, slot.position));
        let color = fc.map(fit_color).unwrap_or(Color::Reset);
        Cell::new(format!("{}{}", slot.full_name, pace_str)).fg(color)
    }
}

fn empty_cell() -> Cell {
    Cell::new("—")
}

// ── render_team_card ─────────────────────────────────────────────────────────

/// Print a team depth chart to stdout.
///
/// Forwards are shown in a 4×3 grid (lines × [LW, C, RW]).
/// Defense is shown in a 3×2 grid (pairs × [D, D]).
fn metric_f64(metrics: &[icelines_core::MetricCell], key: &str) -> Option<f64> {
    metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Decimal(value) => Some(value),
                MetricValue::Integer(value) => Some(value as f64),
                MetricValue::Missing | MetricValue::Text(_) => None,
            }
        } else {
            None
        }
    })
}

fn metric_u32(metrics: &[icelines_core::MetricCell], key: &str) -> Option<u32> {
    metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Integer(value) => u32::try_from(value).ok(),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn view_slot_cell(slot: &DepthPlayerSlot, no_color: bool) -> Cell {
    let pace = metric_f64(&slot.metrics, "pace_82");
    let pace_str = pace.map(|p| format!(" {p:.1}")).unwrap_or_default();

    if no_color {
        let prefix = pace
            .map(|p| format!("[{}] ", classify_fit(p, slot.position).label()))
            .unwrap_or_default();
        Cell::new(format!("{}{}{}", prefix, slot.display_name, pace_str))
    } else {
        let color = pace
            .map(|p| fit_color(classify_fit(p, slot.position)))
            .unwrap_or(Color::Reset);
        Cell::new(format!("{}{}", slot.display_name, pace_str)).fg(color)
    }
}

fn goalie_line(slot: &DepthGoalieSlot) -> String {
    let gp = metric_u32(&slot.metrics, "gp")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "â€”".to_owned());
    let starts = metric_u32(&slot.metrics, "starts")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "â€”".to_owned());
    let save_pct = metric_f64(&slot.metrics, "save_pct")
        .map(|v| format!("{v:.3}"))
        .unwrap_or_else(|| "â€”".to_owned());
    format!(
        "  {:<24} {:<8} GP {}  GS {}  SV% {}",
        slot.display_name, slot.role, gp, starts, save_pct
    )
}

pub fn render_team_card(chart: &DepthChart, no_color: bool) {
    println!("\n=== {} — {} ===\n", chart.team.as_str(), chart.season);

    // ── Forwards ─────────────────────────────────────────────────────────────
    println!("FORWARDS");
    let mut fwd_table = Table::new();
    fwd_table.set_header(vec!["Line", "LW", "C", "RW"]);

    for (i, row) in chart.forward_lines.iter().enumerate() {
        let line_label = format!("L{}", i + 1);
        let lw = row[0]
            .as_ref()
            .map(|s| slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        let c = row[1]
            .as_ref()
            .map(|s| slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        let rw = row[2]
            .as_ref()
            .map(|s| slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        fwd_table.add_row(vec![Cell::new(line_label), lw, c, rw]);
    }
    println!("{fwd_table}");

    // ── Defense ──────────────────────────────────────────────────────────────
    println!("\nDEFENSE");
    let mut def_table = Table::new();
    def_table.set_header(vec!["Pair", "D1", "D2"]);

    for (i, pair) in chart.defense_pairs.iter().enumerate() {
        let pair_label = format!("P{}", i + 1);
        let d1 = pair[0]
            .as_ref()
            .map(|s| slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        let d2 = pair[1]
            .as_ref()
            .map(|s| slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        def_table.add_row(vec![Cell::new(pair_label), d1, d2]);
    }
    println!("{def_table}");

    if !chart.unplaced.is_empty() {
        println!("\nAdditional ({}):", chart.unplaced.len());
        for s in &chart.unplaced {
            let ppg = s
                .pace_82
                .map(|p| format!("{:.2}", p / 82.0))
                .unwrap_or_else(|| "—".to_owned());
            let gp =
                s.gp.map(|g| g.to_string())
                    .unwrap_or_else(|| "—".to_owned());
            println!(
                "  {:<24} {}  {}gp  {}",
                s.full_name,
                s.position.abbreviation(),
                gp,
                ppg
            );
        }
    }
    if !chart.below_min_gp.is_empty() {
        println!("\nBelow min GP ({}):", chart.below_min_gp.len());
        for s in &chart.below_min_gp {
            let gp =
                s.gp.map(|g| g.to_string())
                    .unwrap_or_else(|| "0".to_owned());
            println!(
                "  {:<24} {}  {}gp",
                s.full_name,
                s.position.abbreviation(),
                gp
            );
        }
    }
}

// ── render_rank_table ────────────────────────────────────────────────────────

/// Print a ranked player table to stdout from PlayerView slices.
///
/// Columns: Rank | Name | Team | Pos | GP | PPG | Proj
/// The Proj column is colored by FitClass (or prefixed when no_color=true).
///
/// Hart.5c.7: was render_rank_table(&[Player]); now operates on
/// PlayerView<'_> directly. Caller is responsible for filtering /
/// sorting / position-filtering before passing the slice in (pace
/// score is read via `view.pace_82()`).
pub fn render_team_depth_view(view: &TeamDepthView, no_color: bool) {
    println!(
        "\n=== {} â€” {} ===\n",
        view.team.as_str(),
        view.context.window.season
    );

    println!("FORWARDS");
    let mut fwd_table = Table::new();
    fwd_table.set_header(vec!["Line", "LW", "C", "RW"]);

    for line in &view.forward_lines {
        let line_label = format!("L{}", line.line);
        let lw = line
            .left
            .as_ref()
            .map(|s| view_slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        let c = line
            .center
            .as_ref()
            .map(|s| view_slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        let rw = line
            .right
            .as_ref()
            .map(|s| view_slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        fwd_table.add_row(vec![Cell::new(line_label), lw, c, rw]);
    }
    println!("{fwd_table}");

    println!("\nDEFENSE");
    let mut def_table = Table::new();
    def_table.set_header(vec!["Pair", "D1", "D2"]);

    for pair in &view.defense_pairs {
        let pair_label = format!("P{}", pair.pair);
        let left = pair
            .left
            .as_ref()
            .map(|s| view_slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        let right = pair
            .right
            .as_ref()
            .map(|s| view_slot_cell(s, no_color))
            .unwrap_or_else(empty_cell);
        def_table.add_row(vec![Cell::new(pair_label), left, right]);
    }
    println!("{def_table}");

    if !view.goalies.is_empty() {
        println!("\nGOALIES");
        for goalie in &view.goalies {
            println!("{}", goalie_line(goalie));
        }
    }

    if !view.extras.is_empty() {
        println!("\nAdditional ({}):", view.extras.len());
        for slot in &view.extras {
            let ppg = metric_f64(&slot.metrics, "pace_82")
                .map(|p| format!("{:.2}", p / 82.0))
                .unwrap_or_else(|| "â€”".to_owned());
            let gp = metric_u32(&slot.metrics, "gp")
                .map(|g| g.to_string())
                .unwrap_or_else(|| "â€”".to_owned());
            println!(
                "  {:<24} {}  {}gp  {}",
                slot.display_name,
                slot.position.abbreviation(),
                gp,
                ppg
            );
        }
    }
}

pub fn render_rank_table(views: &[&PlayerView<'_>], top: usize, no_color: bool) {
    if views.is_empty() {
        println!("No rankable players found.");
        return;
    }

    let mut table = Table::new();
    table.set_header(vec!["Rank", "Name", "Team", "Pos", "GP", "PPG", "Proj"]);

    for (rank, v) in views.iter().take(top).enumerate() {
        // Caller filtered to rankable views — pace_82() should be Some.
        let pace_82 = v
            .pace_82()
            .expect("view passed by caller has no pace_82 — caller must filter on is_rankable()");
        let gp = v.gp();
        let gp_str = gp.to_string();
        let ppg_str = if gp > 0 {
            format!("{:.2}", pace_82 / 82.0)
        } else {
            "—".to_owned()
        };
        let proj_str = format!("{:.1}", pace_82);

        let fc = classify_fit(pace_82, v.position());
        let proj_cell = if no_color {
            Cell::new(format!("[{}] {}", fc.label(), proj_str))
        } else {
            Cell::new(proj_str).fg(fit_color(fc))
        };

        table.add_row(vec![
            Cell::new((rank + 1).to_string()),
            Cell::new(v.full_name()),
            Cell::new(v.team_display()),
            Cell::new(v.position().abbreviation()),
            Cell::new(gp_str),
            Cell::new(ppg_str),
            proj_cell,
        ]);
    }

    println!("\n{table}");

    // Quiet usage of owo_colors to satisfy the compiler — the dependency is
    // intentionally available for callers who need inline colorization outside
    // comfy-table (e.g. single-line status messages).
    let _ = "".green();
}
