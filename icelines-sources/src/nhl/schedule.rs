#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScheduledGame {
    pub game_id: u64,
    pub date: String,
    pub game_type: u8,
    pub away_abbrev: String,
    pub away_name: String,
    pub home_abbrev: String,
    pub home_name: String,
    pub start_time_utc: String,
    pub away_score: Option<u8>,
    pub home_score: Option<u8>,
    pub game_state: Option<String>,
    pub last_period: Option<String>,
    pub series_game: Option<String>,
    pub away_wins: Option<u8>,
    pub home_wins: Option<u8>,
}

impl ScheduledGame {
    pub fn is_playoff(&self) -> bool {
        self.game_type == 3
    }

    pub fn is_final(&self) -> bool {
        matches!(self.game_state.as_deref(), Some("FINAL") | Some("OFF"))
    }

    pub fn is_live(&self) -> bool {
        matches!(self.game_state.as_deref(), Some("LIVE") | Some("CRIT"))
    }

    pub fn involves(&self, abbrev: &str) -> bool {
        self.away_abbrev == abbrev || self.home_abbrev == abbrev
    }

    pub fn series_label(&self) -> Option<String> {
        let gm = self.series_game.as_deref()?;
        let aw = self.away_wins?;
        let hw = self.home_wins?;
        Some(format!(
            "{} {aw}–{hw} {} · {gm}",
            self.away_abbrev, self.home_abbrev
        ))
    }
}

/// Parse one game object from either a game-week or club-season payload.
pub fn parse_game(g: &serde_json::Value, fallback_date: Option<&str>) -> Option<ScheduledGame> {
    let game_id = g["id"].as_u64().unwrap_or(0);
    if game_id == 0 {
        return None;
    }
    let date = g["gameDate"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| fallback_date.map(str::to_owned))
        .unwrap_or_default();

    let away = g["awayTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let away_name = g["awayTeam"]["placeName"]["default"]
        .as_str()
        .unwrap_or(&away)
        .to_owned();
    let home = g["homeTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let home_name = g["homeTeam"]["placeName"]["default"]
        .as_str()
        .unwrap_or(&home)
        .to_owned();
    let start = g["startTimeUTC"].as_str().unwrap_or("").to_owned();
    let game_type = g["gameType"].as_u64().unwrap_or(2) as u8;

    let away_score = g["awayTeam"]["score"].as_u64().map(|v| v as u8);
    let home_score = g["homeTeam"]["score"].as_u64().map(|v| v as u8);
    let game_state = g["gameState"].as_str().map(str::to_owned);
    let last_period = g["gameOutcome"]["lastPeriodType"]
        .as_str()
        .map(str::to_owned);

    let series_summary = &g["seriesSummary"];
    let series_status = &g["seriesStatus"];

    let series_game = series_summary["gameLabel"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| series_status["gameLabel"].as_str().map(str::to_owned))
        .or_else(|| g["gameLabel"].as_str().map(str::to_owned))
        .or_else(|| {
            let number = series_status["gameNumberOfSeries"]
                .as_u64()
                .or_else(|| series_status["gameNumberOfSeven"].as_u64())
                .or_else(|| series_summary["gameNumber"].as_u64())
                .or_else(|| g["gameNumber"].as_u64())?;
            Some(format!("Game {number}"))
        });

    let away_wins = series_summary["awayWins"]
        .as_u64()
        .or_else(|| seed_wins_for_team(series_status, &away))
        .map(|value| value as u8);
    let home_wins = series_summary["homeWins"]
        .as_u64()
        .or_else(|| seed_wins_for_team(series_status, &home))
        .map(|value| value as u8);

    Some(ScheduledGame {
        game_id,
        date,
        game_type,
        away_abbrev: away,
        away_name,
        home_abbrev: home,
        home_name,
        start_time_utc: start,
        away_score,
        home_score,
        game_state,
        last_period,
        series_game,
        away_wins,
        home_wins,
    })
}

fn seed_wins_for_team(series_status: &serde_json::Value, team: &str) -> Option<u64> {
    let top_abbrev = series_status["topSeedTeamAbbrev"].as_str().unwrap_or("");
    let bottom_abbrev = series_status["bottomSeedTeamAbbrev"].as_str().unwrap_or("");
    if team == top_abbrev {
        series_status["topSeedWins"].as_u64()
    } else if team == bottom_abbrev {
        series_status["bottomSeedWins"].as_u64()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::parse_game;
    use serde_json::json;

    #[test]
    fn reads_current_series_status_shape() {
        let raw = json!({
            "id": 2025030104_u64,
            "gameType": 3,
            "awayTeam": {"abbrev": "NYR", "score": 3},
            "homeTeam": {"abbrev": "WSH", "score": 2},
            "gameState": "FINAL",
            "seriesStatus": {
                "gameNumberOfSeries": 4,
                "topSeedTeamAbbrev": "WSH",
                "topSeedWins": 1,
                "bottomSeedTeamAbbrev": "NYR",
                "bottomSeedWins": 3
            }
        });

        let game = parse_game(&raw, Some("2026-04-29")).expect("valid game");
        assert_eq!(game.series_game.as_deref(), Some("Game 4"));
        assert_eq!(game.away_wins, Some(3));
        assert_eq!(game.home_wins, Some(1));
        assert!(game.is_final());
        assert_eq!(game.series_label().as_deref(), Some("NYR 3–1 WSH · Game 4"));
    }

    #[test]
    fn rejects_missing_game_id() {
        assert!(parse_game(&json!({"gameType": 2}), None).is_none());
    }
}
