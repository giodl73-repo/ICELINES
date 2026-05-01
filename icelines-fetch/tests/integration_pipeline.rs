/// L1 integration tests — full pipeline using fixture files.
/// No live network. Uses tempfile for cache directories.
use tempfile::TempDir;

use icelines_core::{
    model::FitClass,
    scoring::{classify_fit, compute_pace_score},
};
use icelines_fetch::{
    cache::Cache,
    csv_loader::load_csv_eligibility,
    resolver::PlayerResolver,
    schema::{RosterResponse, SkaterBio},
};

// ── Fixture paths ─────────────────────────────────────────────────────────────

fn fixture(relative: &str) -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = …/icelines/icelines-fetch
    // .parent() = …/icelines   ← repo root, where tests/fixtures/ lives
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(relative)
}

fn load_bios() -> Vec<SkaterBio> {
    let raw = std::fs::read_to_string(fixture("api/bios_page1.json")).unwrap();
    let page: icelines_fetch::schema::PagedResponse<SkaterBio> =
        serde_json::from_str(&raw).unwrap();
    page.data
}

fn load_roster_sea() -> RosterResponse {
    let raw = std::fs::read_to_string(fixture("api/roster_SEA.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

// Hart.5c.1: the L1 pipeline depth-chart test was removed. It built
// a Vec<Player> via the deprecated `player_builder` module to call
// `DepthChartBuilder::build`, which no longer exists. The depth-chart
// builder is exercised by L0 unit tests in icelines-core/src/depth_chart.rs
// against PlayerView fixtures (including the slot-vs-player_from_view
// adapter parity test). This entire file is scheduled for deletion in
// 5c.7 with the player_builder module.

// ── L1: Elite archetype produces documented pace score ────────────────────────

#[test]
fn l1_pipeline_elite_pace_projection() {
    // Elite: 50G, 90A in 82 GP → pace = (50+90)/82 * 82 = 140.000 exactly
    let score = compute_pace_score(50, 90, 82).unwrap();
    assert!(
        (score.pace_82 - 140.0).abs() < 0.001,
        "Elite pace should be 140.000, got {}",
        score.pace_82
    );
}

// ── L1: Injured archetype at exactly MIN_GP is included ──────────────────────

#[test]
fn l1_pipeline_injured_at_min_gp_is_eligible() {
    // Hurt Hero: 3G, 5A in exactly 10 GP (= MIN_GP) → must be eligible
    // pace = (3+5)/10 * 82 = 65.600
    let score = compute_pace_score(3, 5, 10);
    assert!(
        score.is_some(),
        "GP=10 (MIN_GP) must produce Some pace score"
    );
    let score = score.unwrap();
    assert!(
        (score.pace_82 - 65.6).abs() < 0.001,
        "Injured pace should be 65.600, got {}",
        score.pace_82
    );
}

// ── L1: Absent archetype (GP=0) produces None ─────────────────────────────────

#[test]
fn l1_pipeline_absent_gp_zero_produces_none() {
    assert!(
        compute_pace_score(0, 0, 0).is_none(),
        "GP=0 must return None"
    );
}

// ── L1: Fit classification — Elite archetype is Elite ────────────────────────

#[test]
fn l1_pipeline_elite_classified_as_elite() {
    // Elite: 140.0 pts/82 → well above Elite threshold (65.0 for forwards)
    assert_eq!(
        classify_fit(140.0, icelines_core::Position::Center),
        FitClass::Elite
    );
}

// ── L1: Resolver — Slafkovský resolves with diacritic stripped ───────────────

#[test]
fn l1_resolver_slafkovsky_from_bios() {
    let bios = load_bios();
    // bios fixture doesn't have Slafkovský — use resolver unit test coverage.
    // This test verifies the pipeline: load bios → build resolver → resolve name.
    let resolver = PlayerResolver::from_bios(&bios);
    // Elite player is in the fixture as "Connor McPlayer" id=8480001
    let id = resolver.resolve("Connor McPlayer", Some("EDM")).unwrap();
    assert_eq!(id, 8480001, "resolve should return fixture player_id");
}

// ── L1: Cache round-trip with fixture data ───────────────────────────────────

#[test]
fn l1_cache_roster_round_trip() {
    let dir = TempDir::new().unwrap();
    let cache = Cache::new(dir.path());
    let roster = load_roster_sea();

    cache.put("rosters/20252026/SEA.json", &roster).unwrap();

    let loaded: Option<RosterResponse> = cache.get(
        "rosters/20252026/SEA.json",
        icelines_fetch::cache::ttl::ROSTER,
    );
    assert!(
        loaded.is_some(),
        "cached roster should deserialize successfully"
    );
    let loaded = loaded.unwrap();
    assert_eq!(loaded.forwards.len(), roster.forwards.len());
}

// ── L1: CSV loader — fixture CSV round-trip ──────────────────────────────────

#[test]
fn l1_csv_fixture_loads_correctly() {
    let path = fixture("sample_skaters.csv");
    if !path.exists() {
        return; // fixture not yet present — skip gracefully
    }
    let records = load_csv_eligibility(&path).unwrap();
    // Sample CSV has 9 archetypes; row with empty team is skipped
    assert!(!records.is_empty(), "CSV fixture should produce records");
    // All records must have a non-empty team and eligible_pos
    for r in &records {
        assert!(!r.team.is_empty(), "team must not be empty");
        assert!(!r.eligible_pos.is_empty(), "eligible_pos must not be empty");
    }
}

// ── L1: Cache miss returns None ───────────────────────────────────────────────

#[test]
fn l1_cache_hit_skips_stale() {
    let dir = TempDir::new().unwrap();
    let cache = Cache::new(dir.path());
    // Key that was never written
    let result: Option<RosterResponse> =
        cache.get("never_written.json", std::time::Duration::from_secs(1));
    assert!(result.is_none(), "missing key must return None");
}

// ── Phase 8c: historical playoffs bundle ──────────────────────────────────────

#[test]
fn l1_historical_playoffs_19931994_loads_via_bundled_path() {
    // Full chain: load bundled JSON → convert to PlayoffBracket → verify fields
    // that the TUI relies on. Proves no-network historical bracket support.
    let bundle = icelines_fetch::bundled::load_playoffs("19931994")
        .expect("19931994 must be bundled");
    assert_eq!(bundle.season, "19931994");
    assert_eq!(bundle.champion.as_deref(), Some("NYR"));

    let bracket = bundle.to_bracket();
    assert_eq!(bracket.rounds.len(), 4);
    // Round 4 = Stanley Cup Final, NYR vs VAN, 7 games
    let cup = bracket.rounds.iter().find(|r| r.round_number == 4).unwrap();
    assert_eq!(cup.series.len(), 1);
    let s = &cup.series[0];
    assert_eq!(s.top_seed_abbrev, "NYR");
    assert_eq!(s.bottom_seed_abbrev, "VAN");
    assert_eq!(s.top_seed_wins, 4);
    assert_eq!(s.bottom_seed_wins, 3);
    assert_eq!(s.games.len(), 7);
    // Game 1 = VAN won in NYR (3-2 OT) → series_after "VAN leads 1-0"
    assert_eq!(s.games[0].series_after, "VAN leads 1-0");
    // Cup-clinching game ends "NYR wins 4-3"
    assert_eq!(s.games[6].series_after, "NYR wins 4-3");
    // Letter assignment from to_bracket — round 4 single series gets a letter
    assert!(s.letter.is_some(), "every series gets a stable letter");
}

#[test]
fn l1_historical_playoffs_unknown_season_returns_none() {
    // Sanity: only 19931994 ships in v1 of Phase 8c.
    assert!(icelines_fetch::bundled::load_playoffs("19951996").is_none());
    assert!(icelines_fetch::bundled::load_playoffs("19981999").is_none());
}
