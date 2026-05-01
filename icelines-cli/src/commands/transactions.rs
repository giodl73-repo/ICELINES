//! `icelines transactions` — league-wide moves report. Phase T.4.
//!
//! Loads the per-season snapshot via [`load_transactions_with_fallback`]
//! (re-classifies stale rows on load), applies filters, and emits via the
//! shared `commands::output::Format` so CSV / JSON / `--out` work
//! uniformly with the rest of the report commands.

use std::path::PathBuf;

use anyhow::{bail, Context};

use icelines_core::transactions::{
    description_matches_query, transactions_for_player,
    Transaction, TransactionKind, TRANSACTIONS_EARLIEST_SEASON,
};
use icelines_fetch::{
    bundled::load_transactions_with_fallback,
    snapshot::{SnapshotMetaFlags, SnapshotStore},
};

use crate::commands::output::Format;
use crate::config::Config;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    team: Option<String>,
    since: Option<String>,
    until: Option<String>,
    kind: Option<String>,
    search: Option<String>,
    player: Option<String>,
    season: Option<String>,
    top: Option<usize>,
    json: bool,
    csv: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let format = Format::resolve(csv, json)?;

    // ── Resolve season ────────────────────────────────────────────────
    let cfg = Config::load()?;
    let season = season.unwrap_or_else(|| cfg.season_str());

    if season.as_str() < TRANSACTIONS_EARLIEST_SEASON {
        // EDGE-mandated: never tell the user to fetch a season ESPN
        // doesn't archive — that would 404. Surface coverage cleanly.
        bail!(
            "Transactions data begins {early}. The {season} season is not \
             covered by ESPN's archive.",
            early = format_season(TRANSACTIONS_EARLIEST_SEASON),
            season = format_season(&season),
        );
    }

    // ── Validate filter args before doing any I/O ─────────────────────
    let kind_filter: Option<Vec<TransactionKind>> = match kind.as_deref() {
        Some(k) => Some(TransactionKind::parse_filter(k).map_err(anyhow::Error::msg)?),
        None    => None,
    };
    if let Some(s) = since.as_deref() {
        validate_iso_date(s, "--since")?;
    }
    if let Some(u) = until.as_deref() {
        validate_iso_date(u, "--until")?;
    }
    if let (Some(s), Some(u)) = (since.as_deref(), until.as_deref()) {
        if s > u {
            bail!("--since {s} is after --until {u}");
        }
    }

    // ── Load snapshot + meta flags ────────────────────────────────────
    let snapshots_root = cfg.snapshot_dir();
    let store = SnapshotStore::new(snapshots_root.clone());
    let envelope = load_transactions_with_fallback(&season, &store)
        .context("loading transactions snapshot")?;

    let flags = SnapshotMetaFlags::load(&snapshots_root, &season);
    if flags.transactions_stale && format == Format::Table && out.is_none() {
        let last_err = flags
            .transactions_last_error
            .as_deref()
            .unwrap_or("(no detail)");
        eprintln!("WARN: transactions snapshot is stale (last fetch failed: {last_err})");
    }

    // ── Apply filters ─────────────────────────────────────────────────
    let team_norm: Option<String> = team.map(|t| t.to_ascii_uppercase());

    // Player filter pre-resolves to a set of row pointers (cheap on a
    // few-thousand-row dataset). Empty Vec when --player is None.
    let player_hits: Vec<*const Transaction> = match player.as_deref() {
        Some(name) => {
            let team_for_disambig = team_norm.as_deref()
                .filter(|t| !t.eq_ignore_ascii_case("LEAGUE"));
            transactions_for_player(&envelope.rows, name, team_for_disambig)
                .iter().map(|tx| *tx as *const Transaction).collect()
        }
        None => Vec::new(),
    };
    let player_active = player.is_some();

    let mut rows: Vec<&Transaction> = envelope.rows.iter().filter(|tx| {
        if let Some(team) = team_norm.as_deref() {
            let row_label = tx.team.as_ref().map(|t| t.0.as_str()).unwrap_or("LEAGUE");
            if !row_label.eq_ignore_ascii_case(team) {
                return false;
            }
        }
        if let Some(s) = since.as_deref() {
            if tx.date.as_str() < s { return false; }
        }
        if let Some(u) = until.as_deref() {
            if tx.date.as_str() > u { return false; }
        }
        if let Some(kinds) = &kind_filter {
            if !kinds.contains(&tx.kind) {
                return false;
            }
        }
        if let Some(q) = search.as_deref() {
            if !description_matches_query(&tx.description, q) {
                return false;
            }
        }
        if player_active {
            let ptr = *tx as *const Transaction;
            if !player_hits.contains(&ptr) {
                return false;
            }
        }
        true
    }).collect();

    // ── Sort newest-first for human-readable output ──────────────────
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    if let Some(n) = top {
        rows.truncate(n);
    }

    // ── Emit ─────────────────────────────────────────────────────────
    let headers = &["date", "team", "kind", "description", "id"];
    let body: Vec<Vec<String>> = rows.iter().map(|tx| {
        let team_label = tx.team.as_ref().map(|t| t.0.clone()).unwrap_or_else(|| "LEAGUE".to_owned());
        vec![
            tx.date.clone(),
            team_label,
            tx.kind.label().to_owned(),
            tx.description.clone(),
            tx.id.clone(),
        ]
    }).collect();

    if format == Format::Table && out.is_none() {
        let total = envelope.rows.len();
        let shown = body.len();
        println!("Transactions · ESPN · season {} · as of {}",
            format_season(&season), envelope.fetched_at);
        println!("{shown} of {total} rows shown");
    }
    format.emit_to(headers, &body, out.as_deref())?;
    Ok(())
}

/// Format an 8-digit season ID as `YY-YY` (e.g. `20252026` → `25-26`).
fn format_season(s: &str) -> String {
    if s.len() == 8 {
        format!("{}-{}", &s[2..4], &s[6..8])
    } else {
        s.to_owned()
    }
}

/// Validate a `YYYY-MM-DD` argument. We don't depend on chrono::NaiveDate
/// here — a small char-level check is enough and produces nicer errors.
fn validate_iso_date(s: &str, flag: &str) -> anyhow::Result<()> {
    let bad = || anyhow::anyhow!(
        "{flag} is not a valid date (expected YYYY-MM-DD): '{s}'"
    );
    if s.len() != 10 {
        return Err(bad());
    }
    let bytes = s.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(bad());
    }
    for (i, c) in s.chars().enumerate() {
        let must_be_digit = !matches!(i, 4 | 7);
        if must_be_digit && !c.is_ascii_digit() {
            return Err(bad());
        }
    }
    // Range checks on month / day. Forgive Feb 30 etc. — we'd rather
    // accept "filter on this calendar date" than block the user on
    // technically-invalid dates.
    let month: u8 = s[5..7].parse().map_err(|_| bad())?;
    let day:   u8 = s[8..10].parse().map_err(|_| bad())?;
    if !(1..=12).contains(&month) {
        return Err(anyhow::anyhow!("{flag} month out of range: '{s}'"));
    }
    if !(1..=31).contains(&day) {
        return Err(anyhow::anyhow!("{flag} day out of range: '{s}'"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_validate_iso_date_accepts_valid() {
        assert!(validate_iso_date("2026-04-29", "--since").is_ok());
        assert!(validate_iso_date("2018-09-01", "--since").is_ok());
    }

    #[test]
    fn l0_validate_iso_date_rejects_garbage() {
        for bad in &[
            "",
            "abc",
            "2026/04/29",
            "26-04-29",
            "2026-04",
            "2026-04-29T00:00:00Z",
        ] {
            assert!(validate_iso_date(bad, "--since").is_err(),
                "{bad:?} should not validate");
        }
    }

    #[test]
    fn l0_validate_iso_date_month_out_of_range() {
        assert!(validate_iso_date("2026-13-40", "--since").is_err());
        assert!(validate_iso_date("2026-00-15", "--since").is_err());
    }

    #[test]
    fn l0_validate_iso_date_day_out_of_range() {
        assert!(validate_iso_date("2026-04-00", "--since").is_err());
        assert!(validate_iso_date("2026-04-32", "--since").is_err());
    }

    #[test]
    fn l0_format_season_yy_yy() {
        assert_eq!(format_season("20252026"), "25-26");
        assert_eq!(format_season("20182019"), "18-19");
        assert_eq!(format_season("19951996"), "95-96");
    }

    #[test]
    fn l0_format_season_passthrough_on_unexpected() {
        assert_eq!(format_season(""), "");
        assert_eq!(format_season("nope"), "nope");
    }
}
