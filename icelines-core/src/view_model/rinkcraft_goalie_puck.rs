//! Recorded goalie actions. These counts never become capability scores.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Goalie {
    pub player_id: u64,
    pub name: String,
    pub team_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub game_id: u64,
    pub event_id: u64,
    pub player_id: u64,
    pub team_id: u64,
    pub period: u64,
    pub time_in_period: String,
    pub kind: String,
    pub penalty_description: Option<String>,
    /// Temporal context only, not possession or causation.
    pub following_opponent_attempt: Option<Attempt>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attempt {
    pub event_id: u64,
    pub kind: String,
    pub seconds_after: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Game {
    pub game_id: u64,
    pub goalies_on_roster: Vec<Goalie>,
    pub events: Vec<Event>,
}
#[derive(Deserialize)]
struct Ledger {
    schema_version: u32,
    season: u32,
    season_type: String,
    teams: Vec<String>,
    expected_games: usize,
    captured_games: usize,
    event_count: usize,
    games: Vec<Game>,
    captures: Vec<Capture>,
}
#[derive(Deserialize)]
struct Capture {
    url: String,
    sha256: String,
    captured_at: String,
    status: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metric {
    pub label: String,
    pub count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Play {
    pub label: String,
    pub game_id: u64,
    pub event_id: u64,
    pub period: u64,
    pub clock: String,
    pub game_url: String,
    pub source_url: String,
    pub following_attempt: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    pub player_id: u32,
    pub season: u32,
    pub season_type: String,
    pub coverage: String,
    pub captured_through: String,
    pub metrics: Vec<Metric>,
    pub penalty_types: Vec<Metric>,
    pub plays: Vec<Play>,
    pub note: String,
}
const KINDS: [(&str, &str); 6] = [
    ("goal", "Goals"),
    ("primary_assist", "Primary assists"),
    ("secondary_assist", "Secondary assists"),
    ("giveaway", "Giveaways"),
    ("takeaway", "Takeaways"),
    ("penalty", "Penalties"),
];

/// Recompute from game-level records, not cached summary counters.
pub fn project(source: &str, player_id: u32, season: u32) -> Result<Option<Evidence>, String> {
    if source.len() > 16 * 1024 * 1024 {
        return Err("Goalie puck ledger exceeds 16 MiB".into());
    }
    let ledger: Ledger = serde_json::from_str(source).map_err(|e| e.to_string())?;
    if ledger.schema_version != 1
        || ledger.season_type != "regular"
        || ledger.expected_games == 0
        || ledger.expected_games != ledger.captured_games
        || ledger.games.len() != ledger.captured_games
        || ledger.teams.is_empty()
        || ledger.teams.len() > 32
        || ledger.games.iter().map(|g| g.events.len()).sum::<usize>() != ledger.event_count
    {
        return Err("Incomplete or invalid goalie puck ledger".into());
    }
    if ledger.season != season {
        return Ok(None);
    }
    let mut counts = [0u32; 6];
    let mut penalties = BTreeMap::<String, u32>::new();
    let mut plays = vec![];
    let mut found = false;
    let mut through = String::new();
    let mut seen = BTreeSet::new();
    for game in &ledger.games {
        if game.game_id / 1_000_000 != (season / 10000) as u64 || !seen.insert(game.game_id) {
            return Err("Duplicate or wrong-season game".into());
        }
        let url = format!(
            "https://api-web.nhle.com/v1/gamecenter/{}/play-by-play",
            game.game_id
        );
        let capture = ledger
            .captures
            .iter()
            .find(|c| c.url == url)
            .ok_or("Game capture absent")?;
        if capture.status != 200
            || capture.sha256.len() != 64
            || !capture.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid game capture".into());
        }
        let stamp = chrono::DateTime::parse_from_rfc3339(&capture.captured_at)
            .map_err(|e| e.to_string())?;
        if through.is_empty()
            || stamp > chrono::DateTime::parse_from_rfc3339(&through).map_err(|e| e.to_string())?
        {
            through = capture.captured_at.clone();
        }
        let roster: BTreeMap<_, _> = game
            .goalies_on_roster
            .iter()
            .map(|g| (g.player_id, g.team_id))
            .collect();
        if roster.len() != game.goalies_on_roster.len() {
            return Err("Duplicate goalie identity".into());
        }
        found |= roster.contains_key(&(player_id as u64));
        let mut events = BTreeSet::new();
        for event in &game.events {
            let index = KINDS
                .iter()
                .position(|(kind, _)| *kind == event.kind)
                .ok_or("Unknown goalie action")?;
            if event.game_id != game.game_id
                || roster.get(&event.player_id) != Some(&event.team_id)
                || !events.insert((event.event_id, event.player_id, event.kind.clone()))
            {
                return Err("Invalid credited goalie action".into());
            }
            if let Some(a) = &event.following_opponent_attempt {
                if event.kind != "giveaway"
                    || a.seconds_after > 10
                    || !matches!(a.kind.as_str(), "shot-on-goal" | "goal")
                {
                    return Err("Invalid following attempt".into());
                }
            }
            if event.player_id != player_id as u64 {
                continue;
            }
            counts[index] += 1;
            let label = if event.kind == "penalty" {
                let desc = event
                    .penalty_description
                    .as_deref()
                    .unwrap_or("unspecified")
                    .replace('-', " ");
                *penalties.entry(desc.clone()).or_default() += 1;
                format!("Penalty · {desc}")
            } else {
                match event.kind.as_str() {
                    "goal" => "Goal",
                    "primary_assist" => "Primary assist",
                    "secondary_assist" => "Secondary assist",
                    "giveaway" => "Giveaway",
                    _ => "Takeaway",
                }
                .into()
            };
            plays.push(Play {
                label,
                game_id: game.game_id,
                event_id: event.event_id,
                period: event.period,
                clock: event.time_in_period.clone(),
                game_url: format!("https://www.nhl.com/gamecenter/{}", game.game_id),
                source_url: url.clone(),
                following_attempt: event.following_opponent_attempt.as_ref().map(|a| {
                    format!(
                        "Opponent {} {}s later; timing, not causation.",
                        if a.kind == "goal" {
                            "goal"
                        } else {
                            "shot on goal"
                        },
                        a.seconds_after
                    )
                }),
            });
        }
    }
    if !found {
        return Ok(None);
    }
    Ok(Some(Evidence {player_id,season,season_type:"regular".into(),
        coverage:format!("{} captured games · {} selected clubs",ledger.captured_games,ledger.teams.len()),
        captured_through:through,metrics:KINDS.iter().zip(counts).map(|((_,label),count)|Metric{label:(*label).into(),count}).collect(),
        penalty_types:penalties.into_iter().map(|(label,count)|Metric{label,count}).collect(),plays,
        note:"Recorded game-log counts, not projected totals or handling success rates. The game sample includes opponents and backups; it is not this goalie's GP. Source snapshot dated separately from season statistics.".into() }))
}

/// Select only the card's goalie/regular-season window; never attach to a skater.
pub fn attach(
    profile: &mut super::rinkcraft_profile::RinkcraftStatProfile,
    source: &str,
) -> Result<(), String> {
    profile.goalie_puck_events = None;
    if profile.request.season_type != crate::season_stats::SeasonType::Regular {
        return Ok(());
    }
    let target = profile
        .numerical_evidence
        .as_ref()
        .map(|n| n.season)
        .or_else(|| profile.latest_available.as_ref().map(|s| s.season.0));
    let Some(row) = profile.timeline.iter().find(|s| Some(s.season.0) == target) else {
        return Ok(());
    };
    if row.position != crate::model::Position::Goalie {
        return Ok(());
    }
    profile.goalie_puck_events = project(source, profile.request.player_id.0, row.season.0)?;
    Ok(())
}
