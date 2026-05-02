use crate::cli::{FetchSeasonType, FetchSubcommand};
use crate::config::Config;
use anyhow::Context;
use icelines_core::season_stats::SeasonType;
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
            season_type,
        } => do_stats(&season, refresh, dry_run, chunked, season_type).await,
        FetchSubcommand::All {
            season,
            refresh,
            dry_run,
            chunked,
            season_type,
        } => {
            // For `--type playoff` we skip rosters/contracts/transactions
            // — they're regular-season-keyed concepts. For `--type both`
            // we run the full regular pipe first, then the playoff trio.
            let want_regular = matches!(season_type, FetchSeasonType::Regular | FetchSeasonType::Both);
            let want_playoff = matches!(season_type, FetchSeasonType::Playoff | FetchSeasonType::Both);

            if want_regular {
                do_rosters(&season, refresh, dry_run).await?;
            }
            do_stats(&season, refresh, dry_run, chunked, season_type).await?;
            // Goalie stats are best-effort — non-skater data shouldn't block
            // a partial fetch if the goalie endpoint goes down.
            if let Err(e) = do_goalies(&season, dry_run, season_type).await {
                eprintln!("Warning: goalie fetch failed (non-fatal): {e}");
            }
            if want_regular {
                // Contracts and transactions are type-agnostic; they only
                // run on the regular pass.
                if let Err(e) = do_contracts(&season, dry_run).await {
                    eprintln!("Warning: contract fetch failed (non-fatal): {e}");
                }
                if let Err(e) = do_transactions(&season, dry_run).await {
                    eprintln!("Warning: transactions fetch failed (non-fatal): {e}");
                }
            }
            let _ = want_playoff; // bound above; reserved for future per-type tuning
            Ok(())
        }
        FetchSubcommand::Positions { season, dry_run } => do_positions(&season, dry_run).await,
        FetchSubcommand::Realtime { season, dry_run } => do_realtime(&season, dry_run).await,
        FetchSubcommand::MoneyPuck { season, dry_run } => do_moneypuck(&season, dry_run).await,
        FetchSubcommand::Contracts { season, dry_run } => do_contracts(&season, dry_run).await,
        FetchSubcommand::Goalies {
            season,
            refresh: _,
            dry_run,
            season_type,
        } => do_goalies(&season, dry_run, season_type).await,
        FetchSubcommand::Transactions { season, dry_run } => do_transactions(&season, dry_run).await,
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

async fn do_stats(
    season: &str,
    refresh: bool,
    dry_run: bool,
    chunked: bool,
    season_type: FetchSeasonType,
) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today = today_date();
    let snap = format!("{season}-{today}-stats");

    if dry_run {
        let want_regular = matches!(season_type, FetchSeasonType::Regular | FetchSeasonType::Both);
        let want_playoff = matches!(season_type, FetchSeasonType::Playoff | FetchSeasonType::Both);
        if want_regular {
            println!("Would fetch (regular): /stats/rest/en/skater/bios?seasonId={season}&gameTypeId=2");
            println!("Would fetch (regular): /stats/rest/en/skater/summary?seasonId={season}&gameTypeId=2");
            println!("Would fetch (regular): /stats/rest/en/skater/realtime?seasonId={season}&gameTypeId=2");
        }
        if want_playoff {
            println!("Would fetch (playoff): /stats/rest/en/skater/bios?seasonId={season}&gameTypeId=3");
            println!("Would fetch (playoff): /stats/rest/en/skater/summary?seasonId={season}&gameTypeId=3");
            println!("Would write to playoff-bios.json + playoff-stats.json (co-located in stats/ tier).");
        }
        if chunked {
            println!("Would write per-player chunks (--chunked) instead of single JSON files.");
        }
        return Ok(());
    }

    // Check if already sealed today (idempotent for the snapshot, not per-type;
    // the snapshot can hold both regular and playoff data after one --type=both
    // run, so re-running with --refresh re-fetches everything for that day).
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

    // Box::pin breaks the future-chain depth so debug builds don't blow
    // the 8MB Windows main-thread stack. The fetch + serialize locals
    // make this future fat enough to matter on debug builds.
    if matches!(season_type, FetchSeasonType::Regular | FetchSeasonType::Both) {
        Box::pin(run_stats_pass(
            &store,
            &client,
            &snap,
            season,
            SeasonType::Regular,
            chunked,
        ))
        .await?;
    }
    if matches!(season_type, FetchSeasonType::Playoff | FetchSeasonType::Both) {
        Box::pin(run_stats_pass(
            &store,
            &client,
            &snap,
            season,
            SeasonType::Playoff,
            chunked,
        ))
        .await?;
    }

    store.seal(&snap).context("sealing stats snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

/// One pass of bios + stats fetching for a single `season_type`.
///
/// Hart.6.5 — extracted from `do_stats` so a `--type both` run can
/// invoke the same logic twice without re-creating the snapshot or
/// fighting the daily-idempotence guard. Realtime is fetched only on
/// the regular pass (Hart.6 D6 — playoff realtime flows through the
/// live game feed).
async fn run_stats_pass(
    store: &SnapshotStore,
    client: &NhlApiClient,
    snap: &str,
    season: &str,
    ty: SeasonType,
    chunked: bool,
) -> anyhow::Result<()> {
    let label = match ty {
        SeasonType::Regular => "regular",
        SeasonType::Playoff => "playoff",
    };
    println!("Fetching {label} bios...");
    let bios = client
        .fetch_all_bios(season, ty)
        .await
        .with_context(|| format!("fetching {label} bios"))?;
    println!("  {} players", bios.len());

    println!("Fetching {label} stats...");
    let stats = client
        .fetch_all_stats(season, ty)
        .await
        .with_context(|| format!("fetching {label} stats"))?;
    println!("  {} players", stats.len());

    if chunked {
        let cm = store
            .write_chunked_stats(snap, ty, &bios, &stats)
            .with_context(|| format!("writing chunked {label} bios+stats"))?;
        let (n_bios, n_stats) = match ty {
            SeasonType::Regular => (cm.bios.len(), cm.stats.len()),
            SeasonType::Playoff => (
                cm.playoff_bios.as_ref().map(|m| m.len()).unwrap_or(0),
                cm.playoff_stats.as_ref().map(|m| m.len()).unwrap_or(0),
            ),
        };
        println!("  Wrote {} bio + {} stats chunks ({label}).", n_bios, n_stats);
    } else {
        // Hart.6.5 — playoff variants land under co-located filenames
        // in the same Stats tier dir per Hart.6 D3. Loader resolves
        // them via the Hart.6.2 `load_playoff_*_with_fallback` chain.
        let (bios_filename, stats_filename) = match ty {
            SeasonType::Regular => ("bios.json", "stats.json"),
            SeasonType::Playoff => ("playoff-bios.json", "playoff-stats.json"),
        };
        store
            .write_file(
                snap,
                &SnapshotTier::Stats,
                bios_filename,
                &serde_json::to_vec(&bios).with_context(|| format!("serializing {label} bios"))?,
            )
            .with_context(|| format!("writing {bios_filename}"))?;
        store
            .write_file(
                snap,
                &SnapshotTier::Stats,
                stats_filename,
                &serde_json::to_vec(&stats).with_context(|| format!("serializing {label} stats"))?,
            )
            .with_context(|| format!("writing {stats_filename}"))?;
    }

    // Realtime is regular-season only.
    if ty == SeasonType::Regular {
        println!("Fetching realtime stats...");
        let realtime = client
            .fetch_all_realtime(season)
            .await
            .context("fetching realtime stats")?;
        println!("  {} players", realtime.len());
        store
            .write_file(
                snap,
                &SnapshotTier::Realtime,
                "realtime.json",
                &serde_json::to_vec(&realtime).context("serializing realtime")?,
            )
            .context("writing realtime")?;
    }
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

/// Fetch goalie season stats and write `goalie-stats.json` into a new
/// snapshot. Phase G.2.
///
/// Mirrors `do_realtime` — single endpoint, paginated fetch, written
/// under `SnapshotTier::Stats` so the same fallback chain in
/// `bundled::load_goalies_with_fallback` picks it up first when
/// active.
async fn do_goalies(
    season: &str,
    dry_run: bool,
    season_type: FetchSeasonType,
) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let client = NhlApiClient::production();
    let today = today_date();
    let snap = format!("{season}-{today}-goalies");

    if dry_run {
        let want_regular = matches!(season_type, FetchSeasonType::Regular | FetchSeasonType::Both);
        let want_playoff = matches!(season_type, FetchSeasonType::Playoff | FetchSeasonType::Both);
        if want_regular {
            println!("Would fetch (regular): /stats/rest/en/goalie/summary?seasonId={season}&gameTypeId=2");
        }
        if want_playoff {
            println!("Would fetch (playoff): /stats/rest/en/goalie/summary?seasonId={season}&gameTypeId=3");
            println!("Would write playoff-goalie-stats.json (co-located in stats/ tier).");
        }
        return Ok(());
    }

    store
        .create(&snap, season, SnapshotTier::Stats, None, &today)
        .context("creating goalies snapshot")?;

    if matches!(season_type, FetchSeasonType::Regular | FetchSeasonType::Both) {
        Box::pin(run_goalies_pass(&store, &client, &snap, season, SeasonType::Regular)).await?;
    }
    if matches!(season_type, FetchSeasonType::Playoff | FetchSeasonType::Both) {
        Box::pin(run_goalies_pass(&store, &client, &snap, season, SeasonType::Playoff)).await?;
    }

    store.seal(&snap).context("sealing goalies snapshot")?;
    println!("Snapshot '{snap}' sealed and set as active.");
    Ok(())
}

/// One pass of goalie fetching for a single `season_type`.
async fn run_goalies_pass(
    store: &SnapshotStore,
    client: &NhlApiClient,
    snap: &str,
    season: &str,
    ty: SeasonType,
) -> anyhow::Result<()> {
    let label = match ty {
        SeasonType::Regular => "regular",
        SeasonType::Playoff => "playoff",
    };
    println!("Fetching {label} goalie stats...");
    let goalies = client
        .fetch_all_goalies(season, ty)
        .await
        .with_context(|| format!("fetching {label} goalie stats"))?;
    let qualified = goalies.iter().filter(|g| g.games_played >= 15).count();
    println!("  {} goalies ({} qualified at 15+ GP)", goalies.len(), qualified);

    let filename = match ty {
        SeasonType::Regular => "goalie-stats.json",
        SeasonType::Playoff => "playoff-goalie-stats.json",
    };
    store
        .write_file(
            snap,
            &SnapshotTier::Stats,
            filename,
            &serde_json::to_vec(&goalies).with_context(|| format!("serializing {label} goalie stats"))?,
        )
        .with_context(|| format!("writing {filename}"))?;
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

/// Fetch league-wide transactions from ESPN — Phase T.3.
///
/// Hits ESPN's site.api, classifies each row, sanitizes descriptions,
/// maps team abbrevs to canonical NHL form, writes
/// `transactions.json` into a new snapshot, and updates
/// `SnapshotMetaFlags::transactions_*` so callers can surface staleness.
///
/// Best-effort: failures log a WARN and set the stale flag, but don't
/// abort `fetch all` (the rest of the pipeline still completes).
async fn do_transactions(season: &str, dry_run: bool) -> anyhow::Result<()> {
    use icelines_core::transactions::CURRENT_CLASSIFIER_VERSION;
    use icelines_fetch::{
        bundled::TransactionsEnvelope,
        snapshot::SnapshotMetaFlags,
        transactions::{raw_to_transactions, EspnSource},
    };

    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let snapshots_root = cfg.snapshot_dir();
    let today = today_date();
    let snap = format!("{season}-{today}-transactions");

    if dry_run {
        println!("Would fetch: ESPN site.api /transactions for season {season}");
        return Ok(());
    }

    println!("Fetching transactions from ESPN...");
    let espn = EspnSource::production();
    let outcome = match espn.fetch_season(season).await {
        Ok(o) => o,
        Err(e) => {
            // Set the stale flag so the next `icelines transactions`
            // surfaces "snapshot is N days stale (last fetch failed)".
            let mut flags = SnapshotMetaFlags::load(&snapshots_root, season);
            flags.transactions_stale = true;
            flags.transactions_last_error = Some(e.to_string());
            flags.transactions_fetched_at = Some(today.clone());
            let _ = flags.save(&snapshots_root, season); // best-effort
            return Err(e.into());
        }
    };

    if !outcome.dropped_unknown_schema.is_empty() {
        eprintln!(
            "  WARN: ESPN response contained unknown fields ({}): {:?}",
            outcome.dropped_unknown_schema.len(),
            outcome.dropped_unknown_schema,
        );
    }

    let raw_count = outcome.rows.len();
    let (rows, warnings) = raw_to_transactions(&outcome.rows, season);
    for w in &warnings {
        eprintln!("  WARN: {w}");
    }

    // Per-kind counts for the observability log.
    let mut counts = std::collections::HashMap::<&'static str, usize>::new();
    for row in &rows {
        *counts.entry(row.kind.label()).or_default() += 1;
    }
    let other_count = counts.get("other").copied().unwrap_or(0);
    let other_rate = if rows.is_empty() { 0.0 } else { other_count as f64 / rows.len() as f64 };

    println!("  classified: {} rows", rows.len());
    let mut kinds: Vec<_> = counts.into_iter().collect();
    kinds.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (label, n) in &kinds {
        println!("    {label}: {n}");
    }
    if other_rate > 0.05 {
        eprintln!("  WARN: other_rate is {:.1}% (>5% threshold) — \
                   ESPN prose may have drifted; review the regex set",
                  other_rate * 100.0);
    }

    let envelope = TransactionsEnvelope {
        season:             season.to_owned(),
        source:             "espn".to_owned(),
        fetched_at:         outcome.fetched_at,
        classifier_version: CURRENT_CLASSIFIER_VERSION,
        rows,
    };

    store
        .create(&snap, season, SnapshotTier::Stats, None, &today)
        .context("creating transactions snapshot")?;
    store
        .write_file(
            &snap,
            &SnapshotTier::Stats,
            "transactions.json",
            &serde_json::to_vec_pretty(&envelope).context("serializing transactions")?,
        )
        .context("writing transactions.json")?;
    store.seal(&snap).context("sealing transactions snapshot")?;

    // Clear the stale flag on success.
    let mut flags = SnapshotMetaFlags::load(&snapshots_root, season);
    flags.transactions_stale = false;
    flags.transactions_last_error = None;
    flags.transactions_fetched_at = Some(today);
    let _ = flags.save(&snapshots_root, season); // best-effort

    println!("Snapshot '{snap}' sealed and set as active. Raw rows: {raw_count}.");
    Ok(())
}
