//! Authored interpretation help, not player evidence or a scoring instrument.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceGuideTopic {
    pub key: String,
    /// Presentation branch: all, skater, or goalie. Not a player classification.
    pub scope: String,
    pub title: String,
    pub question: String,
    pub evidence: String,
    pub boundary: String,
}

/// Shared explanatory copy. No source counts, observations, ratings or model inputs.
pub fn topics() -> Vec<EvidenceGuideTopic> {
    [
        ("record", "all", "Record and capability",
         "What happened in this statistical window, and what can the player reliably execute?",
         "Measured Record shows source-bound activity and outcomes. A capability assessment needs defined tasks, difficulty, repeated opportunities and a suitable reference cohort.",
         "A raw percentile is not a 1–20 attribute. Unrated means the capability assessment is incomplete, not that the player has low ability."),
        ("delivery", "skater", "Passing · delivery",
         "Did the selected delivery give the intended target playable access?",
         "Record the intended target, delivery endpoint, pressure and task difficulty, including ordinary and unsuccessful attempts.",
         "Assists and completed-pass labels do not isolate delivery execution. An incomplete pass does not automatically identify a passer error."),
        ("reception", "skater", "Handling · reception",
         "Could the receiver turn a playable arrival into a purposeful next action?",
         "Assess receiving control separately from delivery. A deliberate one-touch pass or shot can be a successful reception.",
         "Blade contact alone is not control. An unplayable arrival is not automatically a receiver failure; unresolved attribution remains ambiguous."),
        ("choice", "skater", "Reading · choice",
         "Which feasible option did the player recognize and choose, and when?",
         "Assess the visible options and timing before using the later outcome. Keep technical execution as a separate observation.",
         "A goal does not prove a good choice, and a turnover does not prove a bad one. Unseen options remain unknown."),
        ("style", "all", "Tendencies · direction and tempo",
         "How does the player move the puck in this zone, phase and strength state?",
         "Describe north–south and east–west movement, pace, controlled advances and send-deep methods with their source and context.",
         "These styles can coexist. Puck pace is not skating speed or a quality grade. Unit outcomes are not automatically an individual effect or chemistry bonus."),
        ("goalie-exchange", "goalie", "Puck Play · goalie exchanges",
         "Did the goalie provide playable access, and could the receiving teammate control it?",
         "Keep goalie delivery, receiving control and the available outlet choices separate. Verify the goalie actor, pressure, support and actual manpower.",
         "Saves do not measure puck delivery. Skater passing results do not establish goalie capability; missing exchange evidence remains unassessed."),
    ]
    .into_iter()
    .map(|(key, scope, title, question, evidence, boundary)| EvidenceGuideTopic {
        key: key.into(), scope: scope.into(), title: title.into(),
        question: question.into(), evidence: evidence.into(), boundary: boundary.into(),
    })
    .collect()
}
