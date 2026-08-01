use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("icelines-fetch must be a workspace child")
        .to_path_buf()
}

fn inventory() -> serde_json::Value {
    let path = workspace_root().join("design/data/icelines-fetch-module-inventory.v1.json");
    serde_json::from_slice(&fs::read(path).expect("read source-module inventory"))
        .expect("parse source-module inventory")
}

#[test]
fn l0_source_inventory_covers_every_public_fetch_module() {
    let root = workspace_root();
    let lib = fs::read_to_string(root.join("icelines-fetch/src/lib.rs"))
        .expect("read icelines-fetch lib.rs");
    let declared = lib
        .lines()
        .filter_map(|line| {
            line.strip_prefix("pub mod ")
                .and_then(|module| module.strip_suffix(';'))
        })
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    let document = inventory();
    assert_eq!(document["schema"], "icelines_fetch_module_inventory.v1");
    let allowed_responsibilities = document["responsibility_classes"]
        .as_array()
        .expect("responsibility_classes array")
        .iter()
        .map(|value| value.as_str().expect("responsibility string"))
        .collect::<BTreeSet<_>>();
    let allowed_dispositions = document["migration_dispositions"]
        .as_array()
        .expect("migration_dispositions array")
        .iter()
        .map(|value| value.as_str().expect("disposition string"))
        .collect::<BTreeSet<_>>();

    let mut inventoried = BTreeSet::new();
    for row in document["modules"].as_array().expect("modules array") {
        let module = row["module"].as_str().expect("module name");
        assert!(
            inventoried.insert(module.to_owned()),
            "duplicate module inventory row: {module}"
        );
        let source_file = root.join(format!("icelines-fetch/src/{module}.rs"));
        let source_dir = root.join(format!("icelines-fetch/src/{module}/mod.rs"));
        assert!(
            source_file.is_file() || source_dir.is_file(),
            "inventoried module has no Rust source: {module}"
        );
        let responsibilities = row["responsibilities"]
            .as_array()
            .expect("responsibilities array");
        assert!(
            !responsibilities.is_empty(),
            "module has no responsibility classification: {module}"
        );
        for responsibility in responsibilities {
            let responsibility = responsibility.as_str().expect("responsibility string");
            assert!(
                allowed_responsibilities.contains(responsibility),
                "unknown responsibility {responsibility} for {module}"
            );
        }
        let disposition = row["disposition"].as_str().expect("disposition");
        assert!(
            allowed_dispositions.contains(disposition),
            "unknown migration disposition {disposition} for {module}"
        );
    }

    assert_eq!(
        inventoried, declared,
        "source inventory must change with the public icelines-fetch module set"
    );
}

#[test]
fn l0_source_compatibility_fixture_hashes_are_frozen() {
    let root = workspace_root();
    let document = inventory();
    for fixture in document["compatibility_baseline"]["fixtures"]
        .as_array()
        .expect("compatibility fixtures")
    {
        let relative = fixture["path"].as_str().expect("fixture path");
        let expected = fixture["sha256"].as_str().expect("fixture sha256");
        let bytes = fs::read(root.join(relative))
            .unwrap_or_else(|error| panic!("read compatibility fixture {relative}: {error}"));
        let actual = format!("{:x}", Sha256::digest(bytes));
        assert_eq!(
            actual, expected,
            "compatibility fixture changed: {relative}"
        );
    }
}

#[test]
fn l0_prospect_ranking_depth_baseline_reproduces_23_complete_9_partial() {
    let root = workspace_root();
    let path = root.join("design/data/prospect-ranking-depth-baseline-2026-07-31.v1.json");
    let baseline: serde_json::Value =
        serde_json::from_slice(&fs::read(path).expect("read ranking baseline"))
            .expect("parse ranking baseline");
    assert_eq!(baseline["schema"], "prospect_ranking_depth_baseline.v1");
    let requested = baseline["requested_depth"]
        .as_u64()
        .expect("requested depth");
    let rows = baseline["organizations"]
        .as_array()
        .expect("organization rows");

    let mut teams = BTreeSet::new();
    let mut complete = 0_u64;
    let mut partial = 0_u64;
    let mut total_shortfall = 0_u64;
    for row in rows {
        let team = row["team"].as_str().expect("team");
        assert!(
            teams.insert(team.to_owned()),
            "duplicate baseline team {team}"
        );
        let ranked = row["ranked"].as_u64().expect("ranked");
        let shortfall = row["shortfall"].as_u64().expect("shortfall");
        assert_eq!(ranked + shortfall, requested, "bad depth math for {team}");
        if shortfall == 0 {
            complete += 1;
        } else {
            partial += 1;
            total_shortfall += shortfall;
        }
    }

    let expected_teams = icelines_fetch::nhl_teams_for_season("20262027")
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert_eq!(teams, expected_teams);
    assert_eq!(complete, 23);
    assert_eq!(partial, 9);
    assert_eq!(total_shortfall, 17);
    assert_eq!(baseline["summary"]["complete"], complete);
    assert_eq!(baseline["summary"]["partial"], partial);
    assert_eq!(baseline["summary"]["total_shortfall"], total_shortfall);
}
