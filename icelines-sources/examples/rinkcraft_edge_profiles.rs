//! Add source-bound EDGE context without changing existing profile values.
use serde_json::{
    json,
    value::{to_raw_value, RawValue},
    Value,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::Path};
type RawMap = BTreeMap<String, Box<RawValue>>;
fn read(spec: &Value) -> Result<Vec<u8>, String> {
    let b =
        fs::read(spec["path"].as_str().ok_or("input path absent")?).map_err(|e| e.to_string())?;
    if spec["sha256"].as_str() != Some(format!("{:x}", Sha256::digest(&b)).as_str()) {
        return Err("input checksum mismatch".into());
    }
    Ok(b)
}
fn project(
    bytes: &[u8],
    edge: Option<&icelines_core::view_model::rinkcraft_edge::EdgeEvidence>,
    status: &Value,
) -> Result<Vec<u8>, String> {
    let mut root: RawMap = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let frame = root.get("numerical_evidence").ok_or("no numerical frame")?;
    let mut numerical: RawMap =
        serde_json::from_str(frame.get()).map_err(|_| "no numerical frame")?;
    if numerical.contains_key("edge_evidence") || numerical.contains_key("edge_capture_status") {
        return Err("profile already contains EDGE evidence".into());
    }
    if let Some(e) = edge {
        numerical.insert(
            "edge_evidence".into(),
            to_raw_value(e).map_err(|e| e.to_string())?,
        );
    }
    numerical.insert(
        "edge_capture_status".into(),
        to_raw_value(status).map_err(|e| e.to_string())?,
    );
    root.insert(
        "numerical_evidence".into(),
        to_raw_value(&numerical).map_err(|e| e.to_string())?,
    );
    serde_json::to_vec(&root).map_err(|e| e.to_string())
}
fn run(m: &Value, out: &Path) -> Result<(), String> {
    if out.exists() {
        return Err("new output directory required".into());
    }
    let catalog_bytes = read(&m["catalog"])?;
    let catalog: Value = serde_json::from_slice(&catalog_bytes).map_err(|e| e.to_string())?;
    let capture: Value =
        serde_json::from_slice(&read(&m["capture"])?).map_err(|e| e.to_string())?;
    if catalog["schema"] != "rinkcraft.public-preview.v1"
        || capture["schema"] != "icelines.edge-cohort-capture.v1"
        || capture["catalog_sha256"] != m["catalog"]["sha256"]
    {
        return Err("capture/catalog binding mismatch".into());
    }
    let base = Path::new(
        m["base_directory"]
            .as_str()
            .ok_or("base directory absent")?,
    );
    let mut inventory: BTreeMap<String, String> =
        serde_json::from_slice(&read(&m["base_inventory"])?).map_err(|e| e.to_string())?;
    let mut records = BTreeMap::new();
    for r in capture["records"]
        .as_array()
        .ok_or("capture records absent")?
    {
        let key = (
            r["season"].as_u64().ok_or("season absent")?,
            r["player_id"].as_u64().ok_or("actor absent")?,
        );
        if records.insert(key, r).is_some() {
            return Err("duplicate EDGE capture actor/window".into());
        }
    }
    let mut receipts = Vec::new();
    for w in catalog["windows"].as_array().ok_or("windows absent")? {
        let season = w["season"].as_u64().ok_or("season absent")? as u32;
        for p in w["players"].as_array().ok_or("players absent")? {
            let id = p["id"].as_u64().ok_or("actor absent")? as u32;
            let games = p["games"].as_u64().ok_or("GP absent")? as u32;
            let group = p["group"].as_str().ok_or("group absent")?;
            let relative = format!("data/{season}/{id}.json");
            if p["url"] != relative {
                return Err("unexpected card path".into());
            }
            let record = records
                .remove(&(season as u64, id as u64))
                .ok_or("capture record missing")?;
            if record["group"] != group || record["expected_games"] != games {
                return Err("capture target differs from source card".into());
            }
            let bytes = fs::read(base.join(&relative)).map_err(|e| e.to_string())?;
            let base_hash = format!("{:x}", Sha256::digest(&bytes));
            if inventory.remove(&relative).as_deref() != Some(base_hash.as_str()) {
                return Err("baseline profile inventory mismatch".into());
            }
            let original: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if original["request"]["player_id"] != id
                || original["latest_available"]["season"] != season
                || original["numerical_evidence"]["season"] != season
                || original["numerical_evidence"]["group"] != group
                || original["latest_available"]["games"] != games
            {
                return Err("base card binding mismatch".into());
            }
            let parsed = if record["status"] == "captured" {
                let raw = read(record)?;
                if record["bytes"].as_u64() != Some(raw.len() as u64) {
                    return Err("capture size differs".into());
                }
                icelines_sources::nhl::edge::parse(
                    &raw,
                    id,
                    season,
                    group,
                    games,
                    record["url"].as_str().ok_or("URL absent")?,
                    record["accessed_utc"]
                        .as_str()
                        .ok_or("capture timestamp absent")?,
                )
            } else {
                Err("NHL EDGE request unavailable for this capture".into())
            };
            let state = if parsed.is_ok() {
                "available"
            } else {
                "unavailable"
            };
            let reason = parsed.as_ref().err().cloned();
            let status = json!({"state":state,"reason":reason,"source_url":record["url"],"attempted_at":record["accessed_utc"]});
            let new = project(&bytes, parsed.as_ref().ok(), &status)?;
            let path = out.join(&relative);
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(path, new).map_err(|e| e.to_string())?;
            receipts.push(json!({"player_id":id,"season":season,"state":state,"reason":reason,"source_url":record["url"],"captured_at":record["accessed_utc"]}));
        }
    }
    if !records.is_empty() {
        return Err("capture contains actors outside source catalog".into());
    }
    if !inventory.is_empty() {
        return Err("unused baseline inventory profiles".into());
    }
    fs::write(out.join("catalog.json"), catalog_bytes).map_err(|e| e.to_string())?;
    let available = receipts
        .iter()
        .filter(|r| r["state"] == "available")
        .count();
    fs::write(out.join("edge-coverage.json"),serde_json::to_vec(&json!({"schema":"rinkcraft.edge-coverage.v1","profiles":receipts.len(),"available":available,"unavailable":receipts.len()-available,"records":receipts})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    println!(
        "EDGE context: {available}/{} available; baseline profile values preserved.",
        receipts.len()
    );
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let result = (|| {
        if a.len() != 3 {
            return Err("Usage: INPUT_MANIFEST NEW_OUTPUT_DIRECTORY".into());
        }
        let m: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        run(&m, Path::new(&a[2]))
    })();
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_is_not_a_zero_measure_and_original_numbers_survive() {
        let raw =
            br#"{"numerical_evidence":{"value":93.71134020618557},"other":{"unchanged":true}}"#;
        let status = json!({"state":"unavailable","reason":"source unavailable"});
        let b = project(raw, None, &status).unwrap();
        let text = String::from_utf8(b.clone()).unwrap();
        assert!(text.contains("93.71134020618557"));
        let v: Value = serde_json::from_slice(&b).unwrap();
        assert!(v["numerical_evidence"]["edge_evidence"].is_null());
        assert_eq!(v["other"]["unchanged"], true);
        assert!(project(&b, None, &status).is_err());
    }
}
