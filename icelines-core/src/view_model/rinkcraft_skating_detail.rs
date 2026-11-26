//! Display the measured inputs already admitted to the Skating estimate.
use super::rinkcraft_stat_scores::Profile;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub key: String,
    pub label: String,
    pub value: String,
    pub comparison: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    pub headline: String,
    pub secondary: String,
    pub rows: Vec<Row>,
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
    let scores = p
        .scores
        .iter()
        .filter(|s| s.label == "Skating")
        .collect::<Vec<_>>();
    if scores.len() != 1 {
        return None;
    }
    let part = |key: &str, unit: &str| {
        let found = scores[0]
            .parts
            .iter()
            .filter(|s| s.key == key)
            .collect::<Vec<_>>();
        if found.len() != 1 {
            return None;
        }
        let s = found[0];
        if s.unit != unit || s.value.is_none_or(|v| !v.is_finite() || v < 0.) {
            None
        } else {
            Some(s)
        }
    };
    let speed = part("peak-speed", "mph");
    let bursts = part("bursts", "/60");
    if speed.is_none() && bursts.is_none() {
        return None;
    }
    let peak = speed.map(|s| format!("{:.2} mph", s.value.unwrap()));
    let rate = bursts.map(|s| format!("{:.2} /60", s.value.unwrap()));
    let comparison = |s: Option<&super::rinkcraft_stat_scores::Part>| {
        s.and_then(|s| {
            s.percentile
                .filter(|v| v.is_finite() && (0. ..=100.).contains(v))
                .filter(|_| s.peers > 0)
                .map(|v| {
                    format!(
                        "{v:.1}th percentile · {} measured {}",
                        s.peers,
                        p.group.to_lowercase()
                    )
                })
        })
    };
    let mut rows = vec![
        Row {
            key: "peak-speed".into(),
            label: "Reported peak speed".into(),
            value: peak.clone().unwrap_or_else(|| "Unavailable".into()),
            comparison: comparison(speed),
        },
        Row {
            key: "bursts".into(),
            label: "20+ mph bursts".into(),
            value: rate.clone().unwrap_or_else(|| "Unavailable".into()),
            comparison: comparison(bursts),
        },
    ];
    if let Some(distance) = p
        .skater_context
        .as_ref()
        .and_then(|c| c.values.get("edge-distance"))
        .filter(|v| v.is_finite() && **v >= 0.)
    {
        rows.push(Row {
            key: "edge-distance".into(),
            label: "Recorded skating distance".into(),
            value: format!("{distance:.2} miles"),
            comparison: None,
        });
    }
    Some(Record {schema:"rinkcraft.skating-detail.v1".into(),player_id:p.player_id,season:p.season,group:p.group.clone(),games:p.games,minutes:p.minutes,
        headline:peak.clone().or_else(||rate.clone()).unwrap(),secondary:if peak.is_some(){rate.map(|r|format!("20+ mph bursts {r}")).unwrap_or_else(||"Burst rate unavailable".into())}else{"20+ mph bursts · peak unavailable".into()},rows,
        note:"Recorded all-strength inputs, separate from the 1–20 estimate. Peak speed is the reported season maximum. Burst rate uses recorded 20-plus-mph bursts × 60 / official all-strength TOI minutes. Skating distance is a cumulative activity total. None measures puck-carrying speed, acceleration, turns, pivots or reaction time. Peer positions reuse the selected-season Skating inputs; no new rank or feat is awarded.".into()})
}
pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
    if build(p).as_ref() != Some(r) {
        Err("Skating detail does not match admitted inputs/window".into())
    } else {
        Ok(())
    }
}
