//! Phase 5A–5B query engine: icelines query leaders / player / compare
//!
//! Hart.5c.3 — full migration to PlayerView. SortMetric audit pinned in
//! the commit message: every metric reviewed for cast, None policy, and
//! sentinel preservation. Legacy Player paths remain only inside
//! `run_goalies` (Goalie struct lives until 5c.7).

use crate::config::Config;
use anyhow::{bail, Context};
use icelines_core::{
    filter::PlayerFilter,
    model::Position,
    name::normalize_name,
    position::PositionResolver,
    season_stats::SeasonType,
    stats_repository::PlayerView,
};
use icelines_fetch::{aggregate, career::load_career, snapshot::SnapshotStore};

// ── Sort metric ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum SortMetric {
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
                    if rate { format!("{:.3}", p / 82.0) } else { format!("{p:.1}") }
                }
                None => "—".to_owned(),
            },
            Self::Ppg => match v.pace_82() {
                Some(p) => format!("{:.3}", p / 82.0),
                None => "—".to_owned(),
            },
            Self::GPace => match v.goals_per_82() {
                Some(g) => {
                    if rate { format!("{:.3}", g / 82.0) } else { format!("{g:.1}") }
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
                if pm >= 0 { format!("+{pm}") } else { pm.to_string() }
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
            Self::Xg => v.xg().map(|val| format!("{val:.2}")).unwrap_or_else(|| "—".to_owned()),
            Self::XgPer60 => v.xg_per_60().map(|val| format!("{val:.2}")).unwrap_or_else(|| "—".to_owned()),
            Self::CfPct => v.cf_pct().map(|val| format!("{val:.1}%")).unwrap_or_else(|| "—".to_owned()),
            Self::FfPct => v.ff_pct().map(|val| format!("{val:.1}%")).unwrap_or_else(|| "—".to_owned()),
            Self::XgfPct => v.xgf_pct().map(|val| format!("{val:.1}%")).unwrap_or_else(|| "—".to_owned()),
            Self::Improvement => "—".to_owned(), // displayed by special-case handler
        }
    }

    fn header(self, rate: bool) -> &'static str {
        match self {
            Self::PtsPace => if rate { "PPG" } else { "Pts/82" },
            Self::Ppg => "PPG",
            Self::GPace => if rate { "GPG" } else { "G/82" },
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
    pub csv: bool,
    /// Phase Lindsay L.3.1 — generic stat filters. Each string is parsed
    /// via `icelines_core::stats_catalog::parse_filter` and added to
    /// `PlayerFilter.stat_filters`; `normalize_stat_filters` runs before
    /// apply.
    pub filters: Vec<String>,
}

// ── icelines query leaders ────────────────────────────────────────────────────

pub async fn run_leaders(args: LeadersArgs) -> anyhow::Result<()> {
    let metric = SortMetric::parse(&args.sort)?;

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

    // Phase Lindsay L.3.1 — generic stat filters. Each --filter flag
    // routes through `parse_filter` (which gates NaN/inf at construction
    // and rejects malformed grammar with a 7-variant error). Multiple
    // --filter flags accumulate (implicit AND); `normalize_stat_filters`
    // collapses Min+Min/Max+Max to tightest bounds before apply.
    for raw in &args.filters {
        let f = icelines_core::stats_catalog::parse_filter(raw)
            .with_context(|| format!("--filter {raw:?}"))?;
        filter.stat_filters.push(f);
    }
    filter.normalize_stat_filters();

    let mut matched: Vec<PlayerView<'_>> = filter.apply_views(all_views.iter().copied());

    // gp_max (not in PlayerFilter — inline here)
    if let Some(gp_max) = args.gp_max {
        matched.retain(|v| v.gp() <= gp_max);
    }

    // Contract filters
    let wants_contract = args.ufa || args.rfa || args.elc || args.expiry_year.is_some();
    if args.ufa {
        matched.retain(|v| v.contract_expiry_type().map(|t| t.eq_ignore_ascii_case("UFA")).unwrap_or(false));
    }
    if args.rfa {
        matched.retain(|v| v.contract_expiry_type().map(|t| t.eq_ignore_ascii_case("RFA")).unwrap_or(false));
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
            matches!(
                f.stat.category(),
                Possession | OnIceGoals | TimeOnIce
            ) || matches!(
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
                args.season.as_deref().unwrap_or(icelines_core::CURRENT_SEASON_STR),
            );
        }
    }

    // Improvement sort requires the Y/Y delta map.
    if matches!(metric, SortMetric::Improvement) {
        let imp_map = aggregate::load_improvement_map();
        matched.sort_by(|a, b| {
            let da = imp_map.get(&a.identity.id.0).copied().unwrap_or(f64::NEG_INFINITY);
            let db = imp_map.get(&b.identity.id.0).copied().unwrap_or(f64::NEG_INFINITY);
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });
        let total_matched = matched.len();
        let results: Vec<PlayerView<'_>> = matched.iter().copied().take(args.top).collect();

        if args.json {
            println!("{}", leaders_json(&results));
            return Ok(());
        }
        print_improvement_table(&results, &imp_map, args.top, total_matched, args.seasons);
        return Ok(());
    }

    // Warn if a realtime/MoneyPuck metric has no data (all None/zero).
    let data_missing = match metric {
        SortMetric::HitsPace
        | SortMetric::Hits
        | SortMetric::BlocksPace
        | SortMetric::Blocks
        | SortMetric::Takeaways
        | SortMetric::Giveaways
        | SortMetric::Pim => matched
            .iter()
            .all(|v| v.hits().unwrap_or(0) == 0
                && v.blocked_shots().unwrap_or(0) == 0
                && v.takeaways().unwrap_or(0) == 0),
        SortMetric::Xg
        | SortMetric::XgPer60
        | SortMetric::CfPct
        | SortMetric::FfPct
        | SortMetric::XgfPct => matched
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
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        matched.sort_by(|a, b| {
            metric
                .sort_value(b)
                .partial_cmp(&metric.sort_value(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let total_matched = matched.len();
    let results: Vec<PlayerView<'_>> = matched.into_iter().take(args.top).collect();

    if args.json {
        println!("{}", leaders_json(&results));
        return Ok(());
    }
    if args.csv {
        leaders_csv(&results);
        return Ok(());
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
        &results,
        &percentiles,
        metric,
        args.rate,
        args.top,
        total_matched,
        args.seasons,
    );
    Ok(())
}

fn leaders_table(
    views: &[PlayerView<'_>],
    percentiles: &[Option<u8>],
    metric: SortMetric,
    rate: bool,
    top: usize,
    total: usize,
    seasons: u8,
) {
    let col = if seasons > 1 {
        format!("{} ({}yr)", metric.header(rate), seasons)
    } else {
        metric.header(rate).to_owned()
    };
    let show_pct = percentiles.iter().any(|p| p.is_some());

    if show_pct {
        println!(
            "{:<4} {:<24} {:<5} {:<4} {:<4} {:<10} {:<6}",
            "Rank", "Player", "Team", "Pos", "GP", col, "Pctl"
        );
        println!("{}", "─".repeat(61));
        for (i, (v, pct)) in views.iter().zip(percentiles.iter()).enumerate() {
            let val = metric.display(v, rate);
            let pct_s = pct.map(|x| format!("{x}{}", ordinal(x))).unwrap_or_else(|| "—".to_owned());
            println!(
                "{:<4} {:<24} {:<5} {:<4} {:<4} {:<10} {:<6}",
                i + 1,
                v.identity.full_name,
                v.team_display(),
                v.position().abbreviation(),
                v.gp(),
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
        for (i, v) in views.iter().enumerate() {
            let val = metric.display(v, rate);
            println!(
                "{:<4} {:<24} {:<5} {:<4} {:<4} {:<10}",
                i + 1,
                v.identity.full_name,
                v.team_display(),
                v.position().abbreviation(),
                v.gp(),
                val
            );
        }
    }
    println!("\n{total} matched, showing {}.", views.len().min(top));
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
        let curr_ppg = v.pace_82().map(|p| format!("{:.3}", p / 82.0)).unwrap_or_else(|| "—".to_owned());
        let prior_ppg = format!("{:.3}", v.pace_82().map(|p| p / 82.0).unwrap_or(0.0) - delta);
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

fn leaders_json(views: &[PlayerView<'_>]) -> String {
    let rows: Vec<serde_json::Value> = views
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let totals = &v.stats.totals;
            serde_json::json!({
                "rank": i + 1,
                "name": v.identity.full_name,
                "team": v.team_display(),
                "pos": v.position().abbreviation(),
                "gp": v.gp(),
                "ppg": v.pace_82().map(|p| round2(p / 82.0)),
                "pts_per_82": v.pace_82().map(round1),
                "goals_per_82": v.goals_per_82().map(round1),
                "season_pts": totals.points,
                "season_goals": totals.goals,
                "season_assists": totals.assists,
            })
        })
        .collect();
    serde_json::to_string_pretty(&rows).unwrap_or_default()
}

fn leaders_csv(views: &[PlayerView<'_>]) {
    println!("rank,name,team,pos,gp,ppg,pts_per_82,goals_per_82,pts,goals,assists");
    for (i, v) in views.iter().enumerate() {
        let totals = &v.stats.totals;
        println!(
            "{},{},{},{},{},{:.3},{:.1},{:.1},{},{},{}",
            i + 1,
            v.identity.full_name,
            v.team_display(),
            v.position().abbreviation(),
            v.gp(),
            v.pace_82().map(|p| p / 82.0).unwrap_or(0.0),
            v.pace_82().unwrap_or(0.0),
            v.goals_per_82().unwrap_or(0.0),
            totals.points,
            totals.goals,
            totals.assists,
        );
    }
}

fn position_percentile(all: &[PlayerView<'_>], target: &PlayerView<'_>, metric: SortMetric) -> Option<u8> {
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

pub async fn run_player(
    name: String,
    breakdown: String,
    percentiles: bool,
    last_n: Option<u32>,
    season: Option<String>,
    season_type: SeasonType,
) -> anyhow::Result<()> {
    let (outcome, season_key, season_type) =
        crate::commands::players::load_repo_for_season(season.as_deref(), Some(season_type))?;
    let repo = &outcome.repo;
    let all_views: Vec<PlayerView<'_>> = repo.skaters(season_key, season_type).collect();
    let v = find_view(&all_views, &name)?;

    let age = age_str(v);
    let draft = draft_str(v);
    println!(
        "PLAYER PROFILE — {} ({} · {} · Age {} · {})",
        v.identity.full_name,
        v.team_display(),
        v.position().abbreviation(),
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
            print_current_stats(v);
            if percentiles {
                print_percentile(&all_views, v);
            }
            print_career(v).await;
        }
        "situation" => {
            println!("  Situational breakdown (5v5/PP/PK) requires Phase 5C shift data.");
            println!("  Currently available: all-situations stats only.");
            println!();
            print_current_stats(v);
            if percentiles {
                print_percentile(&all_views, v);
            }
        }
        other => bail!("unknown breakdown '{other}' — valid: career, situation"),
    }
    Ok(())
}

fn print_current_stats(v: &PlayerView<'_>) {
    let totals = &v.stats.totals;
    let (ppg, proj) = pace_strings(v);
    let toi = v.toi_mmss().unwrap_or_else(|| "—".to_owned());
    let sh_pct = totals
        .shooting_pct
        .map(|val| format!("{:.1}%", val * 100.0))
        .unwrap_or_else(|| "—".to_owned());
    let pm_val = v.plus_minus();
    let pm = if pm_val >= 0 { format!("+{pm_val}") } else { pm_val.to_string() };
    println!("CURRENT SEASON");
    println!(
        "  GP {:<4}  G {:<4}  A {:<4}  Pts {:<4}  PPG {}  Pts/82 {}",
        v.gp(),
        totals.goals,
        totals.assists,
        totals.points,
        ppg,
        proj
    );
    println!(
        "  PP: {:<3}G / {:<3}Pts   SH: {}G   GWG: {}   Shots: {}   SH%: {}",
        totals.pp_goals, totals.pp_points, totals.sh_goals, totals.gwg, v.shots(), sh_pct
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

    if let Some(expiry_type) = v.contract_expiry_type() {
        let expiry_year = v
            .contract_expiry_year()
            .map(|y| y.to_string())
            .unwrap_or_else(|| "?".to_owned());
        let salary_str = v
            .contract_salary()
            .map(|s| format!("${:.1}M", s as f64 / 1_000_000.0))
            .unwrap_or_else(|| "—".to_owned());
        println!(
            "  Contract: {} expires {}  Salary: {}",
            expiry_type.to_uppercase(),
            expiry_year,
            salary_str
        );
        println!();
    }
}

fn print_percentile(all: &[PlayerView<'_>], target: &PlayerView<'_>) {
    let peers: Vec<&PlayerView<'_>> = all
        .iter()
        .filter(|v| v.position() == target.position() && v.is_rankable())
        .collect();
    if peers.is_empty() {
        return;
    }
    let target_val = target.pace_82().unwrap_or(0.0);
    let n_better = peers
        .iter()
        .filter(|v| v.pace_82().unwrap_or(0.0) > target_val)
        .count();
    let rank = n_better + 1;
    let pct = ((1.0 - n_better as f64 / peers.len() as f64) * 100.0) as u8;
    println!(
        "LEAGUE RANK  #{rank} of {} {}'s  ({pct}{} percentile by Pts/82)",
        peers.len(),
        target.position().abbreviation(),
        ordinal(pct)
    );
    println!();
}

async fn print_career(v: &PlayerView<'_>) {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(_) => return,
    };
    let store = SnapshotStore::new(cfg.snapshot_dir());
    match load_career(&v.identity.full_name, 5, &store) {
        Some(career) => {
            println!("CAREER ARC — {} seasons", career.seasons.len());
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
}

/// JSON / CSV output row for `query goalies`. Hart.5c.7: stable shape
/// for CLI consumers, decoupled from the icelines-core model. Mirrors
/// the field set the legacy Goalie struct produced via serde.
#[derive(serde::Serialize)]
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
}

pub async fn run_goalies(args: GoaliesArgs) -> anyhow::Result<()> {
    use icelines_core::stats_repository::PlayerView;
    use icelines_fetch::snapshot::SnapshotStore;

    if args.json && args.csv {
        bail!("--json and --csv are mutually exclusive");
    }

    let cfg = Config::load()?;
    let season = match args.season.as_deref() {
        Some(s) => {
            crate::commands::players::validate_bundled_season(s)?;
            s.to_owned()
        }
        None => cfg.season_str(),
    };
    let season_u32: u32 = season
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{season}' is not a YYYYZZZZ id"))?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = icelines_fetch::stats_loader::load_into_repo(
        icelines_core::model::Season(season_u32),
        args.season_type,
        &store,
    )
    .map_err(|e| {
        let hint = match args.season_type {
            icelines_core::season_stats::SeasonType::Regular => "Try: icelines fetch goalies",
            icelines_core::season_stats::SeasonType::Playoff => {
                "Try: icelines fetch goalies --type playoff"
            }
        };
        anyhow::anyhow!("{e}\n  {hint}")
    })?;

    let mut views: Vec<PlayerView<'_>> = outcome
        .repo
        .goalies(icelines_core::model::Season(season_u32), args.season_type)
        .filter(|v| v.gp() >= args.min_gp)
        .collect();

    if let Some(team) = args.team.as_deref() {
        let abbrev = team.to_ascii_uppercase();
        views.retain(|v| v.team_display() == abbrev);
    }

    use std::cmp::Ordering;
    let sort_key = args.sort.to_ascii_lowercase();
    views.sort_by(|a, b| {
        let sa = a.stats.goalie.as_ref();
        let sb = b.stats.goalie.as_ref();
        match sort_key.as_str() {
            "sv-pct" | "svpct" | "sv%" => {
                let av = sa.and_then(|s| s.save_pct).unwrap_or(0.0);
                let bv = sb.and_then(|s| s.save_pct).unwrap_or(0.0);
                bv.partial_cmp(&av).unwrap_or(Ordering::Equal)
            }
            "gaa" => {
                let av = sa.and_then(|s| s.goals_against_average).unwrap_or(f32::INFINITY);
                let bv = sb.and_then(|s| s.goals_against_average).unwrap_or(f32::INFINITY);
                av.partial_cmp(&bv).unwrap_or(Ordering::Equal)
            }
            "wins" | "w" => sb.map(|s| s.wins).unwrap_or(0).cmp(&sa.map(|s| s.wins).unwrap_or(0)),
            "gp" => b.gp().cmp(&a.gp()),
            "saves" => sb.map(|s| s.saves).unwrap_or(0).cmp(&sa.map(|s| s.saves).unwrap_or(0)),
            "so" | "shutouts" => sb.map(|s| s.shutouts).unwrap_or(0).cmp(&sa.map(|s| s.shutouts).unwrap_or(0)),
            other => {
                eprintln!("  Hint: unknown sort '{other}' — falling back to sv-pct.");
                let av = sa.and_then(|s| s.save_pct).unwrap_or(0.0);
                let bv = sb.and_then(|s| s.save_pct).unwrap_or(0.0);
                bv.partial_cmp(&av).unwrap_or(Ordering::Equal)
            }
        }
    });
    views.truncate(args.top);

    let rows: Vec<GoalieRow> = views
        .iter()
        .map(|v| {
            let s = v.stats.goalie.as_ref();
            GoalieRow {
                nhl_id: v.identity.id.0,
                full_name: v.full_name().to_owned(),
                team: v.team_display().to_owned(),
                games_played: v.gp(),
                wins: s.map(|s| s.wins).unwrap_or(0),
                losses: s.map(|s| s.losses).unwrap_or(0),
                ot_losses: s.and_then(|s| s.ot_losses),
                save_pct: s.and_then(|s| s.save_pct),
                goals_against_average: s.and_then(|s| s.goals_against_average),
                shutouts: s.map(|s| s.shutouts).unwrap_or(0),
                saves: s.map(|s| s.saves).unwrap_or(0),
            }
        })
        .collect();

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).context("serializing goalies to JSON")?
        );
        return Ok(());
    }
    if args.csv {
        println!("rank,goalie,team,gp,wins,losses,ot_losses,sv_pct,gaa,so,saves");
        for (i, row) in rows.iter().enumerate() {
            println!(
                "{},\"{}\",{},{},{},{},{},{:.4},{:.3},{},{}",
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
            );
        }
        return Ok(());
    }

    println!(
        "{:<4} {:<24} {:<5} {:<4} {:<10} {:<6} {:<6} {:<3} {:<6}",
        "Rank", "Goalie", "Team", "GP", "W-L-OT", "SV%", "GAA", "SO", "Saves"
    );
    println!("{}", "─".repeat(80));
    for (i, row) in rows.iter().enumerate() {
        let record = match row.ot_losses {
            Some(otl) => format!("{}-{}-{}", row.wins, row.losses, otl),
            None => format!("{}-{}", row.wins, row.losses),
        };
        let sv = row.save_pct.map(|v| format!("{v:.3}")).unwrap_or_else(|| "—".to_owned());
        let gaa = row.goals_against_average.map(|v| format!("{v:.2}")).unwrap_or_else(|| "—".to_owned());
        println!(
            "{:<4} {:<24} {:<5} {:<4} {:<10} {:<6} {:<6} {:<3} {:<6}",
            i + 1,
            row.full_name.chars().take(24).collect::<String>(),
            row.team,
            row.games_played,
            record,
            sv,
            gaa,
            row.shutouts,
            row.saves,
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

pub async fn run_compare(
    player1: String,
    player2: Option<String>,
    similar: Option<usize>,
    _by: String,
    season: Option<String>,
    season_type: SeasonType,
) -> anyhow::Result<()> {
    let (outcome, season_key, season_type) =
        crate::commands::players::load_repo_for_season(season.as_deref(), Some(season_type))?;
    let repo = &outcome.repo;
    let all_views: Vec<PlayerView<'_>> = repo.skaters(season_key, season_type).collect();

    if let Some(n) = similar {
        run_similar(&all_views, &player1, n)
    } else if let Some(p2_name) = player2 {
        let v1 = find_view(&all_views, &player1)?;
        let v2 = find_view(&all_views, &p2_name)?;
        if v1.identity.name_normalized == v2.identity.name_normalized {
            eprintln!(
                "  Note: both sides resolved to the same player ({}).",
                v1.identity.full_name
            );
        }
        print_head_to_head(v1, v2);
        Ok(())
    } else {
        bail!("provide a second player name, or use --similar N for similarity search")
    }
}

fn print_head_to_head(v1: &PlayerView<'_>, v2: &PlayerView<'_>) {
    let (ppg1, proj1) = pace_strings(v1);
    let (ppg2, proj2) = pace_strings(v2);
    let col = 28usize;

    println!("{:<col$} {:<col$}", v1.identity.full_name, v2.identity.full_name);
    println!("{:<col$} {:<col$}", v1.team_display(), v2.team_display());
    println!("{}", "─".repeat(col * 2 + 2));

    let row = |label: &str, c1: &str, c2: &str| {
        println!("  {:<18} {:<col$} {:<col$}", label, c1, c2, col = col - 2);
    };

    row("Position", v1.position().abbreviation(), v2.position().abbreviation());
    row("Age", &age_str(v1), &age_str(v2));
    row("Draft", &draft_str(v1), &draft_str(v2));
    row("GP", &v1.gp().to_string(), &v2.gp().to_string());
    row("PPG", &ppg1, &ppg2);
    row("Pts/82", &proj1, &proj2);
    row(
        "Goals/82",
        &v1.goals_per_82().map(|g| format!("{g:.1}")).unwrap_or_else(|| "—".to_owned()),
        &v2.goals_per_82().map(|g| format!("{g:.1}")).unwrap_or_else(|| "—".to_owned()),
    );
    row("PP Points", &v1.stats.totals.pp_points.to_string(), &v2.stats.totals.pp_points.to_string());
    row("PP Goals", &v1.stats.totals.pp_goals.to_string(), &v2.stats.totals.pp_goals.to_string());
    row("GWG", &v1.stats.totals.gwg.to_string(), &v2.stats.totals.gwg.to_string());
    row("Shots", &v1.shots().to_string(), &v2.shots().to_string());
    row(
        "SH%",
        &v1.stats.totals.shooting_pct.map(|val| format!("{:.1}%", val * 100.0)).unwrap_or_else(|| "—".to_owned()),
        &v2.stats.totals.shooting_pct.map(|val| format!("{:.1}%", val * 100.0)).unwrap_or_else(|| "—".to_owned()),
    );
    let pm_str = |pm: i32| if pm >= 0 { format!("+{pm}") } else { pm.to_string() };
    row("+/-", &pm_str(v1.plus_minus()), &pm_str(v2.plus_minus()));
    row("TOI/g", &v1.toi_mmss().unwrap_or_else(|| "—".to_owned()), &v2.toi_mmss().unwrap_or_else(|| "—".to_owned()));
    row(
        "Contract",
        &v1.contract_expiry_type().map(|t| t.to_uppercase()).unwrap_or_else(|| "—".to_owned()),
        &v2.contract_expiry_type().map(|t| t.to_uppercase()).unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Expires",
        &v1.contract_expiry_year().map(|y| y.to_string()).unwrap_or_else(|| "—".to_owned()),
        &v2.contract_expiry_year().map(|y| y.to_string()).unwrap_or_else(|| "—".to_owned()),
    );
}

fn run_similar(views: &[PlayerView<'_>], target_name: &str, n: usize) -> anyhow::Result<()> {
    let target = find_view(views, target_name)?;
    let target_age = view_age(target);

    let cohort: Vec<&PlayerView<'_>> = views
        .iter()
        .filter(|v| {
            v.position() == target.position()
                && v.is_rankable()
                && view_age(v)
                    .zip(target_age)
                    .map(|(a, ta)| (a as i32 - ta as i32).abs() <= 2)
                    .unwrap_or(false)
        })
        .collect();

    if cohort.len() < 3 {
        bail!(
            "cohort too small ({} players) for similarity search — need at least 3",
            cohort.len()
        );
    }

    let ppgs: Vec<f64> = cohort
        .iter()
        .map(|v| v.pace_82().map(|p| p / 82.0).unwrap_or(0.0))
        .collect();
    let gpgs: Vec<f64> = cohort
        .iter()
        .map(|v| v.goals_per_82().map(|g| g / 82.0).unwrap_or(0.0))
        .collect();
    let picks: Vec<f64> = cohort
        .iter()
        .map(|v| v.identity.bio.draft_overall.map(|pk| 1.0 - (pk as f64 - 1.0) / 399.0).unwrap_or(0.0))
        .collect();

    let (ppg_mu, ppg_sd) = mean_std(&ppgs);
    let (gpg_mu, gpg_sd) = mean_std(&gpgs);
    let (pick_mu, pick_sd) = mean_std(&picks);

    let target_norm = &target.identity.name_normalized;
    let ti = cohort
        .iter()
        .position(|v| &v.identity.name_normalized == target_norm)
        .unwrap_or(0);
    let tz_ppg = zscore(ppgs[ti], ppg_mu, ppg_sd);
    let tz_gpg = zscore(gpgs[ti], gpg_mu, gpg_sd);
    let tz_pick = zscore(picks[ti], pick_mu, pick_sd);

    let mut scored: Vec<(&PlayerView<'_>, f64)> = cohort
        .iter()
        .zip(ppgs.iter())
        .zip(gpgs.iter())
        .zip(picks.iter())
        .map(|(((v, &ppg), &gpg), &pick)| {
            let dz_ppg = zscore(ppg, ppg_mu, ppg_sd) - tz_ppg;
            let dz_gpg = zscore(gpg, gpg_mu, gpg_sd) - tz_gpg;
            let dz_pick = zscore(pick, pick_mu, pick_sd) - tz_pick;
            let dist = (dz_ppg * dz_ppg + dz_gpg * dz_gpg + dz_pick * dz_pick).sqrt();
            (*v, dist)
        })
        .collect();

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.retain(|(v, _)| &v.identity.name_normalized != target_norm);

    let age_s = target_age.map(|a| a.to_string()).unwrap_or_else(|| "?".to_owned());
    let draft_s = draft_str(target);
    println!(
        "SIMILAR PLAYERS TO {} ({} · {} · Age {} · {})",
        target.identity.full_name,
        target.team_display(),
        target.position().abbreviation(),
        age_s,
        draft_s
    );
    println!("{}", "─".repeat(72));
    println!(
        "{:<6} {:<24} {:<5} {:<4} {:<10} {:<8} Similarity",
        "Rank", "Player", "Team", "Age", "Draft", "PPG"
    );
    println!("{}", "─".repeat(72));

    for (i, (v, dist)) in scored.iter().take(n).enumerate() {
        let age = view_age(v).map(|a| a.to_string()).unwrap_or_else(|| "?".to_owned());
        let draft = draft_str(v);
        let ppg = v.pace_82().map(|p| format!("{:.3}", p / 82.0)).unwrap_or_else(|| "—".to_owned());
        let sim = (100.0 / (1.0 + dist)) as u32;
        println!(
            "{:<6} {:<24} {:<5} {:<4} {:<10} {:<8} {sim}%",
            i + 1,
            v.identity.full_name,
            v.team_display(),
            age,
            draft,
            ppg
        );
    }
    println!(
        "\nCohort: {} {} players aged {}±2.",
        cohort.len(),
        target.position().abbreviation(),
        age_s
    );
    Ok(())
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn parse_positions(s: &str) -> Vec<Position> {
    match s.to_uppercase().as_str() {
        "F" => vec![Position::Center, Position::LeftWing, Position::RightWing],
        "G" => vec![Position::Goalie],
        _ => PositionResolver::parse(s)
            .map(|(_, all)| all)
            .unwrap_or_default(),
    }
}

fn find_view<'a, 'r>(views: &'a [PlayerView<'r>], name: &str) -> anyhow::Result<&'a PlayerView<'r>> {
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
    view_age(v).map(|a| a.to_string()).unwrap_or_else(|| "—".to_owned())
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
        Some(p) => (
            format!("{:.3}", p / 82.0),
            format!("{p:.1}"),
        ),
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
    use icelines_core::{fixtures, identity::PlayerId, model::Season};

    #[test]
    fn l0_sort_metric_parse_valid() {
        assert!(matches!(SortMetric::parse("pts-pace"), Ok(SortMetric::PtsPace)));
        assert!(matches!(SortMetric::parse("ppg"), Ok(SortMetric::Ppg)));
        assert!(matches!(SortMetric::parse("g-pace"), Ok(SortMetric::GPace)));
        assert!(matches!(SortMetric::parse("gpg"), Ok(SortMetric::Gpg)));
        assert!(matches!(SortMetric::parse("pts"), Ok(SortMetric::Pts)));
        assert!(matches!(SortMetric::parse("goals"), Ok(SortMetric::Goals)));
        assert!(matches!(SortMetric::parse("assists"), Ok(SortMetric::Assists)));
        assert!(matches!(SortMetric::parse("gp"), Ok(SortMetric::Gp)));
        assert!(matches!(SortMetric::parse("pp-pts-pace"), Ok(SortMetric::PpPtsPace)));
        assert!(matches!(SortMetric::parse("pp-g-pace"), Ok(SortMetric::PpGPace)));
        assert!(matches!(SortMetric::parse("pp-pts"), Ok(SortMetric::PpPts)));
        assert!(matches!(SortMetric::parse("pp-g"), Ok(SortMetric::PpGoals)));
        assert!(matches!(SortMetric::parse("sh-g-pace"), Ok(SortMetric::ShGPace)));
        assert!(matches!(SortMetric::parse("sh-g"), Ok(SortMetric::ShGoals)));
        assert!(matches!(SortMetric::parse("gwg-pace"), Ok(SortMetric::GwgPace)));
        assert!(matches!(SortMetric::parse("gwg"), Ok(SortMetric::Gwg)));
        assert!(matches!(SortMetric::parse("shots-pace"), Ok(SortMetric::ShotsPace)));
        assert!(matches!(SortMetric::parse("shots"), Ok(SortMetric::Shots)));
        assert!(matches!(SortMetric::parse("sh-pct"), Ok(SortMetric::ShPct)));
        assert!(matches!(SortMetric::parse("plus-minus"), Ok(SortMetric::PlusMinus)));
        assert!(matches!(SortMetric::parse("toi"), Ok(SortMetric::Toi)));
        assert!(matches!(SortMetric::parse("fo-pct"), Ok(SortMetric::FoPct)));
        assert!(matches!(SortMetric::parse("hits-pace"), Ok(SortMetric::HitsPace)));
        assert!(matches!(SortMetric::parse("hits"), Ok(SortMetric::Hits)));
        assert!(matches!(SortMetric::parse("blocks-pace"), Ok(SortMetric::BlocksPace)));
        assert!(matches!(SortMetric::parse("blocks"), Ok(SortMetric::Blocks)));
        assert!(matches!(SortMetric::parse("takeaways"), Ok(SortMetric::Takeaways)));
        assert!(matches!(SortMetric::parse("giveaways"), Ok(SortMetric::Giveaways)));
        assert!(matches!(SortMetric::parse("pim"), Ok(SortMetric::Pim)));
        assert!(matches!(SortMetric::parse("xg"), Ok(SortMetric::Xg)));
        assert!(matches!(SortMetric::parse("xg-per-60"), Ok(SortMetric::XgPer60)));
        assert!(matches!(SortMetric::parse("cf-pct"), Ok(SortMetric::CfPct)));
        assert!(matches!(SortMetric::parse("ff-pct"), Ok(SortMetric::FfPct)));
        assert!(matches!(SortMetric::parse("xgf-pct"), Ok(SortMetric::XgfPct)));
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
        assert!((sd - 1.0).abs() < 1e-10, "constant sequence must give sd=1.0");
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
        let id = fixtures::identity(8478402)
            .draft(2019, 1, 3)
            .build();
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
}
