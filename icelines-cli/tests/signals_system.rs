//! L2 system tests for `icelines signals` (Phase Hurricane / WP-010 pulse-03).
//!
//! Invokes the compiled binary against bundled data only — no live network calls.

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn run_signals(args: &[&str]) -> serde_json::Value {
    let home = tempfile::TempDir::new().expect("temp home");
    let mut full = vec!["signals"];
    full.extend_from_slice(args);
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args(&full)
        .output()
        .expect("run icelines signals");
    assert!(
        output.status.success(),
        "signals {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("signals --json parses")
}

#[test]
fn l2_signals_json_envelope_is_signals_v1_with_three_rows() {
    let json = run_signals(&["Connor McDavid", "--json"]);
    assert_eq!(json["schema"], "signals.v1");
    assert_eq!(json["route"], "signals");
    let rows = json["data"]["rows"]
        .as_array()
        .expect("data.rows is an array");
    assert_eq!(rows.len(), 3, "three signals ship today");

    let ids: Vec<&str> = rows.iter().filter_map(|r| r["id"].as_str()).collect();
    assert!(ids.contains(&"physical-engagement-rate"));
    assert!(ids.contains(&"puck-management-differential"));
    assert!(ids.contains(&"penalty-drag-rate"));

    // Non-claim copy must travel with the JSON surface (promotion rule).
    let non_claims = json["data"]["non_claims"]
        .as_array()
        .expect("non_claims present");
    assert!(!non_claims.is_empty());
}

#[test]
fn l2_signals_missing_evidence_renders_null_not_zero() {
    // 1988-89 is a bundled skeleton season with no realtime (hits/blocks/
    // takeaways) data, so Physical Engagement Rate has no value. It must be
    // JSON `null`, never 0.0 (spec §Evidence contract).
    let json = run_signals(&["Wayne Gretzky", "--season", "19881989", "--json"]);
    let rows = json["data"]["rows"].as_array().expect("rows array");
    let phys = rows
        .iter()
        .find(|r| r["id"] == "physical-engagement-rate")
        .expect("physical-engagement-rate row present");
    assert!(
        phys["value"].is_null(),
        "missing realtime must yield null, got {}",
        phys["value"]
    );
    assert_ne!(phys["value"].as_f64(), Some(0.0));
    assert_ne!(phys["evidence_tier"], "full");
}

#[test]
fn l2_signals_text_surface_prints_disclaimer() {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args(["signals", "Connor McDavid"])
        .output()
        .expect("run icelines signals text");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("SIGNALS — "));
    assert!(text.contains("Disclaimer:"));
    // No silent zero-fill: the word "unavailable" is allowed; a bare 0.00 for a
    // missing signal is not the concern here since McDavid has full evidence.
    assert!(text.contains("evidence:"));
}
