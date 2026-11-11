//! Inspect a moment, or branch its verified fictional replay using the shared Rust engine.
use icelines_core::view_model::rinkcraft_possession::{
    branch_replay, inspect_replay_from_json, ForkRequest, MAX_REPLAY_BYTES,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=4).contains(&args.len()) || (args.len() == 4 && args[3] != "--replay") {
        return Err(
            "Usage: rinkcraft_replay_inspect REPLAY.json STEP [hold|commit|same [--replay]]".into(),
        );
    }
    if std::fs::metadata(&args[0])?.len() > MAX_REPLAY_BYTES as u64 {
        return Err("Replay exceeds 256 KiB".into());
    }
    let raw = std::fs::read_to_string(&args[0])?;
    let step: usize = args[1].parse()?;
    let result = if args.len() == 2 {
        inspect_replay_from_json(&raw, step)?
    } else {
        let support = match args[2].as_str() {
            "hold" => Some(false),
            "commit" => Some(true),
            "same" => None,
            _ => return Err("Branch plan must be hold, commit or same".into()),
        };
        let branch = branch_replay(
            &raw,
            ForkRequest {
                step,
                commit_support: support,
            },
        )?;
        if args.len() == 4 {
            serde_json::to_string(&branch.replay)?
        } else {
            serde_json::to_string(&branch)?
        }
    };
    println!("{result}");
    Ok(())
}
