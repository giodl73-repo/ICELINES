//! Shot generation and observed finishing, with provider windows kept separate.
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
    pub shots: String,
    pub conversion: String,
    pub rows: Vec<Row>,
    pub formula: String,
    pub note: String,
    pub source_notes: Vec<String>,
}
impl Record {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.shooting-record.v1"
            || self.player_id == 0
            || !["Forwards", "Defense"].contains(&self.group.as_str())
            || !self.minutes.is_finite()
            || self.minutes < 0.
            || self.rows.len() != 11
            || [&self.shots, &self.conversion, &self.formula, &self.note]
                .iter()
                .any(|s| s.is_empty())
            || self
                .rows
                .iter()
                .any(|r| r.attribute != "Shooting" || r.label.is_empty() || r.value.is_empty())
        {
            return Err("Invalid shooting record".into());
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
        let value = |key: &str| c.values.get(key).copied();
        let official = |key: &str| value(key).filter(|_| p.games > 0 && p.minutes > 0.);
        let rate = |v: Option<f64>| {
            v.map(|v| format!("{v:.2} /60"))
                .unwrap_or_else(|| "Unavailable".into())
        };
        let shots = rate(official("shots"));
        let conversion = official("goals")
            .zip(official("shots"))
            .filter(|(_, s)| *s > 0.)
            .map(|(g, s)| format!("{:.1}%", 100. * g / s))
            .unwrap_or_else(|| "Unavailable".into());
        let attempts = value("quality-attempts");
        let sampled = |key: &str| value(key).filter(|_| attempts.is_some_and(|a| a > 0.));
        let number = |v: Option<f64>, digits: usize, unit: &str| {
            v.map(|v| format!("{v:.digits$}{unit}"))
                .unwrap_or_else(|| "Unavailable".into())
        };
        let finishing = sampled("quality-goals")
            .zip(sampled("ixg"))
            .map(|(g, x)| format!("{:+.2} goals", g - x))
            .unwrap_or_else(|| "Unavailable".into());
        let rows = [
            ("NHL goals", rate(official("goals"))),
            ("NHL shots on goal", shots.clone()),
            ("NHL shot conversion", conversion.clone()),
            (
                "Archive unblocked attempts · goalie in net",
                number(attempts, 0, " attempts"),
            ),
            (
                "Archive goals · goalie in net",
                number(value("quality-goals"), 0, " goals"),
            ),
            (
                "Archive expected goals · goalie in net",
                number(value("ixg"), 2, " xG"),
            ),
            ("Archive goals minus expected", finishing),
            (
                "Archive expected goals / unblocked attempt",
                number(sampled("shot-quality"), 3, " xG/attempt"),
            ),
            (
                "Archive average attempt distance",
                number(sampled("shot-distance"), 1, " ft"),
            ),
            (
                "EDGE maximum tracked shot speed",
                number(value("edge-shot-speed"), 1, " mph"),
            ),
            (
                "NHL all-strength exposure",
                format!("{} GP · {:.1} min", p.games, p.minutes),
            ),
        ]
        .into_iter()
        .map(|(label, value)| Row {
            attribute: "Shooting".into(),
            label: label.into(),
            value,
        })
        .collect();
        c.shooting = Some(Record {
            schema:"rinkcraft.shooting-record.v1".into(),player_id:p.player_id,season:p.season,group:p.group.clone(),games:p.games,minutes:p.minutes,shots,conversion,rows,
            formula:"NHL rates = official counts × 60 / all-strength minutes. NHL conversion = goals/60 ÷ shots on goal/60 × 100; zero shots have no conversion. Archive residual = archive goals − archive xG; average danger uses unblocked attempts, including misses. Archive figures use their own goalie-in-net sample, never NHL minutes or shot totals. Displayed figures are rounded.".into(),
            note:"Generation, conversion and modeled shot danger describe different results. Archive and NHL capture windows can differ; their counts must not be combined. A positive goals-minus-xG result is not established finishing talent. EDGE maximum shot speed is a recorded maximum, not release speed or accuracy. Missing observations are not zero. No Shooting grade, feat, class or simulation bonus changes.".into(),
            source_notes:c.notes.iter().filter(|n| n.contains("shot archive captured")).cloned().collect(),
        });
    }
}
