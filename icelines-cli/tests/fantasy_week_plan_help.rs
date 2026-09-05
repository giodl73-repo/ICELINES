use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn help(args: &[&str]) -> String {
    let output = Command::new(icelines_bin())
        .args(args)
        .output()
        .expect("run icelines help");
    assert!(
        output.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn l2_week_plan_and_journal_commands_are_discoverable() {
    let fantasy = help(&["fantasy", "--help"]);
    for command in ["week-plan", "decision-record", "decision-review"] {
        assert!(fantasy.contains(command), "fantasy help missing {command}");
    }

    let plan = help(&["fantasy", "week-plan", "--help"]);
    for flag in [
        "--week",
        "--team",
        "--candidates",
        "--max-moves",
        "--beam-width",
        "--alternatives",
        "--json",
    ] {
        assert!(plan.contains(flag), "week-plan help missing {flag}");
    }

    let record = help(&["fantasy", "decision-record", "--help"]);
    assert!(record.contains("--chosen"));
    assert!(record.contains("--rationale"));

    let review = help(&["fantasy", "decision-review", "--help"]);
    assert!(review.contains("--include-private"));
}
