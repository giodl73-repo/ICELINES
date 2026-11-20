//! Authenticate explicitly supplied source files and summarize completed coding.
use icelines_core::view_model::rinkcraft_task_measurements::{measure, Input};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::Path,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "Usage: rinkcraft_task_measurements INPUT_JSON SOURCE_PATHS_JSON NEW_OUTPUT_JSON"
                .into(),
        );
    }
    if fs::metadata(&args[0])?.len() > 128 * 1024 * 1024
        || fs::metadata(&args[1])?.len() > 1024 * 1024
    {
        return Err("Input exceeds limit".into());
    }
    let raw = fs::read_to_string(&args[0])?;
    let report = measure(&raw)?;
    let input: Input = serde_json::from_str(&raw)?;
    let paths: BTreeMap<String, String> = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    if paths.len() != input.protocol.sources.len() {
        return Err("Source path inventory must match protocol exactly".into());
    }
    let base = Path::new(&args[1]).parent().unwrap_or(Path::new("."));
    for source in &input.protocol.sources {
        let path = base.join(paths.get(&source.id).ok_or("Missing source file binding")?);
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        if format!("{:x}", hasher.finalize()) != source.sha256 {
            return Err(format!("Source checksum mismatch: {}", source.id).into());
        }
    }
    let bytes = serde_json::to_vec_pretty(&report)?;
    // Validation and source checks finish before any output is created.
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?
        .write_all(&bytes)?;
    println!("Measured {} observations across {} cells; {} component coverage rows; no capability scores", report.observations, report.cells.len(), report.coverage.len());
    Ok(())
}
