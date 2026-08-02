use icelines_core::{ScoringEventInput, ShotEventKind, ShotLocation, TeamSide};

// ── Boxscore types (Phase 7c gap-fix) ─────────────────────────────────────────

/// One goal scored in a game.
#[derive(Debug, Clone)]
pub struct Goal {
    pub scorer_id: Option<u32>,
    pub period: u8,             // 1, 2, 3, OT=4+
    pub period_type: String,    // "REG" | "OT" | "SO"
    pub time_in_period: String, // "MM:SS"
    pub scorer_name: String,
    pub scorer_team: String, // home/away abbrev
    pub assist1_name: Option<String>,
    pub assist2_name: Option<String>,
    pub away_score: u8,
    pub home_score: u8,
}

/// Goalie line for one team's starting goalie in a game.
#[derive(Debug, Clone)]
pub struct GoalieLine {
    /// NHL player_id from playerByGameStats.{home,away}Team.goalies[].playerId.
    /// Phase Foster +24 — was missing pre-v0.18, forcing favorites
    /// to do a name-substring match. Now PID-aware. Kept Optional
    /// for resilience against API shape drift; consumers fall back
    /// to name match when 0.
    pub player_id: u32,
    pub player_name: String,
    pub team_abbrev: String,
    pub saves: u32,
    pub shots: u32,
    pub decision: Option<String>, // "W" | "L" | "OTL" | None
}

/// One skater's line in a single game's boxscore. Sourced from
/// `playerByGameStats.{home,away}Team.{forwards,defense}` on
/// `/v1/gamecenter/{id}/boxscore`. Used by the game-detail screen to
/// pick out per-team stat leaders (TOI, SOG, Hits, Blocks, Takeaways).
#[derive(Debug, Clone)]
pub struct SkaterLine {
    pub player_id: u32,
    pub player_name: String,
    pub team_abbrev: String,
    pub position: String, // "C" | "L" | "R" | "D"
    /// Time on ice in seconds. Parsed from the API's "MM:SS" string.
    pub toi_seconds: u32,
    pub goals: u32,
    pub assists: u32,
    pub plus_minus: i32,
    pub sog: u32,
    pub hits: u32,
    pub blocked_shots: u32,
    pub takeaways: u32,
    pub giveaways: u32,
    pub pim: u32,
}

/// Detailed boxscore for one game.
#[derive(Debug, Clone)]
pub struct Boxscore {
    pub game_id: u64,
    pub away_abbrev: String,
    pub home_abbrev: String,
    pub away_score: u8,
    pub home_score: u8,
    pub game_state: Option<String>,
    pub last_period: Option<String>,
    pub goals: Vec<Goal>,
    pub goalies: Vec<GoalieLine>,
    /// Per-team skater rows with full stat block. `away_skaters` first,
    /// `home_skaters` second. Empty when the boxscore endpoint
    /// pre-dates the `playerByGameStats` schema.
    pub away_skaters: Vec<SkaterLine>,
    pub home_skaters: Vec<SkaterLine>,
}

/// Event-level play-by-play projection for one game.
#[derive(Debug, Clone)]
pub struct PlayByPlay {
    pub game_id: u64,
    pub game_date: Option<String>,
    pub away_team_id: Option<u32>,
    pub away_abbrev: String,
    pub home_team_id: Option<u32>,
    pub home_abbrev: String,
    pub goals: Vec<PlayByPlayGoal>,
    pub penalties: Vec<PlayByPlayPenalty>,
    pub scoring_events: Vec<ScoringEventInput>,
}

#[derive(Debug, Clone)]
pub struct PlayByPlayGoal {
    pub event_id: u32,
    pub period: u8,
    pub period_type: String,
    pub time_in_period: String,
    pub situation_code: Option<String>,
    pub event_owner_team_id: Option<u32>,
    pub scoring_player_id: Option<u32>,
    pub goalie_in_net_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PlayByPlayPenalty {
    pub event_id: u32,
    pub period: u8,
    pub period_type: String,
    pub time_in_period: String,
    pub situation_code: Option<String>,
    pub event_owner_team_id: Option<u32>,
    pub penalty_type: Option<String>,
    pub desc_key: Option<String>,
    pub duration: Option<u32>,
    pub committed_by_player_id: Option<u32>,
    pub drawn_by_player_id: Option<u32>,
}

/// Defensive boxscore parser. NHL's boxscore endpoint shape varies — this
/// accepts the common forms and silently drops fields it doesn't recognize.
pub fn parse_boxscore(raw: &serde_json::Value, game_id: u64) -> Boxscore {
    let away_abbrev = raw["awayTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let home_abbrev = raw["homeTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let away_score = raw["awayTeam"]["score"].as_u64().unwrap_or(0) as u8;
    let home_score = raw["homeTeam"]["score"].as_u64().unwrap_or(0) as u8;
    let game_state = raw["gameState"].as_str().map(str::to_owned);
    let last_period = raw["gameOutcome"]["lastPeriodType"]
        .as_str()
        .map(str::to_owned);

    // Goals — try a few common nesting paths
    let mut goals = Vec::new();
    let goal_arrays: Vec<&serde_json::Value> =
        if let Some(arr) = raw["summary"]["scoring"].as_array() {
            // Newer endpoint: summary.scoring is array of period blocks; each has "goals"
            arr.iter().collect()
        } else if let Some(arr) = raw["scoring"].as_array() {
            arr.iter().collect()
        } else {
            Vec::new()
        };

    for period_block in goal_arrays {
        let period_num = period_block["periodDescriptor"]["number"]
            .as_u64()
            .or_else(|| period_block["period"].as_u64())
            .unwrap_or(0) as u8;
        let period_type = period_block["periodDescriptor"]["periodType"]
            .as_str()
            .or_else(|| period_block["periodType"].as_str())
            .unwrap_or("REG")
            .to_owned();

        if let Some(g_arr) = period_block["goals"].as_array() {
            for g in g_arr {
                let scorer_name = g["firstName"]["default"]
                    .as_str()
                    .map(|fn_| {
                        let ln = g["lastName"]["default"].as_str().unwrap_or("");
                        format!("{fn_} {ln}").trim().to_owned()
                    })
                    .or_else(|| g["name"]["default"].as_str().map(str::to_owned))
                    .or_else(|| g["scorer"].as_str().map(str::to_owned))
                    .unwrap_or_default();
                let scorer_id = goal_player_id(g);
                let scorer_team = g["teamAbbrev"]["default"]
                    .as_str()
                    .or_else(|| g["teamAbbrev"].as_str())
                    .unwrap_or("")
                    .to_owned();
                let time_in_period = g["timeInPeriod"]
                    .as_str()
                    .or_else(|| g["time"].as_str())
                    .unwrap_or("")
                    .to_owned();

                // Assists: prefer structured array
                let mut assists: Vec<String> = Vec::new();
                if let Some(arr) = g["assists"].as_array() {
                    for a in arr {
                        if let Some(name) = a["name"]["default"].as_str() {
                            assists.push(name.to_owned());
                        } else if let (Some(fnm), Some(lnm)) = (
                            a["firstName"]["default"].as_str(),
                            a["lastName"]["default"].as_str(),
                        ) {
                            assists.push(format!("{fnm} {lnm}"));
                        }
                    }
                }
                let assist1_name = assists.first().cloned();
                let assist2_name = assists.get(1).cloned();

                let aw_score = g["awayScore"].as_u64().unwrap_or(0) as u8;
                let hm_score = g["homeScore"].as_u64().unwrap_or(0) as u8;

                goals.push(Goal {
                    scorer_id,
                    period: period_num,
                    period_type: period_type.clone(),
                    time_in_period,
                    scorer_name,
                    scorer_team,
                    assist1_name,
                    assist2_name,
                    away_score: aw_score,
                    home_score: hm_score,
                });
            }
        }
    }

    // Goalies — try common shapes: playerByGameStats.{home,away}Team.goalies / boxscore.goalies
    let mut goalies = Vec::new();
    let goalie_paths = [
        (
            &raw["playerByGameStats"]["awayTeam"]["goalies"],
            away_abbrev.as_str(),
        ),
        (
            &raw["playerByGameStats"]["homeTeam"]["goalies"],
            home_abbrev.as_str(),
        ),
        (
            &raw["boxscore"]["awayTeam"]["goalies"],
            away_abbrev.as_str(),
        ),
        (
            &raw["boxscore"]["homeTeam"]["goalies"],
            home_abbrev.as_str(),
        ),
    ];
    for (val, team) in goalie_paths {
        if let Some(arr) = val.as_array() {
            for g in arr {
                let player_id = g["playerId"].as_u64().unwrap_or(0) as u32;
                let player_name = g["name"]["default"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| {
                        let fnm = g["firstName"]["default"].as_str()?;
                        let lnm = g["lastName"]["default"].as_str().unwrap_or("");
                        Some(format!("{fnm} {lnm}").trim().to_owned())
                    })
                    .unwrap_or_default();
                let saves = g["saves"].as_u64().unwrap_or(0) as u32;
                let shots = g["shotsAgainst"]
                    .as_u64()
                    .or_else(|| g["shots"].as_u64())
                    .unwrap_or(0) as u32;
                let decision = g["decision"].as_str().map(str::to_owned);
                if !player_name.is_empty() {
                    goalies.push(GoalieLine {
                        player_id,
                        player_name,
                        team_abbrev: team.to_owned(),
                        saves,
                        shots,
                        decision,
                    });
                }
            }
        }
    }

    // Per-team skater stats from `playerByGameStats.{home,away}Team.
    // {forwards,defense}`. Goalies live alongside but are already
    // pulled into the dedicated `goalies` array above.
    let pgs = &raw["playerByGameStats"];
    let away_skaters = parse_skater_lines(&pgs["awayTeam"], &away_abbrev);
    let home_skaters = parse_skater_lines(&pgs["homeTeam"], &home_abbrev);

    Boxscore {
        game_id,
        away_abbrev,
        home_abbrev,
        away_score,
        home_score,
        game_state,
        last_period,
        goals,
        goalies,
        away_skaters,
        home_skaters,
    }
}

/// Parse the NHL web play-by-play endpoint into the event projection needed by
/// records and scoring reports. Unknown event families are intentionally ignored.
pub fn parse_play_by_play(raw: &serde_json::Value, fallback_game_id: u64) -> PlayByPlay {
    let game_id = raw["id"].as_u64().unwrap_or(fallback_game_id);
    let game_date = raw["gameDate"].as_str().map(str::to_owned);
    let away_team_id = play_u32(&raw["awayTeam"], "id");
    let away_abbrev = raw["awayTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let home_team_id = play_u32(&raw["homeTeam"], "id");
    let home_abbrev = raw["homeTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let mut goals = Vec::new();
    let mut penalties = Vec::new();
    let mut scoring_events = Vec::new();

    if let Some(plays) = raw["plays"].as_array() {
        for play in plays {
            match play["typeDescKey"].as_str() {
                Some("goal") => {
                    goals.push(parse_play_by_play_goal(play));
                    scoring_events.push(parse_play_by_play_scoring_event(
                        play,
                        game_id,
                        game_date.clone(),
                        ShotEventKind::Goal,
                        TeamLookup {
                            away_team_id,
                            away_abbrev: &away_abbrev,
                            home_team_id,
                            home_abbrev: &home_abbrev,
                        },
                    ));
                }
                Some("shot-on-goal") => scoring_events.push(parse_play_by_play_scoring_event(
                    play,
                    game_id,
                    game_date.clone(),
                    ShotEventKind::ShotOnGoal,
                    TeamLookup {
                        away_team_id,
                        away_abbrev: &away_abbrev,
                        home_team_id,
                        home_abbrev: &home_abbrev,
                    },
                )),
                Some("missed-shot") => scoring_events.push(parse_play_by_play_scoring_event(
                    play,
                    game_id,
                    game_date.clone(),
                    ShotEventKind::MissedShot,
                    TeamLookup {
                        away_team_id,
                        away_abbrev: &away_abbrev,
                        home_team_id,
                        home_abbrev: &home_abbrev,
                    },
                )),
                Some("blocked-shot") => scoring_events.push(parse_play_by_play_scoring_event(
                    play,
                    game_id,
                    game_date.clone(),
                    ShotEventKind::BlockedShot,
                    TeamLookup {
                        away_team_id,
                        away_abbrev: &away_abbrev,
                        home_team_id,
                        home_abbrev: &home_abbrev,
                    },
                )),
                Some("penalty") => penalties.push(parse_play_by_play_penalty(play)),
                _ => {}
            }
        }
    }

    PlayByPlay {
        game_id,
        game_date,
        away_team_id,
        away_abbrev,
        home_team_id,
        home_abbrev,
        goals,
        penalties,
        scoring_events,
    }
}

#[derive(Clone, Copy)]
struct TeamLookup<'a> {
    away_team_id: Option<u32>,
    away_abbrev: &'a str,
    home_team_id: Option<u32>,
    home_abbrev: &'a str,
}

fn parse_play_by_play_scoring_event(
    play: &serde_json::Value,
    game_id: u64,
    date: Option<String>,
    kind: ShotEventKind,
    teams: TeamLookup<'_>,
) -> ScoringEventInput {
    let details = &play["details"];
    let event_owner_team_id = play_u32(details, "eventOwnerTeamId");
    let scoring_player_id = play_u32(details, "scoringPlayerId");
    let shooting_player_id = play_u32(details, "shootingPlayerId").or(scoring_player_id);
    ScoringEventInput {
        game_id,
        event_id: play_u32(play, "eventId").unwrap_or(0),
        date,
        kind,
        period: period_number(play),
        period_type: period_type(play),
        time_in_period: play["timeInPeriod"].as_str().unwrap_or("").to_owned(),
        situation_code: play["situationCode"].as_str().map(str::to_owned),
        event_owner_team_id,
        event_owner_team_abbrev: team_abbrev_for_event_owner_id(event_owner_team_id, teams),
        event_owner_side: team_side_for_event_owner_id(event_owner_team_id, teams),
        shooting_player_id,
        scoring_player_id,
        blocking_player_id: play_u32(details, "blockingPlayerId"),
        goalie_in_net_id: play_u32(details, "goalieInNetId"),
        location: ShotLocation {
            x_coord: play_i16(details, "xCoord"),
            y_coord: play_i16(details, "yCoord"),
            zone_code: details["zoneCode"].as_str().map(str::to_owned),
        },
        shot_type: details["shotType"].as_str().map(str::to_owned),
        reason: details["reason"].as_str().map(str::to_owned),
        home_team_defending_side: play["homeTeamDefendingSide"].as_str().map(str::to_owned),
        away_score: play_u8(details, "awayScore"),
        home_score: play_u8(details, "homeScore"),
    }
}

fn team_side_for_event_owner_id(
    event_owner_team_id: Option<u32>,
    teams: TeamLookup<'_>,
) -> Option<TeamSide> {
    match event_owner_team_id {
        Some(id) if Some(id) == teams.away_team_id => Some(TeamSide::Away),
        Some(id) if Some(id) == teams.home_team_id => Some(TeamSide::Home),
        _ => None,
    }
}

fn team_abbrev_for_event_owner_id(
    event_owner_team_id: Option<u32>,
    teams: TeamLookup<'_>,
) -> Option<String> {
    match event_owner_team_id {
        Some(id) if Some(id) == teams.away_team_id && !teams.away_abbrev.is_empty() => {
            Some(teams.away_abbrev.to_owned())
        }
        Some(id) if Some(id) == teams.home_team_id && !teams.home_abbrev.is_empty() => {
            Some(teams.home_abbrev.to_owned())
        }
        _ => None,
    }
}

fn parse_play_by_play_goal(play: &serde_json::Value) -> PlayByPlayGoal {
    let details = &play["details"];
    PlayByPlayGoal {
        event_id: play_u32(play, "eventId").unwrap_or(0),
        period: period_number(play),
        period_type: period_type(play),
        time_in_period: play["timeInPeriod"].as_str().unwrap_or("").to_owned(),
        situation_code: play["situationCode"].as_str().map(str::to_owned),
        event_owner_team_id: play_u32(details, "eventOwnerTeamId"),
        scoring_player_id: play_u32(details, "scoringPlayerId"),
        goalie_in_net_id: play_u32(details, "goalieInNetId"),
    }
}

fn parse_play_by_play_penalty(play: &serde_json::Value) -> PlayByPlayPenalty {
    let details = &play["details"];
    PlayByPlayPenalty {
        event_id: play_u32(play, "eventId").unwrap_or(0),
        period: period_number(play),
        period_type: period_type(play),
        time_in_period: play["timeInPeriod"].as_str().unwrap_or("").to_owned(),
        situation_code: play["situationCode"].as_str().map(str::to_owned),
        event_owner_team_id: play_u32(details, "eventOwnerTeamId"),
        penalty_type: details["typeCode"].as_str().map(str::to_owned),
        desc_key: details["descKey"].as_str().map(str::to_owned),
        duration: play_u32(details, "duration"),
        committed_by_player_id: play_u32(details, "committedByPlayerId"),
        drawn_by_player_id: play_u32(details, "drawnByPlayerId"),
    }
}

fn period_number(play: &serde_json::Value) -> u8 {
    play["periodDescriptor"]["number"]
        .as_u64()
        .or_else(|| play["period"].as_u64())
        .unwrap_or(0) as u8
}

fn period_type(play: &serde_json::Value) -> String {
    play["periodDescriptor"]["periodType"]
        .as_str()
        .or_else(|| play["periodType"].as_str())
        .unwrap_or("REG")
        .to_owned()
}

fn play_u32(value: &serde_json::Value, key: &str) -> Option<u32> {
    value[key]
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
        .filter(|id| *id != 0)
}

fn play_u8(value: &serde_json::Value, key: &str) -> Option<u8> {
    value[key].as_u64().and_then(|id| u8::try_from(id).ok())
}

fn play_i16(value: &serde_json::Value, key: &str) -> Option<i16> {
    value[key]
        .as_i64()
        .and_then(|coord| i16::try_from(coord).ok())
}

fn goal_player_id(g: &serde_json::Value) -> Option<u32> {
    [
        &g["playerId"],
        &g["scorerPlayerId"],
        &g["scoringPlayerId"],
        &g["scorerId"],
        &g["player"]["playerId"],
        &g["player"]["id"],
    ]
    .iter()
    .find_map(|value| value.as_u64())
    .and_then(|id| u32::try_from(id).ok())
    .filter(|id| *id != 0)
}

/// Pull all forwards + defense out of one team's `playerByGameStats`
/// block. Goalies are intentionally excluded — they're handled by the
/// dedicated `goalies` parsing path above. Returns an empty Vec when
/// the `playerByGameStats` shape isn't present (older boxscore
/// endpoints; partial responses while a game is loading).
fn parse_skater_lines(team: &serde_json::Value, abbrev: &str) -> Vec<SkaterLine> {
    let mut out = Vec::new();
    for group in &["forwards", "defense"] {
        let Some(arr) = team[group].as_array() else {
            continue;
        };
        for p in arr {
            let player_id = p["playerId"].as_u64().unwrap_or(0) as u32;
            let player_name = p["name"]["default"]
                .as_str()
                .or_else(|| p["name"].as_str())
                .unwrap_or("")
                .to_owned();
            let position = p["position"].as_str().unwrap_or("").to_owned();
            let toi_seconds = parse_mmss(p["toi"].as_str().unwrap_or("0:00"));
            out.push(SkaterLine {
                player_id,
                player_name,
                team_abbrev: abbrev.to_owned(),
                position,
                toi_seconds,
                goals: p["goals"].as_u64().unwrap_or(0) as u32,
                assists: p["assists"].as_u64().unwrap_or(0) as u32,
                plus_minus: p["plusMinus"].as_i64().unwrap_or(0) as i32,
                sog: p["sog"].as_u64().unwrap_or(0) as u32,
                hits: p["hits"].as_u64().unwrap_or(0) as u32,
                blocked_shots: p["blockedShots"].as_u64().unwrap_or(0) as u32,
                takeaways: p["takeaways"].as_u64().unwrap_or(0) as u32,
                giveaways: p["giveaways"].as_u64().unwrap_or(0) as u32,
                pim: p["pim"].as_u64().unwrap_or(0) as u32,
            });
        }
    }
    out
}

/// Parse "MM:SS" → seconds. Returns 0 on malformed input. Handles the
/// boxscore convention where TOI is published as a colon-separated
/// minutes-seconds string ("18:45") rather than a number.
fn parse_mmss(s: &str) -> u32 {
    let mut parts = s.splitn(2, ':');
    let m = parts
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    let s = parts
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    m * 60 + s
}

#[cfg(test)]
mod tests {
    use super::{parse_boxscore, parse_play_by_play};
    use icelines_core::{ShotEventKind, TeamSide};
    use serde_json::json;

    #[test]
    fn parses_boxscore_goal_and_skater_lines() {
        let raw = json!({
            "awayTeam": {"abbrev": "SEA", "score": 1},
            "homeTeam": {"abbrev": "EDM", "score": 0},
            "summary": {"scoring": [{
                "periodDescriptor": {"number": 1, "periodType": "REG"},
                "goals": [{
                    "playerId": 8477444,
                    "firstName": {"default": "Andre"},
                    "lastName": {"default": "Burakovsky"},
                    "teamAbbrev": {"default": "SEA"},
                    "timeInPeriod": "04:12",
                    "awayScore": 1,
                    "homeScore": 0
                }]
            }]},
            "playerByGameStats": {"awayTeam": {"forwards": [{
                "playerId": 8477444,
                "name": {"default": "Andre Burakovsky"},
                "position": "L",
                "toi": "18:45",
                "goals": 1,
                "sog": 4
            }]}}
        });

        let parsed = parse_boxscore(&raw, 2025020001);
        assert_eq!(parsed.goals[0].scorer_id, Some(8477444));
        assert_eq!(parsed.away_skaters[0].toi_seconds, 1125);
        assert_eq!(parsed.away_skaters[0].sog, 4);
    }

    #[test]
    fn projects_play_by_play_shot_with_team_and_location() {
        let raw = json!({
            "id": 2025020001_u64,
            "gameDate": "2026-10-10",
            "awayTeam": {"id": 55, "abbrev": "SEA"},
            "homeTeam": {"id": 22, "abbrev": "EDM"},
            "plays": [{
                "eventId": 154,
                "periodDescriptor": {"number": 1, "periodType": "REG"},
                "timeInPeriod": "09:48",
                "typeDescKey": "shot-on-goal",
                "details": {
                    "eventOwnerTeamId": 55,
                    "shootingPlayerId": 8477444,
                    "goalieInNetId": 8479973,
                    "xCoord": 74,
                    "yCoord": -8
                }
            }]
        });

        let parsed = parse_play_by_play(&raw, 0);
        let event = &parsed.scoring_events[0];
        assert_eq!(event.kind, ShotEventKind::ShotOnGoal);
        assert_eq!(event.event_owner_side, Some(TeamSide::Away));
        assert_eq!(event.event_owner_team_abbrev.as_deref(), Some("SEA"));
        assert_eq!(event.location.x_coord, Some(74));
    }
}
