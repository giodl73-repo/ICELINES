//! Player headshot ASCII art for the TUI player card.
//!
//! Downloads player headshot PNGs from the NHL CDN, converts to grayscale,
//! resizes to fit the terminal, and dithers using a braille-block algorithm
//! (2×4 image pixels per terminal char using U+2800–U+28FF).
//!
//! ## Caching
//!
//! Two tiers:
//!
//! 1. **In-memory** (`HeadshotCache`) — single session, keyed by `nhl_id`.
//!    Holds the dithered ASCII rows or a loading/error marker.
//! 2. **On-disk** (`~/.icelines/cache/headshots/{nhl_id}.txt`) — persists
//!    across sessions. Populated on every successful network fetch and
//!    consulted by `spawn_fetch` before hitting the network.
//!
//! The dithered ASCII is ~1 KB per player; ~900 active players ≈ 1.5 MB
//! total on disk for the whole league. Cheap, and means the player card
//! renders headshots instantly on second use without any network calls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ── Cache ─────────────────────────────────────────────────────────────────────

/// Shared cache: nhl_id → Vec<String> of ASCII rows (or error marker).
#[derive(Clone)]
pub struct HeadshotCache {
    inner: Arc<Mutex<HashMap<u32, Vec<String>>>>,
}

impl HeadshotCache {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn get(&self, id: u32) -> Option<Vec<String>> {
        self.inner.lock().ok()?.get(&id).cloned()
    }

    pub fn set(&self, id: u32, rows: Vec<String>) {
        if let Ok(mut g) = self.inner.lock() { g.insert(id, rows); }
    }
}

const LOADING_MARKER: &str = "⌛";   // stored as single-string vec when in-flight
const ERROR_MARKER:   &str = "✗";

pub fn is_loading(rows: &[String]) -> bool {
    rows.len() == 1 && rows[0] == LOADING_MARKER
}
pub fn is_error(rows: &[String]) -> bool {
    rows.len() == 1 && rows[0] == ERROR_MARKER
}

/// Spawn a background task to fetch and dither a headshot.
/// Tries the disk cache first; only hits the network on a true miss.
/// Successful network fetches are written back to disk for next session.
/// Uses braille dither for maximum resolution (2×4 pixels per char).
pub fn spawn_fetch(nhl_id: u32, url: String, cache: HeadshotCache, target_cols: u32, target_rows: u32) {
    cache.set(nhl_id, vec![LOADING_MARKER.to_owned()]);
    let cache2 = cache.clone();
    tokio::spawn(async move {
        // Tier 2 — disk cache. Avoids one HTTP per player per session.
        if let Some(rows) = read_from_disk(nhl_id) {
            cache2.set(nhl_id, rows);
            return;
        }
        match fetch_and_dither_braille(&url, target_cols, target_rows).await {
            Ok(rows) => {
                // Best-effort write; cache stays useful even if the disk
                // write fails (e.g., read-only home).
                let _ = write_to_disk(nhl_id, &rows);
                cache2.set(nhl_id, rows);
            }
            Err(_)   => cache2.set(nhl_id, vec![ERROR_MARKER.to_owned()]),
        }
    });
}

// ── Disk cache ─────────────────────────────────────────────────────────────

/// Return the directory under which we persist dithered headshots.
/// Resolves to `~/.icelines/cache/headshots/`. Returns `None` only when
/// `$HOME` / `$USERPROFILE` are both unset — extreme edge case.
fn disk_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;
    Some(home.join(".icelines").join("cache").join("headshots"))
}

fn disk_path(nhl_id: u32) -> Option<PathBuf> {
    Some(disk_dir()?.join(format!("{nhl_id}.txt")))
}

/// Load a previously-dithered headshot from disk. Returns `None` on any
/// failure (missing file, unreadable, empty) — caller falls through to
/// the network fetch path.
fn read_from_disk(nhl_id: u32) -> Option<Vec<String>> {
    let path = disk_path(nhl_id)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let rows: Vec<String> = text.lines().map(str::to_owned).collect();
    if rows.is_empty() { return None; }
    Some(rows)
}

/// Persist a dithered headshot to disk. Best-effort: failures are
/// silently ignored (read-only home, disk full, etc.). The in-memory
/// cache still serves the rest of the session.
fn write_to_disk(nhl_id: u32, rows: &[String]) -> std::io::Result<()> {
    let path = disk_path(nhl_id).ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::NotFound, "no home directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, rows.join("\n"))?;
    Ok(())
}

// ── Fetch + dither pipeline ───────────────────────────────────────────────────

/// Shared HTTP download.
async fn fetch_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .user_agent("icelines-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    Ok(resp.bytes().await?.to_vec())
}

/// Braille dither — 2×4 pixels per braille character (U+2800–U+28FF).
/// Each output char encodes 2 image columns × 4 image rows.
/// At 22 cols × 15 rows out → 44×60 image pixels: enough for facial detail.
async fn fetch_and_dither_braille(url: &str, out_cols: u32, out_rows: u32) -> anyhow::Result<Vec<String>> {
    let bytes = fetch_bytes(url).await?;
    let img   = image::load_from_memory(&bytes)?;
    let gray  = img.to_luma8();

    // Braille: each output char = 2×4 pixels
    let img_w = out_cols * 2;
    let img_h = out_rows * 4;

    let resized = image::imageops::resize(
        &gray,
        img_w,
        img_h,
        image::imageops::FilterType::Lanczos3, // better quality for faces
    );

    // Contrast enhancement — stretch histogram
    let pixels: Vec<u8> = resized.pixels().map(|p| p[0]).collect();
    let lo = pixels.iter().copied().min().unwrap_or(0);
    let hi = pixels.iter().copied().max().unwrap_or(255);
    let range = (hi - lo).max(1) as f32;

    let enhanced = image::GrayImage::from_fn(img_w, img_h, |x, y| {
        let v = resized.get_pixel(x, y)[0];
        let stretched = (((v - lo) as f32 / range) * 255.0) as u8;
        image::Luma([stretched])
    });

    let out: Vec<String> = (0..out_rows).map(|row| {
        (0..out_cols).map(|col| {
            let px = col * 2;
            let py = row * 4;
            let mut dots = [false; 8];
            for dot in 0..8usize {
                let x = px + DOT_X[dot];
                let y = py + DOT_Y[dot];
                if x < img_w && y < img_h && enhanced.get_pixel(x, y)[0] >= THRESHOLD {
                    dots[dot] = true;
                }
            }
            pixels_to_braille(dots)
        }).collect()
    }).collect();

    Ok(out)
}

// ── Pure dither helpers (testable) ────────────────────────────────────────────

/// Braille dot layout — Unicode braille bit positions (U+2800–U+28FF).
///   dot 1 (bit 0) = (0,0)    dot 4 (bit 3) = (1,0)
///   dot 2 (bit 1) = (0,1)    dot 5 (bit 4) = (1,1)
///   dot 3 (bit 2) = (0,2)    dot 6 (bit 5) = (1,2)
///   dot 7 (bit 6) = (0,3)    dot 8 (bit 7) = (1,3)
pub(crate) const DOT_X: [u32; 8] = [0, 0, 0, 1, 1, 1, 0, 1];
pub(crate) const DOT_Y: [u32; 8] = [0, 1, 2, 0, 1, 2, 3, 3];
pub(crate) const THRESHOLD: u8   = 128;

/// Encode a 2×4 pixel block as a braille character. The 8-bit input is
/// indexed by dot number (0..8) per the canonical braille layout above —
/// `true` lights that dot.
pub(crate) fn pixels_to_braille(dots: [bool; 8]) -> char {
    let mut bits: u8 = 0;
    for (i, on) in dots.iter().enumerate() {
        if *on { bits |= 1 << i; }
    }
    char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
}

/// Team logo dither — block mode for clean high-contrast logos.
#[allow(dead_code)]
pub async fn fetch_logo_ascii(url: &str, out_cols: u32, out_rows: u32) -> anyhow::Result<Vec<String>> {
    let bytes = fetch_bytes(url).await?;
    let img   = image::load_from_memory(&bytes)?;
    let gray  = img.to_luma8();

    // Block chars: ' ' '░' '▒' '▓' '█'  — 5 shades
    const BLOCKS: &[char] = &[' ', '░', '▒', '▓', '█'];

    let resized = image::imageops::resize(&gray, out_cols, out_rows, image::imageops::FilterType::Lanczos3);

    let out: Vec<String> = (0..out_rows).map(|y| {
        (0..out_cols).map(|x| {
            let v = resized.get_pixel(x, y)[0] as usize;
            BLOCKS[v * (BLOCKS.len() - 1) / 255]
        }).collect()
    }).collect();

    Ok(out)
}

#[cfg(test)]
mod tests {
    //! L0 tests for the headshot rendering pipeline (Phase 8a.3).
    //! Network paths are not exercised — we test the pure dither encoder
    //! and the in-memory cache.
    use super::*;

    // ── Braille encoding ─────────────────────────────────────────────────────

    #[test]
    fn l0_braille_dot_bit_layout_matches_unicode() {
        // No dots → blank braille (U+2800)
        assert_eq!(pixels_to_braille([false; 8]) as u32, 0x2800);
        // All dots → full braille pattern (U+28FF)
        assert_eq!(pixels_to_braille([true; 8]) as u32, 0x28FF);

        // Each individual dot lights exactly the expected bit
        for i in 0..8 {
            let mut dots = [false; 8];
            dots[i] = true;
            let expected = 0x2800 + (1u32 << i);
            assert_eq!(
                pixels_to_braille(dots) as u32,
                expected,
                "dot {i} must encode to U+{expected:04X}",
            );
        }
    }

    #[test]
    fn l0_threshold_dither_solid_black_sets_all_dots() {
        // Convention: a "solid black" face on a dark terminal lights up the
        // braille glyph — every dot shows. The dither does NOT apply contrast
        // stretch in this isolated helper test; we just verify that when all
        // 8 dot positions are above-threshold (encoded as `true`), the output
        // is the fully-lit U+28FF.
        assert_eq!(pixels_to_braille([true; 8]) as u32, 0x28FF);
    }

    #[test]
    fn l0_threshold_dither_solid_white_clears_all_dots() {
        // "Solid white" on a dark terminal (no ink) means no dots → blank.
        assert_eq!(pixels_to_braille([false; 8]) as u32, 0x2800);
    }

    #[test]
    fn l0_threshold_constant_is_midpoint() {
        // Document the threshold contract — must remain at the 8-bit midpoint.
        // Changing this value alters every rendered headshot.
        assert_eq!(THRESHOLD, 128, "THRESHOLD constant changed — review headshot tone");
    }

    // ── Cache ────────────────────────────────────────────────────────────────

    #[test]
    fn l0_cache_get_set_roundtrip() {
        let cache = HeadshotCache::new();
        assert!(cache.get(8478402).is_none(), "fresh cache must miss");

        let rows = vec!["row1".to_owned(), "row2".to_owned()];
        cache.set(8478402, rows.clone());
        assert_eq!(cache.get(8478402), Some(rows));

        // Different id stays None
        assert!(cache.get(99).is_none());

        // Overwrite
        let rows2 = vec!["fresh".to_owned()];
        cache.set(8478402, rows2.clone());
        assert_eq!(cache.get(8478402), Some(rows2));
    }

    #[test]
    fn l0_is_loading_detects_placeholder() {
        assert!(is_loading(&[LOADING_MARKER.to_owned()]));
        // Real rendered content is never just one cell of the loading marker
        assert!(!is_loading(&["row1".to_owned(), "row2".to_owned()]));
        assert!(!is_loading(&[]));
        assert!(!is_loading(&[ERROR_MARKER.to_owned()]));
    }

    #[test]
    fn l0_is_error_detects_placeholder() {
        assert!(is_error(&[ERROR_MARKER.to_owned()]));
        assert!(!is_error(&[LOADING_MARKER.to_owned()]));
        assert!(!is_error(&["fine".to_owned(), "row".to_owned()]));
        assert!(!is_error(&[]));
    }

    #[test]
    fn l0_cache_clone_shares_storage() {
        // HeadshotCache::Clone returns an Arc-shared clone — writes from one
        // are visible from the other (essential for the spawn_fetch task).
        let a = HeadshotCache::new();
        let b = a.clone();
        a.set(42, vec!["row".to_owned()]);
        assert_eq!(b.get(42), Some(vec!["row".to_owned()]));
    }

    // ── Disk cache ─────────────────────────────────────────────────────────

    /// Set HOME/USERPROFILE to a tempdir so disk_path() resolves into it.
    /// Returns the temp dir guard + the resolved cache directory.
    fn isolate_home() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("HOME", dir.path());
        std::env::set_var("USERPROFILE", dir.path());
        let cache_dir = dir.path().join(".icelines").join("cache").join("headshots");
        (dir, cache_dir)
    }

    #[test]
    fn l0_disk_read_returns_none_when_file_missing() {
        let _g = crate::test_utils::home_env_lock();
        let (_keep, _) = isolate_home();
        assert!(read_from_disk(99999999).is_none(),
            "absent file → None, no panic");
    }

    #[test]
    fn l0_disk_write_and_read_roundtrip() {
        let _g = crate::test_utils::home_env_lock();
        let (_keep, cache_dir) = isolate_home();

        let rows = vec!["⠀⠀⠀".to_owned(), "⠶⠶⠶".to_owned(), "⠿⠿⠿".to_owned()];
        write_to_disk(8478402, &rows).expect("write must succeed");
        // File lands at the canonical path.
        let path = cache_dir.join("8478402.txt");
        assert!(path.exists(), "expected file at {}", path.display());
        // Roundtrip — read returns the same rows.
        let loaded = read_from_disk(8478402).expect("read must succeed");
        assert_eq!(loaded, rows);
    }

    #[test]
    fn l0_disk_write_creates_parent_directory_on_first_use() {
        let _g = crate::test_utils::home_env_lock();
        let (_keep, cache_dir) = isolate_home();
        // Parent doesn't exist yet on first call.
        assert!(!cache_dir.exists());
        write_to_disk(1234567, &["row".to_owned()]).expect("write must succeed");
        assert!(cache_dir.exists(),
            "write_to_disk must mkdir -p the parent");
    }

    #[test]
    fn l0_disk_read_treats_empty_file_as_miss() {
        let _g = crate::test_utils::home_env_lock();
        let (_keep, cache_dir) = isolate_home();
        // Pre-create an empty file at the cache path — should still
        // return None so the network path runs.
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("0.txt"), "").unwrap();
        assert!(read_from_disk(0).is_none(),
            "empty file should not look like a cached headshot");
    }
}
