use anyhow::ensure;
use icelines_core::view_model::rinkcraft_stat_scores::Catalog;
use sha2::{Digest, Sha256};
use std::{fs, io::Cursor, path::Path};
pub fn run(
    catalog: &Path,
    season: u32,
    csv: &Path,
    captured_at: &str,
    output: &Path,
) -> anyhow::Result<()> {
    ensure!(!output.exists(), "Choose a new output directory");
    chrono::DateTime::parse_from_rfc3339(captured_at)?;
    ensure!(
        fs::metadata(catalog)?.len() <= 32 * 1024 * 1024
            && fs::metadata(csv)?.len() <= 32 * 1024 * 1024,
        "Input exceeds 32 MiB"
    );
    let raw = fs::read(catalog)?;
    let source = fs::read(csv)?;
    let mut data = Catalog::from_json(std::str::from_utf8(&raw)?).map_err(anyhow::Error::msg)?;
    ensure!(
        data.profiles
            .iter()
            .any(|p| p.season == season && p.group != "Goalies"),
        "Selected season has no skaters"
    );
    let source_sha256 = format!("{:x}", Sha256::digest(&source));
    let url = format!(
        "https://moneypuck.com/moneypuck/playerData/seasonSummary/{}/regular/skaters.csv",
        season / 10000
    );
    let matched = icelines_sources::rinkcraft_defending_context::enrich(
        &mut data.profiles,
        season,
        Cursor::new(&source),
        &format!("{captured_at} · {url} · SHA-256 {source_sha256}"),
    )
    .map_err(anyhow::Error::msg)?;
    data.validate().map_err(anyhow::Error::msg)?;
    let bytes = serde_json::to_vec(&data)?;
    ensure!(
        bytes.len() <= 32 * 1024 * 1024,
        "Output catalog exceeds 32 MiB"
    );
    let report = serde_json::json!({"schema":"rinkcraft.defending-record-import.v1","season":season,"source_credit":"NHL; MoneyPuck.com","source_url":url,"source_captured_at":captured_at,"source_sha256":source_sha256,"input_catalog_sha256":format!("{:x}",Sha256::digest(&raw)),"output_catalog_sha256":format!("{:x}",Sha256::digest(&bytes)),"matched_defending_players":matched,"production_grades_changed":false});
    fs::create_dir_all(output)?;
    fs::write(output.join("rinkcraft-stat-attributes.json"), bytes)?;
    fs::write(
        output.join("import-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
