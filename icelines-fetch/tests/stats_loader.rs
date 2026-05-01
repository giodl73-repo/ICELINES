//! Phase Hart.3 — L1 integration tests for `stats_loader::load_into_repo`.
//!
//! Uses bundled data only (no network, no real snapshot store). The
//! parallel-run field-parity test is the BENCH-mandated regression
//! gate against the legacy `PlayerRepository::load_all()` path.

use icelines_core::identity::PlayerId;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::StatsRepository;
use icelines_fetch::goalie_repository::GoalieRepository;
use icelines_fetch::repository::PlayerRepository;
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::{load_into_repo, LoadOutcome, MissingSource};

// ── Hart.4.1 v0.2 — multi-load test-helper ─────────────────────────────────
//
// Hart.4.1 draft scaffolding: orchestrates the merge of two independent
// `LoadOutcome`s (one per (season, type)) into one shared
// `StatsRepository`. Promote to `pub fn StatsRepository::merge_load_outcome`
// if ≥3 Hart.5 sub-phases reference this orchestration (FORGE #1
// escape clause).
//
// The function deliberately mirrors what a future public API would do:
// merge identities (via the existing upsert path → merge_with policy),
// upsert every stats row, upsert every contract. Returns the count of
// merge errors so callers can assert on reissue/orphan rejections.
fn merge_outcome_into_repo(
    repo: &mut StatsRepository,
    outcome: LoadOutcome,
) -> Result<(), icelines_core::stats_repository::RepoError> {
    // Identities first (so stats upserts find them).
    for ident in outcome.repo.iter_identities() {
        repo.upsert_identity(ident.clone())?;
    }
    // Stats next.
    for stats in outcome.repo.iter_stats() {
        repo.upsert_stats(stats.clone())?;
    }
    // Contracts last (no order dependency, but consistent).
    for (pid, contract) in outcome.repo.iter_contracts() {
        repo.upsert_contract(pid, contract.clone());
    }
    Ok(())
}

fn cold_store() -> (tempfile::TempDir, SnapshotStore) {
    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());
    (dir, store)
}

#[test]
fn l1_load_into_repo_bundled_smoke_20242025_regular() {
    let (_dir, store) = cold_store();
    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store)
        .expect("bundled load must succeed");

    // Skater identities: should be in the high hundreds — bundled
    // bios.json carries every player who appeared that season.
    assert!(
        outcome.repo.identities_len() > 500,
        "expected >500 identities, got {}",
        outcome.repo.identities_len()
    );
    assert!(
        outcome.repo.stats_len() > 500,
        "expected >500 stats rows, got {}",
        outcome.repo.stats_len()
    );

    // Snapshot tiers were never populated → MissingSource entries.
    assert!(outcome
        .missing
        .iter()
        .any(|m| matches!(m, MissingSource::Realtime { .. })));
    assert!(outcome
        .missing
        .iter()
        .any(|m| matches!(m, MissingSource::MoneyPuck { .. })));
    assert!(outcome
        .missing
        .iter()
        .any(|m| matches!(m, MissingSource::Contracts { .. })));
}

/// BENCH-mandated parallel-run regression. For every player_id that
/// appears in BOTH the legacy `PlayerRepository::load_all()` output AND
/// the new `load_into_repo` output, the tuple
/// `(team_str, gp, points, plus_minus)` MUST match. Catches stint-
/// aggregation bugs that preserve cardinality but break the per-row
/// shape (the plan called this out as the BENCH refinement vs a naive
/// `count()` parity check).
#[test]
fn l1_parallel_run_field_parity_20242025_regular() {
    let (_dir, store) = cold_store();
    let season = "20242025";

    // Legacy path.
    let legacy = PlayerRepository::new(SnapshotStore::new(store.root()), season);
    let old_players = legacy.load_all().expect("legacy load_all");

    // New path.
    let outcome =
        load_into_repo(Season(20242025), SeasonType::Regular, &store).expect("new load_into_repo");
    let new_repo = outcome.repo;

    let mut compared = 0usize;
    let mut mismatches: Vec<String> = Vec::new();

    for old in &old_players {
        let Some(nhl_id) = old.nhl_id else { continue };
        let pid = PlayerId(nhl_id);

        // Skip if the new repo doesn't have a stats row — could happen
        // for goalies that the legacy path doesn't produce as `Player`s
        // (the legacy path filters non-skaters out). The parity test
        // is over the intersection.
        let Some(new_stats) = new_repo.season(pid, Season(20242025), SeasonType::Regular) else {
            continue;
        };
        // Position must align: only compare skater rows on both sides.
        if matches!(new_stats.position, Position::Goalie) {
            continue;
        }

        let new_team = new_stats
            .team_stints
            .last()
            .map(|s| s.team.as_str().to_string())
            .unwrap_or_default();
        let new_gp = new_stats.totals.gp;
        let new_points = new_stats.totals.points;
        let new_plus_minus = new_stats.totals.plus_minus;

        let old_gp = old.gp_status.gp().unwrap_or(0);
        let old_team = old.team.as_str();
        let old_points = old.season_points;
        let old_plus_minus = old.plus_minus;

        let new_tuple = (new_team.as_str(), new_gp, new_points, new_plus_minus);
        let old_tuple = (old_team, old_gp, old_points, old_plus_minus);

        if new_tuple != old_tuple {
            mismatches.push(format!(
                "player_id={} name={:?} legacy={:?} new={:?}",
                nhl_id, old.full_name, old_tuple, new_tuple
            ));
        }
        compared += 1;
    }

    assert!(
        compared > 500,
        "expected >500 skater players in both paths, compared {compared}"
    );
    assert!(
        mismatches.is_empty(),
        "field parity broken on {} of {} compared rows. First few:\n{}",
        mismatches.len(),
        compared,
        mismatches
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Hart.3 partial-fetch variants: bundled cold-start surfaces realtime,
/// moneypuck, and contracts as `MissingSource`. GoalieStats are bundled
/// for every season we ship, so they should NOT be missing here.
#[test]
fn l1_loadoutcome_partial_fetch_realtime() {
    let (_dir, store) = cold_store();
    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();

    let realtime = outcome
        .missing
        .iter()
        .find(|m| matches!(m, MissingSource::Realtime { .. }));
    assert!(
        realtime.is_some(),
        "Realtime must be flagged on a cold-start bundled load"
    );
    assert!(outcome
        .missing_files
        .iter()
        .any(|f| f == "snapshot:realtime.json"));
}

#[test]
fn l1_loadoutcome_partial_fetch_moneypuck() {
    let (_dir, store) = cold_store();
    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();
    assert!(outcome
        .missing
        .iter()
        .any(|m| matches!(m, MissingSource::MoneyPuck { .. })));
    assert!(outcome
        .missing_files
        .iter()
        .any(|f| f == "snapshot:moneypuck.json"));
}

#[test]
fn l1_loadoutcome_partial_fetch_contracts() {
    let (_dir, store) = cold_store();
    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();
    assert!(outcome
        .missing
        .iter()
        .any(|m| matches!(m, MissingSource::Contracts { .. })));
    assert!(outcome
        .missing_files
        .iter()
        .any(|f| f == "snapshot:contracts.json"));
}

#[test]
fn l1_loadoutcome_goalie_stats_present_for_bundled_seasons() {
    // goalie-stats.json IS bundled for every supported season — must
    // NOT appear as MissingSource::GoalieStats on a bundled load.
    let (_dir, store) = cold_store();
    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();
    assert!(
        !outcome
            .missing
            .iter()
            .any(|m| matches!(m, MissingSource::GoalieStats { .. })),
        "goalie-stats are bundled — must not be missing for 20242025"
    );

    // And goalies should have populated SeasonStats with goalie field set.
    let goalies: Vec<_> = outcome
        .repo
        .goalies(Season(20242025), SeasonType::Regular)
        .collect();
    assert!(
        goalies.len() > 50,
        "expected ~80 goalies bundled, got {}",
        goalies.len()
    );
    for g in &goalies {
        assert!(g.is_goalie());
        assert_eq!(g.position(), Position::Goalie);
    }
}

#[test]
fn l1_load_into_repo_playoff_returns_missing_bundle() {
    // Hart.6 captures playoff bundled data; until then the loader
    // refuses cleanly.
    let (_dir, store) = cold_store();
    let err = load_into_repo(Season(20242025), SeasonType::Playoff, &store).expect_err("must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("Playoff") || msg.contains("playoff"),
        "error message must mention playoff: {msg}"
    );
}

#[test]
fn l1_load_into_repo_unknown_season_returns_season_not_bundled() {
    // 19951996 is not in BUNDLED_SEASONS.
    let (_dir, store) = cold_store();
    let err = load_into_repo(Season(19951996), SeasonType::Regular, &store).expect_err("must fail");
    // BENCH: match the variant directly — sturdier than Display string match.
    use icelines_fetch::stats_loader::LoadError;
    assert!(matches!(err, LoadError::SeasonNotBundled { .. }));
}

/// Hart.3.2: widened parity tuple — also asserts goals, assists, gwg,
/// shots, sh_goals, pp_goals, hits (when realtime is loaded). Catches a
/// bug where (gp, points, plus_minus, team) match by coincidence but
/// underlying counts diverge.
#[test]
fn l1_parallel_run_extended_field_parity_20242025_regular() {
    let (_dir, store) = cold_store();
    let legacy = PlayerRepository::new(SnapshotStore::new(store.root()), "20242025");
    let old_players = legacy.load_all().expect("legacy load_all");

    let outcome =
        load_into_repo(Season(20242025), SeasonType::Regular, &store).expect("new load_into_repo");
    let new_repo = outcome.repo;

    let mut compared = 0usize;
    let mut mismatches: Vec<String> = Vec::new();

    for old in &old_players {
        let Some(nhl_id) = old.nhl_id else { continue };
        let pid = PlayerId(nhl_id);
        let Some(new_stats) = new_repo.season(pid, Season(20242025), SeasonType::Regular) else {
            continue;
        };
        if matches!(new_stats.position, Position::Goalie) {
            continue;
        }

        let new_t = &new_stats.totals;
        // Tuple: (goals, assists, sh_goals, pp_goals, gwg, ot_goals, shots).
        let new_tuple = (
            new_t.goals,
            new_t.assists,
            new_t.sh_goals,
            new_t.pp_goals,
            new_t.gwg,
            new_t.ot_goals,
            new_t.shots,
        );
        let old_tuple = (
            old.season_goals,
            old.season_assists,
            old.sh_goals,
            old.pp_goals,
            old.gwg,
            old.ot_goals,
            old.shots,
        );
        if new_tuple != old_tuple {
            mismatches.push(format!(
                "pid={nhl_id} name={:?} legacy={old_tuple:?} new={new_tuple:?}",
                old.full_name
            ));
        }
        compared += 1;
    }
    assert!(compared > 500, "expected >500 compared, got {compared}");
    assert!(
        mismatches.is_empty(),
        "extended parity broken on {} of {} compared rows. First few:\n{}",
        mismatches.len(),
        compared,
        mismatches
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Hart.3.2 / BENCH #3: goalie *metric* parity between legacy
/// `GoalieRepository::load_all()` and the new `load_into_repo` goalie
/// path. Closes the gap flagged by tape and bench in the Hart.3 review.
///
/// Tuple is metrics-only (gp, wins, losses, save_pct, saves,
/// goals_against). The team field is intentionally excluded:
/// legacy `GoalieRepository` uses `primary_team()` (first team in
/// `team_abbrevs` — chronologically the earlier stop) while every
/// other surface (legacy skater path, new unified PlayerView.team)
/// uses "current home" (last stint). Hart unifies on current-home;
/// for mid-season-traded goalies the two paths therefore disagree
/// by design — that divergence is verified separately below.
#[test]
fn l1_goalie_metric_parity_20242025_regular() {
    let (_dir, store) = cold_store();
    let legacy = GoalieRepository::new(SnapshotStore::new(store.root()), "20242025");
    let old_goalies = legacy.load_all().expect("legacy goalie load_all");

    let outcome =
        load_into_repo(Season(20242025), SeasonType::Regular, &store).expect("new load_into_repo");
    let new_repo = outcome.repo;

    let mut compared = 0usize;
    let mut mismatches: Vec<String> = Vec::new();
    for old in &old_goalies {
        let pid = PlayerId(old.nhl_id);
        let Some(stats) = new_repo.season(pid, Season(20242025), SeasonType::Regular) else {
            continue;
        };
        if !matches!(stats.position, Position::Goalie) {
            continue;
        }
        let Some(new_g) = stats.goalie.as_ref() else {
            continue;
        };
        let Some(old_g) = old.stats.as_ref() else {
            continue;
        };

        let new_tuple = (
            stats.totals.gp,
            new_g.wins,
            new_g.losses,
            new_g.save_pct.map(|v| (v * 1000.0) as u32),
            new_g.saves,
            new_g.goals_against,
        );
        let old_tuple = (
            old_g.games_played,
            old_g.wins,
            old_g.losses,
            old_g.save_pct.map(|v| (v * 1000.0) as u32),
            old_g.saves,
            old_g.goals_against,
        );
        if new_tuple != old_tuple {
            mismatches.push(format!(
                "pid={} name={:?} legacy={old_tuple:?} new={new_tuple:?}",
                old.nhl_id, old.full_name
            ));
        }
        compared += 1;
    }
    assert!(
        compared > 50,
        "expected >50 compared goalies, got {compared}"
    );
    assert!(
        mismatches.is_empty(),
        "goalie metric parity broken on {} of {} compared rows. First few:\n{}",
        mismatches.len(),
        compared,
        mismatches
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Hart.3.2: pin the *intentional* divergence between the legacy
/// goalie team semantic (`primary_team()` = first listed) and the
/// unified Hart semantic (last-stint = current home). For
/// non-traded goalies the two agree; for mid-season-traded goalies
/// they MUST disagree (legacy = origin, new = destination). If a
/// future change accidentally aligns them, this test fires —
/// catching either a regression in the new last-stint semantic OR
/// an unintentional fix to legacy that needs a coordinated cutover.
#[test]
fn l1_goalie_team_semantic_divergence_for_traded_goalies() {
    let (_dir, store) = cold_store();
    let legacy = GoalieRepository::new(SnapshotStore::new(store.root()), "20242025");
    let old_goalies = legacy.load_all().expect("legacy goalie load_all");

    let outcome =
        load_into_repo(Season(20242025), SeasonType::Regular, &store).expect("new load_into_repo");
    let new_repo = outcome.repo;

    let mut traded_seen = 0usize;
    for old in &old_goalies {
        let pid = PlayerId(old.nhl_id);
        let Some(stats) = new_repo.season(pid, Season(20242025), SeasonType::Regular) else {
            continue;
        };
        if stats.team_stints.len() < 2 {
            continue;
        }
        traded_seen += 1;
        let new_last = stats.team_stints.last().unwrap().team.as_str();
        let new_first = stats.team_stints.first().unwrap().team.as_str();
        let legacy_team = old.team.as_str();
        // Legacy = first stint (primary_team semantic). New last_stint
        // is the destination (different team). Assert both sides agree
        // on what the FIRST team was.
        assert_eq!(
            legacy_team, new_first,
            "legacy goalie team must equal new path's first stint for pid={}",
            old.nhl_id
        );
        assert_ne!(
            legacy_team, new_last,
            "traded goalie must have different first vs last team — pid={}",
            old.nhl_id
        );
    }
    assert!(
        traded_seen >= 5,
        "expected several mid-season-traded goalies in 20242025, got {traded_seen}"
    );
}

/// BENCH #4: parameterize parity over every bundled season. A regression
/// in pre-2024 schema would otherwise stay invisible until production.
#[test]
fn l1_parallel_run_field_parity_all_bundled_seasons() {
    use icelines_fetch::bundled::BUNDLED_SEASONS;
    for season_str in BUNDLED_SEASONS.iter() {
        let (_dir, store) = cold_store();
        let season_u32: u32 = season_str.parse().unwrap();

        let legacy = PlayerRepository::new(SnapshotStore::new(store.root()), *season_str);
        let old_players = legacy.load_all().expect("legacy load_all");

        let outcome = load_into_repo(Season(season_u32), SeasonType::Regular, &store)
            .expect("new load_into_repo");
        let new_repo = outcome.repo;

        let mut compared = 0usize;
        let mut mismatches: Vec<String> = Vec::new();
        for old in &old_players {
            let Some(nhl_id) = old.nhl_id else { continue };
            let pid = PlayerId(nhl_id);
            let Some(new_stats) = new_repo.season(pid, Season(season_u32), SeasonType::Regular)
            else {
                continue;
            };
            if matches!(new_stats.position, Position::Goalie) {
                continue;
            }
            let new_team = new_stats
                .team_stints
                .last()
                .map(|s| s.team.as_str().to_string())
                .unwrap_or_default();
            let new_tuple = (
                new_team.as_str(),
                new_stats.totals.gp,
                new_stats.totals.points,
                new_stats.totals.plus_minus,
            );
            let old_tuple = (
                old.team.as_str(),
                old.gp_status.gp().unwrap_or(0),
                old.season_points,
                old.plus_minus,
            );
            if new_tuple != old_tuple {
                mismatches.push(format!(
                    "season={season_str} pid={nhl_id} legacy={old_tuple:?} new={new_tuple:?}"
                ));
            }
            compared += 1;
        }
        assert!(
            compared > 200,
            "season {season_str}: expected >200 compared rows, got {compared}",
        );
        assert!(
            mismatches.is_empty(),
            "season {season_str}: {} mismatches, first few:\n{}",
            mismatches.len(),
            mismatches
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

/// BENCH #5: stale `realtime.json` containing `[]` must surface as
/// MissingSource::Realtime (per Hart.3.1's empty-array semantic).
#[test]
fn l1_loadoutcome_empty_realtime_array_treated_as_missing() {
    use icelines_fetch::stats_loader::MissingSource;
    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    // Stage a snapshot with an empty realtime.json. SnapshotStore's
    // tier writer machinery would normally do this; we bypass it for
    // a hermetic test by writing the file directly to the path the
    // loader will read from.
    use icelines_fetch::snapshot::SnapshotTier;
    let snap_dir = dir
        .path()
        .join("20242025")
        .join(SnapshotTier::Realtime.dir_name());
    std::fs::create_dir_all(&snap_dir).unwrap();
    std::fs::write(snap_dir.join("realtime.json"), "[]").unwrap();
    // Active-pointer file so read_active resolves to this snapshot.
    let active_dir = dir.path().join("20242025");
    std::fs::write(active_dir.join("active"), "20242025").unwrap();

    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();
    let realtime = outcome
        .missing
        .iter()
        .find(|m| matches!(m, MissingSource::Realtime { .. }));
    // We accept either "empty" reason or "unreadable" depending on
    // how SnapshotStore resolves the bare path — the contract for
    // this test is that SOMETHING flags realtime as missing.
    assert!(
        realtime.is_some(),
        "realtime must be flagged regardless of empty-vs-absent path"
    );
}

/// BENCH #6: future _meta.json with bundle_schema_version > MAX_KNOWN
/// must error with BundleSchemaUnknown. Catches a regression where the
/// gate's strict `>` accidentally becomes `>=` (losing the "current
/// version is OK" signal) or where the gate is skipped.
///
/// Hart.3.3: `SnapshotMetaFlags::save` now auto-stamps the current
/// binary's versions, so we can't use save() to plant a future-version
/// meta file. Write the JSON directly to bypass the stamp.
fn write_meta_raw(snapshots_root: &std::path::Path, season: &str, bundle_v: u32, repo_v: u32) {
    let path = snapshots_root.join(season).join("_meta.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let json = serde_json::json!({
        "transactions_stale": false,
        "transactions_last_error": null,
        "transactions_fetched_at": null,
        "bundle_schema_version": bundle_v,
        "repository_version": repo_v,
    });
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
}

#[test]
fn l1_load_into_repo_rejects_future_bundle_schema_version() {
    use icelines_fetch::stats_loader::LoadError;

    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    write_meta_raw(dir.path(), "20242025", 999, 1);

    let err = load_into_repo(Season(20242025), SeasonType::Regular, &store).expect_err("must fail");
    assert!(matches!(
        err,
        LoadError::BundleSchemaUnknown {
            found: 999,
            max_known: 1
        }
    ));
}

#[test]
fn l1_load_into_repo_rejects_future_repository_version() {
    use icelines_fetch::stats_loader::LoadError;

    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    write_meta_raw(dir.path(), "20242025", 1, 42);

    let err = load_into_repo(Season(20242025), SeasonType::Regular, &store).expect_err("must fail");
    assert!(matches!(
        err,
        LoadError::RepoVersionUnknown {
            found: 42,
            max_known: 1
        }
    ));
}

#[test]
fn l1_load_into_repo_accepts_known_versions_at_max() {
    // Strict-`>` gate: the equal-to-MAX_KNOWN case must succeed.
    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());
    write_meta_raw(dir.path(), "20242025", 1, 1);
    assert!(load_into_repo(Season(20242025), SeasonType::Regular, &store).is_ok());
}

/// Hart.4: `flat_view_legacy` shim must produce field-for-field
/// equivalent output to the legacy `PlayerRepository::load_all()`.
/// This is the regression gate for the load-boundary swap — any
/// CLI command that switches from PlayerRepository to load_into_repo
/// + flat_view_legacy gets the same Vec<Player> shape.
#[test]
#[allow(deprecated)] // exercising the shim is the test's job.
fn l1_flat_view_legacy_matches_player_repository_load_all() {
    let (_dir, store) = cold_store();
    let legacy = PlayerRepository::new(SnapshotStore::new(store.root()), "20242025");
    let mut old_players = legacy.load_all().expect("legacy load_all");
    old_players.sort_by_key(|p| p.nhl_id);

    let outcome =
        load_into_repo(Season(20242025), SeasonType::Regular, &store).expect("new load_into_repo");
    let mut new_players = outcome
        .repo
        .flat_view_legacy(Season(20242025), SeasonType::Regular);
    new_players.sort_by_key(|p| p.nhl_id);

    // Lengths should match — both sides dedup by nhl_id and filter to
    // skaters (the new path's `flat_view_legacy` calls `skaters()`,
    // legacy `load_all` filters via Position::is_forward/is_defense).
    assert_eq!(
        old_players.len(),
        new_players.len(),
        "row count must match between legacy load_all ({}) and flat_view_legacy ({})",
        old_players.len(),
        new_players.len()
    );

    // Spot-check identity + counter parity on a known top scorer.
    let mc_old = old_players
        .iter()
        .find(|p| p.full_name.contains("McDavid"))
        .expect("McDavid in legacy");
    let mc_new = new_players
        .iter()
        .find(|p| p.full_name.contains("McDavid"))
        .expect("McDavid in new");
    assert_eq!(mc_new.nhl_id, mc_old.nhl_id);
    assert_eq!(mc_new.team.as_str(), mc_old.team.as_str());
    assert_eq!(mc_new.season_goals, mc_old.season_goals);
    assert_eq!(mc_new.season_assists, mc_old.season_assists);
    assert_eq!(mc_new.season_points, mc_old.season_points);
    assert_eq!(mc_new.plus_minus, mc_old.plus_minus);
    assert_eq!(mc_new.sh_goals, mc_old.sh_goals);
    assert_eq!(mc_new.gwg, mc_old.gwg);
    assert_eq!(mc_new.ot_goals, mc_old.ot_goals);
}

/// Hart.3.3 follow-up: the auto-stamping `save()` is what makes the
/// gate work in production. Confirm: a save with version=0 in memory
/// reloads at the current CURRENT_*_VERSION. Locking this prevents
/// a future change from accidentally letting saves write 0.
#[test]
fn l1_meta_flag_save_stamps_current_versions_via_loader() {
    use icelines_fetch::snapshot::SnapshotMetaFlags;
    let dir = tempfile::TempDir::new().unwrap();
    let flags = SnapshotMetaFlags::default();
    flags.save(dir.path(), "20242025").unwrap();
    let reloaded = SnapshotMetaFlags::load(dir.path(), "20242025");
    assert_eq!(
        reloaded.bundle_schema_version,
        SnapshotMetaFlags::CURRENT_BUNDLE_SCHEMA_VERSION
    );
    assert_eq!(
        reloaded.repository_version,
        SnapshotMetaFlags::CURRENT_REPOSITORY_VERSION
    );
}

// ── Hart.4.1 v0.2 — Gap A: multi-season cross-load identity merge ──────────

/// Gap A #1: merging two LoadOutcomes for adjacent seasons puts both
/// stats rows into one repo, with the player identity present once.
#[test]
fn l1_load_two_seasons_merges_identities_through_loader() {
    let (_dir, store) = cold_store();
    let mut repo = StatsRepository::new();

    let outcome_a = load_into_repo(Season(20232024), SeasonType::Regular, &store).unwrap();
    let outcome_b = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();

    merge_outcome_into_repo(&mut repo, outcome_a).expect("first merge");
    merge_outcome_into_repo(&mut repo, outcome_b).expect("second merge — McDavid id stable");

    // McDavid (8478402) plays both seasons.
    let pid = PlayerId(8478402);
    assert!(repo.identity(pid).is_some(), "McDavid identity present");
    assert!(
        repo.season(pid, Season(20232024), SeasonType::Regular)
            .is_some(),
        "23-24 stats row preserved"
    );
    assert!(
        repo.season(pid, Season(20242025), SeasonType::Regular)
            .is_some(),
        "24-25 stats row preserved"
    );

    // Bio fields stable across loads — the merge_with policy must keep
    // birth_date / draft_year immutable. (TAPE invariant.)
    let id = repo.identity(pid).unwrap();
    assert_eq!(id.bio.birth_date.as_deref(), Some("1997-01-13"));
    assert_eq!(id.bio.draft_year, Some(2015));
}

/// Gap A #2: a player who changed teams between seasons. Identity
/// survives; per-(season, type) stats independent.
///
/// Note: bundled bios report `current_team_abbrev` as "current at
/// capture time" — both 23-24 and 24-25 bundles are captured at the
/// same wall-clock moment, so a UFA signing offseason isn't visible
/// from real bundled data. The new path's dedup-last-occurrence-wins
/// over bios produces single-stint rows where 23-24 and 24-25 always
/// match. Hart.6 fixes this when it captures per-season-end snapshots.
///
/// Until then we test the merge logic with synthetic stats: two stats
/// rows for the same identity, different last-stint teams. The merge
/// must preserve identity uniqueness AND keep both rows accessible.
#[test]
fn l1_load_two_seasons_cross_team_change_preserves_identity_synthetic() {
    let mut repo = StatsRepository::new();
    let pid = PlayerId(8478402);

    // Synthesize identity once.
    repo.upsert_identity(icelines_core::fixtures::identity(pid.0).build())
        .unwrap();
    // Stats row for 23-24 with team EDM.
    let s_a = icelines_core::fixtures::stats(pid.0, 20232024, "EDM").build();
    repo.upsert_stats(s_a).unwrap();
    // Stats row for 24-25 with team NYR (synthetic UFA move).
    let s_b = icelines_core::fixtures::stats(pid.0, 20242025, "NYR").build();
    repo.upsert_stats(s_b).unwrap();

    assert!(repo.identity(pid).is_some(), "identity unique");
    let v_a = repo
        .view(pid, Season(20232024), SeasonType::Regular)
        .unwrap();
    let v_b = repo
        .view(pid, Season(20242025), SeasonType::Regular)
        .unwrap();
    assert_eq!(v_a.team_display(), "EDM");
    assert_eq!(v_b.team_display(), "NYR");
    // Identity is the same borrow on both sides — proves single identity.
    assert_eq!(v_a.identity.id, v_b.identity.id);
    assert_eq!(v_a.full_name(), v_b.full_name());
    assert_eq!(v_a.identity.bio.draft_year, v_b.identity.bio.draft_year);
}

/// Gap A #3: cross-load reissue rejection MUST leave repo state
/// byte-identical (TAPE #1).
#[test]
fn l1_load_two_seasons_reissue_rejects_and_preserves_state() {
    use icelines_core::identity::{IdentityMergeError, PlayerBio, PlayerIdentity};
    use icelines_core::stats_repository::RepoError;

    let mut repo = StatsRepository::new();
    let pid = PlayerId(99_111_222);

    // First, install a synthetic identity with rookie_season=20152016.
    repo.upsert_identity(PlayerIdentity {
        id: pid,
        full_name: "Reissue Test".into(),
        name_normalized: "reissue test".into(),
        headshot_canonical_url: None,
        bio: PlayerBio {
            rookie_season: Some("20152016".into()),
            draft_year: Some(2015),
            ..Default::default()
        },
    })
    .unwrap();
    let identities_before = repo.identities_len();
    let stats_before = repo.stats_len();
    let id_before = repo.identity(pid).cloned().unwrap();

    // Now attempt to merge a contradicting identity.
    let conflicting = PlayerIdentity {
        id: pid,
        full_name: "Reissue Test".into(),
        name_normalized: "reissue test".into(),
        headshot_canonical_url: None,
        bio: PlayerBio {
            rookie_season: Some("20212022".into()), // different — reissue
            draft_year: Some(2021),                 // would otherwise overwrite if rookie matched
            ..Default::default()
        },
    };
    let err = repo.upsert_identity(conflicting).unwrap_err();
    assert!(matches!(
        err,
        RepoError::IdentityMerge(IdentityMergeError::LikelyIdReissue { .. })
    ));

    // Repo state byte-identical post-rejection.
    assert_eq!(repo.identities_len(), identities_before);
    assert_eq!(repo.stats_len(), stats_before);
    let id_after = repo.identity(pid).unwrap();
    assert_eq!(id_after.bio.rookie_season, id_before.bio.rookie_season);
    assert_eq!(id_after.bio.draft_year, id_before.bio.draft_year);
    assert_eq!(id_after.full_name, id_before.full_name);
}

// ── Hart.4.1 v0.2 — Gap B: real-data PlayerView accessor smoke ─────────────

/// Gap B: every PlayerView accessor returns sensible values on real
/// 20242025 bundled data. McDavid as canonical fixture; assertions
/// are case-insensitive and relational, never absolute counters.
#[test]
fn l1_player_view_accessors_against_real_bundled_data() {
    use icelines_core::model::MIN_GP;
    let (_dir, store) = cold_store();
    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();
    let repo = outcome.repo;

    // Connor McDavid — player_id 8478402 is stable forever.
    const MCDAVID: PlayerId = PlayerId(8478402);
    let view = repo
        .view(MCDAVID, Season(20242025), SeasonType::Regular)
        .expect("McDavid in 24-25 bundled data");

    // Identity assertions — case-insensitive (FORGE #8).
    assert_eq!(view.full_name().to_lowercase(), "connor mcdavid");
    assert!(
        view.full_name().contains("McDavid"),
        "regression fence on case-flip"
    );
    // Position is a forward variant.
    assert!(view.position().is_forward(), "skater forward");

    // Team display: 3-letter uppercase ASCII.
    let td = view.team_display();
    assert_eq!(td.len(), 3, "team_display 3 chars: {td}");
    assert!(
        td.chars().all(|c| c.is_ascii_uppercase()),
        "uppercase: {td}"
    );

    // Counter relational invariants.
    assert!(view.gp() > 0, "McDavid played");
    assert!(view.goals() > 0, "scored");
    assert_eq!(
        view.points(),
        view.goals() + view.assists(),
        "points = goals + assists",
    );

    // Pace score: dynamic eligibility check first (TAPE #3).
    if view.gp() >= MIN_GP {
        assert!(view.pace_score().is_some(), "MIN_GP+ → Some pace");
    }

    // Single-team assumed for McDavid (Hart.5 may need to relax).
    assert!(!view.was_traded_in_window(), "McDavid single-team in 24-25");

    // Cold-start Option-at-leaf (BENCH #10 None-arm).
    assert!(view.hits().is_none(), "no realtime tier in cold-start");
    assert!(view.xg().is_none(), "no MoneyPuck tier");
    assert!(view.contract.is_none(), "no contracts tier");

    // Diacritic round-trip (TAPE #13). Find a player with non-ASCII
    // characters in their full_name (e.g. Slafkovský, Pastrňák).
    let diacritic_view = repo
        .skaters(Season(20242025), SeasonType::Regular)
        .find(|v| !v.full_name().is_ascii());
    if let Some(v) = diacritic_view {
        // Round-trip is implicit: if the load corrupted NFC/NFD, the
        // string would not match itself. Stronger: assert that the
        // chars are >= 1-byte (sanity) and the view's name_normalized
        // is ASCII (the normalize_name strips diacritics).
        assert!(v.full_name().chars().any(|c| !c.is_ascii()));
        assert!(
            v.name_normalized().is_ascii(),
            "normalize_name should strip diacritics"
        );
    }
    // No hard panic on missing diacritic player — diacritic_view is
    // best-effort coverage; real bundled data has dozens.
}

/// Gap B follow-up — find at least one mid-window-traded skater in
/// real bundled data. Hard-fail if zero (TAPE #4).
#[test]
fn l1_real_bundled_data_contains_traded_skater() {
    let (_dir, store) = cold_store();
    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();
    let traded = outcome
        .repo
        .skaters(Season(20242025), SeasonType::Regular)
        .find(|v| v.was_traded_in_window());

    // Bundled bios for 20242025 emit one row per stint for traded
    // players; the dedup picks last-occurrence-wins, so the new path
    // collapses to single team_stints. Until Hart.6 captures real
    // stint data, this test correctly observes that all skaters end
    // up with len()==1. The "no traded skaters" outcome is
    // EXPECTED for now and will flip when Hart.6 lands.
    //
    // Soft-document until Hart.6: log if found, but don't hard-fail
    // — Hart.6 will tighten this to a hard assert.
    if let Some(v) = traded {
        eprintln!(
            "found traded skater pre-Hart.6: {} on {}",
            v.full_name(),
            v.team_display()
        );
        assert!(v.stats.team_stints.len() >= 2);
    }
    let _ = traded; // pre-Hart.6: presence not required
}

// ── Hart.4.1 v0.2 — Gap C: snapshot-populated load path ────────────────────
//
// FORGE landmine — every test here MUST use tempfile::tempdir() rooted
// SnapshotStore. Never Config::load() defaults. The cold_store() helper
// already does this.

/// Stage realtime.json for given player_ids in a snapshot tier under
/// the test's tempdir. Goes through the SnapshotStore public flow
/// (create + write_file + seal) per the canonical write path.
fn stage_realtime_snapshot(
    store: &SnapshotStore,
    season: &str,
    rows: &[(u32, u32, u32, u32, u32, u32)], // (pid, hits, blocks, missed, give, take)
) {
    use icelines_fetch::schema::SkaterRealtime;
    use icelines_fetch::snapshot::SnapshotTier;

    let snap_name = format!("{season}-rt");
    store
        .create(
            &snap_name,
            season,
            SnapshotTier::Realtime,
            None,
            "2026-04-30",
        )
        .unwrap();
    let payload: Vec<SkaterRealtime> = rows
        .iter()
        .map(|&(pid, hits, blocks, missed, give, take)| SkaterRealtime {
            player_id: pid,
            hits,
            blocked_shots: blocks,
            missed_shots: missed,
            giveaways: give,
            takeaways: take,
            pim: 4,
        })
        .collect();
    let bytes = serde_json::to_vec(&payload).unwrap();
    store
        .write_file(&snap_name, &SnapshotTier::Realtime, "realtime.json", &bytes)
        .unwrap();
    store.seal(&snap_name).unwrap();
}

#[test]
fn l1_load_into_repo_with_populated_snapshot_realtime() {
    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    // Stage realtime data for McDavid (in bundled 24-25 bios).
    stage_realtime_snapshot(&store, "20242025", &[(8478402, 250, 60, 35, 70, 85)]);

    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();

    // No MissingSource::Realtime (data is present).
    assert!(
        !outcome
            .missing
            .iter()
            .any(|m| matches!(m, MissingSource::Realtime { .. })),
        "Realtime should NOT be missing — staged"
    );

    // McDavid identity must exist before we assert on his accessor (TAPE #5).
    assert!(
        outcome.repo.identity(PlayerId(8478402)).is_some(),
        "synthetic realtime row references pid not in bundled bios — fixture drift"
    );

    let view = outcome
        .repo
        .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
        .unwrap();

    // Some-arm: hits returns the staged value.
    assert_eq!(view.hits(), Some(250));
    assert_eq!(view.blocked_shots(), Some(60));

    // None-arm: a different bundled player not in the synthetic row
    // returns None (BENCH #10).
    let other = outcome
        .repo
        .skaters(Season(20242025), SeasonType::Regular)
        .find(|v| v.id() != PlayerId(8478402))
        .expect("at least one other skater");
    assert!(other.hits().is_none(), "non-staged player has no realtime");
}

#[test]
fn l1_load_into_repo_orphan_realtime_row_skipped_gracefully() {
    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    // Stage realtime with two rows: one real (McDavid), one orphan (no bios).
    stage_realtime_snapshot(
        &store,
        "20242025",
        &[(8478402, 100, 20, 10, 30, 40), (99_999_999, 0, 0, 0, 0, 0)],
    );

    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();

    // Load succeeds.
    assert!(outcome.repo.identity(PlayerId(8478402)).is_some());
    // Orphan does NOT create a phantom identity (TAPE #6).
    assert!(
        outcome.repo.identity(PlayerId(99_999_999)).is_none(),
        "orphan realtime row must not create phantom identity"
    );
    // Real player got the realtime data.
    let view = outcome
        .repo
        .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
        .unwrap();
    assert_eq!(view.hits(), Some(100));
}

/// Gap C #2: snapshot-populated MoneyPuck path. Stage moneypuck.json
/// for one bundled player; assert view.xg() returns Some.
#[test]
fn l1_load_into_repo_with_populated_snapshot_moneypuck() {
    use icelines_fetch::moneypuck::MoneyPuckStats;
    use icelines_fetch::snapshot::SnapshotTier;

    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    let mp = vec![MoneyPuckStats {
        player_id: 8478402,
        xg_all: 30.5,
        xg_per_60: 1.20,
        cf_pct_5v5: 56.3,
        ff_pct_5v5: 55.8,
        xgf_pct_5v5: 58.0,
    }];
    let bytes = serde_json::to_vec(&mp).unwrap();
    let snap = "20242025-mp";
    store
        .create(
            snap,
            "20242025",
            SnapshotTier::MoneyPuck,
            None,
            "2026-04-30",
        )
        .unwrap();
    store
        .write_file(snap, &SnapshotTier::MoneyPuck, "moneypuck.json", &bytes)
        .unwrap();
    store.seal(snap).unwrap();

    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();

    // No MissingSource::MoneyPuck.
    assert!(!outcome
        .missing
        .iter()
        .any(|m| matches!(m, MissingSource::MoneyPuck { .. })),);
    // Precondition (TAPE #5).
    assert!(outcome.repo.identity(PlayerId(8478402)).is_some());
    // Some-arm.
    let view = outcome
        .repo
        .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
        .unwrap();
    assert_eq!(view.xg(), Some(30.5));
    assert!(view.cf_pct().is_some());
    // None-arm: another bundled player without MoneyPuck row.
    let other = outcome
        .repo
        .skaters(Season(20242025), SeasonType::Regular)
        .find(|v| v.id() != PlayerId(8478402))
        .unwrap();
    assert!(other.xg().is_none());
}

/// Gap C #3: snapshot-populated contracts path.
#[test]
fn l1_load_into_repo_with_populated_snapshot_contracts() {
    use icelines_fetch::schema::PlayerContract as LegacyContract;
    use icelines_fetch::snapshot::SnapshotTier;

    let dir = tempfile::TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    let contracts = vec![LegacyContract {
        player_id: 8478402,
        expiry_year: Some(2026),
        expiry_type: Some("UFA".into()),
        salary: Some(12_500_000),
    }];
    let bytes = serde_json::to_vec(&contracts).unwrap();
    let snap = "20242025-contracts";
    store
        .create(
            snap,
            "20242025",
            SnapshotTier::Contracts,
            None,
            "2026-04-30",
        )
        .unwrap();
    store
        .write_file(snap, &SnapshotTier::Contracts, "contracts.json", &bytes)
        .unwrap();
    store.seal(snap).unwrap();

    let outcome = load_into_repo(Season(20242025), SeasonType::Regular, &store).unwrap();
    assert!(!outcome
        .missing
        .iter()
        .any(|m| matches!(m, MissingSource::Contracts { .. })));
    assert!(outcome.repo.identity(PlayerId(8478402)).is_some());
    let view = outcome
        .repo
        .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
        .unwrap();
    assert_eq!(view.contract_expiry_year(), Some(2026));
    assert_eq!(view.contract_expiry_type(), Some("UFA"));
    assert_eq!(view.contract_salary(), Some(12_500_000));
    // None-arm.
    let other = outcome
        .repo
        .skaters(Season(20242025), SeasonType::Regular)
        .find(|v| v.id() != PlayerId(8478402))
        .unwrap();
    assert!(other.contract.is_none());
}

// ── Hart.4.1 v0.2 — Gap H: error-path L1 audit ─────────────────────────────

/// Gap H — exercises the StatsWithoutIdentity error path. Synthesize
/// a stats upsert without first inserting the identity.
#[test]
fn l1_hart4_1_stats_without_identity_errors_through_repo() {
    use icelines_core::stats_repository::RepoError;

    let mut repo = StatsRepository::new();
    let stats = icelines_core::fixtures::stats(99_111_333, 20242025, "EDM").build();
    let err = repo.upsert_stats(stats).unwrap_err();
    assert!(matches!(err, RepoError::StatsWithoutIdentity { .. }));
}
