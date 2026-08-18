use crate::cli::{
    ContractSource, FetchSeasonType, FetchSubcommand, QuerySeasonType, ReportKindArg,
};
use crate::config::Config;
use anyhow::{anyhow, Context};
use chrono::Utc;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_catalog::{ReportKind, Tier, TIER1_REPORTS};
use icelines_fetch::{
    boxscore_client::{aggregate_profiles, BoxscoreClient},
    fetch_lock,
    fletch::roster_url,
    moneypuck,
    nhl_api::NhlApiClient,
    schema::SkaterBio,
    snapshot::{
        today_date, OfficialNhlRosterCapture, OfficialNhlRosterCaptureManifest, SnapshotStore,
        SnapshotTier, OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE, OFFICIAL_NHL_LIVE_ROSTER_SCHEMA,
        OFFICIAL_NHL_LIVE_ROSTER_SOURCE,
    },
};
use sha2::{Digest, Sha256};

pub async fn run(args: FetchSubcommand) -> anyhow::Result<()> {
    match args {
        FetchSubcommand::ProspectSources {
            catalog,
            store,
            out,
            captured_at,
            ahl_roster_snapshot,
            ahl_identity_reviews,
            ahl_review_registry_url,
            include_roster_player_landings,
            contract_control_ledger,
            camp_participation_ledger,
            identity_review_ledgers,
            dry_run,
        } => {
            do_prospect_sources(ProspectSourcesCommand {
                catalog,
                store,
                out,
                captured_at,
                ahl_roster_snapshot,
                ahl_identity_reviews,
                ahl_review_registry_url,
                include_roster_player_landings,
                contract_control_ledger,
                camp_participation_ledger,
                identity_review_ledgers,
                dry_run,
            })
            .await
        }
        FetchSubcommand::Rosters {
            season,
            refresh,
            dry_run,
        } => do_rosters(&season, refresh, dry_run).await,
        FetchSubcommand::FletchSources {
            season,
            season_type,
            out,
            gate,
        } => do_fletch_sources(&season, season_type, &out, gate).await,
        FetchSubcommand::FletchPartitions {
            season,
            season_type,
            out,
            gate,
        } => do_fletch_partitions(&season, season_type, &out, gate).await,
        FetchSubcommand::FletchQuivers {
            season,
            season_type,
            out,
            gate,
        } => do_fletch_quivers(&season, season_type, &out, gate).await,
        FetchSubcommand::FletchCacheIndex {
            season,
            season_type,
            manifest,
            out,
            gate,
        } => do_fletch_cache_index(&season, season_type, manifest.as_deref(), &out, gate).await,
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
                if let Err(e) =
                    do_contracts(&season, &season, ContractSource::Nhl, None, None, dry_run).await
                {
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
        FetchSubcommand::MoneyPuck {
            season,
            seasons,
            dry_run,
        } => do_moneypuck(&season, seasons, dry_run).await,
        FetchSubcommand::Contracts {
            season,
            valuation_season,
            source,
            input,
            cap_limit,
            dry_run,
        } => {
            do_contracts(
                &season,
                valuation_season.as_deref().unwrap_or(&season),
                source,
                input.as_deref(),
                cap_limit,
                dry_run,
            )
            .await
        }
        FetchSubcommand::Career {
            dry_run,
            bundled_seasons,
            prospect_context,
            camp_forecast,
            league_crosswalk,
            affiliate_workboard,
        } => {
            do_career(
                dry_run,
                bundled_seasons,
                prospect_context.as_deref(),
                camp_forecast.as_deref(),
                league_crosswalk.as_deref(),
                affiliate_workboard.as_deref(),
            )
            .await
        }
        FetchSubcommand::Goalies {
            season,
            refresh: _,
            dry_run,
            season_type,
        } => do_goalies(&season, dry_run, season_type).await,
        FetchSubcommand::Ahl {
            season,
            teams,
            out,
            refresh,
            dry_run,
        } => do_ahl(&season, &teams, out.as_deref(), refresh, dry_run).await,
        FetchSubcommand::AhlTransactions {
            season,
            out,
            refresh,
            dry_run,
        } => do_ahl_transactions(&season, out.as_deref(), refresh, dry_run).await,
        FetchSubcommand::Transactions { season, dry_run } => {
            do_transactions(&season, dry_run).await
        }
        FetchSubcommand::Boxscore {
            date,
            for_favorites,
            dry_run,
        } => do_boxscore(date, for_favorites, dry_run).await,
        FetchSubcommand::PlayByPlay {
            date,
            for_favorites,
            dry_run,
        } => do_play_by_play(date, for_favorites, dry_run).await,
        FetchSubcommand::GoalVisualizer {
            game_id,
            event_id,
            force,
            dry_run,
        } => do_goal_visualizer(game_id, event_id, force, dry_run).await,
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

struct ProspectSourcesCommand {
    catalog: std::path::PathBuf,
    store: Option<std::path::PathBuf>,
    out: Option<std::path::PathBuf>,
    captured_at: Option<String>,
    ahl_roster_snapshot: Option<std::path::PathBuf>,
    ahl_identity_reviews: Vec<std::path::PathBuf>,
    ahl_review_registry_url: Option<String>,
    include_roster_player_landings: bool,
    contract_control_ledger: Option<std::path::PathBuf>,
    camp_participation_ledger: Option<std::path::PathBuf>,
    identity_review_ledgers: Vec<std::path::PathBuf>,
    dry_run: bool,
}

async fn do_prospect_sources(command: ProspectSourcesCommand) -> anyhow::Result<()> {
    use icelines_core::source_facts::OrganizationId;
    use icelines_fetch::{
        nhl_teams_for_season, run_prospect_source_audit_with_artifacts,
        ProspectPopulationSourceFamily, ProspectSourceAuditArtifacts, ProspectSourceAuditInput,
        ProspectSourceCatalog, SourcePackageStore,
    };

    let ProspectSourcesCommand {
        catalog: catalog_path,
        store: store_root,
        out,
        captured_at,
        ahl_roster_snapshot,
        ahl_identity_reviews,
        ahl_review_registry_url,
        include_roster_player_landings,
        contract_control_ledger,
        camp_participation_ledger,
        identity_review_ledgers,
        dry_run,
    } = command;
    let bytes = std::fs::read(&catalog_path)
        .with_context(|| format!("read prospect source catalog {}", catalog_path.display()))?;
    let catalog: ProspectSourceCatalog = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse prospect source catalog {}", catalog_path.display()))?;
    let organizations = nhl_teams_for_season(&catalog.season.to_string())
        .into_iter()
        .map(OrganizationId::try_new)
        .collect::<Result<Vec<_>, _>>()?;
    let families = [
        ProspectPopulationSourceFamily::Draft,
        ProspectPopulationSourceFamily::CampPublication,
        ProspectPopulationSourceFamily::ContractPublication,
        ProspectPopulationSourceFamily::TransactionPublication,
        ProspectPopulationSourceFamily::CurrentNhlAssignment,
        ProspectPopulationSourceFamily::CurrentAhlAssignment,
        ProspectPopulationSourceFamily::NhlPlayerLanding,
    ];
    let requests = catalog.expand(&organizations, &families)?;
    let unique_urls = requests
        .iter()
        .map(|request| request.source_url.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let cataloged_objects = requests
        .iter()
        .map(|request| request.coverage_object_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    if dry_run {
        println!(
            "prospect source audit: season={} organizations={} requested_matrix={} cataloged_objects={} unique_urls={} uncataloged_objects={} ahl_snapshot={} ahl_reviews={} roster_landings={} contract_ledger={} camp_ledger={} identity_review_ledgers={}",
            catalog.season,
            organizations.len(),
            organizations.len() * families.len(),
            cataloged_objects,
            unique_urls,
            organizations.len() * families.len() - cataloged_objects,
            ahl_roster_snapshot.is_some(),
            ahl_identity_reviews.len(),
            include_roster_player_landings,
            contract_control_ledger.is_some(),
            camp_participation_ledger.is_some(),
            identity_review_ledgers.len(),
        );
        return Ok(());
    }
    let finalize_cutoffs_after_acquisition = captured_at.is_none();
    let captured_at = captured_at
        .as_deref()
        .map(chrono::DateTime::parse_from_rfc3339)
        .transpose()
        .context("--captured-at must be RFC 3339")?
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let store =
        SourcePackageStore::new(store_root.unwrap_or_else(SourcePackageStore::default_root));
    let ahl_roster_snapshot = ahl_roster_snapshot
        .as_deref()
        .map(std::fs::read)
        .transpose()
        .context("read --ahl-roster-snapshot")?;
    let ahl_identity_reviews = ahl_identity_reviews
        .iter()
        .map(|path| {
            std::fs::read(path)
                .with_context(|| format!("read --ahl-identity-review {}", path.display()))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let roster_player_landing_cache_root = include_roster_player_landings
        .then(|| Config::load().map(|config| fletch_cache_root(&config)))
        .transpose()
        .context("load FLETCH cache configuration for roster player landings")?;
    let contract_control_ledger = contract_control_ledger
        .as_deref()
        .map(std::fs::read)
        .transpose()
        .context("read --contract-control-ledger")?;
    let camp_participation_ledger = camp_participation_ledger
        .as_deref()
        .map(std::fs::read)
        .transpose()
        .context("read --camp-participation-ledger")?;
    let identity_review_ledgers = identity_review_ledgers
        .iter()
        .map(|path| {
            std::fs::read(path)
                .with_context(|| format!("read --identity-review-ledger {}", path.display()))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let package = run_prospect_source_audit_with_artifacts(
        &NhlApiClient::production(),
        &store,
        &catalog,
        ProspectSourceAuditInput {
            captured_at,
            effective_cutoff: captured_at,
            knowledge_cutoff: captured_at,
            finalize_cutoffs_after_acquisition,
        },
        ProspectSourceAuditArtifacts {
            ahl_roster_snapshot,
            ahl_identity_reviews,
            ahl_review_registry_url,
            roster_player_landing_cache_root,
            contract_control_ledger,
            camp_participation_ledger,
            identity_review_ledgers,
        },
    )
    .await?;
    if let Some(path) = out {
        icelines_fetch::atomic_write::write_json_atomic(&path, &package)
            .with_context(|| format!("write prospect source package {}", path.display()))?;
    }
    let acquired = package
        .run_manifest
        .objects
        .iter()
        .filter(|object| {
            matches!(
                object.state,
                icelines_core::source_facts::SourceObjectState::Acquired { .. }
            )
        })
        .count();
    println!(
        "sealed {} fingerprint={} complete={} objects={} acquired={} disclosures={}",
        package.package_id,
        package.fingerprint,
        package.run_manifest.complete,
        package.run_manifest.objects.len(),
        acquired,
        package.disclosures.len(),
    );
    Ok(())
}

async fn do_ahl_transactions(
    season: &str,
    out: Option<&std::path::Path>,
    refresh: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let season_id: u32 = season
        .parse()
        .with_context(|| format!("AHL season must be an 8-digit value, got `{season}`"))?;
    if season.len() != 8 {
        return Err(anyhow!(
            "AHL season must be an 8-digit value, got `{season}`"
        ));
    }
    if dry_run {
        println!("Would resolve AHL regular season {season} from the official season catalog.");
        println!("Would acquire every league transaction page through verified FLETCH cachelines.");
        println!("Would seal {season}-<date>-ahl-transactions/ahl/ahl-transactions.json.");
        if let Some(out) = out {
            println!("Would also export {}", out.display());
        }
        return Ok(());
    }

    let cfg = Config::load().context("loading IceLines config for AHL transactions")?;
    let icelines_home = cfg
        .snapshot_dir()
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _lock = fetch_lock::acquire(&icelines_home, std::time::Duration::from_secs(120))
        .with_context(|| {
            format!(
                "acquiring AHL transaction fetch lock at {}/.fetch.lock",
                icelines_home.display()
            )
        })?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let parent = store
        .load_manifest()
        .context("loading snapshot manifest before AHL transaction side-fetch")?
        .active;
    let client =
        icelines_fetch::ahl::AhlFeedClient::production_cached(fletch_cache_root(&cfg), refresh);
    let snapshot = icelines_fetch::ahl_transactions::fetch_ahl_transactions(&client, season_id)
        .await
        .context("fetching official AHL transaction snapshot")?;
    let bytes = serde_json::to_vec_pretty(&snapshot)
        .context("serializing official AHL transaction snapshot")?;
    let today = today_date();
    let snapshot_name = format!("{season}-{today}-ahl-transactions");
    store
        .create(&snapshot_name, season, SnapshotTier::Ahl, parent, &today)
        .context("creating AHL transaction snapshot")?;
    store
        .write_file(
            &snapshot_name,
            &SnapshotTier::Ahl,
            "ahl-transactions.json",
            &bytes,
        )
        .context("writing typed AHL transaction snapshot")?;
    store
        .seal(&snapshot_name)
        .context("sealing AHL transaction snapshot")?;
    if let Some(out) = out {
        icelines_fetch::atomic_write::write_bytes_atomic(out, &bytes)
            .with_context(|| format!("exporting AHL transaction snapshot to {}", out.display()))?;
    }
    println!(
        "AHL {}: {} transaction(s) across {} source page(s)",
        snapshot.provider_season_name,
        snapshot.total_results,
        snapshot.pages.len()
    );
    println!("Sealed snapshot '{snapshot_name}' (tier: ahl).");
    if let Some(out) = out {
        println!("Exported {}", out.display());
    }
    Ok(())
}

async fn do_ahl(
    season: &str,
    teams: &[String],
    out: Option<&std::path::Path>,
    refresh: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let season_id: u32 = season
        .parse()
        .with_context(|| format!("AHL season must be an 8-digit value, got `{season}`"))?;
    if season.len() != 8 {
        return Err(anyhow!(
            "AHL season must be an 8-digit value, got `{season}`"
        ));
    }

    if dry_run {
        let scope = if teams.is_empty() {
            "all provider-catalog teams".to_owned()
        } else {
            teams.join(", ")
        };
        println!("Would resolve AHL regular season {season} from the official season catalog.");
        println!("Would acquire roster/skater/goalie reports for {scope} through FLETCH.");
        let snapshot_scope = if teams.is_empty() {
            String::new()
        } else {
            format!("-{}", teams.join("-").to_ascii_lowercase())
        };
        println!("Would seal {season}-<date>-ahl{snapshot_scope}/ahl/ahl-roster-stats.json.");
        if let Some(out) = out {
            println!("Would also export {}", out.display());
        }
        return Ok(());
    }

    let cfg = Config::load().context("loading IceLines config for AHL snapshot")?;
    let icelines_home = cfg
        .snapshot_dir()
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _lock = fetch_lock::acquire(&icelines_home, std::time::Duration::from_secs(120))
        .with_context(|| {
            format!(
                "acquiring AHL fetch lock at {}/.fetch.lock",
                icelines_home.display()
            )
        })?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let parent = store
        .load_manifest()
        .context("loading snapshot manifest before AHL side-fetch")?
        .active;
    let snapshot =
        icelines_fetch::ahl::AhlFeedClient::production_cached(fletch_cache_root(&cfg), refresh)
            .fetch_roster_stats(season_id, teams)
            .await
            .context("fetching official AHL roster/stat snapshot")?;
    let bytes =
        serde_json::to_vec_pretty(&snapshot).context("serializing AHL roster/stat snapshot")?;
    let today = today_date();
    // A team-scoped fetch is a useful side snapshot, but must never replace the
    // same-day full-league AHL snapshot in the manifest or on disk.
    let snapshot_scope = if teams.is_empty() {
        String::new()
    } else {
        let mut team_codes: Vec<_> = snapshot
            .teams
            .iter()
            .map(|team| team.team_code.to_ascii_lowercase())
            .collect();
        team_codes.sort();
        format!("-{}", team_codes.join("-"))
    };
    let snapshot_name = format!("{season}-{today}-ahl{snapshot_scope}");
    store
        .create(&snapshot_name, season, SnapshotTier::Ahl, parent, &today)
        .context("creating AHL snapshot")?;
    store
        .write_file(
            &snapshot_name,
            &SnapshotTier::Ahl,
            "ahl-roster-stats.json",
            &bytes,
        )
        .context("writing typed AHL snapshot")?;
    store.seal(&snapshot_name).context("sealing AHL snapshot")?;
    if let Some(out) = out {
        icelines_fetch::atomic_write::write_bytes_atomic(out, &bytes)
            .with_context(|| format!("exporting AHL roster/stat snapshot to {}", out.display()))?;
    }
    let skaters: usize = snapshot.teams.iter().map(|team| team.skaters.len()).sum();
    let goalies: usize = snapshot.teams.iter().map(|team| team.goalies.len()).sum();
    let roster_players: usize = snapshot.teams.iter().map(|team| team.roster.len()).sum();
    println!(
        "AHL {}: {} team(s), {} roster player(s), {} skater row(s), {} goalie row(s)",
        snapshot.provider_season_name,
        snapshot.teams.len(),
        roster_players,
        skaters,
        goalies
    );
    println!("Sealed snapshot '{snapshot_name}' (tier: ahl).");
    if let Some(out) = out {
        println!("Exported {}", out.display());
    }
    Ok(())
}

async fn do_goal_visualizer(
    game_id: u64,
    event_id: Option<u32>,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    use icelines_core::identity::GameId;
    use icelines_fetch::goal_visualizer::{
        build_event, discover_goals, fetch_replay_bytes, merge_bundle, sha256_hex,
    };
    use icelines_fetch::manifest::{DataKey, DataKind, ManifestEntry};

    let client = NhlApiClient::production();
    let landing = client
        .fetch_game_landing_raw(game_id)
        .await
        .with_context(|| format!("fetching Gamecenter landing for game {game_id}"))?;
    let mut goals = discover_goals(&landing)
        .with_context(|| format!("discovering Goal Visualizer rows for game {game_id}"))?;
    if let Some(event_id) = event_id {
        goals.retain(|goal| goal.event_id == event_id);
        if goals.is_empty() {
            anyhow::bail!("game {game_id} has no goal event {event_id} in Gamecenter landing");
        }
    }
    if goals.is_empty() {
        println!("Game {game_id} has no goals in Gamecenter landing.");
        return Ok(());
    }

    println!(
        "Goal Visualizer discovery — game {game_id} · {} goal(s){}",
        goals.len(),
        event_id
            .map(|id| format!(" · event {id}"))
            .unwrap_or_default()
    );
    for goal in &goals {
        println!(
            "  · event {} P{} {} {} — {}",
            goal.event_id,
            goal.period,
            goal.time_in_period,
            goal.scorer_name.as_deref().unwrap_or("unknown scorer"),
            if goal.replay_url.is_some() {
                "tracking available"
            } else {
                "tracking unavailable"
            }
        );
    }
    if dry_run {
        println!("(dry run — no tracking frames fetched or written)");
        return Ok(());
    }

    let data_root = icelines_data_root()?;
    let store = icelines_fetch::datastore::DataStore::open(&data_root).context("open DataStore")?;
    let path = data_root
        .join("goal_visualizer")
        .join(format!("{game_id}.json"));

    let mut fetched_events = Vec::new();
    let mut unavailable = 0usize;
    for goal in goals {
        let Some(replay_url) = goal.replay_url.clone() else {
            unavailable += 1;
            continue;
        };
        let bytes = fetch_replay_bytes(
            game_id,
            goal.event_id,
            &replay_url,
            data_root.join(".fletch"),
            force,
        )
        .await?;
        let raw: serde_json::Value = serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "parsing Goal Visualizer game {game_id} event {}",
                goal.event_id
            )
        })?;
        let source_sha256 = sha256_hex(&bytes);
        let event = build_event(goal, &raw, source_sha256)?;
        println!(
            "  · event {} validated: {} frames, {} tracked players, puck={}",
            event.goal.event_id,
            event.frame_count,
            event.tracked_player_ids.len(),
            if event.puck_object_observed {
                "observed"
            } else {
                "not observed"
            }
        );
        fetched_events.push(event);
    }

    let persisted = fetched_events.len();
    if persisted > 0 {
        let lock_root = data_root
            .join("goal_visualizer")
            .join(".locks")
            .join(game_id.to_string());
        let _guard =
            icelines_fetch::fetch_lock::acquire(&lock_root, std::time::Duration::from_secs(120))
                .with_context(|| format!("locking Goal Visualizer bundle for game {game_id}"))?;
        merge_bundle(&path, game_id, chrono::Utc::now(), fetched_events)?;
        store.manifest().upsert(
            DataKind::GoalVisualizer,
            ManifestEntry {
                key: DataKey::Game(GameId(game_id)),
                path: path.clone(),
                freshness: icelines_core::Freshness {
                    fetched_at: chrono::Utc::now(),
                    source: icelines_core::FetchSource::Live,
                    ttl: icelines_core::Ttl::Static,
                },
            },
        )?;
        println!("Wrote {}", path.display());
    }
    println!("Done. Goal Visualizer persisted: {persisted}; unavailable: {unavailable}");
    Ok(())
}

async fn do_fletch_quivers(
    season: &str,
    season_type: FetchSeasonType,
    out: &std::path::Path,
    gate: bool,
) -> anyhow::Result<()> {
    let report =
        icelines_fetch::fletch::fletch_query_quiver_report(season, season_type_label(season_type));
    icelines_fetch::fletch::write_fletch_query_quivers(out, &report)
        .with_context(|| format!("writing FLETCH query quiver report to {}", out.display()))?;

    println!(
        "FLETCH query quivers: {} quiver(s), {} member partition(s), {} adapter-required",
        report.quiver_count, report.member_count, report.adapter_required_partition_count,
    );
    println!("Wrote {}", out.display());

    if gate {
        let failures = icelines_fetch::fletch::fletch_query_quiver_gate_failures(&report);
        if !failures.is_empty() {
            return Err(anyhow!(
                "FLETCH query quiver gate failed:\n{}",
                failures.join("\n")
            ));
        }
        println!("FLETCH query quiver gate passed.");
    }

    Ok(())
}

async fn do_fletch_cache_index(
    season: &str,
    season_type: FetchSeasonType,
    manifest: Option<&std::path::Path>,
    out: &std::path::Path,
    gate: bool,
) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let default_manifest =
        icelines_fetch::fletch::fletch_cache_manifest_path(&fletch_cache_root(&cfg));
    let manifest_path = manifest.unwrap_or(default_manifest.as_path());
    let manifest = icelines_fetch::fletch::read_fletch_cache_manifest(manifest_path)?;
    let report = icelines_fetch::fletch::fletch_cache_index_report(
        season,
        season_type_label(season_type),
        &manifest,
    );
    icelines_fetch::fletch::write_fletch_cache_index(out, &report)
        .with_context(|| format!("writing FLETCH cache-index report to {}", out.display()))?;

    println!(
        "FLETCH cache index: {} indexed source(s), {} missing, {} unexpected, {} unverified",
        report.indexed_source_count,
        report.missing_source_count,
        report.unexpected_index_count,
        report.unverified_index_count,
    );
    println!("Read {}", manifest_path.display());
    println!("Wrote {}", out.display());

    if gate {
        let failures = icelines_fetch::fletch::fletch_cache_index_gate_failures(&report);
        if !failures.is_empty() {
            return Err(anyhow!(
                "FLETCH cache index gate failed:\n{}",
                failures.join("\n")
            ));
        }
        println!("FLETCH cache index gate passed.");
    }

    Ok(())
}

async fn do_fletch_partitions(
    season: &str,
    season_type: FetchSeasonType,
    out: &std::path::Path,
    gate: bool,
) -> anyhow::Result<()> {
    let report = icelines_fetch::fletch::fletch_query_partition_report(
        season,
        season_type_label(season_type),
    );
    icelines_fetch::fletch::write_fletch_query_partitions(out, &report)
        .with_context(|| format!("writing FLETCH query partition report to {}", out.display()))?;

    println!(
        "FLETCH query partitions: {} partition(s), {} rollup(s), {} adapter-required",
        report.partition_count, report.rollup_count, report.adapter_required_count,
    );
    println!("Wrote {}", out.display());

    if gate {
        let failures = icelines_fetch::fletch::fletch_query_partition_gate_failures(&report);
        if !failures.is_empty() {
            return Err(anyhow!(
                "FLETCH query partition gate failed:\n{}",
                failures.join("\n")
            ));
        }
        println!("FLETCH query partition gate passed.");
    }

    Ok(())
}

async fn do_fletch_sources(
    season: &str,
    season_type: FetchSeasonType,
    out: &std::path::Path,
    gate: bool,
) -> anyhow::Result<()> {
    let report = icelines_fetch::fletch::fletch_source_handoff_report(
        season,
        season_type_label(season_type),
    );
    icelines_fetch::fletch::write_fletch_source_handoff(out, &report)
        .with_context(|| format!("writing FLETCH source handoff to {}", out.display()))?;

    println!(
        "FLETCH source handoff: {} fletches, {} source(s), {} adapter-required, {} validation finding(s)",
        report.fletch_count,
        report.source_count,
        report.adapter_source_count,
        report.validation_finding_count,
    );
    println!("Wrote {}", out.display());

    if gate {
        let failures = icelines_fetch::fletch::fletch_source_handoff_gate_failures(&report);
        if !failures.is_empty() {
            return Err(anyhow!(
                "FLETCH source gate failed:\n{}",
                failures.join("\n")
            ));
        }
        println!("FLETCH source gate passed.");
    }

    Ok(())
}

fn season_type_label(season_type: FetchSeasonType) -> &'static str {
    match season_type {
        FetchSeasonType::Regular => "regular",
        FetchSeasonType::Playoff => "playoff",
        FetchSeasonType::Both => "both",
    }
}

async fn do_play_by_play(
    date: Option<String>,
    for_favorites: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    use crate::commands::tonight::parse_iso_date;
    use chrono::Utc;
    use icelines_core::identity::GameId;
    use icelines_fetch::manifest::{DataKey, DataKind, ManifestEntry};

    let anchor_str = match date.as_deref() {
        Some(d) => parse_iso_date(d)?,
        None => Utc::now().date_naive().format("%Y-%m-%d").to_string(),
    };

    if !crate::config::live_feeds_enabled() {
        anyhow::bail!(
            "live feeds are disabled; `fetch play-by-play` requires live NHL schedule data"
        );
    }

    let client = NhlApiClient::production();
    let games = client
        .fetch_schedule_for_date(&anchor_str)
        .await
        .with_context(|| format!("fetching schedule for {anchor_str}"))?;
    let same_day: Vec<_> = games.into_iter().filter(|g| g.date == anchor_str).collect();
    if same_day.is_empty() {
        println!("No games scheduled on {anchor_str}.");
        return Ok(());
    }

    let favorited_teams: std::collections::HashSet<String> = if for_favorites {
        let db = crate::db::GroupDb::open().context("open group db")?;
        db.list_members_with_kind("Favorites")
            .unwrap_or_default()
            .iter()
            .filter(|(_, kind)| matches!(kind, crate::db::MemberKind::Team))
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
        "Play-by-play fetch — {anchor_str} · {} game(s){}",
        to_fetch.len(),
        if for_favorites {
            format!(
                " (favorites only — {} team(s) tracked)",
                favorited_teams.len()
            )
        } else {
            String::new()
        }
    );

    if dry_run {
        for g in &to_fetch {
            println!(
                "  · {} @ {}  game_id={}",
                g.away_abbrev, g.home_abbrev, g.game_id
            );
        }
        println!("(dry run — no play-by-play files fetched)");
        return Ok(());
    }

    let data_root = icelines_data_root()?;
    let store = icelines_fetch::datastore::DataStore::open(&data_root).context("open DataStore")?;
    let game_ids = to_fetch.iter().map(|game| game.game_id).collect::<Vec<_>>();
    let raw_by_game = icelines_fetch::fletch::fetch_gamecenter_batch_bytes_async(
        game_ids,
        icelines_fetch::fletch::FletchGamecenterArtifact::PlayByPlay,
        data_root.join(".fletch"),
        false,
    )
    .await
    .context("fetching play-by-play batch through FLETCH")?;
    let mut persisted = 0usize;
    let mut skipped = 0usize;

    for g in &to_fetch {
        match raw_by_game.get(&g.game_id) {
            Some(raw_bytes) => {
                let raw: serde_json::Value =
                    serde_json::from_slice(raw_bytes).context("parse play-by-play body")?;
                let parsed = icelines_fetch::nhl_api::parse_play_by_play(&raw, g.game_id);
                let path = data_root
                    .join("play_by_play")
                    .join(&anchor_str)
                    .join(format!("{}.json", g.game_id));
                if let Err(e) = icelines_fetch::atomic_write::write_bytes_atomic(&path, raw_bytes) {
                    skipped += 1;
                    eprintln!(
                        "  ! play-by-play body write failed for game {}: {e}",
                        g.game_id
                    );
                    continue;
                }

                let entry = ManifestEntry {
                    key: DataKey::Game(GameId(g.game_id)),
                    path,
                    freshness: icelines_core::Freshness {
                        fetched_at: chrono::Utc::now(),
                        source: icelines_core::FetchSource::Live,
                        ttl: icelines_core::Ttl::Static,
                    },
                };
                if let Err(e) = store.manifest().upsert(DataKind::PlayByPlay, entry) {
                    skipped += 1;
                    eprintln!("  ! manifest upsert failed for game {}: {e}", g.game_id);
                } else {
                    persisted += 1;
                    println!(
                        "  · game {} persisted ({} scoring events, {} goals, {} penalties)",
                        parsed.game_id,
                        parsed.scoring_events.len(),
                        parsed.goals.len(),
                        parsed.penalties.len()
                    );
                }
            }
            None => {
                skipped += 1;
                eprintln!(
                    "  · play-by-play skipped for game {} (missing FLETCH cache result)",
                    g.game_id
                );
            }
        }
    }

    println!("Done. Play-by-play persisted: {persisted}; skipped: {skipped}");
    Ok(())
}

/// Phase Lindsay L.1.6 — generic per-report fetcher.
///
/// Tier-1 only for L.1; Tier-2 lands in L.6 alongside the runtime
/// `extra_reports` cache. Decision tree:
///   1. validate `kind.is_known_working()` (TAPE-R3 follow-up #3)
///   2. validate `kind.tier() == Tier1` (Tier-2 deferred)
///   3. acquire fs lock (unless `--no-lock`) — TAPE-R3 cross-process guard
///   4. fetch via FLETCH's generic paged JSON cacheline (DI-29 fence applies
///      at *load* time when the file is read back through
///      `load_report_with_fallback`; here we just write the envelope verbatim)
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

    // (4) HTTP fetch via FLETCH's generic paged JSON cacheline. ICELINES
    // still owns the snapshot write, typed parsing, and active pointer.
    let report_bytes = icelines_fetch::fletch::fetch_paged_report_bytes_async(
        kind,
        season.to_string(),
        st.label().to_string(),
        fletch_cache_root(&cfg),
        false,
    )
    .await
    .with_context(|| format!("fetching {} {} {}", kind.url_path(), season, st.label()))?;
    let envelope: serde_json::Value = serde_json::from_slice(&report_bytes)
        .with_context(|| format!("parsing FLETCH paged report {}", kind.url_path()))?;
    let row_count = envelope
        .get("data")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .with_context(|| format!("FLETCH paged report {} missing data array", kind.url_path()))?;

    // (5) Write the FLETCH-acquired `{"data": [...], "total": N}`
    // envelope to the per-window file. Atomic write via the existing
    // snapshot helpers.
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
        row_count,
        kind.url_path(),
        st.label(),
        target.display(),
    );
    Ok(())
}

async fn do_rosters(season: &str, refresh: bool, dry_run: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let today = today_date();
    let snap = format!("{season}-{today}-rosters");
    let teams = icelines_fetch::nhl_teams_for_season(season);

    if dry_run {
        println!("Would create snapshot: {snap}");
        for team in &teams {
            println!("  {team}: would fetch {}", roster_url(team, season));
        }
        return Ok(());
    }

    // Check if already sealed today
    if !refresh {
        if let Ok(entries) = store.list() {
            if entries
                .iter()
                .any(|e| e.name == snap && e.sealed && e.file_count >= teams.len())
            {
                println!("Rosters already fetched today (use --refresh to re-fetch).");
                println!("  Snapshot: {snap}");
                return Ok(());
            }
            if entries
                .iter()
                .any(|e| e.name == snap && e.file_count < teams.len())
            {
                println!("Roster snapshot is incomplete; resuming from verified cache.");
            }
        }
    }

    let resumed_files: std::collections::HashMap<&str, Vec<u8>> = if refresh {
        std::collections::HashMap::new()
    } else {
        teams
            .iter()
            .filter_map(|team| {
                let path = store
                    .root()
                    .join(&snap)
                    .join(SnapshotTier::Rosters.dir_name())
                    .join(format!("{team}.json"));
                std::fs::read(path).ok().map(|bytes| (*team, bytes))
            })
            .collect()
    };

    let observed_at = Utc::now().to_rfc3339();
    store
        .create_with_evidence(
            &snap,
            season,
            SnapshotTier::Rosters,
            None,
            &today,
            Some(observed_at.clone()),
            Some(OFFICIAL_NHL_LIVE_ROSTER_SOURCE.to_owned()),
        )
        .context("creating roster snapshot")?;

    println!("Fetching rosters → snapshot '{snap}'");
    let fletch_cache = fletch_cache_root(&cfg);
    let requests: Vec<_> = teams
        .iter()
        .filter(|team| !resumed_files.contains_key(**team))
        .map(|team| {
            (
                icelines_fetch::fletch::roster_dataset_id(team, season),
                icelines_fetch::fletch::roster_url(team, season),
            )
        })
        .collect();
    let fetched =
        icelines_fetch::fletch::fetch_generic_http_batch_async(requests, fletch_cache, refresh, 8)
            .await;
    let fetched_by_id: std::collections::HashMap<_, _> = fetched.into_iter().collect();
    let mut written = 0usize;
    for team in &teams {
        let fletch_id = icelines_fetch::fletch::roster_dataset_id(team, season);
        let roster_bytes = match resumed_files.get(team) {
            Some(bytes) => bytes.clone(),
            None => match fetched_by_id.get(&fletch_id) {
                Some(Ok(bytes)) => bytes.clone(),
                Some(Err(e)) => {
                    // Preserve an explicit partial failure if an expected
                    // season member cannot be fetched.
                    println!("  {team}: skipped ({e:#})");
                    continue;
                }
                None => {
                    println!("  {team}: skipped (fetch result missing)");
                    continue;
                }
            },
        };
        let roster: icelines_fetch::schema::RosterResponse = serde_json::from_slice(&roster_bytes)
            .with_context(|| format!("parsing roster JSON for {team}"))?;
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
        written += 1;
        println!("  {team}: {count} players");
    }

    if written != teams.len() {
        anyhow::bail!(
            "roster snapshot incomplete: wrote {written}/{} teams; retry without --refresh to resume",
            teams.len()
        );
    }
    let source_manifest = OfficialNhlRosterCaptureManifest {
        schema: OFFICIAL_NHL_LIVE_ROSTER_SCHEMA.to_owned(),
        season: season.to_owned(),
        observed_at,
        captures: teams
            .iter()
            .map(|team| OfficialNhlRosterCapture {
                team: (*team).to_owned(),
                source_url: roster_url(team, season),
            })
            .collect(),
    };
    store
        .write_file(
            &snap,
            &SnapshotTier::Rosters,
            OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE,
            &serde_json::to_vec_pretty(&source_manifest)
                .context("serializing official roster source manifest")?,
        )
        .context("writing official roster source manifest")?;
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

async fn do_moneypuck(season: &str, seasons: u8, dry_run: bool) -> anyhow::Result<()> {
    let season_window = moneypuck_season_window(season, seasons)?;
    if season_window.len() > 1 {
        println!(
            "MoneyPuck historical xG fetch — {} season(s), latest {}",
            season_window.len(),
            season
        );
    }

    for season in season_window {
        do_moneypuck_one(&season, dry_run).await?;
    }

    Ok(())
}

async fn do_moneypuck_one(season: &str, dry_run: bool) -> anyhow::Result<()> {
    let url = moneypuck::csv_url(season).with_context(|| {
        format!("invalid season format '{season}' — expected 8 digits like 20252026")
    })?;

    if dry_run {
        println!("Would download MoneyPuck CSV: {url}");
        return Ok(());
    }

    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let today = today_date();
    let snap = format!("{season}-{today}-moneypuck");

    println!("Downloading MoneyPuck CSV: {url}");
    let fletch_id = format!("icelines.moneypuck.{season}.skaters");
    let csv_bytes = icelines_fetch::fletch::fetch_generic_http_bytes_async(
        fletch_id,
        url,
        fletch_cache_root(&cfg),
        true,
    )
    .await
    .context("downloading MoneyPuck CSV through FLETCH")?;
    let csv_text = String::from_utf8(csv_bytes).context("MoneyPuck CSV is not valid UTF-8")?;

    let stats_map = moneypuck::parse_csv_checked(&csv_text)
        .context("parsing MoneyPuck CSV; source schema or required columns changed")?;
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

fn moneypuck_season_window(latest_season: &str, count: u8) -> anyhow::Result<Vec<String>> {
    if latest_season.len() != 8 || !latest_season.chars().all(|c| c.is_ascii_digit()) {
        anyhow::bail!("invalid season format '{latest_season}' — expected 8 digits like 20252026");
    }
    let start: i32 = latest_season[..4].parse()?;
    let end: i32 = latest_season[4..].parse()?;
    if end != start + 1 {
        anyhow::bail!(
            "invalid season format '{latest_season}' — expected consecutive years like 20252026"
        );
    }
    Ok((0..count)
        .map(|offset| {
            let y = start - i32::from(offset);
            format!("{y}{next}", next = y + 1)
        })
        .collect())
}

async fn do_contracts(
    season: &str,
    valuation_season: &str,
    source: ContractSource,
    input: Option<&std::path::Path>,
    cap_limit: Option<u64>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());

    // A goalie or transaction side-fetch may be the active Stats-tier snapshot
    // without containing bios.json. Resolve the newest sealed snapshot for the
    // requested season that actually carries the file.
    let bios: Vec<SkaterBio> = store
        .read_tier_file_any_for_season(&SnapshotTier::Stats, "bios.json", season)
        .context(
            "reading bios.json from a sealed Stats snapshot — run `icelines fetch stats` first",
        )?;

    let player_ids: Vec<u32> = bios.iter().map(|b| b.player_id).collect();
    let n = player_ids.len();

    let csv_input = match (source, input) {
        (ContractSource::Csv, Some(path)) => Some(path),
        (ContractSource::Csv, None) => anyhow::bail!("--source csv requires --input PATH"),
        (_, Some(_)) => anyhow::bail!("--input is only valid with --source csv"),
        (_, None) => None,
    };

    if dry_run {
        if source == ContractSource::CapWages {
            println!("Would fetch licensed CapWages contract data for {n} players.");
            println!("Roster snapshot season: {season}");
            println!("Contract valuation season: {valuation_season}");
            println!("Authentication: CAPWAGES_API_KEY environment variable (never persisted)");
            if let Some(limit) = cap_limit {
                println!("Would write team-cap-summary.json using upper limit ${limit}.");
            }
            return Ok(());
        }
        if let Some(path) = csv_input {
            let contracts =
                icelines_fetch::contracts_csv::load_contracts_csv(path, &bios, valuation_season)?;
            println!("Validated {} local contract rows.", contracts.len());
            println!("Input: {}", path.display());
            println!("Roster snapshot season: {season}");
            println!("Contract valuation season: {valuation_season}");
            if let Some(limit) = cap_limit {
                println!("Would write team-cap-summary.json using upper limit ${limit}.");
            }
            return Ok(());
        }
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

    let mut contracts = match source {
        ContractSource::CapWages => {
            println!("Fetching licensed CapWages values for {n} players...");
            icelines_fetch::capwages::CapWagesClient::from_env()?
                .fetch_contracts(&bios, valuation_season)
                .await?
        }
        ContractSource::Csv => {
            let path = csv_input.expect("validated CSV input");
            println!("Loading local contract overlay from {}...", path.display());
            icelines_fetch::contracts_csv::load_contracts_csv(path, &bios, valuation_season)?
        }
        ContractSource::Nhl => {
            println!(
                "Fetching contract data for {n} players (this takes ~{}s)...",
                (n as f64 * 0.05).ceil() as u64
            );
            let landing_by_player = icelines_fetch::fletch::fetch_player_landing_batch_bytes_async(
                player_ids.clone(),
                icelines_fetch::fletch::FletchPlayerLandingArtifact::Landing,
                fletch_cache_root(&cfg),
                false,
                50,
            )
            .await
            .context("fetching player landing contract batch through FLETCH")?;
            player_ids
                .iter()
                .filter_map(|player_id| {
                    let raw_bytes = landing_by_player.get(player_id)?;
                    let raw = match serde_json::from_slice::<serde_json::Value>(raw_bytes) {
                        Ok(raw) => raw,
                        Err(error) => {
                            eprintln!("  contracts: skipping player {player_id}: {error}");
                            return None;
                        }
                    };
                    Some(icelines_fetch::nhl_api::parse_player_landing_contract(
                        *player_id, &raw,
                    ))
                })
                .collect::<Vec<_>>()
        }
    };
    for contract in &mut contracts {
        contract.valuation_season = Some(valuation_season.to_owned());
    }
    let found = contracts.len();

    // Do not create a snapshot until the upstream or local source has parsed
    // successfully; validation failures must not leave partial snapshots.
    let json = serde_json::to_vec(&contracts).context("serializing contracts")?;
    let today = today_date();
    let snap = if source == ContractSource::Csv {
        let digest = format!("{:x}", Sha256::digest(&json));
        format!("{season}-{today}-contracts-csv-{}", &digest[..8])
    } else {
        format!("{season}-{today}-contracts")
    };
    store
        .create(&snap, season, SnapshotTier::Contracts, None, &today)
        .context("creating contracts snapshot")?;

    store
        .write_file(&snap, &SnapshotTier::Contracts, "contracts.json", &json)
        .context("writing contracts.json")?;

    if let Some(upper_limit) = cap_limit {
        let mut roster_players = Vec::new();
        for team in icelines_fetch::nhl_teams_for_season(season) {
            let roster: icelines_fetch::schema::RosterResponse = match store
                .read_tier_file_any_for_season(
                    &SnapshotTier::Rosters,
                    &format!("{team}.json"),
                    season,
                ) {
                Ok(roster) => roster,
                Err(_) => continue,
            };
            roster_players.extend(
                roster
                    .forwards
                    .iter()
                    .chain(&roster.defensemen)
                    .chain(&roster.goalies)
                    .map(|player| (team.to_owned(), player.id)),
            );
        }
        if roster_players.is_empty() {
            anyhow::bail!("cannot calculate team cap shares: no roster snapshots found");
        }
        let summary =
            icelines_fetch::capwages::summarize_team_caps(&roster_players, &contracts, upper_limit);
        let json = serde_json::to_vec_pretty(&summary).context("serializing team cap summary")?;
        store
            .write_file(
                &snap,
                &SnapshotTier::Contracts,
                "team-cap-summary.json",
                &json,
            )
            .context("writing team-cap-summary.json")?;
    }

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
async fn do_career(
    dry_run: bool,
    bundled_seasons: u8,
    prospect_context_path: Option<&std::path::Path>,
    camp_forecast_path: Option<&std::path::Path>,
    league_crosswalk_path: Option<&std::path::Path>,
    affiliate_workboard_path: Option<&std::path::Path>,
) -> anyhow::Result<()> {
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
    let target_modes = usize::from(prospect_context_path.is_some())
        + usize::from(camp_forecast_path.is_some())
        + usize::from(league_crosswalk_path.is_some())
        + usize::from(affiliate_workboard_path.is_some())
        + usize::from(bundled_seasons > 0);
    if target_modes > 1 {
        anyhow::bail!(
            "choose only one of --prospect-context, --camp-forecast, --league-crosswalk, --affiliate-workboard, or --bundled-seasons"
        );
    }
    let player_ids_result: anyhow::Result<Vec<u32>> = if let Some(context_path) =
        prospect_context_path
    {
        let context: icelines_fetch::ProspectLeagueContext = serde_json::from_slice(
            &std::fs::read(context_path)
                .with_context(|| format!("read prospect context {}", context_path.display()))?,
        )
        .with_context(|| format!("parse prospect context {}", context_path.display()))?;
        if context.schema != icelines_fetch::PROSPECT_LEAGUE_CONTEXT_SCHEMA {
            anyhow::bail!(
                "invalid prospect context schema in {}",
                context_path.display()
            );
        }
        Ok(context
            .players
            .into_iter()
            .map(|player| player.player_id)
            .collect())
    } else if let Some(forecast_path) = camp_forecast_path {
        let forecast: icelines_core::TrainingCampLeagueForecastView = serde_json::from_slice(
            &std::fs::read(forecast_path)
                .with_context(|| format!("read camp forecast {}", forecast_path.display()))?,
        )
        .with_context(|| format!("parse camp forecast {}", forecast_path.display()))?;
        if forecast.schema != icelines_core::TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA {
            anyhow::bail!(
                "invalid camp forecast schema in {}",
                forecast_path.display()
            );
        }
        let mut ids = std::collections::BTreeSet::new();
        for team in forecast.teams {
            if let Some(team_forecast) = team.forecast {
                ids.extend(
                    team_forecast
                        .players
                        .into_iter()
                        .filter(|player| player.prospect)
                        .map(|player| player.player_id),
                );
            }
        }
        Ok(ids.into_iter().collect())
    } else if let Some(crosswalk_path) = league_crosswalk_path {
        let crosswalk: icelines_fetch::ahl::AhlIdentityLeagueCrosswalkView =
            serde_json::from_slice(
                &std::fs::read(crosswalk_path).with_context(|| {
                    format!("read league crosswalk {}", crosswalk_path.display())
                })?,
            )
            .with_context(|| format!("parse league crosswalk {}", crosswalk_path.display()))?;
        if crosswalk.schema != icelines_fetch::ahl::AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA {
            anyhow::bail!(
                "invalid AHL identity league crosswalk schema in {}",
                crosswalk_path.display()
            );
        }
        let mut ids = std::collections::BTreeSet::new();
        for team in crosswalk.crosswalks {
            ids.extend(
                team.rows
                    .into_iter()
                    .filter(|row| {
                        row.review_status == icelines_fetch::ahl::AhlIdentityReviewStatus::Reviewed
                    })
                    .filter_map(|row| row.nhl_player_id),
            );
        }
        Ok(ids.into_iter().collect())
    } else if let Some(workboard_path) = affiliate_workboard_path {
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(workboard_path).with_context(|| {
                format!("read affiliate workboard {}", workboard_path.display())
            })?)
            .with_context(|| format!("parse affiliate workboard {}", workboard_path.display()))?;
        let value = if value.get("schema").and_then(serde_json::Value::as_str)
            == Some(
                icelines_fetch::ahl_preseason_facts::AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA,
            ) {
            value
        } else {
            value.get("workboard").cloned().with_context(|| {
                format!(
                    "{} is neither an affiliate workboard nor an application containing one",
                    workboard_path.display()
                )
            })?
        };
        let workboard: icelines_fetch::ahl_preseason_facts::AhlPreseasonLeagueFactsWorkboardView =
            serde_json::from_value(value).with_context(|| {
                format!(
                    "parse nested affiliate workboard {}",
                    workboard_path.display()
                )
            })?;
        if workboard.schema
            != icelines_fetch::ahl_preseason_facts::AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA
        {
            anyhow::bail!(
                "invalid affiliate workboard schema in {}",
                workboard_path.display()
            );
        }
        icelines_fetch::ahl_preseason_facts::validate_workboard(&workboard)
            .map_err(anyhow::Error::msg)?;
        let ids = workboard
                .team_workboards
                .into_iter()
                .flat_map(|team| team.players)
                .filter(|player| {
                    player.status
                        == icelines_fetch::ahl_preseason_facts::AhlPreseasonFactsCandidateStatus::Candidate
                })
                .filter_map(|player| player.nhl_player_id)
                .collect::<std::collections::BTreeSet<_>>();
        Ok(ids.into_iter().collect())
    } else if bundled_seasons > 0 {
        Ok(union_pids_from_bundled(bundled_seasons))
    } else {
        store
            .read_tier::<Vec<SkaterBio>>(&SnapshotTier::Stats, "bios.json")
            .context("reading bios.json from active Stats snapshot")
            .map(|bios| bios.iter().map(|b| b.player_id).collect())
    };

    if dry_run {
        if (prospect_context_path.is_some()
            || camp_forecast_path.is_some()
            || league_crosswalk_path.is_some()
            || affiliate_workboard_path.is_some())
            && player_ids_result.is_err()
        {
            return player_ids_result
                .map(|_| ())
                .context("could not resolve explicit player set for `fetch career --dry-run`");
        }
        let n = player_ids_result.as_ref().map(|p| p.len()).unwrap_or(0);
        let est_secs = (n.max(700) as f64 * 0.06).ceil() as u64;
        println!("Would fetch career history for {n} players.");
        if bundled_seasons > 0 {
            println!("Source: union of last {bundled_seasons} bundled seasons (skaters + goalies)");
        } else if let Some(context_path) = prospect_context_path {
            println!("Source: prospect context {}", context_path.display());
        } else if let Some(forecast_path) = camp_forecast_path {
            println!("Source: camp prospects in {}", forecast_path.display());
        } else if let Some(crosswalk_path) = league_crosswalk_path {
            println!(
                "Source: canonical reviewed AHL identities in {}",
                crosswalk_path.display()
            );
        } else if let Some(workboard_path) = affiliate_workboard_path {
            println!(
                "Source: canonical AHL preseason candidates in {}",
                workboard_path.display()
            );
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
    let cache_root = fletch_cache_root(&cfg);
    let landing_by_player = icelines_fetch::fletch::fetch_player_landing_batch_bytes_async(
        player_ids.clone(),
        icelines_fetch::fletch::FletchPlayerLandingArtifact::Landing,
        cache_root.clone(),
        false,
        50,
    )
    .await
    .context("fetching player landing career batch through FLETCH")?;
    let landing_manifest = icelines_fetch::fletch::read_fletch_cache_manifest(
        &icelines_fetch::fletch::fletch_cache_manifest_path(&cache_root),
    )
    .context("reading verified FLETCH player landing manifest")?;
    let landing_fetched_at = landing_manifest
        .entries
        .into_iter()
        .filter(|entry| entry.verified)
        .map(|entry| (entry.dataset_id, entry.fetched_at_ms))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut histories = Vec::with_capacity(landing_by_player.len());
    let mut birth_dates = Vec::new();
    let mut positions = Vec::new();
    let mut organization_facts = Vec::new();
    let mut skipped = Vec::new();
    let refresh_completed_at = chrono::Utc::now().to_rfc3339();
    for player_id in &player_ids {
        let Some(raw_bytes) = landing_by_player.get(player_id) else {
            skipped.push((*player_id, "missing FLETCH cache result".to_string()));
            continue;
        };
        let dataset_id = format!("icelines.player.landing.{player_id}");
        let Some(observed_at) = landing_fetched_at
            .get(&dataset_id)
            .and_then(|milliseconds| i64::try_from(*milliseconds).ok())
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|timestamp| timestamp.to_rfc3339())
        else {
            skipped.push((
                *player_id,
                "verified FLETCH landing acquisition timestamp is missing or invalid".to_owned(),
            ));
            continue;
        };
        let raw = match serde_json::from_slice::<serde_json::Value>(raw_bytes) {
            Ok(raw) => raw,
            Err(error) => {
                skipped.push((*player_id, error.to_string()));
                continue;
            }
        };
        if let Some(birth_date) = raw.get("birthDate").and_then(serde_json::Value::as_str) {
            birth_dates.push((*player_id, birth_date.to_owned()));
        }
        if let Some(position) = raw.get("position").and_then(serde_json::Value::as_str) {
            positions.push((*player_id, position.to_owned()));
        }
        match icelines_fetch::career_landing::parse_official_nhl_organization_fact(
            *player_id,
            observed_at,
            &raw,
        ) {
            Ok(fact) => organization_facts.push(fact),
            Err(error) => {
                skipped.push((*player_id, error.to_string()));
                continue;
            }
        }
        match icelines_fetch::career_landing::parse_career_history(*player_id, &raw) {
            Ok(history) => histories.push(history),
            Err(error) => skipped.push((*player_id, error.to_string())),
        }
    }

    // Merge into the existing on-disk store rather than replace it —
    // a player who was on the active roster last fetch but not this
    // one (traded out / retired) shouldn't lose their cached history.
    let mut blob = CareerHistoryStore::load(&path).context("loading existing career_history")?;
    for h in &histories {
        blob.upsert(h.clone());
    }
    for (player_id, birth_date) in birth_dates {
        blob.upsert_birth_date(player_id, birth_date);
    }
    for (player_id, position) in positions {
        blob.upsert_position(player_id, position);
    }
    for fact in organization_facts {
        blob.upsert_organization_fact(fact);
    }
    blob.fetched_at = Some(refresh_completed_at);
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
        bundled::TransactionsEnvelope, snapshot::SnapshotMetaFlags,
        transactions::raw_to_transactions,
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
    let outcome = match icelines_fetch::fletch::fetch_transactions_batch_async(
        season.to_string(),
        fletch_cache_root(&cfg),
        false,
    )
    .await
    {
        Ok(o) => o,
        Err(e) => {
            // Set the stale flag so the next `icelines transactions`
            // surfaces "snapshot is N days stale (last fetch failed)".
            let mut flags = SnapshotMetaFlags::load(&snapshots_root, season);
            flags.transactions_stale = true;
            flags.transactions_last_error = Some(e.to_string());
            flags.transactions_fetched_at = Some(today.clone());
            let _ = flags.save(&snapshots_root, season); // best-effort
            return Err(e);
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
    let anchor =
        chrono::NaiveDate::parse_from_str(&anchor_str, "%Y-%m-%d").context("parse anchor date")?;

    if dry_run && !for_favorites && !crate::config::live_feeds_enabled() {
        println!("Boxscore fetch — {anchor_str} · dry run (live feeds disabled)");
        println!("(dry run — no schedule fetched, no boxscores fetched, no events written)");
        return Ok(());
    }

    if !crate::config::live_feeds_enabled() {
        anyhow::bail!("live feeds are disabled; `fetch boxscore` requires live NHL schedule data");
    }

    // Step 1: schedule fetch (already covered by Foster.1).
    let client = NhlApiClient::production();
    let games = client
        .fetch_schedule_for_date(&anchor_str)
        .await
        .with_context(|| format!("fetching schedule for {anchor_str}"))?;
    let same_day: Vec<_> = games.into_iter().filter(|g| g.date == anchor_str).collect();
    if same_day.is_empty() {
        println!("No games scheduled on {anchor_str}.");
        return Ok(());
    }

    // Step 2: read the Favorites group up-front so we can both
    // (a) optionally filter the slate to favorited teams and
    // (b) populate per-game favorited_*_lines in the score payload
    // (Foster +4 / SCOUT-flagged in the spec).
    let (favorited_teams, favorited_player_ids): (
        std::collections::HashSet<String>,
        std::collections::HashSet<u32>,
    ) = {
        let db = crate::db::GroupDb::open().context("open group db")?;
        let members = db.list_members_with_kind("Favorites").unwrap_or_default();
        let teams: std::collections::HashSet<String> = members
            .iter()
            .filter(|(_, k)| matches!(k, crate::db::MemberKind::Team))
            .map(|(key, _)| key.to_uppercase())
            .collect();
        // Resolve each player member's normalized-name → PlayerId
        // via the bundled bios index. Members the resolver can't
        // place (rookies, retired pre-bundle, etc.) skip silently.
        let player_ids: std::collections::HashSet<u32> = members
            .iter()
            .filter(|(_, k)| matches!(k, crate::db::MemberKind::Player))
            .filter_map(|(key, _)| icelines_fetch::stats_loader::resolve_player_id_by_name(key))
            .collect();
        (teams, player_ids)
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
            format!(
                " (favorites only — {} team(s) tracked)",
                favorited_teams.len()
            )
        } else {
            String::new()
        }
    );

    if dry_run {
        for g in &to_fetch {
            println!(
                "  · {} @ {}  game_id={}",
                g.away_abbrev, g.home_abbrev, g.game_id
            );
        }
        println!("(dry run — no boxscores fetched, no events written)");
        return Ok(());
    }

    // Step 3: per game, fetch the raw boxscore JSON, persist it
    // under data/boxscores/<date>/<game_id>.json (Foster +3), then
    // upsert the score event. The manifest entry binds the JSON
    // file to (Boxscore, Game(id)) so future favorites-view reads
    // can find it deterministically. TAPE H4: write JSON first,
    // then manifest, then event — manifest is the commit point.
    let event_stream = crate::event_stream::EventStream::open().context("open events table")?;

    let home_dir = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow!("cannot determine home directory"))?;
    let data_root = home_dir.join(".icelines").join("data");
    let store = icelines_fetch::datastore::DataStore::open(&data_root).context("open DataStore")?;
    let game_ids = to_fetch.iter().map(|game| game.game_id).collect::<Vec<_>>();
    let raw_by_game = icelines_fetch::fletch::fetch_gamecenter_batch_bytes_async(
        game_ids,
        icelines_fetch::fletch::FletchGamecenterArtifact::Boxscore,
        data_root.join(".fletch"),
        false,
    )
    .await
    .context("fetching boxscore batch through FLETCH")?;

    let mut wrote = 0usize;
    let mut updated = 0usize;
    let mut persisted = 0usize;
    let mut persist_skipped = 0usize;
    for g in &to_fetch {
        let game = GameId(g.game_id);
        let away = TeamAbbr(g.away_abbrev.clone());
        let home = TeamAbbr(g.home_abbrev.clone());
        let mut parsed_for_game = None;

        // Foster +3 — fetch the raw boxscore body, persist atomically,
        // register a manifest entry. Best-effort: if the body fetch
        // fails (game not yet started, API hiccup), keep going with
        // the slate-level score event so the user still sees the row.
        match raw_by_game.get(&g.game_id) {
            Some(raw_bytes) => {
                let raw: serde_json::Value =
                    serde_json::from_slice(raw_bytes).context("parse boxscore body")?;
                parsed_for_game = Some(icelines_fetch::nhl_api::parse_boxscore(&raw, g.game_id));
                let path = data_root
                    .join("boxscores")
                    .join(&anchor_str)
                    .join(format!("{}.json", g.game_id));
                if let Err(e) = icelines_fetch::atomic_write::write_bytes_atomic(&path, raw_bytes) {
                    eprintln!("  ! boxscore body write failed for game {}: {e}", g.game_id);
                } else {
                    let entry = icelines_fetch::manifest::ManifestEntry {
                        key: icelines_fetch::manifest::DataKey::Game(game),
                        path,
                        freshness: icelines_core::Freshness {
                            fetched_at: chrono::Utc::now(),
                            source: icelines_core::FetchSource::Live,
                            ttl: icelines_core::Ttl::Static, // boxscores immutable post-game
                        },
                    };
                    if let Err(e) = store
                        .manifest()
                        .upsert(icelines_fetch::manifest::DataKind::Boxscore, entry)
                    {
                        eprintln!("  ! manifest upsert failed for game {}: {e}", g.game_id);
                    } else {
                        persisted += 1;
                    }
                }
            }
            None => {
                persist_skipped += 1;
                eprintln!(
                    "  · boxscore body skipped for game {} (missing FLETCH cache result)",
                    g.game_id
                );
            }
        }

        let result = match g.game_state.as_deref() {
            Some(s @ ("FINAL" | "OFF")) => s.to_owned(),
            Some(s @ ("LIVE" | "CRIT")) => s.to_owned(),
            Some(other) => other.to_owned(),
            None => "FUT".to_owned(),
        };
        let mut payload = proto::ScorePayloadV1::new(
            game,
            home,
            away,
            g.home_score.unwrap_or(0) as u32,
            g.away_score.unwrap_or(0) as u32,
            result,
        );

        // Phase Foster +4 — fetch the parsed boxscore (we already
        // have it from fetch_boxscore_with_raw above when the body
        // persist path succeeded) and intersect skater + goalie
        // PIDs with the Favorites group. Quiet on failure: a
        // missing boxscore body just means the lines stay empty.
        if !favorited_player_ids.is_empty() {
            if let Some(parsed_box) = parsed_for_game.as_ref() {
                use icelines_core::entity::EntityRef;
                use icelines_core::identity::PlayerId;
                for skater in parsed_box
                    .away_skaters
                    .iter()
                    .chain(parsed_box.home_skaters.iter())
                {
                    if favorited_player_ids.contains(&skater.player_id) {
                        payload
                            .favorited_skater_lines
                            .push(EntityRef::Player(PlayerId(skater.player_id)));
                    }
                }
                // Goalie matching is name-based today —
                // GoalieLine doesn't yet carry player_id from the
                // boxscore parse path. A future fetch.rs refactor
                // can switch to PID intersection mirroring skaters
                // once nhl_api::parse_boxscore extracts the goalie
                // player_id field.
                let _ = &parsed_box.goalies; // surface field; not yet wired
            }
        }
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

        // Phase Foster +25 — mid-day trade detection for any
        // favorited skater whose team in tonight's boxscore differs
        // from the bundled-bios "current team". detect_mid_day_trade
        // returns None for matches; we silently no-op when there's
        // no swap. Each detected trade gets a trade event upsert
        // with the alphabetic-team-sort dedup key from
        // event_stream::trade_event_id so re-fetching the same date
        // doesn't double-record.
        for skater_ref in &payload.favorited_skater_lines {
            let icelines_core::entity::EntityRef::Player(pid) = skater_ref else {
                continue;
            };
            // The "today" team: search both home_skaters / away_skaters
            // for the pid; the team is whichever side they appear on.
            let today_team = if let Some(parsed) = parsed_for_game.as_ref() {
                if parsed.home_skaters.iter().any(|s| s.player_id == pid.0) {
                    Some(icelines_core::TeamAbbr(parsed.home_abbrev.clone()))
                } else if parsed.away_skaters.iter().any(|s| s.player_id == pid.0) {
                    Some(icelines_core::TeamAbbr(parsed.away_abbrev.clone()))
                } else {
                    None
                }
            } else {
                None
            };
            let Some(today_team) = today_team else {
                continue;
            };
            let prior_team =
                bundled_player_team(*pid).map(|s| icelines_core::TeamAbbr(s.to_uppercase()));
            if let Some(trade) = proto::detect_mid_day_trade(*pid, &today_team, prior_team.as_ref())
            {
                let trade_json =
                    serde_json::to_string(&trade).context("serialize trade payload")?;
                let trade_id = proto::trade_event_id(anchor, &trade.from_team, &trade.to_team);
                let inserted = event_stream.upsert(
                    anchor,
                    &EntityRef::Player(*pid),
                    "trade",
                    &trade_id,
                    &trade_json,
                    proto::TRADE_PAYLOAD_VERSION,
                )?;
                if inserted {
                    wrote += 1;
                    eprintln!(
                        "  · trade detected: player:{} {} → {}",
                        pid.0, trade.from_team.0, trade.to_team.0
                    );
                }
            }
        }
    }
    println!(
        "Done — {wrote} new event(s), {updated} updated event(s), \
         {persisted} boxscore body(ies) persisted, {persist_skipped} skipped on {anchor_str}."
    );
    Ok(())
}

/// Phase Foster +25 — duplicate of favorites_view::bundled_player_team
/// scoped here so do_boxscore doesn't have to expose it through
/// the cross-module API surface. Walks the bundled bios for the
/// most recent season this PID appears in and returns their
/// `current_team_abbrev`. Returns `None` for PIDs the bundle
/// doesn't know about.
fn bundled_player_team(pid: icelines_core::identity::PlayerId) -> Option<String> {
    for season in icelines_fetch::bundled::BUNDLED_SEASONS {
        if let Some(bios) = icelines_fetch::bundled::get_bios(season) {
            if let Some(b) = bios.iter().find(|b| b.player_id == pid.0) {
                if let Some(team) = &b.current_team_abbrev {
                    return Some(team.clone());
                }
            }
        }
    }
    None
}

fn icelines_data_root() -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow!("cannot determine home directory"))?;
    Ok(home.join(".icelines").join("data"))
}

fn fletch_cache_root(cfg: &Config) -> std::path::PathBuf {
    cfg.snapshot_dir()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| cfg.snapshot_dir())
        .join("data")
        .join(".fletch")
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
            if force {
                "would be refreshed (--force)"
            } else {
                "would be refreshed"
            }
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

    #[test]
    fn l0_moneypuck_season_window_counts_back_from_latest() {
        assert_eq!(
            moneypuck_season_window("20252026", 3).unwrap(),
            vec!["20252026", "20242025", "20232024"]
        );
    }

    #[test]
    fn l0_moneypuck_season_window_rejects_non_consecutive_years() {
        let err = moneypuck_season_window("20252027", 1).expect_err("bad season must fail");
        assert!(
            err.to_string().contains("consecutive years"),
            "unexpected error: {err}"
        );
    }
}
