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

use icelines_core::view_model::context::RecoveryActionKind;
use icelines_core::{
    cross_team::{compute_all_views, ScoringMode},
    filter::PlayerFilter,
    scoring::sort_views_by_pace,
    stats_repository::PlayerView,
    view_model::{poach_report_from_board, PoachBoardView, PoachQuery, PoachReportView},
    Completeness, DepthLeagueView, EmptyKind, EmptyState, LeaderKind, LeadersView,
    PlayoffsSeriesRow, PlayoffsView, RecoveryAction, ScheduleRecord, SortDirection, SortKey,
    SortState, SourceKind, SourceState, TeamAbbr, TeamDepthView, TeamSeasonGameRow,
    TeamSeasonVenue, TeamSeasonView, ViewContext, ViewWarning, ViewWindow, WarningKind,
};
use icelines_fetch::nhl_api::NhlApiClient;

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
            season,
            season_type,
            sort,
            gp_min,
            filters,
            columns,
            p1,
            p2,
            series,
            width,
            height,
        } => {
            let body = match shape {
                MdShape::Leaders => render_leaders(
                    LeadersOpts {
                        pos: pos.clone(),
                        top,
                        sort: sort.clone(),
                        gp_min,
                        filters: filters.clone(),
                        columns: columns.clone(),
                        width,
                        height,
                    },
                    season.as_deref(),
                    season_type.to_core(),
                )?,
                MdShape::Team => render_team(TeamOpts {
                    team: team
                        .clone()
                        .context("--team is required for `export md team`")?,
                    width,
                    height,
                })?,
                MdShape::TeamSeason => {
                    render_team_season(TeamSeasonOpts {
                        team: team
                            .clone()
                            .context("--team is required for `export md team-season`")?,
                        width,
                        height,
                    })
                    .await?
                }
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
                MdShape::Series => render_series(SeriesOpts {
                    series: series.clone(),
                    width,
                    height,
                })?,
                MdShape::Fantasy => render_fantasy(FantasyOpts { top, width, height })?,
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

fn load_views_for(
    season: Option<&str>,
    season_type: icelines_core::season_stats::SeasonType,
) -> anyhow::Result<(
    icelines_fetch::stats_loader::LoadOutcome,
    icelines_core::model::Season,
    icelines_core::season_stats::SeasonType,
)> {
    load_repo_for_season(season, Some(season_type))
}

// ── leaders ──────────────────────────────────────────────────────────────────

pub(crate) struct LeadersOpts {
    pub pos: Option<String>,
    pub top: usize,
    pub sort: String,
    pub gp_min: Option<u32>,
    pub filters: Vec<String>,
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

pub(crate) fn render_leaders(
    opts: LeadersOpts,
    season: Option<&str>,
    season_type: icelines_core::season_stats::SeasonType,
) -> anyhow::Result<String> {
    let (outcome, season, season_type) = load_views_for(season, season_type)?;
    let views: Vec<PlayerView<'_>> = outcome.repo.skaters(season, season_type).collect();
    render_leaders_from_views_with_window(&views, &opts, season, season_type)
}

/// Pure renderer separated from I/O so tests can inject fixture views.
#[cfg(test)]
pub(crate) fn render_leaders_from_views(
    views: &[PlayerView<'_>],
    opts: &LeadersOpts,
) -> anyhow::Result<String> {
    render_leaders_from_views_with_window(
        views,
        opts,
        icelines_core::model::Season(icelines_core::CURRENT_SEASON),
        icelines_core::season_stats::SeasonType::Regular,
    )
}

fn render_leaders_from_views_with_window(
    views: &[PlayerView<'_>],
    opts: &LeadersOpts,
    season: icelines_core::model::Season,
    season_type: icelines_core::season_stats::SeasonType,
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
    let (bio_constraints, stat_residue) =
        crate::commands::query::extract_bio_for_cli(&opts.filters);
    let (new_pipeline_plans, legacy_residue) =
        crate::commands::query::partition_filter_dispatch(&stat_residue)?;
    for raw in &legacy_residue {
        let expr = icelines_core::stats_catalog::parse_filter_expr(raw)
            .with_context(|| format!("--filter {raw:?}"))?;
        match expr.as_atom() {
            Some(atom) => filter.stat_filters.push(*atom),
            None => filter.expr_filters.push(expr),
        }
    }
    filter.normalize_stat_filters();

    let mut filtered: Vec<PlayerView<'_>> = filter.apply_views(views.iter().copied());
    if bio_constraints.is_active() {
        filtered.retain(|v| bio_constraints.matches(v, season.0));
    }
    if !new_pipeline_plans.is_empty() {
        let (provider, clock) = crate::commands::query::build_cli_eval_ctx(season.0)?;
        let ctx = icelines_query::EvalCtx::from_clock(
            &provider,
            icelines_query::StrictMode::Off,
            false,
            &clock,
            season.0,
        );
        for plan in &new_pipeline_plans {
            filtered.retain(|v| plan.root.matches(v, &ctx));
        }
    }

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

    let total = filtered.len();
    let top = filtered.into_iter().take(opts.top).collect::<Vec<_>>();
    let returned = top.len();
    let mut context = ViewContext::new(ViewWindow::new(season, season_type));
    context
        .source_state
        .push(SourceState::complete(SourceKind::Roster));
    let mut leaders_view =
        LeadersView::from_player_views(context, LeaderKind::Skaters, top.iter().copied());
    leaders_view.sort = Some(SortState {
        key: SortKey::from("pace_82"),
        label: "Pts/82".to_string(),
        direction: SortDirection::Desc,
    });
    apply_leaders_warning_state(&mut leaders_view, opts.pos.as_deref());

    let title = match &opts.pos {
        Some(p) if !p.is_empty() => format!("Top {} {} (Pts/82)", opts.top, p.to_uppercase()),
        _ => format!("Top {} skaters (Pts/82)", opts.top),
    };

    let mut out = String::new();
    let result_metadata = leaders_result_metadata(total, returned, opts);
    write_front_matter_with_context(
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
            ("filter", opts.filters.join(";")),
        ],
        (opts.width, opts.height),
        FrontMatterMetadata {
            context: Some(&leaders_view.context),
            result: Some(&result_metadata),
            leaders_state: Some(&leaders_view),
        },
    );
    write_leaders_context_summary(&mut out, &leaders_view);
    write_leaders_result_summary(&mut out, &result_metadata);
    write_leaders_warning_empty_summary(&mut out, &leaders_view);

    // Phase Lindsay L.5.4 — `--columns` overrides the canonical column
    // set. None preserves the v1 hardcoded shape (Rank Player Team Pos
    // Age GP G A Pts PPG Pts/82) byte-identically.
    if let Some(cols_spec) = opts.columns.as_deref() {
        let stat_cols = parse_columns_list(cols_spec)?;
        write_leaders_view_table_with_columns(&mut out, &leaders_view, &stat_cols);
    } else {
        write_leaders_view_table(&mut out, &leaders_view);
    }

    Ok(out)
}

fn write_leaders_context_summary(out: &mut String, view: &LeadersView) {
    let window = view.context.window;
    let _ = writeln!(out, "## Context\n");
    let _ = writeln!(out, "- Season: {}", window.season.0);
    let _ = writeln!(out, "- Season type: {}", window.season_type.label());
    let _ = writeln!(out, "- Sources:");
    for source in &view.context.source_state {
        let _ = writeln!(
            out,
            "  - {}: {}",
            source_kind_label(source.source),
            completeness_label(source.state)
        );
    }
    let _ = writeln!(out);
}

struct LeadersResultMetadata<'a> {
    total: usize,
    returned: usize,
    top: usize,
    sort: &'a str,
    active_filters: String,
}

#[derive(Default)]
struct FrontMatterMetadata<'a> {
    context: Option<&'a ViewContext>,
    result: Option<&'a LeadersResultMetadata<'a>>,
    leaders_state: Option<&'a LeadersView>,
}

fn leaders_result_metadata<'a>(
    total: usize,
    returned: usize,
    opts: &'a LeadersOpts,
) -> LeadersResultMetadata<'a> {
    LeadersResultMetadata {
        total,
        returned,
        top: opts.top,
        sort: &opts.sort,
        active_filters: leaders_active_filters_label(opts),
    }
}

fn write_leaders_result_summary(out: &mut String, result: &LeadersResultMetadata<'_>) {
    let _ = writeln!(out, "## Result\n");
    let _ = writeln!(out, "- Total: {}", result.total);
    let _ = writeln!(out, "- Returned: {}", result.returned);
    let _ = writeln!(out, "- Top: {}", result.top);
    let _ = writeln!(out, "- Sort: {}", result.sort);
    let _ = writeln!(out, "- Active filters: {}", result.active_filters);
    let _ = writeln!(out);
}

fn write_leaders_warning_empty_summary(out: &mut String, view: &LeadersView) {
    if !view.warnings.is_empty() {
        let _ = writeln!(out, "## Warnings\n");
        for warning in &view.warnings {
            let _ = writeln!(
                out,
                "- {}: {}",
                warning_kind_label(warning.kind),
                warning.message
            );
            for recovery in &warning.recovery {
                let _ = writeln!(out, "  - Recovery: {}", recovery_action_label(recovery));
            }
        }
        let _ = writeln!(out);
    }

    if let Some(empty) = &view.empty_state {
        let _ = writeln!(out, "## Empty State\n");
        let _ = writeln!(out, "- Kind: {}", empty_kind_label(empty.kind));
        let _ = writeln!(out, "- Title: {}", empty.title);
        if let Some(detail) = &empty.detail {
            let _ = writeln!(out, "- Detail: {detail}");
        }
        for recovery in &empty.recovery {
            let _ = writeln!(out, "- Recovery: {}", recovery_action_label(recovery));
        }
        let _ = writeln!(out);
    }
}

fn leaders_active_filters_label(opts: &LeadersOpts) -> String {
    let mut filters = Vec::new();
    if let Some(pos) = opts
        .pos
        .as_deref()
        .map(str::trim)
        .filter(|pos| !pos.is_empty())
    {
        filters.push(format!("position={}", pos.to_uppercase()));
    }
    if let Some(gp_min) = opts.gp_min {
        filters.push(format!("gp>={gp_min}"));
    }
    filters.extend(opts.filters.iter().filter_map(|filter| {
        let filter = filter.trim();
        (!filter.is_empty()).then(|| filter.to_owned())
    }));
    if filters.is_empty() {
        "-".to_owned()
    } else {
        filters.join("; ")
    }
}

fn apply_leaders_warning_state(view: &mut LeadersView, pos: Option<&str>) {
    if !matches!(
        pos.map(|value| value.trim().to_ascii_uppercase()),
        Some(value) if value == "G"
    ) {
        return;
    }

    let recovery = vec![RecoveryAction {
        label: "Open goalie leaders".to_owned(),
        action: RecoveryActionKind::OpenRoute {
            route: "goalies".to_owned(),
        },
    }];
    view.warnings.push(ViewWarning {
        kind: WarningKind::UnsupportedFilter,
        source: None,
        message: "The leaders surface is skater-only; use goalies for goalie leaders.".to_owned(),
        recovery: recovery.clone(),
    });
    if view.rows.is_empty() {
        view.empty_state = Some(EmptyState {
            kind: EmptyKind::NoRows,
            title: "No skater leaders".to_owned(),
            detail: Some(
                "The leaders surface is skater-only; use the goalies surface for goalie leaders."
                    .to_owned(),
            ),
            recovery,
        });
    }
}

fn empty_kind_label(kind: EmptyKind) -> &'static str {
    match kind {
        EmptyKind::NoRows => "no_rows",
        EmptyKind::NoMatch => "no_match",
        EmptyKind::MissingSource => "missing_source",
        EmptyKind::UnsupportedWindow => "unsupported_window",
        EmptyKind::BadFilter => "bad_filter",
        EmptyKind::NotFound => "not_found",
    }
}

fn warning_kind_label(kind: WarningKind) -> &'static str {
    match kind {
        WarningKind::PartialSource => "partial_source",
        WarningKind::StaleSource => "stale_source",
        WarningKind::MissingSource => "missing_source",
        WarningKind::EstimatedDeployment => "estimated_deployment",
        WarningKind::DuplicateName => "duplicate_name",
        WarningKind::UnsupportedFilter => "unsupported_filter",
        WarningKind::RendererProjection => "renderer_projection",
    }
}

fn recovery_action_label(recovery: &RecoveryAction) -> String {
    match &recovery.action {
        RecoveryActionKind::ClearFilter { key } => match key {
            Some(key) => format!("{} -> clear filter {}", recovery.label, key.0),
            None => format!("{} -> clear filters", recovery.label),
        },
        RecoveryActionKind::ChangeWindow { window } => format!(
            "{} -> {} {}",
            recovery.label,
            window.season.0,
            window.season_type.label()
        ),
        RecoveryActionKind::InstallData { source } => {
            format!(
                "{} -> install {}",
                recovery.label,
                source_kind_label(*source)
            )
        }
        RecoveryActionKind::RefreshSource { source } => {
            format!(
                "{} -> refresh {}",
                recovery.label,
                source_kind_label(*source)
            )
        }
        RecoveryActionKind::OpenRoute { route } => {
            format!("{} -> /{}", recovery.label, route.trim_start_matches('/'))
        }
    }
}

fn leader_metric_i64(row: &icelines_core::LeaderRow, key: &str) -> Option<i64> {
    row.secondary.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                icelines_core::MetricValue::Integer(value) => Some(value),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn leader_primary_f64(row: &icelines_core::LeaderRow) -> Option<f64> {
    match row.primary.value {
        icelines_core::MetricValue::Decimal(value) => Some(value),
        icelines_core::MetricValue::Integer(value) => Some(value as f64),
        _ => None,
    }
}

fn write_leaders_view_table(out: &mut String, view: &LeadersView) {
    let _ = writeln!(
        out,
        "| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |"
    );
    let _ = writeln!(
        out,
        "|-----:|--------|:----:|:---:|----:|---:|---:|---:|----:|----:|-------:|"
    );
    for row in &view.rows {
        let rank = row.rank;
        let name = truncate(&row.display_name, 24);
        let pos = row.position.abbreviation();
        let age = leader_metric_i64(row, "age")
            .map(|value| value.to_string())
            .unwrap_or_else(|| "—".to_owned());
        let gp = leader_metric_i64(row, "gp").unwrap_or(0);
        let goals = leader_metric_i64(row, "goals").unwrap_or(0);
        let asts = leader_metric_i64(row, "assists").unwrap_or(0);
        let pts = leader_metric_i64(row, "points").unwrap_or(0);
        let ppg = leader_primary_f64(row)
            .map(|p| format!("{:.3}", p / 82.0))
            .unwrap_or_else(|| "—".to_owned());
        let pts82 = leader_primary_f64(row)
            .map(|p| format!("{p:.1}"))
            .unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(
            out,
            "| {rank:>4} | {name} | {team} | {pos} | {age:>3} | {gp:>3} | {goals:>3} | {asts:>3} | {pts:>4} | {ppg:>5} | {pts82:>6} |",
            team = row.team.0,
        );
    }
}

/// Phase Lindsay L.5.4 — render leaders table with custom StatId columns.
/// Headers come from `StatId::short_label()`; cells route through the
/// same per-StatUnit formatting (Count → integer, Pct → `XX.X%`,
/// Per60/Rate → `X.XX`, Seconds → `M:SS`, Inverted → `X.XX`).
#[allow(dead_code)]
fn write_leaders_table_with_columns(
    out: &mut String,
    top: &[PlayerView<'_>],
    stat_cols: &[icelines_core::stats_catalog::StatId],
) {
    use icelines_core::stats_catalog::StatUnit;
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

fn write_leaders_view_table_with_columns(
    out: &mut String,
    view: &LeadersView,
    stat_cols: &[icelines_core::stats_catalog::StatId],
) {
    out.push_str("| Rank | Player | Team | Pos");
    for sid in stat_cols {
        let _ = write!(out, " | {}", sid.short_label());
    }
    out.push_str(" |\n");

    out.push_str("|-----:|--------|:----:|:---:");
    for _ in stat_cols {
        out.push_str("|----:");
    }
    out.push_str("|\n");

    for row in &view.rows {
        let rank = row.rank;
        let name = truncate(&row.display_name, 24);
        let pos = row.position.abbreviation();
        let _ = write!(
            out,
            "| {rank:>4} | {name} | {team} | {pos}",
            team = row.team.0,
        );
        for sid in stat_cols {
            let cell = row
                .catalog_metrics
                .iter()
                .find(|metric| metric.key.0 == sid.cli_key())
                .map(render_metric_cell)
                .unwrap_or_else(|| "—".to_owned());
            let _ = write!(out, " | {cell}");
        }
        out.push_str(" |\n");
    }
}

fn render_metric_cell(cell: &icelines_core::MetricCell) -> String {
    match &cell.value {
        icelines_core::MetricValue::Missing => "—".to_owned(),
        icelines_core::MetricValue::Text(value) => value.clone(),
        icelines_core::MetricValue::Integer(value) => {
            if cell.unit == icelines_core::MetricUnit::Seconds {
                let seconds = (*value).max(0) as u64;
                if seconds < 3600 {
                    format!("{}:{:02}", seconds / 60, seconds % 60)
                } else {
                    format!("{}m", seconds / 60)
                }
            } else {
                value.to_string()
            }
        }
        icelines_core::MetricValue::Decimal(value) => match cell.precision {
            icelines_core::ValuePrecision::PercentOneDecimal => format!("{:.1}%", value * 100.0),
            icelines_core::ValuePrecision::OneDecimal => format!("{value:.1}"),
            icelines_core::ValuePrecision::ThreeDecimals => format!("{value:.3}"),
            _ => format!("{value:.2}"),
        },
    }
}

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

    let team_view = TeamDepthView::from_player_views(
        TeamAbbr(team_up.clone()),
        icelines_core::model::Season(icelines_core::CURRENT_SEASON),
        icelines_core::season_stats::SeasonType::Regular,
        &roster,
    );
    write_team_depth_view_markdown(&mut out, &team_view);

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

// ── team-season ───────────────────────────────────────────────────────────────

pub(crate) struct TeamSeasonOpts {
    pub team: String,
    pub width: u16,
    pub height: u16,
}

pub(crate) async fn render_team_season(opts: TeamSeasonOpts) -> anyhow::Result<String> {
    let team_abbr = TeamAbbr::parse(&opts.team)
        .map_err(|_| anyhow::anyhow!("'{}' is not a valid NHL team abbreviation", opts.team))?;
    let season = icelines_core::model::Season(icelines_core::CURRENT_SEASON);
    let season_type = icelines_core::season_stats::SeasonType::Regular;
    let season_str = icelines_core::CURRENT_SEASON_STR.to_string();
    let client = NhlApiClient::production();
    let standings = client
        .fetch_standings_now()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.to_team_standing_input())
        .collect();
    let games = client
        .fetch_team_season_schedule(&team_abbr.0, &season_str)
        .await
        .with_context(|| format!("fetching {team_abbr} season schedule for {season_str}"))?
        .into_iter()
        .map(crate::commands::team::scheduled_game_input)
        .collect();
    let view = TeamSeasonView::from_games_and_standings(
        ViewContext::new(ViewWindow::new(season, season_type)),
        season_str,
        team_abbr.0,
        games,
        standings,
    );
    render_team_season_from_view(&view, &opts)
}

pub(crate) fn render_team_season_from_view(
    view: &TeamSeasonView,
    opts: &TeamSeasonOpts,
) -> anyhow::Result<String> {
    let mut out = String::new();
    write_front_matter(
        &mut out,
        "team-season",
        &format!("{} team season - {}", view.team, view.season_pretty),
        &[("team", view.team.clone()), ("season", view.season.clone())],
        opts.width,
        opts.height,
    );

    let headline = &view.headline;
    let _ = writeln!(out, "## Summary\n");
    let _ = writeln!(out, "- Team: {}", view.team);
    let _ = writeln!(out, "- Season: {}", view.season_pretty);
    let _ = writeln!(out, "- Record: {}", schedule_record_label(headline.record));
    let _ = writeln!(out, "- Points: {}", headline.points);
    let _ = writeln!(
        out,
        "- Points percentage: {:.3}",
        headline.points_percentage
    );
    let _ = writeln!(
        out,
        "- Goals: {} for / {} against ({})",
        headline.goals_for,
        headline.goals_against,
        signed_i32(headline.goal_differential)
    );
    if let Some(standings) = &view.standings {
        let _ = writeln!(
            out,
            "- Standings: {} pts, {:.3} pts%, {}",
            standings.points, standings.points_percentage, standings.playoff_position_label
        );
        if let Some(above) = standings.points_above_cutline {
            let _ = writeln!(out, "- Playoff cutline: {above} points above cutline");
        } else if let Some(behind) = standings.points_behind_cutline {
            let _ = writeln!(out, "- Playoff cutline: {behind} points behind cutline");
        }
    }
    let _ = writeln!(
        out,
        "- Remaining: {} games ({} home, {} away)",
        view.remaining.games, view.remaining.home, view.remaining.away
    );
    let _ = writeln!(
        out,
        "- Recent form: last 5 {}, last 10 {} ({})",
        schedule_record_label(view.form.last_5),
        schedule_record_label(view.form.last_10),
        signed_i32(view.form.last_10_goal_differential)
    );
    if !view.remaining.next_opponents.is_empty() {
        let _ = writeln!(
            out,
            "- Next opponents: {}",
            view.remaining.next_opponents.join(", ")
        );
    }
    let _ = writeln!(out);

    write_team_season_source_state(&mut out, view);
    write_team_season_splits(&mut out, view);
    write_team_season_strength_and_ledger(&mut out, view);
    write_team_season_game_log(&mut out, view);
    Ok(out)
}

fn write_team_season_source_state(out: &mut String, view: &TeamSeasonView) {
    let _ = writeln!(out, "## Source State\n");
    let _ = writeln!(out, "| Source | State | Message |");
    let _ = writeln!(out, "|---|---|---|");
    for source in &view.context.source_state {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            source_kind_label(source.source),
            completeness_label(source.state),
            source.message.as_deref().unwrap_or("-")
        );
    }
    if !view.warnings.is_empty() {
        let _ = writeln!(out, "\n## Warnings\n");
        for warning in &view.warnings {
            let _ = writeln!(out, "- {}", warning.message);
        }
    }
    let _ = writeln!(out);
}

fn write_team_season_splits(out: &mut String, view: &TeamSeasonView) {
    let _ = writeln!(out, "## Splits\n");
    let _ = writeln!(out, "| Split | Record | GF | GA | GD |");
    let _ = writeln!(out, "|---|---:|---:|---:|---:|");
    let rows = [
        ("Home", &view.splits.home),
        ("Away", &view.splits.away),
        ("One-goal", &view.splits.one_goal),
    ];
    for (label, split) in rows {
        let _ = writeln!(
            out,
            "| {label} | {} | {} | {} | {} |",
            schedule_record_label(split.record),
            split.goals_for,
            split.goals_against,
            signed_i32(split.goal_differential)
        );
    }
    let _ = writeln!(out);
}

fn write_team_season_strength_and_ledger(out: &mut String, view: &TeamSeasonView) {
    let _ = writeln!(out, "## Schedule Strength\n");
    let _ = writeln!(
        out,
        "| Window | Games | Avg opp pts% | Top | Middle | Bottom | Unknown |"
    );
    let _ = writeln!(out, "|---|---:|---:|---:|---:|---:|---:|");
    let _ = writeln!(
        out,
        "| Faced | {} | {} | {} | {} | {} | {} |",
        view.schedule_strength.faced_games,
        pct_or_dash(view.schedule_strength.faced_average_points_percentage),
        view.schedule_strength.faced.top,
        view.schedule_strength.faced.middle,
        view.schedule_strength.faced.bottom,
        view.schedule_strength.faced.unknown
    );
    let _ = writeln!(
        out,
        "| Remaining | {} | {} | {} | {} | {} | {} |",
        view.schedule_strength.remaining_games,
        pct_or_dash(view.schedule_strength.remaining_average_points_percentage),
        view.schedule_strength.remaining.top,
        view.schedule_strength.remaining.middle,
        view.schedule_strength.remaining.bottom,
        view.schedule_strength.remaining.unknown
    );
    let _ = writeln!(out, "\n## Quality Ledger\n");
    let _ = writeln!(out, "| Quality wins | Expected wins | Bad losses | Missed points | Top-opponent games | Bottom-opponent games |");
    let _ = writeln!(out, "|---:|---:|---:|---:|---:|---:|");
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} |",
        view.quality_ledger.quality_wins,
        view.quality_ledger.expected_wins,
        view.quality_ledger.bad_losses,
        view.quality_ledger.missed_points,
        view.quality_ledger.top_opponent_games,
        view.quality_ledger.bottom_opponent_games
    );
    let _ = writeln!(out);
}

fn write_team_season_game_log(out: &mut String, view: &TeamSeasonView) {
    let _ = writeln!(out, "## Game Log\n");
    if view.rows.is_empty() {
        let _ = writeln!(out, "_No games found for this team season._");
        return;
    }
    let _ = writeln!(out, "| Date | V | Opp | Result | Score | GD | Status |");
    let _ = writeln!(out, "|---|:---:|:---:|---|---:|---:|---|");
    for row in &view.rows {
        let _ = writeln!(out, "{}", team_season_markdown_row(row));
    }
}

fn team_season_markdown_row(row: &TeamSeasonGameRow) -> String {
    format!(
        "| {} | {} | {} | {} | {} | {} | {} |",
        row.date,
        match row.venue {
            TeamSeasonVenue::Home => "H",
            TeamSeasonVenue::Away => "A",
        },
        row.opponent_abbrev,
        row.result,
        match (row.team_score, row.opponent_score) {
            (Some(team_score), Some(opponent_score)) => format!("{team_score}-{opponent_score}"),
            _ => "-".to_string(),
        },
        row.goal_differential
            .map(|value| signed_i32(value as i32))
            .unwrap_or_else(|| "-".to_string()),
        row.state_label
    )
}

fn schedule_record_label(record: ScheduleRecord) -> String {
    format!(
        "{}-{}-{}",
        record.wins, record.losses, record.overtime_losses
    )
}

fn signed_i32(value: i32) -> String {
    format!("{value:+}")
}

fn pct_or_dash(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

fn source_kind_label(source: icelines_core::SourceKind) -> &'static str {
    match source {
        icelines_core::SourceKind::Schedule => "schedule",
        icelines_core::SourceKind::Standings => "standings",
        icelines_core::SourceKind::Roster => "roster",
        icelines_core::SourceKind::Scores => "scores",
        icelines_core::SourceKind::Playoffs => "playoffs",
        icelines_core::SourceKind::Favorites => "favorites",
        icelines_core::SourceKind::Watchlist => "watchlist",
        icelines_core::SourceKind::Career => "career",
        icelines_core::SourceKind::Home => "home",
        icelines_core::SourceKind::Docs => "docs",
        icelines_core::SourceKind::GameLog => "game-log",
        icelines_core::SourceKind::Boxscore => "boxscore",
        icelines_core::SourceKind::PlayByPlay => "play-by-play",
        icelines_core::SourceKind::Shifts => "shifts",
        icelines_core::SourceKind::Transactions => "transactions",
        icelines_core::SourceKind::Contracts => "contracts",
        icelines_core::SourceKind::FantasyImport => "fantasy-import",
        icelines_core::SourceKind::Snapshot => "snapshot",
        icelines_core::SourceKind::Bundle => "bundle",
        icelines_core::SourceKind::Cache => "cache",
        icelines_core::SourceKind::Unknown => "unknown",
    }
}

fn completeness_label(completeness: Completeness) -> &'static str {
    match completeness {
        Completeness::Complete => "complete",
        Completeness::Partial => "partial",
        Completeness::Stale => "stale",
        Completeness::Unavailable => "unavailable",
    }
}

// ── depth ────────────────────────────────────────────────────────────────────

fn metric_display(metrics: &[icelines_core::MetricCell], key: &str) -> String {
    metrics
        .iter()
        .find_map(|metric| {
            if metric.key.0 == key {
                match metric.value {
                    icelines_core::MetricValue::Integer(value) => Some(value.to_string()),
                    icelines_core::MetricValue::Decimal(value) => Some(format!("{value:.1}")),
                    icelines_core::MetricValue::Text(ref value) => Some(value.clone()),
                    icelines_core::MetricValue::Missing => None,
                }
            } else {
                None
            }
        })
        .unwrap_or_else(|| "â€”".to_owned())
}

fn depth_player_cell(slot: &icelines_core::view_model::DepthPlayerSlot) -> String {
    format!(
        "{} ({})",
        truncate(&slot.display_name, 24),
        metric_display(&slot.metrics, "pace_82")
    )
}

fn write_team_depth_view_markdown(out: &mut String, view: &TeamDepthView) {
    let _ = writeln!(out, "## Estimated lineup");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Line | LW | C | RW |");
    let _ = writeln!(out, "|-----:|----|---|----|");
    for line in &view.forward_lines {
        let left = line
            .left
            .as_ref()
            .map(depth_player_cell)
            .unwrap_or_else(|| "â€”".to_owned());
        let center = line
            .center
            .as_ref()
            .map(depth_player_cell)
            .unwrap_or_else(|| "â€”".to_owned());
        let right = line
            .right
            .as_ref()
            .map(depth_player_cell)
            .unwrap_or_else(|| "â€”".to_owned());
        let _ = writeln!(
            out,
            "| {line_no:>4} | {left} | {center} | {right} |",
            line_no = line.line
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "| Pair | D1 | D2 |");
    let _ = writeln!(out, "|-----:|----|----|");
    for pair in &view.defense_pairs {
        let left = pair
            .left
            .as_ref()
            .map(depth_player_cell)
            .unwrap_or_else(|| "â€”".to_owned());
        let right = pair
            .right
            .as_ref()
            .map(depth_player_cell)
            .unwrap_or_else(|| "â€”".to_owned());
        let _ = writeln!(
            out,
            "| {pair_no:>4} | {left} | {right} |",
            pair_no = pair.pair
        );
    }
    let _ = writeln!(out);
}

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
    let season = views
        .first()
        .map(|view| view.season())
        .unwrap_or(icelines_core::model::Season(icelines_core::CURRENT_SEASON));
    let season_type = views
        .first()
        .map(|view| view.season_type())
        .unwrap_or(icelines_core::season_stats::SeasonType::Regular);
    let league_view =
        DepthLeagueView::from_player_views(season, season_type, true, views, ScoringMode::Pace);

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
    write_depth_league_view_markdown(&mut out, &league_view);
    let _ = writeln!(out);

    let _ = writeln!(
        out,
        "## Line-value players\n\n| Rank | Player | Team | Pos | Own line | Avg other | Delta | Fit |"
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
            fit  = crate::visual::web_fit_ascii_label(m.web_fit_class()),
        );
    }
    Ok(out)
}

fn write_depth_league_view_markdown(out: &mut String, view: &DepthLeagueView) {
    let _ = writeln!(
        out,
        "## Team strength\n\n| Rank | Team | C | LW | RW | D | Total | C top | LW top | RW top | D top |"
    );
    let _ = writeln!(
        out,
        "|-----:|:----:|--:|---:|---:|--:|------:|-------|--------|--------|-------|"
    );
    for (idx, row) in view.rows.iter().enumerate() {
        let _ = writeln!(
            out,
            "| {rank:>4} | {team} | {c:>3.0} | {lw:>3.0} | {rw:>3.0} | {d:>3.0} | {total:>5.0} | {c_top} | {lw_top} | {rw_top} | {d_top} |",
            rank = idx + 1,
            team = row.team.0,
            c = row.c_score,
            lw = row.lw_score,
            rw = row.rw_score,
            d = row.d_score,
            total = row.total,
            c_top = truncate(&row.c_top, 18),
            lw_top = truncate(&row.lw_top, 18),
            rw_top = truncate(&row.rw_top, 18),
            d_top = truncate(&row.d_top, 18),
        );
    }
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

pub(crate) struct FantasyOpts {
    pub top: usize,
    pub width: u16,
    pub height: u16,
}

pub(crate) fn render_fantasy(opts: FantasyOpts) -> anyhow::Result<String> {
    let (outcome, season, season_type) = load_repo_for_season(None, None)?;
    let mut query = PoachQuery::new(season, season_type, "yahoo-standard");
    query.limit = Some(opts.top.clamp(1, u16::MAX as usize) as u16);
    query.sort = Some("poach_score".to_owned());
    let board = PoachBoardView::from_repository(&outcome.repo, query);
    let report = poach_report_from_board(board);
    render_fantasy_from_report(&report, &opts)
}

pub(crate) fn render_fantasy_from_report(
    report: &PoachReportView,
    opts: &FantasyOpts,
) -> anyhow::Result<String> {
    let mut out = String::new();
    write_front_matter(
        &mut out,
        "fantasy-poacher",
        &report.context.title,
        &[
            ("scheme", report.scoring_scheme.clone()),
            ("window", format!("{:?}", report.window)),
        ],
        opts.width,
        opts.height,
    );
    out.push_str(&crate::commands::poach::render_report_markdown(report));
    Ok(out)
}

pub(crate) struct SeriesOpts {
    pub series: Option<String>,
    pub width: u16,
    pub height: u16,
}

pub(crate) fn render_series(opts: SeriesOpts) -> anyhow::Result<String> {
    let season = crate::commands::playoffs::default_season()
        .context("no playoff seasons available in the bundle")?;
    let bundle = icelines_fetch::bundled::load_playoffs(&season)
        .with_context(|| format!("no playoff bundle for season '{season}'"))?;
    let view = crate::commands::playoffs::playoffs_view_from_bundle(&bundle);
    render_series_from_view(&view, &opts)
}

pub(crate) fn render_series_from_view(
    view: &PlayoffsView,
    opts: &SeriesOpts,
) -> anyhow::Result<String> {
    let requested = opts.series.as_deref().map(|value| value.to_uppercase());
    let (round_label, series) = find_series(view, requested.as_deref()).with_context(|| {
        if let Some(letter) = requested.as_deref() {
            format!("no playoff series '{letter}' in {}", view.season_pretty)
        } else {
            format!("no playoff series in {}", view.season_pretty)
        }
    })?;
    let letter = if series.letter.is_empty() {
        "unknown".to_owned()
    } else {
        series.letter.clone()
    };
    let title = format!(
        "{} {} vs {} - {}",
        view.season_pretty, series.top_abbrev, series.bottom_abbrev, round_label
    );

    let mut out = String::new();
    write_front_matter(
        &mut out,
        "series-log",
        &title,
        &[("series", letter.clone()), ("season", view.season.clone())],
        opts.width,
        opts.height,
    );
    let _ = writeln!(out, "## Summary");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Series: {letter}");
    let _ = writeln!(out, "- Round: {round_label}");
    let _ = writeln!(
        out,
        "- Matchup: {} vs {}",
        series.top_name, series.bottom_name
    );
    let _ = writeln!(out, "- Result: {}", series.summary);
    let _ = writeln!(out);
    let _ = writeln!(out, "## Game Log");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Game | Date | Result | Series After |");
    let _ = writeln!(out, "|-----:|------|--------|--------------|");
    for game in &series.games {
        let result = format!(
            "{} {}-{} {}",
            game.away_abbrev, game.away_score, game.home_score, game.home_abbrev
        );
        let _ = writeln!(
            out,
            "| {game_no:>4} | {date} | {result} | {after} |",
            game_no = game.game_number,
            date = game.date,
            result = result,
            after = game.series_after,
        );
    }
    Ok(out)
}

fn find_series<'a>(
    view: &'a PlayoffsView,
    requested: Option<&str>,
) -> Option<(&'a str, &'a PlayoffsSeriesRow)> {
    for round in &view.rounds {
        for series in &round.series {
            let matches = requested
                .map(|letter| series.letter.eq_ignore_ascii_case(letter))
                .unwrap_or(true);
            if matches {
                return Some((round.label.as_str(), series));
            }
        }
    }
    None
}

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
    write_front_matter_with_context(
        out,
        ty,
        title,
        filters,
        (width, height),
        FrontMatterMetadata::default(),
    );
}

fn write_front_matter_with_context(
    out: &mut String,
    ty: &str,
    title: &str,
    filters: &[(&str, String)],
    dimensions: (u16, u16),
    metadata: FrontMatterMetadata<'_>,
) {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "type: {ty}");
    let _ = writeln!(out, "title: \"{title}\"");
    let _ = writeln!(out, "generated_at: {now}");
    let season = metadata
        .context
        .map(|context| context.window.season.0.to_string())
        .unwrap_or_else(|| icelines_core::CURRENT_SEASON_STR.to_string());
    let _ = writeln!(out, "season: {season}");
    if let Some(context) = metadata.context {
        let _ = writeln!(out, "season_type: {}", context.window.season_type.label());
        if !context.source_state.is_empty() {
            let _ = writeln!(out, "sources:");
            for source in &context.source_state {
                let _ = writeln!(out, "  - kind: {}", source_kind_label(source.source));
                let _ = writeln!(
                    out,
                    "    completeness: {}",
                    completeness_label(source.state)
                );
            }
        }
    }
    if let Some(result) = metadata.result {
        let _ = writeln!(out, "result:");
        let _ = writeln!(out, "  total: {}", result.total);
        let _ = writeln!(out, "  returned: {}", result.returned);
        let _ = writeln!(out, "  top: {}", result.top);
        let _ = writeln!(out, "  sort: \"{}\"", result.sort);
        let _ = writeln!(out, "  active_filters: \"{}\"", result.active_filters);
    }
    if let Some(view) = metadata.leaders_state {
        write_leaders_state_front_matter(out, view);
    }
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
    let (width, height) = dimensions;
    let _ = writeln!(out, "  width: {width}");
    let _ = writeln!(out, "  height: {height}");
    let _ = writeln!(out, "---");
    let _ = writeln!(out);
    write_public_report_disclosure(out);
}

fn write_public_report_disclosure(out: &mut String) {
    let _ = writeln!(out, "## Disclosure\n");
    let _ = writeln!(
        out,
        "- Methodology: IceLines reports summarize available roster, schedule, boxscore, play-by-play, and source-state data exactly as labeled."
    );
    let _ = writeln!(
        out,
        "- Completeness: Incomplete, partial, stale, missing, or skeleton source states are evidence limits, not zero-value truth; review the source, warning, and empty-state sections before using the rows."
    );
    let _ = writeln!(
        out,
        "- Limitations: This artifact is not era-adjusted, predictive, betting, injury, special-teams, deployment, or linemate analysis."
    );
    let _ = writeln!(out);
}

fn write_leaders_state_front_matter(out: &mut String, view: &LeadersView) {
    if view.empty_state.is_none() && view.warnings.is_empty() {
        return;
    }

    let _ = writeln!(out, "state:");
    if let Some(empty) = &view.empty_state {
        let _ = writeln!(out, "  empty_state:");
        let _ = writeln!(out, "    kind: \"{}\"", empty_kind_label(empty.kind));
        let _ = writeln!(out, "    title: {}", yaml_quote(&empty.title));
        if let Some(detail) = &empty.detail {
            let _ = writeln!(out, "    detail: {}", yaml_quote(detail));
        }
        if !empty.recovery.is_empty() {
            let _ = writeln!(out, "    recovery:");
            for recovery in &empty.recovery {
                let _ = writeln!(
                    out,
                    "      - {}",
                    yaml_quote(&recovery_action_label(recovery))
                );
            }
        }
    }
    if !view.warnings.is_empty() {
        let _ = writeln!(out, "  warnings:");
        for warning in &view.warnings {
            let _ = writeln!(out, "    - kind: \"{}\"", warning_kind_label(warning.kind));
            let _ = writeln!(out, "      message: {}", yaml_quote(&warning.message));
            if !warning.recovery.is_empty() {
                let _ = writeln!(out, "      recovery:");
                for recovery in &warning.recovery {
                    let _ = writeln!(
                        out,
                        "        - {}",
                        yaml_quote(&recovery_action_label(recovery))
                    );
                }
            }
        }
    }
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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
        PaceScore, ScheduledGameInput, TeamAbbr, TeamSeasonView, TeamStandingInput, ViewContext,
        ViewWindow,
    };

    /// Build a one-row repo at the given (id, name, team, pos, pace_82)
    /// for export-shape tests. Pace_82 drives sort order; other counters
    /// are filled from a default (30G, 50A, 60GP) shape.
    fn fixture_repo(
        rows: &[(u32, &str, &str, Position, f64)],
    ) -> icelines_core::stats_repository::StatsRepository {
        let rows_with_gp: Vec<(u32, &str, &str, Position, f64, u32)> = rows
            .iter()
            .map(|(id, name, team, pos, pace_82)| (*id, *name, *team, *pos, *pace_82, 60))
            .collect();
        fixture_repo_with_gp(&rows_with_gp)
    }

    fn fixture_repo_with_gp(
        rows: &[(u32, &str, &str, Position, f64, u32)],
    ) -> icelines_core::stats_repository::StatsRepository {
        let mut repo = icelines_core::stats_repository::StatsRepository::new();
        for (id, name, team, pos, pace_82, gp) in rows {
            let identity = icelines_core::fixtures::identity(*id)
                .name(name, &name.to_lowercase().replace(' ', "_"))
                .build();
            let totals = StatTotals {
                gp: *gp,
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
                    gp: *gp,
                }),
            };
            let stint = TeamStint {
                team: TeamAbbr((*team).to_owned()),
                started: Some("2024-10-15".into()),
                ended: Some("2025-04-13".into()),
                gp: *gp,
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

    fn team_season_fixture() -> TeamSeasonView {
        fn game(
            id: u64,
            date: &str,
            away: &str,
            home: &str,
            away_score: Option<u8>,
            home_score: Option<u8>,
            last_period: Option<&str>,
        ) -> ScheduledGameInput {
            ScheduledGameInput {
                game_id: id,
                date: date.to_string(),
                game_type: 2,
                away_abbrev: away.to_string(),
                away_name: away.to_string(),
                home_abbrev: home.to_string(),
                home_name: home.to_string(),
                start_time_utc: format!("{date}T23:00:00Z"),
                away_score,
                home_score,
                game_state: Some(if away_score.is_some() {
                    "FINAL".to_string()
                } else {
                    "FUT".to_string()
                }),
                last_period: last_period.map(str::to_string),
                series_game: None,
                away_wins: None,
                home_wins: None,
            }
        }
        fn standing(team: &str, points_percentage: f32, points: u32) -> TeamStandingInput {
            TeamStandingInput {
                team: team.to_string(),
                conference: Some("Western".to_string()),
                division: Some("Pacific".to_string()),
                games_played: 40,
                wins: 20,
                losses: 15,
                overtime_losses: 5,
                points,
                points_percentage,
                regulation_wins: Some(18),
                goal_differential: 0,
                league_rank: None,
                conference_rank: None,
                division_rank: None,
                wild_card_rank: None,
            }
        }
        TeamSeasonView::from_games_and_standings(
            ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular)),
            "20242025".to_string(),
            "SEA".to_string(),
            vec![
                game(1, "2024-10-01", "SEA", "COL", Some(4), Some(2), Some("REG")),
                game(2, "2024-10-03", "SEA", "SJS", Some(1), Some(2), Some("REG")),
                game(3, "2024-10-05", "ANA", "SEA", Some(3), Some(2), Some("OT")),
                game(4, "2024-10-07", "SEA", "VGK", None, None, None),
            ],
            vec![
                standing("COL", 0.720, 58),
                standing("VGK", 0.690, 56),
                standing("SEA", 0.600, 48),
                standing("ANA", 0.430, 34),
                standing("SJS", 0.350, 28),
            ],
        )
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
            filters: Vec::new(),
            columns: None,
            width: 100,
            height: 30,
        };
        let out = render_leaders_from_views(&views, &opts).unwrap();
        assert!(out.starts_with("---\n"));
        assert!(
            out.contains("\n---\n\n## Disclosure\n"),
            "front-matter must precede the disclosure section"
        );
        assert!(
            out.find("## Disclosure").unwrap() < out.find("## Context").unwrap(),
            "disclosure must precede the context section"
        );
        assert!(out.contains("type: leaderboard"));
        assert!(out.contains("title:"));
        assert!(out.contains("generated_at:"));
        assert!(out.contains("season: "));
        assert!(out.contains("season_type: regular"));
        assert!(out.contains("sources:\n  - kind: roster\n    completeness: complete"));
        assert!(out.contains("result:\n  total: 3\n  returned: 3\n  top: 25"));
        assert!(out.contains("  sort: \"pts-pace\""));
        assert!(out.contains("  active_filters: \"-\""));
        assert!(out.contains("proof:"));
        assert!(out.contains("width: 100"));
        assert!(out.contains("height: 30"));
    }

    #[test]
    fn l0_export_leaders_discloses_methodology_limits_near_top() {
        let repo = fixture_repo(&[(1, "A One", "EDM", Position::Center, 109.0)]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(
            out.find("## Disclosure").unwrap() < out.find("| Rank | Player").unwrap(),
            "public disclosure must precede the shared table"
        );
        for expected in [
            "source-state data exactly as labeled",
            "Incomplete, partial, stale, missing, or skeleton source states",
            "not zero-value truth",
            "source, warning, and empty-state sections",
            "not era-adjusted",
            "predictive",
            "betting",
            "injury",
            "special-teams",
            "deployment",
            "linemate analysis",
        ] {
            assert!(
                out.contains(expected),
                "missing disclosure term: {expected}"
            );
        }
    }

    #[test]
    fn l0_export_leaders_reports_context_and_source_state() {
        let repo = fixture_repo(&[
            (1, "A One", "EDM", Position::Center, 109.0),
            (2, "B Two", "COL", Position::Center, 95.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.contains("## Context\n"));
        assert!(out.contains(&format!("- Season: {}", icelines_core::CURRENT_SEASON)));
        assert!(out.contains("- Season type: regular"));
        assert!(out.contains("- Sources:\n  - roster: complete"));
        assert!(
            out.find("## Context").unwrap() < out.find("| Rank | Player").unwrap(),
            "context/source state must precede the leaders table"
        );
    }

    #[test]
    fn l0_export_leaders_reports_result_and_query_intent() {
        let repo = fixture_repo(&[
            (1, "A One", "EDM", Position::Center, 109.0),
            (2, "B Two", "COL", Position::Center, 95.0),
            (3, "C Tre", "SEA", Position::LeftWing, 80.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: Some("c".into()),
                top: 1,
                sort: "pts-pace".into(),
                gp_min: Some(50),
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.contains("## Result\n"));
        assert!(out.contains("result:\n  total: 2\n  returned: 1\n  top: 1"));
        assert!(out.contains("  sort: \"pts-pace\""));
        assert!(out.contains("  active_filters: \"position=C; gp>=50\""));
        assert!(out.contains("- Total: 2"));
        assert!(out.contains("- Returned: 1"));
        assert!(out.contains("- Top: 1"));
        assert!(out.contains("- Sort: pts-pace"));
        assert!(out.contains("- Active filters: position=C; gp>=50"));
        assert!(
            out.find("## Context").unwrap() < out.find("## Result").unwrap(),
            "result metadata must follow context/source state"
        );
        assert!(
            out.find("## Result").unwrap() < out.find("| Rank | Player").unwrap(),
            "result metadata must precede the leaders table"
        );
    }

    #[test]
    fn l0_export_leaders_gp_min_filters_rows_and_reports_threshold() {
        let repo = fixture_repo_with_gp(&[
            (1, "Threshold Keeper", "EDM", Position::Center, 109.0, 41),
            (2, "Threshold Miss", "COL", Position::Center, 110.0, 40),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: Some(41),
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.contains("gp_min: \"41\""));
        assert!(out.contains("result:\n  total: 1\n  returned: 1\n  top: 25"));
        assert!(out.contains("  active_filters: \"gp>=41\""));
        assert!(out.contains("- Active filters: gp>=41"));
        assert!(out.contains("Threshold Keeper"));
        assert!(
            !out.contains("Threshold Miss"),
            "--gp-min must exclude rows below the threshold before table rendering"
        );
        assert!(
            out.find("## Result").unwrap() < out.find("| Rank | Player").unwrap(),
            "threshold evidence must be reported before the shared table"
        );
    }

    #[test]
    fn l0_export_leaders_free_form_filter_reports_and_filters_rows() {
        let repo = fixture_repo_with_gp(&[
            (1, "Keeper", "EDM", Position::Center, 109.0, 62),
            (2, "Dropped", "COL", Position::Center, 95.0, 59),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 5,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: vec!["gp>=60".into()],
                columns: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.contains("result:\n  total: 1\n  returned: 1\n  top: 5"));
        assert!(out.contains("  active_filters: \"gp>=60\""));
        assert!(out.contains("- Active filters: gp>=60"));
        assert!(out.contains("Keeper"));
        assert!(
            !out.contains("Dropped"),
            "free-form filters must narrow rendered report rows"
        );
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
                filters: Vec::new(),
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
                filters: Vec::new(),
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
    fn l0_export_leaders_preserves_unicode_and_duplicate_names_as_rows() {
        let repo = fixture_repo(&[
            (1, "Jose Nunez", "EDM", Position::Center, 109.0),
            (2, "Jose Nunez", "COL", Position::LeftWing, 108.0),
            (3, "José Núñez", "SEA", Position::Defense, 107.0),
        ]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert_eq!(
            out.matches("| Jose Nunez |").count(),
            2,
            "duplicate display names must remain separate rendered rows"
        );
        assert!(
            out.contains("| José Núñez |"),
            "Unicode player names must survive Markdown rendering"
        );
        assert!(out.contains("| Jose Nunez | EDM | C |"));
        assert!(out.contains("| Jose Nunez | COL | LW |"));
        assert!(out.contains("| José Núñez | SEA | D |"));
    }

    #[test]
    fn l0_export_leaders_traded_player_renders_once_with_last_stint_team() {
        let mut repo = icelines_core::stats_repository::StatsRepository::new();
        let identity = icelines_core::fixtures::identity(42)
            .name("Trade Continuity", "trade_continuity")
            .build();
        let totals = StatTotals {
            gp: 60,
            goals: 20,
            assists: 35,
            points: 55,
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
                pace_82: 75.2,
                goals_per_82: 27.3,
                raw_points: 55,
                gp: 60,
            }),
        };
        let stats = SeasonStatsBuilder::new(
            PlayerId(42),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .with_totals(totals)
        .add_team_stint(TeamStint {
            team: TeamAbbr("EDM".to_owned()),
            started: Some("2024-10-15".into()),
            ended: Some("2025-02-01".into()),
            gp: 25,
            goals: 8,
            assists: 15,
            points: 23,
            goalie: None,
        })
        .add_team_stint(TeamStint {
            team: TeamAbbr("NYR".to_owned()),
            started: Some("2025-02-02".into()),
            ended: Some("2025-04-13".into()),
            gp: 35,
            goals: 12,
            assists: 20,
            points: 32,
            goalie: None,
        })
        .build();
        repo.upsert_identity(identity).unwrap();
        repo.upsert_stats(stats).unwrap();

        let views = fixture_views(&repo);
        assert_eq!(views.len(), 1);
        assert!(views[0].was_traded_in_window());

        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert_eq!(
            out.matches("Trade Continuity").count(),
            1,
            "traded player must render as one aggregate row"
        );
        assert!(out.contains("| Trade Continuity | NYR | C |"));
        assert!(out.contains("|  60 |"));
        assert!(out.contains("|  20 |  35 |   55 |"));
        assert!(
            !out.contains("| Trade Continuity | EDM |"),
            "leaders export should use last-stint team rather than splitting traded rows"
        );
    }

    #[test]
    fn l0_export_leaders_honors_lockout_and_october_rollover_windows() {
        fn repo_for_window(
            id: u32,
            name: &str,
            team: &str,
            season: Season,
            season_type: SeasonType,
            gp: u32,
            date_range: (&str, &str),
        ) -> icelines_core::stats_repository::StatsRepository {
            let mut repo = icelines_core::stats_repository::StatsRepository::new();
            let identity = icelines_core::fixtures::identity(id)
                .name(name, &name.to_lowercase().replace(' ', "_"))
                .build();
            let totals = StatTotals {
                gp,
                goals: 12,
                assists: 24,
                points: 36,
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
                    pace_82: 61.5,
                    goals_per_82: 20.5,
                    raw_points: 36,
                    gp,
                }),
            };
            let stats =
                SeasonStatsBuilder::new(PlayerId(id), season, season_type, Position::Center)
                    .with_totals(totals)
                    .add_team_stint(TeamStint {
                        team: TeamAbbr(team.to_owned()),
                        started: Some(date_range.0.into()),
                        ended: Some(date_range.1.into()),
                        gp,
                        goals: 12,
                        assists: 24,
                        points: 36,
                        goalie: None,
                    })
                    .build();
            repo.upsert_identity(identity).unwrap();
            repo.upsert_stats(stats).unwrap();
            repo
        }

        let lockout_repo = repo_for_window(
            77,
            "Lockout Window",
            "NJD",
            Season(20122013),
            SeasonType::Regular,
            48,
            ("2013-01-19", "2013-04-27"),
        );
        let lockout_views: Vec<_> = lockout_repo
            .skaters(Season(20122013), SeasonType::Regular)
            .collect();
        let lockout = render_leaders_from_views_with_window(
            &lockout_views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
            Season(20122013),
            SeasonType::Regular,
        )
        .unwrap();
        assert!(lockout.contains("season: 20122013"));
        assert!(lockout.contains("- Season: 20122013"));
        assert!(lockout.contains("- Season type: regular"));
        assert!(lockout.contains("| Lockout Window | NJD | C |"));
        assert!(lockout.contains("|  48 |"));

        let rollover_repo = repo_for_window(
            78,
            "October Rollover",
            "SEA",
            Season(20252026),
            SeasonType::Regular,
            1,
            ("2025-10-07", "2025-10-07"),
        );
        let rollover_views: Vec<_> = rollover_repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect();
        let rollover = render_leaders_from_views_with_window(
            &rollover_views,
            &LeadersOpts {
                pos: None,
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 100,
                height: 30,
            },
            Season(20252026),
            SeasonType::Regular,
        )
        .unwrap();
        assert!(rollover.contains("season: 20252026"));
        assert!(rollover.contains("- Season: 20252026"));
        assert!(rollover.contains("| October Rollover | SEA | C |"));
        assert!(rollover.contains("|   1 |"));
        assert!(
            !rollover.contains("20242025"),
            "explicit October rollover window must not fall back to the prior season"
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
                filters: Vec::new(),
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
    fn l0_export_leaders_warning_empty_summary_reports_recovery() {
        let repo = fixture_repo(&[(1, "Cee", "EDM", Position::Center, 100.0)]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: Some("G".into()),
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.contains("## Warnings\n"));
        assert!(out.contains(
            "- unsupported_filter: The leaders surface is skater-only; use goalies for goalie leaders."
        ));
        assert!(out.contains("## Empty State\n"));
        assert!(out.contains("- Kind: no_rows"));
        assert!(out.contains("- Title: No skater leaders"));
        assert!(out.contains(
            "- Detail: The leaders surface is skater-only; use the goalies surface for goalie leaders."
        ));
        assert!(out.contains("- Recovery: Open goalie leaders -> /goalies"));
        assert!(
            out.find("## Result").unwrap() < out.find("## Warnings").unwrap(),
            "warning summary must follow result metadata"
        );
        assert!(
            out.find("## Empty State").unwrap() < out.find("| Rank | Player").unwrap(),
            "empty-state summary must precede the leaders table"
        );
    }

    #[test]
    fn l0_export_leaders_front_matter_reports_empty_warning_state() {
        let repo = fixture_repo(&[(1, "Cee", "EDM", Position::Center, 100.0)]);
        let views = fixture_views(&repo);
        let out = render_leaders_from_views(
            &views,
            &LeadersOpts {
                pos: Some("G".into()),
                top: 25,
                sort: "pts-pace".into(),
                gp_min: None,
                filters: Vec::new(),
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap();

        let after_open = out
            .strip_prefix("---\n")
            .expect("front matter opens with delimiter");
        let close = after_open
            .find("\n---\n")
            .expect("front matter closes with delimiter");
        let yaml = &after_open[..close];
        assert!(yaml.contains("state:\n  empty_state:\n    kind: \"no_rows\""));
        assert!(yaml.contains("    title: \"No skater leaders\""));
        assert!(yaml.contains(
            "    detail: \"The leaders surface is skater-only; use the goalies surface for goalie leaders.\""
        ));
        assert!(yaml.contains("    recovery:\n      - \"Open goalie leaders -> /goalies\""));
        assert!(yaml.contains("  warnings:\n    - kind: \"unsupported_filter\""));
        assert!(yaml.contains(
            "      message: \"The leaders surface is skater-only; use goalies for goalie leaders.\""
        ));
        assert!(yaml.contains("      recovery:\n        - \"Open goalie leaders -> /goalies\""));
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
                filters: Vec::new(),
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
                filters: Vec::new(),
                columns: None,
                width: 80,
                height: 30,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid-metric"));
    }

    #[test]
    fn l0_export_team_season_renders_viewmodel_report_sections() {
        let view = team_season_fixture();
        let out = render_team_season_from_view(
            &view,
            &TeamSeasonOpts {
                team: "SEA".to_string(),
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.starts_with("---\n"));
        assert!(out.contains("type: team-season"));
        assert!(out.contains("## Summary"));
        assert!(out.contains("- Recent form: last 5"));
        assert!(out.contains("## Source State"));
        assert!(out.contains("| schedule | complete | - |"));
        assert!(out.contains("| standings | complete | - |"));
        assert!(out.contains("## Splits"));
        assert!(out.contains("## Schedule Strength"));
        assert!(out.contains("## Quality Ledger"));
        assert!(out.contains("## Game Log"));
        assert!(out.contains("| 2024-10-01 | A | COL | W | 4-2 | +2 | FINAL |"));
    }

    #[test]
    fn l0_export_team_season_preserves_missing_source_warning() {
        let view = TeamSeasonView::from_games(
            ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular)),
            "20242025".to_string(),
            "SEA".to_string(),
            Vec::new(),
        );
        let out = render_team_season_from_view(
            &view,
            &TeamSeasonOpts {
                team: "SEA".to_string(),
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.contains("| standings | unavailable | source window is not loaded |"));
        assert!(out.contains("## Warnings"));
        assert!(out.contains("Standings source not loaded"));
        assert!(out.contains("_No games found for this team season._"));
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
                filters: Vec::new(),
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
        assert!(out.contains("## Estimated lineup"));
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
        assert!(out.contains("## Team strength"));
        assert!(out.contains("| Rank | Team | C | LW | RW | D | Total |"));
        assert!(out.contains("## Line-value players"));
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
    fn l0_export_fantasy_wraps_poach_report_with_front_matter() {
        let context = icelines_core::ViewContext::new(icelines_core::ViewWindow::new(
            icelines_core::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        ));
        let report = PoachReportView {
            context: icelines_core::view_model::poach_report_context(context, "fantasy-export"),
            scoring_scheme: "yahoo-standard".to_string(),
            scoring_categories: vec!["hits".to_string(), "blocks".to_string()],
            window: icelines_core::view_model::PoachWindow::Days14,
            source_state: Vec::new(),
            warnings: Vec::new(),
            omissions: vec!["fantasy_import: unavailable".to_string()],
            sections: vec![icelines_core::view_model::PoachReportSection {
                id: "top_adds".to_string(),
                title: "Top Adds".to_string(),
                rows: Vec::new(),
            }],
        };

        let out = render_fantasy_from_report(
            &report,
            &FantasyOpts {
                top: 25,
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.starts_with("---\n"));
        assert!(out.contains("type: fantasy-poacher"));
        assert!(out.contains("scheme: \"yahoo-standard\""));
        assert!(out.contains("# Fantasy Poacher"));
        assert!(out.contains("fantasy_import: unavailable"));
    }

    #[test]
    fn l0_export_series_emits_game_log_from_playoffs_view() {
        let view = PlayoffsView::from_bracket(
            ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Playoff)),
            "20242025".to_owned(),
            "fixture".to_owned(),
            icelines_core::PlayoffsBracketInput {
                rounds: vec![icelines_core::PlayoffsRoundInput {
                    round_number: 1,
                    label: "Round 1".to_owned(),
                    series: vec![icelines_core::PlayoffsSeriesInput {
                        letter: Some("A".to_owned()),
                        top_abbrev: "FLA".to_owned(),
                        top_name: "Florida Panthers".to_owned(),
                        top_wins: 1,
                        top_seed_rank: Some("1".to_owned()),
                        bottom_abbrev: "TBL".to_owned(),
                        bottom_name: "Tampa Bay Lightning".to_owned(),
                        bottom_wins: 0,
                        bottom_seed_rank: Some("WC1".to_owned()),
                        winner_abbrev: None,
                        conference: Some("East".to_owned()),
                        games: vec![icelines_core::PlayoffsGameInput {
                            date: "2025-04-19".to_owned(),
                            home_abbrev: "FLA".to_owned(),
                            away_abbrev: "TBL".to_owned(),
                            home_score: 4,
                            away_score: 2,
                            series_after: "FLA leads 1-0".to_owned(),
                        }],
                    }],
                }],
            },
        );
        let out = render_series_from_view(
            &view,
            &SeriesOpts {
                series: Some("A".to_owned()),
                width: 100,
                height: 30,
            },
        )
        .unwrap();

        assert!(out.contains("type: series-log"));
        assert!(out.contains("series: \"A\""));
        assert!(out.contains("## Game Log"));
        assert!(out.contains("|    1 | 2025-04-19 | TBL 2-4 FLA | FLA leads 1-0 |"));
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
                filters: Vec::new(),
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
