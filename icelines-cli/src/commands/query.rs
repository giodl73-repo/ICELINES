//! Phase 5A–5B query engine: icelines query leaders / player / compare
//!
//! Hart.5c.3 — full migration to PlayerView. SortMetric audit pinned in
//! the commit message: every metric reviewed for cast, None policy, and
//! sentinel preservation. Legacy Player paths remain only inside
//! `run_goalies` (Goalie struct lives until 5c.7).

use crate::config::Config;
use anyhow::{bail, Context};
use icelines_core::view_model::context::RecoveryActionKind;
use icelines_core::{
    filter::PlayerFilter,
    model::{Position, Season},
    name::normalize_name,
    position::PositionResolver,
    season_stats::SeasonType,
    stats_catalog::{StatId, StatUnit},
    stats_repository::PlayerView,
    CompareView, Completeness, EmptyKind, EmptyState, LeaderKind, LeadersView, MetricCell,
    MetricUnit, MetricValue, PlayerCardView, RecoveryAction, SemanticToken, SimilarPlayersView,
    SortDirection, SortKey, SortState, SourceKind, SourceState, StatKey, ValuePrecision,
    ViewContext, ViewWarning, ViewWindow, WarningKind,
};
use icelines_fetch::{aggregate, career::load_career, snapshot::SnapshotStore};
use icelines_query::{extract_bio, BioConstraints};

/// QueryB — pre-extract bio atoms (`age<=24`, `country=CAN`, `height>=72`,
/// etc.) from a list of `--filter` strings. Returns the folded
/// `BioConstraints` plus the stat residue (filter strings with bio atoms
/// peeled off the top-level AND chain). Filter strings containing OR/NOT
/// pass through unchanged in the residue — the catalog parser handles
/// or rejects them.
/// Wave 16-19 dispatch helper — partition raw filter strings
/// into new-pipeline plans (handle full Phase Art Ross grammar)
/// vs legacy-residue (handled by `parse_filter_expr`). Helpful
/// errors short-circuit by returning Err on the first one.
///
/// Used by run_leaders, run_player (peers), run_goalies,
/// run_compare. Each call site applies the new_plans via
/// Constraint::matches with an IcelinesProvider after the
/// legacy filter+bio passes.
pub(crate) fn partition_filter_dispatch(
    raw_residue: &[String],
) -> anyhow::Result<(Vec<icelines_query::QueryPlan>, Vec<String>)> {
    let mut plans: Vec<icelines_query::QueryPlan> = Vec::new();
    let mut legacy: Vec<String> = Vec::new();
    for s in raw_residue {
        match icelines_query::parse_query(icelines_query::FilterInput::Cli(s.clone())) {
            Ok(plan) => plans.push(plan),
            Err(es) => {
                let prefer_new = es.iter().any(|e| {
                    matches!(
                        e,
                        icelines_query::ParseError::IncompatiblePredicate { .. }
                            | icelines_query::ParseError::EmptySet { .. }
                            | icelines_query::ParseError::FeatureNotYet { .. }
                            | icelines_query::ParseError::UnknownWindowUnit { .. }
                            | icelines_query::ParseError::ZeroWindowSize { .. }
                            | icelines_query::ParseError::WindowSizeOutOfRange { .. }
                    )
                });
                if prefer_new {
                    let msgs: Vec<String> = es.iter().map(|e| e.to_string()).collect();
                    anyhow::bail!("--filter {s:?}\n  {}", msgs.join("\n  "));
                }
                legacy.push(s.clone());
            }
        }
    }
    Ok((plans, legacy))
}

/// Build an EvalCtx for applying new-pipeline plans against
/// `PlayerView` retained sets in CLI handlers. Reads HOME for
/// the IcelinesProvider's data root, uses SystemClock.
pub(crate) fn build_cli_eval_ctx(
    season: u32,
) -> anyhow::Result<(
    icelines_fetch::query_provider::IcelinesProvider,
    icelines_core::freshness::SystemClock,
)> {
    let home_dir = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let _ = season; // ctx is built per-call by the caller
    let data_root = home_dir.join(".icelines").join("data");
    let provider = icelines_fetch::query_provider::IcelinesProvider::new(data_root);
    let clock = icelines_core::freshness::SystemClock;
    Ok((provider, clock))
}

pub(crate) fn extract_bio_for_cli(raw_filters: &[String]) -> (BioConstraints, Vec<String>) {
    let (atoms, residue) = extract_bio(raw_filters);
    let mut bc = BioConstraints::default();
    for atom in &atoms {
        bc.merge(atom);
    }
    (bc, residue)
}

// ── Sort metric ───────────────────────────────────────────────────────────────

// `SortMetric` lives at `pub(crate)` so it's visible through
// `SortDispatch::Legacy(SortMetric)` (which is `pub` for the same crate).
// Without this the visibility leak fires `clippy::private_interfaces`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SortMetric {
    // All-situations pace
    PtsPace,
    Ppg,
    GPace,
    Gpg,
    // Raw season totals
    Pts,
    Goals,
    Assists,
    Gp,
    // Power play
    PpPtsPace,
    PpGPace,
    PpPts,
    PpGoals,
    // Shorthanded
    ShGPace,
    ShGoals,
    // Other
    GwgPace,
    Gwg,
    // Shot metrics
    ShotsPace,
    Shots,
    ShPct,
    // Two-way
    PlusMinus,
    Toi,
    FoPct,
    // Realtime physical stats
    HitsPace,
    Hits,
    BlocksPace,
    Blocks,
    Takeaways,
    Giveaways,
    Pim,
    // MoneyPuck advanced metrics
    Xg,
    XgPer60,
    CfPct,
    FfPct,
    XgfPct,
    // Year-over-year trend (computed separately from improvement_map)
    Improvement,
}

impl SortMetric {
    fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "pts-pace"    | "pts_pace"    => Ok(Self::PtsPace),
            "ppg"                         => Ok(Self::Ppg),
            "g-pace"      | "g_pace"      => Ok(Self::GPace),
            "gpg"                         => Ok(Self::Gpg),
            "pts"         | "points"      => Ok(Self::Pts),
            "goals"       | "g"           => Ok(Self::Goals),
            "assists"     | "a"           => Ok(Self::Assists),
            "gp"                          => Ok(Self::Gp),
            // PP
            "pp-pts-pace" | "pp_pts_pace" => Ok(Self::PpPtsPace),
            "pp-g-pace"   | "pp_g_pace"   => Ok(Self::PpGPace),
            "pp-pts"      | "pp_pts"      => Ok(Self::PpPts),
            "pp-g"        | "pp_g"        => Ok(Self::PpGoals),
            // SH
            "sh-g-pace"   | "sh_g_pace"   => Ok(Self::ShGPace),
            "sh-g"        | "sh_g"        => Ok(Self::ShGoals),
            // GWG
            "gwg-pace"    | "gwg_pace"    => Ok(Self::GwgPace),
            "gwg"                         => Ok(Self::Gwg),
            // Shots
            "shots-pace"  | "shots_pace"  => Ok(Self::ShotsPace),
            "shots"                       => Ok(Self::Shots),
            "sh-pct"      | "sh_pct"      => Ok(Self::ShPct),
            // Two-way
            "plus-minus"  | "plus_minus"  => Ok(Self::PlusMinus),
            "toi"                         => Ok(Self::Toi),
            "fo-pct"      | "fo_pct"      => Ok(Self::FoPct),
            // Realtime physical stats
            "hits-pace"   | "hits_pace"   => Ok(Self::HitsPace),
            "hits"                        => Ok(Self::Hits),
            "blocks-pace" | "blocks_pace" => Ok(Self::BlocksPace),
            "blocks"                      => Ok(Self::Blocks),
            "takeaways"                   => Ok(Self::Takeaways),
            "giveaways"                   => Ok(Self::Giveaways),
            "pim"                         => Ok(Self::Pim),
            // MoneyPuck advanced metrics
            "xg"                          => Ok(Self::Xg),
            "xg-per-60"   | "xg_per_60"  => Ok(Self::XgPer60),
            "cf-pct"      | "cf_pct"      => Ok(Self::CfPct),
            "ff-pct"      | "ff_pct"      => Ok(Self::FfPct),
            "xgf-pct"     | "xgf_pct"    => Ok(Self::XgfPct),
            "improvement" | "trend"       => Ok(Self::Improvement),
            other => bail!(
                "unknown sort metric '{other}'\n\
                 Pace: pts-pace, ppg, g-pace, gpg, pp-pts-pace, pp-g-pace, sh-g-pace, gwg-pace, shots-pace\n\
                 Totals: pts, goals, assists, gp, pp-pts, pp-g, sh-g, gwg, shots\n\
                 Rates: sh-pct, plus-minus, toi, fo-pct\n\
                 Realtime: hits-pace, hits, blocks-pace, blocks, takeaways, giveaways, pim\n\
                 MoneyPuck: xg, xg-per-60, cf-pct, ff-pct, xgf-pct\n\
                 Trend:     improvement (Y/Y PPG delta; requires 2+ bundled seasons)"
            ),
        }
    }

    /// Sort key: descending. Cold-start / missing-data policy per Tape #5
    /// audit (commit message captures the per-metric table).
    fn sort_value(self, v: &PlayerView<'_>) -> f64 {
        let totals = &v.stats.totals;
        match self {
            Self::PtsPace | Self::Ppg => v.pace_82().unwrap_or(0.0),
            Self::GPace | Self::Gpg => v.goals_per_82().unwrap_or(0.0),
            Self::Pts => totals.points as f64,
            Self::Goals => totals.goals as f64,
            Self::Assists => totals.assists as f64,
            Self::Gp => v.gp() as f64,
            // PP
            Self::PpPtsPace => v.pp_points_per_82().unwrap_or(0.0),
            Self::PpGPace => v.pp_goals_per_82().unwrap_or(0.0),
            Self::PpPts => totals.pp_points as f64,
            Self::PpGoals => totals.pp_goals as f64,
            // SH
            Self::ShGPace => v.sh_goals_per_82().unwrap_or(0.0),
            Self::ShGoals => totals.sh_goals as f64,
            // GWG
            Self::GwgPace => v.gwg_per_82().unwrap_or(0.0),
            Self::Gwg => totals.gwg as f64,
            // Shots
            Self::ShotsPace => v.shots_per_82().unwrap_or(0.0),
            Self::Shots => v.shots() as f64,
            Self::ShPct => totals.shooting_pct.unwrap_or(0.0) as f64,
            // Two-way
            Self::PlusMinus => v.plus_minus() as f64,
            // Tape #5: legacy was f32, new is u32; cast quantization is identical
            // for whole-second values which is what the API returns.
            Self::Toi => totals.toi_per_game_sec.unwrap_or(0) as f64,
            Self::FoPct => totals.faceoff_win_pct.unwrap_or(0.0) as f64,
            // Realtime — Option<u32> in the new model. unwrap_or(0) preserves
            // legacy behavior (cold-start sorts to bottom of zero-tied cluster).
            Self::HitsPace => v.hits_per_82().unwrap_or(0.0),
            Self::Hits => v.hits().unwrap_or(0) as f64,
            Self::BlocksPace => v.blocked_shots_per_82().unwrap_or(0.0),
            Self::Blocks => v.blocked_shots().unwrap_or(0) as f64,
            Self::Takeaways => v.takeaways().unwrap_or(0) as f64,
            Self::Giveaways => v.giveaways().unwrap_or(0) as f64,
            Self::Pim => totals.pim as f64,
            // MoneyPuck
            Self::Xg => v.xg().unwrap_or(0.0),
            Self::XgPer60 => v.xg_per_60().unwrap_or(0.0),
            // Tape #5: 50.0 median sentinel preserved on cold-start (was on
            // Player; PlayerView accessors return Option<f64> so we restore
            // the sentinel here at the sort boundary).
            Self::CfPct => v.cf_pct().unwrap_or(50.0),
            Self::FfPct => v.ff_pct().unwrap_or(50.0),
            Self::XgfPct => v.xgf_pct().unwrap_or(50.0),
            // Improvement: sorted separately via improvement_map; value unused for sort
            Self::Improvement => 0.0,
        }
    }

    fn display(self, v: &PlayerView<'_>, rate: bool) -> String {
        let totals = &v.stats.totals;
        match self {
            Self::PtsPace => match v.pace_82() {
                Some(p) => {
                    if rate {
                        format!("{:.3}", p / 82.0)
                    } else {
                        format!("{p:.1}")
                    }
                }
                None => "—".to_owned(),
            },
            Self::Ppg => match v.pace_82() {
                Some(p) => format!("{:.3}", p / 82.0),
                None => "—".to_owned(),
            },
            Self::GPace => match v.goals_per_82() {
                Some(g) => {
                    if rate {
                        format!("{:.3}", g / 82.0)
                    } else {
                        format!("{g:.1}")
                    }
                }
                None => "—".to_owned(),
            },
            Self::Gpg => match v.goals_per_82() {
                Some(g) => format!("{:.3}", g / 82.0),
                None => "—".to_owned(),
            },
            Self::Pts => totals.points.to_string(),
            Self::Goals => totals.goals.to_string(),
            Self::Assists => totals.assists.to_string(),
            Self::Gp => v.gp().to_string(),
            // PP
            Self::PpPtsPace => format!("{:.1}", v.pp_points_per_82().unwrap_or(0.0)),
            Self::PpGPace => format!("{:.1}", v.pp_goals_per_82().unwrap_or(0.0)),
            Self::PpPts => totals.pp_points.to_string(),
            Self::PpGoals => totals.pp_goals.to_string(),
            // SH
            Self::ShGPace => format!("{:.1}", v.sh_goals_per_82().unwrap_or(0.0)),
            Self::ShGoals => totals.sh_goals.to_string(),
            // GWG
            Self::GwgPace => format!("{:.1}", v.gwg_per_82().unwrap_or(0.0)),
            Self::Gwg => totals.gwg.to_string(),
            // Shots
            Self::ShotsPace => format!("{:.1}", v.shots_per_82().unwrap_or(0.0)),
            Self::Shots => v.shots().to_string(),
            Self::ShPct => totals
                .shooting_pct
                .map(|val| format!("{:.1}%", val * 100.0))
                .unwrap_or_else(|| "—".to_owned()),
            // Two-way
            Self::PlusMinus => {
                let pm = v.plus_minus();
                if pm >= 0 {
                    format!("+{pm}")
                } else {
                    pm.to_string()
                }
            }
            Self::Toi => v.toi_mmss().unwrap_or_else(|| "—".to_owned()),
            Self::FoPct => totals
                .faceoff_win_pct
                .map(|val| format!("{:.1}%", val * 100.0))
                .unwrap_or_else(|| "—".to_owned()),
            // Realtime
            Self::HitsPace => format!("{:.1}", v.hits_per_82().unwrap_or(0.0)),
            Self::Hits => v.hits().unwrap_or(0).to_string(),
            Self::BlocksPace => format!("{:.1}", v.blocked_shots_per_82().unwrap_or(0.0)),
            Self::Blocks => v.blocked_shots().unwrap_or(0).to_string(),
            Self::Takeaways => v.takeaways().unwrap_or(0).to_string(),
            Self::Giveaways => v.giveaways().unwrap_or(0).to_string(),
            Self::Pim => totals.pim.to_string(),
            // MoneyPuck — display path uses None → "—" (no median sentinel here;
            // the sentinel is sort-only).
            Self::Xg => v
                .xg()
                .map(|val| format!("{val:.2}"))
                .unwrap_or_else(|| "—".to_owned()),
            Self::XgPer60 => v
                .xg_per_60()
                .map(|val| format!("{val:.2}"))
                .unwrap_or_else(|| "—".to_owned()),
            Self::CfPct => v
                .cf_pct()
                .map(|val| format!("{val:.1}%"))
                .unwrap_or_else(|| "—".to_owned()),
            Self::FfPct => v
                .ff_pct()
                .map(|val| format!("{val:.1}%"))
                .unwrap_or_else(|| "—".to_owned()),
            Self::XgfPct => v
                .xgf_pct()
                .map(|val| format!("{val:.1}%"))
                .unwrap_or_else(|| "—".to_owned()),
            Self::Improvement => "—".to_owned(), // displayed by special-case handler
        }
    }

    fn header(self, rate: bool) -> &'static str {
        match self {
            Self::PtsPace => {
                if rate {
                    "PPG"
                } else {
                    "Pts/82"
                }
            }
            Self::Ppg => "PPG",
            Self::GPace => {
                if rate {
                    "GPG"
                } else {
                    "G/82"
                }
            }
            Self::Gpg => "GPG",
            Self::Pts => "Pts",
            Self::Goals => "Goals",
            Self::Assists => "Assists",
            Self::Gp => "GP",
            Self::PpPtsPace => "PP-Pts/82",
            Self::PpGPace => "PP-G/82",
            Self::PpPts => "PP-Pts",
            Self::PpGoals => "PP-Goals",
            Self::ShGPace => "SH-G/82",
            Self::ShGoals => "SH-Goals",
            Self::GwgPace => "GWG/82",
            Self::Gwg => "GWG",
            Self::ShotsPace => "Shots/82",
            Self::Shots => "Shots",
            Self::ShPct => "SH%",
            Self::PlusMinus => "+/-",
            Self::Toi => "TOI/g",
            Self::FoPct => "FO%",
            // Realtime
            Self::HitsPace => "Hits/82",
            Self::Hits => "Hits",
            Self::BlocksPace => "Blk/82",
            Self::Blocks => "Blocks",
            Self::Takeaways => "TkA",
            Self::Giveaways => "GvA",
            Self::Pim => "PIM",
            // MoneyPuck
            Self::Xg => "ixG",
            Self::XgPer60 => "ixG/60",
            Self::CfPct => "CF%",
            Self::FfPct => "FF%",
            Self::XgfPct => "xGF%",
            Self::Improvement => "Δ PPG",
        }
    }
}

// ── Phase Lindsay L.5.1 — `--sort` dispatch ──────────────────────────────────
//
// `SortDispatch` is the unified `--sort` dispatcher. Legacy strings
// (the ~37 `pts-pace` / `ppg` / `g-pace` style values) keep going through
// `SortMetric` and its existing `display`/`header`/`sort_value` paths
// — that's the byte-stable surface the L.3.0 stdout-golden fence locks.
//
// Any string that is NOT a legacy alias gets a second chance via
// `StatId::from_cli_key`. On match, the catalog dispatches `read()`,
// `sort_cmp()`, `label()`, `unit()` to render. Legacy-first precedence
// preserves byte-equality for the fence; catalog-only stats become
// available additively.
//
// `Improvement` stays Legacy-only — it's a derived comparison metric
// computed from the Y/Y improvement_map, not a stat read.
#[derive(Debug, Clone, Copy)]
pub enum SortDispatch {
    Legacy(SortMetric),
    Catalog(StatId),
}

impl SortDispatch {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        // Legacy first: byte-stable fence semantics for ~37 strings.
        if let Ok(m) = SortMetric::parse(s) {
            return Ok(Self::Legacy(m));
        }
        // Catalog fallback: any StatId::cli_key.
        if let Some(sid) = StatId::from_cli_key(s) {
            return Ok(Self::Catalog(sid));
        }
        // Neither matched — emit the legacy help message so users see
        // the supported set. Catalog keys self-document via `--help`.
        SortMetric::parse(s).map(Self::Legacy)
    }

    pub fn is_improvement(self) -> bool {
        matches!(self, Self::Legacy(SortMetric::Improvement))
    }

    /// Sort comparator: `(value desc/asc-by-unit, nhl_id asc)`. Catalog
    /// path delegates to `StatId::sort_cmp` (AI-06). Legacy path
    /// preserves the post-L.3.2 deterministic comparator.
    pub fn cmp(self, a: &PlayerView<'_>, b: &PlayerView<'_>) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match self {
            Self::Legacy(m) => m
                .sort_value(b)
                .partial_cmp(&m.sort_value(a))
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.identity.id.0.cmp(&b.identity.id.0)),
            Self::Catalog(sid) => sid.sort_cmp(a, b),
        }
    }

    pub fn header(self, rate: bool) -> String {
        match self {
            Self::Legacy(m) => m.header(rate).to_owned(),
            Self::Catalog(sid) => sid.short_label().to_owned(),
        }
    }

    pub fn display(self, v: &PlayerView<'_>, rate: bool) -> String {
        match self {
            Self::Legacy(m) => m.display(v, rate),
            Self::Catalog(sid) => format_catalog_cell(sid, v),
        }
    }

    /// Used by `position_percentile` for the "% of peers better than
    /// target" calculation. None reads sort to the bottom (NEG_INFINITY)
    /// for higher_is_better stats; INFINITY for inverted (Gaa).
    pub fn sort_value(self, v: &PlayerView<'_>) -> f64 {
        match self {
            Self::Legacy(m) => m.sort_value(v),
            Self::Catalog(sid) => sid.read(v).unwrap_or(if sid.higher_is_better() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }),
        }
    }
}

/// Phase Lindsay L.5.1 — render a catalog cell value for the leaders
/// table. Mirrors `tui::screens::player::render_career_cell` (L.4.3) so
/// `query leaders --sort <cli_key>` and the TUI career table speak the
/// same value-formatting language. Output cells are short — the column
/// is 10 chars wide.
fn format_catalog_cell(sid: StatId, view: &PlayerView<'_>) -> String {
    match sid.read(view) {
        None => "—".to_owned(),
        Some(v) => match sid.unit() {
            StatUnit::Count => format!("{}", v as i64),
            StatUnit::Seconds => {
                let secs = v as u64;
                if secs < 3600 {
                    format!("{}:{:02}", secs / 60, secs % 60)
                } else {
                    format!("{}m", secs / 60)
                }
            }
            StatUnit::Pct => format!("{:.1}%", v * 100.0),
            StatUnit::Per60 | StatUnit::Rate => format!("{v:.2}"),
            StatUnit::Inverted => format!("{v:.2}"),
        },
    }
}

// ── Public arg structs ────────────────────────────────────────────────────────

pub struct LeadersArgs {
    pub pos: Option<String>,
    pub team: Option<String>,
    pub age_min: Option<u8>,
    pub age_max: Option<u8>,
    pub nationality: Option<String>,
    pub draft_year: Option<u16>,
    pub round: Option<u8>,
    pub draft_pick_max: Option<u16>,
    pub undrafted: bool,
    pub rookie: bool,
    pub handedness: Option<String>,
    pub ppg_min: Option<f64>,
    pub gp_min: Option<u32>,
    pub gp_max: Option<u32>,
    // New demographic filters
    pub birth_province: Option<String>,
    // New statistical threshold filters
    pub toi_min: Option<f32>, // minutes per game (converted to seconds internally)
    pub plus_minus_min: Option<i32>,
    pub shots_pg_min: Option<f32>,
    // Contract filters
    pub ufa: bool,
    pub rfa: bool,
    pub elc: bool,
    pub expiry_year: Option<u16>,
    /// Aggregate stats across this many bundled seasons (1 = current only)
    pub seasons: u8,
    /// Specific historical season override (Phase 8f). Conflicts with `seasons > 1`.
    pub season: Option<String>,
    /// Hart.6.9 — season-type window (Regular default, Playoff opt-in).
    /// Conflicts with `seasons > 1` (the aggregate path is regular-only
    /// today; folding playoff stats into a multi-season aggregate would
    /// blur per-game vs per-series counting).
    pub season_type: SeasonType,
    pub sort: String,
    pub top: usize,
    pub rate: bool,
    pub percentiles: bool,
    pub json: bool,
    pub json_envelope: bool,
    pub csv: bool,
    pub out: Option<std::path::PathBuf>,
    /// Phase Lindsay L.3.1 — generic stat filters. Each string is parsed
    /// via `icelines_core::stats_catalog::parse_filter` and added to
    /// `PlayerFilter.stat_filters`; `normalize_stat_filters` runs before
    /// apply.
    pub filters: Vec<String>,
}

// ── Phase Art Ross A.5 — `--explain` ──────────────────────────────────────────

/// Parse the user's `--filter` arguments + print the resulting
/// `QueryPlan` tree, data requirements, and estimated cost. Exits
/// without running the query (no player data loaded). When
/// `--json` is also set, emits the `explain.v1` envelope shape
/// per the spec (frozen v1; additive changes only).
pub fn run_explain(filters: &[String], json: bool) -> anyhow::Result<()> {
    use icelines_query::{parse_query, FilterInput};

    let mut plans: Vec<icelines_query::QueryPlan> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for raw in filters {
        match parse_query(FilterInput::Cli(raw.clone())) {
            Ok(plan) => plans.push(plan),
            Err(es) => {
                for e in es {
                    errors.push(format!("--filter {raw:?}: {e}"));
                }
            }
        }
    }

    if !errors.is_empty() {
        anyhow::bail!("filter parse errors:\n{}", errors.join("\n"));
    }

    if filters.is_empty() {
        if json {
            println!(
                r#"{{"schema_version":"explain.v1","route":"leaders.explain","data":{{"plans":[]}},"meta":{{"note":"no filters provided"}}}}"#
            );
        } else {
            println!("(no --filter arguments to explain)");
        }
        return Ok(());
    }

    if json {
        emit_explain_json(&plans, filters);
    } else {
        emit_explain_text(&plans, filters);
    }
    Ok(())
}

fn emit_explain_text(plans: &[icelines_query::QueryPlan], filters: &[String]) {
    println!("QUERY PLAN  (explain.v1)");
    println!("{}", "═".repeat(66));
    for (idx, plan) in plans.iter().enumerate() {
        let raw = &filters[idx];
        println!("--filter {raw:?}");
        for line in plan.explain().lines() {
            println!("  {line}");
        }
        let req = plan.requirements();
        let needs_provider = plan.root.needs_provider();
        println!();
        println!("  DATA REQUIREMENTS");
        if req.reports_needed.is_empty() && !needs_provider {
            println!("    (bio-only — no per-season report needed)");
        } else {
            for r in &req.reports_needed {
                println!("    ✓ {r:<16} (active season)");
            }
            if needs_provider {
                println!(
                    "    ↓ provider           (sliding-window / career-aggregate / cross-league)"
                );
            }
        }
        println!();
    }
    println!("{}", "═".repeat(66));
}

fn emit_explain_json(plans: &[icelines_query::QueryPlan], filters: &[String]) {
    let plans_arr: Vec<serde_json::Value> = plans
        .iter()
        .zip(filters.iter())
        .map(|(plan, raw)| {
            let req = plan.requirements();
            serde_json::json!({
                "filter_input": raw,
                "plan_tree_text": plan.explain(),
                "needs_provider": plan.root.needs_provider(),
                "requirements": {
                    "seasons_needed": req.seasons_needed,
                    "reports_needed": req.reports_needed,
                    "boxscore_seasons_needed": req.boxscore_seasons_needed,
                    "career_pids_needed": req.career_pids_needed,
                },
            })
        })
        .collect();
    let envelope = serde_json::json!({
        "schema_version": "explain.v1",
        "route": "leaders.explain",
        "data": {
            "plans": plans_arr,
        },
        "meta": {
            "note": "explain.v1 is frozen — additive changes only; breaking changes ship as explain.v2",
        },
    });
    println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
}

// ── icelines query leaders ────────────────────────────────────────────────────

pub async fn run_leaders(args: LeadersArgs) -> anyhow::Result<()> {
    if args.out.is_some() && !(args.json || args.json_envelope || args.csv) {
        anyhow::bail!("--out requires --json, --json-envelope, or --csv");
    }
    // Phase Lindsay L.5.1 — try legacy-first, fall through to catalog.
    let metric = SortDispatch::parse(&args.sort)?;

    if args.season.is_some() && args.seasons > 1 {
        anyhow::bail!(
            "--season and --seasons N > 1 are mutually exclusive.\n  \
             Use --season YYYYZZZZ for a single historical season,\n  \
             or --seasons N for an N-season aggregate of recent seasons."
        );
    }

    // Hart.6.9: aggregate path is regular-only. Reject the combo cleanly
    // rather than silently falling back to regular for the playoff case.
    if args.season_type == SeasonType::Playoff && args.seasons > 1 {
        anyhow::bail!(
            "--type playoff cannot be combined with --seasons N > 1.\n  \
             The N-season aggregate path is regular-season only today; \
             folding playoff series counts into per-game ratios would \
             produce misleading numbers. Use --season YYYYZZZZ --type \
             playoff for a single playoff window."
        );
    }

    // Load player pool — single season (default or overridden) or N-season aggregate.
    // Both paths produce a `(StatsRepository, Season)` pair so the iterator code
    // below is uniform.
    let (repo, season_key, season_type) = if args.seasons > 1 {
        let (r, s) = aggregate::load_aggregate_into_repo(args.seasons as usize);
        (r, s, SeasonType::Regular)
    } else {
        let (outcome, season, ty) = crate::commands::players::load_repo_for_season(
            args.season.as_deref(),
            Some(args.season_type),
        )?;
        (outcome.repo, season, ty)
    };
    let all_views: Vec<PlayerView<'_>> = repo.skaters(season_key, season_type).collect();

    let mut filter = PlayerFilter::new();
    if let Some(ref p) = args.pos {
        if p.is_empty() {
            eprintln!("  Hint: --pos \"\" has no effect — omit the flag to include all positions.");
        } else {
            filter.positions = Some(parse_positions(p));
        }
    }
    if let Some(t) = args.team {
        filter.teams = Some(vec![t.to_uppercase()]);
    }
    filter.age_min = args.age_min;
    filter.age_max = args.age_max;
    filter.nationalities = args.nationality.map(|n| vec![n.to_uppercase()]);
    filter.draft_years = args.draft_year.map(|y| vec![y]);
    filter.draft_rounds = args.round.map(|r| vec![r]);
    filter.draft_pick_max = args.draft_pick_max;
    filter.undrafted = if args.undrafted { Some(true) } else { None };
    filter.rookie_only = if args.rookie { Some(true) } else { None };
    filter.handedness = args.handedness;
    filter.ppg_min = args.ppg_min;
    filter.gp_min = args.gp_min;
    filter.toi_min_sec = args.toi_min.map(|m| m * 60.0);
    filter.plus_minus_min = args.plus_minus_min;
    filter.shots_pg_min = args.shots_pg_min;
    filter.birth_provinces = args
        .birth_province
        .map(|bp| bp.split(',').map(|s| s.trim().to_uppercase()).collect());

    // QueryB — pre-extract bio atoms (`age<=24`, `country=CAN`, `height>=72`)
    // from each --filter's top-level AND chain. Stat residue continues
    // through the existing parse_filter_expr loop; BioConstraints is
    // applied as a post-step. Filters containing OR/NOT bypass the
    // splitter and pass whole-cloth to the catalog parser.
    let (bio_constraints, stat_residue) = extract_bio_for_cli(&args.filters);

    // Phase Art Ross A.2.4 / Wave 16 fix — route every
    // successfully-parsed plan through the new pipeline. This
    // covers BOTH:
    //   - sliding-window / career / league atoms (need provider)
    //   - new operators on existing atoms (`g<5`, `country IN (...)`,
    //     `age BETWEEN 22 AND 28`, `country LIKE "CA*"`)
    // The legacy `parse_filter_expr` only understands `>=`/`<=`/`==`/`=` —
    // sending it `g<5` produces a BadNumber error. Wave 16 found
    // this routing bug; the fix is to use the new parser as
    // the primary path and only fall through to legacy when the
    // new parser ITSELF rejects the input (which means the user
    // typed something neither parser handles — surface that
    // error from the legacy path which has the more battle-tested
    // diagnostics).
    let mut new_pipeline_plans: Vec<icelines_query::QueryPlan> = Vec::new();
    let mut legacy_residue: Vec<String> = Vec::new();
    for raw in &stat_residue {
        match icelines_query::parse_query(icelines_query::FilterInput::Cli(raw.to_string())) {
            Ok(plan) => new_pipeline_plans.push(plan),
            Err(es) => {
                // Wave 16 fix — when the new parser errors with
                // a HELPFUL diagnostic (IncompatiblePredicate
                // for `g IN (...)` suggests BETWEEN, etc.), don't
                // fall through to the legacy parser which gives
                // a less useful "no op" error. Surface the new
                // parser's error directly.
                let prefer_new_error = es.iter().any(|e| {
                    matches!(
                        e,
                        icelines_query::ParseError::IncompatiblePredicate { .. }
                            | icelines_query::ParseError::EmptySet { .. }
                            | icelines_query::ParseError::FeatureNotYet { .. }
                            | icelines_query::ParseError::UnknownWindowUnit { .. }
                            | icelines_query::ParseError::ZeroWindowSize { .. }
                            | icelines_query::ParseError::WindowSizeOutOfRange { .. }
                    )
                });
                if prefer_new_error {
                    let msgs: Vec<String> = es.iter().map(|e| e.to_string()).collect();
                    anyhow::bail!("--filter {raw:?}\n  {}", msgs.join("\n  "));
                }
                legacy_residue.push(raw.clone());
            }
        }
    }

    // Phase Lindsay L.3.1 — generic stat filters. Each --filter flag
    // routes through `parse_filter` (which gates NaN/inf at construction
    // and rejects malformed grammar with a 7-variant error). Multiple
    // --filter flags accumulate (implicit AND); `normalize_stat_filters`
    // collapses Min+Min/Max+Max to tightest bounds before apply.
    for raw in &legacy_residue {
        // Filter.OR — try the boolean grammar (AND / OR / NOT / parens)
        // first. Bare atoms (e.g. "g>=50") still produce a single
        // StatFilter and route through stat_filters so normalization
        // (Min+Min → tightest) keeps working.
        let expr = icelines_core::stats_catalog::parse_filter_expr(raw)
            .with_context(|| format!("--filter {raw:?}"))?;
        match expr.as_atom() {
            Some(atom) => filter.stat_filters.push(*atom),
            None => filter.expr_filters.push(expr),
        }
    }
    filter.normalize_stat_filters();

    let mut matched: Vec<PlayerView<'_>> = filter.apply_views(all_views.iter().copied());

    // QueryB — apply bio constraints from the filter grammar after
    // PlayerFilter. The CLI's --age-min/--age-max already flow
    // through PlayerFilter; bio atoms from --filter use Hockey-
    // Reference's Jan-31 age convention (slightly different but
    // documented). Atoms for keys without dedicated CLI flags
    // (height, weight, country, draft year range) get applied here
    // for the first time.
    if bio_constraints.is_active() {
        matched.retain(|v| bio_constraints.matches(v, season_key.0));
    }

    // Phase Art Ross A.2.4 — apply sliding-window filters via the
    // new pipeline. The IcelinesProvider walks the boxscore
    // manifest for each player on demand. Compound atoms (Bio +
    // SlidingWindow in one filter) work because Constraint::matches
    // handles every variant — but bio atoms in those filters are
    // ALSO applied via the legacy bio_constraints path above (the
    // bio extractor doesn't peel sliding-window filters because
    // they contain `.last`). Result: bio constraints apply twice
    // for `age<=24 AND g.last10g>=5` — harmless (idempotent
    // intersection) but worth noting for the A.2.7 surface swap.
    if !new_pipeline_plans.is_empty() {
        let home_dir = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
        let data_root = home_dir.join(".icelines").join("data");
        let provider = icelines_fetch::query_provider::IcelinesProvider::new(data_root);
        let clock = icelines_core::freshness::SystemClock;
        let ctx = icelines_query::EvalCtx::from_clock(
            &provider,
            icelines_query::StrictMode::Off,
            /*no_fetch=*/ false,
            &clock,
            season_key.0,
        );
        for plan in &new_pipeline_plans {
            matched.retain(|v| plan.root.matches(v, &ctx));
        }
    }

    // gp_max (not in PlayerFilter — inline here)
    if let Some(gp_max) = args.gp_max {
        matched.retain(|v| v.gp() <= gp_max);
    }

    // Contract filters
    let wants_contract = args.ufa || args.rfa || args.elc || args.expiry_year.is_some();
    if args.ufa {
        matched.retain(|v| {
            v.contract_expiry_type()
                .map(|t| t.eq_ignore_ascii_case("UFA"))
                .unwrap_or(false)
        });
    }
    if args.rfa {
        matched.retain(|v| {
            v.contract_expiry_type()
                .map(|t| t.eq_ignore_ascii_case("RFA"))
                .unwrap_or(false)
        });
    }
    if args.elc {
        matched.retain(|v| {
            v.contract_expiry_type()
                .map(|t| t.eq_ignore_ascii_case("ELC"))
                .unwrap_or(false)
        });
    }
    if let Some(yr) = args.expiry_year {
        matched.retain(|v| v.contract_expiry_year() == Some(yr));
    }
    if wants_contract && matched.is_empty() {
        eprintln!("  Hint: no contract data found. Run `icelines fetch contracts` to enable UFA/RFA/ELC filtering.");
    }
    if matched.is_empty() {
        if let Some(ref nats) = filter.nationalities {
            eprintln!("  Hint: no players found for nationality code(s) {:?}. Use ISO-3166 alpha-3 (e.g. CAN, USA, SWE, FIN, RUS, CZE, SVK, DEU).", nats);
        }
        // Phase Lindsay L.3.1 — KEEL pre-commit fix: surface a hint when
        // an empty result set traces back to an unloaded Lindsay substruct
        // rather than a real "no matches" outcome. Reads from realtime /
        // possession / goalsForAgainst / xG return None pre-bundling, so
        // any --filter on those StatIds returns 0 silently. Hint user
        // toward `fetch report` (the L.1 path that populates per-window
        // files) so the empty result doesn't read as a contradiction
        // ("Goal-scoring leaders have ZERO hits?").
        let has_lindsay_filter = filter.stat_filters.iter().any(|f| {
            use icelines_core::stats_catalog::StatCategory::*;
            matches!(f.stat.category(), Possession | OnIceGoals | TimeOnIce)
                || matches!(
                    f.stat,
                    icelines_core::stats_catalog::StatId::Hits
                        | icelines_core::stats_catalog::StatId::BlockedShots
                        | icelines_core::stats_catalog::StatId::Takeaways
                        | icelines_core::stats_catalog::StatId::Giveaways
                        | icelines_core::stats_catalog::StatId::MissedShots
                        | icelines_core::stats_catalog::StatId::HitsPer60
                        | icelines_core::stats_catalog::StatId::BlockedShotsPer60
                        | icelines_core::stats_catalog::StatId::TakeawaysPer60
                        | icelines_core::stats_catalog::StatId::GiveawaysPer60
                )
        });
        if has_lindsay_filter {
            eprintln!(
                "  Hint: Lindsay-tier stat data (realtime / possession / TOI splits / xG) \
                 isn't fully bundled yet. Run `icelines fetch report --kind <kind> \
                 --season {}` to populate per-window files, or wait for L.7 historical bundling.",
                args.season
                    .as_deref()
                    .unwrap_or(icelines_core::CURRENT_SEASON_STR),
            );
        }
    }

    // Improvement sort requires the Y/Y delta map.
    if metric.is_improvement() {
        let imp_map = aggregate::load_improvement_map();
        matched.sort_by(|a, b| {
            let da = imp_map
                .get(&a.identity.id.0)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            let db = imp_map
                .get(&b.identity.id.0)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            // Phase Lindsay L.3.2 / AI-06 universal tiebreak: stable
            // `nhl_id asc` so tied values produce deterministic order
            // across process invocations. Pre-Lindsay this was
            // HashMap-iteration random.
            db.partial_cmp(&da)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.identity.id.0.cmp(&b.identity.id.0))
        });
        let total_matched = matched.len();
        let results: Vec<PlayerView<'_>> = matched.iter().copied().take(args.top).collect();
        let mut leaders_view = leaders_view_from_results(
            &results,
            metric,
            args.rate,
            args.seasons,
            season_key,
            season_type,
        );

        apply_leaders_warning_state(&mut leaders_view, args.pos.as_deref());

        if args.json_envelope {
            return emit_query_output(
                args.out.as_deref(),
                &leaders_json_envelope(
                    &leaders_view,
                    total_matched,
                    args.top,
                    &args.filters,
                    args.pos.as_deref(),
                    &args.sort,
                ),
            );
        }
        if args.json {
            return emit_query_output(
                args.out.as_deref(),
                &leaders_json(&leaders_view, total_matched, args.top, &args.filters),
            );
        }
        if args.csv {
            return emit_query_output(
                args.out.as_deref(),
                &leaders_csv(
                    &leaders_view,
                    total_matched,
                    args.top,
                    &args.sort,
                    &args.filters,
                ),
            );
        }
        println!("{}", leaders_context_line(&leaders_view));
        println!();
        if let Some(disclosure) = bundled_depth_disclosure(args.seasons as usize) {
            println!("{disclosure}");
            println!();
        }
        print_improvement_table(&results, &imp_map, args.top, total_matched, args.seasons);
        return Ok(());
    }

    // Warn if a realtime/MoneyPuck metric has no data (all None/zero).
    // L.5.1: data-missing fallback only fires for the legacy realtime/
    // MoneyPuck SortMetric variants. Catalog-path stats (any cli_key
    // not in the legacy alias map) sort with `None` last per AI-06 —
    // no fallback, the user sees correct output without a guess.
    let data_missing = match metric {
        SortDispatch::Legacy(SortMetric::HitsPace)
        | SortDispatch::Legacy(SortMetric::Hits)
        | SortDispatch::Legacy(SortMetric::BlocksPace)
        | SortDispatch::Legacy(SortMetric::Blocks)
        | SortDispatch::Legacy(SortMetric::Takeaways)
        | SortDispatch::Legacy(SortMetric::Giveaways)
        | SortDispatch::Legacy(SortMetric::Pim) => matched.iter().all(|v| {
            v.hits().unwrap_or(0) == 0
                && v.blocked_shots().unwrap_or(0) == 0
                && v.takeaways().unwrap_or(0) == 0
        }),
        SortDispatch::Legacy(SortMetric::Xg)
        | SortDispatch::Legacy(SortMetric::XgPer60)
        | SortDispatch::Legacy(SortMetric::CfPct)
        | SortDispatch::Legacy(SortMetric::FfPct)
        | SortDispatch::Legacy(SortMetric::XgfPct) => matched
            .iter()
            .all(|v| v.xg().is_none() && v.cf_pct().is_none()),
        _ => false,
    };
    if data_missing {
        eprintln!("  Warning: no realtime/MoneyPuck data loaded for sort '{}'. Run `icelines fetch` to download it.", args.sort);
        eprintln!("  Results below are sorted by Pts/82 as a fallback.");
        matched.sort_by(|a, b| {
            let sa = a.pace_82().unwrap_or(0.0);
            let sb = b.pace_82().unwrap_or(0.0);
            // Phase Lindsay L.3.2 / AI-06 universal tiebreak.
            sb.partial_cmp(&sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.identity.id.0.cmp(&b.identity.id.0))
        });
    } else {
        matched.sort_by(|a, b| metric.cmp(a, b));
    }
    let total_matched = matched.len();
    let results: Vec<PlayerView<'_>> = matched.into_iter().take(args.top).collect();
    let mut leaders_view = leaders_view_from_results(
        &results,
        metric,
        args.rate,
        args.seasons,
        season_key,
        season_type,
    );

    apply_leaders_warning_state(&mut leaders_view, args.pos.as_deref());

    if args.json_envelope {
        return emit_query_output(
            args.out.as_deref(),
            &leaders_json_envelope(
                &leaders_view,
                total_matched,
                args.top,
                &args.filters,
                args.pos.as_deref(),
                &args.sort,
            ),
        );
    }
    if args.json {
        return emit_query_output(
            args.out.as_deref(),
            &leaders_json(&leaders_view, total_matched, args.top, &args.filters),
        );
    }
    if args.csv {
        return emit_query_output(
            args.out.as_deref(),
            &leaders_csv(
                &leaders_view,
                total_matched,
                args.top,
                &args.sort,
                &args.filters,
            ),
        );
    }

    let percentiles: Vec<Option<u8>> = if args.percentiles {
        results
            .iter()
            .map(|v| position_percentile(&all_views, v, metric))
            .collect()
    } else {
        vec![None; results.len()]
    };

    leaders_table(
        &leaders_view,
        &percentiles,
        args.top,
        total_matched,
        &args.sort,
        &args.filters,
        args.seasons,
    );
    Ok(())
}

fn leaders_view_from_results(
    views: &[PlayerView<'_>],
    metric: SortDispatch,
    rate: bool,
    seasons: u8,
    season: Season,
    season_type: SeasonType,
) -> LeadersView {
    let col = leader_column_label(metric, rate, seasons);
    let mut view = LeadersView::from_player_views_with_primary(
        leaders_view_context(season, season_type),
        LeaderKind::Skaters,
        views.iter().copied(),
        |v| MetricCell {
            key: StatKey::from("leader_metric"),
            label: col.clone(),
            value: MetricValue::Text(metric.display(v, rate)),
            unit: MetricUnit::None,
            precision: ValuePrecision::Raw,
            token: Some(SemanticToken::DecisionHighlight),
        },
    );
    view.sort = Some(SortState {
        key: SortKey::from("leader_metric"),
        label: col,
        direction: SortDirection::Desc,
    });
    view
}

fn leaders_view_context(season: Season, season_type: SeasonType) -> ViewContext {
    let mut context = ViewContext::new(ViewWindow::new(season, season_type));
    context
        .source_state
        .push(SourceState::complete(SourceKind::Roster));
    context
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

fn leader_column_label(metric: SortDispatch, rate: bool, seasons: u8) -> String {
    if seasons > 1 {
        format!("{} ({}yr)", metric.header(rate), seasons)
    } else {
        metric.header(rate)
    }
}

fn leaders_table(
    view: &LeadersView,
    percentiles: &[Option<u8>],
    top: usize,
    total: usize,
    sort: &str,
    active_filters: &[String],
    seasons: u8,
) {
    let col = view
        .sort
        .as_ref()
        .map(|sort| sort.label.as_str())
        .or_else(|| view.rows.first().map(|row| row.primary.label.as_str()))
        .unwrap_or("Value");
    let show_pct = percentiles.iter().any(|p| p.is_some());

    println!("{}", leaders_context_line(view));
    println!(
        "{}",
        leaders_result_line(view, total, top, sort, active_filters)
    );
    println!();

    let warning_empty_lines = leaders_warning_empty_lines(view);
    if !warning_empty_lines.is_empty() {
        for line in warning_empty_lines {
            println!("{line}");
        }
        println!();
    }

    if let Some(disclosure) = bundled_depth_disclosure(seasons as usize) {
        println!("{disclosure}");
        println!();
    }

    if show_pct {
        println!(
            "{:<4} {:<24} {:<5} {:<4} {:<4} {:<10} {:<6}",
            "Rank", "Player", "Team", "Pos", "GP", col, "Pctl"
        );
        println!("{}", "─".repeat(61));
        for (row, pct) in view.rows.iter().zip(percentiles.iter()) {
            let val = leader_primary_text(row);
            let pct_s = pct
                .map(|x| format!("{x}{}", ordinal(x)))
                .unwrap_or_else(|| "—".to_owned());
            println!(
                "{:<4} {:<24} {:<5} {:<4} {:<4} {:<10} {:<6}",
                row.rank,
                row.display_name,
                leader_team(row),
                row.position.abbreviation(),
                leader_gp(row),
                val,
                pct_s
            );
        }
    } else {
        println!(
            "{:<4} {:<24} {:<5} {:<4} {:<4} {:<10}",
            "Rank", "Player", "Team", "Pos", "GP", col
        );
        println!("{}", "─".repeat(55));
        for row in &view.rows {
            let val = leader_primary_text(row);
            println!(
                "{:<4} {:<24} {:<5} {:<4} {:<4} {:<10}",
                row.rank,
                row.display_name,
                leader_team(row),
                row.position.abbreviation(),
                leader_gp(row),
                val
            );
        }
    }
    println!("\n{total} matched, showing {}.", view.rows.len().min(top));
}

fn leaders_context_line(view: &LeadersView) -> String {
    let window = view.context.window;
    let (source_kind, source_completeness) = leaders_source_state_labels(view);
    format!(
        "Context: {} {} | source {} {}",
        window.season.0,
        window.season_type.label(),
        source_kind,
        source_completeness
    )
}

fn leaders_result_line(
    view: &LeadersView,
    total: usize,
    top: usize,
    sort: &str,
    active_filters: &[String],
) -> String {
    format!(
        "Result: total {} | returned {} | top {} | sort {} | active_filters {}",
        total,
        view.rows.len(),
        top,
        sort,
        leaders_active_filters_label(active_filters)
    )
}

fn leaders_warning_empty_lines(view: &LeadersView) -> Vec<String> {
    let mut lines = Vec::new();
    for warning in &view.warnings {
        lines.push(format!(
            "Warning: {} | {}",
            warning_kind_label(warning.kind),
            warning.message
        ));
    }
    if let Some(empty) = &view.empty_state {
        lines.push(format!(
            "Empty: {} | {}",
            empty_kind_label(empty.kind),
            empty.title
        ));
        if let Some(detail) = &empty.detail {
            lines.push(format!("Detail: {detail}"));
        }
        for recovery in &empty.recovery {
            lines.push(format!("Recovery: {}", recovery_action_label(recovery)));
        }
    } else {
        for warning in &view.warnings {
            for recovery in &warning.recovery {
                lines.push(format!("Recovery: {}", recovery_action_label(recovery)));
            }
        }
    }
    lines
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

fn leaders_source_state_labels(view: &LeadersView) -> (&'static str, &'static str) {
    view.context
        .source_state
        .first()
        .map(|state| {
            (
                source_kind_label(state.source),
                completeness_label(state.state),
            )
        })
        .unwrap_or(("unknown", "unavailable"))
}

fn source_kind_label(source: SourceKind) -> &'static str {
    match source {
        SourceKind::Roster => "roster",
        SourceKind::Schedule => "schedule",
        SourceKind::Scores => "scores",
        SourceKind::Playoffs => "playoffs",
        SourceKind::Favorites => "favorites",
        SourceKind::Watchlist => "watchlist",
        SourceKind::Career => "career",
        SourceKind::Home => "home",
        SourceKind::Docs => "docs",
        SourceKind::GameLog => "game-log",
        SourceKind::Boxscore => "boxscore",
        SourceKind::PlayByPlay => "play-by-play",
        SourceKind::Shifts => "shifts",
        SourceKind::Transactions => "transactions",
        SourceKind::Contracts => "contracts",
        SourceKind::Standings => "standings",
        SourceKind::FantasyImport => "fantasy-import",
        SourceKind::Snapshot => "snapshot",
        SourceKind::Bundle => "bundle",
        SourceKind::Cache => "cache",
        SourceKind::Unknown => "unknown",
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

fn leader_primary_text(row: &icelines_core::LeaderRow) -> String {
    match &row.primary.value {
        MetricValue::Text(value) => value.clone(),
        MetricValue::Integer(value) => value.to_string(),
        MetricValue::Decimal(value) => format!("{value:.1}"),
        MetricValue::Missing => "—".to_string(),
    }
}

fn leader_gp(row: &icelines_core::LeaderRow) -> String {
    row.secondary
        .iter()
        .find(|metric| metric.key.0 == "gp")
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => Some(value.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "—".to_string())
}

fn leader_team(row: &icelines_core::LeaderRow) -> &str {
    if row.team.0 == "UNK" {
        "—"
    } else {
        row.team.0.as_str()
    }
}

fn leader_metric_i64(row: &icelines_core::LeaderRow, key: &str) -> Option<i64> {
    row.secondary
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => Some(value),
            _ => None,
        })
}

fn leader_metric_f64(row: &icelines_core::LeaderRow, key: &str) -> Option<f64> {
    row.secondary
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Decimal(value) => Some(value),
            MetricValue::Integer(value) => Some(value as f64),
            _ => None,
        })
}

fn print_improvement_table(
    views: &[PlayerView<'_>],
    imp_map: &std::collections::HashMap<u32, f64>,
    top: usize,
    total: usize,
    seasons: u8,
) {
    let window = if seasons > 1 {
        format!("({seasons}-season window) ")
    } else {
        String::new()
    };
    println!(
        "IMPROVEMENT LEADERS {}— Y/Y PPG delta (current vs prior season)",
        window
    );
    println!(
        "{:<4} {:<24} {:<5} {:<4} {:<4} {:<8} {:<8} {:<8}",
        "Rank", "Player", "Team", "Pos", "GP", "Curr", "Prior", "Δ PPG"
    );
    println!("{}", "─".repeat(69));
    for (i, v) in views.iter().enumerate() {
        let delta = imp_map.get(&v.identity.id.0).copied().unwrap_or(0.0);
        let curr_ppg = v
            .pace_82()
            .map(|p| format!("{:.3}", p / 82.0))
            .unwrap_or_else(|| "—".to_owned());
        let prior_ppg = format!(
            "{:.3}",
            v.pace_82().map(|p| p / 82.0).unwrap_or(0.0) - delta
        );
        let delta_s = if delta >= 0.0 {
            format!("+{delta:.3}")
        } else {
            format!("{delta:.3}")
        };
        println!(
            "{:<4} {:<24} {:<5} {:<4} {:<4} {:<8} {:<8} {:<8}",
            i + 1,
            v.identity.full_name,
            v.team_display(),
            v.position().abbreviation(),
            v.gp(),
            curr_ppg,
            prior_ppg,
            delta_s
        );
    }
    println!("\n{total} matched, showing {}.", views.len().min(top));
}

fn leaders_json(view: &LeadersView, total: usize, top: usize, active_filters: &[String]) -> String {
    serde_json::to_string_pretty(&leaders_json_rows(view, total, top, active_filters))
        .unwrap_or_default()
}

fn leaders_json_rows(
    view: &LeadersView,
    total: usize,
    top: usize,
    active_filters: &[String],
) -> Vec<serde_json::Value> {
    let returned = view.rows.len();
    view.rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "rank": row.rank,
                "total": total,
                "returned": returned,
                "top": top,
                "active_filters": active_filters,
                "season": view.context.window.season.0,
                "season_type": view.context.window.season_type.label(),
                "nhl_id": row.player_id.0,
                "name": row.display_name,
                "team": leader_team(row),
                "team_abbrev": leader_team(row),
                "pos": row.position.abbreviation(),
                "gp": leader_metric_i64(row, "gp").unwrap_or_default(),
                "ppg": leader_metric_f64(row, "ppg").map(round2),
                "pts_per_82": leader_metric_f64(row, "pts_per_82").map(round1),
                "goals_per_82": leader_metric_f64(row, "goals_per_82").map(round1),
                "season_pts": leader_metric_i64(row, "points").unwrap_or_default(),
                "season_goals": leader_metric_i64(row, "goals").unwrap_or_default(),
                "season_assists": leader_metric_i64(row, "assists").unwrap_or_default(),
                "source_completeness": &view.context.completeness,
                "source_state": &view.context.source_state,
            })
        })
        .collect()
}

fn leaders_json_envelope(
    view: &LeadersView,
    total: usize,
    top: usize,
    active_filters: &[String],
    pos: Option<&str>,
    sort: &str,
) -> String {
    let position_filter = pos
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| !value.is_empty());
    let meta = serde_json::json!({
        "season": view.context.window.season.0.to_string(),
        "season_type": view.context.window.season_type.label(),
        "sort": sort,
        "position_filter": position_filter,
        "active_filters": active_filters,
        "completeness": &view.context.completeness,
        "source_state": &view.context.source_state,
        "total": total,
        "returned": view.rows.len(),
        "top": top,
        "empty_state": &view.empty_state,
        "warnings": &view.warnings,
    });
    let envelope = serde_json::json!({
        "schema_version": 1,
        "route": "leaders",
        "data": leaders_json_rows(view, total, top, active_filters),
        "meta": meta,
    });
    serde_json::to_string_pretty(&envelope).unwrap_or_default()
}

fn leaders_csv(
    view: &LeadersView,
    total: usize,
    top: usize,
    sort: &str,
    active_filters: &[String],
) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out, "{}", leaders_csv_header());
    let window = view.context.window;
    let (source_kind, source_completeness) = leaders_source_state_labels(view);
    let returned = view.rows.len();
    let active_filters = leaders_active_filters_label(active_filters);
    for row in &view.rows {
        let _ = writeln!(
            out,
            "{},{},{},{},{},{:.3},{:.1},{:.1},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            row.rank,
            row.display_name,
            leader_team(row),
            row.position.abbreviation(),
            leader_metric_i64(row, "gp").unwrap_or_default(),
            leader_metric_f64(row, "ppg").unwrap_or_default(),
            leader_metric_f64(row, "pts_per_82").unwrap_or_default(),
            leader_metric_f64(row, "goals_per_82").unwrap_or_default(),
            leader_metric_i64(row, "points").unwrap_or_default(),
            leader_metric_i64(row, "goals").unwrap_or_default(),
            leader_metric_i64(row, "assists").unwrap_or_default(),
            row.player_id.0,
            window.season.0,
            window.season_type.label(),
            source_kind,
            source_completeness,
            total,
            returned,
            top,
            sort,
            active_filters,
        );
    }
    out
}

fn emit_query_output(path: Option<&std::path::Path>, body: &str) -> anyhow::Result<()> {
    match path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("✓ wrote {} ({} bytes)", path.display(), body.len());
        }
        None => println!("{body}"),
    }
    Ok(())
}

fn leaders_csv_header() -> &'static str {
    "rank,name,team,pos,gp,ppg,pts_per_82,goals_per_82,pts,goals,assists,nhl_id,season,season_type,source_kind,source_completeness,total,returned,top,sort,active_filters"
}

fn leaders_active_filters_label(active_filters: &[String]) -> String {
    if active_filters.is_empty() {
        "-".to_owned()
    } else {
        active_filters.join(";")
    }
}

fn position_percentile(
    all: &[PlayerView<'_>],
    target: &PlayerView<'_>,
    metric: SortDispatch,
) -> Option<u8> {
    let peers: Vec<&PlayerView<'_>> = all
        .iter()
        .filter(|v| v.position() == target.position() && v.is_rankable())
        .collect();
    if peers.is_empty() {
        return None;
    }
    let target_val = metric.sort_value(target);
    let n_better = peers
        .iter()
        .filter(|v| metric.sort_value(v) > target_val)
        .count();
    let pct = ((1.0 - n_better as f64 / peers.len() as f64) * 100.0) as u8;
    Some(pct)
}

// ── icelines query player ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)] // CLI inputs by design; struct would obscure dispatch.
pub async fn run_player(
    name: String,
    breakdown: String,
    percentiles: bool,
    last_n: Option<u32>,
    season: Option<String>,
    season_type: SeasonType,
    // Phase Lindsay L.5.3b — optional override for the percentile/rank
    // metric. None falls back to legacy Pts/82 ranking.
    rank_by: Option<String>,
    // Phase Lindsay D1b — narrow the percentile peer pool. Empty Vec
    // preserves the legacy "all same-position rankable peers" behavior.
    filters: Vec<String>,
    // Gaps.2 — number of bundled seasons to include in the career arc
    // (newest-first). 38 = full bundled history. 5 = legacy modern-era.
    seasons: u8,
) -> anyhow::Result<()> {
    // Resolve `--rank-by` once at command entry so a typo errors with
    // a clear message before we load the snapshot.
    let rank_metric: Option<SortDispatch> = match rank_by.as_deref() {
        Some(s) => Some(SortDispatch::parse(s)?),
        None => None,
    };
    // D1b — parse filter exprs up front so a typo errors before the
    // snapshot read.
    // QueryB — bio atoms (`age<=24`, `country=CAN`, `height>=72`)
    // pre-extracted from the top-level AND chain; stat residue
    // continues through the catalog parser as before.
    let (peer_bio, peer_stat_residue) = extract_bio_for_cli(&filters);
    // Wave 18+ dispatch — partition new-grammar atoms from legacy
    // residue. Same pattern as run_leaders / run_goalies.
    let (peer_new_plans, peer_legacy_residue) = partition_filter_dispatch(&peer_stat_residue)?;
    let mut peer_filter = PlayerFilter::new();
    for s in &peer_legacy_residue {
        // Filter.OR — boolean grammar accepted; bare atoms route to stat_filters.
        let expr = icelines_core::stats_catalog::parse_filter_expr(s)?;
        match expr.as_atom() {
            Some(atom) => peer_filter.stat_filters.push(*atom),
            None => peer_filter.expr_filters.push(expr),
        }
    }
    peer_filter.normalize_stat_filters();
    let (mut outcome, season_key, season_type) =
        crate::commands::players::load_repo_for_season(season.as_deref(), Some(season_type))?;
    // Gaps.2/5 — if the player isn't in the active season, fall back
    // to a bundled-season name lookup + lazy career fan-out so
    // historical players (Gretzky, Lemieux, Roy) resolve naturally
    // without forcing the user to know which season they played in.
    let mut historical_pid: Option<icelines_core::identity::PlayerId> = None;
    {
        let active_views: Vec<PlayerView<'_>> = outcome
            .repo
            .skaters(season_key, season_type)
            .chain(outcome.repo.goalies(season_key, season_type))
            .collect();
        if find_view(&active_views, &name).is_err() {
            drop(active_views);
            if let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(&name) {
                let pid = icelines_core::identity::PlayerId(pid);
                let _ = icelines_fetch::stats_loader::load_player_career_into_repo(
                    &mut outcome.repo,
                    pid,
                );
                historical_pid = Some(pid);
            }
        }
    }
    // Gaps.5 — search both skaters and goalies in the active window.
    // Gaps.2 — for historical-only players, append the most-recent
    // career row as a synthetic view so find_view resolves.
    let repo = &outcome.repo;
    let mut all_views: Vec<PlayerView<'_>> = repo
        .skaters(season_key, season_type)
        .chain(repo.goalies(season_key, season_type))
        .collect();
    if let Some(pid) = historical_pid {
        // Walk career_all and grab the most-recent row to use as the
        // primary view for find_view + percentile rendering. The lazy
        // loader populated all seasons, so career_all returns ≥1.
        if let Some(career_iter) = repo.career_all(pid) {
            let career: Vec<_> = career_iter.collect();
            if let Some(last) = career.last() {
                if let Some(v) = repo.view(pid, last.season, last.season_type) {
                    all_views.push(v);
                }
            }
        }
    }
    let v = find_view(&all_views, &name)?;
    let card = PlayerCardView::from_repository(repo, v.identity.id, v.season(), v.season_type());
    // D1b — apply the filter to compute the peer pool. The target
    // player itself stays available via `find_view(&all_views, ...)`
    // — only the percentile/rank cohort is narrowed.
    let peer_views: Vec<PlayerView<'_>> = if peer_filter.stat_filters.is_empty()
        && !peer_bio.is_active()
        && peer_new_plans.is_empty()
    {
        all_views.clone()
    } else {
        let mut pool = peer_filter.apply_views(all_views.iter().copied());
        if peer_bio.is_active() {
            pool.retain(|v| peer_bio.matches(v, season_key.0));
        }
        // Wave 18+ — apply new-pipeline plans for cohort filtering.
        if !peer_new_plans.is_empty() {
            let (provider, clock) = build_cli_eval_ctx(season_key.0)?;
            let ctx = icelines_query::EvalCtx::from_clock(
                &provider,
                icelines_query::StrictMode::Off,
                false,
                &clock,
                season_key.0,
            );
            for plan in &peer_new_plans {
                pool.retain(|v| plan.root.matches(v, &ctx));
            }
        }
        pool
    };

    let age = age_str(v);
    let draft = draft_str(v);
    println!(
        "PLAYER PROFILE — {} ({} · {} · Age {} · {})",
        card.as_ref()
            .map(|card| card.display_name.as_str())
            .unwrap_or(v.identity.full_name.as_str()),
        card.as_ref()
            .and_then(|card| card.active.as_ref())
            .map(|active| active.team_display.as_str())
            .unwrap_or_else(|| v.team_display()),
        card.as_ref()
            .and_then(|card| card.active.as_ref())
            .map(|active| active.position.abbreviation())
            .unwrap_or_else(|| v.position().abbreviation()),
        age,
        draft
    );
    println!("{}", "═".repeat(72));

    if last_n.is_some() {
        println!("  Note: --last-n requires game-log data (Phase 5C, not yet available).");
        println!();
    }

    match breakdown.to_lowercase().as_str() {
        "career" | "career-arc" => {
            print_current_stats(card.as_ref(), v);
            if percentiles {
                print_percentile(&peer_views, v, rank_metric);
            }
            print_career(v, seasons as usize).await;
            print_pre_nhl_career(v.identity.id.0).await;
        }
        "situation" => {
            println!("  Situational breakdown (5v5/PP/PK) requires Phase 5C shift data.");
            println!("  Currently available: all-situations stats only.");
            println!();
            print_current_stats(card.as_ref(), v);
            if percentiles {
                print_percentile(&peer_views, v, rank_metric);
            }
        }
        other => bail!("unknown breakdown '{other}' — valid: career, situation"),
    }
    Ok(())
}

fn print_current_stats(card: Option<&PlayerCardView>, v: &PlayerView<'_>) {
    let totals = &v.stats.totals;
    let metrics = card
        .and_then(|card| card.active.as_ref())
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);
    let ppg_value = card_metric_f64(metrics, "points_per_game");
    let ppg = ppg_value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| pace_strings(v).0);
    let proj = ppg_value
        .map(|value| format!("{:.1}", value * 82.0))
        .unwrap_or_else(|| pace_strings(v).1);
    let toi = card_metric_i64(metrics, "toi_per_game_sec")
        .map(format_seconds_mmss)
        .unwrap_or_else(|| v.toi_mmss().unwrap_or_else(|| "—".to_owned()));
    let sh_pct = card_metric_f64(metrics, "shooting_pct")
        .map(|val| format!("{:.1}%", val * 100.0))
        .unwrap_or_else(|| {
            totals
                .shooting_pct
                .map(|val| format!("{:.1}%", val * 100.0))
                .unwrap_or_else(|| "—".to_owned())
        });
    let pm = card_metric_i64(metrics, "plus_minus")
        .map(format_signed)
        .unwrap_or_else(|| format_signed(v.plus_minus() as i64));
    println!("CURRENT SEASON");
    println!(
        "  GP {:<4}  G {:<4}  A {:<4}  Pts {:<4}  PPG {}  Pts/82 {}",
        card_metric_i64(metrics, "gp").unwrap_or_else(|| v.gp() as i64),
        card_metric_i64(metrics, "goals").unwrap_or(totals.goals as i64),
        card_metric_i64(metrics, "assists").unwrap_or(totals.assists as i64),
        card_metric_i64(metrics, "points").unwrap_or(totals.points as i64),
        ppg,
        proj
    );
    println!(
        "  PP: {:<3}G / {:<3}Pts   SH: {}G   GWG: {}   Shots: {}   SH%: {}",
        card_metric_i64(metrics, "pp_goals").unwrap_or(totals.pp_goals as i64),
        card_metric_i64(metrics, "pp_points").unwrap_or(totals.pp_points as i64),
        card_metric_i64(metrics, "sh_goals").unwrap_or(totals.sh_goals as i64),
        card_metric_i64(metrics, "gwg").unwrap_or(totals.gwg as i64),
        card_metric_i64(metrics, "shots").unwrap_or_else(|| v.shots() as i64),
        sh_pct
    );
    println!(
        "  +/-: {:<5}  TOI/g: {:<6}{}",
        pm,
        toi,
        totals
            .faceoff_win_pct
            .map(|val| format!("  FO%: {:.1}%", val * 100.0))
            .unwrap_or_default()
    );
    println!();

    if v.contract.is_some() {
        let expiry_year = v
            .contract_expiry_year()
            .map(|y| y.to_string())
            .unwrap_or_else(|| "?".to_owned());
        let expiry_type = v.contract_expiry_type().unwrap_or("unknown");
        println!(
            "  Contract: {} expires {}  Valuation season: {}  Cap hit: {}  AAV: {}  Salary: {}",
            expiry_type.to_uppercase(),
            expiry_year,
            v.contract_valuation_season().unwrap_or("unknown"),
            format_contract_money(v.contract_cap_hit()),
            format_contract_money(v.contract_aav()),
            format_contract_money(v.contract_salary()),
        );
        if let Some(source) = v.contract_source() {
            if let Some(url) = v.contract_source_url() {
                println!("  Contract source: {source} ({url})");
            } else {
                println!("  Contract source: {source}");
            }
        }
        println!();
    }
}

fn format_contract_money(value: Option<u64>) -> String {
    value
        .map(|amount| format!("${:.2}M", amount as f64 / 1_000_000.0))
        .unwrap_or_else(|| "—".to_owned())
}

fn print_percentile(
    all: &[PlayerView<'_>],
    target: &PlayerView<'_>,
    // Phase Lindsay L.5.3b — when Some, use the dispatched metric for
    // the rank/percentile. None preserves legacy Pts/82 ranking.
    metric: Option<SortDispatch>,
) {
    let peers: Vec<&PlayerView<'_>> = all
        .iter()
        .filter(|v| v.position() == target.position() && v.is_rankable())
        .collect();
    if peers.is_empty() {
        return;
    }

    let (target_val, n_better, label) = match metric {
        Some(m) => {
            // Catalog/legacy via SortDispatch. `sort_value` returns
            // None as ±infinity (worst) per the SortDispatch contract,
            // so the percentile is well-defined even when target/peer
            // reads are None.
            let target_val = m.sort_value(target);
            let n_better = peers
                .iter()
                .filter(|v| m.sort_value(v) > target_val)
                .count();
            // Header text uses the metric's catalog/legacy label.
            let label = m.header(false);
            (target_val, n_better, label)
        }
        None => {
            // Legacy default: Pts/82 ranking.
            let target_val = target.pace_82().unwrap_or(0.0);
            let n_better = peers
                .iter()
                .filter(|v| v.pace_82().unwrap_or(0.0) > target_val)
                .count();
            (target_val, n_better, "Pts/82".to_owned())
        }
    };
    let _ = target_val; // used only for closure capture above

    let rank = n_better + 1;
    let pct = ((1.0 - n_better as f64 / peers.len() as f64) * 100.0) as u8;
    println!(
        "LEAGUE RANK  #{rank} of {} {}'s  ({pct}{} percentile by {label})",
        peers.len(),
        target.position().abbreviation(),
        ordinal(pct),
    );
    println!();
}

/// Phase Calder.3 — pre-NHL career stints from the multi-league
/// store. Renders only when the user has populated the store via
/// `icelines fetch career`; silent no-op otherwise. Skips NHL stints
/// (those are already covered by `print_career`'s NHL career arc) and
/// any IIHF/Olympic tournaments — the focus here is the development
/// path: junior, NCAA, AHL, European pro.
async fn print_pre_nhl_career(player_id: u32) {
    let store = icelines_fetch::career_landing::load_local_store();
    if store.is_empty() {
        return;
    }
    let Some(history) = store.get(player_id) else {
        return;
    };
    let pre_nhl = icelines_fetch::career_landing::extract_pre_nhl_stints(history);
    if pre_nhl.is_empty() {
        return;
    }
    println!();
    print!("{}", render_pre_nhl_career_table(&pre_nhl));
}

/// Phase Calder.3 — pure renderer. Returns the section as a String
/// so tests can pin format without capturing stdout.
pub(crate) fn render_pre_nhl_career_table(
    pre_nhl: &[icelines_core::career_history::CareerStint],
) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "PRE-NHL CAREER — {} stints", pre_nhl.len());
    // Width 14 on the League column fits "J20 Nationell", "Champions HL",
    // "Allsvenskan" cleanly. Team width 22 is enough for "U. Mass-Lowell".
    let _ = writeln!(
        out,
        "{:<10} {:<14} {:<22} {:<4} {:<4} {:<4} {:<5} {:<6}",
        "Season", "League", "Team", "GP", "G", "A", "P", "PPG"
    );
    let _ = writeln!(out, "{}", "─".repeat(76));
    for s in pre_nhl {
        let ppg = s
            .points_per_game()
            .map(|p| format!("{p:.2}"))
            .unwrap_or_else(|| "—".into());
        let team = if s.team.len() > 22 {
            &s.team[..22]
        } else {
            &s.team[..]
        };
        let league = if s.league.0.len() > 14 {
            &s.league.0[..14]
        } else {
            s.league.0.as_str()
        };
        let _ = writeln!(
            out,
            "{:<10} {:<14} {:<22} {:<4} {:<4} {:<4} {:<5} {:<6}",
            season_label(&s.season.0.to_string()),
            league,
            team,
            s.gp,
            s.goals.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
            s.assists
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            s.points
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            ppg,
        );
    }
    out
}

async fn print_career(v: &PlayerView<'_>, seasons: usize) {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(_) => return,
    };
    let store = SnapshotStore::new(cfg.snapshot_dir());
    // Clamp at the bundled-season floor so a user passing `--seasons 100`
    // gets the maximum (38) without erroring. 0 also clamps to 1.
    let n = seasons.clamp(1, icelines_fetch::BUNDLED_SEASONS.len());
    match load_career(&v.identity.full_name, n, &store) {
        Some(career) => {
            println!("CAREER ARC — {} seasons", career.seasons.len());
            if let Some(disclosure) = bundled_depth_disclosure(career.seasons.len()) {
                println!("{disclosure}");
            }
            if let Some(summary) = career_arc_sparkline_summary(&career.seasons) {
                println!("{summary}");
            }
            println!(
                "{:<10} {:<6} {:<4} {:<4} {:<4} {:<8} {:<8}",
                "Season", "Team", "GP", "G", "A", "PPG", "Pts/82"
            );
            println!("{}", "─".repeat(52));
            for line in &career.seasons {
                println!(
                    "{:<10} {:<6} {:<4} {:<4} {:<4} {:<8.3} {:<8.1}",
                    season_label(&line.season),
                    line.team,
                    line.gp,
                    line.goals,
                    line.assists,
                    line.ppg,
                    line.pts_per_82()
                );
            }
            println!("{}", "─".repeat(52));
            println!(
                "Career: {:.3} pts/gp  |  Peak: {} ({:.3} pts/gp)",
                career.career_ppg,
                season_label(&career.peak_season),
                career.peak_ppg
            );
        }
        None => {
            println!("Career history: not available (bundled season data required).");
        }
    }
}

fn career_arc_sparkline_summary(lines: &[icelines_core::history::SeasonLine]) -> Option<String> {
    if lines.len() < 2 {
        return None;
    }
    let chronological: Vec<_> = lines.iter().rev().collect();
    let first = season_label(&chronological.first()?.season);
    let last = season_label(&chronological.last()?.season);
    let pts_per_82: Vec<f64> = chronological
        .iter()
        .map(|line| line.pts_per_82() as f64)
        .collect();
    let goals_per_82: Vec<f64> = chronological
        .iter()
        .map(|line| line.goals_per_82 as f64)
        .collect();
    Some(format!(
        "Career trend ({first} → {last}): Pts/82 {}  G/82 {}",
        crate::tui::sparkline::render(&pts_per_82, 24),
        crate::tui::sparkline::render(&goals_per_82, 24)
    ))
}

fn bundled_depth_disclosure(seasons_rendered: usize) -> Option<String> {
    let modern = icelines_fetch::bundled::MODERN_BUNDLED_SEASONS.len();
    if seasons_rendered <= modern {
        None
    } else {
        Some(format!(
            "Data depth: newest {modern} bundled seasons carry modern Tier-1 depth; older seasons in this window are historical/skeleton season totals. Missing modern fields render unavailable, not zero."
        ))
    }
}

// ── icelines query goalies (Phase G.5) ────────────────────────────────────────
//
// Hart.5c.3 leaves run_goalies on the legacy `Goalie` path because the
// `Goalie` struct lives until 5c.7 (final delete). Migrating it here
// would require a parallel goalie-view leaderboard sketch with no
// remaining Goalie consumer, which is wasted churn.

pub struct GoaliesArgs {
    pub top: usize,
    pub sort: String,
    pub team: Option<String>,
    pub min_gp: u32,
    pub season: Option<String>,
    /// Hart.6.9 — season-type window (Regular default, Playoff opt-in).
    pub season_type: SeasonType,
    pub json: bool,
    pub csv: bool,
    /// Phase Lindsay L.5b post-fix (II-06 partial roll) — generic
    /// stat filters in the same grammar as `query leaders --filter`.
    pub filters: Vec<String>,
}

/// JSON / CSV output row for `query goalies`. Hart.5c.7: stable shape
/// for CLI consumers, decoupled from the icelines-core model. Mirrors
/// the field set the legacy Goalie struct produced via serde.
#[derive(Debug, serde::Serialize)]
struct GoalieRow {
    nhl_id: u32,
    full_name: String,
    team: String,
    games_played: u32,
    wins: u32,
    losses: u32,
    ot_losses: Option<u32>,
    save_pct: Option<f32>,
    goals_against_average: Option<f32>,
    shutouts: u32,
    saves: u32,
    quality_start_pct: Option<f32>,
    shots_against_per_60: Option<f32>,
}

pub async fn run_goalies(args: GoaliesArgs) -> anyhow::Result<()> {
    use icelines_core::stats_repository::PlayerView;

    if args.json && args.csv {
        bail!("--json and --csv are mutually exclusive");
    }

    let (outcome, season, _) = crate::commands::players::load_repo_for_season(
        args.season.as_deref(),
        Some(args.season_type),
    )?;
    let season_u32 = season.0;

    let mut views: Vec<PlayerView<'_>> = outcome
        .repo
        .goalies(season, args.season_type)
        .filter(|v| v.gp() >= args.min_gp)
        .collect();

    if let Some(team) = args.team.as_deref() {
        let abbrev = team.to_ascii_uppercase();
        views.retain(|v| v.team_display() == abbrev);
    }

    // Phase Lindsay L.5b post-fix (D1 / II-06 partial roll) — apply
    // generic catalog `--filter` expressions against the goalie pool.
    // Same parse_filter grammar as query leaders; filter typos exit
    // non-zero with the actionable hint (KEEL D2 typo path also fires
    // here for free).
    // QueryB — bio atoms (`age<=24`, `country=CAN`) pre-extracted
    // from the top-level AND chain. Stat residue routes through the
    // catalog parser; bio constraints are applied as a post-step.
    let (goalie_bio, goalie_stat_residue) = extract_bio_for_cli(&args.filters);
    let mut filter = PlayerFilter::new();
    let mut goalie_new_plans: Vec<icelines_query::QueryPlan> = Vec::new();
    if !goalie_stat_residue.is_empty() {
        use icelines_core::stats_catalog::parse_filter_expr;
        for s in &goalie_stat_residue {
            // Gaps.4 — rewrite each atom's leading key to its goalie
            // context equivalent before parsing. `gp` → `goalie-games`,
            // `starts` → `goalie-starts`. Filter.OR — the rewrite walks
            // the WHOLE expression text, so atoms inside AND/OR/NOT/
            // parens get rewritten too.
            let rewritten = goalie_filter_rewrite_expr(s);

            // Wave 18 fix — try the new pipeline first on the
            // rewritten string (so `gp>=15` becomes
            // `goalie-games>=15` and parses through both pipelines).
            // If parse_query succeeds, use the new pipeline; if it
            // produces a helpful error, surface that; else fall
            // through to the legacy parser.
            match icelines_query::parse_query(icelines_query::FilterInput::Cli(rewritten.clone())) {
                Ok(plan) => {
                    goalie_new_plans.push(plan);
                    continue;
                }
                Err(es) => {
                    let prefer_new = es.iter().any(|e| {
                        matches!(
                            e,
                            icelines_query::ParseError::IncompatiblePredicate { .. }
                                | icelines_query::ParseError::EmptySet { .. }
                                | icelines_query::ParseError::FeatureNotYet { .. }
                                | icelines_query::ParseError::UnknownWindowUnit { .. }
                                | icelines_query::ParseError::ZeroWindowSize { .. }
                                | icelines_query::ParseError::WindowSizeOutOfRange { .. }
                        )
                    });
                    if prefer_new {
                        let msgs: Vec<String> = es.iter().map(|e| e.to_string()).collect();
                        anyhow::bail!(
                            "--filter {s:?}\n  {}\n  hint: goalies use \
                             `goalie-games`, `goalie-starts`, `save-pct`, \
                             `gaa`, `wins`, `losses`, `ot-losses`, `shutouts`",
                            msgs.join("\n  ")
                        );
                    }
                    // Else: fall through to legacy
                }
            }

            let expr = parse_filter_expr(&rewritten).map_err(|e| {
                anyhow::anyhow!(
                    "--filter {s:?}\n  {e}\n  hint: goalies use `goalie-games`, `goalie-starts`, `save-pct`, `gaa`, `wins`, `losses`, `ot-losses`, `shutouts`"
                )
            })?;
            match expr.as_atom() {
                Some(atom) => filter.stat_filters.push(*atom),
                None => filter.expr_filters.push(expr),
            }
        }
        filter.normalize_stat_filters();
        views = filter.apply_views(views.iter().copied());
    }
    if goalie_bio.is_active() {
        views.retain(|v| goalie_bio.matches(v, season_u32));
    }
    // Wave 18 — apply new-pipeline goalie plans.
    if !goalie_new_plans.is_empty() {
        let home_dir = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
        let data_root = home_dir.join(".icelines").join("data");
        let provider = icelines_fetch::query_provider::IcelinesProvider::new(data_root);
        let clock = icelines_core::freshness::SystemClock;
        let ctx = icelines_query::EvalCtx::from_clock(
            &provider,
            icelines_query::StrictMode::Off,
            false,
            &clock,
            season_u32,
        );
        for plan in &goalie_new_plans {
            views.retain(|v| plan.root.matches(v, &ctx));
        }
    }

    let sort_key = args.sort.to_ascii_lowercase();
    views.sort_by(|a, b| {
        if let Some(sort) = icelines_core::GoalieLeaderboardSort::from_key(&sort_key) {
            return sort.compare_player_views(a, b);
        }
        if let Some(sid) = StatId::from_cli_key(&sort_key) {
            return sid.sort_cmp(a, b);
        }
        eprintln!("  Hint: unknown sort '{sort_key}' - falling back to sv-pct.");
        icelines_core::GoalieLeaderboardSort::SavePct.compare_player_views(a, b)
    });
    views.truncate(args.top);

    let mut goalies_view = icelines_core::GoaliesView::from_player_views(
        icelines_core::ViewContext::new(icelines_core::ViewWindow::new(
            icelines_core::model::Season(season_u32),
            args.season_type,
        )),
        views.iter().copied(),
    );
    goalies_view.sort = Some(icelines_core::SortState {
        key: icelines_core::SortKey(sort_key.clone()),
        label: sort_key.clone(),
        direction: if sort_key == "gaa" {
            icelines_core::SortDirection::Asc
        } else {
            icelines_core::SortDirection::Desc
        },
    });
    let rows = goalie_output_rows_from_view(&goalies_view);

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).context("serializing goalies to JSON")?
        );
        return Ok(());
    }
    if args.csv {
        println!("rank,goalie,team,gp,wins,losses,ot_losses,sv_pct,gaa,so,saves,qs_pct,sa_per_60");
        for (i, row) in rows.iter().enumerate() {
            println!(
                "{},\"{}\",{},{},{},{},{},{:.4},{:.3},{},{},{:.4},{:.1}",
                i + 1,
                row.full_name,
                row.team,
                row.games_played,
                row.wins,
                row.losses,
                row.ot_losses.unwrap_or(0),
                row.save_pct.unwrap_or(0.0),
                row.goals_against_average.unwrap_or(0.0),
                row.shutouts,
                row.saves,
                row.quality_start_pct.unwrap_or(0.0),
                row.shots_against_per_60.unwrap_or(0.0),
            );
        }
        return Ok(());
    }

    println!(
        "{:<4} {:<24} {:<5} {:<4} {:<10} {:<6} {:<6} {:<3} {:<6} {:<6} {:<6}",
        "Rank", "Goalie", "Team", "GP", "W-L-OT", "SV%", "GAA", "SO", "Saves", "QS%", "SA/60"
    );
    println!("{}", "─".repeat(94));
    for (i, row) in rows.iter().enumerate() {
        let record = match row.ot_losses {
            Some(otl) => format!("{}-{}-{}", row.wins, row.losses, otl),
            None => format!("{}-{}", row.wins, row.losses),
        };
        let sv = row
            .save_pct
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "—".to_owned());
        let gaa = row
            .goals_against_average
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "—".to_owned());
        let quality_start_pct = row
            .quality_start_pct
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "—".to_owned());
        let shots_against_per_60 = row
            .shots_against_per_60
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "—".to_owned());
        println!(
            "{:<4} {:<24} {:<5} {:<4} {:<10} {:<6} {:<6} {:<3} {:<6} {:<6} {:<6}",
            i + 1,
            row.full_name.chars().take(24).collect::<String>(),
            row.team,
            row.games_played,
            record,
            sv,
            gaa,
            row.shutouts,
            row.saves,
            quality_start_pct,
            shots_against_per_60,
        );
    }
    println!(
        "\n{} goalies (min {} GP, sorted by {}) for season {}.",
        rows.len(),
        args.min_gp,
        sort_key,
        season
    );
    Ok(())
}

// ── icelines query compare ────────────────────────────────────────────────────

fn goalie_output_rows_from_view(view: &icelines_core::GoaliesView) -> Vec<GoalieRow> {
    view.rows
        .iter()
        .map(|row| GoalieRow {
            nhl_id: row.player_id.0,
            full_name: row.display_name.clone(),
            team: row.team.0.clone(),
            games_played: metric_u32(row, "gp"),
            wins: metric_u32(row, "wins"),
            losses: metric_u32(row, "losses"),
            ot_losses: metric_optional_u32(row, "ot_losses"),
            save_pct: metric_optional_f32(row, "save_pct"),
            goals_against_average: metric_optional_f32(row, "gaa"),
            shutouts: metric_u32(row, "shutouts"),
            saves: metric_u32(row, "saves"),
            quality_start_pct: metric_optional_f32(row, "quality_start_pct"),
            shots_against_per_60: metric_optional_f32(row, "shots_against_per_60"),
        })
        .collect()
}

fn metric_u32(row: &icelines_core::GoalieRow, key: &str) -> u32 {
    metric_optional_u32(row, key).unwrap_or(0)
}

fn metric_optional_u32(row: &icelines_core::GoalieRow, key: &str) -> Option<u32> {
    row.metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                icelines_core::MetricValue::Integer(value) => u32::try_from(value).ok(),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn metric_optional_f32(row: &icelines_core::GoalieRow, key: &str) -> Option<f32> {
    row.metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                icelines_core::MetricValue::Decimal(value) => Some(value as f32),
                _ => None,
            }
        } else {
            None
        }
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn run_compare(
    player1: String,
    player2: Option<String>,
    similar: Option<usize>,
    _by: String,
    season: Option<String>,
    season_type: SeasonType,
    // Phase Lindsay D1b — narrow the similarity cohort. Only applies
    // when --similar N is set (head-to-head shows just two players, no
    // cohort to filter).
    filters: Vec<String>,
    // Gaps.3 — number of bundled seasons to print under each player on
    // head-to-head. Active only on head-to-head; --similar ignores this
    // (it's already an N-cohort Z-score over a single window).
    seasons: u8,
) -> anyhow::Result<()> {
    // D1b — parse filter exprs up front; typos exit before snapshot load.
    // Filter.OR — boolean grammar accepted; bare atoms route to stat_filters.
    // QueryB — bio atoms (`age<=24`, `country=CAN`, `height>=72`)
    // pre-extracted from the top-level AND chain.
    let (cohort_bio, cohort_stat_residue) = extract_bio_for_cli(&filters);
    // Wave 18+ — partition new-grammar from legacy residue.
    let (cohort_new_plans, cohort_legacy_residue) =
        partition_filter_dispatch(&cohort_stat_residue)?;
    let mut cohort_filter = PlayerFilter::new();
    for s in &cohort_legacy_residue {
        let expr = icelines_core::stats_catalog::parse_filter_expr(s)?;
        match expr.as_atom() {
            Some(atom) => cohort_filter.stat_filters.push(*atom),
            None => cohort_filter.expr_filters.push(expr),
        }
    }
    cohort_filter.normalize_stat_filters();
    let (mut outcome, season_key, season_type) =
        crate::commands::players::load_repo_for_season(season.as_deref(), Some(season_type))?;
    // Gaps.3 — same historical-resolution fallback as run_player.
    // For each side of the head-to-head, if the name isn't in the
    // active window, fan out across bundled seasons.
    let active_names_present = {
        let active_views: Vec<PlayerView<'_>> = outcome
            .repo
            .skaters(season_key, season_type)
            .chain(outcome.repo.goalies(season_key, season_type))
            .collect();
        let p1_in = find_view(&active_views, &player1).is_ok();
        let p2_in = match &player2 {
            Some(n) => find_view(&active_views, n).is_ok(),
            None => true, // similar mode — only player1 needs to resolve
        };
        (p1_in, p2_in)
    };
    if !active_names_present.0 {
        if let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(&player1) {
            let _ = icelines_fetch::stats_loader::load_player_career_into_repo(
                &mut outcome.repo,
                icelines_core::identity::PlayerId(pid),
            );
        }
    }
    if let Some(p2_name) = &player2 {
        if !active_names_present.1 {
            if let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(p2_name) {
                let _ = icelines_fetch::stats_loader::load_player_career_into_repo(
                    &mut outcome.repo,
                    icelines_core::identity::PlayerId(pid),
                );
            }
        }
    }
    let repo = &outcome.repo;
    let mut all_views: Vec<PlayerView<'_>> = repo
        .skaters(season_key, season_type)
        .chain(repo.goalies(season_key, season_type))
        .collect();
    // Append a most-recent-window view for each historical-only
    // identity loaded above.
    for player_name in std::iter::once(&player1).chain(player2.iter()) {
        if let Some(raw_pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(player_name)
        {
            let pid = icelines_core::identity::PlayerId(raw_pid);
            if all_views.iter().all(|v| v.identity.id != pid) {
                if let Some(career_iter) = repo.career_all(pid) {
                    let career: Vec<_> = career_iter.collect();
                    if let Some(last) = career.last() {
                        if let Some(v) = repo.view(pid, last.season, last.season_type) {
                            all_views.push(v);
                        }
                    }
                }
            }
        }
    }

    if let Some(n) = similar {
        // D1b — narrow cohort. Empty filter passes through unchanged.
        let cohort_views: Vec<PlayerView<'_>> = if cohort_filter.stat_filters.is_empty()
            && !cohort_bio.is_active()
            && cohort_new_plans.is_empty()
        {
            all_views.clone()
        } else {
            let mut pool = cohort_filter.apply_views(all_views.iter().copied());
            if cohort_bio.is_active() {
                pool.retain(|v| cohort_bio.matches(v, season_key.0));
            }
            // Wave 18+ — apply new-pipeline plans to the cohort.
            if !cohort_new_plans.is_empty() {
                let (provider, clock) = build_cli_eval_ctx(season_key.0)?;
                let ctx = icelines_query::EvalCtx::from_clock(
                    &provider,
                    icelines_query::StrictMode::Off,
                    false,
                    &clock,
                    season_key.0,
                );
                for plan in &cohort_new_plans {
                    pool.retain(|v| plan.root.matches(v, &ctx));
                }
            }
            pool
        };
        run_similar(&cohort_views, &player1, n)
    } else if let Some(p2_name) = player2 {
        let v1 = find_view(&all_views, &player1)?;
        let v2 = find_view(&all_views, &p2_name)?;
        if v1.identity.name_normalized == v2.identity.name_normalized {
            eprintln!(
                "  Note: both sides resolved to the same player ({}).",
                v1.identity.full_name
            );
        }
        let compare = CompareView::from_repository(
            repo,
            Some(v1.identity.id),
            Some(v2.identity.id),
            season_key,
            season_type,
        );
        print_head_to_head_from_compare_view(&compare, v1, v2);

        // Gaps.3 — multi-season career arcs for context. Reuses the
        // print_career helper used by run_player; runs sequentially
        // for each player so the two arcs land back-to-back in stdout.
        if seasons > 1 {
            println!();
            print_career(v1, seasons as usize).await;
            println!();
            print_career(v2, seasons as usize).await;
        }
        Ok(())
    } else {
        bail!("provide a second player name, or use --similar N for similarity search")
    }
}

fn print_head_to_head(v1: &PlayerView<'_>, v2: &PlayerView<'_>) {
    let (ppg1, proj1) = pace_strings(v1);
    let (ppg2, proj2) = pace_strings(v2);
    let col = 28usize;

    println!(
        "{:<col$} {:<col$}",
        v1.identity.full_name, v2.identity.full_name
    );
    println!("{:<col$} {:<col$}", v1.team_display(), v2.team_display());
    println!("{}", "─".repeat(col * 2 + 2));

    let row = |label: &str, c1: &str, c2: &str| {
        println!("  {:<18} {:<col$} {:<col$}", label, c1, c2, col = col - 2);
    };

    row(
        "Position",
        v1.position().abbreviation(),
        v2.position().abbreviation(),
    );
    row("Age", &age_str(v1), &age_str(v2));
    row("Draft", &draft_str(v1), &draft_str(v2));
    row("GP", &v1.gp().to_string(), &v2.gp().to_string());
    row("PPG", &ppg1, &ppg2);
    row("Pts/82", &proj1, &proj2);
    row(
        "Goals/82",
        &v1.goals_per_82()
            .map(|g| format!("{g:.1}"))
            .unwrap_or_else(|| "—".to_owned()),
        &v2.goals_per_82()
            .map(|g| format!("{g:.1}"))
            .unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "PP Points",
        &v1.stats.totals.pp_points.to_string(),
        &v2.stats.totals.pp_points.to_string(),
    );
    row(
        "PP Goals",
        &v1.stats.totals.pp_goals.to_string(),
        &v2.stats.totals.pp_goals.to_string(),
    );
    row(
        "GWG",
        &v1.stats.totals.gwg.to_string(),
        &v2.stats.totals.gwg.to_string(),
    );
    row("Shots", &v1.shots().to_string(), &v2.shots().to_string());
    row(
        "SH%",
        &v1.stats
            .totals
            .shooting_pct
            .map(|val| format!("{:.1}%", val * 100.0))
            .unwrap_or_else(|| "—".to_owned()),
        &v2.stats
            .totals
            .shooting_pct
            .map(|val| format!("{:.1}%", val * 100.0))
            .unwrap_or_else(|| "—".to_owned()),
    );
    let pm_str = |pm: i32| {
        if pm >= 0 {
            format!("+{pm}")
        } else {
            pm.to_string()
        }
    };
    row("+/-", &pm_str(v1.plus_minus()), &pm_str(v2.plus_minus()));
    row(
        "TOI/g",
        &v1.toi_mmss().unwrap_or_else(|| "—".to_owned()),
        &v2.toi_mmss().unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Contract",
        &v1.contract_expiry_type()
            .map(|t| t.to_uppercase())
            .unwrap_or_else(|| "—".to_owned()),
        &v2.contract_expiry_type()
            .map(|t| t.to_uppercase())
            .unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Expires",
        &v1.contract_expiry_year()
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_owned()),
        &v2.contract_expiry_year()
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Cap hit",
        &format_contract_money(v1.contract_cap_hit()),
        &format_contract_money(v2.contract_cap_hit()),
    );
    row(
        "AAV",
        &format_contract_money(v1.contract_aav()),
        &format_contract_money(v2.contract_aav()),
    );
}

fn print_head_to_head_from_compare_view(
    compare: &CompareView,
    fallback1: &PlayerView<'_>,
    fallback2: &PlayerView<'_>,
) {
    match (compare.a.as_ref(), compare.b.as_ref()) {
        (Some(card1), Some(card2)) => print_head_to_head_cards(card1, card2, fallback1, fallback2),
        _ => print_head_to_head(fallback1, fallback2),
    }
}

fn print_head_to_head_cards(
    card1: &PlayerCardView,
    card2: &PlayerCardView,
    fallback1: &PlayerView<'_>,
    fallback2: &PlayerView<'_>,
) {
    let col = 28usize;
    let active1 = card1.active.as_ref();
    let active2 = card2.active.as_ref();
    let metrics1 = active1
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);
    let metrics2 = active2
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);

    println!("{:<col$} {:<col$}", card1.display_name, card2.display_name);
    println!(
        "{:<col$} {:<col$}",
        active1
            .map(|active| active.team_display.as_str())
            .unwrap_or_else(|| fallback1.team_display()),
        active2
            .map(|active| active.team_display.as_str())
            .unwrap_or_else(|| fallback2.team_display())
    );
    println!("{}", "─".repeat(col * 2 + 2));

    let row = |label: &str, c1: &str, c2: &str| {
        println!("  {:<18} {:<col$} {:<col$}", label, c1, c2, col = col - 2);
    };

    row(
        "Position",
        active1
            .map(|active| active.position.abbreviation())
            .unwrap_or_else(|| fallback1.position().abbreviation()),
        active2
            .map(|active| active.position.abbreviation())
            .unwrap_or_else(|| fallback2.position().abbreviation()),
    );
    row("Age", &age_str(fallback1), &age_str(fallback2));
    row("Draft", &draft_str(fallback1), &draft_str(fallback2));
    row(
        "GP",
        &card_metric_i64(metrics1, "gp")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback1.gp().to_string()),
        &card_metric_i64(metrics2, "gp")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback2.gp().to_string()),
    );
    row(
        "PPG",
        &card_metric_f64(metrics1, "points_per_game")
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| pace_strings(fallback1).0),
        &card_metric_f64(metrics2, "points_per_game")
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| pace_strings(fallback2).0),
    );
    row(
        "Pts/82",
        &points_82_from_metrics(metrics1, fallback1),
        &points_82_from_metrics(metrics2, fallback2),
    );
    row(
        "Goals/82",
        &fallback1
            .goals_per_82()
            .map(|g| format!("{g:.1}"))
            .unwrap_or_else(|| "—".to_owned()),
        &fallback2
            .goals_per_82()
            .map(|g| format!("{g:.1}"))
            .unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "PP Points",
        &card_metric_i64(metrics1, "pp_points")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback1.stats.totals.pp_points.to_string()),
        &card_metric_i64(metrics2, "pp_points")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback2.stats.totals.pp_points.to_string()),
    );
    row(
        "PP Goals",
        &card_metric_i64(metrics1, "pp_goals")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback1.stats.totals.pp_goals.to_string()),
        &card_metric_i64(metrics2, "pp_goals")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback2.stats.totals.pp_goals.to_string()),
    );
    row(
        "GWG",
        &card_metric_i64(metrics1, "gwg")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback1.stats.totals.gwg.to_string()),
        &card_metric_i64(metrics2, "gwg")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback2.stats.totals.gwg.to_string()),
    );
    row(
        "Shots",
        &card_metric_i64(metrics1, "shots")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback1.shots().to_string()),
        &card_metric_i64(metrics2, "shots")
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback2.shots().to_string()),
    );
    row(
        "SH%",
        &card_metric_f64(metrics1, "shooting_pct")
            .map(|val| format!("{:.1}%", val * 100.0))
            .unwrap_or_else(|| "—".to_owned()),
        &card_metric_f64(metrics2, "shooting_pct")
            .map(|val| format!("{:.1}%", val * 100.0))
            .unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "+/-",
        &card_metric_i64(metrics1, "plus_minus")
            .map(format_signed)
            .unwrap_or_else(|| format_signed(fallback1.plus_minus() as i64)),
        &card_metric_i64(metrics2, "plus_minus")
            .map(format_signed)
            .unwrap_or_else(|| format_signed(fallback2.plus_minus() as i64)),
    );
    row(
        "TOI/g",
        &card_metric_i64(metrics1, "toi_per_game_sec")
            .map(format_seconds_mmss)
            .unwrap_or_else(|| fallback1.toi_mmss().unwrap_or_else(|| "—".to_owned())),
        &card_metric_i64(metrics2, "toi_per_game_sec")
            .map(format_seconds_mmss)
            .unwrap_or_else(|| fallback2.toi_mmss().unwrap_or_else(|| "—".to_owned())),
    );
    row(
        "Contract",
        &fallback1
            .contract_expiry_type()
            .map(|t| t.to_uppercase())
            .unwrap_or_else(|| "—".to_owned()),
        &fallback2
            .contract_expiry_type()
            .map(|t| t.to_uppercase())
            .unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Expires",
        &fallback1
            .contract_expiry_year()
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_owned()),
        &fallback2
            .contract_expiry_year()
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Cap hit",
        &format_contract_money(fallback1.contract_cap_hit()),
        &format_contract_money(fallback2.contract_cap_hit()),
    );
    row(
        "AAV",
        &format_contract_money(fallback1.contract_aav()),
        &format_contract_money(fallback2.contract_aav()),
    );
}

fn card_metric_i64(metrics: &[MetricCell], key: &str) -> Option<i64> {
    metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Integer(value) => Some(value),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn card_metric_f64(metrics: &[MetricCell], key: &str) -> Option<f64> {
    metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Decimal(value) => Some(value),
                MetricValue::Integer(value) => Some(value as f64),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn points_82_from_metrics(metrics: &[MetricCell], fallback: &PlayerView<'_>) -> String {
    card_metric_f64(metrics, "points_per_game")
        .map(|value| format!("{:.1}", value * 82.0))
        .unwrap_or_else(|| pace_strings(fallback).1)
}

fn format_signed(value: i64) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

fn format_seconds_mmss(value: i64) -> String {
    let value = value.max(0);
    format!("{}:{:02}", value / 60, value % 60)
}

fn run_similar(views: &[PlayerView<'_>], target_name: &str, n: usize) -> anyhow::Result<()> {
    let target = find_view(views, target_name)?;
    let view = SimilarPlayersView::from_player_views(
        views,
        target,
        n,
        target.season(),
        target.season_type(),
        true,
    );
    if let Some(empty) = &view.empty_state {
        bail!(
            "{}",
            empty.detail.clone().unwrap_or_else(|| empty.title.clone())
        );
    }

    let age_s = view
        .target
        .age
        .map(|a| a.to_string())
        .unwrap_or_else(|| "?".to_owned());
    println!(
        "SIMILAR PLAYERS TO {} ({} · {} · Age {} · {})",
        view.target.display_name,
        view.target.team_display,
        view.target.position.abbreviation(),
        age_s,
        view.target.draft_label
    );
    println!("{}", "─".repeat(72));
    println!(
        "{:<6} {:<24} {:<5} {:<4} {:<10} {:<8} Similarity",
        "Rank", "Player", "Team", "Age", "Draft", "PPG"
    );
    println!("{}", "─".repeat(72));

    for row in &view.rows {
        let age = row
            .age
            .map(|a| a.to_string())
            .unwrap_or_else(|| "?".to_owned());
        let ppg = card_metric_f64(&row.metrics, "points_per_game")
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "—".to_owned());
        println!(
            "{:<6} {:<24} {:<5} {:<4} {:<10} {:<8} {}%",
            row.rank,
            row.display_name,
            row.team_display,
            age,
            row.draft_label,
            ppg,
            row.similarity_pct
        );
    }
    println!(
        "\nCohort: {} {} players aged {}±2.",
        view.cohort_count,
        view.target.position.abbreviation(),
        age_s
    );
    Ok(())
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Gaps.4 — rewrite a `key OP value` filter expression so common
/// short keys map to their goalie-context cli_key. `gp>=15` becomes
/// `goalie-games>=15`. The rewrite happens BEFORE `parse_filter` runs,
/// so the existing parser doesn't need a goalie mode.
///
/// Only the leading key portion (before any `<`/`>`/`!`/`=`) is
/// considered. Anything that doesn't match a known short key passes
/// through unchanged.
/// Filter.OR — apply `goalie_filter_rewrite` to every atom inside a
/// boolean filter expression. Splits on `(`, `)`, and case-insensitive
/// keyword boundaries (AND/OR/NOT), rewrites each atom-segment, then
/// rejoins. Whitespace is preserved verbatim around delimiters.
fn goalie_filter_rewrite_expr(expr: &str) -> String {
    // Walk the chars and split into segments. Keywords + parens are
    // preserved verbatim; everything else is an atom we rewrite.
    let chars: Vec<char> = expr.chars().collect();
    let mut out = String::with_capacity(expr.len() + 16);
    let mut atom = String::new();
    let mut i = 0;
    let flush_atom = |atom: &mut String, out: &mut String| {
        if !atom.is_empty() {
            // Wave 11 #136 — preserve BOTH leading and trailing
            // whitespace around the rewritten core. Without leading-
            // preservation, a keyword followed by a stat-key (e.g.
            // `AND sv%>=0.9`) loses the space when the rewriter only
            // saved the trailing-side, producing `ANDsv%>=0.9` which
            // then fails the `parse_filter` ops check.
            let leading: String = atom.chars().take_while(|c| c.is_whitespace()).collect();
            let trailing: String = atom
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            out.push_str(&leading);
            out.push_str(&goalie_filter_rewrite(atom.trim()));
            out.push_str(&trailing);
            atom.clear();
        }
    };
    'outer: while i < chars.len() {
        let c = chars[i];
        if c == '(' || c == ')' {
            flush_atom(&mut atom, &mut out);
            out.push(c);
            i += 1;
            continue;
        }
        // Keyword detection at word boundary.
        let prev_is_boundary =
            i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '(' || chars[i - 1] == ')';
        if prev_is_boundary {
            for kw in ["AND", "OR", "NOT"] {
                if i + kw.len() <= chars.len() {
                    let upper: String = chars[i..i + kw.len()]
                        .iter()
                        .map(|c| c.to_ascii_uppercase())
                        .collect();
                    let next_is_boundary = match chars.get(i + kw.len()) {
                        None => true,
                        Some(&c) => c.is_whitespace() || c == '(' || c == ')',
                    };
                    if upper == kw && next_is_boundary {
                        flush_atom(&mut atom, &mut out);
                        out.push_str(&chars[i..i + kw.len()].iter().collect::<String>());
                        i += kw.len();
                        // Wave 11 #136 — must continue the outer loop;
                        // bare `continue` only re-enters the inner for,
                        // and then the outer while pushes the stale `c`
                        // captured at the top — eating the boundary
                        // whitespace and corrupting the next atom
                        // (`gp>=10 AND sv%>=0.9` → `… ANDAsv%>=0.9`).
                        continue 'outer;
                    }
                }
            }
        }
        atom.push(c);
        i += 1;
    }
    flush_atom(&mut atom, &mut out);
    out
}

fn goalie_filter_rewrite(expr: &str) -> String {
    // Find the operator boundary — first occurrence of `<`, `>`, `!`,
    // or `=` after the key. Whitespace inside the key is unusual but
    // tolerable; trim it on the key side.
    let split_at = expr.find(['<', '>', '!', '=']).unwrap_or(expr.len());
    let (key_part, rest) = expr.split_at(split_at);
    let key = key_part.trim().to_ascii_lowercase();

    let goalie_key: Option<&str> = match key.as_str() {
        // GP collisions — in goalie context these mean goalie-games.
        "gp" | "games" | "games-played" => Some("goalie-games"),
        "starts" | "gs" | "games-started" => Some("goalie-starts"),
        // No rewrite — pass the original expression through.
        _ => None,
    };
    match goalie_key {
        Some(g) => format!("{g}{rest}"),
        None => expr.to_owned(),
    }
}

fn parse_positions(s: &str) -> Vec<Position> {
    match s.to_uppercase().as_str() {
        "F" => vec![Position::Center, Position::LeftWing, Position::RightWing],
        "G" => vec![Position::Goalie],
        _ => PositionResolver::parse(s)
            .map(|(_, all)| all)
            .unwrap_or_default(),
    }
}

fn find_view<'a, 'r>(
    views: &'a [PlayerView<'r>],
    name: &str,
) -> anyhow::Result<&'a PlayerView<'r>> {
    let norm = normalize_name(name);
    views
        .iter()
        .find(|v| v.identity.name_normalized.contains(&norm))
        .with_context(|| format!("player '{name}' not found — try a partial name"))
}

fn view_age(v: &PlayerView<'_>) -> Option<u8> {
    v.identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u32>().ok())
        .map(|birth_year| 2026u32.saturating_sub(birth_year) as u8)
}

fn age_str(v: &PlayerView<'_>) -> String {
    view_age(v)
        .map(|a| a.to_string())
        .unwrap_or_else(|| "—".to_owned())
}

fn draft_str(v: &PlayerView<'_>) -> String {
    let bio = &v.identity.bio;
    match (bio.draft_year, bio.draft_round, bio.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r}#{o}"),
        (Some(y), _, _) => format!("{y}"),
        _ => "UD".to_owned(),
    }
}

fn pace_strings(v: &PlayerView<'_>) -> (String, String) {
    match v.pace_82() {
        Some(p) => (format!("{:.3}", p / 82.0), format!("{p:.1}")),
        None => ("—".to_owned(), "—".to_owned()),
    }
}

fn season_label(season: &str) -> String {
    if season.len() == 8 {
        format!("{}-{}", &season[2..4], &season[6..8])
    } else {
        season.to_owned()
    }
}

#[cfg(test)]
fn mean_std(vals: &[f64]) -> (f64, f64) {
    let n = vals.len() as f64;
    if n == 0.0 {
        return (0.0, 1.0);
    }
    let mean = vals.iter().sum::<f64>() / n;
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();
    (mean, if std < 1e-10 { 1.0 } else { std })
}

#[cfg(test)]
fn zscore(val: f64, mean: f64, std: f64) -> f64 {
    (val - mean) / std
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Return the correct English ordinal suffix for a number.
fn ordinal(n: u8) -> &'static str {
    match n % 100 {
        11..=13 => "th",
        _ => match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::history::SeasonLine;
    use icelines_core::{fixtures, identity::PlayerId, model::Season};

    // Phase Calder.3 — pre-NHL career table renderer tests.
    use icelines_core::career_history::{CareerGameType, CareerStint, LeagueAbbrev};

    fn pre_nhl_stint(
        season: u32,
        league: &str,
        team: &str,
        gp: u32,
        g: u32,
        a: u32,
    ) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: team.into(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp,
            goals: Some(g),
            assists: Some(a),
            points: Some(g + a),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    #[test]
    fn l0_query_leaders_context_line_reports_context_and_source_state() {
        let view = LeadersView::new(
            leaders_view_context(Season(20242025), SeasonType::Regular),
            LeaderKind::Skaters,
        );

        assert_eq!(
            leaders_context_line(&view),
            "Context: 20242025 regular | source roster complete"
        );
    }

    #[test]
    fn l0_query_leaders_result_line_reports_query_result_metadata() {
        let view = LeadersView::new(
            leaders_view_context(Season(20242025), SeasonType::Regular),
            LeaderKind::Skaters,
        );

        assert_eq!(
            leaders_result_line(&view, 12, 5, "goals", &["goals>=1".to_owned()]),
            "Result: total 12 | returned 0 | top 5 | sort goals | active_filters goals>=1"
        );
    }

    #[test]
    fn l0_query_leaders_warning_empty_lines_report_recovery() {
        let mut view = LeadersView::new(
            leaders_view_context(Season(20242025), SeasonType::Regular),
            LeaderKind::Skaters,
        );
        apply_leaders_warning_state(&mut view, Some("G"));

        assert_eq!(
            leaders_warning_empty_lines(&view),
            vec![
                "Warning: unsupported_filter | The leaders surface is skater-only; use goalies for goalie leaders.".to_owned(),
                "Empty: no_rows | No skater leaders".to_owned(),
                "Detail: The leaders surface is skater-only; use the goalies surface for goalie leaders.".to_owned(),
                "Recovery: Open goalie leaders -> /goalies".to_owned(),
            ]
        );
    }

    #[test]
    fn l0_query_leaders_csv_header_reports_identity_context_and_source_state() {
        assert_eq!(
            leaders_csv_header(),
            "rank,name,team,pos,gp,ppg,pts_per_82,goals_per_82,pts,goals,assists,nhl_id,season,season_type,source_kind,source_completeness,total,returned,top,sort,active_filters"
        );
    }

    /// Calder.3 / l0_render_pre_nhl_career_table_emits_header
    #[test]
    fn l0_render_pre_nhl_career_table_emits_header() {
        let stints = vec![pre_nhl_stint(20142015, "OHL", "Erie", 47, 44, 76)];
        let out = render_pre_nhl_career_table(&stints);
        assert!(
            out.contains("PRE-NHL CAREER — 1 stints"),
            "header missing in:\n{out}"
        );
        assert!(
            out.contains("Season") && out.contains("League") && out.contains("PPG"),
            "column header missing in:\n{out}"
        );
    }

    /// Calder.3 / l0_render_pre_nhl_career_table_formats_row
    /// — McDavid 2014-15 OHL Erie (44G, 76A, 120P, 2.55 PPG).
    #[test]
    fn l0_render_pre_nhl_career_table_formats_row() {
        let stints = vec![pre_nhl_stint(20142015, "OHL", "Erie", 47, 44, 76)];
        let out = render_pre_nhl_career_table(&stints);
        assert!(out.contains("14-15"), "season label missing");
        assert!(out.contains("OHL"), "league missing");
        assert!(out.contains("Erie"), "team missing");
        assert!(out.contains("47"), "GP missing");
        assert!(out.contains("44"), "G missing");
        assert!(out.contains("76"), "A missing");
        assert!(out.contains("120"), "P missing");
        assert!(out.contains("2.55"), "PPG missing");
    }

    /// Calder.3 / l0_render_pre_nhl_career_table_truncates_long_team
    /// — `U. Mass-Lowell Riverhawks` exceeds the 22-char team column.
    ///   The renderer truncates to a slice rather than overflow the
    ///   table.
    #[test]
    fn l0_render_pre_nhl_career_table_truncates_long_team() {
        let stints = vec![pre_nhl_stint(
            20132014,
            "H-East",
            "Massachusetts-Lowell Riverhawks XYZ",
            29,
            0,
            0,
        )];
        let out = render_pre_nhl_career_table(&stints);
        // The full name is too long; only the prefix should appear.
        assert!(out.contains("Massachusetts-Lowell R"));
        assert!(!out.contains("Riverhawks XYZ"));
    }

    #[test]
    fn l0_bundled_depth_disclosure_skips_modern_window() {
        let modern = icelines_fetch::bundled::MODERN_BUNDLED_SEASONS.len();
        assert!(bundled_depth_disclosure(modern).is_none());
    }

    #[test]
    fn l0_bundled_depth_disclosure_names_skeleton_and_missing_semantics() {
        let modern = icelines_fetch::bundled::MODERN_BUNDLED_SEASONS.len();
        let disclosure = bundled_depth_disclosure(modern + 1).expect("older arc needs disclosure");
        assert!(disclosure.contains("newest 5 bundled seasons"));
        assert!(disclosure.contains("historical/skeleton season totals"));
        assert!(disclosure.contains("unavailable, not zero"));
    }

    #[test]
    fn l0_career_arc_sparkline_summary_renders_chronological_metrics() {
        let lines = vec![
            SeasonLine::new("20232024", "EDM", 82, 50, 90),
            SeasonLine::new("20222023", "EDM", 82, 40, 70),
            SeasonLine::new("20212022", "EDM", 82, 30, 60),
        ];
        let summary = career_arc_sparkline_summary(&lines).expect("multi-season arc");

        assert!(summary.contains("Career trend (21-22 → 23-24)"));
        assert!(summary.contains("Pts/82"));
        assert!(summary.contains("G/82"));
        assert!(
            summary.contains('▁') || summary.contains('█'),
            "spark blocks missing: {summary}"
        );
    }

    #[test]
    fn l0_career_arc_sparkline_summary_skips_single_season() {
        let lines = vec![SeasonLine::new("20232024", "EDM", 82, 50, 90)];

        assert!(career_arc_sparkline_summary(&lines).is_none());
    }

    #[test]
    fn l0_sort_metric_parse_valid() {
        assert!(matches!(
            SortMetric::parse("pts-pace"),
            Ok(SortMetric::PtsPace)
        ));
        assert!(matches!(SortMetric::parse("ppg"), Ok(SortMetric::Ppg)));
        assert!(matches!(SortMetric::parse("g-pace"), Ok(SortMetric::GPace)));
        assert!(matches!(SortMetric::parse("gpg"), Ok(SortMetric::Gpg)));
        assert!(matches!(SortMetric::parse("pts"), Ok(SortMetric::Pts)));
        assert!(matches!(SortMetric::parse("goals"), Ok(SortMetric::Goals)));
        assert!(matches!(
            SortMetric::parse("assists"),
            Ok(SortMetric::Assists)
        ));
        assert!(matches!(SortMetric::parse("gp"), Ok(SortMetric::Gp)));
        assert!(matches!(
            SortMetric::parse("pp-pts-pace"),
            Ok(SortMetric::PpPtsPace)
        ));
        assert!(matches!(
            SortMetric::parse("pp-g-pace"),
            Ok(SortMetric::PpGPace)
        ));
        assert!(matches!(SortMetric::parse("pp-pts"), Ok(SortMetric::PpPts)));
        assert!(matches!(SortMetric::parse("pp-g"), Ok(SortMetric::PpGoals)));
        assert!(matches!(
            SortMetric::parse("sh-g-pace"),
            Ok(SortMetric::ShGPace)
        ));
        assert!(matches!(SortMetric::parse("sh-g"), Ok(SortMetric::ShGoals)));
        assert!(matches!(
            SortMetric::parse("gwg-pace"),
            Ok(SortMetric::GwgPace)
        ));
        assert!(matches!(SortMetric::parse("gwg"), Ok(SortMetric::Gwg)));
        assert!(matches!(
            SortMetric::parse("shots-pace"),
            Ok(SortMetric::ShotsPace)
        ));
        assert!(matches!(SortMetric::parse("shots"), Ok(SortMetric::Shots)));
        assert!(matches!(SortMetric::parse("sh-pct"), Ok(SortMetric::ShPct)));
        assert!(matches!(
            SortMetric::parse("plus-minus"),
            Ok(SortMetric::PlusMinus)
        ));
        assert!(matches!(SortMetric::parse("toi"), Ok(SortMetric::Toi)));
        assert!(matches!(SortMetric::parse("fo-pct"), Ok(SortMetric::FoPct)));
        assert!(matches!(
            SortMetric::parse("hits-pace"),
            Ok(SortMetric::HitsPace)
        ));
        assert!(matches!(SortMetric::parse("hits"), Ok(SortMetric::Hits)));
        assert!(matches!(
            SortMetric::parse("blocks-pace"),
            Ok(SortMetric::BlocksPace)
        ));
        assert!(matches!(
            SortMetric::parse("blocks"),
            Ok(SortMetric::Blocks)
        ));
        assert!(matches!(
            SortMetric::parse("takeaways"),
            Ok(SortMetric::Takeaways)
        ));
        assert!(matches!(
            SortMetric::parse("giveaways"),
            Ok(SortMetric::Giveaways)
        ));
        assert!(matches!(SortMetric::parse("pim"), Ok(SortMetric::Pim)));
        assert!(matches!(SortMetric::parse("xg"), Ok(SortMetric::Xg)));
        assert!(matches!(
            SortMetric::parse("xg-per-60"),
            Ok(SortMetric::XgPer60)
        ));
        assert!(matches!(SortMetric::parse("cf-pct"), Ok(SortMetric::CfPct)));
        assert!(matches!(SortMetric::parse("ff-pct"), Ok(SortMetric::FfPct)));
        assert!(matches!(
            SortMetric::parse("xgf-pct"),
            Ok(SortMetric::XgfPct)
        ));
    }

    #[test]
    fn l0_sort_metric_parse_invalid() {
        assert!(SortMetric::parse("").is_err());
        assert!(SortMetric::parse("rapm").is_err());
        assert!(SortMetric::parse("gar").is_err());
    }

    #[test]
    fn l0_mean_std_basic() {
        let (mu, sd) = mean_std(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert!((mu - 5.0).abs() < 1e-10, "mean should be 5.0");
        assert!((sd - 2.0).abs() < 1e-6, "std should be 2.0");
    }

    #[test]
    fn l0_mean_std_constant_returns_std_one() {
        let (mu, sd) = mean_std(&[3.0, 3.0, 3.0]);
        assert!((mu - 3.0).abs() < 1e-10);
        assert!(
            (sd - 1.0).abs() < 1e-10,
            "constant sequence must give sd=1.0"
        );
    }

    #[test]
    fn l0_zscore() {
        assert!((zscore(7.0, 5.0, 2.0) - 1.0).abs() < 1e-10);
        assert!((zscore(5.0, 5.0, 2.0) - 0.0).abs() < 1e-10);
        assert!((zscore(3.0, 5.0, 2.0) - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn l0_season_label() {
        assert_eq!(season_label("20252026"), "25-26");
        assert_eq!(season_label("20242025"), "24-25");
        assert_eq!(season_label("short"), "short");
    }

    #[test]
    fn l0_parse_positions_forward() {
        let pos = parse_positions("F");
        assert_eq!(pos.len(), 3);
        assert!(pos.contains(&Position::Center));
        assert!(pos.contains(&Position::LeftWing));
        assert!(pos.contains(&Position::RightWing));
    }

    #[test]
    fn l0_parse_positions_single() {
        let pos = parse_positions("C");
        assert_eq!(pos, vec![Position::Center]);
    }

    #[test]
    fn l0_draft_str_full() {
        // Hart.5c.3: rewritten on the PlayerView fixture pattern. Replaces a
        // 60-line Player struct literal.
        let id = fixtures::identity(8478402).draft(2019, 1, 3).build();
        let stats = fixtures::stats(8478402, 20242025, "EDM").build();
        let repo = fixtures::test_repo_with(id, stats);
        let v = repo
            .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
            .unwrap();
        assert_eq!(draft_str(&v), "2019 R1#3");
    }

    #[test]
    fn l0_draft_str_undrafted() {
        let id = fixtures::identity(8478402);
        // Override draft fields via builder semantics: identity() default sets
        // draft_year=2015. Build a custom identity with no draft.
        let mut id = id.build();
        id.bio.draft_year = None;
        id.bio.draft_round = None;
        id.bio.draft_overall = None;
        let stats = fixtures::stats(8478402, 20242025, "EDM").build();
        let repo = fixtures::test_repo_with(id, stats);
        let v = repo
            .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
            .unwrap();
        assert_eq!(draft_str(&v), "UD");
    }

    // ── Phase Lindsay L.5.1 — SortDispatch tests ──────────────────────

    /// Legacy strings parse via `SortDispatch::parse` to the legacy arm
    /// (byte-stable golden fence semantics).
    #[test]
    fn l0_lindsay_l5_sort_dispatch_legacy_first() {
        let m = SortDispatch::parse("pts-pace").expect("legacy parses");
        assert!(matches!(m, SortDispatch::Legacy(SortMetric::PtsPace)));
        let m = SortDispatch::parse("ppg").expect("legacy ppg parses");
        assert!(matches!(m, SortDispatch::Legacy(SortMetric::Ppg)));
        let m = SortDispatch::parse("xg").expect("legacy xg parses");
        assert!(matches!(m, SortDispatch::Legacy(SortMetric::Xg)));
    }

    /// "goals" matches BOTH the legacy alias and `StatId::Goals.cli_key()` —
    /// legacy-first precedence routes it through `SortMetric::Goals`. This
    /// preserves byte-equality for the L.3.0 stdout-golden fence.
    #[test]
    fn l0_lindsay_l5_sort_dispatch_collision_prefers_legacy() {
        let m = SortDispatch::parse("goals").expect("collision parses");
        assert!(
            matches!(m, SortDispatch::Legacy(SortMetric::Goals)),
            "legacy-first precedence preserves fence semantics"
        );
        // Same for "shots", "assists", "points", "gp", "gwg", "pim".
        for s in ["shots", "assists", "points", "gp", "gwg", "pim"] {
            assert!(
                matches!(SortDispatch::parse(s), Ok(SortDispatch::Legacy(_))),
                "{s} should route through legacy (fence preservation)"
            );
        }
    }

    /// WIRE-6 (L.5 review carry-forward made-active) — collision pin.
    ///
    /// Every `StatId::cli_key()` either routes to `Catalog(_)` or
    /// appears in this explicit allow-list of intentional legacy
    /// shadows. New collisions trip CI loudly so a contributor must
    /// either pick a non-conflicting cli_key or update the allow-list
    /// and acknowledge the legacy-first routing.
    #[test]
    fn l0_lindsay_l5_no_unsanctioned_legacy_shadows_catalog_keys() {
        // The seven currently-shadowed keys (post-L.5.1). If you're
        // adding a new StatId whose cli_key collides with a legacy
        // SortMetric alias and you want legacy-first to keep winning,
        // add the cli_key here. Otherwise pick a different cli_key.
        // Every cli_key that the legacy SortMetric::parse map matches.
        // Includes both unique aliases (e.g. "ppg" has no catalog key
        // by that exact spelling but still routes via legacy) AND any
        // cli_key that happens to equal a legacy alias spelling.
        const ALLOWED_SHADOWS: &[&str] = &[
            "goals",
            "assists",
            "points",
            "gp",
            "gwg",
            "pim",
            "shots",
            "plus-minus",
            "hits",
            "blocks",
            "takeaways",
            "giveaways",
            "toi",
            "xg",
        ];

        for sid in StatId::all() {
            let key = sid.cli_key();
            let parsed = SortDispatch::parse(key).expect("every cli_key must parse");
            match parsed {
                SortDispatch::Catalog(s) => {
                    assert_eq!(
                        s, *sid,
                        "round-trip drift: {key} parsed to a different StatId"
                    );
                }
                SortDispatch::Legacy(_) => {
                    assert!(
                        ALLOWED_SHADOWS.contains(&key),
                        "WIRE-6: cli_key `{key}` ({sid:?}) is shadowed by a \
                         legacy SortMetric alias but isn't in ALLOWED_SHADOWS. \
                         Either pick a non-conflicting cli_key or add `{key}` \
                         to ALLOWED_SHADOWS to acknowledge the legacy-first \
                         routing."
                    );
                }
            }
        }
    }

    /// Catalog-only cli_keys (no legacy alias) route through Catalog.
    #[test]
    fn l0_lindsay_l5_sort_dispatch_catalog_fallback() {
        // `points-per-game` is a StatId::PointsPerGame cli_key; not a
        // legacy alias.
        let m = SortDispatch::parse("points-per-game").expect("catalog parses");
        assert!(matches!(m, SortDispatch::Catalog(StatId::PointsPerGame)));
        // `regulation-wins` — goalie-only catalog key, no legacy.
        let m = SortDispatch::parse("regulation-wins").expect("catalog parses");
        assert!(matches!(m, SortDispatch::Catalog(StatId::RegulationWins)));
        // `pp-goals-per-60` — pure catalog (no legacy alias).
        let m = SortDispatch::parse("pp-goals-per-60").expect("catalog parses");
        assert!(matches!(m, SortDispatch::Catalog(StatId::PpGoalsPer60)));
    }

    /// Unknown key fails with the legacy help-message format.
    #[test]
    fn l0_lindsay_l5_sort_dispatch_unknown_bails() {
        let err = SortDispatch::parse("not-a-real-stat").expect_err("must fail");
        let s = format!("{err}");
        assert!(
            s.contains("unknown sort metric"),
            "error must reference legacy help text — got: {s}"
        );
    }

    /// `is_improvement()` returns true only for the legacy Improvement variant.
    #[test]
    fn l0_lindsay_l5_sort_dispatch_is_improvement() {
        let m = SortDispatch::parse("improvement").expect("ok");
        assert!(m.is_improvement());
        let m = SortDispatch::parse("trend").expect("ok");
        assert!(m.is_improvement());
        let m = SortDispatch::parse("goals").expect("ok");
        assert!(!m.is_improvement());
        let m = SortDispatch::parse("points-per-game").expect("ok");
        assert!(!m.is_improvement(), "catalog stat is never improvement");
    }

    /// L.5.2 — similarity dimensions read via catalog produce the same
    /// per-game values that the legacy formulas produced (points/gp and
    /// goals/gp). Same MIN_GP=10 gate. Without this parity, the
    /// similarity migration would silently change cohort distances.
    #[test]
    fn l0_lindsay_l5_similarity_ppg_parity_with_legacy() {
        let identity = fixtures::identity(8478402).build();
        let stats = icelines_core::season_stats::SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(icelines_core::season_stats::TeamStint {
            team: icelines_core::model::TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            goalie: None,
        })
        .with_totals(icelines_core::season_stats::StatTotals {
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            ..Default::default()
        })
        .build();
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };

        let catalog_ppg = StatId::PointsPerGame.read(&view).expect("PPG over MIN_GP");
        let expected_ppg = 110.0 / 70.0;
        assert!(
            (catalog_ppg - expected_ppg).abs() < 1e-9,
            "PPG: catalog={catalog_ppg} expected={expected_ppg}"
        );

        let catalog_gpg = StatId::GoalsPerGame.read(&view).expect("GPG over MIN_GP");
        let expected_gpg = 30.0 / 70.0;
        assert!(
            (catalog_gpg - expected_gpg).abs() < 1e-9,
            "GPG: catalog={catalog_gpg} expected={expected_gpg}"
        );
    }

    /// MIN_GP gate fires for sub-10-GP players in similarity reads.
    /// Catalog returns None — caller must apply `.unwrap_or(0.0)` so
    /// the cohort distance calc treats them as zero (parity with the
    /// legacy `.unwrap_or(0.0)`).
    #[test]
    fn l0_lindsay_l5_similarity_min_gp_gate_returns_none() {
        let identity = fixtures::identity(8478402).build();
        let stats = icelines_core::season_stats::SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(icelines_core::season_stats::TeamStint {
            team: icelines_core::model::TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 5,
            goals: 2,
            assists: 3,
            points: 5,
            goalie: None,
        })
        .with_totals(icelines_core::season_stats::StatTotals {
            gp: 5,
            goals: 2,
            assists: 3,
            points: 5,
            ..Default::default()
        })
        .build();
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };

        assert_eq!(
            StatId::PointsPerGame.read(&view),
            None,
            "below MIN_GP=10 returns None"
        );
        assert_eq!(StatId::GoalsPerGame.read(&view), None);
    }

    /// `format_catalog_cell` formats per `StatId::unit` — Count → integer,
    /// Pct → percentage with 1 decimal, Per60/Rate → 2 decimals,
    /// Inverted (GAA) → 2 decimals.
    #[test]
    fn l0_lindsay_l5_format_catalog_cell_unit_aware() {
        let identity = fixtures::identity(8478402).build();
        let stats = icelines_core::season_stats::SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(icelines_core::season_stats::TeamStint {
            team: icelines_core::model::TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            goalie: None,
        })
        .with_totals(icelines_core::season_stats::StatTotals {
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            shots: 280,
            shooting_pct: Some(0.107),
            ..Default::default()
        })
        .build();
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };

        // Count units render as integers.
        assert_eq!(format_catalog_cell(StatId::Goals, &view), "30");
        assert_eq!(format_catalog_cell(StatId::Games, &view), "70");
        // Pct renders with `%` and 1 decimal (vs L.4.3 career-cell renderer
        // which omits the `%` to save column width — different surface
        // budget here).
        assert_eq!(format_catalog_cell(StatId::ShootingPct, &view), "10.7%");
    }
}
