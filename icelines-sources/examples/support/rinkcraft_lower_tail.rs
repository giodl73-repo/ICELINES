use serde_json::{json, Value};
use std::collections::BTreeMap;
// Lower-tail observations are descriptive annotations, never negative feat awards.
// Whitelist excludes penalties/giveaways (low may be favorable) and raw PP/SH
// production (zero without deployment is not evidence of poor performance).
pub fn low_callouts(profile: &Value) -> Result<Vec<Value>, String> {
    let mut metrics = BTreeMap::<String, Value>::new();
    for feat in profile["feats"].as_array().ok_or("feat rows missing")? {
        for c in feat["conditions"].as_array().ok_or("conditions missing")? {
            let key = c["key"].as_str().ok_or("metric key missing")?;
            if !matches!(
                key,
                "points" | "goals" | "assists" | "shots" | "hits" | "blocks" | "takeaways"
            ) {
                continue;
            }
            if let Some(prior) = metrics.insert(key.to_owned(), c.clone()) {
                if prior != *c {
                    return Err("repeated metric evidence differs".into());
                }
            }
        }
    }
    let mut lows = Vec::new();
    for (key, c) in metrics {
        if profile["games"].as_u64().unwrap_or(0) < 20 || c["peers"].as_u64().unwrap_or(0) < 20 {
            continue;
        }
        let Some(p) = c["percentile"]
            .as_f64()
            .filter(|p| p.is_finite() && (0.0..=20.0).contains(p))
        else {
            continue;
        };
        if c["value"]
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.)
            .is_none()
        {
            return Err("ranked low metric lacks a valid value".into());
        }
        let kind = if matches!(key.as_str(), "points" | "goals" | "assists") {
            "low_output"
        } else {
            "low_activity"
        };
        lows.push(json!({"id":format!("low-{key}"),"kind":kind,"label":format!("Low recorded {}",c["label"].as_str().unwrap_or(&key)),"condition":c,"percentile":p,"reference_group":profile["group"],"source_sha256":profile["source_sha256"],"source_urls":profile["source_urls"],"interpretation":if kind=="low_output"{"Low recorded per-game production; deployment and teammates affect output. Not a capability rating."}else{"Low recorded event volume; style and opportunities affect activity. Not proof of poor defending or effort."}}));
    }
    Ok(lows)
}
