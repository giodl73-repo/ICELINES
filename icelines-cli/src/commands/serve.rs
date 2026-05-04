//! `icelines serve` — boot the web dashboard. Phase King Clancy King.1.5.
//!
//! Constructs a `WebState` (active-season label from the user's
//! `~/.icelines/config.toml`), mounts the `icelines-web` router, binds
//! a TCP socket, optionally launches the user's browser, and runs
//! until Ctrl-C.
//!
//! ## Behavior contract (from spec "Migration mechanics")
//!
//! 1. Print `→ http://...` to stdout BEFORE attempting to open a
//!    browser. The user always has the URL even if the open fails.
//! 2. Honor the `BROWSER` env var (the `open` crate does this for us).
//! 3. Swallow open errors silently — never fail `serve` because a
//!    browser couldn't launch (headless, WSL without a registered
//!    handler, SSH-tunnel use, etc.).
//! 4. `--no-open` skips steps 2-3.
//! 5. Bind error (port in use) → exit 1 with a clear hint, NOT
//!    auto-bump (auto-bump silently changes the printed URL).
//! 6. `--bind 0.0.0.0`: print a `WARNING: LAN mode...` banner.

use std::net::SocketAddr;
use std::process;

use crate::config::Config;
use icelines_core::CURRENT_SEASON_STR;
use icelines_web::{router, WebConfig, WebState};

/// Entry point — `Commands::Serve` arm of `dispatch()` calls this.
pub async fn run(
    port: u16,
    bind: Option<String>,
    no_open: bool,
    no_cache: bool,
    cors_origin: Option<String>,
    cfg: &Config,
) -> anyhow::Result<()> {
    // King.1.5 doesn't yet implement --no-cache or --cors-origin
    // semantics; King.2 wires the response cache, King.1.6 the CORS
    // middleware. Document the no-op for users now so the flag isn't
    // a silent lie.
    if no_cache {
        eprintln!("info: --no-cache acknowledged but inert until King.2 ships the response cache.");
    }
    if cors_origin.is_some() {
        eprintln!(
            "info: --cors-origin acknowledged but inert until King.1.6 ships the CORS middleware."
        );
    }

    let addr = resolve_bind(port, bind.as_deref())?;

    // Active-season label sourced from the user's config (with a
    // safe fallback to `CURRENT_SEASON`). King.6's PATCH
    // /api/v1/active-season will let users change this from the UI;
    // until then, edit `~/.icelines/config.toml` to switch.
    let active_season = cfg
        .season
        .map(|s| s.to_string())
        .unwrap_or_else(|| CURRENT_SEASON_STR.to_owned());
    // King.6 will introduce a per-user season-type setting; for
    // now everyone defaults to regular season.
    let web_config = WebConfig::new(active_season.clone(), "regular");

    // King.2.1 — load the active season's skater + goalie data into
    // the repo at boot. Same code path the CLI's query commands use.
    // First-paint cost is one-time at boot (~hundreds of ms for the
    // bundled current season); per-request handlers just take a brief
    // read lock.
    let active_season_u32: u32 = active_season.parse().map_err(|e| {
        anyhow::anyhow!("active season '{active_season}' is not a YYYYZZZZ id: {e}")
    })?;
    let store = icelines_fetch::snapshot::SnapshotStore::new(cfg.snapshot_dir());
    let load_outcome = icelines_fetch::stats_loader::load_into_repo(
        icelines_core::model::Season(active_season_u32),
        icelines_core::season_stats::SeasonType::Regular,
        &store,
    );
    let repo = match load_outcome {
        Ok(o) => {
            let n_identities = o.repo.iter_identities().count();
            println!("  loaded {n_identities} player identities for season {active_season}");
            o.repo
        }
        Err(e) => {
            eprintln!("warn: failed to load season {active_season} into repo: {e}");
            eprintln!("      /leaders will show an empty table. To populate, run:");
            eprintln!("        icelines fetch all  (or `icelines fetch stats`)");
            icelines_core::stats_repository::StatsRepository::new()
        }
    };

    let state = WebState {
        repo: std::sync::Arc::new(tokio::sync::RwLock::new(repo)),
        config: std::sync::Arc::new(tokio::sync::RwLock::new(web_config.clone())),
    };

    let app = router(state);

    // Bind. Fail loud on AddrInUse — auto-bump silently changes the
    // printed URL and breaks scripts.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("error: port {} already in use", addr.port());
            eprintln!(
                "hint:  try '--port {}' or stop the process holding {}",
                addr.port() + 1,
                addr.port()
            );
            process::exit(1);
        }
        Err(e) => {
            eprintln!("error: failed to bind {addr}: {e}");
            process::exit(1);
        }
    };

    let url = format!("http://{addr}/");

    // 1. Print URL FIRST — before any browser-open attempt — so users
    //    always have the URL even if open fails.
    println!("→ {url}  (active season: {})", web_config.active_label);
    println!("  Ctrl-C to stop.");

    // 6. LAN-mode security banner.
    if addr.ip() != std::net::IpAddr::from([127, 0, 0, 1]) {
        eprintln!();
        eprintln!("WARNING: LAN mode — no auth, no TLS.");
        eprintln!("         Anyone on your network can read your data.");
        eprintln!("         Bind to 127.0.0.1 (default) for localhost-only access.");
        eprintln!();
    }

    // 2-4. Auto-open browser unless --no-open.
    if !no_open {
        // open::that honors $BROWSER on Linux; uses Launch Services
        // on macOS; uses ShellExecute on Windows. Errors are NOT
        // failures of `serve` — print a brief note and continue.
        if let Err(e) = open::that(&url) {
            eprintln!("info: browser auto-open failed ({e}). Open the URL above manually.");
        }
    }

    // Run forever (until Ctrl-C signals shutdown).
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Resolve `--port` + `--bind` into a final SocketAddr per the spec's
/// "--bind vs --port precedence" rules.
fn resolve_bind(port: u16, bind: Option<&str>) -> anyhow::Result<SocketAddr> {
    use std::str::FromStr;
    match bind {
        // No --bind: 127.0.0.1 + --port
        None => Ok(SocketAddr::new([127, 0, 0, 1].into(), port)),
        // --bind ADDR (no port): use --port
        Some(addr) if !addr.contains(':') => {
            let ip = std::net::IpAddr::from_str(addr)
                .map_err(|e| anyhow::anyhow!("invalid --bind address {addr:?}: {e}"))?;
            Ok(SocketAddr::new(ip, port))
        }
        // --bind ADDR:PORT
        Some(addr) => {
            let sa = SocketAddr::from_str(addr)
                .map_err(|e| anyhow::anyhow!("invalid --bind {addr:?}: {e}"))?;
            // If both --bind:PORT and --port were specified with
            // different ports, --bind wins (per spec) but warn the
            // user so they don't get a surprise.
            if port != 8000 && sa.port() != port {
                eprintln!(
                    "warn: --bind {addr} overrides --port {port}; serving on {}",
                    sa.port()
                );
            }
            Ok(sa)
        }
    }
}

/// Future-proof shutdown handler — listen for Ctrl-C only today;
/// King.10 adds SIGTERM for systemd-style deployments.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("\n→ shutdown signal received, draining connections...");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// l0_resolve_bind_default
    /// — No --bind, default port → 127.0.0.1:8000.
    #[test]
    fn l0_resolve_bind_default() {
        let addr = resolve_bind(8000, None).unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:8000");
    }

    /// l0_resolve_bind_custom_port
    /// — --port 9000 only → 127.0.0.1:9000.
    #[test]
    fn l0_resolve_bind_custom_port() {
        let addr = resolve_bind(9000, None).unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:9000");
    }

    /// l0_resolve_bind_addr_only_uses_port_flag
    /// — --bind 0.0.0.0 (no port) → uses --port.
    #[test]
    fn l0_resolve_bind_addr_only_uses_port_flag() {
        let addr = resolve_bind(8000, Some("0.0.0.0")).unwrap();
        assert_eq!(addr.to_string(), "0.0.0.0:8000");
    }

    /// l0_resolve_bind_addr_with_port
    /// — --bind 0.0.0.0:9999 → uses that exact port.
    #[test]
    fn l0_resolve_bind_addr_with_port() {
        let addr = resolve_bind(8000, Some("0.0.0.0:9999")).unwrap();
        assert_eq!(addr.to_string(), "0.0.0.0:9999");
    }

    /// l0_resolve_bind_invalid_address_errors
    /// — bad address rejected with a useful error (not panic).
    #[test]
    fn l0_resolve_bind_invalid_address_errors() {
        let err = resolve_bind(8000, Some("not-an-ip")).unwrap_err();
        assert!(err.to_string().contains("invalid --bind"));
    }
}
