use anyhow::Context;
use icelines_core::view_model::{
    rinkcraft_attribute_calibration as study, rinkcraft_stat_scores::Catalog,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
pub fn carryover(paths: &[std::path::PathBuf], output: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(!output.exists(), "Choose a new output directory");
    anyhow::ensure!(
        (2..=10).contains(&paths.len()),
        "Use two to ten season reports"
    );
    let mut reports = vec![];
    let mut inputs = vec![];
    for path in paths {
        anyhow::ensure!(
            fs::metadata(path)?.len() <= 32 * 1024 * 1024,
            "Report exceeds 32 MiB"
        );
        let raw = fs::read(path)?;
        let value: serde_json::Value = serde_json::from_slice(&raw)?;
        let report: study::Report = serde_json::from_slice(&raw)?;
        inputs.push(serde_json::json!({"season":report.season,"report_sha256":format!("{:x}",Sha256::digest(&raw)),"input_catalog_sha256":value["input_catalog_sha256"],"input_package_sha256":value["input_package_sha256"]}));
        reports.push(report);
    }
    let result = icelines_core::view_model::rinkcraft_attribute_carryover::compare(&reports)
        .map_err(anyhow::Error::msg)?;
    inputs.sort_by_key(|v| v["season"].as_u64());
    let mut value = serde_json::to_value(&result)?;
    value["inputs"] = serde_json::json!(inputs);
    let mut md=String::from("# Attribute carryover\n\nNHL / MoneyPuck.com · unchanged diagnostic recipes; no production grades change.\n\n");
    for note in &result.notes {
        md.push_str(&format!("- {note}\n"));
    }
    md.push_str("\n| Seasons | Recipe | Group | Sample | Pairs | Repeatability baseline → candidate | Next production baseline → candidate |\n| --- | --- | --- | --- | ---: | --- | --- |\n");
    let fmt = |v: Option<f64>| v.map(|v| format!("{v:.3}")).unwrap_or_else(|| "—".into());
    for c in &result.comparisons {
        md.push_str(&format!(
            "| {} → {} | {} | {} | {} | {} | {} → {} | {} → {} |\n",
            c.from,
            c.to,
            c.label,
            c.group,
            c.sample,
            c.matched_players,
            fmt(c.baseline_repeatability),
            fmt(c.candidate_repeatability),
            fmt(c.baseline_next_production),
            fmt(c.candidate_next_production)
        ));
    }
    fs::create_dir_all(output)?;
    fs::write(
        output.join("carryover.json"),
        serde_json::to_vec_pretty(&value)?,
    )?;
    fs::write(output.join("carryover.md"), md)?;
    fs::write(
        output.join("index.html"),
        include_str!("../../../design/previews/rinkcraft-carryover.html"),
    )?;
    println!(
        "{}",
        serde_json::json!({"seasons":result.seasons,"comparisons":result.comparisons.len(),"output":output,"production_grades_changed":false})
    );
    Ok(())
}
pub fn run(catalog: &Path, package: &Path, season: u32, output: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(!output.exists(), "Choose a new output directory");
    anyhow::ensure!(
        fs::metadata(catalog)?.len() <= 32 * 1024 * 1024,
        "Catalog exceeds 32 MiB"
    );
    anyhow::ensure!(
        fs::metadata(package)?.len() <= 32 * 1024 * 1024,
        "Package exceeds 32 MiB"
    );
    let raw = fs::read(catalog)?;
    let package_raw = fs::read(package)?;
    let catalog = Catalog::from_json(std::str::from_utf8(&raw)?).map_err(anyhow::Error::msg)?;
    let package: icelines_sources::schema::SeasonPackage = serde_json::from_slice(&package_raw)?;
    anyhow::ensure!(
        package.schema_version == 1
            && package.season == season
            && package.season_type == icelines_core::season_stats::SeasonType::Regular,
        "Package must match the selected regular season"
    );
    let bios = package
        .bios
        .iter()
        .map(|b| (b.player_id, b))
        .collect::<BTreeMap<_, _>>();
    for p in catalog
        .profiles
        .iter()
        .filter(|p| p.season == season && p.group != "Goalies")
    {
        let bio = bios
            .get(&p.player_id)
            .context("Player absent from package")?;
        anyhow::ensure!(
            bio.games_played == p.games,
            "Package/catalog GP mismatch for {}",
            p.player_id
        );
    }
    let names = bios
        .into_iter()
        .map(|(id, b)| (id, b.skater_full_name.clone()))
        .collect();
    let report = study::compare(&catalog, season, &names).map_err(anyhow::Error::msg)?;
    let mut value = serde_json::to_value(&report)?;
    value["input_catalog_sha256"] = format!("{:x}", Sha256::digest(&raw)).into();
    value["input_package_sha256"] = format!("{:x}", Sha256::digest(&package_raw)).into();
    let mut md=format!("# Attribute recipe comparison — {season}\n\nDiagnostic candidates; production grades remain unchanged.\n\n");
    for note in &report.notes {
        md.push_str(&format!("- {note}\n"));
    }
    for c in &report.candidates {
        md.push_str(&format!("\n## {}\n\n{}\n\n| Group | Paired | Up | Down | Same | Mean absolute change | PP correlation: baseline → candidate |\n| --- | ---: | ---: | ---: | ---: | ---: | --- |\n",c.label,c.formula));
        let number = |v: Option<f64>| v.map(|v| format!("{v:.3}")).unwrap_or_else(|| "—".into());
        for g in &c.groups {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} → {} |\n",
                g.group,
                g.paired_players,
                g.up,
                g.down,
                g.unchanged,
                number(g.mean_absolute_change),
                number(g.baseline_pp_correlation),
                number(g.candidate_pp_correlation)
            ));
        }
        md.push_str("\nLargest changes among reference-eligible players:\n\n| Player | Group | Baseline | Candidate | Change | Input |\n| --- | --- | ---: | ---: | ---: | --- |\n");
        let mut movers = c
            .rows
            .iter()
            .filter(|r| r.reference_eligible && r.delta.is_some())
            .collect::<Vec<_>>();
        movers.sort_by(|a, b| {
            b.delta
                .unwrap()
                .abs()
                .cmp(&a.delta.unwrap().abs())
                .then(a.name.cmp(&b.name))
        });
        for r in movers.into_iter().take(15) {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {:+} | {} |\n",
                r.name.replace('|', "/"),
                r.group,
                r.baseline.unwrap(),
                r.candidate.unwrap(),
                r.delta.unwrap(),
                r.input_detail
            ));
        }
    }
    fs::create_dir_all(output)?;
    fs::write(
        output.join("calibration.json"),
        serde_json::to_vec_pretty(&value)?,
    )?;
    fs::write(output.join("calibration.md"), md)?;
    fs::write(
        output.join("index.html"),
        include_str!("../../../design/previews/rinkcraft-calibration.html"),
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"season":season,"candidate_recipes":report.candidates.len(),"output":output,"production_grades_changed":false})
        )?
    );
    Ok(())
}
