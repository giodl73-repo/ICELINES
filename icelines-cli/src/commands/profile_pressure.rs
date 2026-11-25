//! Offline, checksum-verified faceoff samples from the existing public game cache.
use anyhow::{ensure, Context};
use icelines_core::view_model::rinkcraft_under_pressure::Faceoffs;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
pub struct Cache {
    pub samples: BTreeMap<u32, Faceoffs>,
    pub sources: Value,
    pub games: u32,
}
pub fn read(root: &Path, season: u32) -> anyhow::Result<Cache> {
    let root = root.join(season.to_string());
    let mut captures = vec![];
    let mut seen = BTreeSet::new();
    let mut totals = BTreeMap::<u32, icelines_sources::rinkcraft_pressure::Counts>::new();
    for entry in fs::read_dir(&root).with_context(|| format!("Read {}", root.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        ensure!(
            fs::metadata(&path)?.len() <= 256 * 1024,
            "Cache metadata exceeds bound"
        );
        let c: Value = serde_json::from_slice(&fs::read(&path)?)?;
        let Some(url) = c["url"].as_str() else {
            continue;
        };
        let Some(id) = url
            .strip_prefix("https://api-web.nhle.com/v1/gamecenter/")
            .and_then(|s| s.strip_suffix("/play-by-play"))
            .and_then(|s| s.parse::<u64>().ok())
        else {
            continue;
        };
        ensure!(
            id / 1_000_000 == (season / 10000) as u64,
            "Wrong cached game season"
        );
        ensure!(
            seen.insert(id) && seen.len() <= 2000,
            "Duplicate or excessive cached games"
        );
        ensure!(c["status"] == 200, "Cached game was not successful");
        chrono::DateTime::parse_from_rfc3339(
            c["captured_at"].as_str().context("Missing capture time")?,
        )?;
        let key = format!("{:x}", Sha256::digest(url.as_bytes()));
        ensure!(
            path.file_stem().and_then(|s| s.to_str()) == Some(key.as_str()),
            "Cached URL key differs"
        );
        let raw = path.with_extension("bin");
        ensure!(
            fs::metadata(&raw)?.len() <= 16 * 1024 * 1024,
            "Game exceeds bound"
        );
        let bytes = fs::read(raw)?;
        ensure!(
            c["sha256"].as_str() == Some(format!("{:x}", Sha256::digest(&bytes)).as_str()),
            "Cached game checksum differs"
        );
        let game = icelines_sources::rinkcraft_pressure::parse_game(&bytes, id, season)
            .map_err(anyhow::Error::msg)?;
        for (id, n) in game {
            let s = totals.entry(id).or_default();
            s.taken += n.taken;
            s.won += n.won;
            s.late_taken += n.late_taken;
            s.late_won += n.late_won;
        }
        captures.push(c);
    }
    ensure!(
        !captures.is_empty(),
        "No finalized public game cache supplied"
    );
    captures.sort_by(|a, b| a["url"].as_str().cmp(&b["url"].as_str()));
    let captured_at = captures
        .iter()
        .filter_map(|c| c["captured_at"].as_str())
        .max()
        .context("Missing capture time")?
        .to_string();
    let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&captures)?));
    let games = captures.len() as u32;
    let samples = totals
        .into_iter()
        .map(|(player_id, c)| {
            (
                player_id,
                Faceoffs {
                    season,
                    player_id,
                    taken: c.taken,
                    won: c.won,
                    late_taken: c.late_taken,
                    late_won: c.late_won,
                    coverage_games: games,
                    captured_at: captured_at.clone(),
                    source_sha256: fingerprint.clone(),
                },
            )
        })
        .collect();
    Ok(Cache {
        samples,
        sources: json!({"schema":"rinkcraft.pressure-sources.v1","season":season,"coverage_games":games,"source_sha256":fingerprint,"captured_at":captured_at,"captures":captures,"note":"Cached finalized-game sample, not a claim of full-season or player-GP coverage."}),
        games,
    })
}
