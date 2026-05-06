use crate::cli::{FetchSeasonType, FetchSubcommand, QuerySeasonType, ReportKindArg};
use crate::config::Config;
use anyhow::{anyhow, Context};
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_catalog::{ReportKind, Tier, TIER1_REPORTS};
use icelines_fetch::{
    boxscore_client::{aggregate_profiles, BoxscoreClient},
    fetch_lock, moneypuck,
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
            let want_regular = matches!(
                season_type,
                FetchSeasonType::Regular | FetchSeasonType::Both
            );
            let want_playoff = matches!(
                season_type,
                FetchSeasonType::Playoff | FetchSeasonType::Both
            );

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
        FetchSubcommand::Career {
            dry_run,
            bundled_seasons,
        } => do_career(dry_run, bundled_seasons).await,
        FetchSubcommand::Goalies {
            season,
            refresh: _,
            dry_run,
            season_type,
        } => do_goalies(&season, dry_run, season_type).await,
        FetchSubcommand::Transactions { season, dry_run } => {
            do_transactions(&season, dry_run).await
        }
        FetchSubcommand::Boxscore {
            date,
            for_favorites,
            dry_run,
        } => do_boxscore(date, for_favorites, dry_run).await,
        FetchSubcommand::Sync { dry_run, force } => do_sync(dry_run, force).await,
        FetchSubcommand::Report {
            kind,
            season,
            season_type,
            no_lock,
            dry_run,
        } => do_report(kind, &season, season_type, no_lock, dry_run).await,
    }
}

/// Phase Lindsay L.1.6 — generic per-report fetcher.
///
/// Tier-1 only for L.1; Tier-2 lands in L.6 alongside the runtime
/// `extra_reports` cache. Decision tree:
///   1. validate `kind.is_known_working()` (TAPE-R3 follow-up #3)
///   2. validate `kind.tier() == Tier1` (Tier-2 deferred)
///   3. acquire fs lock (unless `--no-lock`) — TAPE-R3 cross-process guard
///   4. fetch via `NhlApiClient::fetch_report_paged` (DI-29 fence applies
///      at *load* time when the file is read back through
///      `load_report_with_fallback`; here we just write the rows verbatim)
///   5. write `{"data": [...], "total": N}` envelope to
///      `<snapshot_root>/<season>/<season_type>/<filename>`
async fn do_report(
    kind_arg: ReportKindArg,
    season: &str,
    season_type: QuerySeasonType,
    no_lock: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let kind: ReportKind = kind_arg.to_core();
    let st: SeasonType = season_type.to_core();

    // (1) Refuse known-broken endpoints up front. The current catalog
    // doesn't list any, but the gate exists so a future variant addition
    // can flip `is_known_working()` to `false` and have the CLI catch it
    // without touching this dispatch.
    if !kind.is_known_working() {
        return Err(anyhow!(
            "endpoint {:?} is documented-broken (returns 500 server-side); \
             refusing to dispatch — see data/api-probe-2026-05-02.txt",
            kind,
        ));
    }

    // (2) Phase Lindsay L.6 — Tier-1 uses the explicit TIER1_REPORTS
    // dispatch table for filename. Tier-2 derives filename from
    // `kind.url_path()` since Tier-2 has no typed deserializer (per
    // DI-27 / runtime-only `extra_reports` cache). The fetch flow is
    // otherwise identical: same fs lock, same fetch_report_paged,
    // same envelope shape, same atomic write. The deserialization
    // boundary is the only Tier-1/Tier-2 split — handled at load
    // time, not fetch time.
    let filename: String = report_filename(kind)?;

    let url_preview = format!(
        "https://api.nhle.com/stats/rest/en/{}?cayenneExp=seasonId={season} and gameTypeId={}",
        kind.url_path(),
        match st {
            SeasonType::Regular => 2,
            SeasonType::Playoff => 3,
        },
    );

    if dry_run {
        println!(
            "[dry-run] fetch report kind={:?} season={season} type={:?}",
            kind, st
        );
        println!("[dry-run] URL: {url_preview}");
        let cfg = Config::load().context("loading icelines config")?;
        let target = cfg
            .snapshot_dir()
            .join(season)
            .join(st.label())
            .join(&filename);
        println!("[dry-run] would write: {}", target.display());
        return Ok(());
    }

    // (3) fs lock guard — released on Drop. `_lock` keeps the guard live
    // for the duration of this function. `--no-lock` skips for users who
    // accept the rate-limit risk (TAPE-R3 follow-up: error message
    // references this flag).
    let cfg = Config::load().context("loading icelines config")?;
    let icelines_home = cfg
        .snapshot_dir()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _lock = if no_lock {
        None
    } else {
        Some(
            fetch_lock::acquire(&icelines_home, std::time::Duration::from_secs(120)).with_context(
                || {
                    format!(
                        "acquiring fetch lock at {}/.fetch.lock",
                        icelines_home.display(),
                    )
                },
            )?,
        )
    };

    // (4) HTTP fetch via the generic helper.
    let client = NhlApiClient::production();
    let rows = client
        .fetch_report_paged(kind, season, st)
        .await
        .with_context(|| format!("fetching {} {} {}", kind.url_path(), season, st.label()))?;

    // (5) Wrap in the API's `{"data": [...], "total": N}` envelope and
    // write to the per-window file. Atomic write via the existing
    // snapshot helpers.
    let envelope = serde_json::json!({
        "data": rows,
        "total": rows.len(),
    });
    let target = cfg
        .snapshot_dir()
        .join(season)
        .join(st.label())
        .join(&filename);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating dir {}", parent.display()))?;
    }
    icelines_fetch::snapshot::atomic_write_json(&target, &envelope)
        .with_context(|| format!("writing {}", target.display()))?;

    println!(
        "✓ fetched {} rows for {} {} → {}",
        rows.len(),
        kind.url_path(),
        st.label(),
        target.display(),
    );
    Ok(())
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
        let want_regular = matches!(
            season_type,
            FetchSeasonType::Regular | FetchSeasonType::Both
        );
        let want_playoff = matches!(
            season_type,
            FetchSeasonType::Playoff | FetchSeasonType::Both
        );
        if want_regular {
            println!(
                "Would fetch (regular): /stats/rest/en/skater/bios?seasonId={season}&gameTypeId=2"
            );
            println!("Would fetch (regular): /stats/rest/en/skater/summary?seasonId={season}&gameTypeId=2");
            println!("Would fetch (regular): /stats/rest/en/skater/realtime?seasonId={season}&gameTypeId=2");
        }
        if want_playoff {
            println!(
                "Would fetch (playoff): /stats/rest/en/skater/bios?seasonId={season}&gameTypeId=3"
            );
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
    if matches!(
        season_type,
        FetchSeasonType::Regular | FetchSeasonType::Both
    ) {
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
    if matches!(
        season_type,
        FetchSeasonType::Playoff | FetchSeasonType::Both
    ) {
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
            SeasonType::Regular => (cm.bios().len(), cm.stats().len()),
            SeasonType::Playoff => (
                cm.playoff_bios().map(|m| m.len()).unwrap_or(0),
                cm.playoff_stats().map(|m| m.len()).unwrap_or(0),
            ),
        };
        println!(
            "  Wrote {} bio + {} stats chunks ({label}).",
            n_bios, n_stats
        );
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
                &serde_json::to_vec(&stats)
                    .with_context(|| format!("serializing {label} stats"))?,
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
        let want_regular = matches!(
            season_type,
            FetchSeasonType::Regular | FetchSeasonType::Both
        );
        let want_playoff = matches!(
            season_type,
            FetchSeasonType::Playoff | FetchSeasonType::Both
        );
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

    if matches!(
        season_type,
        FetchSeasonType::Regular | FetchSeasonType::Both
    ) {
        Box::pin(run_goalies_pass(
            &store,
            &client,
            &snap,
            season,
            SeasonType::Regular,
        ))
        .await?;
    }
    if matches!(
        season_type,
        FetchSeasonType::Playoff | FetchSeasonType::Both
    ) {
        Box::pin(run_goalies_pass(
            &store,
            &client,
            &snap,
            season,
            SeasonType::Playoff,
        ))
        .await?;
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
    println!(
        "  {} goalies ({} qualified at 15+ GP)",
        goalies.len(),
        qualified
    );

    let filename = match ty {
        SeasonType::Regular => "goalie-stats.json",
        SeasonType::Playoff => "playoff-goalie-stats.json",
    };
    store
        .write_file(
            snap,
            &SnapshotTier::Stats,
            filename,
            &serde_json::to_vec(&goalies)
                .with_context(|| format!("serializing {label} goalie stats"))?,
        )
        .with_context(|| format!("writing {filename}"))?;
    Ok(())
}

async fn do_moneypuck(season: &str, dry_run: bool) -> anyhow::Result<()> {
    let url = moneypuck::csv_url(season).with_context(|| {
        format!("invalid season format '{season}' — expected 8 digits like 20252026")
    })?;

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
    let bios: Vec<SkaterBio> = store.read_tier(&SnapshotTier::Stats, "bios.json").context(
        "reading bios.json from active Stats snapshot — run `icelines fetch stats` first",
    )?;

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

    println!(
        "Fetching contract data for {n} players (this takes ~{}s)...",
        (n as f64 * 0.05).ceil() as u64
    );
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

/// Phase Calder.2 — `icelines fetch career`.
///
/// Walks the active stats snapshot's bios, calls
/// `/v1/player/{id}/landing` for each, parses `seasonTotals` into
/// `CareerHistory`, and writes the merged result to
/// `~/.icelines/career_history.json` (single global blob — career
/// history is per-player, not per-season).
async fn do_career(dry_run: bool, bundled_seasons: u8) -> anyhow::Result<()> {
    use icelines_fetch::career_landing::CareerHistoryStore;

    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let path = cfg.career_history_path();

    // Resolve the player set. Two paths:
    // - bundled_seasons > 0: walk the last N bundled seasons' bios
    //   (skaters + goalies), union the pids. Used to refresh the
    //   shipped bundle.
    // - bundled_seasons == 0: read the active stats snapshot's
    //   bios.json. Used for the user's local refresh after pulling
    //   new stats.
    let player_ids_result: anyhow::Result<Vec<u32>> = if bundled_seasons > 0 {
        Ok(union_pids_from_bundled(bundled_seasons))
    } else {
        store
            .read_tier::<Vec<SkaterBio>>(&SnapshotTier::Stats, "bios.json")
            .context("reading bios.json from active Stats snapshot")
            .map(|bios| bios.iter().map(|b| b.player_id).collect())
    };

    if dry_run {
        let n = player_ids_result.as_ref().map(|p| p.len()).unwrap_or(0);
        let est_secs = (n.max(700) as f64 * 0.06).ceil() as u64;
        println!("Would fetch career history for {n} players.");
        if bundled_seasons > 0 {
            println!("Source: union of last {bundled_seasons} bundled seasons (skaters + goalies)");
        }
        println!("Endpoint: /v1/player/{{id}}/landing.seasonTotals (50ms delay between calls)");
        println!("Estimated time: ~{est_secs}s");
        println!("Output: {}", path.display());
        if player_ids_result.is_err() {
            println!();
            println!(
                "Note: no active stats snapshot found — run `icelines fetch stats` first to \
                populate the player list. Estimated time above assumes a typical ~700-player roster."
            );
        }
        return Ok(());
    }

    let player_ids =
        player_ids_result.context("could not resolve player set for `fetch career`")?;
    let n = player_ids.len();

    println!(
        "Fetching career history for {n} players (~{}s)...",
        (n as f64 * 0.06).ceil() as u64
    );
    let client = NhlApiClient::production();
    let (histories, skipped) = client.fetch_all_career_histories(&player_ids).await;

    // Merge into the existing on-disk store rather than replace it —
    // a player who was on the active roster last fetch but not this
    // one (traded out / retired) shouldn't lose their cached history.
    let mut blob = CareerHistoryStore::load(&path).context("loading existing career_history")?;
    for h in &histories {
        blob.upsert(h.clone());
    }
    blob.stamp_now();
    blob.save(&path).context("saving career_history.json")?;

    println!(
        "Fetched career history for {} players ({} skipped). Blob: {}",
        histories.len(),
        skipped.len(),
        path.display()
    );
    if !skipped.is_empty() {
        eprintln!("Skipped pids:");
        for (pid, msg) in skipped.iter().take(20) {
            eprintln!("  {pid}: {msg}");
        }
        if skipped.len() > 20 {
            eprintln!("  ...and {} more", skipped.len() - 20);
        }
    }
    Ok(())
}

/// Phase Calder.2 — collect the union of skater + goalie pids across
/// the most recent N bundled seasons. Used by `fetch career
/// --bundled-seasons N` to refresh the shipped career-history blob.
fn union_pids_from_bundled(n: u8) -> Vec<u32> {
    use icelines_fetch::bundled::{get_goalie_stats, BUNDLED_SEASONS};
    use icelines_fetch::get_bundled_bios;
    let take = (n as usize).min(BUNDLED_SEASONS.len());
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut out: Vec<u32> = Vec::new();
    for season in &BUNDLED_SEASONS[..take] {
        if let Some(bios) = get_bundled_bios(season) {
            for b in bios {
                if seen.insert(b.player_id) {
                    out.push(b.player_id);
                }
            }
        }
        if let Some(goalies) = get_goalie_stats(season) {
            for g in goalies {
                if seen.insert(g.player_id) {
                    out.push(g.player_id);
                }
            }
        }
    }
    out
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
    let other_rate = if rows.is_empty() {
        0.0
    } else {
        other_count as f64 / rows.len() as f64
    };

    println!("  classified: {} rows", rows.len());
    let mut kinds: Vec<_> = counts.into_iter().collect();
    kinds.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (label, n) in &kinds {
        println!("    {label}: {n}");
    }
    if other_rate > 0.05 {
        eprintln!(
            "  WARN: other_rate is {:.1}% (>5% threshold) — \
                   ESPN prose may have drifted; review the regex set",
            other_rate * 100.0
        );
    }

    let envelope = TransactionsEnvelope {
        season: season.to_owned(),
        source: "espn".to_owned(),
        fetched_at: outcome.fetched_at,
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

/// Phase Lindsay L.6 — derive the per-window filename for a report.
///
/// Tier-1 reads from the explicit `TIER1_REPORTS` dispatch table
/// (filename pinned alongside the typed deserializer). Tier-2 has no
/// typed deserializer (per DI-27 / runtime-only `extra_reports` cache),
/// so we derive the filename from `kind.url_path()` by replacing `/`
/// with `-` and appending `.json`. Examples:
///   skater/summaryshooting          → skater-summaryshooting.json
///   skater/scoringRates             → skater-scoringRates.json
///   goalie/startedVsRelieved        → goalie-startedVsRelieved.json
fn report_filename(kind: ReportKind) -> anyhow::Result<String> {
    match kind.tier() {
        Tier::Tier1 => Ok(TIER1_REPORTS
            .iter()
            .find(|r| r.kind == kind)
            .ok_or_else(|| anyhow!("BUG: TIER1_REPORTS missing entry for {:?}", kind))?
            .filename
            .to_owned()),
        Tier::Tier2 => Ok(format!("{}.json", kind.url_path().replace('/', "-"))),
    }
}

// ── Phase Foster.3 — boxscore fetcher + EventStream writer ───────────────────

async fn do_boxscore(
    date: Option<String>,
    for_favorites: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    use crate::commands::tonight::parse_iso_date;
    use crate::config::Config;
    use chrono::Utc;
    use icelines_core::entity::EntityRef;
    use icelines_core::event_stream as proto;
    use icelines_core::identity::GameId;
    use icelines_core::model::TeamAbbr;
    use icelines_fetch::nhl_api::NhlApiClient;

    let cfg = Config::load().context("load config")?;
    let _ = cfg; // currently unused beyond bootstrapping; future TTL gates will read it.

    let anchor_str = match date.as_deref() {
        Some(d) => parse_iso_date(d)?,
        None => Utc::now().date_naive().format("%Y-%m-%d").to_string(),
    };
    let anchor = chrono::NaiveDate::parse_from_str(&anchor_str, "%Y-%m-%d")
        .context("parse anchor date")?;

    // Step 1: schedule fetch (already covered by Foster.1).
    let client = NhlApiClient::production();
    let games = client
        .fetch_schedule_for_date(&anchor_str)
        .await
        .with_context(|| format!("fetching schedule for {anchor_str}"))?;
    let same_day: Vec<_> = games
        .into_iter()
        .filter(|g| g.date == anchor_str)
        .collect();
    if same_day.is_empty() {
        println!("No games scheduled on {anchor_str}.");
        return Ok(());
    }

    // Step 2: optional favorites filter — keep only games involving
    // a favorited team. Player-favorites mid-day-trade resolution is
    // a Foster.4 polish item (needs career_history lookup).
    let favorited_teams: std::collections::HashSet<String> = if for_favorites {
        let db = crate::db::GroupDb::open().context("open group db")?;
        let members = db
            .list_members_with_kind("Favorites")
            .unwrap_or_default();
        members
            .iter()
            .filter(|(_, k)| matches!(k, crate::db::MemberKind::Team))
            .map(|(key, _)| key.to_uppercase())
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    let to_fetch: Vec<_> = if for_favorites {
        same_day
            .iter()
            .filter(|g| {
                favorited_teams.contains(g.away_abbrev.to_uppercase().as_str())
                    || favorited_teams.contains(g.home_abbrev.to_uppercase().as_str())
            })
            .collect()
    } else {
        same_day.iter().collect()
    };

    println!(
        "Boxscore fetch — {anchor_str} · {} game(s){}",
        to_fetch.len(),
        if for_favorites {
            format!(" (favorites only — {} team(s) tracked)", favorited_teams.len())
        } else {
            String::new()
        }
    );

    if dry_run {
        for g in &to_fetch {
            println!("  · {} @ {}  game_id={}", g.away_abbrev, g.home_abbrev, g.game_id);
        }
        println!("(dry run — no boxscores fetched, no events written)");
        return Ok(());
    }

    // Step 3: insert a score event per game. Without --for-favorites
    // we still record every game on the date so the timeline view
    // works league-wide. The boxscore JSON itself is fetched +
    // persisted in a follow-up sub-step (F.3.x); for now the score
    // event carries the slate-level summary the dashboard needs.
    let event_stream = crate::event_stream::EventStream::open()
        .context("open events table")?;
    let mut wrote = 0usize;
    let mut updated = 0usize;
    for g in &to_fetch {
        let game = GameId(g.game_id);
        let away = TeamAbbr(g.away_abbrev.clone());
        let home = TeamAbbr(g.home_abbrev.clone());
        let result = match g.game_state.as_deref() {
            Some(s @ ("FINAL" | "OFF")) => s.to_owned(),
            Some(s @ ("LIVE" | "CRIT")) => s.to_owned(),
            Some(other) => other.to_owned(),
            None => "FUT".to_owned(),
        };
        let payload = proto::ScorePayloadV1::new(
            game,
            home,
            away,
            g.home_score.unwrap_or(0) as u32,
            g.away_score.unwrap_or(0) as u32,
            result,
        );
        let payload_json = serde_json::to_string(&payload).context("serialize payload")?;
        let event_id = proto::score_final_event_id(game);
        let inserted = event_stream
            .upsert(
                anchor,
                &EntityRef::Game(game),
                "score",
                &event_id,
                &payload_json,
                proto::SCORE_PAYLOAD_VERSION,
            )
            .with_context(|| format!("upsert score event for game {}", g.game_id))?;
        if inserted {
            wrote += 1;
        } else {
            updated += 1;
        }
    }
    println!(
        "Done — {wrote} new event(s), {updated} updated event(s) on {anchor_str}."
    );
    Ok(())
}

// ── Phase Foster.4 — `icelines fetch sync` CLI surface ───────────────────────

async fn do_sync(dry_run: bool, force: bool) -> anyhow::Result<()> {
    use icelines_fetch::datastore::{DataStore, Fetcher, NhlApiFetcher};
    use icelines_fetch::sync_engine::{
        enumerate_stale_for_dry_run, force_refresh_filter, run_sync_blocking,
    };
    use std::sync::Arc;

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow!("cannot determine home directory"))?;
    let data_root = home.join(".icelines").join("data");
    let store = DataStore::open(&data_root)
        .context("open DataStore")?
        .with_fetcher(Arc::new(NhlApiFetcher::default()) as Arc<dyn Fetcher>);
    let store = Arc::new(store);

    if dry_run {
        let entries = if force {
            force_refresh_filter(&store)
        } else {
            enumerate_stale_for_dry_run(&store)
        };
        if entries.is_empty() {
            println!("Nothing stale.");
            return Ok(());
        }
        println!(
            "{} entry(ies) {}:",
            entries.len(),
            if force { "would be refreshed (--force)" } else { "would be refreshed" }
        );
        for (kind, key) in entries {
            println!("  · {kind:?} / {key:?}");
        }
        println!("(dry run — no fetches issued)");
        return Ok(());
    }

    let summary = run_sync_blocking(store).await;
    println!(
        "Sync complete — {} refreshed, {} failed in {:.1}s",
        summary.refreshed,
        summary.failed,
        summary.elapsed.as_secs_f32(),
    );
    if !summary.errors.is_empty() {
        println!("First few errors:");
        for e in summary.errors.iter().take(5) {
            println!("  ! {e}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tier-1 filename comes from the dispatch table verbatim.
    #[test]
    fn l0_lindsay_l6_report_filename_tier1_uses_dispatch_table() {
        let f = report_filename(ReportKind::SkaterSummary).unwrap();
        assert_eq!(f, "summary.json");
        let f = report_filename(ReportKind::SkaterTimeOnIce).unwrap();
        assert_eq!(f, "timeonice.json");
        let f = report_filename(ReportKind::GoalieSavesByStrength).unwrap();
        assert_eq!(f, "goalie-savesByStrength.json");
    }

    /// Tier-2 filename is derived from `kind.url_path()` with `/` → `-`.
    /// Pin a few representative variants so a future url_path rename
    /// surfaces the cache-key drift loudly.
    #[test]
    fn l0_lindsay_l6_report_filename_tier2_derived_from_url_path() {
        let f = report_filename(ReportKind::SkaterSummaryShooting).unwrap();
        assert_eq!(f, "skater-summaryshooting.json");
        let f = report_filename(ReportKind::SkaterPuckPossessions).unwrap();
        assert_eq!(f, "skater-puckPossessions.json");
        let f = report_filename(ReportKind::GoalieStartedVsRelieved).unwrap();
        assert_eq!(f, "goalie-startedVsRelieved.json");
    }

    /// Every ReportKind variant produces a non-empty filename ending
    /// in `.json` — total over the 23-endpoint catalog.
    #[test]
    fn l0_lindsay_l6_report_filename_total_over_all_kinds() {
        for kind in ReportKind::all() {
            // Skip known-broken endpoints (none today, but this gate
            // exists for future).
            if !kind.is_known_working() {
                continue;
            }
            let f = report_filename(*kind).unwrap_or_else(|e| panic!("{kind:?} failed: {e}"));
            assert!(
                f.ends_with(".json"),
                "{kind:?} filename `{f}` must end with `.json`"
            );
            assert!(
                !f.contains('/'),
                "{kind:?} filename `{f}` must have `/` replaced"
            );
        }
    }
}
