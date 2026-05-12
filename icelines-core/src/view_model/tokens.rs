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

pub const ALL_SEMANTIC_TOKENS: &[SemanticToken] = &[
    SemanticToken::FitElite,
    SemanticToken::FitSolid,
    SemanticToken::FitBuried,
    SemanticToken::FitStretch,
    SemanticToken::SourceComplete,
    SemanticToken::SourcePartial,
    SemanticToken::SourceStale,
    SemanticToken::SourceUnavailable,
    SemanticToken::GamePre,
    SemanticToken::GameLive,
    SemanticToken::GameFinal,
    SemanticToken::GameOvertime,
    SemanticToken::GameShootout,
    SemanticToken::Rising,
    SemanticToken::Stash,
    SemanticToken::Stream,
    SemanticToken::CategoryFit,
    SemanticToken::ScheduleEdge,
    SemanticToken::Risk,
    SemanticToken::PrimaryAction,
    SemanticToken::SupportingEvidence,
    SemanticToken::QuietMetadata,
    SemanticToken::DecisionHighlight,
    SemanticToken::Warning,
    SemanticToken::Error,
    SemanticToken::Info,
];

impl SemanticToken {
    pub fn key(self) -> &'static str {
        match self {
            Self::FitElite => "fit_elite",
            Self::FitSolid => "fit_solid",
            Self::FitBuried => "fit_buried",
            Self::FitStretch => "fit_stretch",
            Self::SourceComplete => "source_complete",
            Self::SourcePartial => "source_partial",
            Self::SourceStale => "source_stale",
            Self::SourceUnavailable => "source_unavailable",
            Self::GamePre => "game_pre",
            Self::GameLive => "game_live",
            Self::GameFinal => "game_final",
            Self::GameOvertime => "game_overtime",
            Self::GameShootout => "game_shootout",
            Self::Rising => "rising",
            Self::Stash => "stash",
            Self::Stream => "stream",
            Self::CategoryFit => "category_fit",
            Self::ScheduleEdge => "schedule_edge",
            Self::Risk => "risk",
            Self::PrimaryAction => "primary_action",
            Self::SupportingEvidence => "supporting_evidence",
            Self::QuietMetadata => "quiet_metadata",
            Self::DecisionHighlight => "decision_highlight",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Info => "info",
        }
    }

    pub fn ascii_label(self) -> &'static str {
        match self {
            Self::FitElite => "ELITE",
            Self::FitSolid => "SOLID",
            Self::FitBuried => "UNDERUSED",
            Self::FitStretch => "OVEREXTENDED",
            Self::SourceComplete => "SRC complete",
            Self::SourcePartial => "SRC partial",
            Self::SourceStale => "SRC stale",
            Self::SourceUnavailable => "SRC unavailable",
            Self::GamePre => "PRE",
            Self::GameLive => "LIVE",
            Self::GameFinal => "FINAL",
            Self::GameOvertime => "OT",
            Self::GameShootout => "SO",
            Self::Rising => "RISING",
            Self::Stash => "STASH",
            Self::Stream => "STREAM",
            Self::CategoryFit => "CATEGORY",
            Self::ScheduleEdge => "SCHEDULE",
            Self::Risk => "RISK",
            Self::PrimaryAction => "ACTION",
            Self::SupportingEvidence => "EVIDENCE",
            Self::QuietMetadata => "META",
            Self::DecisionHighlight => "KEY",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
            Self::Info => "INFO",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SemanticToken, ALL_SEMANTIC_TOKENS};

    #[test]
    fn l0_semantic_tokens_have_stable_keys_and_ascii_labels() {
        assert_eq!(ALL_SEMANTIC_TOKENS.len(), 26);

        for token in ALL_SEMANTIC_TOKENS {
            let key = token.key();
            assert!(!key.is_empty(), "{token:?} key is empty");
            assert!(
                key.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_'),
                "{token:?} key is not renderer-safe: {key}"
            );

            let label = token.ascii_label();
            assert!(!label.is_empty(), "{token:?} label is empty");
            assert!(label.is_ascii(), "{token:?} label is not ASCII: {label}");
        }

        assert_eq!(SemanticToken::FitElite.key(), "fit_elite");
        assert_eq!(SemanticToken::FitBuried.ascii_label(), "UNDERUSED");
        assert_eq!(SemanticToken::SourceUnavailable.key(), "source_unavailable");
        assert_eq!(SemanticToken::GameShootout.ascii_label(), "SO");
    }
}
