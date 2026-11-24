//! Attribute-specific measured context; no additional ability bonuses.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub attribute: String,
    pub label: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub values: BTreeMap<String, f64>,
    pub source_credit: String,
    pub notes: Vec<String>,
    pub rows: Vec<Row>,
}
impl Context {
    pub fn new(values: BTreeMap<String, f64>, source_credit: String, notes: Vec<String>) -> Self {
        let mut rows = vec![];
        for (attribute, key, label, unit) in [
            ("Skating", "peak-speed", "Top skating speed", "mph"),
            ("Skating", "bursts", "20+ mph bursts", "/60"),
            (
                "Skating",
                "edge-distance",
                "Recorded skating distance",
                "miles",
            ),
            ("Handling", "giveaways", "Giveaways", "/60"),
            ("Handling", "drawn", "Penalties drawn", "/60"),
            (
                "Handling",
                "loss-share",
                "Giveaways / recorded offensive actions",
                "%",
            ),
            ("Passing", "primary", "Primary assists", "count"),
            ("Passing", "secondary", "Secondary assists", "count"),
            ("Passing", "primary-rate", "Primary assists", "/60"),
            ("Passing", "ev-assists", "Even-strength assists", "/60 EV"),
            ("Passing", "pp-assists", "Power-play assists", "/60 PP"),
            ("Shooting", "goals", "Goals", "/60"),
            ("Shooting", "shots", "Shots on goal", "/60"),
            ("Shooting", "ixg", "Expected goals (goalie in net)", "xG"),
            (
                "Shooting",
                "finishing",
                "Goals minus expected (goalie in net)",
                "signed",
            ),
            (
                "Shooting",
                "shot-quality",
                "Expected goals / unblocked attempt",
                "xG",
            ),
            (
                "Shooting",
                "shot-distance",
                "Average attempt distance",
                "ft",
            ),
            (
                "Shooting",
                "edge-shot-speed",
                "Maximum tracked shot speed",
                "mph",
            ),
            ("Reading", "takeaways", "Takeaways", "/60"),
            ("Reading", "taken", "Penalties taken", "/60"),
            (
                "Reading",
                "net-penalties",
                "Drawn minus taken",
                "signed /60",
            ),
            ("Defending", "blocks", "Blocked shots", "/60"),
            ("Defending", "hits", "Hits", "/60"),
            ("Defending", "sh-seconds", "Penalty-kill deployment", "time"),
            (
                "Defending",
                "edge-dz",
                "Puck in defensive zone while on ice",
                "%",
            ),
        ] {
            let value = values
                .get(key)
                .map(|v| match unit {
                    "%" => format!("{:.1}%", v * 100.),
                    "count" => format!("{v:.0}"),
                    "time" => format!("{}:{:02} /GP", (*v as u64) / 60, (*v as u64) % 60),
                    "signed" => format!("{v:+.2}"),
                    "signed /60" => format!("{v:+.2} /60"),
                    _ => format!("{v:.2} {unit}"),
                })
                .unwrap_or_else(|| "Unavailable".into());
            rows.push(Row {
                attribute: attribute.into(),
                label: label.into(),
                value,
            });
        }
        Self {
            values,
            source_credit,
            notes,
            rows,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.values.values().any(|v| !v.is_finite())
            || self
                .values
                .iter()
                .any(|(k, v)| *v < 0. && !matches!(k.as_str(), "finishing" | "net-penalties"))
            || self.rows.len() != 25
            || self.rows.iter().any(|r| {
                !super::rinkcraft_stat_scores::attribute_labels("Forwards")
                    .contains(&r.attribute.as_str())
                    || r.value.is_empty()
            })
        {
            return Err("Invalid skater attribute context".into());
        }
        Ok(())
    }
}
