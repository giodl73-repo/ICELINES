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

fn run_signals_roster(args: &[&str]) -> std::process::Output {
    let home = tempfile::TempDir::new().expect("temp home");
    let mut full = vec!["signals-roster"];
    full.extend_from_slice(args);
    Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args(&full)
        .output()
        .expect("run icelines signals-roster")
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
    assert!(text.contains("Authority: Signals authority: descriptive derived metrics"));
    assert!(text.contains("Authority source: PlayerSignalsView stat inputs"));
    assert!(text.contains("Coverage state: descriptive_derived"));
    assert!(text.contains("Covered inputs: season_stat_summary"));
    assert!(text.contains("Covered metrics: physical_engagement_rate"));
    assert!(text.contains("Blocked claims: prediction"));
    assert!(text.contains("stat_catalog_promotion"));
    assert!(text.contains("leaderboard_ranking"));
    assert!(text.contains("not predictions, betting, injury, deployment"));
    // No silent zero-fill: the word "unavailable" is allowed; a bare 0.00 for a
    // missing signal is not the concern here since McDavid has full evidence.
    assert!(text.contains("evidence:"));
}

#[test]
fn l2_export_md_signals_writes_markdown_report_to_stdout() {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "export",
            "md",
            "signals",
            "--player",
            "Connor McDavid",
            "--out",
            "-",
        ])
        .output()
        .expect("run icelines export md signals");
    assert!(
        output.status.success(),
        "export md signals failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("type: signals"));
    assert!(text.contains("## Signals Scope"));
    assert!(text.contains("## Source Authority"));
    assert!(text.contains("- Source: PlayerSignalsView stat inputs"));
    assert!(text.contains("- Coverage state: descriptive_derived"));
    assert!(text.contains("- Covered metrics: physical_engagement_rate"));
    assert!(text.contains("- Blocked claims: prediction"));
    assert!(text.contains("deployment_recommendation"));
    assert!(text.contains("| Physical Engagement Rate |"));
    assert!(text.contains("Not a prediction"));
    assert!(text.contains("outside `StatId`, leaderboards, and the `--filter` catalog"));
}

#[test]
fn l2_signals_roster_text_is_team_scoped_discovery_not_leaderboard() {
    let output = run_signals_roster(&["--team", "NYR"]);
    assert!(
        output.status.success(),
        "signals-roster failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("SIGNALS ROSTER — NYR"), "{text}");
    assert!(
        text.contains("Team-scoped Signals discovery matrix"),
        "{text}"
    );
    assert!(text.contains("Evidence filter: all"), "{text}");
    assert!(text.contains("Rows: "), "{text}");
    assert!(text.contains("matched /"), "{text}");
    assert!(text.contains("filtered out"), "{text}");
    assert!(text.contains("Not a Signal leaderboard"), "{text}");
    assert!(text.contains("Authority: Signals authority"), "{text}");
    assert!(
        text.contains("Authority source: PlayerSignalsView stat inputs"),
        "{text}"
    );
    assert!(
        text.contains("Coverage state: descriptive_derived"),
        "{text}"
    );
    assert!(
        text.contains("Covered inputs: season_stat_summary"),
        "{text}"
    );
    assert!(
        text.contains("Covered metrics: physical_engagement_rate"),
        "{text}"
    );
    assert!(text.contains("Blocked claims: prediction"), "{text}");
    assert!(text.contains("leaderboard_ranking"), "{text}");
    assert!(text.contains("Mika Zibanejad"), "{text}");
    assert!(text.contains("Phys/60"), "{text}");
    assert!(text.contains("Evidence"), "{text}");
}

#[test]
fn l2_signals_roster_text_accepts_evidence_filter_without_leaderboard_promotion() {
    let output = run_signals_roster(&["--team", "NYR", "--evidence", "partial"]);
    assert!(
        output.status.success(),
        "signals-roster --evidence partial failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Evidence filter: partial"), "{text}");
    assert!(text.contains("filtered out"), "{text}");
    assert!(text.contains("Not a Signal leaderboard"), "{text}");
    assert!(text.contains("Player"), "{text}");
    assert!(text.contains("Evidence"), "{text}");
}

#[test]
fn l2_signals_roster_empty_evidence_filter_reports_no_match_not_no_skaters() {
    let output = run_signals_roster(&["--team", "NYR", "--evidence", "missing"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no Signals roster rows matched evidence filter 'missing'"),
        "{stderr}"
    );
    assert!(stderr.contains("0 matched /"), "{stderr}");
    assert!(stderr.contains("filtered out"), "{stderr}");
    assert!(!stderr.contains("no skaters found"), "{stderr}");
}

#[test]
fn l2_signals_roster_json_envelope_preserves_non_promotion_copy() {
    let output = run_signals_roster(&["--team", "NYR", "--json"]);
    assert!(
        output.status.success(),
        "signals-roster --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("signals-roster json parses");
    assert_eq!(json["schema"], "signals-roster.v1");
    assert_eq!(json["route"], "signals-roster");
    assert_eq!(json["data"]["team"], "NYR");
    assert_eq!(json["data"]["evidence_filter"], "all");
    assert_eq!(json["meta"]["evidence_filter"], "all");
    assert_eq!(
        json["data"]["total_player_count"],
        json["meta"]["total_player_count"]
    );
    assert_eq!(
        json["data"]["total_player_count"],
        json["meta"]["player_count"]
    );
    assert_eq!(json["meta"]["filtered_out_count"], 0);
    assert!(json["data"]["rows"].as_array().unwrap().len() > 5);
    assert!(
        json["meta"]["non_promotion"]
            .as_str()
            .unwrap()
            .contains("not a leaderboard"),
        "{json}"
    );
}

#[test]
fn l2_signals_roster_empty_full_filter_reports_no_match_not_no_skaters() {
    let output = run_signals_roster(&["--team", "NYR", "--evidence", "full"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no Signals roster rows matched evidence filter 'full'"),
        "{stderr}"
    );
    assert!(stderr.contains("0 matched /"), "{stderr}");
    assert!(stderr.contains("filtered out"), "{stderr}");
    assert!(!stderr.contains("no skaters found"), "{stderr}");
}
