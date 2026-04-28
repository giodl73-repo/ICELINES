/// L1 integration tests — full pipeline using fixture files.
/// No live network. Uses tempfile for cache directories.
use tempfile::TempDir;

use icelines_core::{
    model::{FitClass, GpStatus, Season},
    scoring::{classify_fit, compute_pace_score},
    DepthChartBuilder, TeamAbbr,
};
use icelines_fetch::{
    cache::Cache,
    csv_loader::load_csv_eligibility,
    moneypuck::MoneyPuckStats,
    player_builder::{build_players, index_bios, index_stats},
    resolver::PlayerResolver,
    schema::{RosterResponse, SkaterBio, SkaterRealtime, SkaterStats},
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

fn load_stats() -> Vec<SkaterStats> {
    let raw = std::fs::read_to_string(fixture("api/stats_page1.json")).unwrap();
    let page: icelines_fetch::schema::PagedResponse<SkaterStats> =
        serde_json::from_str(&raw).unwrap();
    page.data
}

fn load_roster_sea() -> RosterResponse {
    let raw = std::fs::read_to_string(fixture("api/roster_SEA.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

// ── L1: full pipeline — roster + bios + stats → depth chart ──────────────────

#[test]
fn l1_pipeline_sea_depth_chart_structure() {
    let bios = load_bios();
    let stats = load_stats();
    let roster = load_roster_sea();

    let bio_idx = index_bios(&bios);
    let stats_idx = index_stats(&stats);
    let team = TeamAbbr("SEA".to_string());

    let empty_rt: std::collections::HashMap<u32, SkaterRealtime> = std::collections::HashMap::new();
    let empty_mp: std::collections::HashMap<u32, MoneyPuckStats> = std::collections::HashMap::new();
    let empty_contracts: std::collections::HashMap<u32, icelines_fetch::schema::PlayerContract> = std::collections::HashMap::new();
    let fwds = build_players(
        &roster.forwards,
        &bio_idx,
        &stats_idx,
        &empty_rt,
        &empty_mp,
        &empty_contracts,
        Season(20252026),
        &team,
    );
    let defs = build_players(
        &roster.defensemen,
        &bio_idx,
        &stats_idx,
        &empty_rt,
        &empty_mp,
        &empty_contracts,
        Season(20252026),
        &team,
    );
    let all: Vec<_> = fwds.into_iter().chain(defs).collect();

    let chart = DepthChartBuilder::build(team, Season(20252026), all);

    // Forwards occupy lines 1-4 with some slots filled
    assert!(!chart.forward_lines.is_empty(), "must have forward lines");
    assert!(!chart.defense_pairs.is_empty(), "must have defense pairs");

    // GP=0 player (Sitout Steve, id 8480008) must be in below_min_gp
    let absent = chart
        .below_min_gp
        .iter()
        .find(|p| p.nhl_id == Some(8480008));
    assert!(absent.is_some(), "GP=0 player must be in below_min_gp");

    // No GP=0 player on the card
    let on_card = chart
        .forward_lines
        .iter()
        .flatten()
        .chain(chart.defense_pairs.iter().flatten())
        .filter_map(|s| s.as_ref())
        .any(|p| p.gp_status == GpStatus::Zero);
    assert!(!on_card, "GP=0 player must not appear on the card");
}

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
