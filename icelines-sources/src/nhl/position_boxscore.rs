use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GameLogResponse {
    #[serde(rename = "gameLog")]
    pub game_log: Vec<GameLogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameLogEntry {
    pub game_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxscoreResponse {
    pub player_by_game_stats: PlayerByGameStats,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerByGameStats {
    pub home_team: TeamStatsResponse,
    pub away_team: TeamStatsResponse,
}

#[derive(Debug, Deserialize)]
pub struct TeamStatsResponse {
    pub forwards: Vec<SkaterEntry>,
    pub defense: Vec<SkaterEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkaterEntry {
    pub player_id: u32,
    pub position: String,
    #[serde(default)]
    pub toi: Option<String>,
    #[serde(default)]
    pub shifts: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::{BoxscoreResponse, GameLogResponse};

    #[test]
    fn decodes_game_log_contract() {
        let response: GameLogResponse =
            serde_json::from_str(r#"{"gameLog":[{"gameId":2025020001}]}"#).expect("valid game log");
        assert_eq!(response.game_log[0].game_id, 2025020001);
    }

    #[test]
    fn decodes_position_boxscore_with_optional_deployment_fields() {
        let response: BoxscoreResponse = serde_json::from_str(
            r#"{
                "playerByGameStats": {
                    "homeTeam": {
                        "forwards": [{"playerId":8478402,"position":"C","toi":"21:10","shifts":24}],
                        "defense": []
                    },
                    "awayTeam": {
                        "forwards": [{"playerId":8484786,"position":"C"}],
                        "defense": []
                    }
                }
            }"#,
        )
        .expect("valid boxscore");

        let home = &response.player_by_game_stats.home_team.forwards[0];
        assert_eq!(home.toi.as_deref(), Some("21:10"));
        assert_eq!(home.shifts, Some(24));
        let away = &response.player_by_game_stats.away_team.forwards[0];
        assert_eq!(away.toi, None);
        assert_eq!(away.shifts, None);
    }
}
