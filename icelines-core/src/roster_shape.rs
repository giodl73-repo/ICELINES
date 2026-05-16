use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::model::Position;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterShape {
    pub name: String,
    pub description: String,
    pub rules: Vec<RosterSlotRule>,
}

impl RosterShape {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        rules: Vec<RosterSlotRule>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            rules,
        }
    }

    pub fn yahoo_standard() -> Self {
        Self::new(
            "yahoo-standard",
            "Yahoo-style fantasy roster shape",
            vec![
                RosterSlotRule::min_max(RosterPositionGroup::Center, 2, Some(4)),
                RosterSlotRule::min_max(RosterPositionGroup::LeftWing, 2, Some(4)),
                RosterSlotRule::min_max(RosterPositionGroup::RightWing, 2, Some(4)),
                RosterSlotRule::min_max(RosterPositionGroup::Defense, 4, Some(6)),
                RosterSlotRule::min_max(RosterPositionGroup::Goalie, 2, Some(3)),
                RosterSlotRule::max(RosterPositionGroup::Total, 23),
            ],
        )
    }

    pub fn validates_group(&self, position: Position) -> bool {
        self.rules.iter().any(|rule| {
            rule.group != RosterPositionGroup::Total && rule.group.matches_position(position)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterPositionGroup {
    Center,
    LeftWing,
    RightWing,
    Forward,
    Defense,
    Skater,
    Goalie,
    Utility,
    Total,
}

impl RosterPositionGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Center => "C",
            Self::LeftWing => "LW",
            Self::RightWing => "RW",
            Self::Forward => "F",
            Self::Defense => "D",
            Self::Skater => "Skater",
            Self::Goalie => "G",
            Self::Utility => "UTIL",
            Self::Total => "Total",
        }
    }

    pub fn matches_position(self, position: Position) -> bool {
        match self {
            Self::Center => position == Position::Center,
            Self::LeftWing => position == Position::LeftWing,
            Self::RightWing => position == Position::RightWing,
            Self::Forward => position.is_forward(),
            Self::Defense => position.is_defense(),
            Self::Skater => position != Position::Goalie,
            Self::Goalie => position == Position::Goalie,
            Self::Utility | Self::Total => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterSlotRule {
    pub group: RosterPositionGroup,
    pub min: u16,
    pub max: Option<u16>,
}

impl RosterSlotRule {
    pub fn min_max(group: RosterPositionGroup, min: u16, max: Option<u16>) -> Self {
        Self { group, min, max }
    }

    pub fn min(group: RosterPositionGroup, min: u16) -> Self {
        Self {
            group,
            min,
            max: None,
        }
    }

    pub fn max(group: RosterPositionGroup, max: u16) -> Self {
        Self {
            group,
            min: 0,
            max: Some(max),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterShapeValidationInput {
    pub league: String,
    pub team: String,
    pub shape: RosterShape,
    pub players: Vec<RosterShapePlayerInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterShapePlayerInput {
    pub player_key: String,
    pub display_name: String,
    pub positions: Vec<Position>,
}

impl RosterShapePlayerInput {
    pub fn known(
        player_key: impl Into<String>,
        display_name: impl Into<String>,
        positions: Vec<Position>,
    ) -> Self {
        Self {
            player_key: player_key.into(),
            display_name: display_name.into(),
            positions,
        }
    }

    pub fn unknown(player_key: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            player_key: player_key.into(),
            display_name: display_name.into(),
            positions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterShapeValidationView {
    pub league: String,
    pub team: String,
    pub shape_name: String,
    pub status: RosterShapeStatus,
    pub summary: RosterShapeSummary,
    pub slots: Vec<RosterShapeSlotRow>,
    pub player_issues: Vec<RosterShapePlayerIssue>,
}

impl RosterShapeValidationView {
    pub fn validate(input: RosterShapeValidationInput) -> Self {
        let mut counts = BTreeMap::<RosterPositionGroup, u16>::new();
        let mut player_issues = Vec::new();
        let mut seen = BTreeSet::<String>::new();

        for player in &input.players {
            if !seen.insert(player.player_key.clone()) {
                player_issues.push(RosterShapePlayerIssue {
                    player_key: player.player_key.clone(),
                    display_name: player.display_name.clone(),
                    kind: RosterShapeIssueKind::DuplicatePlayer,
                    message: format!(
                        "{} appears more than once on this roster",
                        player.display_name
                    ),
                });
                continue;
            }

            *counts.entry(RosterPositionGroup::Total).or_default() += 1;

            if player.positions.is_empty() {
                player_issues.push(RosterShapePlayerIssue {
                    player_key: player.player_key.clone(),
                    display_name: player.display_name.clone(),
                    kind: RosterShapeIssueKind::UnknownPlayer,
                    message: format!(
                        "{} could not be resolved to a canonical roster position",
                        player.display_name
                    ),
                });
                continue;
            }

            if !player
                .positions
                .iter()
                .copied()
                .any(|position| input.shape.validates_group(position))
            {
                player_issues.push(RosterShapePlayerIssue {
                    player_key: player.player_key.clone(),
                    display_name: player.display_name.clone(),
                    kind: RosterShapeIssueKind::IneligibleForShape,
                    message: format!(
                        "{} does not match any configured roster slot",
                        player.display_name
                    ),
                });
            }

            for rule in &input.shape.rules {
                if rule.group == RosterPositionGroup::Total {
                    continue;
                }
                if player
                    .positions
                    .iter()
                    .copied()
                    .any(|position| rule.group.matches_position(position))
                {
                    *counts.entry(rule.group).or_default() += 1;
                }
            }
        }

        let slots = input
            .shape
            .rules
            .iter()
            .map(|rule| slot_row(rule, *counts.get(&rule.group).unwrap_or(&0)))
            .collect::<Vec<_>>();

        let missing_slots = slots
            .iter()
            .filter(|slot| slot.status == RosterSlotStatus::Missing)
            .count();
        let overflow_slots = slots
            .iter()
            .filter(|slot| slot.status == RosterSlotStatus::OverLimit)
            .count();
        let unknown_players = player_issues
            .iter()
            .filter(|issue| issue.kind == RosterShapeIssueKind::UnknownPlayer)
            .count();
        let duplicate_players = player_issues
            .iter()
            .filter(|issue| issue.kind == RosterShapeIssueKind::DuplicatePlayer)
            .count();
        let ineligible_players = player_issues
            .iter()
            .filter(|issue| issue.kind == RosterShapeIssueKind::IneligibleForShape)
            .count();
        let status = if missing_slots == 0
            && overflow_slots == 0
            && unknown_players == 0
            && duplicate_players == 0
            && ineligible_players == 0
        {
            RosterShapeStatus::Legal
        } else {
            RosterShapeStatus::Invalid
        };

        Self {
            league: input.league,
            team: input.team,
            shape_name: input.shape.name,
            status,
            summary: RosterShapeSummary {
                rostered_players: counts
                    .get(&RosterPositionGroup::Total)
                    .copied()
                    .unwrap_or_default(),
                missing_slots,
                overflow_slots,
                unknown_players,
                duplicate_players,
                ineligible_players,
            },
            slots,
            player_issues,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterShapeStatus {
    Legal,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterShapeSummary {
    pub rostered_players: u16,
    pub missing_slots: usize,
    pub overflow_slots: usize,
    pub unknown_players: usize,
    pub duplicate_players: usize,
    pub ineligible_players: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterShapeSlotRow {
    pub group: RosterPositionGroup,
    pub label: String,
    pub min: u16,
    pub max: Option<u16>,
    pub count: u16,
    pub status: RosterSlotStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterSlotStatus {
    Ok,
    Missing,
    OverLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterShapePlayerIssue {
    pub player_key: String,
    pub display_name: String,
    pub kind: RosterShapeIssueKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterShapeIssueKind {
    UnknownPlayer,
    DuplicatePlayer,
    IneligibleForShape,
}

fn slot_row(rule: &RosterSlotRule, count: u16) -> RosterShapeSlotRow {
    let (status, message) = if count < rule.min {
        let missing = rule.min - count;
        (
            RosterSlotStatus::Missing,
            Some(format!("missing {missing} {}", rule.group.label())),
        )
    } else if rule.max.is_some_and(|max| count > max) {
        let overflow = count - rule.max.unwrap_or(count);
        (
            RosterSlotStatus::OverLimit,
            Some(format!("over by {overflow} {}", rule.group.label())),
        )
    } else {
        (RosterSlotStatus::Ok, None)
    };

    RosterShapeSlotRow {
        group: rule.group,
        label: rule.group.label().to_string(),
        min: rule.min,
        max: rule.max,
        count,
        status,
        message,
    }
}

#[cfg(test)]
mod roster_shape_tests {
    use super::*;

    fn small_shape() -> RosterShape {
        RosterShape::new(
            "small",
            "Small test shape",
            vec![
                RosterSlotRule::min_max(RosterPositionGroup::Center, 1, Some(1)),
                RosterSlotRule::min_max(RosterPositionGroup::Defense, 1, Some(1)),
                RosterSlotRule::min_max(RosterPositionGroup::Goalie, 1, Some(1)),
                RosterSlotRule::max(RosterPositionGroup::Total, 3),
            ],
        )
    }

    fn validate(players: Vec<RosterShapePlayerInput>) -> RosterShapeValidationView {
        RosterShapeValidationView::validate(RosterShapeValidationInput {
            league: "Office League".to_string(),
            team: "My Team".to_string(),
            shape: small_shape(),
            players,
        })
    }

    fn known(key: &str, name: &str, position: Position) -> RosterShapePlayerInput {
        RosterShapePlayerInput::known(key, name, vec![position])
    }

    #[test]
    fn roster_shape_legal_roster_has_no_issues() {
        let view = validate(vec![
            known("mcdavid", "Connor McDavid", Position::Center),
            known("makar", "Cale Makar", Position::Defense),
            known("hellebuyck", "Connor Hellebuyck", Position::Goalie),
        ]);

        assert_eq!(view.status, RosterShapeStatus::Legal);
        assert_eq!(view.summary.rostered_players, 3);
        assert!(view.player_issues.is_empty());
        assert!(view
            .slots
            .iter()
            .all(|slot| slot.status == RosterSlotStatus::Ok));
    }

    #[test]
    fn roster_shape_reports_underfilled_required_slots() {
        let view = validate(vec![
            known("mcdavid", "Connor McDavid", Position::Center),
            known("makar", "Cale Makar", Position::Defense),
        ]);

        assert_eq!(view.status, RosterShapeStatus::Invalid);
        assert_eq!(view.summary.missing_slots, 1);
        let goalie = view
            .slots
            .iter()
            .find(|slot| slot.group == RosterPositionGroup::Goalie)
            .expect("goalie slot row");
        assert_eq!(goalie.status, RosterSlotStatus::Missing);
        assert_eq!(goalie.message.as_deref(), Some("missing 1 G"));
    }

    #[test]
    fn roster_shape_reports_overfilled_position_and_total_caps() {
        let view = validate(vec![
            known("mcdavid", "Connor McDavid", Position::Center),
            known("crosby", "Sidney Crosby", Position::Center),
            known("makar", "Cale Makar", Position::Defense),
            known("hellebuyck", "Connor Hellebuyck", Position::Goalie),
        ]);

        assert_eq!(view.status, RosterShapeStatus::Invalid);
        assert_eq!(view.summary.overflow_slots, 2);
        let center = view
            .slots
            .iter()
            .find(|slot| slot.group == RosterPositionGroup::Center)
            .expect("center slot row");
        assert_eq!(center.status, RosterSlotStatus::OverLimit);
        assert_eq!(center.message.as_deref(), Some("over by 1 C"));
    }

    #[test]
    fn roster_shape_reports_unknown_players() {
        let view = validate(vec![
            known("mcdavid", "Connor McDavid", Position::Center),
            known("makar", "Cale Makar", Position::Defense),
            RosterShapePlayerInput::unknown("mystery", "Mystery Player"),
        ]);

        assert_eq!(view.status, RosterShapeStatus::Invalid);
        assert_eq!(view.summary.unknown_players, 1);
        assert_eq!(
            view.player_issues[0].kind,
            RosterShapeIssueKind::UnknownPlayer
        );
    }

    #[test]
    fn roster_shape_reports_duplicate_players_without_double_counting() {
        let view = validate(vec![
            known("mcdavid", "Connor McDavid", Position::Center),
            known("mcdavid", "Connor McDavid", Position::Center),
            known("makar", "Cale Makar", Position::Defense),
            known("hellebuyck", "Connor Hellebuyck", Position::Goalie),
        ]);

        assert_eq!(view.status, RosterShapeStatus::Invalid);
        assert_eq!(view.summary.rostered_players, 3);
        assert_eq!(view.summary.duplicate_players, 1);
        assert_eq!(
            view.player_issues[0].kind,
            RosterShapeIssueKind::DuplicatePlayer
        );
    }

    #[test]
    fn roster_shape_reports_goalie_skater_mismatch() {
        let shape = RosterShape::new(
            "goalie-only",
            "Goalie-only test shape",
            vec![RosterSlotRule::min_max(
                RosterPositionGroup::Goalie,
                1,
                Some(1),
            )],
        );
        let view = RosterShapeValidationView::validate(RosterShapeValidationInput {
            league: "Office League".to_string(),
            team: "My Team".to_string(),
            shape,
            players: vec![known("mcdavid", "Connor McDavid", Position::Center)],
        });

        assert_eq!(view.status, RosterShapeStatus::Invalid);
        assert_eq!(view.summary.ineligible_players, 1);
        assert_eq!(view.summary.missing_slots, 1);
        assert_eq!(
            view.player_issues[0].kind,
            RosterShapeIssueKind::IneligibleForShape
        );
    }
}
