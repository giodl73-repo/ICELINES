//! Observed shot-type repertoire. These counts do not assign class or technique mastery.
use super::rinkcraft_stat_scores::Profile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Counts {
    pub attempts: u32,
    pub on_goal: u32,
    pub goals: u32,
}
impl Counts {
    pub fn add(&mut self, c: &Self) {
        self.attempts += c.attempts;
        self.on_goal += c.on_goal;
        self.goals += c.goals;
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub player_id: u32,
    pub season: u32,
    pub roster_games: u32,
    pub coverage_games: u32,
    pub by_type: BTreeMap<String, Counts>,
    pub captured_at: String,
    pub source_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Method {
    pub id: String,
    pub name: String,
    pub attempts: u32,
    pub on_goal: u32,
    pub goals: u32,
    pub result: String,
    pub share: String,
    pub outcomes: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub typed_attempts: u32,
    pub total_attempts: u32,
    pub coverage: String,
    pub primary: Vec<Method>,
    pub additional: Vec<Method>,
    pub unknown_display: String,
    pub captured_at: String,
    pub note: String,
}
pub fn label(key: &str) -> Option<&'static str> {
    Some(match key {
        "wrist" => "Wrist shot",
        "snap" => "Snap shot",
        "slap" => "Slap shot",
        "backhand" => "Backhand",
        "tip-in" => "Tip-in",
        "deflected" => "Deflection",
        "wrap-around" => "Wraparound",
        "bat" => "Bat shot",
        "poke" => "Poke shot",
        "between-legs" => "Between legs",
        "cradle" => "Cradle",
        _ => return None,
    })
}
pub fn validate_sample(p: &Profile, s: &Sample) -> Result<(), String> {
    if !["Forwards", "Defense"].contains(&p.group.as_str())
        || s.player_id != p.player_id
        || s.season != p.season
        || s.roster_games == 0
        || s.roster_games > s.coverage_games
        || s.coverage_games > 2000
        || s.by_type.len() > 12
        || s.by_type.iter().any(|(k, c)| {
            (k != "unknown" && label(k).is_none())
                || c.attempts == 0
                || c.goals > c.on_goal
                || c.on_goal > c.attempts
        })
        || s.by_type
            .values()
            .map(|c| u64::from(c.attempts))
            .sum::<u64>()
            > 40_000_000
        || s.source_sha256.len() != 64
        || !s.source_sha256.bytes().all(|c| c.is_ascii_hexdigit())
        || chrono::DateTime::parse_from_rfc3339(&s.captured_at).is_err()
    {
        return Err("Invalid shot-method sample".into());
    }
    Ok(())
}
pub fn build(p: &Profile) -> Option<Record> {
    let s = p.shot_method_sample.as_ref()?;
    validate_sample(p, s).ok()?;
    let typed_attempts = s
        .by_type
        .iter()
        .filter(|(k, _)| label(k).is_some())
        .map(|(_, c)| c.attempts)
        .sum::<u32>();
    let total_attempts = s.by_type.values().map(|c| c.attempts).sum();
    let mut methods = s
        .by_type
        .iter()
        .filter_map(|(key, c)| {
            label(key).map(|name| Method {
                id: key.clone(),
                name: name.into(),
                attempts: c.attempts,
                on_goal: c.on_goal,
                goals: c.goals,
                result: format!("Attempts {} · Goals {}", c.attempts, c.goals),
                share: format!(
                    "{:.1}% of typed attempts",
                    100. * f64::from(c.attempts) / f64::from(typed_attempts)
                ),
                outcomes: format!("On goal {} · Misses {}", c.on_goal, c.attempts - c.on_goal),
            })
        })
        .collect::<Vec<_>>();
    methods.sort_by(|a, b| b.attempts.cmp(&a.attempts).then(a.id.cmp(&b.id)));
    let additional = methods.split_off(methods.len().min(3));
    let coverage = if total_attempts == 0 {
        format!(
            "No unblocked attempts · {} cached roster games",
            s.roster_games
        )
    } else {
        format!(
            "{typed_attempts}/{total_attempts} attempts typed · {} cached roster games",
            s.roster_games
        )
    };
    Some(Record {schema:"rinkcraft.shot-methods.v1".into(),player_id:p.player_id,season:p.season,group:p.group.clone(),games:p.games,typed_attempts,total_attempts,
        coverage,primary:methods,additional,unknown_display:format!("{} attempts without a supported type",total_attempts-typed_attempts),captured_at:s.captured_at.clone(),
        note:"Observed NHL event shot types, not mastery, a new feat or a class assignment. Attempts include goals, saved shots and misses; blocks and shootout events are excluded. Goals are included in on-goal counts. Shares divide by supported typed attempts in this event sample, not official season shots or TOI. All manpower states and regular-season overtime are included. Unknown types stay unclassified. Provider type credits do not establish intent. These labels do not establish one-timers, quick release, deception, passing methods or goalie technique. Capture windows can differ from the season-stat snapshot.".into()})
}
pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
    if build(p).as_ref() != Some(r) {
        Err("Shot methods do not match their sample/window".into())
    } else {
        Ok(())
    }
}
