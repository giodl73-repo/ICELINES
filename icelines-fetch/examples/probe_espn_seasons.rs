//! Live probe — discovers which seasons ESPN's transactions archive covers.
//!
//! This is a **manual one-off**, NOT a test. It hits the live ESPN
//! site.api endpoint and prints per-season row counts. The result feeds
//! `icelines_core::transactions::TRANSACTIONS_EARLIEST_SEASON` — copy
//! the earliest non-empty season into that constant.
//!
//! Quarantined out of `cargo test` because BENCH-mandated cross-cutting
//! rule: "no live network in tests." Run on demand:
//!
//!     cargo run --example probe_espn_seasons -- 20212022 20252026
//!
//! Default range is the five bundled seasons.

use icelines_core::transactions::CURRENT_CLASSIFIER_VERSION;
use icelines_fetch::bundled::TransactionsEnvelope;
use icelines_fetch::transactions::{raw_to_transactions, EspnSource};
use std::collections::HashSet;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut write_bundle = false;
    args.retain(|a| {
        if a == "--write-bundle" {
            write_bundle = true;
            false
        } else {
            true
        }
    });
    let seasons: Vec<String> = if args.is_empty() {
        // Default: probe the five seasons IceLines bundles today.
        vec![
            "20212022".to_owned(),
            "20222023".to_owned(),
            "20232024".to_owned(),
            "20242025".to_owned(),
            "20252026".to_owned(),
        ]
    } else if args.len() == 2 {
        // Range: start..=end inclusive, in YYYYZZZZ form.
        let start: u32 = args[0].parse().map_err(|_| "first arg must be 8-digit season")?;
        let end:   u32 = args[1].parse().map_err(|_| "second arg must be 8-digit season")?;
        (start..=end)
            .step_by(10001) // 20212022 → 20222023 step
            .map(|n| format!("{n:08}"))
            .collect()
    } else {
        args
    };

    println!("ESPN transactions archive probe — {} seasons", seasons.len());
    if write_bundle {
        println!("Writing captured envelopes to data/seasons/{{season}}/transactions.json");
    }
    println!("{}", "─".repeat(70));
    println!("{:<10}  {:>8}  {:>8}  {}", "Season", "Rows", "Drift", "Notes");

    let source = EspnSource::production();

    for season in &seasons {
        let label = format!("{}-{}", &season[2..4], &season[6..8]);
        match source.fetch_season(season).await {
            Ok(outcome) => {
                // Cross-page dedup of the drift list — the per-page parser
                // can't see history.
                let unique: HashSet<String> = outcome.dropped_unknown_schema.iter().cloned().collect();
                let dropped = unique.len();
                let n = outcome.rows.len();
                let note = if outcome.partial { " [PARTIAL]" } else { "" };
                println!("{label:<10}  {n:>8}  {dropped:>8}{note}");
                if dropped > 0 {
                    let mut sorted: Vec<&String> = unique.iter().collect();
                    sorted.sort();
                    println!("           drift fields: {sorted:?}");
                }

                if write_bundle {
                    let (rows, warnings) = raw_to_transactions(&outcome.rows, season);
                    for w in &warnings {
                        eprintln!("           WARN: {w}");
                    }
                    let envelope = TransactionsEnvelope {
                        season:             season.to_owned(),
                        source:             "espn".to_owned(),
                        fetched_at:         outcome.fetched_at,
                        classifier_version: CURRENT_CLASSIFIER_VERSION,
                        rows,
                    };
                    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .ok_or("cannot resolve workspace root")?
                        .join("data").join("seasons").join(season);
                    std::fs::create_dir_all(&dir)?;
                    let path = dir.join("transactions.json");
                    let json = serde_json::to_vec_pretty(&envelope)?;
                    std::fs::write(&path, &json)?;
                    println!("           wrote {} ({} bytes)", path.display(), json.len());
                }
            }
            Err(e) => {
                println!("{label:<10}  {:>8}  {:>8}  ERROR: {e}", "—", "—");
            }
        }
    }

    println!();
    println!("Earliest non-empty season is the value for");
    println!("  icelines_core::transactions::TRANSACTIONS_EARLIEST_SEASON");
    if !write_bundle {
        println!();
        println!("Re-run with `--write-bundle` to also capture envelopes into");
        println!("data/seasons/{{season}}/transactions.json for embedding.");
    }

    Ok(())
}
