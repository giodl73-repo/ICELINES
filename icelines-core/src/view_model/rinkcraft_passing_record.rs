//! Credited assist involvement and strength context; not pass completion or unit assignment.
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
    pub primary: String,
    pub secondary: String,
    pub mix: String,
    pub rows: Vec<Row>,
    pub formula: String,
    pub note: String,
}
impl Record {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.passing-record.v1"
            || self.player_id == 0
            || !["Forwards", "Defense"].contains(&self.group.as_str())
            || !self.minutes.is_finite()
            || self.minutes < 0.
            || self.rows.len() != 8
            || [
                &self.primary,
                &self.secondary,
                &self.mix,
                &self.formula,
                &self.note,
            ]
            .iter()
            .any(|s| s.is_empty())
        {
            return Err("Invalid passing record".into());
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
        let value = |key: &str| {
            c.values
                .get(key)
                .copied()
                .filter(|_| p.games > 0 && p.minutes > 0.)
        };
        let rate = |count: Option<f64>| count.map(|v| v * 60. / p.minutes);
        let text = |v: Option<f64>| {
            v.map(|v| format!("{v:.2} /60"))
                .unwrap_or_else(|| "Unavailable".into())
        };
        let (primary, secondary) = (value("primary"), value("secondary"));
        let total = primary.zip(secondary).map(|(a, b)| a + b);
        let mix = match primary.zip(total) {
            Some((_, 0.)) => "No credited assists".into(),
            Some((a, t)) => format!("{:.1}% primary credits", a * 100. / t),
            None => "Assist mix unavailable".into(),
        };
        let credit = |count: Option<f64>| {
            count
                .map(|v| format!("{v:.0} credits · {:.2} /60", v * 60. / p.minutes))
                .unwrap_or_else(|| "Unavailable".into())
        };
        let strength = |key: &str, unit: &str| {
            value(key)
                .map(|v| format!("{v:.2} /60 {unit}"))
                .unwrap_or_else(|| "Unavailable".into())
        };
        let rows = vec![
            Row {
                attribute: "Passing".into(),
                label: "All assists".into(),
                value: text(value("assists")),
            },
            Row {
                attribute: "Passing".into(),
                label: "Primary assists".into(),
                value: credit(primary),
            },
            Row {
                attribute: "Passing".into(),
                label: "Secondary assists".into(),
                value: credit(secondary),
            },
            Row {
                attribute: "Passing".into(),
                label: "Assist mix".into(),
                value: mix.clone(),
            },
            Row {
                attribute: "Passing".into(),
                label: "Even-strength assists".into(),
                value: strength("ev-assists", "EV"),
            },
            Row {
                attribute: "Passing".into(),
                label: "Power-play assists".into(),
                value: strength("pp-assists", "PP"),
            },
            Row {
                attribute: "Passing".into(),
                label: "Power-play time share".into(),
                value: value("pp-share")
                    .map(|v| format!("{:.1}% of all-strength time", v * 100.))
                    .unwrap_or_else(|| "Unavailable".into()),
            },
            Row {
                attribute: "Passing".into(),
                label: "All-strength exposure".into(),
                value: format!("{} GP · {:.1} min", p.games, p.minutes),
            },
        ];
        let formula="All-strength credit rate = assist count × 60 / official all-strength minutes. Primary share = primary / (primary + secondary); a zero denominator has no share. EV and PP rates use their own strength-specific minutes, not all-strength time. EV is not a five-on-five-only label. Displayed figures are rounded.";
        let note="These are NHL credited scoring chains, not pass attempts, completion, touches or every chance created. The split distinguishes involvement without treating secondary credits as worthless. Strength context does not identify a PP unit, station or coaching assignment. Missing exposure/detail is not zero production. No Passing grade, feat, class or simulation bonus changes.";
        c.passing = Some(Record {
            schema: "rinkcraft.passing-record.v1".into(),
            player_id: p.player_id,
            season: p.season,
            group: p.group.clone(),
            games: p.games,
            minutes: p.minutes,
            primary: text(rate(primary)),
            secondary: text(rate(secondary)),
            mix,
            rows,
            formula: formula.into(),
            note: note.into(),
        });
    }
}
