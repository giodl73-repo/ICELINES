fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 3 {
        return Err("Usage: csv_category_inventory INPUT_CSV GROUP_COLUMN OUTPUT_JSON".into());
    }
    if std::fs::metadata(&a[0])?.len() > 32 * 1024 * 1024 {
        return Err("Input exceeds32MiB".into());
    }
    let raw = std::fs::read_to_string(&a[0])?;
    let r = icelines_sources::raw_inventory::csv_categories(&raw, &a[1])?;
    std::fs::write(&a[2], serde_json::to_vec_pretty(&r)?)?;
    Ok(())
}
