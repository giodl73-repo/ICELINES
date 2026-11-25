//! Shot-sequence outcomes; neither goalie travel nor reset time is measured.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Bucket {
    pub attempts: u32,
    pub on_goal: u32,
    pub goals: u32,
    pub xg: f64,
}
impl Bucket {
    pub fn record(&mut self, on: bool, goal: bool, xg: f64) {
        self.attempts += 1;
        self.on_goal += u32::from(on);
        self.goals += u32::from(goal);
        self.xg += xg;
    }
    fn saving(&self) -> String {
        if self.on_goal == 0 {
            "No on-goal opportunities".into()
        } else {
            format!(
                "{}/{} saved · {:.1}%",
                self.on_goal - self.goals,
                self.on_goal,
                100. * (self.on_goal - self.goals) as f64 / self.on_goal as f64
            )
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub attempts: u32,
    pub generated: u32,
    pub expected: f64,
    pub buckets: BTreeMap<String, Bucket>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub attribute: String,
    pub label: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub data: Data,
    pub source_url: String,
    pub captured_at: String,
    pub sha256: String,
    pub headline: Vec<Row>,
    pub rows: Vec<Row>,
}
impl Data {
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = vec![];
        for (key, attribute, label) in [
            ("high", "Movement", "High-danger shots on goal"),
            ("cross", "Movement", "Cross-slot rebound shots on goal"),
            ("rebound", "Recovery", "All rebound shots on goal"),
            ("fast", "Recovery", "Follow-ups within 0–1 second"),
            ("later", "Recovery", "Follow-ups after 1–3 seconds"),
            ("after-shot", "Recovery", "Follow-ups after shots on goal"),
            ("after-miss", "Recovery", "Follow-ups after missed shots"),
        ] {
            let bucket = self.buckets.get(key).cloned().unwrap_or_default();
            rows.push(Row {
                attribute: attribute.into(),
                label: label.into(),
                value: bucket.saving(),
            });
        }
        rows.push(Row {
            attribute: "Rebounds".into(),
            label: "Generated follow-ups".into(),
            value: format!(
                "{} / {:.2} expected · {} unblocked attempts",
                self.generated, self.expected, self.attempts
            ),
        });
        let rebound = self.buckets.get("rebound").cloned().unwrap_or_default();
        rows.push(Row {
            attribute: "Rebounds".into(),
            label: "Follow-up threat".into(),
            value: format!(
                "{:.2} xG · {} attempts · {} missed",
                rebound.xg,
                rebound.attempts,
                rebound.attempts - rebound.on_goal
            ),
        });
        rows
    }
}
impl Context {
    pub fn new(data: Data, source_url: String, captured_at: String, sha256: String) -> Self {
        let rows = data.rows();
        let headline = [1, 2, 7].into_iter().map(|i| rows[i].clone()).collect();
        Self {
            data,
            source_url,
            captured_at,
            sha256,
            rows,
            headline,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.data.generated > self.data.attempts
            || !self.data.expected.is_finite()
            || self.data.expected < 0.
            || self.data.expected > self.data.attempts as f64
            || self.data.buckets.values().any(|b| {
                b.goals > b.on_goal
                    || b.on_goal > b.attempts
                    || b.attempts > self.data.attempts
                    || !b.xg.is_finite()
                    || b.xg < 0.
                    || b.xg > b.attempts as f64
            })
            || self.rows != self.data.rows()
            || self.headline
                != [1, 2, 7]
                    .into_iter()
                    .map(|i| self.rows[i].clone())
                    .collect::<Vec<_>>()
            || !self
                .source_url
                .starts_with("https://peter-tanner.com/moneypuck/downloads/shots_")
            || self.sha256.len() != 64
            || !self.sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || chrono::DateTime::parse_from_rfc3339(&self.captured_at).is_err()
        {
            return Err("Invalid goalie shot context".into());
        }
        Ok(())
    }
}
