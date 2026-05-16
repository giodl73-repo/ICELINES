//! Phase Foster — `Freshness`, the per-manifest-entry recency record.
//!
//! Every dataset stored under `~/.icelines/data/` has one of these.
//! Carries enough information to (a) decide whether to re-fetch
//! ("is this stale?") and (b) explain to the user *why* the answer
//! came from where it did ("source: Bundle / Live / DataInstall").
//!
//! `Clock` is an injection point — production paths use
//! `SystemClock`; tests use `MockClock` so the same staleness
//! computation can be exercised deterministically. Foster.4's
//! background sync engine takes a `&dyn Clock` so a test can advance
//! time without sleeping.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Freshness {
    pub fetched_at: DateTime<Utc>,
    pub source: FetchSource,
    pub ttl: Ttl,
}

/// Where the dataset came from. Marked `#[non_exhaustive]` so future
/// sources (RSS, peer-shared bundles, etc.) can be added without
/// breaking downstream `match` blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FetchSource {
    /// Shipped in the binary (`bundled.rs` `include_bytes!`).
    Bundle,
    /// Pulled by the setup wizard on first run.
    Setup,
    /// Lazy-fetched on read miss.
    Live,
    /// Legacy data-install source. Kept for old manifests; new installs fetch
    /// source snapshots through `icelines fetch all --type both`.
    DataInstall,
    /// Explicit `icelines fetch X` invocation.
    Manual,
}

/// Time-to-live policy. `Static` means "never stale on its own — only
/// `--force` invalidates it". `After(d)` means stale once
/// `fetched_at + d < now()`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Ttl {
    Static,
    After(#[serde(with = "duration_secs")] Duration),
}

/// `Duration` as integer seconds — JSON-friendly, avoids float drift
/// on round-trip (serde's default `Duration` impl emits a tagged
/// struct that's clunky in manifest JSON).
mod duration_secs {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_u64(d.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Duration, D::Error> {
        u64::deserialize(de).map(Duration::from_secs)
    }
}

/// Time source. Production injects `SystemClock`; tests inject
/// `MockClock` so staleness math is deterministic. Foster.4's sync
/// engine takes `&dyn Clock`.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Test-only clock — set the wall time explicitly, advance with
/// `tick`. Lives in production code (not behind `#[cfg(test)]`) so
/// downstream test crates (icelines-fetch, icelines-cli) can drive
/// sync-engine tests against the same trait the production paths
/// use.
#[derive(Debug)]
pub struct MockClock {
    inner: std::sync::Mutex<DateTime<Utc>>,
}

impl MockClock {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            inner: std::sync::Mutex::new(start),
        }
    }

    pub fn tick(&self, by: Duration) {
        let mut t = self.inner.lock().expect("MockClock poisoned");
        *t += chrono::Duration::from_std(by).expect("tick out of range");
    }

    pub fn set(&self, when: DateTime<Utc>) {
        *self.inner.lock().expect("MockClock poisoned") = when;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        *self.inner.lock().expect("MockClock poisoned")
    }
}

impl Freshness {
    /// True if `now() > fetched_at + ttl`. `Ttl::Static` is never
    /// stale via TTL — only `--force` (out-of-band) invalidates.
    /// `DataInstall` source is also pinned regardless of `ttl` value
    /// (a user explicitly installed it; respect that until they
    /// `--force`).
    pub fn is_stale(&self, clock: &dyn Clock) -> bool {
        if matches!(self.source, FetchSource::DataInstall) {
            return false;
        }
        match self.ttl {
            Ttl::Static => false,
            Ttl::After(after) => {
                let deadline = self.fetched_at
                    + chrono::Duration::from_std(after).unwrap_or(chrono::Duration::zero());
                clock.now() > deadline
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn freshness(source: FetchSource, ttl: Ttl, fetched_at: DateTime<Utc>) -> Freshness {
        Freshness {
            fetched_at,
            source,
            ttl,
        }
    }

    fn t(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        chrono::TimeZone::with_ymd_and_hms(&Utc, year, month, day, hour, 0, 0).unwrap()
    }

    #[test]
    fn l0_foster02_fresh_within_ttl() {
        let f = freshness(
            FetchSource::Live,
            Ttl::After(Duration::from_secs(3600)),
            t(2026, 1, 15, 10),
        );
        let clock = MockClock::new(t(2026, 1, 15, 10) + chrono::Duration::minutes(30));
        assert!(!f.is_stale(&clock), "30 min < 1 h TTL");
    }

    #[test]
    fn l0_foster02_stale_past_ttl() {
        let f = freshness(
            FetchSource::Live,
            Ttl::After(Duration::from_secs(3600)),
            t(2026, 1, 15, 10),
        );
        let clock = MockClock::new(t(2026, 1, 15, 12)); // 2 h later
        assert!(f.is_stale(&clock), "2 h > 1 h TTL");
    }

    #[test]
    fn l0_foster02_static_never_stale() {
        let f = freshness(FetchSource::Bundle, Ttl::Static, t(1990, 1, 1, 0));
        // Decades later — still fresh.
        let clock = MockClock::new(t(2050, 1, 1, 0));
        assert!(!f.is_stale(&clock), "Ttl::Static is never stale");
    }

    #[test]
    fn l0_foster02_clock_skew_just_before_deadline() {
        // Off-by-one: if deadline is exactly equal to now, NOT stale yet.
        let f = freshness(
            FetchSource::Live,
            Ttl::After(Duration::from_secs(60)),
            t(2026, 1, 15, 10),
        );
        let exact_deadline = MockClock::new(t(2026, 1, 15, 10) + chrono::Duration::seconds(60));
        assert!(!f.is_stale(&exact_deadline), "exact-deadline still fresh");

        let one_sec_past = MockClock::new(t(2026, 1, 15, 10) + chrono::Duration::seconds(61));
        assert!(f.is_stale(&one_sec_past), "1s past deadline is stale");
    }

    #[test]
    fn l0_foster02_data_install_pinned_regardless_of_ttl() {
        // User explicitly ran `icelines data install`. Respect that:
        // even with a tight TTL, never auto-refresh.
        let f = freshness(
            FetchSource::DataInstall,
            Ttl::After(Duration::from_secs(60)),
            t(2026, 1, 15, 10),
        );
        let way_past = MockClock::new(t(2027, 1, 1, 0));
        assert!(!f.is_stale(&way_past), "DataInstall is pinned");
    }

    #[test]
    fn l0_foster02_mock_clock_tick() {
        let clock = MockClock::new(t(2026, 1, 15, 10));
        clock.tick(Duration::from_secs(3600));
        assert_eq!(clock.now(), t(2026, 1, 15, 11));
    }

    #[test]
    fn l0_foster02_serde_round_trip_freshness() {
        let f = freshness(
            FetchSource::Live,
            Ttl::After(Duration::from_secs(86400)),
            t(2026, 1, 15, 10),
        );
        let s = serde_json::to_string(&f).unwrap();
        // Ttl::After serializes as a tagged variant with seconds.
        assert!(
            s.contains("\"After\":86400"),
            "Ttl::After is integer seconds: {s}"
        );
        let back: Freshness = serde_json::from_str(&s).unwrap();
        assert_eq!(back.fetched_at, f.fetched_at);
        assert_eq!(back.source, f.source);
        match back.ttl {
            Ttl::After(d) => assert_eq!(d.as_secs(), 86400),
            Ttl::Static => panic!("Ttl::After round-tripped to Static"),
        }
    }

    #[test]
    fn l0_foster02_serde_static_round_trip() {
        let f = freshness(FetchSource::Bundle, Ttl::Static, t(1990, 1, 1, 0));
        let s = serde_json::to_string(&f).unwrap();
        let back: Freshness = serde_json::from_str(&s).unwrap();
        assert!(matches!(back.ttl, Ttl::Static));
    }
}
