//! Offline projection using the same history selector as native and WASM consumers.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "Usage: rinkcraft_profile_snapshot HISTORY_JSON PLAYER_ID SEASON OUTPUT_JSON".into(),
        );
    }
    let history =
        icelines_core::view_model::rinkcraft_history::parse(&std::fs::read_to_string(&args[0])?)?;
    let selected = history.snapshot(args[1].parse()?, None, None, Some(args[2].parse()?))?;
    std::fs::write(&args[3], serde_json::to_vec_pretty(&selected)?)?;
    Ok(())
}
