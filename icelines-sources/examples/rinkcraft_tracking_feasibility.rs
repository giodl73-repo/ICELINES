use std::{fs, io::Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("Usage: rinkcraft_tracking_feasibility INPUT_CSV NEW_REPORT_JSON".into());
    }
    if fs::metadata(&args[0])?.len() > 64 * 1024 * 1024 {
        return Err("Tracking input exceeds64MiB".into());
    }
    let report = icelines_sources::tracking_feasibility::audit(&fs::read(&args[0])?)?;
    let bytes = serde_json::to_vec_pretty(&report)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?
        .write_all(&bytes)?;
    println!(
        "Audited {} source rows / {} frames; no capability measurements inferred",
        report.rows, report.frames
    );
    Ok(())
}
