fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("Usage: raw_source_inventory json|csv INPUT_FILE OUTPUT_JSON".into());
    }
    if std::fs::metadata(&args[1])?.len() > 32 * 1024 * 1024 {
        return Err("Raw input exceeds32MiB".into());
    }
    let raw = std::fs::read_to_string(&args[1])?;
    let result = icelines_sources::raw_inventory::inventory(&args[0], &raw)?;
    std::fs::write(&args[2], serde_json::to_vec_pretty(&result)?)?;
    Ok(())
}
