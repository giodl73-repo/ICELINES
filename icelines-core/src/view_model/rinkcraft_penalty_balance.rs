//! Matched all-strength penalty-call rates; not a power-play or judgment rating.
use super::rinkcraft_stat_scores::Profile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    pub drawn: String,
    pub taken: String,
    pub balance: String,
    pub formula: String,
    pub note: String,
}

pub fn build(p: &Profile) -> Option<Record> {
    if !["Forwards", "Defense"].contains(&p.group.as_str())
        || p.games == 0
        || !p.minutes.is_finite()
        || p.minutes <= 0.
        || !p.scope.to_ascii_lowercase().starts_with("all strengths")
    {
        return None;
    }
    let context = p.skater_context.as_ref()?;
    let rate = |key: &str| {
        context
            .values
            .get(key)
            .copied()
            .filter(|v| v.is_finite() && *v >= 0.)
    };
    let drawn = rate("drawn");
    let taken = rate("taken");
    if drawn.is_none() && taken.is_none() {
        return None;
    }
    let text = |v: Option<f64>| {
        v.map(|v| format!("{v:.2} /60"))
            .unwrap_or_else(|| "Unavailable".into())
    };
    let balance = drawn
        .zip(taken)
        .map(|(d, t)| {
            // Avoid a misleading negative zero after display rounding.
            let value = d - t;
            format!("{:+.2} /60", if value.abs() < 0.005 { 0. } else { value })
        })
        .unwrap_or_else(|| "Unavailable".into());
    Some(Record {
        schema: "rinkcraft.penalty-balance.v1".into(),
        player_id: p.player_id, season: p.season, group: p.group.clone(),
        games: p.games, minutes: p.minutes,
        drawn: text(drawn), taken: text(taken), balance,
        formula: "Each rate = NHL recorded penalty calls × 60 / official all-strength minutes. Balance = drawn rate − taken rate, calculated before display rounding. Positive means more calls drawn than taken per 60; negative means more taken. Missing inputs do not become zero.".into(),
        note: "Recorded calls, not penalty minutes, power plays created, goals gained or decision quality. Coincidental penalties, majors and other calls can have different manpower consequences. Role, possession, opponents and officiating affect these observations. No ten-GP gate, new feat, capability change, class assignment or simulation bonus.".into(),
    })
}

pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
    if build(p).as_ref() != Some(r) {
        Err("Penalty balance does not match admitted inputs/window".into())
    } else {
        Ok(())
    }
}
