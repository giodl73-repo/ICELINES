use std::{fs, path::PathBuf};

use chrono::{TimeZone, Utc};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    build_season_simulation_card, Season, SeasonSimulationCardInput, TeamSeasonForecastView,
    ViewContext, ViewWindow,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    write_cards(
        "examples/icecast-alp-development-variance-10000-result.json",
        Utc.with_ymd_and_hms(2026, 7, 22, 12, 0, 0).single(),
        "nhl-2026-27-1344-games",
    )?;
    write_cards(
        "examples/icecast-2024-25-replay-1000-result.json",
        Utc.with_ymd_and_hms(2025, 6, 18, 12, 0, 0).single(),
        "nhl-2024-25-1312-games",
    )?;
    Ok(())
}

fn write_cards(
    forecast_path: &str,
    evidence_at: Option<chrono::DateTime<Utc>>,
    calendar_fingerprint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let forecast: TeamSeasonForecastView = serde_json::from_slice(&fs::read(forecast_path)?)?;
    for (team, name) in [("NYR", "New York Rangers"), ("SEA", "Seattle Kraken")] {
        let mut view = ViewContext::new(ViewWindow::new(
            Season(forecast.season),
            SeasonType::Regular,
        ));
        view.generated_at = evidence_at;
        view.data_generation = Some("sealed-example".to_string());
        let card = build_season_simulation_card(SeasonSimulationCardInput {
            forecast: forecast.clone(),
            focus_team: team.to_string(),
            team_name: name.to_string(),
            view,
            evidence_at,
            calendar_fingerprint: Some(calendar_fingerprint.to_string()),
        })?;
        let start_year = forecast.season / 10_000;
        let end_year = forecast.season % 10_000;
        let path = PathBuf::from(format!(
            "examples/season-simulation-card-{}-{}-{:02}.json",
            team.to_ascii_lowercase(),
            start_year,
            end_year % 100
        ));
        fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&card)?))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
