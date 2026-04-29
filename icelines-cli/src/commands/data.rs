//! `icelines data` — download and manage additional season data bundles.

use crate::cli::DataSubcommand;
use anyhow::Context;

/// All seasons with published GitHub Releases, newest first.
/// The first 5 are also bundled in the binary; the rest require `data install`.
const AVAILABLE_SEASONS: &[&str] = &[
    // Current + recent (bundled in binary)
    "20252026", "20242025", "20232024", "20222023", "20212022",
    // Salary-cap era history 2005-06 → 2020-21
    "20202021", "20192020", "20182019", "20172018", "20162017",
    "20152016", "20142015", "20132014", "20122013", "20112012",
    "20102011", "20092010", "20082009", "20072008", "20062007",
    "20052006",
    // Pre-cap era 2000-01 → 2003-04
    "20032004", "20022003", "20012002", "20002001",
    // Pre-cap / Gretzky-trade era 1987-88 → 1999-2000
    "19992000", "19981999", "19971998", "19961997", "19951996",
    "19941995", "19931994", "19921993", "19911992", "19901991",
    "19891990", "19881989", "19871988",
    // Note: 20042005 omitted — full lockout, no games played
];

const RELEASE_URL_TEMPLATE: &str =
    "https://github.com/giodl73-repo/ICELINES/releases/download/data-{SEASON}/data-{SEASON}.tar.gz";

pub async fn run(cmd: DataSubcommand) -> anyhow::Result<()> {
    match cmd {
        DataSubcommand::Install { seasons, season, force } => {
            run_install(seasons, season, force).await
        }
        DataSubcommand::List => run_list(),
        DataSubcommand::Remove { season } => run_remove(&season),
        DataSubcommand::Verify { season, all } => run_verify(season.as_deref(), all),
    }
}

// ── install ───────────────────────────────────────────────────────────────────

async fn run_install(seasons: u8, season: Option<String>, force: bool) -> anyhow::Result<()> {
    let seasons_dir = seasons_base_dir()?;
    std::fs::create_dir_all(&seasons_dir)
        .with_context(|| format!("create {}", seasons_dir.display()))?;

    // Build the list of seasons to install.
    let to_install: Vec<&str> = if let Some(ref s) = season {
        if s == "20042005" {
            println!(
                "Season 20042005 was the NHL lockout — no games were played, no data exists."
            );
            return Ok(());
        }
        if !AVAILABLE_SEASONS.contains(&s.as_str()) {
            println!(
                "Season {s} is not available as a pre-built bundle.\n  \
                 To fetch it yourself: `icelines fetch stats --season {s}`\n  \
                 Available: 19871988–20252026 (excluding 20042005 lockout)"
            );
            return Ok(());
        }
        vec![s.as_str()]
    } else {
        AVAILABLE_SEASONS
            .iter()
            .take(seasons as usize)
            .copied()
            .collect()
    };

    for s in to_install {
        install_season(&seasons_dir, s, force).await?;
    }

    Ok(())
}

/// Public entry point for TUI-triggered installs.
pub async fn install_season_tui(season: &str) -> anyhow::Result<u64> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .context("cannot determine home directory")?;
    let seasons_dir = std::path::Path::new(&home).join(".icelines").join("seasons");
    std::fs::create_dir_all(&seasons_dir)?;
    let dest = seasons_dir.join(season);
    let bundle_dir = dest.join(format!("bundle-{season}"));
    if bundle_dir.join("bios.json").exists() {
        anyhow::bail!("already installed");
    }
    let url = RELEASE_URL_TEMPLATE.replace("{SEASON}", season);
    let client = reqwest::Client::builder().user_agent("icelines-cli").build()?;
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }
    let bytes = response.bytes().await?;
    let kb = bytes.len() as u64 / 1024;
    std::fs::create_dir_all(&dest)?;
    extract_tar_gz(&bytes, &dest)?;
    write_bundle_manifest(&dest, season)
        .with_context(|| format!("writing manifest for {season}"))?;
    Ok(kb)
}

async fn install_season(
    seasons_dir: &std::path::Path,
    season: &str,
    force: bool,
) -> anyhow::Result<()> {
    let dest = seasons_dir.join(season);

    // Installed seasons are extracted under dest/bundle-{season}/ by the tar.gz layout.
    let bundle_dir = dest.join(format!("bundle-{season}"));

    // Skip if already installed and not forcing a refresh.
    if !force && bundle_dir.join("bios.json").exists() && bundle_dir.join("stats.json").exists() {
        println!("Season {season} already installed (use --force to re-download).");
        return Ok(());
    }

    let url = RELEASE_URL_TEMPLATE.replace("{SEASON}", season);

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

    // Phase 8f.8: write a SHA-256 manifest covering each JSON file in the
    // extracted bundle so `data verify` can flag corruption / tampering.
    write_bundle_manifest(&dest, season)
        .with_context(|| format!("writing manifest for {season}"))?;

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
    // Installed bundles extract to bundle-{season}/bios.json
    let season_name = season_dir.file_name()?.to_string_lossy().to_string();
    let bundle = season_dir.join(format!("bundle-{season_name}")).join("bios.json");
    let direct = season_dir.join("bios.json");
    let bios_path = if bundle.exists() { bundle } else { direct };
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

// ── Phase 8f.8: SHA-256 verification of installed bundles ────────────────────

/// Format a byte slice as lowercase hex. Avoids pulling in the `hex` crate
/// for a single use.
fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Files we hash for a season bundle. Optional files (playoffs.json) are
/// skipped silently when absent so older bundles still verify cleanly.
const HASHED_FILES: &[&str] = &["bios.json", "stats.json", "playoffs.json"];

/// Manifest written next to the bundle's JSON files at install time.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BundleManifest {
    season:     String,
    /// `{filename: hex-sha256}` for each file we hash.
    sha256:     std::collections::BTreeMap<String, String>,
    /// Schema version. Bump when the manifest format changes.
    #[serde(default = "default_manifest_version")]
    version:    u32,
    /// ISO-8601 timestamp of when the manifest was written.
    written_at: String,
}

fn default_manifest_version() -> u32 { 1 }

/// Resolve the directory holding the bundle's JSON files. Installed tarballs
/// extract to either `dest/bundle-{season}/` or directly into `dest/`,
/// depending on the tarball layout. We pick whichever has bios.json.
pub(crate) fn bundle_files_dir(dest: &std::path::Path, season: &str) -> std::path::PathBuf {
    let nested = dest.join(format!("bundle-{season}"));
    if nested.join("bios.json").exists() { nested } else { dest.to_owned() }
}

/// Compute SHA-256 hex of a single file. Returns `Ok(None)` when the file
/// does not exist (so optional files can be skipped without an error path).
fn file_sha256(path: &std::path::Path) -> anyhow::Result<Option<String>> {
    use sha2::{Digest, Sha256};
    if !path.exists() { return Ok(None); }
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(Some(to_hex(&hasher.finalize())))
}

/// Walk the bundle's JSON files, compute hashes, and write `manifest.json`.
fn write_bundle_manifest(dest: &std::path::Path, season: &str) -> anyhow::Result<()> {
    let dir = bundle_files_dir(dest, season);
    let mut sha256 = std::collections::BTreeMap::new();
    for f in HASHED_FILES {
        if let Some(hex) = file_sha256(&dir.join(f))? {
            sha256.insert((*f).to_owned(), hex);
        }
    }
    if sha256.is_empty() {
        anyhow::bail!("nothing to hash in {} — bundle layout unexpected", dir.display());
    }
    let manifest = BundleManifest {
        season: season.to_owned(),
        sha256,
        version: default_manifest_version(),
        written_at: chrono::Utc::now().to_rfc3339(),
    };
    let json = serde_json::to_string_pretty(&manifest)
        .context("serialize manifest")?;
    let path = dir.join("manifest.json");
    std::fs::write(&path, json)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Verify hashes of one or all installed bundles. Errors loud for any
/// mismatched, missing, or unmanifested bundle.
fn run_verify(season: Option<&str>, all: bool) -> anyhow::Result<()> {
    if season.is_some() && all {
        anyhow::bail!("--all and a season name are mutually exclusive");
    }
    let seasons_dir = seasons_base_dir()?;
    if !seasons_dir.exists() {
        anyhow::bail!(
            "no installed seasons in {} — run `icelines data install` first",
            seasons_dir.display()
        );
    }

    // Collect the seasons to verify.
    let to_verify: Vec<String> = if all {
        std::fs::read_dir(&seasons_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    } else if let Some(s) = season {
        vec![s.to_owned()]
    } else {
        anyhow::bail!("specify a season or pass --all");
    };

    let mut had_error = false;
    for s in &to_verify {
        match verify_one(&seasons_dir, s) {
            Ok(VerifyOutcome::Ok { count }) => {
                println!("✓ {s}: {count} file(s) verified");
            }
            Ok(VerifyOutcome::Missing) => {
                println!("? {s}: no manifest.json — install with the current binary to generate one");
                if !all { had_error = true; }
            }
            Err(e) => {
                println!("✗ {s}: {e}");
                had_error = true;
            }
        }
    }
    if had_error {
        anyhow::bail!("verification failed");
    }
    Ok(())
}

enum VerifyOutcome { Ok { count: usize }, Missing }

/// Reread + rehash every file in the manifest, comparing against the recorded
/// hex digest. Pure function — exposed for tests via `verify_dir`.
pub(crate) fn verify_dir(dir: &std::path::Path) -> anyhow::Result<VerifyResult> {
    let manifest_path = dir.join("manifest.json");
    if !manifest_path.exists() {
        return Ok(VerifyResult::Missing);
    }
    let text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: BundleManifest = serde_json::from_str(&text)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;
    let mut mismatches = Vec::new();
    for (file, expected) in &manifest.sha256 {
        match file_sha256(&dir.join(file))? {
            None => mismatches.push(format!("{file}: file is missing")),
            Some(actual) if actual != *expected =>
                mismatches.push(format!("{file}: sha256 mismatch")),
            _ => {}
        }
    }
    Ok(VerifyResult::Ok {
        count: manifest.sha256.len(),
        mismatches,
    })
}

#[derive(Debug)]
pub(crate) enum VerifyResult {
    Missing,
    Ok { count: usize, mismatches: Vec<String> },
}

fn verify_one(seasons_dir: &std::path::Path, season: &str)
    -> anyhow::Result<VerifyOutcome>
{
    let dest = seasons_dir.join(season);
    if !dest.exists() {
        anyhow::bail!("season {season} is not installed");
    }
    let dir = bundle_files_dir(&dest, season);
    match verify_dir(&dir)? {
        VerifyResult::Missing => Ok(VerifyOutcome::Missing),
        VerifyResult::Ok { count, mismatches } if mismatches.is_empty() =>
            Ok(VerifyOutcome::Ok { count }),
        VerifyResult::Ok { mismatches, .. } => {
            anyhow::bail!("{} mismatch(es): {}", mismatches.len(), mismatches.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_sha256(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(bytes);
        super::to_hex(&h.finalize())
    }

    #[test]
    fn l0_file_sha256_matches_known_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, b"hello world").unwrap();
        let expected = hex_sha256(b"hello world");
        let actual = file_sha256(&path).unwrap().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn l0_file_sha256_returns_none_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.txt");
        assert!(file_sha256(&path).unwrap().is_none());
    }

    #[test]
    fn l0_write_and_verify_roundtrip() {
        // Simulate an installed bundle: dir with bios.json + stats.json,
        // write manifest, verify_dir reports no mismatches.
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().to_owned();
        std::fs::write(dest.join("bios.json"),  b"[{\"x\":1}]").unwrap();
        std::fs::write(dest.join("stats.json"), b"[{\"y\":2}]").unwrap();
        write_bundle_manifest(&dest, "20242025").unwrap();
        let result = verify_dir(&dest).unwrap();
        match result {
            VerifyResult::Ok { count, mismatches } => {
                assert_eq!(count, 2);
                assert!(mismatches.is_empty(), "no mismatches expected, got: {mismatches:?}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn l0_verify_detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().to_owned();
        std::fs::write(dest.join("bios.json"),  b"[{\"x\":1}]").unwrap();
        std::fs::write(dest.join("stats.json"), b"[{\"y\":2}]").unwrap();
        write_bundle_manifest(&dest, "20242025").unwrap();
        // Tamper with one file after manifest.
        std::fs::write(dest.join("bios.json"), b"[{\"x\":999}]").unwrap();
        match verify_dir(&dest).unwrap() {
            VerifyResult::Ok { mismatches, .. } => {
                assert_eq!(mismatches.len(), 1, "exactly one file should mismatch");
                assert!(mismatches[0].contains("bios.json"),
                    "must name bios.json, got: {mismatches:?}");
                assert!(mismatches[0].contains("mismatch"),
                    "must say 'mismatch', got: {mismatches:?}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn l0_verify_detects_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().to_owned();
        std::fs::write(dest.join("bios.json"),  b"[]").unwrap();
        std::fs::write(dest.join("stats.json"), b"[]").unwrap();
        write_bundle_manifest(&dest, "20242025").unwrap();
        // Delete a file after the manifest is written.
        std::fs::remove_file(dest.join("stats.json")).unwrap();
        match verify_dir(&dest).unwrap() {
            VerifyResult::Ok { mismatches, .. } => {
                assert_eq!(mismatches.len(), 1);
                assert!(mismatches[0].contains("stats.json"));
                assert!(mismatches[0].contains("missing"),
                    "must say 'missing', got: {mismatches:?}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn l0_verify_no_manifest_returns_missing() {
        let dir = tempfile::tempdir().unwrap();
        // No manifest, no files.
        match verify_dir(dir.path()).unwrap() {
            VerifyResult::Missing => {} // ok
            other => panic!("expected Missing, got {other:?}"),
        }
    }
}
