use anyhow::{bail, Context};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{PlayerStreaksView, ViewContext, ViewWindow, CURRENT_SEASON};

use crate::commands::output::Format;
use crate::config::Config;

pub async fn run(
    player: String,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let format = Format::resolve(csv, json)?;
    let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(&player) else {
        bail!("No player matched '{player}'");
    };
    let cfg = Config::load()?;
    let data_root = cfg
        .cache_dir
        .parent()
        .unwrap_or(&cfg.cache_dir)
        .join("data");
    let store = icelines_fetch::datastore::DataStore::open(&data_root)?;
    let lines = icelines_fetch::streaks_provider::load_player_game_lines(&store, pid);
    let (shot_lines, play_by_play_source_loaded) =
        icelines_fetch::streaks_provider::load_player_shot_lines(&store, pid);
    let player_name = lines
        .first()
        .map(|line| line.player_name.clone())
        .or_else(|| shot_lines.first().map(|line| line.player_name.clone()))
        .unwrap_or_else(|| player.clone());
    let context = ViewContext::new(ViewWindow::new(Season(CURRENT_SEASON), SeasonType::Regular));
    let view = PlayerStreaksView::from_game_and_shot_lines(
        context,
        pid,
        player_name,
        &lines,
        &shot_lines,
        play_by_play_source_loaded,
    );

    if format == Format::Json {
        let s = serde_json::to_string_pretty(&view)?;
        match out {
            Some(path) => std::fs::write(&path, format!("{s}\n"))
                .with_context(|| format!("writing streaks to {}", path.display()))?,
            None => println!("{s}"),
        }
        return Ok(());
    }

    let headers = ["metric", "current", "status", "longest", "start", "end"];
    let rows = view
        .rows
        .iter()
        .map(|row| {
            vec![
                row.metric.clone(),
                row.current.to_string(),
                row.current_status.clone(),
                row.longest.to_string(),
                row.longest_start_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                row.longest_end_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();

    if view.games_loaded == 0 {
        let message = format!(
            "No cached boxscore/play-by-play game lines found for {player} ({pid}). Run `icelines fetch boxscore --date YYYY-MM-DD` and `icelines fetch play-by-play --date YYYY-MM-DD` to populate streak inputs."
        );
        match out {
            Some(path) => std::fs::write(&path, format!("{message}\n"))
                .with_context(|| format!("writing streaks to {}", path.display()))?,
            None => println!("{message}"),
        }
        return Ok(());
    }
    format.emit_to(&headers, &rows, out.as_deref())
}
