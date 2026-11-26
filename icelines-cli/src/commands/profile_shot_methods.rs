//! Offline shot-repertoire enrichment from the existing checksum-verified game cache.
use anyhow::{ensure, Context};
use icelines_core::view_model::{rinkcraft_shot_methods::Sample, rinkcraft_stat_scores::Catalog};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
pub(super) fn collect(
    cache: &super::profile_pressure::Cache,
    season: u32,
) -> anyhow::Result<BTreeMap<u32, Sample>> {
    let mut samples = cache
        .actions
        .iter()
        .map(|(&id, a)| {
            (
                id,
                Sample {
                    player_id: id,
                    season,
                    roster_games: a.roster_games,
                    coverage_games: a.coverage_games,
                    by_type: BTreeMap::new(),
                    captured_at: a.captured_at.clone(),
                    source_sha256: a.source_sha256.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (&game, (path, hash)) in &cache.bodies {
        ensure!(
            fs::metadata(path)?.len() <= 16 * 1024 * 1024,
            "Shot-method body exceeds bound"
        );
        let bytes = fs::read(path)?;
        ensure!(
            format!("{:x}", Sha256::digest(&bytes)) == *hash,
            "Shot-method body changed during import"
        );
        for (id, types) in
            icelines_sources::rinkcraft_shot_methods::parse_game(&bytes, game, season)
                .map_err(anyhow::Error::msg)?
        {
            let s = samples
                .get_mut(&id)
                .context("Shot actor outside admitted skater sample")?;
            for (key, c) in types {
                s.by_type.entry(key).or_default().add(&c);
            }
        }
    }
    Ok(samples)
}
pub fn run(catalog: &Path, season: u32, cache: &Path, output: &Path) -> anyhow::Result<()> {
    ensure!(
        !output.exists(),
        "Choose a new shot-method output directory"
    );
    ensure!(
        fs::metadata(catalog)?.len() <= 32 * 1024 * 1024,
        "Shot-method catalog exceeds bound"
    );
    let raw = fs::read(catalog)?;
    let mut data = Catalog::from_json(std::str::from_utf8(&raw)?).map_err(anyhow::Error::msg)?;
    let cache = super::profile_pressure::read(cache, season)?;
    let samples = collect(&cache, season)?;
    let mut attached = 0;
    let mut attempts = 0u64;
    let mut typed = 0u64;
    for p in data
        .profiles
        .iter_mut()
        .filter(|p| p.season == season && p.group != "Goalies")
    {
        p.shot_method_sample = samples.get(&p.player_id).cloned();
        p.shot_methods = None;
        if let Some(s) = &p.shot_method_sample {
            attached += 1;
            for (key, c) in &s.by_type {
                attempts += u64::from(c.attempts);
                if key != "unknown" {
                    typed += u64::from(c.attempts);
                }
            }
        }
    }
    ensure!(attached > 0, "No matched shot-method profiles");
    data.validate().map_err(anyhow::Error::msg)?;
    let bytes = serde_json::to_vec(&data)?;
    ensure!(
        bytes.len() <= 32 * 1024 * 1024,
        "Enriched shot-method catalog exceeds bound"
    );
    let report = json!({"schema":"rinkcraft.shot-method-import.v1","season":season,"sample_games":cache.games,"attached_profiles":attached,"unblocked_attempts":attempts,"typed_attempts":typed,"input_catalog_sha256":format!("{:x}",Sha256::digest(raw)),"output_catalog_sha256":format!("{:x}",Sha256::digest(&bytes)),"production_grades_changed":false});
    fs::create_dir_all(output)?;
    fs::write(output.join("rinkcraft-stat-attributes.json"), bytes)?;
    fs::write(
        output.join("shot-method-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(
        output.join("shot-method-sources.json"),
        serde_json::to_vec_pretty(&cache.sources)?,
    )?;
    println!("{report}");
    Ok(())
}
