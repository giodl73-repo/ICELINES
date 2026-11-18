//! Descriptive agreement on supplied anchored opportunities; no reliability or skill gate.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub source_lineage_id: String,
    pub game_id: String,
    pub period: u8,
    pub actor_id: u32,
    pub component_id: String,
    pub source_event_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    TechnicalFailure,
    ChoiceFailure,
    OpponentWins,
    TeammateFailure,
    JointOrAmbiguous,
    Untested,
    Censored,
    OutOfScope,
}
const LABELS: [Outcome; 9] = [
    Outcome::Success,
    Outcome::TechnicalFailure,
    Outcome::ChoiceFailure,
    Outcome::OpponentWins,
    Outcome::TeammateFailure,
    Outcome::JointOrAmbiguous,
    Outcome::Untested,
    Outcome::Censored,
    Outcome::OutOfScope,
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Annotation {
    pub key: Key,
    pub outcome: Option<Outcome>,
}
#[derive(Debug, Serialize)]
pub struct Agreement {
    pub first_candidates: usize,
    pub second_candidates: usize,
    pub matched: usize,
    pub only_first: Vec<Annotation>,
    pub only_second: Vec<Annotation>,
    pub detection_overlap: Option<f64>,
    pub matched_both_labeled: usize,
    pub matched_without_two_labels: usize,
    pub identical_labels: usize,
    pub outcome_agreement: Option<f64>,
    pub matrix_labels: [Outcome; 9],
    pub confusion_matrix: [[usize; 9]; 9],
    pub interpretation: String,
}
fn index(rows: &[Annotation]) -> Result<BTreeMap<Key, &Annotation>, String> {
    let mut result = BTreeMap::new();
    for row in rows {
        let k = &row.key;
        if k.actor_id == 0
            || !(1..=3).contains(&k.period)
            || [
                &k.source_lineage_id,
                &k.game_id,
                &k.component_id,
                &k.source_event_id,
            ]
            .iter()
            .any(|s| s.trim().is_empty())
        {
            return Err("Missing anchored opportunity key or unsupported regulation period".into());
        }
        if result.insert(k.clone(), row).is_some() {
            return Err("Duplicate coder opportunity; preserve revisions separately, do not silently reconcile".into());
        }
    }
    Ok(result)
}
pub fn compare(first: &[Annotation], second: &[Annotation]) -> Result<Agreement, String> {
    let a = index(first)?;
    let b = index(second)?;
    let mut out=Agreement{first_candidates:a.len(),second_candidates:b.len(),matched:0,
        only_first:vec![],only_second:vec![],detection_overlap:None,matched_both_labeled:0,
        matched_without_two_labels:0,identical_labels:0,outcome_agreement:None,
        matrix_labels:LABELS,confusion_matrix:[[0;9];9],
        interpretation:"Descriptive counts only. Exact source/game/period/actor/component/event-ID alignment; no inferred matching or ground truth. Detection overlap = 2*matched/(first+second); outcome agreement uses only matched pairs with both labels. Unmatched and unlabeled counts remain explicit. Empty denominators are unavailable, not perfect agreement. No chance correction, empirical reliability threshold, capability grade or validated observation import.".into()};
    for (k, row) in &a {
        if let Some(other) = b.get(k) {
            out.matched += 1;
            if let Some((x, y)) = row.outcome.zip(other.outcome) {
                out.matched_both_labeled += 1;
                out.identical_labels += usize::from(x == y);
                let i = LABELS.iter().position(|v| *v == x).expect("fixed taxonomy");
                let j = LABELS.iter().position(|v| *v == y).expect("fixed taxonomy");
                out.confusion_matrix[i][j] += 1;
            } else {
                out.matched_without_two_labels += 1;
            }
        } else {
            out.only_first.push((**row).clone());
        }
    }
    for (k, row) in &b {
        if !a.contains_key(k) {
            out.only_second.push((**row).clone());
        }
    }
    if !a.is_empty() || !b.is_empty() {
        out.detection_overlap = Some(2. * out.matched as f64 / (a.len() as f64 + b.len() as f64));
    }
    if out.matched_both_labeled > 0 {
        out.outcome_agreement = Some(out.identical_labels as f64 / out.matched_both_labeled as f64);
    }
    Ok(out)
}
