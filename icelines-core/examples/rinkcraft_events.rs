//! A request is an authored bounded episode, not a full game or observed play.
use icelines_core::view_model::rinkcraft_events;
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: rinkcraft_events INPUT.json OUTPUT.json".into());
    }
    let q = serde_json::from_slice(&std::fs::read(&args[0]).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let result = rinkcraft_events::run(&q)?;
    std::fs::write(
        &args[1],
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{} events; goals {}-{}; remaining {}s; {}",
        result.events.len(),
        result.final_state.score[0],
        result.final_state.score[1],
        result.final_state.seconds_remaining,
        result
            .final_state
            .unresolved
            .as_deref()
            .unwrap_or("bounded episode ended")
    );
    Ok(())
}
