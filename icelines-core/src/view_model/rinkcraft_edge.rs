//! Source-bound public NHL EDGE observations, distinct from capability estimates.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeSource {
    pub url: String,
    pub sha256: String,
    pub bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeMeasure {
    pub key: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    /// Provider-reported 0..1 percentile; reference eligibility is not supplied.
    pub provider_percentile: Option<f64>,
    pub provider_average: Option<f64>,
    pub source_field: String,
    pub attribute_id: Option<String>,
    pub component_id: Option<String>,
    pub interpretation: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeLocation {
    pub code: String,
    pub shots: Option<u32>,
    pub goals: Option<u32>,
    pub saves: Option<u32>,
    pub goals_against: Option<u32>,
    pub fraction: Option<f64>,
    pub provider_percentile: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeEvidence {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub season_type: String,
    pub group: String,
    pub games: u32,
    pub captured_at: String,
    pub strength_scope: String,
    pub source: EdgeSource,
    pub measures: Vec<EdgeMeasure>,
    pub locations: Vec<EdgeLocation>,
    pub notes: Vec<String>,
}
