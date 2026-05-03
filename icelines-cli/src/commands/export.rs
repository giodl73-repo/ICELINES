//! `icelines export md <shape>` — Phase 8d.
//!
//! Produces deterministic markdown tables consumable by external readers.
//! Every shape follows the format from `design/specs/export-markdown.md`:
//!   - YAML front-matter (`type`, `title`, `season`, `filters`, hints)
//!   - One or more GitHub-flavored markdown tables in the body
//!
//! The writer prints to stdout with `--out -`, otherwise writes to the
//! given path, otherwise to `~/.icelines/reports/{shape}.md`.
//!
//! Hart.5c.5: every renderer takes `&[PlayerView<'_>]`. The five
//! shapes (leaders, team, depth, compare, roster) preserve their
//! exact deterministic output (front-matter format, table columns,
//! sort tie-breaks).

use anyhow::{bail, Context};
use std::fmt::Write as _;
use std::path::PathBuf;

use icelines_core::{
    cross_team::compute_all_views, filter::PlayerFilter, scoring::sort_views_by_pace,
    stats_repository::PlayerView,
};

use crate::cli::{ExportSubcommand, MdShape};
use crate::commands::players::load_repo_for_season;

pub async fn run(cmd: ExportSubcommand) -> anyhow::Result<()> {
    match cmd {
        ExportSubcommand::Md {
            shape,
            out,
            pos,
            team,
            top,
            sort,
            gp_min,
            columns,
            p1,
            p2,
            series: _,
            width,
            height,
        } => {
            let body = match shape {
                MdShape::Leaders => render_leaders(LeadersOpts {
                    pos: pos.clone(),
                    top,
                    sort: sort.clone(),
                    gp_min,
                    columns: columns.clone(),
                    width,
                    height,
                })?,
                MdShape::Team => render_team(TeamOpts {
                    team: team
                        .clone()
                        .context("--team is required for `export md team`")?,
                    width,
                    height,
                })?,
                MdShape::Depth => render_depth(DepthOpts { width, height })?,
                MdShape::Compare => render_compare(CompareOpts {
                    p1: p1
                        .clone()
                        .context("--p1 is required for `export md compare`")?,
                    p2: p2
                        .clone()
                        .context("--p2 is required for `export md compare`")?,
                    width,
                    height,
                })?,
                MdShape::Roster => render_roster(RosterOpts {
                    pos: pos.clone(),
                    width,
                    height,
                })?,
                MdShape::Fantasy | MdShape::Series => {
                    bail!(
                        "shape `{}` is deferred — `fantasy` needs FantasyDb + scheme \
                         integration, `series` needs the historical playoffs.json bundle \
                         (Phase 8c). Tracked in design/plans/2026-04-28-spec-delta-catchup.md.",
                        shape.label(),
                    )
                }
            };
            write_or_print(&out, &shape, &body)?;
        }
    }
    Ok(())
}

/// Helper used by every render entry point: load a `LoadOutcome` and
/// collect the skater views into a Vec the renderer borrows from. The
/// outcome must outlive the views.
fn load_views() -> anyhow::Result<icelines_fetch::stats_loader::LoadOutcome> {
    let (outcome, _season, _ty) = load_repo_for_season(None, None)?;
    Ok(outcome)
}

// ── leaders ──────────────────────────────────────────────────────────────────

pub(crate) struct LeadersOpts {
    pub pos: Option<String>,
    pub top: usize,
    pub sort: String,
    pub gp_min: Option<u32>,
    /// Phase Lindsay L.5.4 — optional StatId column override. None = canonical.
    pub columns: Option<String>,
    pub width: u16,
    pub height: u16,
}

/// Phase Lindsay L.5.4 — parse `--columns "g,a,p,hits,blocks"` into a
/// `Vec<StatId>`. Whitespace around commas tolerated; empty string after
/// trim returns Ok(vec![]). Unknown keys bail with the list-of-valid hint.
pub(crate) fn parse_columns_list(
    s: &str,
) -> anyhow::Result<Vec<icelines_core::stats_catalog::StatId>> {
    use icelines_core::stats_catalog::StatId;
    let mut out: Vec<StatId> = Vec::new();
    for raw in s.split(',') {
        let key = raw.trim();
        if key.is_empty() {
            continue;
        }
        let sid = StatId::from_cli_key(key).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown column `{key}`. Valid keys are any StatId::cli_key — \
                 see `icelines query leaders --help` or the catalog \
                 documentation for the full list."
            )
        })?;
        out.push(sid);
    }
    Ok(out)
}

pub(crate) fn render_leaders(opts: LeadersOpts) -> anyhow::Result<String> {
    let outcome = load_views()?;
    let views: Vec<PlayerView<'_>> = outcome
        .repo
        .skaters(
            icelines_core::model::Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .collect();
    render_leaders_from_views(&views, &opts)
}

/// Pure renderer separated from I/O so tests can inject fixture views.
pub(crate) fn render_leaders_from_views(
    views: &[PlayerView<'_>],
    opts: &LeadersOpts,
) -> anyhow::Result<String> {
    let mut filter = PlayerFilter::new();
    if let Some(p) = &opts.pos {
        if !p.is_empty() {
            filter.positions = Some(parse_positions(p));
        }
    }
    if let Some(g) = opts.gp_min {
        filter.gp_min = Some(g);
    }

    let mut filtered: Vec<PlayerView<'_>> = filter.apply_views(views.iter().copied());

    // Sort — only `pts-pace` (default) supported in 8d.1.
    match opts.sort.as_str() {
        "pts-pace" | "pace" | "pts" => {
            filtered.sort_by(|a, b| {
                let pa = a.pace_82().unwrap_or(0.0);
                let pb = b.pace_82().unwrap_or(0.0);
                pb.partial_cmp(&pa)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    // Stable tie-break: player_id asc.
                    .then(a.identity.id.0.cmp(&b.identity.id.0))
            });
        }
        other => {
            bail!("sort metric `{other}` not supported in `export md leaders` v1 (use `pts-pace`)",)
        }
    }

    let top = filtered.into_iter().take(opts.top).collect::<Vec<_>>();

    let title = match &opts.pos {
        Some(p) if !p.is_empty() => format!("Top {} {} (Pts/82)", opts.top, p.to_uppercase()),
        _ => format!("Top {} skaters (Pts/82)", opts.top),
    };

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "leaderboard",
        &title,
        &[
            ("pos", opts.pos.clone().unwrap_or_default()),
            ("top", opts.top.to_string()),
            ("sort", opts.sort.clone()),
            (
                "gp_min",
                opts.gp_min.map(|g| g.to_string()).unwrap_or_default(),
            ),
        ],
        opts.width,
        opts.height,
    );

    // Phase Lindsay L.5.4 — `--columns` overrides the canonical column
    // set. None preserves the v1 hardcoded shape (Rank Player Team Pos
    // Age GP G A Pts PPG Pts/82) byte-identically.
    if let Some(cols_spec) = opts.columns.as_deref() {
        let stat_cols = parse_columns_list(cols_spec)?;
        write_leaders_table_with_columns(&mut out, &top, &stat_cols);
    } else {
        let _ = writeln!(
            out,
            "| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |"
        );
        let _ = writeln!(
            out,
            "|-----:|--------|:----:|:---:|----:|---:|---:|---:|----:|----:|-------:|"
        );
        for (i, v) in top.iter().enumerate() {
            let totals = &v.stats.totals;
            let rank = i + 1;
            let name = truncate(&v.identity.full_name, 24);
            let pos = v.position().abbreviation();
            let age = v
                .identity
                .bio
                .birth_date
                .as_deref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<u16>().ok())
                .map(|y| 2026u16.saturating_sub(y).to_string())
                .unwrap_or_else(|| "—".to_owned());
            let gp = v.gp().to_string();
            let goals = totals.goals;
            let asts = totals.assists;
            let pts = totals.points;
            let ppg = v
                .pace_82()
                .map(|p| format!("{:.3}", p / 82.0))
                .unwrap_or_else(|| "—".to_owned());
            let pts82 = v
                .pace_82()
                .map(|p| format!("{p:.1}"))
                .unwrap_or_else(|| "—".to_owned());
            let _ = writeln!(
                out,
                "| {rank:>4} | {name} | {team} | {pos} | {age:>3} | {gp:>3} | {goals:>3} | {asts:>3} | {pts:>4} | {ppg:>5} | {pts82:>6} |",
                team = v.team_display(),
            );
        }
    }

    Ok(out)
}

/// Phase Lindsay L.5.4 — render leaders table with custom StatId columns.
/// Headers come from `StatId::short_label()`; cells route through the
/// same per-StatUnit formatting (Count → integer, Pct → `XX.X%`,
/// Per60/Rate → `X.XX`, Seconds → `M:SS`, Inverted → `X.XX`).
fn write_leaders_table_with_columns(
    out: &mut String,
    top: &[PlayerView<'_>],
    stat_cols: &[icelines_core::stats_catalog::StatId],
) {
    use icelines_core::stats_catalog::{StatId, StatUnit};
    // Header row.
    out.push_str("| Rank | Player | Team | Pos");
    for sid in stat_cols {
        let _ = write!(out, " | {}", sid.short_label());
    }
    out.push_str(" |\n");
    // Alignment row — right-align numeric columns.
    out.push_str("|-----:|--------|:----:|:---:");
    for _ in stat_cols {
        out.push_str("|----:");
    }
    out.push_str("|\n");
    // Data rows.
    for (i, v) in top.iter().enumerate() {
        let rank = i + 1;
        let name = truncate(&v.identity.full_name, 24);
        let pos = v.position().abbreviation();
        let _ = write!(
            out,
            "| {rank:>4} | {name} | {team} | {pos}",
            team = v.team_display(),
        );
        for sid in stat_cols {
            let cell = render_cell(*sid, v);
            let _ = write!(out, " | {cell}");
        }
        out.push_str(" |\n");
    }

    fn render_cell(sid: icelines_core::stats_catalog::StatId, v: &PlayerView<'_>) -> String {
        match sid.read(v) {
            None => "—".to_owned(),
            Some(val) => match sid.unit() {
                StatUnit::Count => format!("{}", val as i64),
                StatUnit::Seconds => {
                    let s = val as u64;
                    if s < 3600 {
                        format!("{}:{:02}", s / 60, s % 60)
                    } else {
                        format!("{}m", s / 60)
                    }
                }
                StatUnit::Pct => format!("{:.1}%", val * 100.0),
                StatUnit::Per60 | StatUnit::Rate => format!("{val:.2}"),
                StatUnit::Inverted => format!("{val:.2}"),
            },
        }
    }
}

// ── team ─────────────────────────────────────────────────────────────────────

pub(crate) struct TeamOpts {
    pub team: String,
    pub width: u16,
    pub height: u16,
}

pub(crate) fn render_team(opts: TeamOpts) -> anyhow::Result<String> {
    let outcome = load_views()?;
    let views: Vec<PlayerView<'_>> = outcome
        .repo
        .skaters(
            icelines_core::model::Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .collect();
    render_team_from_views(&views, &opts)
}

pub(crate) fn render_team_from_views(
    views: &[PlayerView<'_>],
    opts: &TeamOpts,
) -> anyhow::Result<String> {
    let team_up = opts.team.to_uppercase();
    let mut roster: Vec<PlayerView<'_>> = views
        .iter()
        .filter(|v| v.team_display() == team_up.as_str())
        .copied()
        .collect();
    sort_views_by_pace(&mut roster);

    let title = format!("{team_up} — Lineup card");
    let mut out = String::new();
    write_front_matter(
        &mut out,
        "lineup-card",
        &title,
        &[("team", team_up.clone())],
        opts.width,
        opts.height,
    );

    if roster.is_empty() {
        let _ = writeln!(out, "_No players loaded for {team_up}._");
        return Ok(out);
    }

    let _ = writeln!(out, "## All skaters");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Rank | Player | Pos | GP | G | A | Pts | Pts/82 |");
    let _ = writeln!(out, "|-----:|--------|:---:|---:|---:|---:|----:|-------:|");
    for (i, v) in roster.iter().enumerate() {
        let totals = &v.stats.totals;
        let rank = i + 1;
        let name = truncate(&v.identity.full_name, 24);
        let gp = v.gp().to_string();
        let pts82 = v
            .pace_82()
            .map(|p| format!("{p:.1}"))
            .unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(
            out,
            "| {rank:>4} | {name} | {pos} | {gp:>3} | {g:>3} | {a:>3} | {pts:>4} | {pts82:>6} |",
            pos = v.position().abbreviation(),
            g = totals.goals,
            a = totals.assists,
            pts = totals.points,
        );
    }

    Ok(out)
}

// ── depth ────────────────────────────────────────────────────────────────────

pub(crate) struct DepthOpts {
    pub width: u16,
    pub height: u16,
}

pub(crate) fn render_depth(opts: DepthOpts) -> anyhow::Result<String> {
    let outcome = load_views()?;
    let views: Vec<PlayerView<'_>> = outcome
        .repo
        .skaters(
            icelines_core::model::Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .collect();
    render_depth_from_views(&views, &opts)
}

pub(crate) fn render_depth_from_views(
    views: &[PlayerView<'_>],
    opts: &DepthOpts,
) -> anyhow::Result<String> {
    let metrics = compute_all_views(views);

    // Index views by player_id so we can render names + teams alongside metrics.
    let by_id: std::collections::HashMap<u32, &PlayerView<'_>> =
        views.iter().map(|v| (v.identity.id.0, v)).collect();

    // Sort by delta descending; tie-break: player_nhl_id asc.
    let mut rows: Vec<&icelines_core::cross_team::CrossTeamMetrics> = metrics.iter().collect();
    rows.sort_by(|a, b| {
        b.delta
            .partial_cmp(&a.delta)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.player_nhl_id.cmp(&b.player_nhl_id))
    });

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "depth-rankings",
        "Cross-team line value rankings",
        &[],
        opts.width,
        opts.height,
    );

    let _ = writeln!(
        out,
        "| Rank | Player | Team | Pos | Own line | Avg other | Delta | Fit |"
    );
    let _ = writeln!(
        out,
        "|-----:|--------|:----:|:---:|---------:|----------:|------:|:---:|"
    );
    for (i, m) in rows.iter().take(50).enumerate() {
        let pid = match m.player_nhl_id {
            Some(id) => id,
            None => continue,
        };
        let v = match by_id.get(&pid) {
            Some(v) => *v,
            None => continue,
        };
        let _ = writeln!(
            out,
            "| {rank:>4} | {name} | {team} | {pos} | {own:>8} | {avg:>9.2} | {delta:>+6.2} | {fit} |",
            rank = i + 1,
            name = truncate(&v.identity.full_name, 24),
            team = v.team_display(),
            pos  = v.position().abbreviation(),
            own  = m.own_line,
            avg  = m.avg_other_line,
            delta= m.delta,
            fit  = m.web_fit_class().label(),
        );
    }
    Ok(out)
}

// ── compare ──────────────────────────────────────────────────────────────────

pub(crate) struct CompareOpts {
    pub p1: String,
    pub p2: String,
    pub width: u16,
    pub height: u16,
}

pub(crate) fn render_compare(opts: CompareOpts) -> anyhow::Result<String> {
    let outcome = load_views()?;
    let views: Vec<PlayerView<'_>> = outcome
        .repo
        .skaters(
            icelines_core::model::Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .collect();
    render_compare_from_views(&views, &opts)
}

pub(crate) fn render_compare_from_views(
    views: &[PlayerView<'_>],
    opts: &CompareOpts,
) -> anyhow::Result<String> {
    use icelines_core::name::normalize_name;
    let n1 = normalize_name(&opts.p1);
    let n2 = normalize_name(&opts.p2);
    let a = views
        .iter()
        .find(|v| v.identity.name_normalized.contains(&n1))
        .with_context(|| format!("player '{}' not found", opts.p1))?;
    let b = views
        .iter()
        .find(|v| v.identity.name_normalized.contains(&n2))
        .with_context(|| format!("player '{}' not found", opts.p2))?;

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "compare",
        &format!("{} vs {}", a.identity.full_name, b.identity.full_name),
        &[
            ("p1", a.identity.full_name.clone()),
            ("p2", b.identity.full_name.clone()),
        ],
        opts.width,
        opts.height,
    );

    let _ = writeln!(
        out,
        "| Stat | {} | {} | Diff |",
        a.identity.full_name, b.identity.full_name
    );
    let _ = writeln!(out, "|------|------|------|-----:|");

    let row = |out: &mut String, label: &str, av: f64, bv: f64, fmt: &str| {
        let diff = av - bv;
        let _ = writeln!(
            out,
            "| {label} | {av:fmt$} | {bv:fmt$} | {diff:+.2} |",
            fmt = match fmt {
                "0" => 0,
                _ => 2,
            },
        );
    };
    row(&mut out, "GP", a.gp() as f64, b.gp() as f64, "0");
    row(
        &mut out,
        "G",
        a.stats.totals.goals as f64,
        b.stats.totals.goals as f64,
        "0",
    );
    row(
        &mut out,
        "A",
        a.stats.totals.assists as f64,
        b.stats.totals.assists as f64,
        "0",
    );
    row(
        &mut out,
        "Pts",
        a.stats.totals.points as f64,
        b.stats.totals.points as f64,
        "0",
    );
    let pa = a.pace_82().unwrap_or(0.0);
    let pb = b.pace_82().unwrap_or(0.0);
    let _ = writeln!(out, "| Pts/82 | {pa:.1} | {pb:.1} | {:+.1} |", pa - pb);
    let ppg_a = pa / 82.0;
    let ppg_b = pb / 82.0;
    let _ = writeln!(
        out,
        "| PPG | {ppg_a:.3} | {ppg_b:.3} | {:+.3} |",
        ppg_a - ppg_b
    );
    Ok(out)
}

// ── roster ───────────────────────────────────────────────────────────────────

pub(crate) struct RosterOpts {
    pub pos: Option<String>,
    pub width: u16,
    pub height: u16,
}

pub(crate) fn render_roster(opts: RosterOpts) -> anyhow::Result<String> {
    let outcome = load_views()?;
    let views: Vec<PlayerView<'_>> = outcome
        .repo
        .skaters(
            icelines_core::model::Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .collect();
    render_roster_from_views(&views, &opts)
}

pub(crate) fn render_roster_from_views(
    views: &[PlayerView<'_>],
    opts: &RosterOpts,
) -> anyhow::Result<String> {
    let mut filter = PlayerFilter::new();
    if let Some(p) = &opts.pos {
        if !p.is_empty() {
            filter.positions = Some(parse_positions(p));
        }
    }
    let mut filtered: Vec<PlayerView<'_>> = filter.apply_views(views.iter().copied());
    // Sort by team alpha, then pace desc within team (stable, deterministic).
    filtered.sort_by(|a, b| {
        a.team_display()
            .cmp(b.team_display())
            .then_with(|| {
                let pa = a.pace_82().unwrap_or(0.0);
                let pb = b.pace_82().unwrap_or(0.0);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
            })
            .then(a.identity.id.0.cmp(&b.identity.id.0))
    });

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "roster",
        "All teams — full skater roster",
        &[("pos", opts.pos.clone().unwrap_or_default())],
        opts.width,
        opts.height,
    );

    let _ = writeln!(out, "| Team | Player | Pos | GP | G | A | Pts | Pts/82 |");
    let _ = writeln!(out, "|:----:|--------|:---:|---:|---:|---:|----:|-------:|");
    for v in &filtered {
        let totals = &v.stats.totals;
        let pts82 = v
            .pace_82()
            .map(|p| format!("{p:.1}"))
            .unwrap_or_else(|| "—".to_owned());
        let gp = v.gp().to_string();
        let _ = writeln!(
            out,
            "| {team} | {name} | {pos} | {gp:>3} | {g:>3} | {a:>3} | {pts:>4} | {pts82:>6} |",
            team = v.team_display(),
            name = truncate(&v.identity.full_name, 24),
            pos = v.position().abbreviation(),
            g = totals.goals,
            a = totals.assists,
            pts = totals.points,
        );
    }
    Ok(out)
}

// ── shared helpers ───────────────────────────────────────────────────────────

fn parse_positions(s: &str) -> Vec<icelines_core::model::Position> {
    use icelines_core::model::Position;
    s.split(',')
        .filter_map(|p| match p.trim().to_uppercase().as_str() {
            "C" => Some(vec![Position::Center]),
            "LW" => Some(vec![Position::LeftWing]),
            "RW" => Some(vec![Position::RightWing]),
            "D" => Some(vec![Position::Defense]),
            "F" => Some(vec![
                Position::Center,
                Position::LeftWing,
                Position::RightWing,
            ]),
            _ => None,
        })
        .flatten()
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        s.chars().take(max).collect()
    }
}

fn write_front_matter(
    out: &mut String,
    ty: &str,
    title: &str,
    filters: &[(&str, String)],
    width: u16,
    height: u16,
) {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "type: {ty}");
    let _ = writeln!(out, "title: \"{title}\"");
    let _ = writeln!(out, "generated_at: {now}");
    let _ = writeln!(out, "season: {}", icelines_core::CURRENT_SEASON_STR);
    if !filters.is_empty() {
        let _ = writeln!(out, "filters:");
        for (k, v) in filters {
            if v.is_empty() {
                continue;
            }
            let _ = writeln!(out, "  {k}: \"{v}\"");
        }
    }
    let _ = writeln!(out, "proof:");
    let _ = writeln!(out, "  width: {width}");
    let _ = writeln!(out, "  height: {height}");
    let _ = writeln!(out, "---");
    let _ = writeln!(out);
}

fn write_or_print(out: &Option<String>, shape: &MdShape, body: &str) -> anyhow::Result<()> {
    match out.as_deref() {
        Some("-") | None if matches!(out.as_deref(), Some("-")) => {
            print!("{body}");
            Ok(())
        }
        Some(p) => {
            let path = PathBuf::from(p);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("✓ wrote {} ({} bytes)", path.display(), body.len());
            Ok(())
        }
        None => {
            let home = std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .context("cannot determine home directory")?;
            let path = home
                .join(".icelines")
                .join("reports")
                .join(format!("{}.md", shape.label()));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("✓ wrote {} ({} bytes)", path.display(), body.len());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{
        identity::PlayerId,
        model::{Position, Season},
        season_stats::{SeasonStatsBuilder, SeasonType, StatTotals, TeamStint},
        PaceScore, TeamAbbr,
    };

    /// Build a one-row repo at the given (id, name, team, pos, pace_82)
    /// for export-shape tests. Pace_82 drives sort order; other counters
    /// are filled from a default (30G, 50A, 60GP) shape.
    fn fixture_repo(
        rows: &[(u32, &str, &str, Position, f64)],
    ) -> icelines_core::stats_repository::StatsRepository {
        let mut repo = icelines_core::stats_repository::StatsRepository::new();
        for (id, name, team, pos, pace_82) in rows {
            let identity = icelines_core::fixtures::identity(*id)
                .name(name, &name.to_lowercase().replace(' ', "_"))
                .build();
            let totals = StatTotals {
                gp: 60,
                goals: 30,
                assists: 50,
                points: 80,
                plus_minus: 0,
                pim: 0,
                shots: 0,
                shooting_pct: None,
                toi_per_game_sec: None,
                pp_goals: 0,
                pp_points: 0,
                sh_goals: 0,
                sh_points: 0,
                gwg: 0,
                ot_goals: 0,
                faceoff_win_pct: None,
                pace_score: Some(PaceScore {
                    pace_82: *pace_82,
                    goals_per_82: 30.0,
                    raw_points: 80,
                    gp: 60,
                }),
            };
            let stint = TeamStint {
                team: TeamAbbr((*team).to_owned()),
                started: Some("2024-10-15".into()),
                ended: Some("2025-04-13".into()),
                gp: 60,
                goals: 30,
                assists: 50,
                points: 80,
                goalie: None,
            };
            let stats =
                SeasonStatsBuilder::new(PlayerId(*id), Season(20242025), SeasonType::Regular, *pos)
                    .with_totals(totals)
                    .add_team_stint(stint)
                    .build();
            repo.upsert_identity(identity).unwrap();
            repo.upsert_stats(stats).unwrap();
        }
        repo
    }

    fn fixture_views(
        repo: &icelines_core::stats_repository::StatsRepository,
    ) -> Vec<PlayerView<'_>> {
        repo.skaters(Season(20242025), SeasonType::Regular)
            .collect()
    }

    #[test]
    fn l0_export_leaders_has_required_front_matter() {
        let repo = fixture_repo(&[
            (1, "A One", "EDM", Position::Center, 109.0),
            (2, "B Two", "COL", Position::Center, 95.0),
            (3, "C Tre", "SEA", Position::LeftWing, 80.0),
        ]);
        let views = fixture_views(&repo);
        let opts = LeadersOpts {
            pos: None,
            top: 25,
            sort: "pts-pace".into(),
            gp_min: None,
            columns: None,
            width: 100,
            height: 30,
        };
        let out = render_leaders_from_views(&views, &opts).unwrap();
        assert!(out.starts_with("---\n"));
        assert!(
            out.contains("\n---\n\n|"),
            "front-matter must precede the table"
        );
        assert!(out.contains("type: leaderboard"));
        assert!(out.contains("title:"));
        assert!(out.contains("generated_at:"));
        assert!(out.contains("season: "));
        assert!(out.contains("proof:"));
        assert!(out.contains("width: 100"));
        assert!(out.contains("height: 30"));
    }

    #[test]
    fn l0_export_leaders_table_columns_in_canonical_order() {
        let repo = fixture_repo(&[(1, "Solo", "EDM", Position::Center, 109.0)]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap();
        assert!(
            out.contains("| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |")
        );
        assert!(out.contains("Solo"));
        assert!(out.contains("EDM"));
        assert!(out.contains("109.0"));
    }

    #[test]
    fn l0_export_leaders_sort_is_descending_by_pace() {
        let repo = fixture_repo(&[
            (1, "Mid", "EDM", Position::Center, 80.0),
            (2, "Top", "COL", Position::Center, 110.0),
            (3, "Low", "SEA", Position::LeftWing, 50.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap();
        let i_top = out.find("Top").unwrap();
        let i_mid = out.find("Mid").unwrap();
        let i_low = out.find("Low").unwrap();
        assert!(
            i_top < i_mid && i_mid < i_low,
            "sort order broken: top={i_top} mid={i_mid} low={i_low}"
        );
    }

    #[test]
    fn l0_export_leaders_pos_filter_excludes_others() {
        let repo = fixture_repo(&[
            (1, "Cee", "EDM", Position::Center, 100.0),
            (2, "Dee", "COL", Position::Defense, 90.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: Some("C".into()),
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap();
        assert!(out.contains("Cee"));
        assert!(
            !out.contains("Dee"),
            "defenseman must be filtered out for --pos C"
        );
        assert!(
            out.contains("title: \"Top 25 C"),
            "title must reflect the position filter"
        );
    }

    #[test]
    fn l0_export_leaders_top_limits_rows() {
        let rows: Vec<(u32, String, &str, Position, f64)> = (1..=10)
            .map(|i| {
                (
                    i,
                    format!("P{i}"),
                    "EDM",
                    Position::Center,
                    100.0 - i as f64,
                )
            })
            .collect();
        // Need to re-borrow the strings — fixture_repo takes &[(u32, &str, &str, Position, f64)].
        let row_refs: Vec<(u32, &str, &str, Position, f64)> = rows
            .iter()
            .map(|(i, n, t, p, pace)| (*i, n.as_str(), *t, *p, *pace))
            .collect();
        let repo = fixture_repo(&row_refs);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 3,
                sort: "pts-pace".into(),
                gp_min: None,
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap();
        assert!(out.contains("| P1 ") || out.contains("P1 "));
        assert!(out.contains("P3"));
        assert!(!out.contains("P4"), "top=3 must drop P4-P10");
    }

    #[test]
    fn l0_export_leaders_unknown_sort_errors() {
        let repo = fixture_repo(&[(1, "X", "EDM", Position::Center, 50.0)]);
        let views = fixture_views(&repo);
        let err = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "invalid-metric".into(),
                gp_min: None,
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid-metric"));
    }

    // ── Phase Lindsay L.5.4 — `--columns` StatId list ─────────────────

    /// `parse_columns_list` parses comma-separated StatId::cli_key strings,
    /// trims whitespace, and yields `Vec<StatId>` in declaration order.
    #[test]
    fn l0_lindsay_l5_export_columns_parse_basic() {
        use icelines_core::stats_catalog::StatId;
        let parsed = parse_columns_list("goals,assists,points").unwrap();
        assert_eq!(parsed, vec![StatId::Goals, StatId::Assists, StatId::Points]);
        // Whitespace tolerated.
        let parsed = parse_columns_list("  goals  ,assists,  points  ").unwrap();
        assert_eq!(parsed, vec![StatId::Goals, StatId::Assists, StatId::Points]);
    }

    /// Empty / whitespace-only input parses to empty vec (renderer falls
    /// back to a 4-col table; no panic).
    #[test]
    fn l0_lindsay_l5_export_columns_parse_empty() {
        let parsed = parse_columns_list("").unwrap();
        assert!(parsed.is_empty());
        let parsed = parse_columns_list("   ").unwrap();
        assert!(parsed.is_empty());
        // A trailing comma drops the empty entry, doesn't error.
        let parsed = parse_columns_list("goals,").unwrap();
        assert_eq!(parsed.len(), 1);
    }

    /// Unknown key bails with the actionable hint.
    #[test]
    fn l0_lindsay_l5_export_columns_parse_unknown_bails() {
        let err = parse_columns_list("not-a-real-stat").unwrap_err();
        let s = err.to_string();
        assert!(
            s.contains("not-a-real-stat"),
            "error must mention the bad key — got {s}"
        );
        assert!(
            s.contains("StatId"),
            "error must hint at StatId catalog — got {s}"
        );
    }

    /// `--columns` overrides the canonical column set in the rendered
    /// table. Header reads from `StatId::short_label`; cells from
    /// `StatId::read` + per-StatUnit formatting.
    #[test]
    fn l0_lindsay_l5_export_columns_renders_custom_table() {
        let repo = fixture_repo(&[
            (1, "Alice One", "EDM", Position::Center, 100.0),
            (2, "Bob Two", "EDM", Position::Center, 85.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 10,
                sort: "pts-pace".into(),
                gp_min: None,
                columns: Some("goals,assists,points".into()),
                width: 80,
                height: 30,
            },
        )
        .unwrap();
        // Header reflects StatId::short_label — Points is "P", not "Pts".
        // (The canonical hardcoded shape was "G | A | Pts | PPG | Pts/82";
        // under --columns we emit "G | A | P" since those are the cli_keys
        // requested, mapped to short_label.)
        assert!(
            out.contains("| G | A | P |"),
            "custom header must use short_label values — got:\n{out}"
        );
        // The canonical "Pts/82" column is dropped under --columns.
        assert!(
            !out.contains("Pts/82 |"),
            "Pts/82 column header must be absent under custom columns"
        );
    }

    #[test]
    fn l0_export_team_card_lists_only_target_team() {
        let repo = fixture_repo(&[
            (1, "Edm One", "EDM", Position::Center, 100.0),
            (2, "Col Two", "COL", Position::Center, 95.0),
            (3, "Edm Tre", "EDM", Position::Defense, 70.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_team_from_views(
            &views,
            &TeamOpts {
                team: "EDM".into(),
                width: 100,
                height: 30,
            },
        )
        .unwrap();
        assert!(out.contains("Edm One"));
        assert!(out.contains("Edm Tre"));
        assert!(!out.contains("Col Two"));
        assert!(out.contains("type: lineup-card"));
        assert!(out.contains("EDM"));
    }

    #[test]
    fn l0_export_depth_emits_cross_team_table() {
        let repo = fixture_repo(&[
            (1, "EdmC1", "EDM", Position::Center, 109.0),
            (2, "EdmW1", "EDM", Position::LeftWing, 95.0),
            (3, "EdmD1", "EDM", Position::Defense, 80.0),
            (4, "ColC1", "COL", Position::Center, 85.0),
            (5, "ColD1", "COL", Position::Defense, 70.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_depth_from_views(
            &views,
            &DepthOpts {
                width: 100,
                height: 30,
            },
        )
        .unwrap();
        assert!(out.contains("type: depth-rankings"));
        assert!(out.contains("Cross-team line value rankings"));
        assert!(out.contains("| Rank | Player | Team | Pos | Own line | Avg other | Delta | Fit |"));
        assert!(out.matches("| EDM ").count() + out.matches("| COL ").count() >= 1);
    }

    #[test]
    fn l0_export_compare_finds_both_players_and_emits_diffs() {
        // Custom totals (50G/60A/110pts vs 30G/50A/80pts) — set explicitly.
        let mut repo = icelines_core::stats_repository::StatsRepository::new();
        for (id, name, g, a) in [
            (1u32, "Alpha A", 50u32, 60u32),
            (2u32, "Bravo B", 30u32, 50u32),
        ] {
            let identity = icelines_core::fixtures::identity(id)
                .name(name, &name.to_lowercase().replace(' ', "_"))
                .build();
            let totals = StatTotals {
                gp: 60,
                goals: g,
                assists: a,
                points: g + a,
                plus_minus: 0,
                pim: 0,
                shots: 0,
                shooting_pct: None,
                toi_per_game_sec: None,
                pp_goals: 0,
                pp_points: 0,
                sh_goals: 0,
                sh_points: 0,
                gwg: 0,
                ot_goals: 0,
                faceoff_win_pct: None,
                pace_score: Some(PaceScore {
                    pace_82: (g + a) as f64 / 60.0 * 82.0,
                    goals_per_82: g as f64 / 60.0 * 82.0,
                    raw_points: g + a,
                    gp: 60,
                }),
            };
            let team = if id == 1 { "EDM" } else { "COL" };
            let stint = TeamStint {
                team: TeamAbbr(team.into()),
                started: Some("2024-10-15".into()),
                ended: Some("2025-04-13".into()),
                gp: 60,
                goals: g,
                assists: a,
                points: g + a,
                goalie: None,
            };
            let stats = SeasonStatsBuilder::new(
                PlayerId(id),
                Season(20242025),
                SeasonType::Regular,
                Position::Center,
            )
            .with_totals(totals)
            .add_team_stint(stint)
            .build();
            repo.upsert_identity(identity).unwrap();
            repo.upsert_stats(stats).unwrap();
        }
        let views = fixture_views(&repo);
        let out = render_compare_from_views(
            &views,
            &CompareOpts {
                p1: "Alpha".into(),
                p2: "Bravo".into(),
                width: 100,
                height: 30,
            },
        )
        .unwrap();
        assert!(out.contains("type: compare"));
        assert!(out.contains("Alpha A vs Bravo B"));
        assert!(out.contains("| Stat | Alpha A | Bravo B | Diff |"));
        assert!(out.contains("| G |"));
        assert!(out.contains("| A |"));
        assert!(out.contains("| Pts |"));
        assert!(out.contains("Pts/82"));
        assert!(out.contains("+20") || out.contains("+30") || out.contains("+10"));
    }

    #[test]
    fn l0_export_compare_unknown_player_errors() {
        let repo = fixture_repo(&[(1, "Alpha", "EDM", Position::Center, 100.0)]);
        let views = fixture_views(&repo);
        let err = render_compare_from_views(
            &views,
            &CompareOpts {
                p1: "Alpha".into(),
                p2: "Nope".into(),
                width: 100,
                height: 30,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("'Nope'"));
    }

    #[test]
    fn l0_export_roster_groups_by_team_alpha() {
        let repo = fixture_repo(&[
            (1, "ZTeamPlayer", "ZZZ", Position::Center, 100.0),
            (2, "ATeamPlayer", "AAA", Position::Center, 50.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_roster_from_views(
            &views,
            &RosterOpts {
                pos: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();
        let i_aaa = out.find("AAA").unwrap();
        let i_zzz = out.find("ZZZ").unwrap();
        assert!(
            i_aaa < i_zzz,
            "rosters must be grouped by team alphabetically"
        );
        assert!(out.contains("type: roster"));
    }

    #[test]
    fn l0_export_roster_pos_filter_excludes_others() {
        let repo = fixture_repo(&[
            (1, "Cee", "EDM", Position::Center, 100.0),
            (2, "Dee", "EDM", Position::Defense, 90.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_roster_from_views(
            &views,
            &RosterOpts {
                pos: Some("D".into()),
                width: 100,
                height: 30,
            },
        )
        .unwrap();
        assert!(out.contains("Dee"));
        assert!(!out.contains("Cee"), "centers must be excluded for --pos D");
    }

    #[test]
    fn l0_export_front_matter_is_yaml_parseable() {
        let repo = fixture_repo(&[(1, "Solo", "EDM", Position::Center, 100.0)]);
        let views = fixture_views(&repo);
        let body = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: Some("C".into()),
                top: 5,
                sort: "pts-pace".into(),
                gp_min: None,
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap();
        let after_open = body
            .strip_prefix("---\n")
            .expect("front-matter opens with ---");
        let close = after_open
            .find("\n---\n")
            .expect("front-matter closes with ---");
        let yaml = &after_open[..close];
        for key in &[
            "type:",
            "title:",
            "generated_at:",
            "season:",
            "filters:",
            "proof:",
        ] {
            assert!(yaml.contains(key), "front-matter missing '{key}'");
        }
        assert!(yaml.contains("width: 80"));
        assert!(yaml.contains("height: 30"));
    }
}
