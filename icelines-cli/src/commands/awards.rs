use anyhow::{bail, Context};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{ViewContext, ViewWindow, CURRENT_SEASON};

use crate::commands::output::Format;

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
    let context = ViewContext::new(ViewWindow::new(Season(CURRENT_SEASON), SeasonType::Regular));
    let view = if crate::config::live_feeds_enabled() {
        let view = icelines_fetch::nhl_api::NhlApiClient::production()
            .fetch_player_awards(pid, &player, context)
            .await
            .with_context(|| format!("fetch NHL awards for {player} ({pid})"))?;
        icelines_fetch::career_landing::save_local_awards_view(view.clone())
            .with_context(|| "save player_awards.json")?;
        view
    } else {
        icelines_fetch::career_landing::load_local_awards_store()
            .get(pid)
            .cloned()
            .with_context(|| {
                format!(
                    "live feeds are disabled and no cached awards were found for {player} ({pid})"
                )
            })?
    };

    if format == Format::Json {
        let s = serde_json::to_string_pretty(&view)?;
        match out {
            Some(path) => std::fs::write(&path, format!("{s}\n"))
                .with_context(|| format!("writing awards to {}", path.display()))?,
            None => println!("{s}"),
        }
        return Ok(());
    }

    let headers = ["trophy", "season", "type", "gp", "g", "a", "p", "+/-"];
    let rows = view
        .awards
        .iter()
        .flat_map(|award| {
            award.seasons.iter().map(move |season| {
                vec![
                    award.trophy.clone(),
                    season.season.0.to_string(),
                    game_type_label(season.game_type_id).to_string(),
                    opt_u32(season.games_played),
                    opt_u32(season.goals),
                    opt_u32(season.assists),
                    opt_u32(season.points),
                    season
                        .plus_minus
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ]
            })
        })
        .collect::<Vec<_>>();

    if rows.is_empty() {
        let message = format!("No NHL awards found for {player} ({pid}).");
        match out {
            Some(path) => std::fs::write(&path, format!("{message}\n"))
                .with_context(|| format!("writing awards to {}", path.display()))?,
            None => println!("{message}"),
        }
        return Ok(());
    }
    format.emit_to(&headers, &rows, out.as_deref())
}

fn opt_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn game_type_label(game_type_id: u8) -> &'static str {
    match game_type_id {
        2 => "regular",
        3 => "playoffs",
        _ => "other",
    }
}
