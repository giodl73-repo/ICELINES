//! Compare held cover and committed support on the verified recorded path.
use icelines_core::view_model::rinkcraft_plan_compare::compare_replay_from_json;
use icelines_core::view_model::rinkcraft_possession::MAX_REPLAY_BYTES;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: rinkcraft_replay_compare REPLAY.json".into());
    }
    if std::fs::metadata(&args[0])?.len() > MAX_REPLAY_BYTES as u64 {
        return Err("Replay exceeds 256 KiB".into());
    }
    println!(
        "{}",
        compare_replay_from_json(&std::fs::read_to_string(&args[0])?)?
    );
    Ok(())
}
