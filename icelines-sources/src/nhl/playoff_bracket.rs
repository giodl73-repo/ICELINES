// ── Playoff bracket types (Phase 7e) ──────────────────────────────────────────

/// One series in a playoff round.
#[derive(Debug, Clone)]
pub struct PlayoffSeries {
    pub letter: Option<String>, // e.g. "A" — used as a stable key
    pub top_seed_abbrev: String,
    pub top_seed_name: String,
    pub top_seed_wins: u8,
    pub top_seed_rank: Option<String>, // e.g. "A1", "WC1"
    pub bottom_seed_abbrev: String,
    pub bottom_seed_name: String,
    pub bottom_seed_wins: u8,
    pub bottom_seed_rank: Option<String>,
    pub winner_abbrev: Option<String>, // None until series concludes
    pub conference: Option<String>,    // "Eastern" | "Western" | None
    /// Per-game results for this series. Empty when the live API source
    /// does not include game logs; populated for historical bundles.
    /// Phase 8c.
    pub games: Vec<PlayoffGameResult>,
}

/// One game inside a playoff series. Sourced from bundled `playoffs.json`
/// for historical seasons (Phase 8c). The live `/v1/playoff-bracket/{year}`
/// endpoint does not include per-game logs, so for current-season series
/// this vector is empty.
#[derive(Debug, Clone)]
pub struct PlayoffGameResult {
    pub date: String, // ISO 8601 (YYYY-MM-DD)
    pub home_abbrev: String,
    pub away_abbrev: String,
    pub home_score: u8,
    pub away_score: u8,
    pub series_after: String, // e.g. "TBL 1-0", "tied 2-2"
    pub goals: Vec<PlayoffGoal>,
}

/// One goal scored in a playoff game. v1 of the bundle records scorer name
/// and team abbrev only; assists and timestamps may be added in v2.
#[derive(Debug, Clone)]
pub struct PlayoffGoal {
    pub scorer: String,
    pub team: String,
}

impl PlayoffSeries {
    /// True when one side has 4 wins and the other has fewer.
    pub fn is_complete(&self) -> bool {
        self.top_seed_wins == 4 || self.bottom_seed_wins == 4
    }

    /// Total number of games played so far in the series.
    pub fn games_played(&self) -> u8 {
        self.top_seed_wins + self.bottom_seed_wins
    }

    /// One-line summary like "FLA 4-2 TBL · FLA wins" (or "tied 2-2", "FLA leads 3-1").
    pub fn summary(&self) -> String {
        let (t, b) = (self.top_seed_wins, self.bottom_seed_wins);
        if let Some(w) = &self.winner_abbrev {
            format!(
                "{} {t}-{b} {} · {w} wins",
                self.top_seed_abbrev, self.bottom_seed_abbrev
            )
        } else if t > b {
            format!("{} leads {t}-{b}", self.top_seed_abbrev)
        } else if b > t {
            format!("{} leads {b}-{t}", self.bottom_seed_abbrev)
        } else if t == 0 {
            format!(
                "{} vs {} · series begins",
                self.top_seed_abbrev, self.bottom_seed_abbrev
            )
        } else {
            format!("Tied {t}-{b}")
        }
    }
}

/// One round of a playoff bracket.
#[derive(Debug, Clone)]
pub struct PlayoffRound {
    pub round_number: u8, // 1..=4
    pub label: String,    // "First Round", "Second Round", "Conf Final", "Stanley Cup Final"
    pub series: Vec<PlayoffSeries>,
}

/// Full playoff bracket for one season.
#[derive(Debug, Clone)]
pub struct PlayoffBracket {
    pub season: String,
    pub current_round: Option<u8>,
    pub rounds: Vec<PlayoffRound>,
}

impl PlayoffBracket {
    /// Find a series by its letter (e.g. "A").
    pub fn find_series(&self, letter: &str) -> Option<&PlayoffSeries> {
        for r in &self.rounds {
            for s in &r.series {
                if s.letter.as_deref() == Some(letter) {
                    return Some(s);
                }
            }
        }
        None
    }

    /// True if every round is empty (no series yet — pre-playoffs / off-season).
    pub fn is_empty(&self) -> bool {
        self.rounds.iter().all(|r| r.series.is_empty())
    }
}

/// Parse a playoff-bracket JSON payload. Defensively accepts the shape NHL's
/// API has historically used (`series` list grouped by `playoffRounds`) and
/// extracts the fields we render. Unknown fields are silently dropped.
pub fn parse_playoff_bracket(raw: &serde_json::Value) -> PlayoffBracket {
    let season = raw["season"]
        .as_str()
        .or_else(|| raw["seasonId"].as_str())
        .map(str::to_owned)
        .unwrap_or_default();
    let current_round = raw["currentRound"]
        .as_u64()
        .or_else(|| raw["roundNumber"].as_u64())
        .map(|v| v as u8);

    let mut rounds: Vec<PlayoffRound> = Vec::new();

    // Shape A: legacy nested form — `playoffRounds: [{ roundNumber, series: [..] }]`.
    let round_arrays = raw["playoffRounds"]
        .as_array()
        .or_else(|| raw["rounds"].as_array());
    if let Some(arr) = round_arrays {
        for r in arr {
            let round_number = r["roundNumber"].as_u64().unwrap_or(0) as u8;
            let label = r["roundLabel"]
                .as_str()
                .or_else(|| r["roundName"].as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| default_round_label(round_number));
            let mut series = Vec::new();
            if let Some(s_arr) = r["series"].as_array() {
                for s in s_arr {
                    series.push(parse_series(s));
                }
            }
            rounds.push(PlayoffRound {
                round_number,
                label,
                series,
            });
        }
    }

    // Shape B: current API (verified 2026-04-29) — flat `series: [..]`
    // where each series carries its own `playoffRound`. Bucket by round.
    if rounds.is_empty() {
        if let Some(arr) = raw["series"].as_array() {
            use std::collections::BTreeMap;
            let mut by_round: BTreeMap<u8, Vec<PlayoffSeries>> = BTreeMap::new();
            let mut labels: BTreeMap<u8, String> = BTreeMap::new();
            for s in arr {
                let rn = s["playoffRound"]
                    .as_u64()
                    .or_else(|| s["roundNumber"].as_u64())
                    .unwrap_or(0) as u8;
                if let Some(t) = s["seriesTitle"].as_str() {
                    labels.entry(rn).or_insert_with(|| t.to_owned());
                }
                by_round.entry(rn).or_default().push(parse_series(s));
            }
            for (rn, ser) in by_round {
                let label = labels
                    .get(&rn)
                    .cloned()
                    .unwrap_or_else(|| default_round_label(rn));
                rounds.push(PlayoffRound {
                    round_number: rn,
                    label,
                    series: ser,
                });
            }
        }
    }

    rounds.sort_by_key(|r| r.round_number);
    PlayoffBracket {
        season,
        current_round,
        rounds,
    }
}

fn default_round_label(round_number: u8) -> String {
    match round_number {
        1 => "First Round".to_owned(),
        2 => "Second Round".to_owned(),
        3 => "Conference Final".to_owned(),
        4 => "Stanley Cup Final".to_owned(),
        _ => format!("Round {round_number}"),
    }
}

fn parse_series(s: &serde_json::Value) -> PlayoffSeries {
    let letter = s["seriesLetter"]
        .as_str()
        .or_else(|| s["seriesAbbrev"].as_str())
        .map(str::to_owned);

    let top = &s["topSeedTeam"];
    let bottom = &s["bottomSeedTeam"];

    let top_abbrev = top["abbrev"].as_str().unwrap_or("").to_owned();
    let top_name = top["name"]["default"]
        .as_str()
        .or_else(|| top["placeName"]["default"].as_str())
        .unwrap_or(&top_abbrev)
        .to_owned();
    // Wins: legacy nested API put it on the team object; current API
    // (verified 2026-04-29 against /v1/playoff-bracket/2026) puts it at
    // the series level as `topSeedWins`/`bottomSeedWins`.
    let top_wins = top["wins"]
        .as_u64()
        .or_else(|| s["topSeedWins"].as_u64())
        .unwrap_or(0) as u8;
    // Rank: prefer the abbreviated form ("D1", "WC1") when present —
    // matches what users see in the playoff bracket header.
    let top_rank = s["topSeedRankAbbrev"]
        .as_str()
        .or_else(|| top["seed"].as_str())
        .or_else(|| s["topSeedRank"].as_str())
        .map(str::to_owned)
        // Numeric `topSeedRank` (1..=8) shows up as a u64 — fall through
        // to that and stringify so something usable lands in the UI.
        .or_else(|| s["topSeedRank"].as_u64().map(|n| n.to_string()));

    let bot_abbrev = bottom["abbrev"].as_str().unwrap_or("").to_owned();
    let bot_name = bottom["name"]["default"]
        .as_str()
        .or_else(|| bottom["placeName"]["default"].as_str())
        .unwrap_or(&bot_abbrev)
        .to_owned();
    let bot_wins = bottom["wins"]
        .as_u64()
        .or_else(|| s["bottomSeedWins"].as_u64())
        .unwrap_or(0) as u8;
    let bot_rank = s["bottomSeedRankAbbrev"]
        .as_str()
        .or_else(|| bottom["seed"].as_str())
        .or_else(|| s["bottomSeedRank"].as_str())
        .map(str::to_owned)
        .or_else(|| s["bottomSeedRank"].as_u64().map(|n| n.to_string()));

    // Winner: explicit field or inferred from 4-win threshold
    let winner_abbrev = s["winningTeam"]["abbrev"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            if top_wins == 4 {
                Some(top_abbrev.clone())
            } else if bot_wins == 4 {
                Some(bot_abbrev.clone())
            } else {
                None
            }
        });

    let conference = s["conferenceAbbrev"]
        .as_str()
        .or_else(|| s["conference"].as_str())
        .map(|c| match c {
            "E" | "EAST" | "Eastern" => "Eastern".to_owned(),
            "W" | "WEST" | "Western" => "Western".to_owned(),
            other => other.to_owned(),
        });

    PlayoffSeries {
        letter,
        top_seed_abbrev: top_abbrev,
        top_seed_name: top_name,
        top_seed_wins: top_wins,
        top_seed_rank: top_rank,
        bottom_seed_abbrev: bot_abbrev,
        bottom_seed_name: bot_name,
        bottom_seed_wins: bot_wins,
        bottom_seed_rank: bot_rank,
        winner_abbrev,
        conference,
        games: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_playoff_bracket;
    use serde_json::json;

    #[test]
    fn parses_current_flat_series_shape() {
        let raw = json!({
            "season": "20252026",
            "currentRound": 1,
            "series": [{
                "seriesLetter": "A",
                "playoffRound": 1,
                "seriesTitle": "1st Round",
                "topSeedWins": 3,
                "bottomSeedWins": 1,
                "topSeedRankAbbrev": "M1",
                "bottomSeedRankAbbrev": "WC2",
                "topSeedTeam": {"abbrev": "WSH", "name": {"default": "Capitals"}},
                "bottomSeedTeam": {"abbrev": "NYR", "name": {"default": "Rangers"}}
            }]
        });

        let bracket = parse_playoff_bracket(&raw);
        let series = bracket.find_series("A").expect("series A");
        assert_eq!(series.top_seed_wins, 3);
        assert_eq!(series.bottom_seed_rank.as_deref(), Some("WC2"));
        assert_eq!(bracket.rounds[0].label, "1st Round");
    }

    #[test]
    fn parses_legacy_nested_round_shape_and_infers_winner() {
        let raw = json!({
            "seasonId": "20242025",
            "playoffRounds": [{
                "roundNumber": 2,
                "roundName": "Second Round",
                "series": [{
                    "seriesAbbrev": "I",
                    "conferenceAbbrev": "E",
                    "topSeedTeam": {"abbrev": "FLA", "wins": 4},
                    "bottomSeedTeam": {"abbrev": "TBL", "wins": 2}
                }]
            }]
        });

        let bracket = parse_playoff_bracket(&raw);
        let series = bracket.find_series("I").expect("series I");
        assert_eq!(series.winner_abbrev.as_deref(), Some("FLA"));
        assert_eq!(series.conference.as_deref(), Some("Eastern"));
        assert!(series.is_complete());
    }
}
