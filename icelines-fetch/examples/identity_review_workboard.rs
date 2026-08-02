//! Lightweight source-package workboard renderer for validation and automation.

use icelines_core::source_facts::SourcePackage;
use icelines_fetch::build_identity_review_workboard_from_source_package;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: identity_review_workboard SOURCE_PACKAGE.json")?;
    let bytes = std::fs::read(&path)?;
    let package: SourcePackage = serde_json::from_slice(&bytes)?;
    let view = build_identity_review_workboard_from_source_package(&package)?;
    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}
