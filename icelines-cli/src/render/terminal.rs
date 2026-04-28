#![allow(dead_code)]

use comfy_table::{Cell, Color, Table};
use icelines_core::{classify_fit, DepthChart, FitClass, Player, Position};
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

/// Format a player cell value.
///
/// When `no_color` is false the cell foreground color is set via comfy-table.
/// When `no_color` is true the FitClass label is prepended as plain text
/// (e.g. "[Elite] McDavid 140.0").
fn player_cell(player: &Player, no_color: bool) -> Cell {
    let pace_str = player
        .pace_score
        .map(|ps| format!(" {:.1}", ps.pace_82))
        .unwrap_or_default();

    if no_color {
        let fc = player
            .pace_score
            .map(|ps| classify_fit(ps.pace_82, player.position));
        let prefix = fc.map(|f| format!("[{}] ", f.label())).unwrap_or_default();
        Cell::new(format!("{}{}{}", prefix, player.full_name, pace_str))
    } else {
        let fc = player
            .pace_score
            .map(|ps| classify_fit(ps.pace_82, player.position));
        let color = fc.map(fit_color).unwrap_or(Color::Reset);
        Cell::new(format!("{}{}", player.full_name, pace_str)).fg(color)
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
            .map(|p| player_cell(p, no_color))
            .unwrap_or_else(empty_cell);
        let c = row[1]
            .as_ref()
            .map(|p| player_cell(p, no_color))
            .unwrap_or_else(empty_cell);
        let rw = row[2]
            .as_ref()
            .map(|p| player_cell(p, no_color))
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
            .map(|p| player_cell(p, no_color))
            .unwrap_or_else(empty_cell);
        let d2 = pair[1]
            .as_ref()
            .map(|p| player_cell(p, no_color))
            .unwrap_or_else(empty_cell);
        def_table.add_row(vec![Cell::new(pair_label), d1, d2]);
    }
    println!("{def_table}");

    if !chart.unplaced.is_empty() {
        println!("\nAdditional ({}):", chart.unplaced.len());
        for p in &chart.unplaced {
            let ppg = p.pace_score.map(|s| format!("{:.2}", s.pace_82 / 82.0)).unwrap_or_else(|| "—".to_owned());
            let gp  = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
            println!("  {:<24} {}  {}gp  {}", p.full_name, p.position.abbreviation(), gp, ppg);
        }
    }
    if !chart.below_min_gp.is_empty() {
        println!("\nBelow min GP ({}):", chart.below_min_gp.len());
        for p in &chart.below_min_gp {
            let gp = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "0".to_owned());
            println!("  {:<24} {}  {}gp", p.full_name, p.position.abbreviation(), gp);
        }
    }
}

// ── render_rank_table ────────────────────────────────────────────────────────

/// Print a ranked player table to stdout.
///
/// Columns: Rank | Name | Team | Pos | GP | PPG | Proj
/// The Proj column is colored by FitClass (or prefixed when no_color=true).
pub fn render_rank_table(
    players: &[Player],
    top: usize,
    pos_filter: Option<Position>,
    no_color: bool,
) {
    let filtered: Vec<&Player> = players
        .iter()
        .filter(|p| {
            if let Some(pos) = pos_filter {
                p.position == pos
            } else {
                true
            }
        })
        .filter(|p| p.is_rankable())
        .take(top)
        .collect();

    if filtered.is_empty() {
        println!("No rankable players found.");
        return;
    }

    let mut table = Table::new();
    table.set_header(vec!["Rank", "Name", "Team", "Pos", "GP", "PPG", "Proj"]);

    for (rank, player) in filtered.iter().enumerate() {
        // All rankable players have a pace_score — safe to unwrap with message.
        let ps = player
            .pace_score
            .expect("player passed is_rankable() but has no pace_score");

        let gp_str = ps.gp.to_string();
        let ppg_str = format!("{:.2}", ps.raw_points as f64 / ps.gp as f64);
        let proj_str = format!("{:.1}", ps.pace_82);

        let fc = classify_fit(ps.pace_82, player.position);
        let proj_cell = if no_color {
            Cell::new(format!("[{}] {}", fc.label(), proj_str))
        } else {
            Cell::new(proj_str).fg(fit_color(fc))
        };

        table.add_row(vec![
            Cell::new((rank + 1).to_string()),
            Cell::new(&player.full_name),
            Cell::new(player.team.as_str()),
            Cell::new(player.position.abbreviation()),
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
