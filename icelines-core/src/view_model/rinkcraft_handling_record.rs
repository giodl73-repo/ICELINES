//! Recorded giveaways and penalty drawing are distinct; neither counts puck touches.
use super::{rinkcraft_skater_context::Row, rinkcraft_stat_scores::Profile};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    pub giveaways: String,
    pub drawn: String,
    pub rows: Vec<Row>,
    pub formula: String,
    pub note: String,
}
impl Record {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.handling-record.v1"
            || self.player_id == 0
            || !["Forwards", "Defense"].contains(&self.group.as_str())
            || !self.minutes.is_finite()
            || self.minutes < 0.
            || self.rows.len() != 6
            || self.giveaways.is_empty()
            || self.drawn.is_empty()
            || self.formula.is_empty()
            || self.note.is_empty()
        {
            return Err("Invalid handling record".into());
        }
        Ok(())
    }
}
pub fn attach(profiles: &mut [Profile], season: u32) {
    for p in profiles
        .iter_mut()
        .filter(|p| p.season == season && p.group != "Goalies")
    {
        let Some(c) = p.skater_context.as_mut() else {
            continue;
        };
        let rate = |key: &str| {
            c.values
                .get(key)
                .copied()
                .filter(|_| p.games > 0 && p.minutes > 0.)
        };
        let text = |value: Option<f64>| {
            value
                .map(|v| format!("{v:.2} /60"))
                .unwrap_or_else(|| "Unavailable".into())
        };
        let (giv, drawn, taken, shots, assists) = (
            rate("giveaways"),
            rate("drawn"),
            rate("taken"),
            rate("shots"),
            rate("assists"),
        );
        let total = giv.zip(shots).zip(assists).map(|((g, s), a)| g + s + a);
        let share = giv.zip(total).filter(|(_, t)| *t > 0.).map(|(g, t)| g / t);
        let formula=match (giv,shots,assists,share) {
            (Some(g),Some(s),Some(a),Some(r))=>format!("Giveaway share proxy = {g:.2} / ({s:.2} shots + {a:.2} assists + {g:.2} giveaways) = {:.1}% (rates per60; display rounded).",r*100.),
            _=>"Giveaway share proxy = giveaways / (shots + assists + giveaways). Unavailable when inputs or a positive denominator are missing.".into(),
        };
        let rows = vec![
            Row {
                attribute: "Handling".into(),
                label: "Recorded giveaways".into(),
                value: text(giv),
            },
            Row {
                attribute: "Handling".into(),
                label: "Penalties drawn".into(),
                value: text(drawn),
            },
            Row {
                attribute: "Handling".into(),
                label: "Penalties taken".into(),
                value: text(taken),
            },
            Row {
                attribute: "Handling".into(),
                label: "Drawn minus taken".into(),
                value: drawn
                    .zip(taken)
                    .map(|(d, t)| format!("{:+.2} /60", d - t))
                    .unwrap_or_else(|| "Unavailable".into()),
            },
            Row {
                attribute: "Handling".into(),
                label: "Giveaway share proxy".into(),
                value: share
                    .map(|s| format!("{:.1}%", s * 100.))
                    .unwrap_or_else(|| "Unavailable".into()),
            },
            Row {
                attribute: "Handling".into(),
                label: "All-strength exposure".into(),
                value: format!("{} GP · {:.1} min", p.games, p.minutes),
            },
        ];
        let note="NHL recorded events, all strengths. Giveaways per60 are not losses per touch: puck use, role and scorer practices remain uncontrolled. The share denominator counts shots, assists and giveaways, not possession opportunities; it can improve when offensive production rises. Penalties drawn are not necessarily power plays created because calls can offset. No new Handling grade, feat, class or simulation bonus is awarded.";
        c.handling = Some(Record {
            schema: "rinkcraft.handling-record.v1".into(),
            player_id: p.player_id,
            season: p.season,
            group: p.group.clone(),
            games: p.games,
            minutes: p.minutes,
            giveaways: text(giv),
            drawn: text(drawn),
            rows,
            formula,
            note: note.into(),
        });
    }
}
