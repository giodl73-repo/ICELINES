use std::{env, fs, path::PathBuf};

use chrono::{TimeZone, Utc};
use icelines_core::{
    build_fantasy_trade_card,
    model::{Position, Season},
    season_stats::SeasonType,
    FantasyTradeCardInput, FantasyTradeEvaluationView, FantasyTradePlayerEvaluation,
    FantasyTradeTeamEvaluation, ViewContext, ViewWindow, FANTASY_TRADE_EVALUATION_SCHEMA,
};

fn player(
    key: &str,
    name: &str,
    team: &str,
    positions: &[Position],
    league_value: f64,
    per_game: f64,
    games: u32,
) -> FantasyTradePlayerEvaluation {
    FantasyTradePlayerEvaluation {
        player_key: key.to_string(),
        player: name.to_string(),
        nhl_team: team.to_string(),
        positions: positions.to_vec(),
        league_value,
        league_value_per_game: per_game,
        remaining_games: games,
        projected_remaining_value: per_game * f64::from(games),
    }
}

fn team(
    name: &str,
    before: f64,
    after: f64,
    games_delta: i32,
    missing_before: usize,
    missing_after: usize,
) -> FantasyTradeTeamEvaluation {
    FantasyTradeTeamEvaluation {
        team: name.to_string(),
        before_value: before,
        after_value: after,
        value_delta: after - before,
        remaining_games_delta: games_delta,
        roster_size_after: 16,
        standard_capacity: 16,
        missing_active_slots_before: missing_before,
        missing_active_slots_after: missing_after,
        legal: missing_after <= missing_before,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("examples/fantasy-trade-card-dexters-dawgs-fox-rantanen.json")
    });
    let evaluated_at = Utc.with_ymd_and_hms(2026, 11, 12, 15, 0, 0).unwrap();
    let evaluation = FantasyTradeEvaluationView {
        schema: FANTASY_TRADE_EVALUATION_SCHEMA.to_string(),
        executed: false,
        saved_offer_id: None,
        league: "Dexter's 2026-27 League".to_string(),
        scoring_scheme: "Dexter's Dawgs league scoring".to_string(),
        sending_team: "Dexter's Dawgs".to_string(),
        receiving_team: "Blue Line Bandits".to_string(),
        sends: vec![player(
            "adam-fox",
            "Adam Fox",
            "NYR",
            &[Position::Defense],
            318.0,
            4.42,
            43,
        )],
        receives: vec![player(
            "mikko-rantanen",
            "Mikko Rantanen",
            "DAL",
            &[Position::LeftWing, Position::RightWing],
            334.0,
            4.55,
            41,
        )],
        sending_team_result: team("Dexter's Dawgs", 4_021.8, 4_017.9, -2, 0, 0),
        receiving_team_result: team("Blue Line Bandits", 3_884.2, 3_888.1, 2, 0, 0),
        package_value_gap: -3.51,
        package_value_gap_percent: -1.85,
        recommendation: "Reasonable offer range; Dexter's Dawgs gains elite wing flexibility while Blue Line Bandits gains a top defense anchor".to_string(),
        warnings: vec![
            "Player values, team totals, and remaining games are deterministic fixture inputs, not current trade advice.".to_string(),
            "Refresh injury, role, schedule, and playoff evidence before accepting.".to_string(),
        ],
    };
    let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
    view.generated_at = Some(evaluated_at);
    let card = build_fantasy_trade_card(FantasyTradeCardInput {
        league_id: "dexters-dawgs-league".to_string(),
        scoring_scheme_id: "dexters-dawgs".to_string(),
        sending_team_id: "dexters-dawgs".to_string(),
        receiving_team_id: "blue-line-bandits".to_string(),
        offer_id: None,
        evaluated_at,
        evaluation,
        view,
    })?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output,
        format!("{}\n", serde_json::to_string_pretty(&card)?),
    )?;
    Ok(())
}
