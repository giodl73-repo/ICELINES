//! `icelines data` — download and manage additional season data bundles.

use crate::cli::DataSubcommand;
use anyhow::Context;

/// All seasons with published GitHub Releases, newest first.
/// The first 5 are also bundled in the binary; the rest require `data install`.
const AVAILABLE_SEASONS: &[&str] = &[
    // Current + recent (bundled in binary)
    "20252026", "20242025", "20232024", "20222023", "20212022",
    // Historical — downloadable via `icelines data install`
    "20202021", "20192020", "20182019", "20172018", "20162017",
    "20152016", "20142015", "20132014", "20122013", "20112012",
    "20102011", "20092010", "20082009", "20072008", "20062007",
    "20052006", "20032004", "20022003", "20012002", "20002001",
    // Note: 20042005 is omitted — lockout season, no games played
];

const RELEASE_URL_TEMPLATE: &str =
    "https://github.com/giodl73-repo/ICELINES/releases/download/data-{SEASON}/data-{SEASON}.tar.gz";

pub async fn run(cmd: DataSubcommand) -> anyhow::Result<()> {
    match cmd {
        DataSubcommand::Install { seasons, season } => {
            run_install(seasons, season).await
        }
        DataSubcommand::List => run_list(),
        DataSubcommand::Remove { season } => run_remove(&season),
    }
}

// ── install ───────────────────────────────────────────────────────────────────

async fn run_install(seasons: u8, season: Option<String>) -> anyhow::Result<()> {
    let seasons_dir = seasons_base_dir()?;
    std::fs::create_dir_all(&seasons_dir)
        .with_context(|| format!("create {}", seasons_dir.display()))?;

    // Build the list of seasons to install.
    let to_install: Vec<&str> = if let Some(ref s) = season {
        // Specific season requested.
        if !AVAILABLE_SEASONS.contains(&s.as_str()) {
            println!(
                "Season {s} not yet available — run `icelines fetch stats --season {s}` to build it."
            );
            return Ok(());
        }
        vec![s.as_str()]
    } else {
        // Last N seasons, newest first.
        AVAILABLE_SEASONS
            .iter()
            .take(seasons as usize)
            .copied()
            .collect()
    };

    for s in to_install {
        install_season(&seasons_dir, s).await?;
    }

    Ok(())
}

async fn install_season(
    seasons_dir: &std::path::Path,
    season: &str,
) -> anyhow::Result<()> {
    let url = RELEASE_URL_TEMPLATE.replace("{SEASON}", season);
    let dest = seasons_dir.join(season);

    // Download.
    let client = reqwest::Client::builder()
        .user_agent("icelines-cli")
        .build()
        .context("build HTTP client")?;

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        println!(
            "Season {season} not yet available — run \
             `icelines fetch stats --season {season}` to build it."
        );
        return Ok(());
    }

    if !response.status().is_success() {
        anyhow::bail!(
            "download failed for season {season}: HTTP {}",
            response.status()
        );
    }

    let bytes = response.bytes().await.context("read response bytes")?;
    let kb = bytes.len() / 1024;
    println!("Downloading data-{season}... {kb} KB");

    // Extract tar.gz into dest/
    std::fs::create_dir_all(&dest)
        .with_context(|| format!("create {}", dest.display()))?;

    extract_tar_gz(&bytes, &dest)
        .with_context(|| format!("extract data-{season}.tar.gz"))?;

    println!("Installed season {season} → {}", dest.display());
    Ok(())
}

/// Unpack a tar.gz byte slice into `dest_dir`.
fn extract_tar_gz(bytes: &[u8], dest_dir: &std::path::Path) -> anyhow::Result<()> {
    use std::io::Read;

    // Decompress gzip.
    let mut gz_decoder = flate2_decoder(bytes)?;
    let mut tar_bytes = Vec::new();
    gz_decoder
        .read_to_end(&mut tar_bytes)
        .context("decompress gzip")?;

    // Unpack tar archive.
    let mut archive = tar_archive(&tar_bytes);
    archive
        .unpack(dest_dir)
        .context("unpack tar archive")?;

    Ok(())
}

// ── list ──────────────────────────────────────────────────────────────────────

fn run_list() -> anyhow::Result<()> {
    let seasons_dir = seasons_base_dir()?;

    if !seasons_dir.exists() {
        println!("No seasons installed. Use `icelines data install` to download.");
        return Ok(());
    }

    let mut found = false;
    println!("{:<12} {:<10} {:<8}", "Season", "Size", "Players");
    println!("{}", "─".repeat(34));

    // Iterate installed season directories in alphabetical order.
    let mut entries = std::fs::read_dir(&seasons_dir)
        .context("read seasons directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect::<Vec<_>>();
    entries.sort();

    for entry in entries {
        let season = entry
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();

        let size_kb = dir_size_kb(&entry);

        // Count players from bios.json if present.
        let player_count = bios_player_count(&entry);

        let players_str = match player_count {
            Some(n) => n.to_string(),
            None => "—".to_owned(),
        };

        println!("{:<12} {:<10} {:<8}", season, format!("{size_kb} KB"), players_str);
        found = true;
    }

    if !found {
        println!("No seasons installed. Use `icelines data install` to download.");
    }

    Ok(())
}

// ── remove ────────────────────────────────────────────────────────────────────

fn run_remove(season: &str) -> anyhow::Result<()> {
    let target = seasons_base_dir()?.join(season);

    if !target.exists() {
        anyhow::bail!("season {season} is not installed");
    }

    std::fs::remove_dir_all(&target)
        .with_context(|| format!("remove {}", target.display()))?;

    println!("Removed season {season}.");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn seasons_base_dir() -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".icelines").join("seasons"))
}

/// Recursively sum file sizes in a directory, returning kilobytes.
fn dir_size_kb(dir: &std::path::Path) -> u64 {
    walkdir(dir) / 1024
}

fn walkdir(dir: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                walkdir(&p)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

/// Read bios.json and return player count.
fn bios_player_count(season_dir: &std::path::Path) -> Option<usize> {
    let bios_path = season_dir.join("bios.json");
    let raw = std::fs::read(&bios_path).ok()?;
    let bios: Vec<serde_json::Value> = serde_json::from_slice(&raw).ok()?;
    Some(bios.len())
}

// ── thin wrappers so we can use std/miniz without a new dep ──────────────────
// We already depend on `reqwest` which pulls in flate2 transitively.
// Re-export the flate2 + tar types behind thin functions so the body stays
// dependency-agnostic and easy to swap.

fn flate2_decoder(bytes: &[u8]) -> anyhow::Result<flate2::read::GzDecoder<&[u8]>> {
    Ok(flate2::read::GzDecoder::new(bytes))
}

fn tar_archive(bytes: &[u8]) -> tar::Archive<&[u8]> {
    tar::Archive::new(bytes)
}
