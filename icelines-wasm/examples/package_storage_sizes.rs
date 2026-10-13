//! Diagnostic only: compare raw DTO storage with canonical export JSON.
//! Native sizes are not WASM heap measurements or browser-budget acceptance.
use icelines_sources::schema::{GoalieStats, SkaterBio, SkaterStats};
use icelines_wasm::SeasonPackage;
use serde_json::Value;
use std::{fs, mem::size_of, path::PathBuf};

fn string_bytes(value: &Value) -> usize {
    match value {
        Value::String(value) => value.len(),
        Value::Array(values) => values.iter().map(string_bytes).sum(),
        Value::Object(values) => values.values().map(string_bytes).sum(),
        _ => 0,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide artifact directory")?,
    );
    let catalog: Value = serde_json::from_slice(&fs::read(root.join("catalog.json"))?)?;
    let mut rows = Vec::new();
    for entry in catalog["packages"].as_array().ok_or("missing packages")? {
        let bytes = fs::read(root.join(entry["url"].as_str().ok_or("missing path")?))?;
        let package: SeasonPackage = serde_json::from_slice(&bytes)?;
        let canonical = serde_json::to_vec(&package)?;
        let fixed = size_of::<SeasonPackage>()
            + package.bios.capacity() * size_of::<SkaterBio>()
            + package.stats.capacity() * size_of::<SkaterStats>()
            + package.goalies.capacity() * size_of::<GoalieStats>();
        // Lower bound: UTF-8 lengths, excluding String spare capacity and
        // allocator bookkeeping. JSON object keys are not DTO-owned strings.
        let strings = string_bytes(&serde_json::to_value(&package)?);
        rows.push(
            serde_json::json!({"id":entry["id"],"input_bytes":bytes.len(),
            "canonical_json_bytes":canonical.len(),"native_fixed_bytes":fixed,
            "string_length_bytes":strings,"native_lower_bound_bytes":fixed + strings}),
        );
    }
    println!(
        "{}",
        serde_json::json!({"scope":"Diagnostic native DTO lower bounds; not WASM/browser measurements. Canonical bytes include all existing exported values. No runtime change.",
        "pointer_bytes":size_of::<usize>(),"packages":rows})
    );
    Ok(())
}
