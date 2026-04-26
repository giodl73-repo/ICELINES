use crate::cli::FetchSubcommand;
use crate::config::Config;
use anyhow::Context;
use icelines_fetch::{
    nhl_api::NhlApiClient,
    snapshot::{today_date, SnapshotStore, SnapshotTier},
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

const TEAMS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

async fn do_rosters(season: &str, refresh: bool, dry_run: bool) -> anyhow::Result<()> {
    let cfg    = Config::load()?;
    let store  = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today  = today_date();
    let snap   = format!("{season}-{today}-rosters");

    if dry_run {
        println!("Would create snapshot: {snap}");
        for team in TEAMS {
            println!("  {team}: would fetch /v1/roster/{team}/{season}");
        }
        return Ok(());
    }

    // Check if already sealed today
    if !refresh {
        if let Ok(entries) = store.list() {
            if entries.iter().any(|e| e.name == snap && e.sealed) {
                println!("Rosters already fetched today (use --refresh to re-fetch).");
                println!("  Snapshot: {snap}");
                return Ok(());
            }
        }
    }

    store
        .create(&snap, season, SnapshotTier::Rosters, None, &today)
        .context("creating roster snapshot")?;

    println!("Fetching rosters → snapshot '{snap}'");
    for team in TEAMS {
        let roster = client
            .fetch_team_roster(team, season)
            .await
            .with_context(|| format!("fetching roster for {team}"))?;
        let count = roster.forwards.len() + roster.defensemen.len() + roster.goalies.len();
        let json  = serde_json::to_vec(&roster).context("serializing roster")?;
        store
            .write_file(&snap, &SnapshotTier::Rosters, &format!("{team}.json"), &json)
            .with_context(|| format!("writing {team} to snapshot"))?;
        println!("  {team}: {count} players");
    }

    store.seal(&snap).context("sealing roster snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

async fn do_stats(season: &str, refresh: bool, dry_run: bool) -> anyhow::Result<()> {
    let cfg    = Config::load()?;
    let store  = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today  = today_date();
    let snap   = format!("{season}-{today}-stats");

    if dry_run {
        println!("Would fetch: /stats/rest/en/skater/bios?seasonId={season}");
        println!("Would fetch: /stats/rest/en/skater/summary?seasonId={season}");
        return Ok(());
    }

    // Check if already sealed today
    if !refresh {
        if let Ok(entries) = store.list() {
            if entries.iter().any(|e| e.name == snap && e.sealed) {
                println!("Stats already fetched today (use --refresh to re-fetch).");
                return Ok(());
            }
        }
    }

    // Parent = active rosters snapshot
    let parent = store
        .find_snapshot_for_tier(&SnapshotTier::Rosters)
        .ok()
        .map(|n| n.to_owned());

    store
        .create(&snap, season, SnapshotTier::Stats, parent, &today)
        .context("creating stats snapshot")?;

    println!("Fetching bios...");
    let bios  = client.fetch_all_bios(season).await.context("fetching bios")?;
    println!("  {} players", bios.len());
    store.write_file(&snap, &SnapshotTier::Stats, "bios.json",
        &serde_json::to_vec(&bios).context("serializing bios")?
    ).context("writing bios")?;

    println!("Fetching stats...");
    let stats = client.fetch_all_stats(season).await.context("fetching stats")?;
    println!("  {} players", stats.len());
    store.write_file(&snap, &SnapshotTier::Stats, "stats.json",
        &serde_json::to_vec(&stats).context("serializing stats")?
    ).context("writing stats")?;

    store.seal(&snap).context("sealing stats snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}
