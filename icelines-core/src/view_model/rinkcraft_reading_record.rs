//! Events supporting the Reading proxy; anticipation and assignments stay unmeasured.
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
    pub takeaways: String,
    pub assists: String,
    pub taken: String,
    pub rows: Vec<Row>,
    pub formula: String,
    pub note: String,
}
impl Record {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.reading-record.v1"
            || self.player_id == 0
            || !["Forwards", "Defense"].contains(&self.group.as_str())
            || !self.minutes.is_finite()
            || self.minutes < 0.
            || self.rows.len() != 6
            || [
                &self.takeaways,
                &self.assists,
                &self.taken,
                &self.formula,
                &self.note,
            ]
            .iter()
            .any(|s| s.is_empty())
        {
            return Err("Invalid reading record".into());
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
        let text = |v: Option<f64>| {
            v.map(|v| format!("{v:.2} /60"))
                .unwrap_or_else(|| "Unavailable".into())
        };
        let takeaways = text(rate("takeaways"));
        let assists = text(rate("assists"));
        let taken = text(rate("taken"));
        let secondary = rate("secondary").map(|count| count * 60. / p.minutes);
        let rows = vec![
            Row {
                attribute: "Reading".into(),
                label: "Recorded takeaways".into(),
                value: takeaways.clone(),
            },
            Row {
                attribute: "Reading".into(),
                label: "Assist involvement".into(),
                value: assists.clone(),
            },
            Row {
                attribute: "Reading".into(),
                label: "Penalties taken".into(),
                value: taken.clone(),
            },
            Row {
                attribute: "Reading".into(),
                label: "Primary assists".into(),
                value: text(rate("primary-rate")),
            },
            Row {
                attribute: "Reading".into(),
                label: "Secondary assists".into(),
                value: text(secondary),
            },
            Row {
                attribute: "Reading".into(),
                label: "All-strength exposure".into(),
                value: format!("{} GP · {:.1} min", p.games, p.minutes),
            },
        ];
        let formula="Rates = NHL credited events × 60 / official all-strength minutes. Primary and secondary credits are shown only when the supplied assist detail matches that snapshot; neither measures all passes. Displayed rates are rounded.";
        let note="Anticipation and assigned coverage remain unmeasured. Takeaways are credited events, not all puck recoveries or a direct hockey-IQ measurement. Assists also inform Passing, so these proxy attributes do not have independent inputs. Penalty calls alone do not establish decision quality. This record changes no Reading grade, feat, class or simulation bonus.";
        c.reading = Some(Record {
            schema: "rinkcraft.reading-record.v1".into(),
            player_id: p.player_id,
            season: p.season,
            group: p.group.clone(),
            games: p.games,
            minutes: p.minutes,
            takeaways,
            assists,
            taken,
            rows,
            formula: formula.into(),
            note: note.into(),
        });
    }
}
