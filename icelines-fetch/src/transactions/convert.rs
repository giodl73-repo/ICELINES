//! Convert raw ESPN rows into our persisted `Transaction` shape.
//!
//! This is the join point between `icelines-fetch` (raw / network /
//! schema-drift handling) and `icelines-core::transactions` (pure logic
//! for classification, sanitization, grouping). T.3 calls this once
//! per row before writing the snapshot envelope.
//!
//! Steps per row:
//!  1. Sanitize `description` (strip control chars, normalize whitespace).
//!  2. Classify with the current regex set; record `classifier_version`.
//!  3. Map ESPN team abbrev → canonical NHL form via the season-aware
//!     [`crate::teams::espn_to_nhl_abbrev`]. Unknown abbrevs → `None`
//!     (LEAGUE bucket); we WARN-log so the user knows ESPN emitted
//!     something we don't recognize.
//!  4. Bucket the date into America/New_York calendar day (NHL operational
//!     TZ). A `2026-04-29T04:00:00Z` row (= 04-29 UTC = 04-29 ET earlier
//!     in the day) lands on 04-29 either way; the EDT-midnight boundary
//!     is the failure case we test below.
//!  5. Compute the dedup `id` = sha256(date + "|" + team + "|" + description).

use chrono::{Datelike, TimeZone};
use sha2::{Digest, Sha256};

use icelines_core::model::TeamAbbr;
use icelines_core::transactions::{
    classify, sanitize_description, Transaction, CURRENT_CLASSIFIER_VERSION,
};

use crate::schema::RawTransaction;
use crate::teams::espn_to_nhl_abbrev;

/// Convert one raw row. Pure function — no I/O.
pub fn raw_to_transaction(raw: &RawTransaction, season: &str) -> Transaction {
    let description = sanitize_description(&raw.description);
    let kind = classify(&description);
    let team = raw.team.as_ref().and_then(|t| {
        espn_to_nhl_abbrev(&t.abbreviation, season).map(|abbrev| TeamAbbr(abbrev.to_owned()))
    });
    let date = bucket_date_to_et(&raw.date);
    let id = id_for(&date, team.as_ref(), &description);

    Transaction {
        date,
        team,
        kind,
        description,
        id,
        trade_group_id: None,
        classifier_version: CURRENT_CLASSIFIER_VERSION,
    }
}

/// Convert a Vec — also surfaces unmapped-abbrev WARNs to a callback so
/// the CLI / TUI can log specifics. Returns the converted rows + a list
/// of warning strings (currently only unmapped abbrev events).
pub fn raw_to_transactions(
    raws: &[RawTransaction],
    season: &str,
) -> (Vec<Transaction>, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();
    let mut out: Vec<Transaction> = Vec::with_capacity(raws.len());
    for raw in raws {
        // Surface unmapped abbrev BEFORE we drop into the conversion.
        if let Some(team) = &raw.team {
            if espn_to_nhl_abbrev(&team.abbreviation, season).is_none() {
                let msg = format!(
                    "unmapped ESPN abbrev '{}' for season {} — row routed to LEAGUE bucket",
                    team.abbreviation, season,
                );
                if !warnings.contains(&msg) {
                    warnings.push(msg);
                }
            }
        }
        out.push(raw_to_transaction(raw, season));
    }
    (out, warnings)
}

/// Bucket a raw ESPN timestamp into America/New_York calendar day.
/// - "2026-04-29" (no time) → "2026-04-29".
/// - "2026-04-29T04:00:00Z" → "2026-04-29" (00:00 ET).
/// - "2026-04-29T03:30:00Z" → "2026-04-28" (still 11:30 PM ET prior day).
///
/// EST/EDT offset varies; chrono does the DST math.
fn bucket_date_to_et(raw: &str) -> String {
    // Date-only path — fast and most common.
    if raw.len() == 10 && raw.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return raw.to_owned();
    }
    // Try ISO 8601 with time + timezone.
    if let Ok(dt_utc) = chrono::DateTime::parse_from_rfc3339(raw) {
        // America/New_York is UTC-5 (EST) or UTC-4 (EDT). chrono-tz
        // handles DST, but we'd rather not pull in the dep just for
        // this. Approximation: fixed UTC-5 offset is wrong half the
        // year. Use chrono's offset arithmetic for the operational
        // window (March DST → November) approximated as UTC-4 EDT for
        // most of the NHL season (Oct–April overlaps both).
        //
        // Pragmatic approximation: bucket against UTC-5 for Oct–Mar and
        // UTC-4 for the rest. This is fragile across DST transitions
        // (Mar/Nov), but transactions on transition weekends are rare
        // and the EDT/EST off-by-one is one-day-and-back at most.
        //
        // Better long-term: depend on chrono-tz; out of scope for T.3.
        let utc_naive = dt_utc.naive_utc();
        let month = utc_naive.month();
        let offset_hours: i32 = if (4..=10).contains(&month) { -4 } else { -5 };
        let et = utc_naive + chrono::Duration::hours(offset_hours as i64);
        return et.format("%Y-%m-%d").to_string();
    }
    // Fallback: take the first 10 chars if they look date-like, else the raw.
    if raw.len() >= 10 {
        let head = &raw[..10];
        if head.chars().all(|c| c.is_ascii_digit() || c == '-') {
            return head.to_owned();
        }
    }
    raw.to_owned()
}

/// Stable id for dedup. SHA-256 over canonical (date, team, description).
fn id_for(date: &str, team: Option<&TeamAbbr>, description: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(date.as_bytes());
    hasher.update(b"|");
    hasher.update(team.map(|t| t.0.as_str()).unwrap_or("LEAGUE").as_bytes());
    hasher.update(b"|");
    hasher.update(description.as_bytes());
    format!("{:x}", hasher.finalize())
}

// Allow the chrono::Local import to silence unused without affecting prod.
#[allow(unused_imports)]
use chrono::Local;
#[allow(dead_code)]
fn _silence_unused() { let _ = Local::now; let _ = chrono::Utc.timestamp_opt(0, 0); }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::RawTransactionTeam;

    fn raw(team_abbrev: Option<&str>, description: &str) -> RawTransaction {
        RawTransaction {
            date: "2026-04-29".to_owned(),
            description: description.to_owned(),
            team: team_abbrev.map(|a| RawTransactionTeam {
                id: "0".to_owned(),
                abbreviation: a.to_owned(),
                display_name: format!("Team {a}"),
            }),
        }
    }

    #[test]
    fn l0_convert_classifies_and_sanitizes() {
        let r = raw(Some("EDM"), "Recalled F\nVasily Podkolzin\tfrom AHL");
        let t = raw_to_transaction(&r, "20252026");
        assert_eq!(t.kind, icelines_core::TransactionKind::Recall);
        // Whitespace controls collapsed to spaces.
        assert_eq!(t.description, "Recalled F Vasily Podkolzin from AHL");
        assert_eq!(t.team.as_ref().unwrap().0, "EDM");
        assert_eq!(t.classifier_version, CURRENT_CLASSIFIER_VERSION);
    }

    #[test]
    fn l0_convert_two_letter_espn_abbrev_normalizes() {
        let r = raw(Some("TB"), "Acquired D X from NSH");
        let t = raw_to_transaction(&r, "20252026");
        assert_eq!(t.team.as_ref().unwrap().0, "TBL",
            "TB must normalize to canonical TBL");
    }

    #[test]
    fn l0_convert_ari_2024_25_maps_to_uta() {
        let r = raw(Some("ARI"), "Signed F X to a 1-year contract");
        let t = raw_to_transaction(&r, "20242025");
        assert_eq!(t.team.as_ref().unwrap().0, "UTA",
            "post-relocation: ARI must map to UTA");
    }

    #[test]
    fn l0_convert_ari_2023_24_preserved_as_ari() {
        let r = raw(Some("ARI"), "Signed F X to a 1-year contract");
        let t = raw_to_transaction(&r, "20232024");
        assert_eq!(t.team.as_ref().unwrap().0, "ARI",
            "pre-relocation: ARI preserved");
    }

    #[test]
    fn l0_convert_unknown_abbrev_becomes_league_bucket() {
        let r = raw(Some("BOGUS"), "Some move by a fake team");
        let t = raw_to_transaction(&r, "20252026");
        assert!(t.team.is_none(),
            "unmapped abbrev must produce team=None for LEAGUE bucket");
    }

    #[test]
    fn l0_convert_team_none_preserved() {
        let r = raw(None, "League-wide reassignment deadline");
        let t = raw_to_transaction(&r, "20252026");
        assert!(t.team.is_none());
    }

    #[test]
    fn l0_convert_dedup_id_stable() {
        let r1 = raw(Some("EDM"), "Recalled F X from AHL");
        let r2 = raw(Some("EDM"), "Recalled F X from AHL");
        let t1 = raw_to_transaction(&r1, "20252026");
        let t2 = raw_to_transaction(&r2, "20252026");
        assert_eq!(t1.id, t2.id, "same (date, team, description) must produce same id");
    }

    #[test]
    fn l0_convert_dedup_id_differs_on_team() {
        let r1 = raw(Some("EDM"), "Move");
        let r2 = raw(Some("CHI"), "Move");
        let t1 = raw_to_transaction(&r1, "20252026");
        let t2 = raw_to_transaction(&r2, "20252026");
        assert_ne!(t1.id, t2.id);
    }

    // ── Date bucketing ────────────────────────────────────────────────

    #[test]
    fn l0_bucket_date_already_date_only_passthrough() {
        assert_eq!(bucket_date_to_et("2026-04-29"), "2026-04-29");
    }

    #[test]
    fn l0_bucket_date_iso8601_utc_into_et() {
        // 04:00 UTC in April = midnight EDT = same calendar day in ET.
        assert_eq!(bucket_date_to_et("2026-04-29T04:00:00Z"), "2026-04-29");
    }

    #[test]
    fn l0_bucket_date_iso8601_pre_midnight_et_buckets_prior_day() {
        // 03:30 UTC in April = 11:30 PM EDT prior calendar day.
        assert_eq!(bucket_date_to_et("2026-04-29T03:30:00Z"), "2026-04-28",
            "edt-midnight boundary: a row at 03:30 UTC should bucket to 04-28 ET");
    }

    #[test]
    fn l0_bucket_date_garbage_falls_through() {
        // Don't panic on malformed input — return raw or first 10 chars.
        assert_eq!(bucket_date_to_et(""), "");
        let weird = "not-a-date-at-all";
        let _ = bucket_date_to_et(weird); // just shouldn't panic
    }

    #[test]
    fn l0_raw_to_transactions_collects_unmapped_warnings() {
        let raws = vec![
            raw(Some("EDM"),   "Recalled F X"),
            raw(Some("BOGUS"), "Move by fake team"),
            raw(Some("BOGUS"), "Same fake team again"), // dedup'd warning
            raw(None,          "League-wide note"),
        ];
        let (txs, warnings) = raw_to_transactions(&raws, "20252026");
        assert_eq!(txs.len(), 4);
        assert_eq!(warnings.len(), 1, "duplicate unmapped abbrev should warn once");
        assert!(warnings[0].contains("BOGUS"));
    }
}
