//! Explicit offline plan export; paths in authored JSON are metadata, never opened.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("Usage: rinkcraft_measurement_plan REGISTRY_JSON QUEUE_JSON PRIORITIES_JSON OUTPUT_JSON".into());
    }
    let mut raw = Vec::new();
    for path in &args[..3] {
        if std::fs::metadata(path)?.len() > 1024 * 1024 {
            return Err("Plan input exceeds1MiB".into());
        }
        raw.push(std::fs::read_to_string(path)?);
    }
    let plan =
        icelines_core::view_model::rinkcraft_measurement_plan::build(&raw[0], &raw[1], &raw[2])?;
    std::fs::write(&args[3], serde_json::to_vec_pretty(&plan)?)?;
    Ok(())
}
