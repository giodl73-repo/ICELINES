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
pub fn spawn_fetch(nhl_id: u32, url: String, cache: HeadshotCache, target_cols: u32, target_rows: u32) {
    // Mark as in-flight
    cache.set(nhl_id, vec![LOADING_MARKER.to_owned()]);
    let cache2 = cache.clone();
    tokio::spawn(async move {
        match fetch_and_dither(&url, target_cols, target_rows).await {
            Ok(rows) => cache2.set(nhl_id, rows),
            Err(_)   => cache2.set(nhl_id, vec![ERROR_MARKER.to_owned()]),
        }
    });
}

// ── Fetch + dither pipeline ───────────────────────────────────────────────────

async fn fetch_and_dither(url: &str, cols: u32, rows: u32) -> anyhow::Result<Vec<String>> {
    // Download
    let client = reqwest::Client::builder()
        .user_agent("icelines-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    let bytes = resp.bytes().await?;

    // Decode → grayscale
    let img  = image::load_from_memory(&bytes)?;
    let gray = img.to_luma8();

    // Resize: each terminal row covers 2 image rows (half-block)
    let img_h = rows * 2;
    let resized = image::imageops::resize(
        &gray,
        cols,
        img_h,
        image::imageops::FilterType::Triangle,
    );

    // Half-block dither: 2 image rows → 1 output row using ▀▄█ ' '
    let out_rows: Vec<String> = (0..rows).map(|row| {
        let y_top = row * 2;
        let y_bot = row * 2 + 1;
        (0..cols).map(|x| {
            let top = resized.get_pixel(x, y_top)[0] >= 128;
            let bot = if y_bot < img_h { resized.get_pixel(x, y_bot)[0] >= 128 } else { false };
            match (top, bot) {
                (false, false) => ' ',
                (true,  false) => '▀',
                (false, true)  => '▄',
                (true,  true)  => '█',
            }
        }).collect()
    }).collect();

    Ok(out_rows)
}
