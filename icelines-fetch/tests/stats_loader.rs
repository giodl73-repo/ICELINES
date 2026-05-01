//! Phase Hart.3 — L1 integration tests for `stats_loader::load_into_repo`.
//!
//! Uses bundled data only (no network, no real snapshot store). The
//! parallel-run field-parity test is the BENCH-mandated regression
//! gate against the legacy `PlayerRepository::load_all()` path.

use icelines_core::identity::PlayerId;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
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
    let msg = err.to_string();
    assert!(
        msg.contains("19951996") || msg.contains("not bundled"),
        "error message must mention the season or 'not bundled': {msg}"
    );
}
