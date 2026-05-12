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
use icelines_core::{
    ConfigEntryInput, ConfigMutationIntent, ConfigView, ViewContext, ViewWindow, CURRENT_SEASON,
};

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
            let view = config_view(&cfg, Some(key.to_string()));
            let value = view
                .rows
                .iter()
                .find(|row| row.selected)
                .map(|row| row.value.as_str())
                .unwrap_or(value.as_str());
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
    let intent =
        ConfigMutationIntent::set(key, value).map_err(|message| anyhow::anyhow!(message))?;
    let mut cfg = Config::load().context("load config")?;
    if let Err(e) = cfg.set_key(&intent.key, intent.value.as_deref().unwrap_or_default()) {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
    cfg.save_sync().context("persist config")?;
    let _result = intent.result_view(default_config_context(), true);
    println!("set {} = {}", intent.key, value);
    Ok(())
}

fn run_list() -> Result<()> {
    let cfg = Config::load().context("load config")?;
    let view = config_view(&cfg, None);
    for row in &view.rows {
        println!("{} = {}", row.key, row.value);
    }
    Ok(())
}

fn run_reset(key: &str) -> Result<()> {
    let intent = ConfigMutationIntent::reset(key).map_err(|message| anyhow::anyhow!(message))?;
    let mut cfg = Config::load().context("load config")?;
    if let Err(e) = cfg.reset_key(&intent.key) {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
    cfg.save_sync().context("persist config")?;
    let _result = intent.result_view(default_config_context(), true);
    println!("reset {}", intent.key);
    Ok(())
}

fn config_view(cfg: &Config, selected_key: Option<String>) -> ConfigView {
    ConfigView::from_entries(
        default_config_context(),
        cfg.list_keys()
            .into_iter()
            .map(|(key, value)| ConfigEntryInput { key, value })
            .collect(),
        selected_key,
    )
}

fn default_config_context() -> ViewContext {
    ViewContext::new(ViewWindow::new(
        icelines_core::model::Season(CURRENT_SEASON),
        icelines_core::season_stats::SeasonType::Regular,
    ))
}
