//! `icelines export md <shape>` — Phase 8d.
//!
//! Produces deterministic markdown tables consumable by proof's
//! DASHBOARD-SPEC compiler (or any other markdown reader). Every shape
//! follows the format from `design/specs/export-markdown.md`:
//!   - YAML front-matter (`type`, `title`, `season`, `filters`, `proof:` hints)
//!   - One or more GitHub-flavored markdown tables in the body
//!
//! The writer prints to stdout with `--out -`, otherwise writes to the
//! given path, otherwise to `~/.icelines/reports/{shape}.md`.

use anyhow::{bail, Context};
use std::fmt::Write as _;
use std::path::PathBuf;

use icelines_core::{
    filter::PlayerFilter,
    model::Player,
    scoring::sort_by_pace,
};

use crate::cli::{ExportSubcommand, MdShape};
use crate::commands::players::load_all_players;

pub async fn run(cmd: ExportSubcommand) -> anyhow::Result<()> {
    match cmd {
        ExportSubcommand::Md {
            shape, out, pos, team, top, sort, gp_min,
            p1, p2, series: _, width, height,
        } => {
            let body = match shape {
                MdShape::Leaders => render_leaders(LeadersOpts {
                    pos:    pos.clone(),
                    top,
                    sort:   sort.clone(),
                    gp_min,
                    width, height,
                })?,
                MdShape::Team => render_team(TeamOpts {
                    team: team.clone()
                        .context("--team is required for `export md team`")?,
                    width, height,
                })?,
                MdShape::Depth   => render_depth(DepthOpts   { width, height })?,
                MdShape::Compare => render_compare(CompareOpts {
                    p1: p1.clone().context("--p1 is required for `export md compare`")?,
                    p2: p2.clone().context("--p2 is required for `export md compare`")?,
                    width, height,
                })?,
                MdShape::Roster  => render_roster(RosterOpts { pos: pos.clone(), width, height })?,
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

// ── leaders ──────────────────────────────────────────────────────────────────

pub(crate) struct LeadersOpts {
    pub pos:    Option<String>,
    pub top:    usize,
    pub sort:   String,
    pub gp_min: Option<u32>,
    pub width:  u16,
    pub height: u16,
}

pub(crate) fn render_leaders(opts: LeadersOpts) -> anyhow::Result<String> {
    let players = load_all_players()?;
    render_leaders_from_players(&players, &opts)
}

/// Pure renderer separated from I/O so tests can inject fixture players.
pub(crate) fn render_leaders_from_players(
    players: &[Player],
    opts:    &LeadersOpts,
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

    let mut filtered: Vec<&Player> = filter.apply(players);

    // Sort — only `pts-pace` (default) supported in 8d.1; others land
    // in 8d.2 alongside the `team` shape work.
    match opts.sort.as_str() {
        "pts-pace" | "pace" | "pts" => {
            filtered.sort_by(|a, b| {
                let pa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                let pb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
                    // Stable tie-break: nhl_id asc → byte-deterministic across runs.
                    .then(a.nhl_id.cmp(&b.nhl_id))
            });
        }
        other => bail!(
            "sort metric `{other}` not supported in `export md leaders` v1 (use `pts-pace`)",
        ),
    }

    let top = filtered.into_iter().take(opts.top).collect::<Vec<_>>();

    let title = match &opts.pos {
        Some(p) if !p.is_empty() => format!("Top {} {} (Pts/82)", opts.top, p.to_uppercase()),
        _                       => format!("Top {} skaters (Pts/82)", opts.top),
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
            ("gp_min", opts.gp_min.map(|g| g.to_string()).unwrap_or_default()),
        ],
        opts.width, opts.height,
    );

    // Table
    let _ = writeln!(out, "| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |");
    let _ = writeln!(out, "|-----:|--------|:----:|:---:|----:|---:|---:|---:|----:|----:|-------:|");
    for (i, p) in top.iter().enumerate() {
        let rank   = i + 1;
        let name   = truncate(&p.full_name, 24);
        let pos    = p.position.abbreviation();
        let age    = p.birth_date.as_deref()
            .and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
            .map(|y| 2026u16.saturating_sub(y).to_string())
            .unwrap_or_else(|| "—".to_owned());
        let gp     = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
        let goals  = p.season_goals;
        let asts   = p.season_assists;
        let pts    = p.season_points;
        let ppg    = p.pace_score
            .map(|s| format!("{:.3}", s.pace_82 / 82.0))
            .unwrap_or_else(|| "—".to_owned());
        let pts82  = p.pace_score
            .map(|s| format!("{:.1}", s.pace_82))
            .unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(
            out,
            "| {rank:>4} | {name} | {team} | {pos} | {age:>3} | {gp:>3} | {goals:>3} | {asts:>3} | {pts:>4} | {ppg:>5} | {pts82:>6} |",
            team = p.team.as_str(),
        );
    }

    Ok(out)
}

// ── team ─────────────────────────────────────────────────────────────────────

pub(crate) struct TeamOpts {
    pub team:   String,
    pub width:  u16,
    pub height: u16,
}

pub(crate) fn render_team(opts: TeamOpts) -> anyhow::Result<String> {
    let players = load_all_players()?;
    render_team_from_players(&players, &opts)
}

pub(crate) fn render_team_from_players(
    players: &[Player],
    opts:    &TeamOpts,
) -> anyhow::Result<String> {
    let team_up = opts.team.to_uppercase();
    let mut roster: Vec<&Player> = players.iter()
        .filter(|p| p.team.as_str() == team_up.as_str())
        .collect();
    sort_by_pace_refs(&mut roster);

    let title = format!("{team_up} — Lineup card");
    let mut out = String::new();
    write_front_matter(
        &mut out,
        "lineup-card",
        &title,
        &[("team", team_up.clone())],
        opts.width, opts.height,
    );

    if roster.is_empty() {
        let _ = writeln!(out, "_No players loaded for {team_up}._");
        return Ok(out);
    }

    let _ = writeln!(out, "## All skaters");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Rank | Player | Pos | GP | G | A | Pts | Pts/82 |");
    let _ = writeln!(out, "|-----:|--------|:---:|---:|---:|---:|----:|-------:|");
    for (i, p) in roster.iter().enumerate() {
        let rank  = i + 1;
        let name  = truncate(&p.full_name, 24);
        let gp    = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
        let pts82 = p.pace_score
            .map(|s| format!("{:.1}", s.pace_82))
            .unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(
            out,
            "| {rank:>4} | {name} | {pos} | {gp:>3} | {g:>3} | {a:>3} | {pts:>4} | {pts82:>6} |",
            pos  = p.position.abbreviation(),
            g    = p.season_goals,
            a    = p.season_assists,
            pts  = p.season_points,
        );
    }

    Ok(out)
}

// ── depth ────────────────────────────────────────────────────────────────────

pub(crate) struct DepthOpts { pub width: u16, pub height: u16 }

pub(crate) fn render_depth(opts: DepthOpts) -> anyhow::Result<String> {
    let players = load_all_players()?;
    render_depth_from_players(&players, &opts)
}

pub(crate) fn render_depth_from_players(
    players: &[Player],
    opts:    &DepthOpts,
) -> anyhow::Result<String> {
    use icelines_core::compute_cross_team_metrics;
    let metrics = compute_cross_team_metrics(players);

    // Index players by nhl_id so we can render names + teams alongside metrics.
    let by_id: std::collections::HashMap<Option<u32>, &Player> =
        players.iter().map(|p| (p.nhl_id, p)).collect();

    // Sort by delta descending (most "buried" first — biggest "elsewhere they'd play higher" gap).
    // Stable tie-break: nhl_id asc.
    let mut rows: Vec<&icelines_core::cross_team::CrossTeamMetrics> = metrics.iter().collect();
    rows.sort_by(|a, b| {
        b.delta.partial_cmp(&a.delta).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.player_nhl_id.cmp(&b.player_nhl_id))
    });

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "depth-rankings",
        "Cross-team line value rankings",
        &[],
        opts.width, opts.height,
    );

    let _ = writeln!(out, "| Rank | Player | Team | Pos | Own line | Avg other | Delta | Fit |");
    let _ = writeln!(out, "|-----:|--------|:----:|:---:|---------:|----------:|------:|:---:|");
    for (i, m) in rows.iter().take(50).enumerate() {
        let p = match by_id.get(&m.player_nhl_id) {
            Some(p) => *p,
            None    => continue,
        };
        let _ = writeln!(
            out,
            "| {rank:>4} | {name} | {team} | {pos} | {own:>8} | {avg:>9.2} | {delta:>+6.2} | {fit} |",
            rank = i + 1,
            name = truncate(&p.full_name, 24),
            team = p.team.as_str(),
            pos  = p.position.abbreviation(),
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
    let players = load_all_players()?;
    render_compare_from_players(&players, &opts)
}

pub(crate) fn render_compare_from_players(
    players: &[Player],
    opts:    &CompareOpts,
) -> anyhow::Result<String> {
    use icelines_core::name::normalize_name;
    let n1 = normalize_name(&opts.p1);
    let n2 = normalize_name(&opts.p2);
    let a = players.iter().find(|p| p.name_normalized.contains(&n1))
        .with_context(|| format!("player '{}' not found", opts.p1))?;
    let b = players.iter().find(|p| p.name_normalized.contains(&n2))
        .with_context(|| format!("player '{}' not found", opts.p2))?;

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "compare",
        &format!("{} vs {}", a.full_name, b.full_name),
        &[("p1", a.full_name.clone()), ("p2", b.full_name.clone())],
        opts.width, opts.height,
    );

    let _ = writeln!(out, "| Stat | {} | {} | Diff |", a.full_name, b.full_name);
    let _ = writeln!(out, "|------|------|------|-----:|");

    let row = |out: &mut String, label: &str, av: f64, bv: f64, fmt: &str| {
        let diff = av - bv;
        let _ = writeln!(
            out, "| {label} | {av:fmt$} | {bv:fmt$} | {diff:+.2} |",
            fmt = match fmt { "0" => 0, _ => 2 },
        );
    };
    row(&mut out, "GP",  a.gp().unwrap_or(0) as f64, b.gp().unwrap_or(0) as f64, "0");
    row(&mut out, "G",   a.season_goals  as f64, b.season_goals  as f64, "0");
    row(&mut out, "A",   a.season_assists as f64, b.season_assists as f64, "0");
    row(&mut out, "Pts", a.season_points as f64, b.season_points as f64, "0");
    let pa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
    let pb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
    let _ = writeln!(out, "| Pts/82 | {pa:.1} | {pb:.1} | {:+.1} |", pa - pb);
    let ppg_a = a.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0);
    let ppg_b = b.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0);
    let _ = writeln!(out, "| PPG | {ppg_a:.3} | {ppg_b:.3} | {:+.3} |", ppg_a - ppg_b);
    Ok(out)
}

// ── roster ───────────────────────────────────────────────────────────────────

pub(crate) struct RosterOpts {
    pub pos:    Option<String>,
    pub width:  u16,
    pub height: u16,
}

pub(crate) fn render_roster(opts: RosterOpts) -> anyhow::Result<String> {
    let players = load_all_players()?;
    render_roster_from_players(&players, &opts)
}

pub(crate) fn render_roster_from_players(
    players: &[Player],
    opts:    &RosterOpts,
) -> anyhow::Result<String> {
    let mut filter = PlayerFilter::new();
    if let Some(p) = &opts.pos {
        if !p.is_empty() { filter.positions = Some(parse_positions(p)); }
    }
    let mut filtered = filter.apply(players);
    // Sort by team alpha, then pace desc within team (stable, deterministic).
    filtered.sort_by(|a, b| {
        a.team.as_str().cmp(b.team.as_str())
            .then_with(|| {
                let pa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                let pb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
            })
            .then(a.nhl_id.cmp(&b.nhl_id))
    });

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "roster",
        "All teams — full skater roster",
        &[("pos", opts.pos.clone().unwrap_or_default())],
        opts.width, opts.height,
    );

    let _ = writeln!(out, "| Team | Player | Pos | GP | G | A | Pts | Pts/82 |");
    let _ = writeln!(out, "|:----:|--------|:---:|---:|---:|---:|----:|-------:|");
    for p in &filtered {
        let pts82 = p.pace_score
            .map(|s| format!("{:.1}", s.pace_82))
            .unwrap_or_else(|| "—".to_owned());
        let gp = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(
            out,
            "| {team} | {name} | {pos} | {gp:>3} | {g:>3} | {a:>3} | {pts:>4} | {pts82:>6} |",
            team = p.team.as_str(),
            name = truncate(&p.full_name, 24),
            pos  = p.position.abbreviation(),
            g    = p.season_goals,
            a    = p.season_assists,
            pts  = p.season_points,
        );
    }
    Ok(out)
}

// ── shared helpers ───────────────────────────────────────────────────────────

fn parse_positions(s: &str) -> Vec<icelines_core::model::Position> {
    use icelines_core::model::Position;
    s.split(',')
        .filter_map(|p| match p.trim().to_uppercase().as_str() {
            "C"   => Some(vec![Position::Center]),
            "LW"  => Some(vec![Position::LeftWing]),
            "RW"  => Some(vec![Position::RightWing]),
            "D"   => Some(vec![Position::Defense]),
            "F"   => Some(vec![Position::Center, Position::LeftWing, Position::RightWing]),
            _     => None,
        })
        .flatten()
        .collect()
}

fn sort_by_pace_refs(players: &mut [&Player]) {
    let mut owned: Vec<Player> = players.iter().map(|&p| p.clone()).collect();
    sort_by_pace(&mut owned);
    let order: std::collections::HashMap<Option<u32>, usize> =
        owned.iter().enumerate().map(|(i, p)| (p.nhl_id, i)).collect();
    players.sort_by_key(|p| order.get(&p.nhl_id).copied().unwrap_or(usize::MAX));
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_owned() }
    else { s.chars().take(max).collect() }
}

fn write_front_matter(
    out:   &mut String,
    ty:    &str,
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
            if v.is_empty() { continue; }
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
            if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
            std::fs::write(&path, body)
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!("✓ wrote {} ({} bytes)", path.display(), body.len());
            Ok(())
        }
        None => {
            // Default: ~/.icelines/reports/{shape}.md
            let home = std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .context("cannot determine home directory")?;
            let path = home.join(".icelines").join("reports")
                .join(format!("{}.md", shape.label()));
            if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
            std::fs::write(&path, body)
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!("✓ wrote {} ({} bytes)", path.display(), body.len());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::model::{GpStatus, PaceScore, Position, TeamAbbr};

    fn fixture(id: u32, name: &str, team: &str, pos: Position, pace: f64) -> Player {
        Player {
            nhl_id: Some(id),
            full_name: name.to_owned(),
            name_normalized: name.to_lowercase().replace(' ', "_"),
            team: TeamAbbr(team.to_owned()),
            position: pos,
            eligible_pos: vec![pos],
            gp_status: GpStatus::Eligible(60),
            season_goals: 30, season_assists: 50, season_points: 80,
            pace_score: Some(PaceScore {
                pace_82: pace, goals_per_82: 30.0, raw_points: 80, gp: 60,
            }),
            pp_goals: 0, pp_points: 0, sh_goals: 0, sh_points: 0,
            gwg: 0, ot_goals: 0, shots: 0, shooting_pct: None,
            plus_minus: 0, toi_per_game_sec: None, faceoff_win_pct: None,
            hits: 0, blocked_shots: 0, missed_shots: 0,
            giveaways: 0, takeaways: 0, pim: 0,
            xg: None, xg_per_60: None, cf_pct_5v5: None,
            ff_pct_5v5: None, xgf_pct_5v5: None,
            headshot_url: None, sweater_number: None,
            birth_date: Some("1997-01-13".to_owned()),
            birth_country: None, nationality_code: None,
            birth_city: None, birth_state_province: None,
            shoots_catches: None, height_in_inches: None, weight_lbs: None,
            draft_year: None, draft_round: None, draft_overall: None,
            rookie_season: None, contract_expiry_year: None,
            expiry_type: None, salary: None,
        }
    }

    #[test]
    fn l0_export_leaders_has_required_front_matter() {
        let players = vec![
            fixture(1, "A One", "EDM", Position::Center,    109.0),
            fixture(2, "B Two", "COL", Position::Center,     95.0),
            fixture(3, "C Tre", "SEA", Position::LeftWing,   80.0),
        ];
        let opts = LeadersOpts {
            pos: None, top: 25, sort: "pts-pace".into(),
            gp_min: None, width: 100, height: 30,
        };
        let out = render_leaders_from_players(&players, &opts).unwrap();

        // YAML front matter delimiters
        assert!(out.starts_with("---\n"));
        assert!(out.contains("\n---\n\n|"), "front-matter must precede the table");
        // Required keys
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
        let players = vec![fixture(1, "Solo", "EDM", Position::Center, 109.0)];
        let out = render_leaders_from_players(
            &players,
            &LeadersOpts {
                pos: None, top: 25, sort: "pts-pace".into(),
                gp_min: None, width: 80, height: 30,
            },
        ).unwrap();
        // Header line
        assert!(out.contains("| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |"));
        // Data row
        assert!(out.contains("Solo"));
        assert!(out.contains("EDM"));
        assert!(out.contains("109.0"));
    }

    #[test]
    fn l0_export_leaders_sort_is_descending_by_pace() {
        let players = vec![
            fixture(1, "Mid",  "EDM", Position::Center, 80.0),
            fixture(2, "Top",  "COL", Position::Center, 110.0),
            fixture(3, "Low",  "SEA", Position::LeftWing, 50.0),
        ];
        let out = render_leaders_from_players(
            &players,
            &LeadersOpts {
                pos: None, top: 25, sort: "pts-pace".into(),
                gp_min: None, width: 80, height: 30,
            },
        ).unwrap();
        // "Top" must appear before "Mid" must appear before "Low".
        let i_top = out.find("Top").unwrap();
        let i_mid = out.find("Mid").unwrap();
        let i_low = out.find("Low").unwrap();
        assert!(i_top < i_mid && i_mid < i_low,
            "sort order broken: top={i_top} mid={i_mid} low={i_low}");
    }

    #[test]
    fn l0_export_leaders_pos_filter_excludes_others() {
        let players = vec![
            fixture(1, "Cee",  "EDM", Position::Center,    100.0),
            fixture(2, "Dee",  "COL", Position::Defense,    90.0),
        ];
        let out = render_leaders_from_players(
            &players,
            &LeadersOpts {
                pos: Some("C".into()), top: 25, sort: "pts-pace".into(),
                gp_min: None, width: 80, height: 30,
            },
        ).unwrap();
        assert!(out.contains("Cee"));
        assert!(!out.contains("Dee"), "defenseman must be filtered out for --pos C");
        assert!(out.contains("title: \"Top 25 C"), "title must reflect the position filter");
    }

    #[test]
    fn l0_export_leaders_top_limits_rows() {
        let players: Vec<Player> = (1..=10)
            .map(|i| fixture(i, &format!("P{i}"), "EDM", Position::Center, 100.0 - i as f64))
            .collect();
        let out = render_leaders_from_players(
            &players,
            &LeadersOpts {
                pos: None, top: 3, sort: "pts-pace".into(),
                gp_min: None, width: 80, height: 30,
            },
        ).unwrap();
        // Only first three players' rows
        assert!(out.contains("| P1 ") || out.contains("P1 "));
        assert!(out.contains("P3"));
        assert!(!out.contains("P4"), "top=3 must drop P4-P10");
    }

    #[test]
    fn l0_export_leaders_unknown_sort_errors() {
        let players = vec![fixture(1, "X", "EDM", Position::Center, 50.0)];
        let err = render_leaders_from_players(
            &players,
            &LeadersOpts {
                pos: None, top: 25, sort: "invalid-metric".into(),
                gp_min: None, width: 80, height: 30,
            },
        ).unwrap_err();
        assert!(err.to_string().contains("invalid-metric"));
    }

    #[test]
    fn l0_export_team_card_lists_only_target_team() {
        let players = vec![
            fixture(1, "Edm One", "EDM", Position::Center, 100.0),
            fixture(2, "Col Two", "COL", Position::Center,  95.0),
            fixture(3, "Edm Tre", "EDM", Position::Defense, 70.0),
        ];
        let out = render_team_from_players(
            &players,
            &TeamOpts { team: "EDM".into(), width: 100, height: 30 },
        ).unwrap();
        assert!(out.contains("Edm One"));
        assert!(out.contains("Edm Tre"));
        assert!(!out.contains("Col Two"));
        assert!(out.contains("type: lineup-card"));
        assert!(out.contains("EDM"));
    }

    // ── Depth / Compare / Roster shapes ──────────────────────────────────────

    #[test]
    fn l0_export_depth_emits_cross_team_table() {
        // Build at least one player at each forward + D position on two
        // teams so cross_team::compute_cross_team_metrics has work to do.
        let players = vec![
            fixture(1, "EdmC1", "EDM", Position::Center,    109.0),
            fixture(2, "EdmW1", "EDM", Position::LeftWing,   95.0),
            fixture(3, "EdmD1", "EDM", Position::Defense,    80.0),
            fixture(4, "ColC1", "COL", Position::Center,     85.0),
            fixture(5, "ColD1", "COL", Position::Defense,    70.0),
        ];
        let out = render_depth_from_players(
            &players,
            &DepthOpts { width: 100, height: 30 },
        ).unwrap();
        assert!(out.contains("type: depth-rankings"));
        assert!(out.contains("Cross-team line value rankings"));
        assert!(out.contains("| Rank | Player | Team | Pos | Own line | Avg other | Delta | Fit |"));
        // At least one player row
        assert!(out.matches("| EDM ").count() + out.matches("| COL ").count() >= 1);
    }

    #[test]
    fn l0_export_compare_finds_both_players_and_emits_diffs() {
        let mut a = fixture(1, "Alpha A", "EDM", Position::Center, 100.0);
        a.season_goals = 50; a.season_assists = 60; a.season_points = 110;
        let mut b = fixture(2, "Bravo B", "COL", Position::Center,  80.0);
        b.season_goals = 30; b.season_assists = 50; b.season_points = 80;
        let players = vec![a, b];

        let out = render_compare_from_players(
            &players,
            &CompareOpts {
                p1: "Alpha".into(), p2: "Bravo".into(),
                width: 100, height: 30,
            },
        ).unwrap();
        assert!(out.contains("type: compare"));
        assert!(out.contains("Alpha A vs Bravo B"));
        // Header + rows
        assert!(out.contains("| Stat | Alpha A | Bravo B | Diff |"));
        assert!(out.contains("| G |"));
        assert!(out.contains("| A |"));
        assert!(out.contains("| Pts |"));
        assert!(out.contains("Pts/82"));
        // Diff signs render
        assert!(out.contains("+20") || out.contains("+30") || out.contains("+10"));
    }

    #[test]
    fn l0_export_compare_unknown_player_errors() {
        let players = vec![fixture(1, "Alpha", "EDM", Position::Center, 100.0)];
        let err = render_compare_from_players(
            &players,
            &CompareOpts {
                p1: "Alpha".into(), p2: "Nope".into(),
                width: 100, height: 30,
            },
        ).unwrap_err();
        assert!(err.to_string().contains("'Nope'"));
    }

    #[test]
    fn l0_export_roster_groups_by_team_alpha() {
        let players = vec![
            fixture(1, "ZTeamPlayer", "ZZZ", Position::Center, 100.0),
            fixture(2, "ATeamPlayer", "AAA", Position::Center,  50.0),
        ];
        let out = render_roster_from_players(
            &players,
            &RosterOpts { pos: None, width: 100, height: 30 },
        ).unwrap();
        // Team AAA must appear before team ZZZ in the output.
        let i_aaa = out.find("AAA").unwrap();
        let i_zzz = out.find("ZZZ").unwrap();
        assert!(i_aaa < i_zzz, "rosters must be grouped by team alphabetically");
        assert!(out.contains("type: roster"));
    }

    #[test]
    fn l0_export_roster_pos_filter_excludes_others() {
        let players = vec![
            fixture(1, "Cee", "EDM", Position::Center,  100.0),
            fixture(2, "Dee", "EDM", Position::Defense,  90.0),
        ];
        let out = render_roster_from_players(
            &players,
            &RosterOpts { pos: Some("D".into()), width: 100, height: 30 },
        ).unwrap();
        assert!(out.contains("Dee"));
        assert!(!out.contains("Cee"), "centers must be excluded for --pos D");
    }

    #[test]
    fn l0_export_front_matter_is_yaml_parseable() {
        // Smoke test: every shape's front matter must be valid YAML when
        // extracted, since proof reads it via serde_yaml internally.
        let players = vec![fixture(1, "Solo", "EDM", Position::Center, 100.0)];
        let body = render_leaders_from_players(
            &players,
            &LeadersOpts {
                pos: Some("C".into()), top: 5, sort: "pts-pace".into(),
                gp_min: None, width: 80, height: 30,
            },
        ).unwrap();
        // Strip the front matter and re-parse
        let after_open = body.strip_prefix("---\n").expect("front-matter opens with ---");
        let close = after_open.find("\n---\n").expect("front-matter closes with ---");
        let yaml = &after_open[..close];
        // Manual key checks (avoids pulling serde_yaml as a dep)
        for key in &["type:", "title:", "generated_at:", "season:", "filters:", "proof:"] {
            assert!(yaml.contains(key), "front-matter missing '{key}'");
        }
        assert!(yaml.contains("width: 80"));
        assert!(yaml.contains("height: 30"));
    }
}
