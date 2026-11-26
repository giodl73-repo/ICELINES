//! Explicit offline paired-source import; never enables generic shift synchronization.
use anyhow::{ensure, Context};
use icelines_core::view_model::{
    rinkcraft_stat_scores::Catalog,
    rinkcraft_under_pressure::{self as pressure, Exposure},
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct Source {
    path: PathBuf,
    sha256: String,
    url: String,
}
#[derive(Deserialize)]
struct Game {
    game_id: u64,
    boxscore: Source,
    shifts: Source,
}
#[derive(Deserialize)]
struct Manifest {
    schema: String,
    season: u32,
    captured_at: String,
    games: Vec<Game>,
}
fn read(path: &Path, max: u64) -> anyhow::Result<Vec<u8>> {
    ensure!(
        fs::metadata(path)?.len() <= max,
        "Exposure input exceeds size bound"
    );
    Ok(fs::read(path)?)
}
fn source(s: &Source, url: &str) -> anyhow::Result<Vec<u8>> {
    ensure!(s.url == url, "Unexpected exposure source URL");
    let bytes = read(&s.path, 16 * 1024 * 1024)?;
    ensure!(
        format!("{:x}", Sha256::digest(&bytes)).eq_ignore_ascii_case(&s.sha256),
        "Exposure source checksum differs: {}",
        s.path.display()
    );
    Ok(bytes)
}
pub fn run(
    catalog: &Path,
    season: u32,
    cache: &Path,
    manifest: &Path,
    output: &Path,
) -> anyhow::Result<()> {
    run_import(catalog, season, cache, manifest, output, false)
}
pub fn units(
    catalog: &Path,
    season: u32,
    cache: &Path,
    manifest: &Path,
    output: &Path,
) -> anyhow::Result<()> {
    run_import(catalog, season, cache, manifest, output, true)
}
fn run_import(
    catalog: &Path,
    season: u32,
    cache: &Path,
    manifest: &Path,
    output: &Path,
    units: bool,
) -> anyhow::Result<()> {
    ensure!(!output.exists(), "Choose a new exposure output directory");
    let input = read(catalog, 32 * 1024 * 1024)?;
    let mut data = Catalog::from_json(std::str::from_utf8(&input)?).map_err(anyhow::Error::msg)?;
    let raw = read(manifest, 2 * 1024 * 1024)?;
    let m: Manifest = serde_json::from_slice(&raw)?;
    ensure!(
        m.schema == "rinkcraft.pressure-shift-inputs.v1"
            && m.season == season
            && !m.games.is_empty()
            && m.games.len() <= 2000,
        "Invalid exposure manifest scope"
    );
    chrono::DateTime::parse_from_rfc3339(&m.captured_at)?;
    let events = super::profile_pressure::read(cache, season)?;
    let parent = events.sources["source_sha256"]
        .as_str()
        .context("Missing event fingerprint")?;
    let mut h = Sha256::new();
    h.update(&raw);
    h.update(parent.as_bytes());
    let fingerprint = format!("{:x}", h.finalize());
    let mut seen = BTreeSet::new();
    let mut samples = BTreeMap::<u32, Exposure>::new();
    let mut unit_samples =
        BTreeMap::<u32, Vec<icelines_core::view_model::rinkcraft_unit_links::Game>>::new();
    let mut quality = Vec::new();
    for g in &m.games {
        ensure!(seen.insert(g.game_id), "Duplicate exposure game");
        let (path, sha) = events
            .bodies
            .get(&g.game_id)
            .context("Shift game absent from parent event sample")?;
        let pbp = read(path, 16 * 1024 * 1024)?;
        ensure!(
            format!("{:x}", Sha256::digest(&pbp)) == *sha,
            "Event input changed during exposure import"
        );
        let b = source(
            &g.boxscore,
            &format!(
                "https://api-web.nhle.com/v1/gamecenter/{}/boxscore",
                g.game_id
            ),
        )?;
        let s = source(
            &g.shifts,
            &format!(
                "https://api.nhle.com/stats/rest/en/shiftcharts?cayenneExp=gameId={}",
                g.game_id
            ),
        )?;
        let result =
            icelines_sources::rinkcraft_pressure_exposure::analyze(&pbp, &b, &s, g.game_id, season)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("Exposure game {}", g.game_id))?;
        quality.push(json!({"game_id":g.game_id,"eligible_players":result.players.len(),"excluded_players":result.excluded_players,"pbp_sha256":sha}));
        if units {
            for (id, g) in
                icelines_sources::rinkcraft_unit_links::from_analyzed(&b, g.game_id, &result)
                    .map_err(anyhow::Error::msg)?
            {
                unit_samples.entry(id).or_default().push(g);
            }
        } else {
            for (id, c) in result.players {
                let e = samples.entry(id).or_insert_with(|| Exposure {
                    source_sha256: fingerprint.clone(),
                    ..Default::default()
                });
                e.game_ids.push(g.game_id);
                e.late_lead_seconds += c.late_lead_seconds;
                e.unresolved_team_seconds += c.unresolved_team_seconds;
                e.late_lead_blocks += c.late_lead_blocks;
                e.late_lead_takeaways += c.late_lead_takeaways;
            }
        }
    }
    let mut attached = 0;
    for p in data
        .profiles
        .iter_mut()
        .filter(|p| p.season == season && (units || p.group != "Goalies"))
    {
        if units {
            let Some(mut games) = unit_samples.remove(&p.player_id) else {
                continue;
            };
            games.retain(|g| g.group == p.group);
            if games.is_empty() {
                continue;
            }
            p.unit_links = Some(
                icelines_core::view_model::rinkcraft_unit_links::build(
                    p,
                    games,
                    fingerprint.clone(),
                    parent.into(),
                )
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("Unit links player {}", p.player_id))?,
            );
            attached += 1;
            continue;
        }
        let Some(mut e) = samples.remove(&p.player_id) else {
            continue;
        };
        e.game_ids.sort_unstable();
        let s = p
            .under_pressure
            .as_mut()
            .and_then(|r| r.action_sample.as_mut())
            .context("Paired exposure requires an existing action sample")?;
        ensure!(
            s.source_sha256 == parent,
            "Action and exposure event captures differ"
        );
        s.exposure = Some(e);
        p.under_pressure = Some(pressure::rebuild(p));
        attached += 1;
    }
    ensure!(attached > 0, "No exposure matched selected profiles");
    data.validate().map_err(anyhow::Error::msg)?;
    let bytes = serde_json::to_vec(&data)?;
    ensure!(
        bytes.len() <= 32 * 1024 * 1024,
        "Exposure catalog exceeds size bound"
    );
    let report = json!({"schema":if units {"rinkcraft.unit-links-import.v1"}else{"rinkcraft.pressure-exposure-report.v1"},"season":season,"matched_games":seen.len(),"attached_profiles":attached,"source_sha256":fingerprint,"input_catalog_sha256":format!("{:x}",Sha256::digest(input)),"output_catalog_sha256":format!("{:x}",Sha256::digest(&bytes)),"production_grades_changed":false,"quality":quality});
    fs::create_dir_all(output)?;
    fs::write(output.join("rinkcraft-stat-attributes.json"), bytes)?;
    fs::write(
        output.join(if units {
            "unit-links-report.json"
        } else {
            "exposure-report.json"
        }),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(
        output.join(if units {
            "unit-links-sources.json"
        } else {
            "exposure-sources.json"
        }),
        serde_json::to_vec_pretty(
            &json!({"manifest":serde_json::from_slice::<serde_json::Value>(&raw)?,"parent_event_sources":events.sources,"source_sha256":fingerprint}),
        )?,
    )?;
    println!(
        "{}",
        json!({"matched_games":seen.len(),"attached_profiles":attached,"production_grades_changed":false})
    );
    Ok(())
}
