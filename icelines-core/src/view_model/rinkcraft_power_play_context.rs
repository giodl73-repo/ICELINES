//! Matched power-play exposure and assist composition, not unit assignment.
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
    pub summary: String,
    pub time_share: String,
    pub assist_share: String,
    pub pp_assists: String,
    pub outside_assists: String,
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
    let c = p.skater_context.as_ref()?;
    let value = |key: &str| {
        c.values
            .get(key)
            .copied()
            .filter(|v| v.is_finite() && *v >= 0.)
    };
    let share = value("pp-share").filter(|v| *v <= 1.)?;
    let pp = value("pp-assists");
    let matched = pp.zip(value("assists")).filter(|(rate, all)| {
        rate * share <= *all + 1e-8
            && (share > 0. || *rate == 0.)
            && (share < 1. || (*rate - *all).abs() <= 1e-8)
    });
    let portion = matched
        .filter(|(_, all)| *all > 0.)
        .map(|(rate, all)| (rate * share / all).clamp(0., 1.));
    let assist_share = portion
        .map(|v| format!("{:.1}% of assists", 100. * v))
        .unwrap_or_else(|| {
            if matched.is_some_and(|(_, all)| all == 0.) {
                "No credited assists".into()
            } else {
                "Assist share unavailable".into()
            }
        });
    let outside = matched
        .filter(|_| share < 1.)
        .map(|(rate, all)| ((all - rate * share) / (1. - share)).max(0.))
        .filter(|v| v.is_finite());
    let rate = |v: Option<f64>| {
        v.map(|v| format!("{v:.2} /60"))
            .unwrap_or_else(|| "Unavailable".into())
    };
    let time_share = format!("{:.1}% of own TOI", 100. * share);
    Some(Record {schema:"rinkcraft.power-play-context.v1".into(),player_id:p.player_id,season:p.season,
        group:p.group.clone(),games:p.games,minutes:p.minutes,
        summary:format!("Power play · {}",if portion.is_some(){assist_share.clone()}else{time_share.clone()}),
        time_share,assist_share,pp_assists:if share>0. {rate(pp)}else{"No PP time".into()},
        outside_assists:if share<1. {rate(outside)}else{"No time outside PP".into()},
        formula:"PP assist share = (PP assists/60 × PP share of own TOI) / all-strength assists/60. Outside-PP assist rate = (all-strength assists/60 − PP assists/60 × PP time share) / (1 − PP time share). Uses unrounded matched NHL inputs; absent or inconsistent components have no composition estimate.".into(),
        note:"Assist shares describe credited production; time shares describe deployment. Each rate uses its own strength-specific minutes. Outside PP includes all remaining situations, not just five-on-five. Teammates, opportunity and finishing affect assists. This does not identify PP1/PP2, a station, passing success or isolated player impact.".into()})
}
pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
    if build(p).as_ref() != Some(r) {
        Err("Power-play context does not match admitted inputs/window".into())
    } else {
        Ok(())
    }
}
