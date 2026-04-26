use crate::cli::FetchSubcommand;
use crate::config::Config;
use anyhow::Context;
use icelines_fetch::{
    cache::{ttl, Cache},
    nhl_api::NhlApiClient,
    schema::{RosterResponse, SkaterBio, SkaterStats},
};

pub async fn run(args: FetchSubcommand) -> anyhow::Result<()> {
    match args {
        FetchSubcommand::Rosters {
            season,
            refresh,
            dry_run,
        } => do_rosters(&season, refresh, dry_run).await,
        FetchSubcommand::Stats {
            season,
            refresh,
            dry_run,
        } => do_stats(&season, refresh, dry_run).await,
        FetchSubcommand::All {
            season,
            refresh,
            dry_run,
        } => {
            do_rosters(&season, refresh, dry_run).await?;
            do_stats(&season, refresh, dry_run).await
        }
        FetchSubcommand::Positions => {
            println!(
                "icelines fetch positions: not yet implemented (Phase 2 — requires shift data)"
            );
            Ok(())
        }
    }
}

async fn do_rosters(season: &str, refresh: bool, dry_run: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let cache = Cache::new(&cfg.cache_dir);
    let client = NhlApiClient::production();

    const TEAMS: &[&str] = &[
        "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
        "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
        "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
    ];

    println!("Fetching rosters for season {season}...");
    for team in TEAMS {
        let key = format!("rosters/{season}/{team}.json");
        if !refresh && cache.get::<RosterResponse>(&key, ttl::ROSTER).is_some() {
            println!("  {team}: cached");
            continue;
        }
        if dry_run {
            println!("  {team}: would fetch /v1/roster/{team}/{season}");
            continue;
        }
        let roster = client
            .fetch_team_roster(team, season)
            .await
            .with_context(|| format!("fetching roster for {team}"))?;
        let count = roster.forwards.len() + roster.defensemen.len() + roster.goalies.len();
        cache
            .put(&key, &roster)
            .with_context(|| format!("caching roster for {team}"))?;
        println!("  {team}: {count} players");
    }
    if !dry_run {
        println!("Rosters saved to cache.");
    }
    Ok(())
}

async fn do_stats(season: &str, refresh: bool, dry_run: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let cache = Cache::new(&cfg.cache_dir);
    let client = NhlApiClient::production();

    let bios_key = format!("stats/{season}/bios.json");
    let stats_key = format!("stats/{season}/stats.json");

    if !refresh
        && cache.get::<Vec<SkaterBio>>(&bios_key, ttl::STATS).is_some()
        && cache
            .get::<Vec<SkaterStats>>(&stats_key, ttl::STATS)
            .is_some()
    {
        println!("Stats cached (use --refresh to re-fetch).");
        return Ok(());
    }

    if dry_run {
        println!("Would fetch: /stats/rest/en/skater/bios?seasonId={season}");
        println!("Would fetch: /stats/rest/en/skater/summary?seasonId={season}");
        return Ok(());
    }

    println!("Fetching bios...");
    let bios = client
        .fetch_all_bios(season)
        .await
        .context("fetching bios")?;
    println!("  {} players", bios.len());
    cache.put(&bios_key, &bios).context("caching bios")?;

    println!("Fetching stats...");
    let stats = client
        .fetch_all_stats(season)
        .await
        .context("fetching stats")?;
    println!("  {} players", stats.len());
    cache.put(&stats_key, &stats).context("caching stats")?;

    println!("Stats saved to cache.");
    Ok(())
}
