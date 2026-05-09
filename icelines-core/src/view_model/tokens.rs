use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricCell {
    pub key: StatKey,
    pub label: String,
    pub value: MetricValue,
    pub unit: MetricUnit,
    pub precision: ValuePrecision,
    pub token: Option<SemanticToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StatKey(pub String);

impl From<&str> for StatKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricValue {
    Integer(i64),
    Decimal(f64),
    Text(String),
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricUnit {
    Count,
    Points,
    Goals,
    Assists,
    Games,
    Seconds,
    Minutes,
    Percentage,
    PerGame,
    Per82,
    Ranking,
    Score,
    Money,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuePrecision {
    Integer,
    OneDecimal,
    TwoDecimals,
    ThreeDecimals,
    PercentOneDecimal,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticToken {
    FitElite,
    FitSolid,
    FitBuried,
    FitStretch,
    SourceComplete,
    SourcePartial,
    SourceStale,
    SourceUnavailable,
    GamePre,
    GameLive,
    GameFinal,
    GameOvertime,
    GameShootout,
    Rising,
    Stash,
    Stream,
    CategoryFit,
    ScheduleEdge,
    Risk,
    PrimaryAction,
    SupportingEvidence,
    QuietMetadata,
    DecisionHighlight,
    Warning,
    Error,
    Info,
}
