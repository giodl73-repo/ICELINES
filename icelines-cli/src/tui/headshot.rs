//! Player headshot ASCII art for the TUI player card.
//!
//! Downloads player headshot PNGs from the NHL CDN, converts to grayscale,
//! resizes to fit the terminal, and dithers using proof's half-block algorithm
//! (2 image rows per terminal row using ▀▄█ Unicode block characters).

use std::collections::HashMap;
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
/// Uses braille dither for maximum resolution (2×4 pixels per char).
pub fn spawn_fetch(nhl_id: u32, url: String, cache: HeadshotCache, target_cols: u32, target_rows: u32) {
    cache.set(nhl_id, vec![LOADING_MARKER.to_owned()]);
    let cache2 = cache.clone();
    tokio::spawn(async move {
        match fetch_and_dither_braille(&url, target_cols, target_rows).await {
            Ok(rows) => cache2.set(nhl_id, rows),
            Err(_)   => cache2.set(nhl_id, vec![ERROR_MARKER.to_owned()]),
        }
    });
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

    // Braille dot layout (Unicode braille bit positions):
    //   dot 1 (bit 0) = (0,0)    dot 4 (bit 3) = (1,0)
    //   dot 2 (bit 1) = (0,1)    dot 5 (bit 4) = (1,1)
    //   dot 3 (bit 2) = (0,2)    dot 6 (bit 5) = (1,2)
    //   dot 7 (bit 6) = (0,3)    dot 8 (bit 7) = (1,3)
    const DOT_X: [u32; 8] = [0, 0, 0, 1, 1, 1, 0, 1];
    const DOT_Y: [u32; 8] = [0, 1, 2, 0, 1, 2, 3, 3];
    const THRESHOLD: u8 = 128;

    let out: Vec<String> = (0..out_rows).map(|row| {
        (0..out_cols).map(|col| {
            let px = col * 2;
            let py = row * 4;
            let mut bits: u8 = 0;
            for dot in 0..8u8 {
                let x = px + DOT_X[dot as usize];
                let y = py + DOT_Y[dot as usize];
                if x < img_w && y < img_h && enhanced.get_pixel(x, y)[0] >= THRESHOLD {
                    bits |= 1 << dot;
                }
            }
            char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
        }).collect()
    }).collect();

    Ok(out)
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
