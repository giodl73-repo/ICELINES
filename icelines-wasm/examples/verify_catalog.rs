//! Build-time verifier; filesystem access stays outside the browser library.
use icelines_wasm::BrowserEngine;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("expected catalog path")?);
    let catalog: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let entries = catalog["packages"].as_array().ok_or("missing packages")?;
    let base = path.parent().ok_or("missing catalog directory")?;
    for entry in entries {
        let url = entry["url"].as_str().ok_or("missing package URL")?;
        let package_path = base.join(url).canonicalize()?;
        if !package_path.starts_with(base.canonicalize()?) {
            return Err("package escapes catalog directory".into());
        }
        let bytes = std::fs::read(package_path)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if entry["sha256"].as_str() != Some(&digest) {
            return Err("package checksum mismatch".into());
        }
        let mut engine = BrowserEngine::new();
        engine
            .load_bytes(&bytes)
            .map_err(|e| format!("{}: {e}", entry["id"]))?;
    }
    println!(
        "Verified hashes and shared engine loading for {} packages",
        entries.len()
    );
    Ok(())
}
