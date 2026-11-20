use std::process::Command;

#[test]
fn l2_rinkcraft_profile_help_and_window_validation() {
    let bin = env!("CARGO_BIN_EXE_icelines");
    let help = Command::new(bin)
        .args(["profile", "--help"])
        .output()
        .unwrap();
    assert!(
        help.status.success(),
        "{}",
        String::from_utf8_lossy(&help.stderr)
    );
    let text = String::from_utf8_lossy(&help.stdout);
    for flag in ["--first-season", "--last-season", "--type", "--json"] {
        assert!(text.contains(flag));
    }
    let first = icelines_core::CURRENT_SEASON.to_string();
    let last = (icelines_core::CURRENT_SEASON - 10001).to_string();
    let out = Command::new(bin)
        .args([
            "profile",
            "1",
            "--first-season",
            &first,
            "--last-season",
            &last,
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("first season"));
}

#[test]
fn l2_rinkcraft_profile_historical_goalie_playoffs_do_not_require_latest_playoffs() {
    // The bundled-history boundary requires a bundled id; select it without any
    // named player/team fixture or behavior assumption. No live network is used.
    let (season, id) = icelines_fetch::bundled::BUNDLED_SEASONS
        .iter()
        .find_map(|s| {
            icelines_fetch::bundled::get_playoff_goalie_stats(s)
                .and_then(|rows| rows.first().map(|g| ((*s).to_owned(), g.player_id)))
        })
        .expect("a historical goalie playoff row is bundled");
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_icelines"))
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .args([
            "profile",
            &id.to_string(),
            "--type",
            "playoff",
            "--first-season",
            &season,
            "--last-season",
            &season,
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let profile: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(profile["request"]["season_type"], "playoff");
    assert_eq!(profile["timeline"].as_array().unwrap().len(), 1);
    assert_eq!(profile["timeline"][0]["position"], "Goalie");
}
