//! ESPN site.api transactions fetcher (Phase T.2).
//!
//! Hits `https://site.api.espn.com/apis/site/v2/sports/hockey/nhl/transactions`.
//! No auth. Pagination via `pageIndex` + `pageCount`. Rate-limit and
//! schema-drift handling per WIRE; callers surface partial/empty/HTML
//! conditions via [`FetchOutcome`].
//!
//! All retry sleeps are deterministic in tests via the
//! `with_sleep_provider` constructor — production builds use `tokio::time::sleep`,
//! tests inject a no-op sleep so `httpmock` exercises the retry chain
//! without wall-clock drift.

use std::sync::Arc;
use std::time::Duration;

use crate::error::FetchError;
use crate::schema::{RawTransaction, RawTransactionTeam};

use super::FetchOutcome;

/// Default ESPN base URL. Tests override via `EspnSource::with_base_url`.
pub const DEFAULT_ESPN_BASE: &str = "https://site.api.espn.com/apis/site/v2/sports/hockey/nhl";

const MAX_RETRIES: usize = 3;
const CIRCUIT_BREAK_AFTER_FAILS: usize = 3;
/// Server caps pageSize at 25 regardless of what we request. Send the
/// request anyway (informational; some clones may honor a higher limit).
#[allow(dead_code)]
const PAGE_LIMIT: usize = 200;

/// Convert an 8-digit NHL season ID (e.g. "20252026") into the ESPN
/// `?dates=YYYYMMDD-YYYYMMDD` range that covers the full hockey calendar
/// for that season — preseason camp through the Stanley Cup. Pre/post
/// boundary is forgiving (Sept 1 → July 31 next year) so cup-final rows
/// in late June/early July still land in the right season.
#[allow(dead_code)]
fn season_to_date_range(season: &str) -> Option<String> {
    if season.len() != 8 || !season.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let start_year: u32 = season[..4].parse().ok()?;
    let end_year: u32 = season[4..].parse().ok()?;
    if end_year != start_year + 1 {
        return None;
    }
    Some(format!("{start_year}0901-{end_year}0731"))
}

/// Split an NHL season into monthly date-range strings ESPN can accept.
/// Each string is `YYYYMMDD-YYYYMMDD` covering one calendar month.
/// 11 months total: Sept of the start year through July of the end year.
///
/// Workaround for ESPN's broken pageIndex pagination — see fetch_season.
pub(crate) fn season_month_windows(season: &str) -> Option<Vec<String>> {
    if season.len() != 8 || !season.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let start_year: u32 = season[..4].parse().ok()?;
    let end_year: u32 = season[4..].parse().ok()?;
    if end_year != start_year + 1 {
        return None;
    }

    // (year, month) tuples — 11 months: Sept(start) → July(end).
    let months = [
        (start_year, 9),
        (start_year, 10),
        (start_year, 11),
        (start_year, 12),
        (end_year, 1),
        (end_year, 2),
        (end_year, 3),
        (end_year, 4),
        (end_year, 5),
        (end_year, 6),
        (end_year, 7),
    ];
    let mut out = Vec::with_capacity(months.len());
    for (y, m) in months {
        let last = days_in_month(y, m);
        out.push(format!("{y}{m:02}01-{y}{m:02}{last:02}"));
    }
    Some(out)
}

/// Last day of a given month. Handles February in leap years.
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Configurable HTTP source. Concrete in v1 (no trait per FORGE).
pub struct EspnSource {
    client: reqwest::Client,
    base_url: String,
    sleep: Arc<dyn SleepFn>,
}

/// Indirection so tests can run the retry chain without real wall-clock
/// sleeps. Production uses [`TokioSleep`]; tests inject [`NoopSleep`].
pub trait SleepFn: Send + Sync {
    fn sleep<'a>(
        &'a self,
        dur: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>;
}

pub struct TokioSleep;
impl SleepFn for TokioSleep {
    fn sleep<'a>(
        &'a self,
        dur: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(tokio::time::sleep(dur))
    }
}

pub struct NoopSleep;
impl SleepFn for NoopSleep {
    fn sleep<'a>(
        &'a self,
        _dur: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
}

impl EspnSource {
    /// Production constructor — real ESPN base, real sleep.
    pub fn production() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("icelines-cli")
                .timeout(Duration::from_secs(20))
                .build()
                .expect("HTTP client must build"),
            base_url: DEFAULT_ESPN_BASE.to_owned(),
            sleep: Arc::new(TokioSleep),
        }
    }

    /// Test constructor — caller passes the httpmock URL and a no-op
    /// sleep so retries don't wall-clock the test runner.
    pub fn for_testing(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            sleep: Arc::new(NoopSleep),
        }
    }

    /// Fetch one season. Returns rich [`FetchOutcome`] so caller can
    /// distinguish clean / partial / drift conditions.
    ///
    /// Implementation note: ESPN's site.api does NOT accept `?season=`,
    /// AND its `pageIndex` parameter is broken (returns the same page
    /// regardless of value). We work around this by splitting the season
    /// into monthly date windows and concatenating with a high `limit`
    /// (1000) per window. Server returns the full window count for
    /// reasonable date ranges; we dedup by `(date, description)` to be
    /// safe across overlapping windows.
    pub async fn fetch_season(&self, season: &str) -> Result<FetchOutcome, FetchError> {
        let fetched_at = now_rfc3339();
        let windows = season_month_windows(season).ok_or_else(|| FetchError::SchemaChanged {
            detail: format!("invalid season ID '{season}' — expected 8 digits"),
        })?;
        let mut all_rows: Vec<RawTransaction> = Vec::new();
        let mut all_dropped: Vec<String> = Vec::new();
        let mut consecutive_failures: usize = 0;
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for window in windows {
            let url = format!("{}/transactions?dates={window}&limit=1000", self.base_url);

            let page = match self.fetch_page_with_retries(&url).await {
                Ok(p) => {
                    consecutive_failures = 0;
                    p
                }
                Err(e) => {
                    consecutive_failures += 1;
                    if consecutive_failures >= CIRCUIT_BREAK_AFTER_FAILS {
                        return Err(FetchError::CircuitBreakerTripped {
                            url: url.clone(),
                            after_failures: consecutive_failures,
                        });
                    }
                    return Err(e);
                }
            };

            let (rows, dropped) = parse_page_with_fallback(&page.body);
            for row in rows {
                let key = (row.date.clone(), row.description.clone());
                if seen.insert(key) {
                    all_rows.push(row);
                }
            }
            all_dropped.extend(dropped);
        }

        Ok(FetchOutcome {
            rows: all_rows,
            dropped_unknown_schema: all_dropped,
            partial: false,
            source_etag: None,
            fetched_at,
        })
    }

    /// One page. Retries 429/5xx with backoff; 200+HTML and 200+JSON
    /// are returned to the caller for parsing / detection.
    async fn fetch_page_with_retries(&self, url: &str) -> Result<RawPage, FetchError> {
        let mut attempt: usize = 0;
        loop {
            let result = self.fetch_once(url).await;
            match result {
                Ok(page) => return Ok(page),
                Err(FetchError::Http { status, .. })
                    if (status == 429 || (500..600).contains(&status)) && attempt < MAX_RETRIES =>
                {
                    let backoff = backoff_with_jitter(attempt);
                    self.sleep.sleep(backoff).await;
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Single HTTP call. Returns the body text on 200 (caller distinguishes
    /// HTML vs JSON), or a typed error on non-200.
    async fn fetch_once(&self, url: &str) -> Result<RawPage, FetchError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FetchError::Http {
                status: 0,
                url: format!("{url}: {e}"),
            })?;
        let status = resp.status().as_u16();

        if status != 200 {
            // Surface the raw HTTP code so the caller's retry policy can
            // pattern-match (we retry 429 + 5xx, fail-fast otherwise).
            return Err(FetchError::Http {
                status,
                url: url.to_owned(),
            });
        }

        // 200 — distinguish JSON from HTML by Content-Type before reading
        // the body, so a Cloudflare interstitial never feeds serde.
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        if !content_type.contains("application/json") && !content_type.is_empty() {
            // Empty content-type is rare but seen in mocks; we let it
            // through and let serde decide. A clear HTML or text/* type
            // is rejected here.
            if content_type.contains("text/html") || content_type.starts_with("text/") {
                return Err(FetchError::HtmlBodyResponse {
                    url: url.to_owned(),
                    content_type,
                });
            }
        }

        // Capture pageCount from JSON. If parsing fails outright, we
        // surface SchemaChanged so the run aborts — empty/truncated bodies
        // shouldn't silently zero out the season.
        let body: serde_json::Value = resp.json().await.map_err(|e| FetchError::SchemaChanged {
            detail: format!("ESPN body not parseable as JSON: {e}"),
        })?;
        let page_count = body
            .get("pageCount")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        Ok(RawPage { body, page_count })
    }
}

struct RawPage {
    body: serde_json::Value,
    #[allow(dead_code)]
    page_count: Option<usize>,
}

/// Parse an ESPN page body into rows + a list of dropped (unknown) field
/// paths. Single permissive walk over the JSON Value — explicit drift
/// accounting per row, never throws away the page on a new ESPN field.
pub(crate) fn parse_page_with_fallback(
    body: &serde_json::Value,
) -> (Vec<RawTransaction>, Vec<String>) {
    let mut rows = Vec::new();
    let mut dropped: Vec<String> = Vec::new();

    let Some(arr) = body.get("transactions").and_then(|v| v.as_array()) else {
        // Body has no `transactions` array at all — surface as a drift
        // marker rather than a hard error so caller sees the path was
        // exercised but yielded zero rows.
        dropped.push("(missing top-level 'transactions' array)".to_owned());
        return (rows, dropped);
    };

    for raw in arr {
        // Record any unexpected top-level fields on the row.
        for (k, _) in raw.as_object().into_iter().flat_map(|m| m.iter()) {
            if !matches!(k.as_str(), "date" | "description" | "team") {
                let path = format!("transactions[].{k}");
                if !dropped.contains(&path) {
                    dropped.push(path);
                }
            }
        }
        // Best-effort row extraction.
        let date = raw
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let description = raw
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let team = raw.get("team").and_then(|tv| {
            let id = tv.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let abbreviation = tv
                .get("abbreviation")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let display_name = tv.get("displayName").and_then(|v| v.as_str()).unwrap_or("");
            // Surface unknown team fields too.
            for (k, _) in tv.as_object().into_iter().flat_map(|m| m.iter()) {
                if !matches!(k.as_str(), "id" | "abbreviation" | "displayName") {
                    let path = format!("transactions[].team.{k}");
                    if !dropped.contains(&path) {
                        dropped.push(path);
                    }
                }
            }
            if id.is_empty() && abbreviation.is_empty() && display_name.is_empty() {
                None
            } else {
                Some(RawTransactionTeam {
                    id: id.to_owned(),
                    abbreviation: abbreviation.to_owned(),
                    display_name: display_name.to_owned(),
                })
            }
        });
        // Skip rows with neither date nor description — they're unrenderable.
        if date.is_empty() && description.is_empty() {
            continue;
        }
        rows.push(RawTransaction {
            date,
            description,
            team,
        });
    }
    (rows, dropped)
}

/// Exponential backoff with jitter. Bounds: `[base, 2*base]` for each
/// attempt. Bounded so `l1_mock_429_jitter_bounded` can assert it.
fn backoff_with_jitter(attempt: usize) -> Duration {
    // Deterministic component: 2^attempt * 100ms, capped at 4s.
    let base_ms = 100u64.saturating_mul(1u64 << attempt.min(5));
    let base_ms = base_ms.min(4_000);
    // Jitter: 0..base_ms additional, derived from a simple PRNG seeded
    // by the attempt + the system nanos so retries don't lockstep across
    // concurrent fetches.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let jitter = (nanos
        .wrapping_mul(2_654_435_761)
        .wrapping_add(attempt as u64))
        % base_ms.max(1);
    Duration::from_millis(base_ms + jitter)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_season_to_date_range_25_26() {
        assert_eq!(
            season_to_date_range("20252026"),
            Some("20250901-20260731".to_owned()),
        );
    }

    #[test]
    fn l0_season_to_date_range_24_25() {
        assert_eq!(
            season_to_date_range("20242025"),
            Some("20240901-20250731".to_owned()),
        );
    }

    #[test]
    fn l0_season_to_date_range_invalid() {
        assert_eq!(season_to_date_range("2025"), None);
        assert_eq!(
            season_to_date_range("20252027"),
            None,
            "non-consecutive years must reject"
        );
        assert_eq!(season_to_date_range("abcdefgh"), None);
        assert_eq!(season_to_date_range(""), None);
    }

    #[test]
    fn l0_backoff_bounded_to_double_base() {
        // Each attempt: base = 100 * 2^attempt (capped at 4s).
        // Jitter ∈ [0, base), so total ∈ [base, 2*base).
        for attempt in 0..4 {
            let base_ms = (100u64 * (1u64 << attempt)).min(4_000);
            let backoff = backoff_with_jitter(attempt);
            let ms = backoff.as_millis() as u64;
            assert!(
                ms >= base_ms && ms < base_ms * 2,
                "attempt {attempt}: backoff {ms}ms outside [{base_ms}, {})",
                base_ms * 2,
            );
        }
    }

    #[test]
    fn l0_parse_page_strict_path_no_dropped_fields() {
        let body = serde_json::json!({
            "transactions": [
                {
                    "date": "2026-04-29",
                    "description": "Acquired D X from NSH",
                    "team": {
                        "id": "14",
                        "abbreviation": "TBL",
                        "displayName": "Tampa Bay Lightning"
                    }
                }
            ],
            "pageCount": 1
        });
        let (rows, dropped) = parse_page_with_fallback(&body);
        assert_eq!(rows.len(), 1);
        assert!(dropped.is_empty(), "clean schema must have no drops");
        assert_eq!(rows[0].date, "2026-04-29");
        assert_eq!(rows[0].team.as_ref().unwrap().abbreviation, "TBL");
    }

    #[test]
    fn l0_parse_page_unknown_field_falls_back_records_drop() {
        let body = serde_json::json!({
            "transactions": [
                {
                    "date": "2026-04-29",
                    "description": "Acquired D X from NSH",
                    "newField": "espn added this",
                    "team": {
                        "id": "14",
                        "abbreviation": "TBL",
                        "displayName": "Tampa Bay Lightning",
                        "logos": []
                    }
                }
            ]
        });
        let (rows, dropped) = parse_page_with_fallback(&body);
        // Permissive path still extracts the row.
        assert_eq!(rows.len(), 1);
        // Drift fields surface at full path.
        assert!(
            dropped.iter().any(|d| d.contains("newField")),
            "expected 'newField' in dropped, got: {:?}",
            dropped
        );
        assert!(
            dropped.iter().any(|d| d.contains("team.logos")),
            "expected 'team.logos' in dropped, got: {:?}",
            dropped
        );
    }

    #[test]
    fn l0_parse_page_team_optional() {
        // League-wide row — no team payload.
        let body = serde_json::json!({
            "transactions": [
                { "date": "2026-04-27", "description": "League-wide reassignment deadline" }
            ]
        });
        let (rows, _) = parse_page_with_fallback(&body);
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].team.is_none(),
            "missing team field must produce team=None"
        );
    }

    #[test]
    fn l0_parse_page_missing_transactions_array_records_drift() {
        let body = serde_json::json!({ "wrongShape": [] });
        let (rows, dropped) = parse_page_with_fallback(&body);
        assert!(rows.is_empty());
        assert!(!dropped.is_empty(), "missing array must surface as drift");
    }

    #[test]
    fn l0_parse_page_skips_empty_rows() {
        // A row with no date AND no description is unrenderable; skip it.
        let body = serde_json::json!({
            "transactions": [
                { "date": "", "description": "" },
                { "date": "2026-04-29", "description": "Real row" }
            ]
        });
        let (rows, _) = parse_page_with_fallback(&body);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].description, "Real row");
    }
}
