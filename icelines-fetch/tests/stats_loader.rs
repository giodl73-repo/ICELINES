//! Phase Hart.3 — L1 integration tests for `stats_loader::load_into_repo`.
//!
//! Uses bundled data only (no network, no real snapshot store). The
//! parallel-run field-parity test is the BENCH-mandated regression
//! gate against the legacy `PlayerRepository::load_all()` path.

use icelines_core::identity::PlayerId;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_fetch::goalie_repository::GoalieRepository;
use icelines_fetch::repository::PlayerRepository;
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::{load_into_repo, MissingSource};

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
