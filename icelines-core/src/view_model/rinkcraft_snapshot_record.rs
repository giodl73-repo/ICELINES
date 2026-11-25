//! Season pace from admitted NHL all-strength rates, never from shot-archive totals.
use super::rinkcraft_stat_scores::{estimate_record, EstimatedRecord, Profile, RecordMetric};

pub fn estimate(p: &Profile, maximum_games: Option<u32>) -> Option<EstimatedRecord> {
    if !matches!(p.group.as_str(), "Forwards" | "Defense")
        || p.games == 0
        || !p.minutes.is_finite()
        || p.minutes <= 0.
    {
        return None;
    }
    let values = &p.skater_context.as_ref()?.values;
    let value = |key: &str| {
        values
            .get(key)
            .copied()
            .filter(|v| v.is_finite() && *v >= 0.)
    };
    let mut metrics = vec![];
    for (key, label) in [
        ("shots", "Shots"),
        ("assists", "Assists"),
        ("goals", "Goals"),
        ("hits", "Hits"),
        ("blocks", "Blocks"),
    ] {
        metrics.push(RecordMetric {
            key: key.into(),
            label: label.into(),
            unit: "Per60".into(),
            value: value(key),
        });
    }
    metrics.push(RecordMetric {
        key: "ice-time".into(),
        label: "TOI min/game".into(),
        unit: "Minutes".into(),
        value: Some(p.minutes / p.games as f64),
    });
    metrics.push(RecordMetric {
        key: "conversion".into(),
        label: "Shot conversion".into(),
        unit: "Pct".into(),
        value: value("goals")
            .zip(value("shots"))
            .filter(|(_, s)| *s > 0.)
            .map(|(g, s)| g / s),
    });
    let mut record = estimate_record(p.season, p.games, p.minutes, maximum_games, &metrics);
    record.basis.push_str(" · all strengths");
    Some(record)
}
