use crate::cli::FetchSubcommand;
use crate::config::Config;
use anyhow::Context;
use icelines_fetch::{
    boxscore_client::{aggregate_profiles, BoxscoreClient},
    moneypuck,
    nhl_api::NhlApiClient,
    schema::SkaterBio,
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
            chunked,
        } => do_stats(&season, refresh, dry_run, chunked).await,
        FetchSubcommand::All {
            season,
            refresh,
            dry_run,
            chunked,
        } => {
            do_rosters(&season, refresh, dry_run).await?;
            do_stats(&season, refresh, dry_run, chunked).await?;
            // Contracts are best-effort — don't fail the whole fetch if they fail
            if let Err(e) = do_contracts(&season, dry_run).await {
                eprintln!("Warning: contract fetch failed (non-fatal): {e}");
            }
            Ok(())
        }
        FetchSubcommand::Positions { season, dry_run } => do_positions(&season, dry_run).await,
        FetchSubcommand::Realtime { season, dry_run } => do_realtime(&season, dry_run).await,
        FetchSubcommand::MoneyPuck { season, dry_run } => do_moneypuck(&season, dry_run).await,
        FetchSubcommand::Contracts { season, dry_run } => do_contracts(&season, dry_run).await,
    }
}

const TEAMS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

async fn do_rosters(season: &str, refresh: bool, dry_run: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today = today_date();
    let snap = format!("{season}-{today}-rosters");

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
        let roster = match client.fetch_team_roster(team, season).await {
            Ok(r) => r,
            Err(e) => {
                // Skip teams that didn't exist in this season (e.g. UTA before 2024-25)
                println!("  {team}: skipped ({e})");
                continue;
            }
        };
        let count = roster.forwards.len() + roster.defensemen.len() + roster.goalies.len();
        let json = serde_json::to_vec(&roster).context("serializing roster")?;
        store
            .write_file(
                &snap,
                &SnapshotTier::Rosters,
                &format!("{team}.json"),
                &json,
            )
            .with_context(|| format!("writing {team} to snapshot"))?;
        println!("  {team}: {count} players");
    }

    store.seal(&snap).context("sealing roster snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

async fn do_stats(season: &str, refresh: bool, dry_run: bool, chunked: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today = today_date();
    let snap = format!("{season}-{today}-stats");

    if dry_run {
        println!("Would fetch: /stats/rest/en/skater/bios?seasonId={season}");
        println!("Would fetch: /stats/rest/en/skater/summary?seasonId={season}");
        println!("Would fetch: /stats/rest/en/skater/realtime?seasonId={season}");
        if chunked {
            println!("Would write per-player chunks (--chunked) instead of single JSON files.");
        }
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
    let bios = client
        .fetch_all_bios(season)
        .await
        .context("fetching bios")?;
    println!("  {} players", bios.len());

    println!("Fetching stats...");
    let stats = client
        .fetch_all_stats(season)
        .await
        .context("fetching stats")?;
    println!("  {} players", stats.len());

    if chunked {
        // Phase 8h: per-player content-addressed chunks. Daily snapshots
        // share unchanged player records — typical reuse is 95%+.
        let cm = store
            .write_chunked_stats(&snap, &bios, &stats)
            .context("writing chunked bios+stats")?;
        println!(
            "  Wrote {} bio + {} stats chunks (chunked layout — Phase 8h).",
            cm.bios.len(),
            cm.stats.len(),
        );
    } else {
        // Legacy file-per-tier layout. Larger but simpler to inspect with `cat`.
        store
            .write_file(
                &snap,
                &SnapshotTier::Stats,
                "bios.json",
                &serde_json::to_vec(&bios).context("serializing bios")?,
            )
            .context("writing bios")?;
        store
            .write_file(
                &snap,
                &SnapshotTier::Stats,
                "stats.json",
                &serde_json::to_vec(&stats).context("serializing stats")?,
            )
            .context("writing stats")?;
    }

    println!("Fetching realtime stats...");
    let realtime = client
        .fetch_all_realtime(season)
        .await
        .context("fetching realtime stats")?;
    println!("  {} players", realtime.len());
    // Write realtime.json under SnapshotTier::Realtime so the repository can find it
    // via read_tier(&SnapshotTier::Realtime, "realtime.json") from the stats snapshot.
    store
        .write_file(
            &snap,
            &SnapshotTier::Realtime,
            "realtime.json",
            &serde_json::to_vec(&realtime).context("serializing realtime")?,
        )
        .context("writing realtime")?;

    store.seal(&snap).context("sealing stats snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

async fn do_realtime(season: &str, dry_run: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today = today_date();
    let snap = format!("{season}-{today}-realtime");

    if dry_run {
        println!("Would fetch: /stats/rest/en/skater/realtime?seasonId={season}");
        return Ok(());
    }

    store
        .create(&snap, season, SnapshotTier::Realtime, None, &today)
        .context("creating realtime snapshot")?;

    println!("Fetching realtime stats...");
    let realtime = client
        .fetch_all_realtime(season)
        .await
        .context("fetching realtime stats")?;
    println!("  {} players", realtime.len());
    store
        .write_file(
            &snap,
            &SnapshotTier::Realtime,
            "realtime.json",
            &serde_json::to_vec(&realtime).context("serializing realtime")?,
        )
        .context("writing realtime")?;

    store.seal(&snap).context("sealing realtime snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

async fn do_moneypuck(season: &str, dry_run: bool) -> anyhow::Result<()> {
    let url = moneypuck::csv_url(season)
        .with_context(|| format!("invalid season format '{season}' — expected 8 digits like 20252026"))?;

    if dry_run {
        println!("Would download MoneyPuck CSV: {url}");
        return Ok(());
    }

    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today = today_date();
    let snap = format!("{season}-{today}-moneypuck");

    println!("Downloading MoneyPuck CSV: {url}");
    let csv_text = client
        .fetch_text(&url)
        .await
        .context("downloading MoneyPuck CSV")?;

    let stats_map = moneypuck::parse_csv(&csv_text);
    println!("  {} players parsed", stats_map.len());

    // Convert to Vec for JSON serialization
    let stats_vec: Vec<_> = stats_map.into_values().collect();

    store
        .create(&snap, season, SnapshotTier::MoneyPuck, None, &today)
        .context("creating moneypuck snapshot")?;

    store
        .write_file(
            &snap,
            &SnapshotTier::MoneyPuck,
            "moneypuck.json",
            &serde_json::to_vec(&stats_vec).context("serializing moneypuck stats")?,
        )
        .context("writing moneypuck.json")?;

    store.seal(&snap).context("sealing moneypuck snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

async fn do_contracts(season: &str, dry_run: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());

    // Load player IDs from the active Stats snapshot bios.json (same pattern as do_positions).
    let bios: Vec<SkaterBio> = store
        .read_tier(&SnapshotTier::Stats, "bios.json")
        .context("reading bios.json from active Stats snapshot — run `icelines fetch stats` first")?;

    let player_ids: Vec<u32> = bios.iter().map(|b| b.player_id).collect();
    let n = player_ids.len();

    if dry_run {
        let est_secs = (n as f64 * 0.05).ceil() as u64;
        println!("Would fetch contract/landing data for {n} players.");
        println!("Endpoint: /v1/player/{{id}}/landing (one per player, 50ms delay)");
        println!("Estimated time: ~{est_secs}s");
        println!();
        println!("Note: As of 2026-04-26, the NHL landing API does not expose contract");
        println!("fields (expiry_year, expiry_type, salary). The contracts snapshot will");
        println!("be created with player_id populated and all contract fields as null.");
        println!("This is forward-compatible — fields will populate when the API exposes them.");
        return Ok(());
    }

    let today = today_date();
    let snap = format!("{season}-{today}-contracts");

    store
        .create(&snap, season, SnapshotTier::Contracts, None, &today)
        .context("creating contracts snapshot")?;

    println!("Fetching contract data for {n} players (this takes ~{}s)...", (n as f64 * 0.05).ceil() as u64);
    let client = NhlApiClient::production();
    let contracts = client.fetch_all_contracts(&player_ids).await;
    let found = contracts.len();

    let json = serde_json::to_vec(&contracts).context("serializing contracts")?;
    store
        .write_file(&snap, &SnapshotTier::Contracts, "contracts.json", &json)
        .context("writing contracts.json")?;

    store.seal(&snap).context("sealing contracts snapshot")?;
    println!("Fetched contracts for {n} players ({found} found)");
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

async fn do_positions(season: &str, dry_run: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let today = today_date();
    let snap = format!("{season}-{today}-positions");

    // Load player IDs from the active Stats snapshot (bios.json).
    let bios: Vec<SkaterBio> = store
        .read_tier(&SnapshotTier::Stats, "bios.json")
        .context("reading bios.json from active Stats snapshot")?;

    let player_ids: Vec<u32> = bios.iter().map(|b| b.player_id).collect();

    if dry_run {
        println!("Would fetch game logs for {} players", player_ids.len());
        return Ok(());
    }

    store
        .create(&snap, season, SnapshotTier::Positions, None, &today)
        .context("creating positions snapshot")?;

    let client = BoxscoreClient::production();
    let profiles = aggregate_profiles(&player_ids, season, &client).await;

    for profile in &profiles {
        let json = serde_json::to_vec(profile).context("serializing PositionProfile")?;
        store
            .write_file(
                &snap,
                &SnapshotTier::Positions,
                &format!("{}.json", profile.player_id),
                &json,
            )
            .with_context(|| format!("writing profile for player {}", profile.player_id))?;
    }

    store.seal(&snap).context("sealing positions snapshot")?;
    store
        .set_active(&snap)
        .context("setting positions snapshot active")?;
    println!("Fetched positions for {} players", profiles.len());
    Ok(())
}
