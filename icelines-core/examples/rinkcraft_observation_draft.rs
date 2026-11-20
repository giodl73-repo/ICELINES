//! Generate an unfilled draft, never a completed observation or validated rate.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err("Usage: rinkcraft_observation_draft REGISTRY_JSON QUEUE_JSON PRIORITIES_JSON PLAYER_ID OUTPUT_JSON".into());
    }
    let mut raw = Vec::new();
    for path in &args[..3] {
        if std::fs::metadata(path)?.len() > 1024 * 1024 {
            return Err("Plan input exceeds1MiB".into());
        }
        raw.push(std::fs::read_to_string(path)?);
    }
    let draft = icelines_core::view_model::rinkcraft_measurement_plan::worksheet(
        &raw[0],
        &raw[1],
        &raw[2],
        args[3].parse()?,
    )?;
    std::fs::write(&args[4], serde_json::to_vec_pretty(&draft)?)?;
    Ok(())
}
