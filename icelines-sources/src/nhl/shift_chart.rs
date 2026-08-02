use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialShiftChartRow {
    pub game_id: u64,
    pub player_id: u32,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub team_abbrev: String,
    pub period: u8,
    pub start_time: String,
    pub end_time: String,
    /// Provider duration is nullable; consumers derive interval duration from
    /// start and end boundaries.
    pub duration: Option<String>,
    pub shift_number: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialShiftChartResponse {
    pub data: Vec<OfficialShiftChartRow>,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::OfficialShiftChartResponse;

    #[test]
    fn decodes_official_camel_case_rows_and_nullable_duration() {
        let response: OfficialShiftChartResponse = serde_json::from_str(
            r#"{
                "data": [{
                    "gameId": 2025020001,
                    "playerId": 8478402,
                    "firstName": "Connor",
                    "lastName": "McDavid",
                    "teamAbbrev": "EDM",
                    "period": 1,
                    "startTime": "00:15",
                    "endTime": "00:53",
                    "duration": null,
                    "shiftNumber": 1
                }],
                "total": 1
            }"#,
        )
        .expect("valid official response");

        assert_eq!(response.total, 1);
        assert_eq!(response.data[0].player_id, 8478402);
        assert_eq!(response.data[0].duration, None);
    }
}
