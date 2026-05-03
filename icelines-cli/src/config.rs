use serde::Deserialize;
use std::path::PathBuf;

// ── Raw TOML shape ────────────────────────────────────────────────────────────

/// The shape read directly from `~/.icelines/config.toml`.
/// All fields are optional so that a partial file (or no file at all) still works.
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct RawConfig {
    csv_path: Option<String>,
    cache_dir: Option<String>,
    season: Option<u32>,
    /// Phase 8f.1: when explicitly `false`, live NHL API fetches are
    /// disabled. Absent or `true` keeps live feeds on (the default).
    live: Option<bool>,
    /// Phase 8j: when explicitly `true`, the experimental proof-compiled
    /// dashboard panels render in supporting screens (currently the player
    /// detail card). Absent or `false` keeps them hidden — feature is
    /// off by default while the integration matures.
    dashboards: Option<bool>,
}

// ── Public Config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// Path to a Yahoo CSV export (optional).
    pub csv_path: Option<PathBuf>,
    /// Directory used to cache API responses.
    pub cache_dir: PathBuf,
    /// Season identifier (8-digit YYYYZZZZ, e.g. 20252026).
    pub season: Option<u32>,
    /// Phase 8f.1: explicit value of the `live` config key, if set.
    /// `None` means "respect the defaults / env var / CLI flag".
    pub live: Option<bool>,
    /// Phase 8j: explicit value of the `dashboards` config key, if set.
    /// `None` defers to env / CLI / default(off).
    pub dashboards: Option<bool>,
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
            toml::from_str(&text).map_err(|e| {
                anyhow::anyhow!("invalid config at {}: {}", config_path.display(), e)
            })?
        } else {
            RawConfig::default()
        };

        let default_cache = home.join(".icelines").join("cache");

        Ok(Config {
            csv_path: raw.csv_path.map(PathBuf::from),
            cache_dir: raw.cache_dir.map(PathBuf::from).unwrap_or(default_cache),
            season: raw.season,
            live: raw.live,
            dashboards: raw.dashboards,
        })
    }

    /// Return the season as an 8-digit string (e.g. "20252026").
    pub fn season_str(&self) -> String {
        self.season
            .unwrap_or(icelines_core::CURRENT_SEASON)
            .to_string()
    }

    /// Directory for named snapshots: ~/.icelines/snapshots/
    pub fn snapshot_dir(&self) -> std::path::PathBuf {
        self.cache_dir
            .parent()
            .unwrap_or(&self.cache_dir)
            .join("snapshots")
    }
}

// ── Live-feeds toggle (Phase 8f.1) ────────────────────────────────────────────

/// Process-wide flag for whether live NHL API fetches are allowed.
/// `Some(true)` = on, `Some(false)` = suppressed, `None` = unset (treat as on).
/// Set once at process startup by `init_live_feeds`; queried by
/// `live_feeds_enabled()` from any caller.
static LIVE_FEEDS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Resolve and stash the live-feeds boolean. Precedence (highest first):
/// 1. `--no-live` CLI flag (wins if set)
/// 2. `ICELINES_NO_LIVE` env var (any non-empty / non-zero value disables)
/// 3. `live = false` in `~/.icelines/config.toml`
/// 4. Default: live feeds ON
///
/// Idempotent — first call wins, subsequent calls are no-ops. Safe to call
/// from tests via `init_live_feeds_for_test`.
pub fn init_live_feeds(cli_no_live: bool, cfg: &Config) {
    let env_disable = std::env::var_os("ICELINES_NO_LIVE")
        .map(|v| {
            let s = v.to_string_lossy();
            !s.is_empty() && s != "0" && !s.eq_ignore_ascii_case("false")
        })
        .unwrap_or(false);
    let resolved = if cli_no_live || env_disable {
        false
    } else {
        cfg.live.unwrap_or(true)
    };
    let _ = LIVE_FEEDS.set(resolved);
}

/// True iff live NHL API fetches are permitted.
/// Defaults to `true` if `init_live_feeds` was never called (e.g. unit tests
/// that bypass the CLI entry point — they shouldn't be hitting the network
/// anyway, but the default keeps existing tests' behavior unchanged).
pub fn live_feeds_enabled() -> bool {
    *LIVE_FEEDS.get().unwrap_or(&true)
}

/// Pure precedence resolver — exposed for tests so they can verify the
/// rule without poking the global OnceLock. `init_live_feeds` inlines
/// the same logic against env/config inputs.
#[allow(dead_code)] // production path uses init_live_feeds; tests use this.
pub fn resolve_live(cli_no_live: bool, env_no_live: bool, config_live: Option<bool>) -> bool {
    if cli_no_live {
        return false;
    }
    if env_no_live {
        return false;
    }
    if let Some(c) = config_live {
        return c;
    }
    true
}

// ── Dashboards feature flag (Phase 8j) ────────────────────────────────────────

/// Process-wide flag for whether experimental dashboard panels render.
/// Off by default — opt in via `--dashboards`, `ICELINES_DASHBOARDS=1`, or
/// `dashboards = true` in `~/.icelines/config.toml`.
static DASHBOARDS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Resolve and stash the dashboards boolean. Precedence (highest first):
/// 1. `--no-dashboards` CLI flag (force-off)
/// 2. `ICELINES_DASHBOARDS=0` env var (force-off)
/// 3. `dashboards = false` in `~/.icelines/config.toml`
/// 4. Default: dashboards ON.
///
/// Idempotent — first call wins, subsequent calls are no-ops.
pub fn init_dashboards(cli_no_dashboards: bool, cfg: &Config) {
    let env_off = std::env::var_os("ICELINES_DASHBOARDS")
        .map(|v| {
            let s = v.to_string_lossy();
            // Only an explicit falsy value disables; empty / unset / non-zero leaves it on.
            s == "0" || s.eq_ignore_ascii_case("false") || s.eq_ignore_ascii_case("off")
        })
        .unwrap_or(false);
    let resolved = if cli_no_dashboards || env_off {
        false
    } else {
        cfg.dashboards.unwrap_or(true)
    };
    let _ = DASHBOARDS.set(resolved);
}

/// True iff dashboard panels should render. Returns `true` when
/// `init_dashboards` was never called — matches the new on-by-default
/// behavior so tests that bypass the CLI entry point see panels by
/// default (most TUI render tests work with explicit fixture setups
/// that don't depend on this).
pub fn dashboards_enabled() -> bool {
    *DASHBOARDS.get().unwrap_or(&true)
}

/// Pure precedence resolver for tests — same rule as `init_dashboards`
/// but free of any global state. Inputs: `cli_no` is `--no-dashboards`,
/// `env_off` is `ICELINES_DASHBOARDS=0|false|off`, `config` is the
/// optional `dashboards` config-file key.
#[allow(dead_code)]
pub fn resolve_dashboards(cli_no: bool, env_off: bool, config: Option<bool>) -> bool {
    if cli_no {
        return false;
    }
    if env_off {
        return false;
    }
    if let Some(c) = config {
        return c;
    }
    true
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

// ── Tests for live-feeds precedence (Phase 8f.1) ──────────────────────────────

#[cfg(test)]
mod live_feeds_tests {
    use super::*;

    #[test]
    fn l0_resolve_live_default_is_on() {
        assert!(resolve_live(false, false, None));
    }

    #[test]
    fn l0_resolve_live_cli_flag_wins_over_everything() {
        // CLI true → live disabled regardless of env / config
        assert!(!resolve_live(true, false, None));
        assert!(!resolve_live(true, false, Some(true)));
        assert!(!resolve_live(true, true, Some(true)));
    }

    #[test]
    fn l0_resolve_live_env_wins_over_config() {
        // env disable beats config = true
        assert!(!resolve_live(false, true, Some(true)));
        // env unset → config wins
        assert!(resolve_live(false, false, Some(true)));
        assert!(!resolve_live(false, false, Some(false)));
    }

    #[test]
    fn l0_resolve_live_config_only_when_no_higher_signal() {
        assert!(!resolve_live(false, false, Some(false)));
        assert!(resolve_live(false, false, Some(true)));
        // No config → default true
        assert!(resolve_live(false, false, None));
    }

    // ── Phase 8j: dashboards flag precedence (now opt-OUT — on by default) ──

    #[test]
    fn l0_resolve_dashboards_default_is_on() {
        // With no CLI flag, no env var, and no config — dashboards render.
        assert!(resolve_dashboards(false, false, None));
    }

    #[test]
    fn l0_resolve_dashboards_cli_no_wins_over_everything() {
        // `--no-dashboards` forces off no matter what env/config say.
        assert!(!resolve_dashboards(true, false, None));
        assert!(!resolve_dashboards(true, false, Some(true)));
        assert!(!resolve_dashboards(true, true, Some(true)));
    }

    #[test]
    fn l0_resolve_dashboards_env_off_wins_over_config() {
        // `ICELINES_DASHBOARDS=0` disables even with `dashboards = true`.
        assert!(!resolve_dashboards(false, true, Some(true)));
        // env unset → config wins.
        assert!(!resolve_dashboards(false, false, Some(false)));
        assert!(resolve_dashboards(false, false, Some(true)));
    }

    #[test]
    fn l0_resolve_dashboards_config_only_when_no_higher_signal() {
        assert!(resolve_dashboards(false, false, Some(true)));
        assert!(!resolve_dashboards(false, false, Some(false)));
        // No config → default true.
        assert!(resolve_dashboards(false, false, None));
    }
}
