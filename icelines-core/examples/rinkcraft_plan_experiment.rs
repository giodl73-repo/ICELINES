//! Read a bounded authored request and print a reproducible paired-seed experiment.
use icelines_core::view_model::rinkcraft_plan_experiment::{run_from_json, MAX_INPUT_BYTES};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: rinkcraft_plan_experiment REQUEST.json".into());
    }
    if std::fs::metadata(&args[0])?.len() > MAX_INPUT_BYTES as u64 {
        return Err("Experiment request exceeds 128 KiB".into());
    }
    println!("{}", run_from_json(&std::fs::read_to_string(&args[0])?)?);
    Ok(())
}
