use anyhow::ensure;
use icelines_core::view_model::{
    rinkcraft_skater_context::attach_records, rinkcraft_stat_scores::Catalog,
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
pub fn run(
    catalog: &Path,
    season: u32,
    output: &Path,
    game_cache: Option<&Path>,
) -> anyhow::Result<()> {
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
    let pressure_cache = game_cache
        .map(|root| super::profile_pressure::read(root, season))
        .transpose()?;
    if let Some(cache) = &pressure_cache {
        let methods=super::profile_shot_methods::collect(cache,season)?;
        for p in data
            .profiles
            .iter_mut()
            .filter(|p| p.season == season && p.group != "Goalies")
        {
            p.shot_method_sample=methods.get(&p.player_id).cloned();p.shot_methods=None;
            p.under_pressure = Some(
                icelines_core::view_model::rinkcraft_under_pressure::build_with_actions(
                    p,
                    cache.samples.get(&p.player_id).cloned(),
                    cache.actions.get(&p.player_id).cloned(),
                ),
            );
        }
    }
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
    let passing_records = data
        .profiles
        .iter()
        .filter(|p| {
            p.season == season
                && p.skater_context
                    .as_ref()
                    .is_some_and(|c| c.passing.is_some())
        })
        .count();
    let shooting_records = data
        .profiles
        .iter()
        .filter(|p| {
            p.season == season
                && p.skater_context
                    .as_ref()
                    .is_some_and(|c| c.shooting.is_some())
        })
        .count();
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
    let report = serde_json::json!({"schema":"rinkcraft.profile-records.v1","season":season,"handling_records":records,"reading_records":reading_records,"passing_records":passing_records,"shooting_records":shooting_records,"pressure_sample_games":pressure_cache.as_ref().map(|c|c.games),"input_catalog_sha256":format!("{:x}",Sha256::digest(&raw)),"output_catalog_sha256":format!("{:x}",Sha256::digest(&bytes)),"production_grades_changed":false});
    fs::create_dir_all(output)?;
    fs::write(output.join("rinkcraft-stat-attributes.json"), bytes)?;
    fs::write(
        output.join("records-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    if let Some(cache) = &pressure_cache {
        fs::write(
            output.join("pressure-sources.json"),
            serde_json::to_vec_pretty(&cache.sources)?,
        )?;
    }
    println!("{}", report);
    Ok(())
}
