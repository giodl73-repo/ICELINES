//! Phase Foster.0.7 — `icelines config get/set/list/reset`.
//!
//! Thin wrapper over `crate::config::Config::{get_key, set_key,
//! list_keys, reset_key}`. The typed config schema does the heavy
//! lifting; this command surfaces it to the shell. Persistence goes
//! through `Config::save_sync` so non-sync keys in `~/.icelines/config.toml`
//! survive untouched.

use anyhow::{Context, Result};

use crate::cli::ConfigSubcommand;
use crate::config::Config;

pub async fn run(cmd: ConfigSubcommand) -> Result<()> {
    match cmd {
        ConfigSubcommand::Get { key } => run_get(&key),
        ConfigSubcommand::Set { key, value } => run_set(&key, &value),
        ConfigSubcommand::List => run_list(),
        ConfigSubcommand::Reset { key } => run_reset(&key),
    }
}

fn run_get(key: &str) -> Result<()> {
    let cfg = Config::load().context("load config")?;
    match cfg.get_key(key) {
        Ok(value) => {
            println!("{value}");
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
}

fn run_set(key: &str, value: &str) -> Result<()> {
    let mut cfg = Config::load().context("load config")?;
    if let Err(e) = cfg.set_key(key, value) {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
    cfg.save_sync().context("persist config")?;
    println!("set {key} = {value}");
    Ok(())
}

fn run_list() -> Result<()> {
    let cfg = Config::load().context("load config")?;
    for (k, v) in cfg.list_keys() {
        println!("{k} = {v}");
    }
    Ok(())
}

fn run_reset(key: &str) -> Result<()> {
    let mut cfg = Config::load().context("load config")?;
    if let Err(e) = cfg.reset_key(key) {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
    cfg.save_sync().context("persist config")?;
    println!("reset {key}");
    Ok(())
}
