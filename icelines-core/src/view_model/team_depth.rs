use serde::{Deserialize, Serialize};

use crate::depth_chart::DepthChartBuilder;
use crate::identity::PlayerId;
use crate::model::{DepthChartSlot, Position, Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    Completeness, SourceKind, SourceState, ViewContext, ViewWarning, ViewWindow,
};
use crate::view_model::tokens::{
    MetricCell, MetricUnit, MetricValue, SemanticToken, StatKey, ValuePrecision,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamDepthView {
    pub context: ViewContext,
    pub team: TeamAbbr,
    pub summary: DepthSummary,
    pub forward_lines: Vec<DepthLine>,
    pub defense_pairs: Vec<DepthPair>,
    pub goalies: Vec<DepthGoalieSlot>,
    pub extras: Vec<DepthPlayerSlot>,
    pub warnings: Vec<ViewWarning>,
}

impl TeamDepthView {
    pub fn from_repository(
        repo: &StatsRepository,
        team: TeamAbbr,
        season: Season,
        season_type: SeasonType,
    ) -> Self {
        let has_window = repo.has_window(season, season_type);
        let roster = repo.team_roster(&team, season, season_type);
        Self::from_roster(team, season, season_type, has_window, &roster)
    }

    pub fn from_player_views(
        team: TeamAbbr,
        season: Season,
        season_type: SeasonType,
        roster: &[PlayerView<'_>],
    ) -> Self {
        Self::from_roster(team, season, season_type, true, roster)
    }

    fn from_roster(
        team: TeamAbbr,
        season: Season,
        season_type: SeasonType,
        has_window: bool,
        roster: &[PlayerView<'_>],
    ) -> Self {
        let skaters: Vec<PlayerView<'_>> = roster
            .iter()
            .filter(|player| !player.is_goalie())
            .cloned()
            .collect();
        let goalies: Vec<DepthGoalieSlot> = roster
            .iter()
            .filter(|player| player.is_goalie())
            .map(goalie_slot)
            .collect();
        let chart = DepthChartBuilder::build_views(team.clone(), season, &skaters);

        let forward_lines = chart
            .forward_lines
            .into_iter()
            .enumerate()
            .map(|(idx, slots)| DepthLine {
                line: idx as u8 + 1,
                left: slots[0].as_ref().map(|slot| {
                    depth_slot(
                        slot,
                        DepthSlotKind::forward(idx as u8 + 1, ForwardSlot::LeftWing),
                    )
                }),
                center: slots[1].as_ref().map(|slot| {
                    depth_slot(
                        slot,
                        DepthSlotKind::forward(idx as u8 + 1, ForwardSlot::Center),
                    )
                }),
                right: slots[2].as_ref().map(|slot| {
                    depth_slot(
                        slot,
                        DepthSlotKind::forward(idx as u8 + 1, ForwardSlot::RightWing),
                    )
                }),
            })
            .collect();

        let defense_pairs = chart
            .defense_pairs
            .into_iter()
            .enumerate()
            .map(|(idx, slots)| DepthPair {
                pair: idx as u8 + 1,
                left: slots[0].as_ref().map(|slot| {
                    depth_slot(
                        slot,
                        DepthSlotKind::defense(idx as u8 + 1, DefenseSide::Left),
                    )
                }),
                right: slots[1].as_ref().map(|slot| {
                    depth_slot(
                        slot,
                        DepthSlotKind::defense(idx as u8 + 1, DefenseSide::Right),
                    )
                }),
            })
            .collect();

        let mut extras: Vec<DepthPlayerSlot> = chart
            .below_min_gp
            .iter()
            .map(|slot| depth_slot(slot, DepthSlotKind::Extra))
            .collect();
        extras.extend(
            chart
                .unplaced
                .iter()
                .map(|slot| depth_slot(slot, DepthSlotKind::Unplaced)),
        );

        Self {
            context: view_context(season, season_type, has_window),
            team,
            summary: DepthSummary {
                title: "Estimated depth".to_string(),
                metrics: vec![metric_int("rostered", "Rostered", roster.len() as i64)],
                tokens: vec![SemanticToken::SupportingEvidence],
            },
            forward_lines,
            defense_pairs,
            goalies,
            extras,
            warnings: Vec::new(),
        }
    }
}

fn view_context(season: Season, season_type: SeasonType, has_window: bool) -> ViewContext {
    let mut context = ViewContext::new(ViewWindow::new(season, season_type));
    if !has_window {
        context.completeness = Completeness::Unavailable;
        context
            .source_state
            .push(SourceState::missing(SourceKind::Roster));
    }
    context
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthSummary {
    pub title: String,
    pub metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthLine {
    pub line: u8,
    pub left: Option<DepthPlayerSlot>,
    pub center: Option<DepthPlayerSlot>,
    pub right: Option<DepthPlayerSlot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthPair {
    pub pair: u8,
    pub left: Option<DepthPlayerSlot>,
    pub right: Option<DepthPlayerSlot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthPlayerSlot {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub slot: DepthSlotKind,
    pub position: Position,
    pub evidence: DeploymentEvidence,
    pub metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthGoalieSlot {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub role: String,
    pub evidence: DeploymentEvidence,
    pub metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepthSlotKind {
    Forward { line: u8, slot: ForwardSlot },
    Defense { pair: u8, side: DefenseSide },
    Goalie,
    Extra,
    Unplaced,
}

impl DepthSlotKind {
    fn forward(line: u8, slot: ForwardSlot) -> Self {
        Self::Forward { line, slot }
    }

    fn defense(pair: u8, side: DefenseSide) -> Self {
        Self::Defense { pair, side }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardSlot {
    LeftWing,
    Center,
    RightWing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefenseSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentEvidence {
    Actual,
    Estimated,
    Unknown,
}

fn depth_slot(slot: &DepthChartSlot, kind: DepthSlotKind) -> DepthPlayerSlot {
    DepthPlayerSlot {
        player_id: slot.player_id,
        display_name: slot.full_name.clone(),
        team: slot.team.clone(),
        slot: kind,
        position: slot.position,
        evidence: DeploymentEvidence::Estimated,
        metrics: vec![
            metric_decimal("pace_82", "Pace 82", slot.pace_82),
            metric_decimal("goals_per_82", "G/82", slot.goals_per_82),
            MetricCell {
                key: StatKey::from("gp"),
                label: "GP".to_string(),
                value: slot
                    .gp
                    .map(|gp| MetricValue::Integer(gp as i64))
                    .unwrap_or(MetricValue::Missing),
                unit: MetricUnit::Games,
                precision: ValuePrecision::Integer,
                token: None,
            },
        ],
        tokens: vec![SemanticToken::SupportingEvidence],
    }
}

fn goalie_slot(goalie: &PlayerView<'_>) -> DepthGoalieSlot {
    let stats = goalie.stats.goalie.as_ref();
    let starts = stats.map(|g| g.games_started).unwrap_or(0);
    let role = if starts >= 45 {
        "starter"
    } else if starts >= 25 {
        "tandem"
    } else if starts > 0 {
        "backup"
    } else {
        "unknown"
    };

    DepthGoalieSlot {
        player_id: goalie.id(),
        display_name: goalie.full_name().to_string(),
        team: goalie
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        role: role.to_string(),
        evidence: if starts > 0 {
            DeploymentEvidence::Actual
        } else {
            DeploymentEvidence::Unknown
        },
        metrics: vec![
            MetricCell {
                key: StatKey::from("gp"),
                label: "GP".to_string(),
                value: MetricValue::Integer(goalie.gp() as i64),
                unit: MetricUnit::Games,
                precision: ValuePrecision::Integer,
                token: None,
            },
            MetricCell {
                key: StatKey::from("starts"),
                label: "GS".to_string(),
                value: MetricValue::Integer(starts as i64),
                unit: MetricUnit::Games,
                precision: ValuePrecision::Integer,
                token: None,
            },
            MetricCell {
                key: StatKey::from("save_pct"),
                label: "SV%".to_string(),
                value: stats
                    .and_then(|g| g.save_pct.map(|v| v as f64))
                    .map(MetricValue::Decimal)
                    .unwrap_or(MetricValue::Missing),
                unit: MetricUnit::Percentage,
                precision: ValuePrecision::ThreeDecimals,
                token: None,
            },
        ],
        tokens: vec![SemanticToken::SupportingEvidence],
    }
}

fn metric_int(key: &str, label: &str, value: i64) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: MetricValue::Integer(value),
        unit: MetricUnit::Count,
        precision: ValuePrecision::Integer,
        token: None,
    }
}

fn metric_decimal(key: &str, label: &str, value: Option<f64>) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: value
            .map(MetricValue::Decimal)
            .unwrap_or(MetricValue::Missing),
        unit: MetricUnit::Per82,
        precision: ValuePrecision::OneDecimal,
        token: None,
    }
}
