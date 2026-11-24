//! Two distinct descriptive records. No ability score or feat award changes.
use super::{
    rinkcraft_attribute_calibration::percentile,
    rinkcraft_skater_context::Row,
    rinkcraft_stat_scores::{self as scores, Profile},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub minutes_5v5: Option<f64>,
    pub activity: String,
    pub chances: String,
    pub on_off: String,
    pub rank: Option<f64>,
    pub peers: usize,
    pub rows: Vec<Row>,
    pub note: String,
}
impl Record {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.defending-record.v1"
            || self.player_id == 0
            || !["Forwards", "Defense"].contains(&self.group.as_str())
            || self.minutes_5v5.is_some_and(|m| !m.is_finite() || m < 0.)
            || self.rank.is_some_and(|r| {
                !r.is_finite()
                    || !(0. ..=100.).contains(&r)
                    || self.peers < 20
                    || self.games < 10
                    || self.minutes_5v5.unwrap_or(0.) < 100.
            })
            || self.rows.len() != 8
            || self.activity.is_empty()
            || self.chances.is_empty()
            || self.on_off.is_empty()
        {
            return Err("Invalid defending record".into());
        }
        Ok(())
    }
}
fn value(p: &Profile, key: &str) -> Option<f64> {
    p.skater_context.as_ref()?.values.get(key).copied()
}
pub fn attach(profiles: &mut [Profile], season: u32) {
    let mut references: BTreeMap<(u32, String), Vec<f64>> = BTreeMap::new();
    for p in profiles.iter().filter(|p| {
        p.season == season
            && p.group != "Goalies"
            && p.games >= 10
            && value(p, "def-5v5-seconds").unwrap_or(0.) >= 6000.
    }) {
        if let Some(v) = value(p, "def-5v5-xga60") {
            references
                .entry((p.season, p.group.clone()))
                .or_default()
                .push(scores::rank_value(-v));
        }
    }
    for p in profiles
        .iter_mut()
        .filter(|p| p.season == season && p.group != "Goalies" && p.skater_context.is_some())
    {
        let measured = |key: &str| {
            value(p, key)
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "—".into())
        };
        let minutes = value(p, "def-5v5-seconds").map(|s| s / 60.);
        let activity = format!(
            "{} blocks · {} takeaways /60",
            measured("blocks"),
            measured("takeaways")
        );
        let chances = value(p, "def-5v5-xga60")
            .map(|v| format!("{v:.2} xGA/60"))
            .unwrap_or_else(|| "Unavailable".into());
        let difference = value(p, "def-5v5-off-xga60")
            .zip(value(p, "def-5v5-xga60"))
            .map(|(off, on)| off - on);
        let on_off = difference
            .map(|v| {
                format!(
                    "{:.2} {} xGA/60 on ice",
                    v.abs(),
                    if v >= 0. { "fewer" } else { "more" }
                )
            })
            .unwrap_or_else(|| "On/off comparison unavailable".into());
        let reference = references.get(&(p.season, p.group.clone()));
        let peers = reference.map_or(0, |r| r.len());
        let rank = if p.games >= 10 && minutes.unwrap_or(0.) >= 100. {
            value(p, "def-5v5-xga60")
                .zip(reference)
                .and_then(|(v, r)| percentile(-v, r))
        } else {
            None
        };
        let mut rows = vec![];
        for (label, key, unit) in [
            ("Blocked shots", "blocks", "/60 · all strengths"),
            ("Takeaways", "takeaways", "/60 · all strengths"),
            ("Penalties taken", "taken", "/60 · all strengths"),
            ("Expected goals against", "def-5v5-xga60", "/60 · 5v5"),
            ("Unblocked attempts against", "def-5v5-fa60", "/60 · 5v5"),
            (
                "Off-ice expected goals against",
                "def-5v5-off-xga60",
                "/60 · 5v5",
            ),
        ] {
            rows.push(Row {
                attribute: "Defending".into(),
                label: label.into(),
                value: value(p, key)
                    .map(|v| format!("{v:.2} {unit}"))
                    .unwrap_or_else(|| "Unavailable".into()),
            });
        }
        rows.push(Row {
            attribute: "Defending".into(),
            label: "Five-on-five exposure".into(),
            value: minutes
                .map(|m| format!("{m:.1} min · {} GP", p.games))
                .unwrap_or_else(|| "Unavailable for this snapshot".into()),
        });
        rows.push(Row{attribute:"Defending".into(),label:"Lower-xGA comparison".into(),value:rank.map(|r|format!("{r:.1} percentile · {peers} GP-matched {} peers",p.group.to_lowercase())).unwrap_or_else(||format!("Unranked · 10 GP, 100 5v5 min and 20 measured peers required ({peers} supplied)"))});
        let note = if minutes.is_some() {
            "Activity uses NHL all-strength rates. Chance Control uses MoneyPuck.com five-on-five team results; it does not isolate player impact. On/off = off-ice xGA/60 minus on-ice xGA/60. Peer rank uses only this supplied GP-matched position cohort, with ties split. No change to Defending, class or simulation bonuses."
        } else {
            "Activity uses NHL all-strength rates. Chance Control is unavailable: the five-on-five source row is missing or its GP does not match this snapshot. Missing evidence is not a poor defensive result. No change to Defending, class or simulation bonuses."
        };
        let record = Record {
            schema: "rinkcraft.defending-record.v1".into(),
            player_id: p.player_id,
            season: p.season,
            group: p.group.clone(),
            games: p.games,
            minutes_5v5: minutes,
            activity,
            chances,
            on_off,
            rank,
            peers,
            rows,
            note: note.into(),
        };
        p.skater_context.as_mut().unwrap().defending = Some(record);
    }
}
