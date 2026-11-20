//! Native byte entry point for the same stateless evaluator used by WASM.
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};
fn run(input: &Path, expected: &str, output: &Path) -> Result<(), String> {
    if output.exists() {
        return Err("new output file required".into());
    }
    if fs::metadata(input).map_err(|e| e.to_string())?.len()
        > icelines_core::view_model::rinkcraft_seasonal_records::MAX_INPUT_BYTES as u64
    {
        return Err("input exceeds 32 MiB".into());
    }
    let bytes = fs::read(input).map_err(|e| e.to_string())?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err("input checksum mismatch".into());
    }
    let result = icelines_core::view_model::rinkcraft_seasonal_records::evaluate_json(&bytes)?;
    fs::write(output, result).map_err(|e| e.to_string())?;
    println!("Shared seasonal-record catalog written; no package mutation or awards.");
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    if a.len() != 4 {
        eprintln!("usage: rinkcraft_seasonal_records INPUT_JSON SHA256 NEW_OUTPUT_JSON");
        std::process::exit(2);
    }
    if let Err(e) = run(Path::new(&a[1]), &a[2], Path::new(&a[3])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
