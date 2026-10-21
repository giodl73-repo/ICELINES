#[allow(dead_code)]
#[path = "../tests/support/player_read_cases.rs"]
mod cases;
fn main() {
    println!("{}", serde_json::to_string(&cases::observations()).unwrap());
}
