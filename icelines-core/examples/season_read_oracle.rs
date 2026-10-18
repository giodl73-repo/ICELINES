//! Emit legacy catalog observations before refactoring, then retain as a golden.
#[path = "../tests/support/season_read_cases.rs"]
mod cases;
fn main() {
    println!(
        "{}",
        serde_json::to_string(&cases::legacy_observations(&cases::cases())).unwrap()
    );
}
