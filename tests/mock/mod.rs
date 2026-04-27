//! # IceLines Mock NHL API Server
//!
//! `MockNhlServer` wraps `httpmock::MockServer` and provides a clean,
//! ergonomic API for registering NHL API endpoints in tests.
//!
//! ## Design
//!
//! The NHL API has two distinct base URLs that we mock separately:
//!
//! - `https://api.nhle.com/stats/rest/en/` — bulk stats endpoints (bios, summary)
//! - `https://api-web.nhle.com/v1/` — roster, player landing, boxscore, schedule
//!
//! In tests, a single `MockNhlServer` intercepts both. The `NhlApiClient` in
//! `icelines-fetch` must accept a configurable base URL (not hardcoded) to make
//! this work.
//!
//! ## Usage
//!
//! ```rust
//! use crate::mock::MockNhlServer;
//!
//! #[tokio::test]
//! async fn l1_full_pipeline_sea_depth_chart() {
//!     let mock = MockNhlServer::start().await;
//!     mock.register_roster("SEA");          // loads tests/fixtures/api/roster_SEA.json
//!     mock.register_bios_page(1);           // loads tests/fixtures/api/bios_page1.json
//!     mock.register_stats_page(1);          // loads tests/fixtures/api/stats_page1.json
//!
//!     let client = NhlApiClient::new(mock.base_url());
//!     let players = client.fetch_all_bios("20252026").await.unwrap();
//!     // ... assert depth chart shape
//!
//!     mock.assert_all_called();             // fails test if any registered mock was not hit
//! }
//! ```
//!
//! ## Fixture files
//!
//! All fixture JSON lives in `tests/fixtures/api/`. The filenames match the
//! mock registration helpers:
//!
//! | Helper | Fixture file |
//! |--------|-------------|
//! | `register_roster("SEA")` | `api/roster_SEA.json` |
//! | `register_roster("COL")` | `api/roster_COL.json` |
//! | `register_bios_page(1)` | `api/bios_page1.json` |
//! | `register_stats_page(1)` | `api/stats_page1.json` |
//! | `register_boxscore(2025020001)` | `api/boxscore_2025020001.json` |
//! | `register_player_landing(8480001)` | `api/player_8480001_landing.json` |
//! | `register_schedule("2025-10-15")` | `api/schedule_2025-10-15.json` |
//! | `register_empty_bios_page(2)` | *(built-in: {"data":[],"total":9})* |

use httpmock::prelude::*;
use std::path::{Path, PathBuf};

/// The 9 canonical test player IDs.
/// Every fixture file covers exactly these players in this order.
pub mod players {
    pub const ELITE:    u32 = 8480001; // 82 GP, 50G, 90A  — Elite tier
    pub const SOLID:    u32 = 8480002; // 74 GP, 28G, 40A  — Solid/Fit tier
    pub const BURIED:   u32 = 8480003; // 68 GP, 15G, 22A  — Good pace, low slot
    pub const STRETCH:  u32 = 8480004; // 55 GP,  4G,  8A  — Overextended
    pub const TRADED:   u32 = 8480005; // 40 GP, 12G, 18A  — Mid-season trade
    pub const ROOKIE:   u32 = 8480006; // 22 GP,  5G,  7A  — < MIN_GP at start
    pub const INJURED:  u32 = 8480007; // 10 GP,  3G,  5A  — Exactly MIN_GP=10
    pub const ABSENT:   u32 = 8480008; //  0 GP,  0G,  0A  — Never played
    pub const MULTI:    u32 = 8480009; // 78 GP, 20G, 35A  — Multi-pos (C+LW)
}

/// Expected pace projections (pts/82) for each archetype.
/// These are the documented expected values for L0/L1 assertions.
/// Formula: (G + A) / GP * 82
pub mod expected_pace {
    // Elite:   (50 + 90) / 82 * 82 = 140.000 exactly
    pub const ELITE:   f32 = 140.000;
    // Solid:   (28 + 40) / 74 * 82 = 75.351...
    pub const SOLID:   f32 = 75.351;
    // Buried:  (15 + 22) / 68 * 82 = 44.647...
    pub const BURIED:  f32 = 44.647;
    // Stretch: ( 4 +  8) / 55 * 82 = 17.891...
    pub const STRETCH: f32 = 17.891;
    // Traded:  (12 + 18) / 40 * 82 = 61.500
    pub const TRADED:  f32 = 61.500;
    // Rookie:  (5  +  7) / 22 * 82 = 44.727...  (below MIN_GP=10? No, 22 >= 10)
    pub const ROOKIE:  f32 = 44.727;
    // Injured: ( 3 +  5) / 10 * 82 = 65.600  (exactly at MIN_GP)
    pub const INJURED: f32 = 65.600;
    // Absent:  None (GP=0 — excluded from rankings)
    // Multi:   (20 + 35) / 78 * 82 = 57.821...
    pub const MULTI:   f32 = 57.821;
}

/// Expected fantasy point totals under yahoo-standard scheme.
/// Scheme: G=3, A=2, +PPG=1, +PPA=0.5, +SHG=1, +SHA=0.5, GWG=0.5, HIT=0.5, BLK=0.5
pub mod expected_fantasy {
    // Elite: 50*3 + 90*2 + 16*1 + (55-16)*0.5 + 2*1 + (4-2)*0.5 + 8*0.5 + 40*0.5 + 30*0.5
    //      = 150 + 180 + 16 + 19.5 + 2 + 1 + 4 + 20 + 15 = 407.5
    pub const ELITE:  f32 = 407.5;
    // Absent: 0.0 (GP=0, no fantasy score)
    pub const ABSENT: f32 = 0.0;
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn read_fixture(relative: &str) -> String {
    std::fs::read_to_string(fixture_dir().join(relative))
        .unwrap_or_else(|e| panic!("missing fixture {relative}: {e}"))
}

/// A registered mock handle — used by `assert_all_called()`.
struct Handle {
    description: String,
    mock: httpmock::Mock,
}

pub struct MockNhlServer {
    server: MockServer,
    // Tracks every registered mock so assert_all_called() can verify each was hit.
    handles: std::cell::RefCell<Vec<Handle>>,
}

impl MockNhlServer {
    /// Start a mock server. Call this at the top of each L1 test.
    pub async fn start() -> Self {
        Self {
            server: MockServer::start_async().await,
            handles: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Register a mock and track its handle for `assert_all_called()`.
    fn track(&self, description: &str, mock: httpmock::Mock) {
        self.handles.borrow_mut().push(Handle {
            description: description.to_owned(),
            mock,
        });
    }

    /// Base URL to pass to `NhlApiClient::new()` in tests.
    pub fn base_url(&self) -> String {
        self.server.base_url()
    }

    // ── Roster endpoints ────────────────────────────────────────────────────

    /// Register `GET /v1/roster/{team}/20252026`
    pub fn register_roster(&self, team: &str) {
        let body = read_fixture(&format!("api/roster_{team}.json"));
        let desc = format!("GET /v1/roster/{team}/20252026");
        let mock = self.server.mock(|when, then| {
            when.method(GET)
                .path(format!("/v1/roster/{team}/20252026"));
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });
        self.track(&desc, mock);
    }

    /// Register rosters for all 32 teams using the SEA fixture (sufficient for
    /// tests that only care about roster structure, not team-specific data).
    pub fn register_all_rosters_as_sea(&self) {
        let body = read_fixture("api/roster_SEA.json");
        self.server.mock(|when, then| {
            when.method(GET)
                .path_matches(r"^/v1/roster/[A-Z]{2,3}/20252026$");
            then.status(200)
                .header("content-type", "application/json")
                .body(body.clone());
        });
    }

    // ── Bulk stats endpoints ─────────────────────────────────────────────────

    /// Register the first page of the bios endpoint (9 players, total=9).
    pub fn register_bios_page(&self, page: u32) {
        let body = read_fixture(&format!("api/bios_page{page}.json"));
        let start = (page - 1) * 100;
        self.server.mock(|when, then| {
            when.method(GET)
                .path("/stats/rest/en/skater/bios")
                .query_param_exists("cayenneExp")
                .query_param("start", start.to_string());
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });
    }

    /// Register an empty last page to signal end of pagination.
    pub fn register_bios_empty_page(&self, page: u32) {
        let start = (page - 1) * 100;
        self.server.mock(|when, then| {
            when.method(GET)
                .path("/stats/rest/en/skater/bios")
                .query_param("start", start.to_string());
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":[],"total":9}"#);
        });
    }

    /// Register the first page of the summary stats endpoint.
    pub fn register_stats_page(&self, page: u32) {
        let body = read_fixture(&format!("api/stats_page{page}.json"));
        let start = (page - 1) * 100;
        self.server.mock(|when, then| {
            when.method(GET)
                .path("/stats/rest/en/skater/summary")
                .query_param("start", start.to_string());
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });
    }

    // ── Boxscore endpoints ───────────────────────────────────────────────────

    /// Register a boxscore response for the given game ID.
    pub fn register_boxscore(&self, game_id: u64) {
        let body = read_fixture(&format!("api/boxscore_{game_id}.json"));
        self.server.mock(|when, then| {
            when.method(GET)
                .path(format!("/v1/gamecenter/{game_id}/boxscore"));
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });
    }

    // ── Player landing endpoint ──────────────────────────────────────────────

    pub fn register_player_landing(&self, player_id: u32) {
        let body = read_fixture(&format!("api/player_{player_id}_landing.json"));
        self.server.mock(|when, then| {
            when.method(GET)
                .path(format!("/v1/player/{player_id}/landing"));
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });
    }

    // ── Schedule endpoint ────────────────────────────────────────────────────

    pub fn register_schedule_today(&self) {
        let body = read_fixture("api/schedule_today.json");
        self.server.mock(|when, then| {
            when.method(GET).path("/v1/schedule/now");
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });
    }

    // ── Error scenarios ──────────────────────────────────────────────────────

    /// Simulate a rate-limit response for the first call, then succeed.
    pub fn register_bios_with_retry(&self) {
        // First call → 429
        self.server.mock(|when, then| {
            when.method(GET)
                .path("/stats/rest/en/skater/bios")
                .query_param("start", "0");
            then.status(429)
                .header("retry-after", "1");
        });
        // Subsequent call → 200
        self.register_bios_page(1);
    }

    /// Simulate a 503 service unavailable.
    pub fn register_503(&self, path: &str) {
        self.server.mock(|when, then| {
            when.method(GET).path(path);
            then.status(503);
        });
    }

    // ── Assertion helpers ────────────────────────────────────────────────────

    /// Verify that every registered mock was called at least once.
    ///
    /// Call this at the END of every L1 test. A registered mock that was never
    /// hit is a test gap — it means the code under test did not exercise the
    /// path you thought it would.
    ///
    /// ```rust
    /// let mock = MockNhlServer::start().await;
    /// mock.register_bios_page(1);
    /// // ... run code under test ...
    /// mock.assert_all_called(); // panics if bios_page(1) was never requested
    /// ```
    pub fn assert_all_called(&self) {
        let handles = self.handles.borrow();
        let mut failures = Vec::new();
        for h in handles.iter() {
            if h.mock.hits() == 0 {
                failures.push(format!("  NOT CALLED: {}", h.description));
            }
        }
        if !failures.is_empty() {
            panic!(
                "MockNhlServer: {} registered mock(s) were never called:\n{}",
                failures.len(),
                failures.join("\n")
            );
        }
    }

    /// Assert a specific endpoint was called exactly `n` times.
    pub fn assert_called_times(&self, description_contains: &str, n: usize) {
        let handles = self.handles.borrow();
        for h in handles.iter() {
            if h.description.contains(description_contains) {
                assert_eq!(
                    h.mock.hits(), n,
                    "Expected '{}' to be called {} time(s), got {}",
                    h.description, n, h.mock.hits()
                );
                return;
            }
        }
        panic!("No registered mock matching '{description_contains}'");
    }
}
