//! L1 tests for the bundled transactions loader chain + classifier-version
//! re-classification on load + atomic-rename + .bak recovery semantics.
//! Phase T.3.

use icelines_core::{Transaction, TransactionKind};
use icelines_core::model::TeamAbbr;
use icelines_core::transactions::CURRENT_CLASSIFIER_VERSION;
use icelines_fetch::bundled::{
    get_transactions, load_transactions_with_fallback, TransactionsEnvelope,
};
use icelines_fetch::snapshot::{SnapshotMetaFlags, SnapshotStore};
use tempfile::TempDir;

fn fixture_envelope(version: u16, kind: TransactionKind, description: &str) -> TransactionsEnvelope {
    TransactionsEnvelope {
        season:             "20252026".to_owned(),
        source:             "espn".to_owned(),
        fetched_at:         "2026-04-30T14:00:00-04:00".to_owned(),
        classifier_version: version,
        rows: vec![Transaction {
            date:               "2026-04-29".to_owned(),
            team:               Some(TeamAbbr("EDM".to_owned())),
            kind,
            description:        description.to_owned(),
            id:                 "row-id".to_owned(),
            trade_group_id:     None,
            classifier_version: version,
        }],
    }
}

#[test]
fn l1_get_transactions_25_26_returns_some() {
    let envelope = get_transactions("20252026")
        .expect("25-26 transactions must be bundled");
    assert!(!envelope.rows.is_empty(), "bundled fixture must contain rows");
    assert_eq!(envelope.season, "20252026");
}

#[test]
fn l1_get_transactions_unbundled_season_returns_none() {
    assert!(get_transactions("19951996").is_none());
    assert!(get_transactions("20012002").is_none());
}

// ── Phase T.6: historical backfill regression tests ────────────────────────

#[test]
fn l1_bundled_transactions_present_for_every_covered_season() {
    // Every season at or above TRANSACTIONS_EARLIEST_SEASON must have a
    // non-empty bundled envelope. This catches a future PR that bumps
    // the earliest constant without re-running the probe.
    for season in &["20212022", "20222023", "20232024", "20242025", "20252026"] {
        let env = get_transactions(season)
            .unwrap_or_else(|| panic!("season {season} must be bundled"));
        assert!(
            !env.rows.is_empty(),
            "season {season} bundle is empty — regenerate with \
             `cargo run --example probe_espn_seasons -- --write-bundle`",
        );
        assert_eq!(env.season, *season);
        assert_eq!(env.source, "espn");
    }
}

#[test]
fn l1_bundled_other_rate_below_threshold_per_season() {
    // BENCH-mandated regression catcher: every bundled season's row
    // distribution must classify mostly to known kinds. ESPN archives
    // over-index on coaching/exec moves (~150–200 "Named X assistant
    // coach" rows per season) which are correctly Other; the threshold
    // accommodates that floor while still catching a regex regression.
    // If a real regression drops trades / signings into Other the rate
    // jumps well past 10%.
    for season in &["20212022", "20222023", "20232024", "20242025", "20252026"] {
        let env = get_transactions(season).expect("bundled");
        let total = env.rows.len() as f64;
        let other = env.rows.iter()
            .filter(|tx| tx.kind == icelines_core::TransactionKind::Other)
            .count() as f64;
        let rate = other / total;
        assert!(
            rate < 0.10,
            "season {season} other_rate is {:.1}% (>10% threshold) — \
             classifier regressed or ESPN prose drifted",
            rate * 100.0,
        );
    }
}

#[test]
fn l1_bundled_smoke_signings_present_per_season() {
    // Signings happen year-round and ESPN archives them comprehensively —
    // it's the only kind we can safely require for every bundled season.
    // Trades cluster around the trade deadline (so an in-progress season
    // captured post-deadline like 25-26 may have zero), and AHL kinds
    // (Recall / Reassignment / IR) come and go depending on which window
    // ESPN chose to retain. If a future regex change drops signings into
    // Other, this test fires.
    use icelines_core::TransactionKind;
    for season in &["20212022", "20222023", "20232024", "20242025", "20252026"] {
        let env = get_transactions(season).expect("bundled");
        let signings = env.rows.iter()
            .filter(|tx| tx.kind == TransactionKind::Signing)
            .count();
        assert!(
            signings > 0,
            "season {season} has zero Signing rows — classifier likely regressed",
        );
    }
}

#[test]
fn l1_bundled_minimum_row_count_per_season() {
    // Every bundled season carries ≥1000 rows. Anything less means
    // the capture truncated mid-page or the season window mis-mapped.
    for season in &["20212022", "20222023", "20232024", "20242025", "20252026"] {
        let env = get_transactions(season).expect("bundled");
        assert!(
            env.rows.len() >= 1000,
            "season {season} bundle is suspiciously small ({} rows) — \
             re-run the probe with `--write-bundle`",
            env.rows.len(),
        );
    }
}

#[test]
fn l1_bundled_team_abbrevs_all_canonical() {
    // No row should carry a non-canonical team abbrev — every Team(_)
    // value in the bundle must already be in ALL_NHL_TEAMS (or be None
    // for league-wide rows). Catches a future ESPN-side abbrev quirk
    // that escaped the mapper.
    use std::collections::HashSet;
    let canonical: HashSet<&str> = icelines_fetch::ALL_NHL_TEAMS.iter().copied().collect();
    for season in &["20212022", "20222023", "20232024", "20242025", "20252026"] {
        let env = get_transactions(season).expect("bundled");
        for tx in &env.rows {
            if let Some(team) = &tx.team {
                assert!(
                    canonical.contains(team.0.as_str()) || team.0 == "ARI" || team.0 == "ATL",
                    "season {season} has row with non-canonical team '{}': {:?}",
                    team.0, tx.description,
                );
            }
        }
    }
}

#[test]
fn l1_load_transactions_with_fallback_finds_bundled() {
    // No snapshot store has been populated → falls through to bundled.
    let dir = TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());
    let envelope = load_transactions_with_fallback("20252026", &store)
        .expect("bundled must satisfy load");
    assert!(!envelope.rows.is_empty());
}

#[test]
fn l1_load_transactions_unknown_season_errors_with_hint() {
    let dir = TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());
    let err = load_transactions_with_fallback("19951996", &store)
        .expect_err("pre-coverage season must error");
    let msg = format!("{err}");
    assert!(msg.contains("transactions"),
        "error must mention transactions, got: {msg}");
    assert!(msg.contains("19951996"),
        "error must name the season, got: {msg}");
}

#[test]
fn l1_classifier_version_stale_triggers_reclassify_on_load() {
    // Persist an envelope with classifier_version=0 and a deliberately
    // wrong kind. Loader must run the current classifier and fix it up.
    let dir = TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    // Stale envelope: classifier_version=0, kind=Other on what should be a Trade.
    let mut env = fixture_envelope(
        0,
        TransactionKind::Other,
        "Acquired D Ryan McDonagh from NSH for D Philippe Myers",
    );
    env.classifier_version = 0;

    // Drop into the snapshot store at a sealed snapshot.
    let snap = "test-stale-classifier";
    store.create(snap, "20252026", icelines_fetch::snapshot::SnapshotTier::Stats, None, "2026-04-30").unwrap();
    store.write_file(
        snap,
        &icelines_fetch::snapshot::SnapshotTier::Stats,
        "transactions.json",
        &serde_json::to_vec_pretty(&env).unwrap(),
    ).unwrap();
    store.seal(snap).unwrap();

    let loaded = load_transactions_with_fallback("20252026", &store)
        .expect("load must succeed");
    assert_eq!(loaded.classifier_version, CURRENT_CLASSIFIER_VERSION,
        "envelope-level version must be brought up to current");
    assert_eq!(loaded.rows[0].classifier_version, CURRENT_CLASSIFIER_VERSION);
    assert_eq!(loaded.rows[0].kind, TransactionKind::Trade,
        "stale Other should be re-classified to Trade by the current rules");
}

#[test]
fn l1_classifier_version_downgrade_ignored_on_load() {
    // Forward-compat: an envelope from a NEWER binary (classifier v=99)
    // must NOT be downgraded to v=current. Old binary just leaves the
    // newer classifications alone.
    let dir = TempDir::new().unwrap();
    let store = SnapshotStore::new(dir.path());

    let env = fixture_envelope(
        99,
        TransactionKind::Trade,
        "Future-version row written by a newer icelines binary",
    );
    let snap = "test-forward-compat";
    store.create(snap, "20252026", icelines_fetch::snapshot::SnapshotTier::Stats, None, "2026-04-30").unwrap();
    store.write_file(
        snap,
        &icelines_fetch::snapshot::SnapshotTier::Stats,
        "transactions.json",
        &serde_json::to_vec_pretty(&env).unwrap(),
    ).unwrap();
    store.seal(snap).unwrap();

    let loaded = load_transactions_with_fallback("20252026", &store).unwrap();
    assert_eq!(loaded.classifier_version, 99,
        "forward-compat: envelope version preserved when > CURRENT");
    assert_eq!(loaded.rows[0].classifier_version, 99,
        "forward-compat: row version preserved");
    assert_eq!(loaded.rows[0].kind, TransactionKind::Trade,
        "forward-compat: kind not overwritten");
}

// ── SnapshotMetaFlags ──────────────────────────────────────────────────

#[test]
fn l0_snapshot_meta_flags_default_all_false() {
    let flags = SnapshotMetaFlags::default();
    assert!(!flags.transactions_stale);
    assert!(flags.transactions_last_error.is_none());
    assert!(flags.transactions_fetched_at.is_none());
}

#[test]
fn l0_snapshot_meta_flags_serde_roundtrip() {
    let flags = SnapshotMetaFlags {
        transactions_stale:      true,
        transactions_last_error: Some("ESPN 503".to_owned()),
        transactions_fetched_at: Some("2026-04-30".to_owned()),
    };
    let json = serde_json::to_string(&flags).unwrap();
    let back: SnapshotMetaFlags = serde_json::from_str(&json).unwrap();
    assert_eq!(back.transactions_stale, true);
    assert_eq!(back.transactions_last_error.as_deref(), Some("ESPN 503"));
}

#[test]
fn l0_snapshot_meta_flags_missing_fields_default_for_forward_compat() {
    // An older binary's _meta.json that doesn't have any transactions
    // fields yet must parse cleanly via #[serde(default)].
    let json = r#"{}"#;
    let flags: SnapshotMetaFlags = serde_json::from_str(json).unwrap();
    assert!(!flags.transactions_stale);
}

#[test]
fn l1_meta_flag_load_and_save_roundtrip() {
    let dir = TempDir::new().unwrap();
    let snapshots_root = dir.path();

    // Initially missing → defaults.
    let flags0 = SnapshotMetaFlags::load(snapshots_root, "20252026");
    assert!(!flags0.transactions_stale);

    // Save a non-default state.
    let flags1 = SnapshotMetaFlags {
        transactions_stale:      true,
        transactions_last_error: Some("test error".to_owned()),
        transactions_fetched_at: Some("2026-04-30".to_owned()),
    };
    flags1.save(snapshots_root, "20252026").expect("save must succeed");

    // Reload — must round-trip.
    let flags2 = SnapshotMetaFlags::load(snapshots_root, "20252026");
    assert!(flags2.transactions_stale);
    assert_eq!(flags2.transactions_last_error.as_deref(), Some("test error"));
}

#[test]
fn l1_meta_flag_corrupt_primary_recovers_via_bak() {
    let dir = TempDir::new().unwrap();
    let snapshots_root = dir.path();

    // First save establishes primary.
    SnapshotMetaFlags {
        transactions_stale: false,
        transactions_last_error: None,
        transactions_fetched_at: Some("v1".to_owned()),
    }.save(snapshots_root, "20252026").unwrap();

    // Second save creates .bak of the v1 state.
    SnapshotMetaFlags {
        transactions_stale: true,
        transactions_last_error: Some("v2 problem".to_owned()),
        transactions_fetched_at: Some("v2".to_owned()),
    }.save(snapshots_root, "20252026").unwrap();

    // Corrupt the primary.
    let primary = snapshots_root.join("20252026").join("_meta.json");
    std::fs::write(&primary, "garbage").unwrap();

    // Loader falls through to .bak (the v1 state).
    let recovered = SnapshotMetaFlags::load(snapshots_root, "20252026");
    assert_eq!(recovered.transactions_fetched_at.as_deref(), Some("v1"),
        "corrupt primary must fall back to .bak (v1 content)");
}
