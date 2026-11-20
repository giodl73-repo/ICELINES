//! Verify a saved fictional scene using exactly the native/WASM Rust engine.
use icelines_core::view_model::rinkcraft_possession::{import_replay, MAX_REPLAY_BYTES};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: rinkcraft_replay_verify REPLAY.json".into());
    }
    if std::fs::metadata(&args[0])?.len() > MAX_REPLAY_BYTES as u64 {
        return Err("Replay exceeds 256 KiB".into());
    }
    let saved = import_replay(&std::fs::read_to_string(&args[0])?)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "verified":true,"schema":saved.schema,"hero":saved.request.hero.name,
            "seed":saved.request.seed,"actions":saved.request.actions,"state":saved.session["state"],
            "rules_version":saved.rules.version,"verification":"Internal consistency under embedded authored rules; not an author signature or NHL assessment."
        }))?
    );
    Ok(())
}
