use anyhow::Context;
use icelines_core::{
    filter::PlayerFilter, model::Player, position::PositionResolver, scoring::sort_by_pace,
    TeamAbbr,
};
use icelines_fetch::{
    player_builder::{build_players, index_bios, index_stats},
    schema::{RosterResponse, SkaterBio, SkaterStats},
    snapshot::{SnapshotStore, SnapshotTier},
};
use crate::config::Config;

const ALL_TEAMS: &[&str] = &[
    "ANA","BOS","BUF","CAR","CBJ","CGY","CHI","COL","DAL","DET","EDM","FLA","LAK",
    "MIN","MTL","NJD","NSH","NYI","NYR","OTT","PHI","PIT","SEA","SJS","STL","TBL",
    "TOR","UTA","VAN","VGK","WPG","WSH",
];

/// Load all skaters from the active snapshot chain.
pub fn load_all_players() -> anyhow::Result<Vec<Player>> {
    let cfg   = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let season_u32: u32 = cfg.season_str().parse().unwrap_or(20252026);

    let bios: Vec<SkaterBio>   = store.read_tier(&SnapshotTier::Stats, "bios.json")
        .context("no stats snapshot — run `icelines fetch stats` first")?;
    let stats: Vec<SkaterStats> = store.read_tier(&SnapshotTier::Stats, "stats.json")
        .unwrap_or_default();

    let bio_idx   = index_bios(&bios);
    let stats_idx = index_stats(&stats);
    let season    = icelines_core::model::Season(season_u32);

    let mut all: Vec<Player> = Vec::new();
    for team_str in ALL_TEAMS {
        let roster: Result<RosterResponse, _> =
            store.read_tier(&SnapshotTier::Rosters, &format!("{team_str}.json"));
        if let Ok(r) = roster {
            let team = TeamAbbr(team_str.to_string());
            let fwds = build_players(&r.forwards,   &bio_idx, &stats_idx, season, &team);
            let defs = build_players(&r.defensemen, &bio_idx, &stats_idx, season, &team);
            all.extend(fwds);
            all.extend(defs);
        }
    }
    sort_by_pace(&mut all);
    Ok(all)
}

pub struct PlayersArgs {
    pub pos: Option<String>, pub team: Option<String>,
    pub age_max: Option<u8>, pub age_min: Option<u8>,
    pub nationality: Option<String>, pub draft_year: Option<u16>,
    pub draft_round: Option<u8>, pub ppg_min: Option<f64>,
    pub gp_min: Option<u32>, pub top: usize,
    #[allow(dead_code)] pub json: bool,  // Phase 3: structured output
}

pub async fn run(args: PlayersArgs) -> anyhow::Result<()> {
    let players = load_all_players()?;

    let mut filter = PlayerFilter::new();
    if let Some(p) = args.pos {
        if let Ok((primary, _)) = PositionResolver::parse(&p) {
            filter.positions = Some(vec![primary]);
        }
    }
    if let Some(t) = args.team { filter.teams = Some(vec![t.to_uppercase()]); }
    filter.age_max       = args.age_max;
    filter.age_min       = args.age_min;
    filter.nationalities = args.nationality.map(|n| vec![n.to_uppercase()]);
    filter.draft_years   = args.draft_year.map(|y| vec![y]);
    filter.draft_rounds  = args.draft_round.map(|r| vec![r]);
    filter.ppg_min       = args.ppg_min;
    filter.gp_min        = args.gp_min;

    let matched = filter.apply(&players);
    let shown   = matched.iter().take(args.top);

    println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<7} {:<8}",
        "Rank", "Player", "Team", "Pos", "Age", "PPG", "Proj/82");
    println!("{}", "─".repeat(62usize));

    for (i, p) in shown.enumerate() {
        let age_str = p.birth_date.as_deref()
            .and_then(|d| d.get(..4))
            .and_then(|y| y.parse::<u16>().ok())
            .map(|y| (2026u16.saturating_sub(y)).to_string())
            .unwrap_or_else(|| "—".to_owned());

        let (ppg_str, proj_str) = match p.pace_score {
            Some(s) => (format!("{:.2}", s.pace_82 / 82.0), format!("{:.0}", s.pace_82)),
            None    => ("—".to_owned(), "—".to_owned()),
        };

        println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<7} {:<8}",
            i + 1, p.full_name, p.team.as_str(),
            p.position.abbreviation(), age_str, ppg_str, proj_str);
    }
    println!("\n{} players matched, showing {}.", matched.len(), matched.len().min(args.top));
    Ok(())
}
