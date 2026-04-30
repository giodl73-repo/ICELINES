//! Phase 5A–5B query engine: icelines query leaders / player / compare

// Phase 8f: query commands now use load_all_players_for_season for the
// optional --season override. The unconditional helper is no longer needed.
use crate::config::Config;
use anyhow::{bail, Context};
use icelines_core::{
    filter::PlayerFilter,
    model::{Player, Position},
    name::normalize_name,
    position::PositionResolver,
};
use icelines_fetch::{aggregate, career::load_career, snapshot::SnapshotStore};

// ── Sort metric ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum SortMetric {
    // All-situations pace
    PtsPace, Ppg,
    GPace, Gpg,
    // Raw season totals
    Pts, Goals, Assists, Gp,
    // Power play
    PpPtsPace, PpGPace, PpPts, PpGoals,
    // Shorthanded
    ShGPace, ShGoals,
    // Other
    GwgPace, Gwg,
    // Shot metrics
    ShotsPace, Shots, ShPct,
    // Two-way
    PlusMinus, Toi, FoPct,
    // Realtime physical stats
    HitsPace, Hits, BlocksPace, Blocks, Takeaways, Giveaways, Pim,
    // MoneyPuck advanced metrics
    Xg, XgPer60, CfPct, FfPct, XgfPct,
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

    fn sort_value(self, p: &Player) -> f64 {
        match self {
            Self::PtsPace | Self::Ppg   => p.pace_score.map(|s| s.pace_82).unwrap_or(0.0),
            Self::GPace   | Self::Gpg   => p.pace_score.map(|s| s.goals_per_82).unwrap_or(0.0),
            Self::Pts     => p.season_points as f64,
            Self::Goals   => p.season_goals as f64,
            Self::Assists => p.season_assists as f64,
            Self::Gp      => p.gp().unwrap_or(0) as f64,
            // PP
            Self::PpPtsPace => p.pp_points_per_82().unwrap_or(0.0),
            Self::PpGPace   => p.pp_goals_per_82().unwrap_or(0.0),
            Self::PpPts     => p.pp_points as f64,
            Self::PpGoals   => p.pp_goals as f64,
            // SH
            Self::ShGPace   => p.sh_goals_per_82().unwrap_or(0.0),
            Self::ShGoals   => p.sh_goals as f64,
            // GWG
            Self::GwgPace   => p.gwg_per_82().unwrap_or(0.0),
            Self::Gwg       => p.gwg as f64,
            // Shots
            Self::ShotsPace => p.shots_per_82().unwrap_or(0.0),
            Self::Shots     => p.shots as f64,
            Self::ShPct     => p.shooting_pct.unwrap_or(0.0) as f64,
            // Two-way
            Self::PlusMinus => p.plus_minus as f64,
            Self::Toi       => p.toi_per_game_sec.unwrap_or(0.0) as f64,
            Self::FoPct     => p.faceoff_win_pct.unwrap_or(0.0) as f64,
            // Realtime
            Self::HitsPace   => p.hits_per_82().unwrap_or(0.0),
            Self::Hits        => p.hits as f64,
            Self::BlocksPace  => p.blocked_shots_per_82().unwrap_or(0.0),
            Self::Blocks      => p.blocked_shots as f64,
            Self::Takeaways   => p.takeaways as f64,
            Self::Giveaways   => p.giveaways as f64,
            Self::Pim         => p.pim as f64,
            // MoneyPuck
            Self::Xg          => p.xg.unwrap_or(0.0) as f64,
            Self::XgPer60     => p.xg_per_60.unwrap_or(0.0) as f64,
            Self::CfPct       => p.cf_pct_5v5.unwrap_or(50.0) as f64,
            Self::FfPct       => p.ff_pct_5v5.unwrap_or(50.0) as f64,
            Self::XgfPct      => p.xgf_pct_5v5.unwrap_or(50.0) as f64,
            // Improvement: sorted separately via improvement_map; value unused for sort
            Self::Improvement => 0.0,
        }
    }

    fn display(self, p: &Player, rate: bool) -> String {
        match self {
            Self::PtsPace => match p.pace_score {
                Some(s) => if rate { format!("{:.3}", s.pace_82 / 82.0) } else { format!("{:.1}", s.pace_82) },
                None    => "—".to_owned(),
            },
            Self::Ppg => match p.pace_score {
                Some(s) => format!("{:.3}", s.pace_82 / 82.0),
                None    => "—".to_owned(),
            },
            Self::GPace => match p.pace_score {
                Some(s) => if rate { format!("{:.3}", s.goals_per_82 / 82.0) } else { format!("{:.1}", s.goals_per_82) },
                None    => "—".to_owned(),
            },
            Self::Gpg => match p.pace_score {
                Some(s) => format!("{:.3}", s.goals_per_82 / 82.0),
                None    => "—".to_owned(),
            },
            Self::Pts     => p.season_points.to_string(),
            Self::Goals   => p.season_goals.to_string(),
            Self::Assists => p.season_assists.to_string(),
            Self::Gp      => p.gp().unwrap_or(0).to_string(),
            // PP
            Self::PpPtsPace => format!("{:.1}", p.pp_points_per_82().unwrap_or(0.0)),
            Self::PpGPace   => format!("{:.1}", p.pp_goals_per_82().unwrap_or(0.0)),
            Self::PpPts     => p.pp_points.to_string(),
            Self::PpGoals   => p.pp_goals.to_string(),
            // SH
            Self::ShGPace   => format!("{:.1}", p.sh_goals_per_82().unwrap_or(0.0)),
            Self::ShGoals   => p.sh_goals.to_string(),
            // GWG
            Self::GwgPace   => format!("{:.1}", p.gwg_per_82().unwrap_or(0.0)),
            Self::Gwg       => p.gwg.to_string(),
            // Shots
            Self::ShotsPace => format!("{:.1}", p.shots_per_82().unwrap_or(0.0)),
            Self::Shots     => p.shots.to_string(),
            Self::ShPct     => p.shooting_pct.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| "—".to_owned()),
            // Two-way
            Self::PlusMinus => {
                let pm = p.plus_minus;
                if pm >= 0 { format!("+{pm}") } else { pm.to_string() }
            }
            Self::Toi       => p.toi_mmss().unwrap_or_else(|| "—".to_owned()),
            Self::FoPct     => p.faceoff_win_pct.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| "—".to_owned()),
            // Realtime
            Self::HitsPace  => format!("{:.1}", p.hits_per_82().unwrap_or(0.0)),
            Self::Hits       => p.hits.to_string(),
            Self::BlocksPace => format!("{:.1}", p.blocked_shots_per_82().unwrap_or(0.0)),
            Self::Blocks     => p.blocked_shots.to_string(),
            Self::Takeaways  => p.takeaways.to_string(),
            Self::Giveaways  => p.giveaways.to_string(),
            Self::Pim        => p.pim.to_string(),
            // MoneyPuck
            Self::Xg         => p.xg.map(|v| format!("{v:.2}")).unwrap_or_else(|| "—".to_owned()),
            Self::XgPer60    => p.xg_per_60.map(|v| format!("{v:.2}")).unwrap_or_else(|| "—".to_owned()),
            Self::CfPct      => p.cf_pct_5v5.map(|v| format!("{v:.1}%")).unwrap_or_else(|| "—".to_owned()),
            Self::FfPct      => p.ff_pct_5v5.map(|v| format!("{v:.1}%")).unwrap_or_else(|| "—".to_owned()),
            Self::XgfPct     => p.xgf_pct_5v5.map(|v| format!("{v:.1}%")).unwrap_or_else(|| "—".to_owned()),
            Self::Improvement => "—".to_owned(), // displayed by special-case handler
        }
    }

    fn header(self, rate: bool) -> &'static str {
        match self {
            Self::PtsPace   => if rate { "PPG" } else { "Pts/82" },
            Self::Ppg       => "PPG",
            Self::GPace     => if rate { "GPG" } else { "G/82" },
            Self::Gpg       => "GPG",
            Self::Pts       => "Pts",
            Self::Goals     => "Goals",
            Self::Assists   => "Assists",
            Self::Gp        => "GP",
            Self::PpPtsPace => "PP-Pts/82",
            Self::PpGPace   => "PP-G/82",
            Self::PpPts     => "PP-Pts",
            Self::PpGoals   => "PP-Goals",
            Self::ShGPace   => "SH-G/82",
            Self::ShGoals   => "SH-Goals",
            Self::GwgPace   => "GWG/82",
            Self::Gwg       => "GWG",
            Self::ShotsPace => "Shots/82",
            Self::Shots     => "Shots",
            Self::ShPct     => "SH%",
            Self::PlusMinus => "+/-",
            Self::Toi       => "TOI/g",
            Self::FoPct     => "FO%",
            // Realtime
            Self::HitsPace  => "Hits/82",
            Self::Hits       => "Hits",
            Self::BlocksPace => "Blk/82",
            Self::Blocks     => "Blocks",
            Self::Takeaways  => "TkA",
            Self::Giveaways  => "GvA",
            Self::Pim        => "PIM",
            // MoneyPuck
            Self::Xg         => "ixG",
            Self::XgPer60    => "ixG/60",
            Self::CfPct      => "CF%",
            Self::FfPct      => "FF%",
            Self::XgfPct     => "xGF%",
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
    pub toi_min: Option<f32>,      // minutes per game (converted to seconds internally)
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
    pub sort: String,
    pub top: usize,
    pub rate: bool,
    pub percentiles: bool,
    pub json: bool,
    pub csv: bool,
}

// ── icelines query leaders ────────────────────────────────────────────────────

pub async fn run_leaders(args: LeadersArgs) -> anyhow::Result<()> {
    let metric = SortMetric::parse(&args.sort)?;

    // Phase 8f: --season overrides default; --season + --seasons > 1 is ambiguous.
    if args.season.is_some() && args.seasons > 1 {
        anyhow::bail!(
            "--season and --seasons N > 1 are mutually exclusive.\n  \
             Use --season YYYYZZZZ for a single historical season,\n  \
             or --seasons N for an N-season aggregate of recent seasons."
        );
    }

    // Load player pool — single season (default or overridden) or N-season aggregate.
    let all_players: Vec<Player> = if args.seasons > 1 {
        aggregate::load_aggregate_players(args.seasons as usize)
    } else {
        crate::commands::players::load_all_players_for_season(args.season.as_deref())?
    };

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
    filter.age_min       = args.age_min;
    filter.age_max       = args.age_max;
    filter.nationalities = args.nationality.map(|n| vec![n.to_uppercase()]);
    filter.draft_years   = args.draft_year.map(|y| vec![y]);
    filter.draft_rounds  = args.round.map(|r| vec![r]);
    filter.draft_pick_max = args.draft_pick_max;
    filter.undrafted        = if args.undrafted { Some(true) } else { None };
    filter.rookie_only      = if args.rookie   { Some(true) } else { None };
    filter.handedness       = args.handedness;
    filter.ppg_min          = args.ppg_min;
    filter.gp_min           = args.gp_min;
    filter.toi_min_sec      = args.toi_min.map(|m| m * 60.0);
    filter.plus_minus_min   = args.plus_minus_min;
    filter.shots_pg_min     = args.shots_pg_min;
    filter.birth_provinces  = args.birth_province.map(|bp| {
        bp.split(',').map(|s| s.trim().to_uppercase()).collect()
    });

    let mut matched: Vec<&Player> = filter.apply(&all_players);

    // gp_max (not in PlayerFilter — inline here)
    if let Some(gp_max) = args.gp_max {
        matched.retain(|p| p.gp().unwrap_or(0) <= gp_max);
    }

    // Contract filters
    let wants_contract = args.ufa || args.rfa || args.elc || args.expiry_year.is_some();
    if args.ufa { matched.retain(|p| p.is_ufa()); }
    if args.rfa { matched.retain(|p| p.is_rfa()); }
    if args.elc {
        matched.retain(|p| {
            p.expiry_type
                .as_deref()
                .map(|t| t.eq_ignore_ascii_case("ELC"))
                .unwrap_or(false)
        });
    }
    if let Some(yr) = args.expiry_year {
        matched.retain(|p| p.contract_expiry_year == Some(yr));
    }
    // Hint when contract filter returns nothing — likely no contract data fetched
    if wants_contract && matched.is_empty() {
        eprintln!("  Hint: no contract data found. Run `icelines fetch contracts` to enable UFA/RFA/ELC filtering.");
    }
    // Nationality hint — bad ISO code returns silent empty
    if matched.is_empty() {
        if let Some(ref nats) = filter.nationalities {
            eprintln!("  Hint: no players found for nationality code(s) {:?}. Use ISO-3166 alpha-3 (e.g. CAN, USA, SWE, FIN, RUS, CZE, SVK, DEU).", nats);
        }
    }

    // Improvement sort requires the Y/Y delta map — handle before generic sort
    if matches!(metric, SortMetric::Improvement) {
        let imp_map = aggregate::load_improvement_map();
        matched.sort_by(|a, b| {
            let da = a.nhl_id.and_then(|id| imp_map.get(&id)).copied().unwrap_or(f64::NEG_INFINITY);
            let db = b.nhl_id.and_then(|id| imp_map.get(&id)).copied().unwrap_or(f64::NEG_INFINITY);
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });
        let total_matched = matched.len();
        let results: Vec<&Player> = matched.iter().copied().take(args.top).collect();

        if args.json {
            println!("{}", leaders_json(&results));
            return Ok(());
        }
        print_improvement_table(&results, &imp_map, args.top, total_matched, args.seasons);
        return Ok(());
    }

    // Warn if a realtime/MoneyPuck metric has no data (all zeros/defaults)
    let data_missing = match metric {
        SortMetric::HitsPace | SortMetric::Hits | SortMetric::BlocksPace | SortMetric::Blocks
        | SortMetric::Takeaways | SortMetric::Giveaways | SortMetric::Pim => {
            matched.iter().all(|p| p.hits == 0 && p.blocked_shots == 0 && p.takeaways == 0)
        }
        SortMetric::Xg | SortMetric::XgPer60 | SortMetric::CfPct
        | SortMetric::FfPct | SortMetric::XgfPct => {
            matched.iter().all(|p| p.xg.is_none() && p.cf_pct_5v5.is_none())
        }
        _ => false,
    };
    if data_missing {
        eprintln!("  Warning: no realtime/MoneyPuck data loaded for sort '{}'. Run `icelines fetch` to download it.", args.sort);
        eprintln!("  Results below are sorted by Pts/82 as a fallback.");
        matched.sort_by(|a, b| {
            let sa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
            let sb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        matched.sort_by(|a, b| {
            metric.sort_value(b)
                .partial_cmp(&metric.sort_value(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let total_matched = matched.len();
    let results: Vec<&Player> = matched.into_iter().take(args.top).collect();

    if args.json {
        println!("{}", leaders_json(&results));
        return Ok(());
    }
    if args.csv {
        leaders_csv(&results);
        return Ok(());
    }

    // Percentile: rank among all rankable position peers
    let percentiles: Vec<Option<u8>> = if args.percentiles {
        results
            .iter()
            .map(|p| position_percentile(&all_players, p, metric))
            .collect()
    } else {
        vec![None; results.len()]
    };

    leaders_table(&results, &percentiles, metric, args.rate, args.top, total_matched, args.seasons);
    Ok(())
}

fn leaders_table(
    players: &[&Player],
    percentiles: &[Option<u8>],
    metric: SortMetric,
    rate: bool,
    top: usize,
    total: usize,
    seasons: u8,
) {
    let col = if seasons > 1 {
        // Append season window to column header so user knows it's aggregated
        format!("{} ({}yr)", metric.header(rate), seasons)
    } else {
        metric.header(rate).to_owned()
    };
    let show_pct = percentiles.iter().any(|p| p.is_some());

    if show_pct {
        println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<10} {:<6}", "Rank", "Player", "Team", "Pos", "GP", col, "Pctl");
        println!("{}", "─".repeat(61));
        for (i, (p, pct)) in players.iter().zip(percentiles.iter()).enumerate() {
            let gp  = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
            let val = metric.display(p, rate);
            let pct_s = pct.map(|v| format!("{v}{}", ordinal(v))).unwrap_or_else(|| "—".to_owned());
            println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<10} {:<6}", i + 1, p.full_name, p.team.as_str(), p.position.abbreviation(), gp, val, pct_s);
        }
    } else {
        println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<10}", "Rank", "Player", "Team", "Pos", "GP", col);
        println!("{}", "─".repeat(55));
        for (i, p) in players.iter().enumerate() {
            let gp  = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
            let val = metric.display(p, rate);
            println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<10}", i + 1, p.full_name, p.team.as_str(), p.position.abbreviation(), gp, val);
        }
    }
    println!("\n{total} matched, showing {}.", players.len().min(top));
}

fn print_improvement_table(
    players: &[&Player],
    imp_map: &std::collections::HashMap<u32, f64>,
    top: usize,
    total: usize,
    seasons: u8,
) {
    let window = if seasons > 1 { format!("({seasons}-season window) ") } else { String::new() };
    println!("IMPROVEMENT LEADERS {}— Y/Y PPG delta (current vs prior season)", window);
    println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<8} {:<8} {:<8}", "Rank", "Player", "Team", "Pos", "GP", "Curr", "Prior", "Δ PPG");
    println!("{}", "─".repeat(69));
    for (i, p) in players.iter().enumerate() {
        let delta = p.nhl_id.and_then(|id| imp_map.get(&id)).copied().unwrap_or(0.0);
        let curr_ppg = p.pace_score.map(|s| format!("{:.3}", s.pace_82 / 82.0)).unwrap_or_else(|| "—".to_owned());
        let prior_ppg = format!("{:.3}", (p.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0)) - delta);
        let delta_s = if delta >= 0.0 { format!("+{:.3}", delta) } else { format!("{:.3}", delta) };
        let gp = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
        println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<8} {:<8} {:<8}",
            i + 1, p.full_name, p.team.as_str(), p.position.abbreviation(),
            gp, curr_ppg, prior_ppg, delta_s);
    }
    println!("\n{total} matched, showing {}.", players.len().min(top));
}

fn leaders_json(players: &[&Player]) -> String {
    let rows: Vec<serde_json::Value> = players
        .iter()
        .enumerate()
        .map(|(i, p)| {
            serde_json::json!({
                "rank": i + 1,
                "name": p.full_name,
                "team": p.team.as_str(),
                "pos": p.position.abbreviation(),
                "gp": p.gp(),
                "ppg": p.pace_score.map(|s| round2(s.pace_82 / 82.0)),
                "pts_per_82": p.pace_score.map(|s| round1(s.pace_82)),
                "goals_per_82": p.pace_score.map(|s| round1(s.goals_per_82)),
                "season_pts": p.season_points,
                "season_goals": p.season_goals,
                "season_assists": p.season_assists,
            })
        })
        .collect();
    serde_json::to_string_pretty(&rows).unwrap_or_default()
}

fn leaders_csv(players: &[&Player]) {
    println!("rank,name,team,pos,gp,ppg,pts_per_82,goals_per_82,pts,goals,assists");
    for (i, p) in players.iter().enumerate() {
        println!(
            "{},{},{},{},{},{:.3},{:.1},{:.1},{},{},{}",
            i + 1, p.full_name, p.team.as_str(), p.position.abbreviation(),
            p.gp().unwrap_or(0),
            p.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0),
            p.pace_score.map(|s| s.pace_82).unwrap_or(0.0),
            p.pace_score.map(|s| s.goals_per_82).unwrap_or(0.0),
            p.season_points, p.season_goals, p.season_assists,
        );
    }
}

fn position_percentile(all: &[Player], target: &Player, metric: SortMetric) -> Option<u8> {
    let peers: Vec<&Player> = all
        .iter()
        .filter(|p| p.position == target.position && p.is_rankable())
        .collect();
    if peers.is_empty() {
        return None;
    }
    let target_val = metric.sort_value(target);
    let n_better = peers.iter().filter(|p| metric.sort_value(p) > target_val).count();
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
) -> anyhow::Result<()> {
    let players = crate::commands::players::load_all_players_for_season(season.as_deref())?;
    let p = find_player(&players, &name)?;

    let age = age_str(p);
    let draft = draft_str(p);
    println!(
        "PLAYER PROFILE — {} ({} · {} · Age {} · {})",
        p.full_name,
        p.team.as_str(),
        p.position.abbreviation(),
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
            print_current_stats(p);
            if percentiles {
                print_percentile(&players, p);
            }
            print_career(p).await;
        }
        "situation" => {
            println!("  Situational breakdown (5v5/PP/PK) requires Phase 5C shift data.");
            println!("  Currently available: all-situations stats only.");
            println!();
            print_current_stats(p);
            if percentiles {
                print_percentile(&players, p);
            }
        }
        other => bail!("unknown breakdown '{other}' — valid: career, situation"),
    }
    Ok(())
}

fn print_current_stats(p: &Player) {
    let (ppg, proj) = pace_strings(p);
    let toi = p.toi_mmss().unwrap_or_else(|| "—".to_owned());
    let sh_pct = p.shooting_pct.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| "—".to_owned());
    let pm = if p.plus_minus >= 0 { format!("+{}", p.plus_minus) } else { p.plus_minus.to_string() };
    println!("CURRENT SEASON");
    println!(
        "  GP {:<4}  G {:<4}  A {:<4}  Pts {:<4}  PPG {}  Pts/82 {}",
        p.gp().unwrap_or(0), p.season_goals, p.season_assists, p.season_points, ppg, proj
    );
    println!(
        "  PP: {:<3}G / {:<3}Pts   SH: {}G   GWG: {}   Shots: {}   SH%: {}",
        p.pp_goals, p.pp_points, p.sh_goals, p.gwg, p.shots, sh_pct
    );
    println!(
        "  +/-: {:<5}  TOI/g: {:<6}{}",
        pm, toi,
        p.faceoff_win_pct.map(|v| format!("  FO%: {:.1}%", v * 100.0)).unwrap_or_default()
    );
    println!();

    // Contract info (only shown if expiry_type is known)
    if let Some(expiry_type) = &p.expiry_type {
        let expiry_year = p.contract_expiry_year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "?".to_owned());
        let salary_str = p.salary
            .map(|s| format!("${:.1}M", s as f64 / 1_000_000.0))
            .unwrap_or_else(|| "—".to_owned());
        println!("  Contract: {} expires {}  Salary: {}", expiry_type.to_uppercase(), expiry_year, salary_str);
        println!();
    }
}

fn print_percentile(all: &[Player], target: &Player) {
    let peers: Vec<&Player> = all
        .iter()
        .filter(|p| p.position == target.position && p.is_rankable())
        .collect();
    if peers.is_empty() {
        return;
    }
    let target_val = target.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
    let n_better = peers.iter()
        .filter(|p| p.pace_score.map(|s| s.pace_82).unwrap_or(0.0) > target_val)
        .count();
    let rank = n_better + 1;
    let pct = ((1.0 - n_better as f64 / peers.len() as f64) * 100.0) as u8;
    println!(
        "LEAGUE RANK  #{rank} of {} {}'s  ({pct}{} percentile by Pts/82)",
        peers.len(),
        target.position.abbreviation(),
        ordinal(pct)
    );
    println!();
}

async fn print_career(p: &Player) {
    let cfg = match Config::load() { Ok(c) => c, Err(_) => return };
    let store = SnapshotStore::new(cfg.snapshot_dir());
    match load_career(&p.full_name, 5, &store) {
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
                    line.team, line.gp, line.goals, line.assists,
                    line.ppg, line.pts_per_82()
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

pub struct GoaliesArgs {
    pub top:    usize,
    pub sort:   String,
    pub team:   Option<String>,
    pub min_gp: u32,
    pub season: Option<String>,
    pub json:   bool,
    pub csv:    bool,
}

pub async fn run_goalies(args: GoaliesArgs) -> anyhow::Result<()> {
    use icelines_core::model::Goalie;
    use icelines_fetch::goalie_repository::GoalieRepository;
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
    let repo = GoalieRepository::new(SnapshotStore::new(cfg.snapshot_dir()), season.clone());
    let mut goalies: Vec<Goalie> = repo.load_all()
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch goalies"))?;

    // Filters
    if let Some(team) = args.team.as_deref() {
        let abbrev = team.to_ascii_uppercase();
        goalies.retain(|g| g.team.as_str() == abbrev);
    }
    goalies.retain(|g| g.qualified(args.min_gp));

    // Sort
    use std::cmp::Ordering;
    let sort_key = args.sort.to_ascii_lowercase();
    goalies.sort_by(|a, b| {
        let sa = a.stats.as_ref();
        let sb = b.stats.as_ref();
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
            "wins" | "w" =>
                sb.map(|s| s.wins).unwrap_or(0).cmp(&sa.map(|s| s.wins).unwrap_or(0)),
            "gp" =>
                sb.map(|s| s.games_played).unwrap_or(0).cmp(&sa.map(|s| s.games_played).unwrap_or(0)),
            "saves" =>
                sb.map(|s| s.saves).unwrap_or(0).cmp(&sa.map(|s| s.saves).unwrap_or(0)),
            "so" | "shutouts" =>
                sb.map(|s| s.shutouts).unwrap_or(0).cmp(&sa.map(|s| s.shutouts).unwrap_or(0)),
            other => {
                eprintln!("  Hint: unknown sort '{other}' — falling back to sv-pct.");
                let av = sa.and_then(|s| s.save_pct).unwrap_or(0.0);
                let bv = sb.and_then(|s| s.save_pct).unwrap_or(0.0);
                bv.partial_cmp(&av).unwrap_or(Ordering::Equal)
            }
        }
    });
    goalies.truncate(args.top);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&goalies)
            .context("serializing goalies to JSON")?);
        return Ok(());
    }
    if args.csv {
        println!("rank,goalie,team,gp,wins,losses,ot_losses,sv_pct,gaa,so,saves");
        for (i, g) in goalies.iter().enumerate() {
            let s = g.stats.as_ref();
            println!("{},\"{}\",{},{},{},{},{},{:.4},{:.3},{},{}",
                i + 1,
                g.full_name,
                g.team.as_str(),
                s.map(|s| s.games_played).unwrap_or(0),
                s.map(|s| s.wins).unwrap_or(0),
                s.map(|s| s.losses).unwrap_or(0),
                s.and_then(|s| s.ot_losses).unwrap_or(0),
                s.and_then(|s| s.save_pct).unwrap_or(0.0),
                s.and_then(|s| s.goals_against_average).unwrap_or(0.0),
                s.map(|s| s.shutouts).unwrap_or(0),
                s.map(|s| s.saves).unwrap_or(0),
            );
        }
        return Ok(());
    }

    // Default: terminal-friendly table.
    println!("{:<4} {:<24} {:<5} {:<4} {:<10} {:<6} {:<6} {:<3} {:<6}",
        "Rank", "Goalie", "Team", "GP", "W-L-OT", "SV%", "GAA", "SO", "Saves");
    println!("{}", "─".repeat(80));
    for (i, g) in goalies.iter().enumerate() {
        let s = match g.stats.as_ref() { Some(s) => s, None => continue };
        let record = match s.ot_losses {
            Some(otl) => format!("{}-{}-{}", s.wins, s.losses, otl),
            None      => format!("{}-{}",    s.wins, s.losses),
        };
        let sv  = s.save_pct.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "—".to_owned());
        let gaa = s.goals_against_average.map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "—".to_owned());
        println!("{:<4} {:<24} {:<5} {:<4} {:<10} {:<6} {:<6} {:<3} {:<6}",
            i + 1,
            g.full_name.chars().take(24).collect::<String>(),
            g.team.as_str(),
            s.games_played,
            record, sv, gaa, s.shutouts, s.saves,
        );
    }
    println!("\n{} goalies (min {} GP, sorted by {}) for season {}.",
        goalies.len(), args.min_gp, sort_key, season);
    Ok(())
}

// ── icelines query compare ────────────────────────────────────────────────────

pub async fn run_compare(
    player1: String,
    player2: Option<String>,
    similar: Option<usize>,
    _by: String,
    season: Option<String>,
) -> anyhow::Result<()> {
    let players = crate::commands::players::load_all_players_for_season(season.as_deref())?;

    if let Some(n) = similar {
        run_similar(&players, &player1, n)
    } else if let Some(p2_name) = player2 {
        let p1 = find_player(&players, &player1)?;
        let p2 = find_player(&players, &p2_name)?;
        if p1.name_normalized == p2.name_normalized {
            eprintln!("  Note: both sides resolved to the same player ({}).", p1.full_name);
        }
        print_head_to_head(p1, p2);
        Ok(())
    } else {
        bail!("provide a second player name, or use --similar N for similarity search")
    }
}

fn print_head_to_head(p1: &Player, p2: &Player) {
    let (ppg1, proj1) = pace_strings(p1);
    let (ppg2, proj2) = pace_strings(p2);
    let col = 28usize;

    println!("{:<col$} {:<col$}", p1.full_name, p2.full_name);
    println!("{:<col$} {:<col$}", p1.team.as_str(), p2.team.as_str());
    println!("{}", "─".repeat(col * 2 + 2));

    let row = |label: &str, v1: &str, v2: &str| {
        println!("  {:<18} {:<col$} {:<col$}", label, v1, v2, col = col - 2);
    };

    row("Position", p1.position.abbreviation(), p2.position.abbreviation());
    row("Age", &age_str(p1), &age_str(p2));
    row("Draft", &draft_str(p1), &draft_str(p2));
    row(
        "GP",
        &p1.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned()),
        &p2.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned()),
    );
    row("PPG", &ppg1, &ppg2);
    row("Pts/82", &proj1, &proj2);
    row(
        "Goals/82",
        &p1.pace_score.map(|s| format!("{:.1}", s.goals_per_82)).unwrap_or_else(|| "—".to_owned()),
        &p2.pace_score.map(|s| format!("{:.1}", s.goals_per_82)).unwrap_or_else(|| "—".to_owned()),
    );
    row("PP Points", &p1.pp_points.to_string(), &p2.pp_points.to_string());
    row("PP Goals",  &p1.pp_goals.to_string(),  &p2.pp_goals.to_string());
    row("GWG",       &p1.gwg.to_string(),        &p2.gwg.to_string());
    row("Shots",     &p1.shots.to_string(),       &p2.shots.to_string());
    row(
        "SH%",
        &p1.shooting_pct.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| "—".to_owned()),
        &p2.shooting_pct.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "+/-",
        &if p1.plus_minus >= 0 { format!("+{}", p1.plus_minus) } else { p1.plus_minus.to_string() },
        &if p2.plus_minus >= 0 { format!("+{}", p2.plus_minus) } else { p2.plus_minus.to_string() },
    );
    row(
        "TOI/g",
        &p1.toi_mmss().unwrap_or_else(|| "—".to_owned()),
        &p2.toi_mmss().unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Contract",
        &p1.expiry_type.as_deref().map(|t| t.to_uppercase()).unwrap_or_else(|| "—".to_owned()),
        &p2.expiry_type.as_deref().map(|t| t.to_uppercase()).unwrap_or_else(|| "—".to_owned()),
    );
    row(
        "Expires",
        &p1.contract_expiry_year.map(|y| y.to_string()).unwrap_or_else(|| "—".to_owned()),
        &p2.contract_expiry_year.map(|y| y.to_string()).unwrap_or_else(|| "—".to_owned()),
    );
}

fn run_similar(players: &[Player], target_name: &str, n: usize) -> anyhow::Result<()> {
    let target = find_player(players, target_name)?;
    let target_age = player_age(target);

    // Cohort: same position, age ±2, must be rankable
    let cohort: Vec<&Player> = players
        .iter()
        .filter(|p| {
            p.position == target.position
                && p.is_rankable()
                && player_age(p).zip(target_age).map(|(a, ta)| (a as i32 - ta as i32).abs() <= 2).unwrap_or(false)
        })
        .collect();

    if cohort.len() < 3 {
        bail!(
            "cohort too small ({} players) for similarity search — need at least 3",
            cohort.len()
        );
    }

    // Three metrics: PPG (scoring rate), GPG (goal rate), draft pick score
    let ppgs: Vec<f64> = cohort.iter().map(|p| p.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0)).collect();
    let gpgs: Vec<f64> = cohort.iter().map(|p| p.pace_score.map(|s| s.goals_per_82 / 82.0).unwrap_or(0.0)).collect();
    // Draft pick: invert so higher = better prospect pedigree (1.0 = #1 overall, 0.0 = undrafted)
    let picks: Vec<f64> = cohort
        .iter()
        .map(|p| p.draft_overall.map(|pk| 1.0 - (pk as f64 - 1.0) / 399.0).unwrap_or(0.0))
        .collect();

    let (ppg_mu, ppg_sd)   = mean_std(&ppgs);
    let (gpg_mu, gpg_sd)   = mean_std(&gpgs);
    let (pick_mu, pick_sd) = mean_std(&picks);

    // Locate target in cohort
    let target_norm = &target.name_normalized;
    let ti = cohort.iter().position(|p| &p.name_normalized == target_norm).unwrap_or(0);
    let tz_ppg  = zscore(ppgs[ti],  ppg_mu,  ppg_sd);
    let tz_gpg  = zscore(gpgs[ti],  gpg_mu,  gpg_sd);
    let tz_pick = zscore(picks[ti], pick_mu, pick_sd);

    // Euclidean distance in Z-score space
    let mut scored: Vec<(&Player, f64)> = cohort
        .iter()
        .zip(ppgs.iter())
        .zip(gpgs.iter())
        .zip(picks.iter())
        .map(|(((p, &ppg), &gpg), &pick)| {
            let dz_ppg  = zscore(ppg,  ppg_mu,  ppg_sd)  - tz_ppg;
            let dz_gpg  = zscore(gpg,  gpg_mu,  gpg_sd)  - tz_gpg;
            let dz_pick = zscore(pick, pick_mu, pick_sd) - tz_pick;
            let dist = (dz_ppg * dz_ppg + dz_gpg * dz_gpg + dz_pick * dz_pick).sqrt();
            (*p, dist)
        })
        .collect();

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.retain(|(p, _)| &p.name_normalized != target_norm);

    let age_s  = target_age.map(|a| a.to_string()).unwrap_or_else(|| "?".to_owned());
    let draft_s = draft_str(target);
    println!(
        "SIMILAR PLAYERS TO {} ({} · {} · Age {} · {})",
        target.full_name, target.team.as_str(), target.position.abbreviation(), age_s, draft_s
    );
    println!("{}", "─".repeat(72));
    println!("{:<6} {:<24} {:<5} {:<4} {:<10} {:<8} Similarity", "Rank", "Player", "Team", "Age", "Draft", "PPG");
    println!("{}", "─".repeat(72));

    for (i, (p, dist)) in scored.iter().take(n).enumerate() {
        let age   = player_age(p).map(|a| a.to_string()).unwrap_or_else(|| "?".to_owned());
        let draft = draft_str(p);
        let ppg   = p.pace_score.map(|s| format!("{:.3}", s.pace_82 / 82.0)).unwrap_or_else(|| "—".to_owned());
        // Similarity: 100% when dist=0, decays — using 100 / (1 + dist)
        let sim = (100.0 / (1.0 + dist)) as u32;
        println!("{:<6} {:<24} {:<5} {:<4} {:<10} {:<8} {sim}%", i + 1, p.full_name, p.team.as_str(), age, draft, ppg);
    }
    println!(
        "\nCohort: {} {} players aged {}±2.",
        cohort.len(),
        target.position.abbreviation(),
        age_s
    );
    Ok(())
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn parse_positions(s: &str) -> Vec<Position> {
    match s.to_uppercase().as_str() {
        "F" => vec![Position::Center, Position::LeftWing, Position::RightWing],
        "G" => vec![Position::Goalie],
        _ => PositionResolver::parse(s).map(|(_, all)| all).unwrap_or_default(),
    }
}

fn find_player<'a>(players: &'a [Player], name: &str) -> anyhow::Result<&'a Player> {
    let norm = normalize_name(name);
    players
        .iter()
        .find(|p| p.name_normalized.contains(&norm))
        .with_context(|| format!("player '{name}' not found — try a partial name"))
}

fn player_age(p: &Player) -> Option<u8> {
    p.birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u32>().ok())
        .map(|birth_year| 2026u32.saturating_sub(birth_year) as u8)
}

fn age_str(p: &Player) -> String {
    player_age(p).map(|a| a.to_string()).unwrap_or_else(|| "—".to_owned())
}

fn draft_str(p: &Player) -> String {
    match (p.draft_year, p.draft_round, p.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r}#{o}"),
        (Some(y), _, _)             => format!("{y}"),
        _                           => "UD".to_owned(),
    }
}

fn pace_strings(p: &Player) -> (String, String) {
    match p.pace_score {
        Some(s) => (format!("{:.3}", s.pace_82 / 82.0), format!("{:.1}", s.pace_82)),
        None    => ("—".to_owned(), "—".to_owned()),
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

fn round1(v: f64) -> f64 { (v * 10.0).round() / 10.0 }
fn round2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }

/// Return the correct English ordinal suffix for a number (1→"st", 2→"nd", 3→"rd", rest→"th").
fn ordinal(n: u8) -> &'static str {
    // 11, 12, 13 always use "th" regardless of last digit
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

    #[test]
    fn l0_sort_metric_parse_valid() {
        assert!(matches!(SortMetric::parse("pts-pace"),    Ok(SortMetric::PtsPace)));
        assert!(matches!(SortMetric::parse("ppg"),         Ok(SortMetric::Ppg)));
        assert!(matches!(SortMetric::parse("g-pace"),      Ok(SortMetric::GPace)));
        assert!(matches!(SortMetric::parse("gpg"),         Ok(SortMetric::Gpg)));
        assert!(matches!(SortMetric::parse("pts"),         Ok(SortMetric::Pts)));
        assert!(matches!(SortMetric::parse("goals"),       Ok(SortMetric::Goals)));
        assert!(matches!(SortMetric::parse("assists"),     Ok(SortMetric::Assists)));
        assert!(matches!(SortMetric::parse("gp"),          Ok(SortMetric::Gp)));
        assert!(matches!(SortMetric::parse("pp-pts-pace"), Ok(SortMetric::PpPtsPace)));
        assert!(matches!(SortMetric::parse("pp-g-pace"),   Ok(SortMetric::PpGPace)));
        assert!(matches!(SortMetric::parse("pp-pts"),      Ok(SortMetric::PpPts)));
        assert!(matches!(SortMetric::parse("pp-g"),        Ok(SortMetric::PpGoals)));
        assert!(matches!(SortMetric::parse("sh-g-pace"),   Ok(SortMetric::ShGPace)));
        assert!(matches!(SortMetric::parse("sh-g"),        Ok(SortMetric::ShGoals)));
        assert!(matches!(SortMetric::parse("gwg-pace"),    Ok(SortMetric::GwgPace)));
        assert!(matches!(SortMetric::parse("gwg"),         Ok(SortMetric::Gwg)));
        assert!(matches!(SortMetric::parse("shots-pace"),  Ok(SortMetric::ShotsPace)));
        assert!(matches!(SortMetric::parse("shots"),       Ok(SortMetric::Shots)));
        assert!(matches!(SortMetric::parse("sh-pct"),      Ok(SortMetric::ShPct)));
        assert!(matches!(SortMetric::parse("plus-minus"),  Ok(SortMetric::PlusMinus)));
        assert!(matches!(SortMetric::parse("toi"),         Ok(SortMetric::Toi)));
        assert!(matches!(SortMetric::parse("fo-pct"),      Ok(SortMetric::FoPct)));
        // Realtime
        assert!(matches!(SortMetric::parse("hits-pace"),   Ok(SortMetric::HitsPace)));
        assert!(matches!(SortMetric::parse("hits"),        Ok(SortMetric::Hits)));
        assert!(matches!(SortMetric::parse("blocks-pace"), Ok(SortMetric::BlocksPace)));
        assert!(matches!(SortMetric::parse("blocks"),      Ok(SortMetric::Blocks)));
        assert!(matches!(SortMetric::parse("takeaways"),   Ok(SortMetric::Takeaways)));
        assert!(matches!(SortMetric::parse("giveaways"),   Ok(SortMetric::Giveaways)));
        assert!(matches!(SortMetric::parse("pim"),         Ok(SortMetric::Pim)));
        // MoneyPuck
        assert!(matches!(SortMetric::parse("xg"),          Ok(SortMetric::Xg)));
        assert!(matches!(SortMetric::parse("xg-per-60"),   Ok(SortMetric::XgPer60)));
        assert!(matches!(SortMetric::parse("cf-pct"),      Ok(SortMetric::CfPct)));
        assert!(matches!(SortMetric::parse("ff-pct"),      Ok(SortMetric::FfPct)));
        assert!(matches!(SortMetric::parse("xgf-pct"),     Ok(SortMetric::XgfPct)));
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
        assert!((sd - 2.0).abs() < 1e-6,  "std should be 2.0");
    }

    #[test]
    fn l0_mean_std_constant_returns_std_one() {
        // All same values → std=0, should clamp to 1.0 to avoid div-by-zero
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
        assert_eq!(season_label("short"),    "short");
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
        use icelines_core::model::{GpStatus, PaceScore, Player, TeamAbbr};
        use icelines_core::name::normalize_name;
        let p = Player {
            nhl_id: None, full_name: "Test".to_owned(),
            name_normalized: normalize_name("Test"),
            team: TeamAbbr("SEA".to_owned()), position: Position::Center,
            eligible_pos: vec![], gp_status: GpStatus::Eligible(50),
            season_goals: 20, season_assists: 30, season_points: 50,
            pace_score: Some(PaceScore { pace_82: 82.0, goals_per_82: 32.8, raw_points: 50, gp: 50 }),
            pp_goals: 0, pp_points: 0,
            sh_goals: 0, sh_points: 0,
            gwg: 0, ot_goals: 0,
            shots: 0, shooting_pct: None,
            plus_minus: 0,
            toi_per_game_sec: None,
            faceoff_win_pct: None,
            hits: 0, blocked_shots: 0, missed_shots: 0,
            giveaways: 0, takeaways: 0, pim: 0,
            xg: None, xg_per_60: None, cf_pct_5v5: None, ff_pct_5v5: None, xgf_pct_5v5: None,
            headshot_url: None, sweater_number: None,
            birth_date: Some("2001-01-01".to_owned()),
            birth_country: None, nationality_code: None,
            birth_city: None, birth_state_province: None,
            shoots_catches: None,
            height_in_inches: None, weight_lbs: None,
            draft_year: Some(2019), draft_round: Some(1), draft_overall: Some(3),
            rookie_season: None,
            contract_expiry_year: None,
            expiry_type: None,
            salary: None,
        };
        assert_eq!(draft_str(&p), "2019 R1#3");
    }
}
