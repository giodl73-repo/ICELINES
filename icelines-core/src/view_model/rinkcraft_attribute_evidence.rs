//! Display explanations and contextual links, never numerical class inference.
use super::rinkcraft_numbers::Measurement;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeExplanation {
    pub question: String,
    pub related_keys: Vec<String>,
    pub limit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureContext {
    pub attribute_key: String,
    pub relation: String,
    pub note: String,
    pub measurements: Vec<Measurement>,
}

pub fn explanation(key: &str) -> Option<AttributeExplanation> {
    let (question, related, limit): (&str, &[&str], &str) = match key {
        "primary-assists-per-60" => ("How often did his final pass precede a teammate's goal?", &["primary-assists-per-60", "secondary-assists-per-60", "on-ice-xgf-per-60"], "Scoring output and on-ice environment; no pass attempts, deception, lane creation or completion rate."),
        "shots-per-60" => ("How frequently did he put shots on goal?", &["shots-per-60", "ixg-per-60", "high-danger-per-60"], "Shooting involvement per ice time; no adjustment for touches, role or available shooting opportunities."),
        "ixg-per-60" => ("How much modeled shot danger did he accumulate per ice time?", &["ixg-per-60", "shots-per-60", "shot-quality"], "Provider-model opportunity volume; does not identify who created the chance or finishing talent."),
        "shot-quality" => ("How dangerous was his average unblocked attempt in this model?", &["shot-quality", "high-danger-per-60", "ixg-per-60"], "Includes missed attempts. Shot selection and chance environment can affect this; it is not observed shooting percentage."),
        "contact-per-60" => ("How often was he credited with a hit?", &["contact-per-60", "takeaways-per-60"], "Recorded activity, not puck pursuit, recovery success, intimidation or defensive effectiveness."),
        "goals-above-xg-per-60" => ("Did his observed goals exceed this model's expectation?", &["goals-above-xg-per-60", "goals-per-60", "ixg-per-60"], "One-window outcome residual, not a persistent finishing grade. Model error and chance variation remain."),
        "gsax-per-60" => ("How many fewer goals did he concede than this model expected per ice time?", &["gsax-per-60", "gsax", "save-fraction"], "Model-relative saving outcomes; no direct measurement of reading, depth, recovery mechanics or repeatable talent."),
        "save-fraction" => ("What share of shots on goal did he save?", &["save-fraction", "gsax-per-60", "goalie-threat-rate"], "Unadjusted saving outcome. Shot quality and team context matter; no goalie technique is identified."),
        "goalie-threat-rate" => ("How much modeled danger did he face per ice time?", &["goalie-threat-rate", "goalie-shot-workload", "gsax-per-60"], "Threat environment, not goalie quality. Higher workload or danger is not a better performance grade."),
        "goalie-shot-workload" => ("How many shots on goal did he face per ice time?", &["goalie-shot-workload", "goalie-threat-rate", "save-fraction"], "Deployment and shot environment; does not measure rebound control, positioning or puck handling."),
        _ => return None,
    };
    Some(AttributeExplanation {
        question: question.into(),
        related_keys: related.iter().map(|s| (*s).into()).collect(),
        limit: limit.into(),
    })
}

/// Bind existing authored feature links to explicitly named seasonal measurements.
/// Related rates are context, not aliases for legacy all-strength totals.
pub fn feature_context(metrics: &[Measurement]) -> Vec<FeatureContext> {
    let bindings: &[(&str, &str, &[&str], &str)] = &[
        ("assist-output", "Outcome context", &["primary-assists-per-60", "secondary-assists-per-60"], "Seasonal assist rates provide outcome context; they do not measure distribution choices or deception."),
        ("shooting-involvement", "Measured shooting activity", &["shots-per-60", "ixg-per-60"], "Shot frequency and modeled shot danger describe shooting activity, not release mechanics."),
        ("goal-output", "Outcome context", &["goals-per-60"], "Goal rate describes output, not independent finishing talent."),
        ("conversion-outcome", "Related model-relative outcome", &["goals-above-xg-per-60"], "Goals minus expected goals per 60 complements conversion outcomes; it is a different measure from shooting percentage."),
        ("recorded-contact", "Activity context", &["contact-per-60", "takeaways-per-60"], "Recorded contest rates do not measure pursuit pressure or retrieval success."),
        ("save-outcome", "Outcome context", &["save-fraction", "gsax-per-60"], "Observed and model-relative saving outcomes do not establish the documented goalie technique."),
        ("shots-faced", "Deployment context", &["goalie-shot-workload", "goalie-threat-rate"], "Shot and modeled-threat rates describe the goalie environment, not his execution quality."),
    ];
    bindings
        .iter()
        .filter_map(|(key, relation, keys, note)| {
            let measurements = keys
                .iter()
                .filter_map(|k| metrics.iter().find(|m| m.key == *k).cloned())
                .collect::<Vec<_>>();
            (!measurements.is_empty()).then(|| FeatureContext {
                attribute_key: (*key).into(),
                relation: (*relation).into(),
                note: (*note).into(),
                measurements,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_declared_context_and_explanations_are_linked() {
        assert!(explanation("carrying-skill").is_none());
        assert!(explanation("contact-per-60")
            .unwrap()
            .limit
            .contains("not puck pursuit"));
        assert!(explanation("goalie-threat-rate")
            .unwrap()
            .limit
            .contains("not goalie quality"));
        assert!(feature_context(&[]).is_empty());
        let rows = (1..=25)
            .map(|id| super::super::rinkcraft_numbers::InputRow {
                player_id: id,
                name: format!("Synthetic {id}"),
                group: "Forwards".into(),
                games: 30,
                seconds: 30000.,
                values: [("primary_assists".into(), id as f64)]
                    .into_iter()
                    .collect(),
            })
            .collect::<Vec<_>>();
        let c = super::super::rinkcraft_numbers::build(&rows, 20242025, &[], vec![]).unwrap();
        let p = c.card(1, None).unwrap().unwrap();
        assert_eq!(p.facets.len(), 6);
        assert!(p.facets.iter().all(|f| f.explanation.is_some()));
        let linked = p
            .feature_context
            .iter()
            .find(|f| f.attribute_key == "assist-output")
            .unwrap();
        assert_eq!(linked.relation, "Outcome context");
        assert_eq!(linked.measurements[0], p.metrics[0]);
        assert!(p
            .feature_context
            .iter()
            .all(|f| f.attribute_key != "shot-blocking"));
        let serialized = serde_json::to_string(&p).unwrap();
        assert!(!serialized.contains("carrying-skill"));
    }
}
