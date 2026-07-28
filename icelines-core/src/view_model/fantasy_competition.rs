use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const FANTASY_COMPETITION_RULES_SCHEMA: &str = "fantasy_competition_rules.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyCompetitionMode {
    Points,
    Categories,
}

impl FantasyCompetitionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Points => "points",
            Self::Categories => "categories",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyCategoryDirection {
    HigherWins,
    LowerWins,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyCategoryAggregation {
    Sum,
    /// Numerators and denominators are summed separately before division.
    Ratio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyMatchupTiePolicy {
    Tie,
    HigherSeedWins,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCategoryRule {
    pub key: String,
    pub label: String,
    pub direction: FantasyCategoryDirection,
    pub aggregation: FantasyCategoryAggregation,
    pub tie_epsilon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyCompetitionRules {
    pub schema: String,
    pub mode: FantasyCompetitionMode,
    pub categories: Vec<FantasyCategoryRule>,
    pub minimum_goalie_appearances: u8,
    pub matchup_tie_policy: FantasyMatchupTiePolicy,
}

impl FantasyCompetitionRules {
    pub fn points() -> Self {
        Self {
            schema: FANTASY_COMPETITION_RULES_SCHEMA.to_owned(),
            mode: FantasyCompetitionMode::Points,
            categories: Vec::new(),
            minimum_goalie_appearances: 0,
            matchup_tie_policy: FantasyMatchupTiePolicy::Tie,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != FANTASY_COMPETITION_RULES_SCHEMA {
            return Err(format!(
                "unsupported fantasy competition rules schema '{}'",
                self.schema
            ));
        }
        match self.mode {
            FantasyCompetitionMode::Points if !self.categories.is_empty() => {
                return Err("points mode cannot contain category rules".to_owned());
            }
            FantasyCompetitionMode::Points if self.minimum_goalie_appearances != 0 => {
                return Err("points mode cannot require category goalie appearances".to_owned());
            }
            FantasyCompetitionMode::Categories if self.categories.is_empty() => {
                return Err("category mode requires at least one scored category".to_owned());
            }
            _ => {}
        }

        let mut keys = BTreeSet::new();
        for category in &self.categories {
            let key = category.key.trim().to_ascii_lowercase();
            if key.is_empty() || category.label.trim().is_empty() {
                return Err("category key and label are required".to_owned());
            }
            if !key
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_')
            {
                return Err(format!(
                    "category key '{}' must use lowercase letters and underscores",
                    category.key
                ));
            }
            if !keys.insert(key) {
                return Err(format!("duplicate category key '{}'", category.key));
            }
            if !category.tie_epsilon.is_finite() || category.tie_epsilon < 0.0 {
                return Err(format!(
                    "category '{}' tie epsilon must be finite and non-negative",
                    category.key
                ));
            }
        }
        Ok(())
    }
}

impl Default for FantasyCompetitionRules {
    fn default() -> Self {
        Self::points()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(key: &str) -> FantasyCategoryRule {
        FantasyCategoryRule {
            key: key.to_owned(),
            label: key.to_uppercase(),
            direction: FantasyCategoryDirection::HigherWins,
            aggregation: FantasyCategoryAggregation::Sum,
            tie_epsilon: 0.0,
        }
    }

    #[test]
    fn category_rules_validate_directions_ratios_and_goalie_minimum() {
        let rules = FantasyCompetitionRules {
            schema: FANTASY_COMPETITION_RULES_SCHEMA.to_owned(),
            mode: FantasyCompetitionMode::Categories,
            categories: vec![
                category("goals"),
                FantasyCategoryRule {
                    key: "goals_against_average".to_owned(),
                    label: "GAA".to_owned(),
                    direction: FantasyCategoryDirection::LowerWins,
                    aggregation: FantasyCategoryAggregation::Ratio,
                    tie_epsilon: 0.001,
                },
            ],
            minimum_goalie_appearances: 3,
            matchup_tie_policy: FantasyMatchupTiePolicy::Tie,
        };
        assert!(rules.validate().is_ok());
    }

    #[test]
    fn category_rules_reject_duplicate_keys_and_points_categories() {
        let duplicate = FantasyCompetitionRules {
            schema: FANTASY_COMPETITION_RULES_SCHEMA.to_owned(),
            mode: FantasyCompetitionMode::Categories,
            categories: vec![category("goals"), category("goals")],
            minimum_goalie_appearances: 0,
            matchup_tie_policy: FantasyMatchupTiePolicy::Tie,
        };
        assert!(duplicate.validate().is_err());

        let mut points = FantasyCompetitionRules::points();
        points.categories.push(category("goals"));
        assert!(points.validate().is_err());
    }
}
