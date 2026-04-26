use std::path::PathBuf;
use serde::Deserialize;

// ── Raw TOML shape ────────────────────────────────────────────────────────────

/// The shape read directly from `~/.icelines/config.toml`.
/// All fields are optional so that a partial file (or no file at all) still works.
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct RawConfig {
    csv_path:  Option<String>,
    cache_dir: Option<String>,
    season:    Option<u32>,
}

// ── Public Config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// Path to a Yahoo CSV export (optional).
    pub csv_path:  Option<PathBuf>,
    /// Directory used to cache API responses.
    pub cache_dir: PathBuf,
    /// Season identifier (8-digit YYYYZZZZ, e.g. 20252026).
    pub season:    Option<u32>,
}

impl Config {
    /// Load configuration from `~/.icelines/config.toml`.
    ///
    /// If the file does not exist the function returns a default `Config`
    /// (not an error).  If the file exists but cannot be parsed, the error
    /// is propagated so the user sees a useful message.
    pub fn load() -> anyhow::Result<Self> {
        let home = home_dir()?;
        let config_path = home.join(".icelines").join("config.toml");

        let raw: RawConfig = if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {}", config_path.display(), e))?;
            toml::from_str(&text)
                .map_err(|e| anyhow::anyhow!("invalid config at {}: {}", config_path.display(), e))?
        } else {
            RawConfig::default()
        };

        let default_cache = home.join(".icelines").join("cache");

        Ok(Config {
            csv_path:  raw.csv_path.map(PathBuf::from),
            cache_dir: raw.cache_dir.map(PathBuf::from).unwrap_or(default_cache),
            season:    raw.season,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the current user's home directory.
fn home_dir() -> anyhow::Result<PathBuf> {
    // `std::env::home_dir` is deprecated (and unreliable on Windows in old Rust).
    // Use the HOME / USERPROFILE env vars directly — portable and straightforward.
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory; set HOME or USERPROFILE"))
}
