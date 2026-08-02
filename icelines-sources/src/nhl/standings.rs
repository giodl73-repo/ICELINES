use icelines_core::TeamStandingInput;

#[derive(Debug, Clone, PartialEq)]
pub struct NhlStandingsRow {
    pub team: String,
    pub conference: Option<String>,
    pub division: Option<String>,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub overtime_losses: u32,
    pub points: u32,
    pub points_percentage: f32,
    pub regulation_wins: Option<u32>,
    pub goal_differential: i32,
    pub league_rank: Option<u32>,
    pub conference_rank: Option<u32>,
    pub division_rank: Option<u32>,
    pub wild_card_rank: Option<u32>,
}

impl NhlStandingsRow {
    pub fn to_team_standing_input(&self) -> TeamStandingInput {
        TeamStandingInput {
            team: self.team.clone(),
            conference: self.conference.clone(),
            division: self.division.clone(),
            games_played: self.games_played,
            wins: self.wins,
            losses: self.losses,
            overtime_losses: self.overtime_losses,
            points: self.points,
            points_percentage: self.points_percentage,
            regulation_wins: self.regulation_wins,
            goal_differential: self.goal_differential,
            league_rank: self.league_rank,
            conference_rank: self.conference_rank,
            division_rank: self.division_rank,
            wild_card_rank: self.wild_card_rank,
        }
    }
}

pub fn parse_standings(raw: &serde_json::Value) -> Vec<NhlStandingsRow> {
    raw["standings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_standings_row)
        .collect()
}

fn parse_standings_row(row: &serde_json::Value) -> Option<NhlStandingsRow> {
    let team = localized_string(&row["teamAbbrev"])
        .or_else(|| row["teamAbbrev"].as_str().map(str::to_owned))
        .or_else(|| row["teamCommonName"]["abbrev"].as_str().map(str::to_owned))?
        .to_ascii_uppercase();
    let games_played = u32_field(row, &["gamesPlayed", "gp"]);
    let points = u32_field(row, &["points", "pts"]);
    let points_percentage = f32_field(row, &["pointPctg", "pointsPercentage", "pointsPctg"])
        .unwrap_or_else(|| {
            if games_played > 0 {
                points as f32 / (games_played * 2) as f32
            } else {
                0.0
            }
        });

    Some(NhlStandingsRow {
        team,
        conference: localized_string(&row["conferenceName"])
            .or_else(|| row["conferenceAbbrev"].as_str().map(expand_conference)),
        division: localized_string(&row["divisionName"])
            .or_else(|| row["divisionAbbrev"].as_str().map(str::to_owned)),
        games_played,
        wins: u32_field(row, &["wins", "w"]),
        losses: u32_field(row, &["losses", "l"]),
        overtime_losses: u32_field(row, &["otLosses", "overtimeLosses", "otl"]),
        points,
        points_percentage,
        regulation_wins: optional_u32_field(row, &["regulationWins", "rw"]),
        goal_differential: i32_field(row, &["goalDifferential", "goalDiff"]),
        league_rank: optional_u32_field(row, &["leagueSequence", "leagueRank"]),
        conference_rank: optional_u32_field(row, &["conferenceSequence", "conferenceRank"]),
        division_rank: optional_u32_field(row, &["divisionSequence", "divisionRank"]),
        wild_card_rank: optional_u32_field(row, &["wildcardSequence", "wildCardSequence"]),
    })
}

fn localized_string(value: &serde_json::Value) -> Option<String> {
    value["default"]
        .as_str()
        .or_else(|| value["en"].as_str())
        .or_else(|| value.as_str())
        .map(str::to_owned)
}

fn u32_field(row: &serde_json::Value, keys: &[&str]) -> u32 {
    optional_u32_field(row, keys).unwrap_or(0)
}

fn optional_u32_field(row: &serde_json::Value, keys: &[&str]) -> Option<u32> {
    keys.iter()
        .find_map(|key| row[*key].as_u64().map(|value| value as u32))
}

fn i32_field(row: &serde_json::Value, keys: &[&str]) -> i32 {
    keys.iter()
        .find_map(|key| row[*key].as_i64().map(|value| value as i32))
        .unwrap_or(0)
}

fn f32_field(row: &serde_json::Value, keys: &[&str]) -> Option<f32> {
    keys.iter()
        .find_map(|key| row[*key].as_f64().map(|value| value as f32))
}

fn expand_conference(abbrev: &str) -> String {
    match abbrev {
        "E" | "EAST" => "Eastern".to_string(),
        "W" | "WEST" => "Western".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_standings;
    use serde_json::json;

    #[test]
    fn projects_team_rows_for_core_input() {
        let raw = json!({
            "standings": [
                {
                    "teamAbbrev": { "default": "SEA" },
                    "conferenceName": "Western",
                    "divisionName": "Pacific",
                    "gamesPlayed": 40,
                    "wins": 22,
                    "losses": 13,
                    "otLosses": 5,
                    "points": 49,
                    "pointPctg": 0.613,
                    "regulationWins": 19,
                    "goalDifferential": 14,
                    "leagueSequence": 9,
                    "conferenceSequence": 5,
                    "divisionSequence": 3,
                    "wildcardSequence": 1
                }
            ]
        });

        let rows = parse_standings(&raw);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.team, "SEA");
        assert_eq!(row.conference.as_deref(), Some("Western"));
        assert_eq!(row.division.as_deref(), Some("Pacific"));
        assert_eq!(row.games_played, 40);
        assert_eq!(row.points, 49);
        assert_eq!(row.points_percentage, 0.613);
        assert_eq!(row.wild_card_rank, Some(1));

        let input = row.to_team_standing_input();
        assert_eq!(input.team, "SEA");
        assert_eq!(input.regulation_wins, Some(19));
    }

    #[test]
    fn computes_points_percentage_when_missing() {
        let raw = json!({
            "standings": [
                {
                    "teamAbbrev": "EDM",
                    "conferenceAbbrev": "W",
                    "gamesPlayed": 10,
                    "wins": 6,
                    "losses": 3,
                    "otLosses": 1,
                    "points": 13
                }
            ]
        });

        let rows = parse_standings(&raw);
        assert_eq!(rows[0].team, "EDM");
        assert_eq!(rows[0].conference.as_deref(), Some("Western"));
        assert!((rows[0].points_percentage - 0.65).abs() < 0.001);
    }
}
