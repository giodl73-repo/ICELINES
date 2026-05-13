use crate::config::Config;
use crate::render::terminal::render_team_depth_view;
use anyhow::{bail, Context};
use icelines_core::{
    model::Season, season_stats::SeasonType, ScheduleRecord, ScheduledGameInput, TeamAbbr,
    TeamDepthView, TeamSeasonGameRow, TeamSeasonVenue, TeamSeasonView, ViewContext, ViewWindow,
};
use icelines_fetch::nhl_api::{NhlApiClient, ScheduledGame};
use icelines_fetch::{snapshot::SnapshotStore, stats_loader::load_into_repo};

pub async fn run(team: String, _scheme: Option<String>, no_color: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let team_abbr = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{team}' is not a valid NHL team abbreviation"))?;

    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    let season = Season(season_u32);

    // Hart.5c.1: load directly into a StatsRepository, take the team's
    // roster as PlayerView slice, build the depth chart from views.
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(season, SeasonType::Regular, &store)
        .map_err(|e| anyhow::anyhow!("loading repo: {e}"))?;
    let view = TeamDepthView::from_repository(
        &outcome.repo,
        team_abbr.clone(),
        season,
        SeasonType::Regular,
    );
    if view.is_empty() {
        bail!("no skaters found for {} in data", team_abbr);
    }

    render_team_depth_view(&view, no_color);
    Ok(())
}

pub async fn run_team_season(team: String, json: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let team_abbr = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{team}' is not a valid NHL team abbreviation"))?;

    let season_str = cfg.season_str().to_string();
    let season = season_str
        .parse::<u32>()
        .map(Season)
        .unwrap_or(Season(icelines_core::CURRENT_SEASON));
    let season_type = SeasonType::Regular;

    let client = NhlApiClient::production();
    let standings = client
        .fetch_standings_now()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.to_team_standing_input())
        .collect();
    let games = client
        .fetch_team_season_schedule(&team_abbr.0, &season_str)
        .await
        .with_context(|| format!("fetching {team_abbr} season schedule for {season_str}"))?
        .into_iter()
        .map(scheduled_game_input)
        .collect();

    let view = TeamSeasonView::from_games_and_standings(
        ViewContext::new(ViewWindow::new(season, season_type)),
        season_str,
        team_abbr.0.to_string(),
        games,
        standings,
    );

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing team season view")?
        );
    } else {
        print!("{}", render_team_season_text(&view));
    }
    Ok(())
}

pub(crate) fn scheduled_game_input(game: ScheduledGame) -> ScheduledGameInput {
    ScheduledGameInput {
        game_id: game.game_id,
        date: game.date,
        game_type: game.game_type,
        away_abbrev: game.away_abbrev,
        away_name: game.away_name,
        home_abbrev: game.home_abbrev,
        home_name: game.home_name,
        start_time_utc: game.start_time_utc,
        away_score: game.away_score,
        home_score: game.home_score,
        game_state: game.game_state,
        last_period: game.last_period,
        series_game: game.series_game,
        away_wins: game.away_wins,
        home_wins: game.home_wins,
    }
}

pub(crate) fn render_team_season_text(view: &TeamSeasonView) -> String {
    let mut out = String::new();
    let headline = &view.headline;
    out.push_str(&format!(
        "{} TEAM SEASON - {}\n",
        view.team, view.season_pretty
    ));
    out.push_str(&format!(
        "Record {}  Pts {}  Pts% {:.3}  GF-GA {}-{}  GD {}\n",
        record_label(headline.record),
        headline.points,
        headline.points_percentage,
        headline.goals_for,
        headline.goals_against,
        signed_i32(headline.goal_differential)
    ));
    out.push_str(&format!(
        "Home {}  Away {}  One-goal {}  Last 10 {} ({})\n",
        record_label(view.splits.home.record),
        record_label(view.splits.away.record),
        record_label(view.splits.one_goal.record),
        record_label(view.form.last_10),
        signed_i32(view.form.last_10_goal_differential)
    ));
    out.push_str(&format!(
        "Remaining {} games ({} home, {} away)",
        view.remaining.games, view.remaining.home, view.remaining.away
    ));
    if !view.remaining.next_opponents.is_empty() {
        out.push_str(&format!(
            "  Next: {}",
            view.remaining.next_opponents.join(", ")
        ));
    }
    out.push('\n');

    if let Some(standings) = &view.standings {
        out.push_str(&format!(
            "Standings {} · {} pts · Pts% {:.3} · {}",
            standings
                .conference
                .as_deref()
                .unwrap_or("conference unknown"),
            standings.points,
            standings.points_percentage,
            standings.playoff_position_label
        ));
        if let Some(behind) = standings.points_behind_cutline {
            out.push_str(&format!(" · {behind} pts behind cutline"));
        } else if let Some(above) = standings.points_above_cutline {
            out.push_str(&format!(" · {above} pts above cutline"));
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "SOS faced {} · remaining {} · tiers faced T/M/B/U {}/{}/{}/{}\n",
        pct_or_dash(view.schedule_strength.faced_average_points_percentage),
        pct_or_dash(view.schedule_strength.remaining_average_points_percentage),
        view.schedule_strength.faced.top,
        view.schedule_strength.faced.middle,
        view.schedule_strength.faced.bottom,
        view.schedule_strength.faced.unknown
    ));
    out.push_str(&format!(
        "Ledger quality wins {} · expected wins {} · bad losses {} · missed pts {}\n",
        view.quality_ledger.quality_wins,
        view.quality_ledger.expected_wins,
        view.quality_ledger.bad_losses,
        view.quality_ledger.missed_points
    ));

    if let Some(warning) = view.warnings.first() {
        out.push_str(&format!("Warning: {}\n", warning.message));
    }

    if view.rows.is_empty() {
        out.push_str("No games found for this team season.\n");
        return out;
    }

    out.push_str("\nDate       V Opp Result Score GD  Status\n");
    out.push_str("----------------------------------------\n");
    for row in &view.rows {
        out.push_str(&format_team_season_row(row));
        out.push('\n');
    }
    out
}

fn format_team_season_row(row: &TeamSeasonGameRow) -> String {
    format!(
        "{:<10} {:<1} {:<3} {:<6} {:<5} {:>3} {}\n",
        row.date,
        match row.venue {
            TeamSeasonVenue::Home => "H",
            TeamSeasonVenue::Away => "A",
        },
        row.opponent_abbrev,
        row.result,
        match (row.team_score, row.opponent_score) {
            (Some(team_score), Some(opponent_score)) => format!("{team_score}-{opponent_score}"),
            _ => "-".to_string(),
        },
        row.goal_differential
            .map(signed_i16)
            .unwrap_or_else(|| "-".to_string()),
        row.state_label
    )
    .trim_end()
    .to_string()
}

fn record_label(record: ScheduleRecord) -> String {
    format!(
        "{}-{}-{}",
        record.wins, record.losses, record.overtime_losses
    )
}

fn signed_i32(value: i32) -> String {
    format!("{value:+}")
}

fn signed_i16(value: i16) -> String {
    format!("{value:+}")
}

fn pct_or_dash(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_game(
        date: &str,
        away: &str,
        home: &str,
        away_score: Option<u8>,
        home_score: Option<u8>,
        state: &str,
    ) -> ScheduledGameInput {
        ScheduledGameInput {
            game_id: 1,
            date: date.to_string(),
            game_type: 2,
            away_abbrev: away.to_string(),
            away_name: away.to_string(),
            home_abbrev: home.to_string(),
            home_name: home.to_string(),
            start_time_utc: format!("{date}T23:00:00Z"),
            away_score,
            home_score,
            game_state: Some(state.to_string()),
            last_period: None,
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    fn fixture_view() -> TeamSeasonView {
        TeamSeasonView::from_games(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "20252026".to_string(),
            "EDM".to_string(),
            vec![
                fixture_game("2025-10-01", "EDM", "CGY", Some(4), Some(2), "OFF"),
                fixture_game("2025-10-03", "VAN", "EDM", Some(3), Some(2), "OFF"),
                fixture_game("2025-10-05", "EDM", "SEA", None, None, "FUT"),
            ],
        )
    }

    #[test]
    fn l0_team_season_text_renders_shared_viewmodel_summary() {
        let out = render_team_season_text(&fixture_view());

        assert!(out.contains("EDM TEAM SEASON - 2025-26"), "{out}");
        assert!(out.contains("Record 1-1-0"), "{out}");
        assert!(out.contains("Home 0-1-0  Away 1-0-0"), "{out}");
        assert!(out.contains("Remaining 1 games (0 home, 1 away)"), "{out}");
        assert!(
            out.contains("2025-10-01 A CGY W      4-2    +2 FINAL"),
            "{out}"
        );
        assert!(
            out.contains("Warning: Standings source not loaded"),
            "{out}"
        );
        assert!(out.contains("SOS faced"), "{out}");
        assert!(out.contains("Ledger quality wins"), "{out}");
    }

    #[test]
    fn l0_team_season_text_empty_view_has_clear_empty_state() {
        let view = TeamSeasonView::from_games(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "20252026".to_string(),
            "EDM".to_string(),
            Vec::new(),
        );
        let out = render_team_season_text(&view);

        assert!(
            out.contains("No games found for this team season."),
            "{out}"
        );
    }
}
