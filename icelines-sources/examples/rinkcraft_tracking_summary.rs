use std::{fs, io::Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 4 {
        return Err(
            "Usage: rinkcraft_tracking_summary GAME_ID SUMMARY_JSON ENTITIES_JSON NEW_OUTPUT_JSON"
                .into(),
        );
    }
    for path in &a[1..3] {
        if fs::metadata(path)?.len() > 16 * 1024 * 1024 {
            return Err("Tracking input exceeds 16MiB".into());
        }
    }
    let report = icelines_sources::nhl::tracking_summary::parse(
        &fs::read(&a[1])?,
        &fs::read(&a[2])?,
        a[0].parse()?,
    )?;
    let bytes = serde_json::to_vec_pretty(&report)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[3])?
        .write_all(&bytes)?;
    println!(
        "Parsed {} tracked players; no attribute scores",
        report.players.len()
    );
    Ok(())
}
