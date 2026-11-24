use anyhow::ensure;
use icelines_core::view_model::{
    rinkcraft_skater_context::attach_records, rinkcraft_stat_scores::Catalog,
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
pub fn run(catalog: &Path, season: u32, output: &Path) -> anyhow::Result<()> {
    ensure!(!output.exists(), "Choose a new output directory");
    ensure!(
        fs::metadata(catalog)?.len() <= 32 * 1024 * 1024,
        "Catalog exceeds 32 MiB"
    );
    let raw = fs::read(catalog)?;
    let mut data = Catalog::from_json(std::str::from_utf8(&raw)?).map_err(anyhow::Error::msg)?;
    ensure!(
        data.profiles
            .iter()
            .any(|p| p.season == season && p.skater_context.is_some()),
        "No skater context in selected season"
    );
    attach_records(&mut data.profiles, season);
    data.validate().map_err(anyhow::Error::msg)?;
    let records = data
        .profiles
        .iter()
        .filter(|p| {
            p.season == season
                && p.skater_context
                    .as_ref()
                    .is_some_and(|c| c.handling.is_some())
        })
        .count();
    let bytes = serde_json::to_vec(&data)?;
    let reading_records = data
        .profiles
        .iter()
        .filter(|p| {
            p.season == season
                && p.skater_context
                    .as_ref()
                    .is_some_and(|c| c.reading.is_some())
        })
        .count();
    ensure!(bytes.len() <= 32 * 1024 * 1024, "Output exceeds 32 MiB");
    let report = serde_json::json!({"schema":"rinkcraft.profile-records.v1","season":season,"handling_records":records,"reading_records":reading_records,"input_catalog_sha256":format!("{:x}",Sha256::digest(&raw)),"output_catalog_sha256":format!("{:x}",Sha256::digest(&bytes)),"production_grades_changed":false});
    fs::create_dir_all(output)?;
    fs::write(output.join("rinkcraft-stat-attributes.json"), bytes)?;
    fs::write(
        output.join("records-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", report);
    Ok(())
}
