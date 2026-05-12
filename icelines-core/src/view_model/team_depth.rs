use serde::{Deserialize, Serialize};

use crate::cross_team::{
    compute_all_views_with_mode, compute_team_strength_views, fantasy_score_view, ScoringMode,
    WebFitClass,
};
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
use std::collections::HashMap;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamDepthChartView {
    pub context: ViewContext,
    pub team: TeamAbbr,
    pub scoring_mode: String,
    pub columns: Vec<TeamDepthChartColumn>,
    pub goalies: Vec<DepthGoalieSlot>,
    pub warnings: Vec<ViewWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamTradeImpactView {
    pub context: ViewContext,
    pub team: TeamAbbr,
    pub player_out: TradeImpactPlayer,
    pub player_in: TradeImpactPlayer,
    pub forward_lines: Vec<TradeImpactLine>,
    pub defense_pairs: Vec<TradeImpactPair>,
    pub delta_pace_82: f64,
    pub result_label: String,
    pub warnings: Vec<ViewWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeImpactPlayer {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub pace_82: Option<f64>,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeImpactLine {
    pub line: u8,
    pub before: Vec<Option<TradeImpactSlot>>,
    pub after: Vec<Option<TradeImpactSlot>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeImpactPair {
    pub pair: u8,
    pub before: Vec<Option<TradeImpactSlot>>,
    pub after: Vec<Option<TradeImpactSlot>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeImpactSlot {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub position: Position,
    pub pace_82: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamDepthChartColumn {
    pub key: String,
    pub label: String,
    pub depth: usize,
    pub players: Vec<TeamDepthChartPlayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamDepthChartPlayer {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub position: Position,
    pub line: usize,
    pub score: f64,
    pub fit: Option<WebFitClass>,
    pub metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthLeagueView {
    pub context: ViewContext,
    pub scoring_mode: String,
    pub rows: Vec<DepthTeamStrengthRow>,
    pub warnings: Vec<ViewWarning>,
}

impl DepthLeagueView {
    pub fn pace_from_repository(
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
    ) -> Self {
        let has_window = repo.has_window(season, season_type);
        let skaters: Vec<_> = repo.skaters(season, season_type).collect();
        Self::from_player_views(season, season_type, has_window, &skaters, ScoringMode::Pace)
    }

    pub fn from_player_views(
        season: Season,
        season_type: SeasonType,
        has_window: bool,
        skaters: &[PlayerView<'_>],
        scoring_mode: ScoringMode,
    ) -> Self {
        let strength = compute_team_strength_views(skaters, scoring_mode);
        let mut ranked: Vec<_> = strength.into_iter().collect();
        ranked.sort_by(|a, b| {
            b.1.total
                .partial_cmp(&a.1.total)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        Self {
            context: view_context(season, season_type, has_window),
            scoring_mode: scoring_mode.label().to_string(),
            rows: ranked
                .into_iter()
                .map(|(team, strength)| DepthTeamStrengthRow {
                    team: TeamAbbr(team),
                    c_score: strength.c_score,
                    lw_score: strength.lw_score,
                    rw_score: strength.rw_score,
                    d_score: strength.d_score,
                    total: strength.total,
                    c_top: strength.c_top,
                    lw_top: strength.lw_top,
                    rw_top: strength.rw_top,
                    d_top: strength.d_top,
                })
                .collect(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthTeamStrengthRow {
    pub team: TeamAbbr,
    pub c_score: f64,
    pub lw_score: f64,
    pub rw_score: f64,
    pub d_score: f64,
    pub total: f64,
    pub c_top: String,
    pub lw_top: String,
    pub rw_top: String,
    pub d_top: String,
}

impl TeamDepthView {
    pub fn is_empty(&self) -> bool {
        self.forward_lines
            .iter()
            .all(|line| line.left.is_none() && line.center.is_none() && line.right.is_none())
            && self
                .defense_pairs
                .iter()
                .all(|pair| pair.left.is_none() && pair.right.is_none())
            && self.goalies.is_empty()
            && self.extras.is_empty()
    }

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
            team: team.clone(),
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

impl TeamTradeImpactView {
    pub fn from_player_views(
        team: TeamAbbr,
        season: Season,
        season_type: SeasonType,
        roster: &[PlayerView<'_>],
        player_in: PlayerView<'_>,
        player_out: PlayerView<'_>,
    ) -> Self {
        let before = DepthChartBuilder::build_views(team.clone(), season, roster);
        let after = DepthChartBuilder::build_views_with_swap(
            team.clone(),
            season,
            roster,
            player_in,
            player_out.id(),
        );
        let delta_pace_82 =
            player_in.pace_82().unwrap_or(0.0) - player_out.pace_82().unwrap_or(0.0);

        Self {
            context: view_context(season, season_type, true),
            team,
            player_out: trade_player(&player_out),
            player_in: trade_player(&player_in),
            forward_lines: before
                .forward_lines
                .iter()
                .zip(after.forward_lines.iter())
                .enumerate()
                .map(|(idx, (before, after))| TradeImpactLine {
                    line: idx as u8 + 1,
                    before: before.iter().map(trade_slot).collect(),
                    after: after.iter().map(trade_slot).collect(),
                })
                .collect(),
            defense_pairs: before
                .defense_pairs
                .iter()
                .zip(after.defense_pairs.iter())
                .enumerate()
                .map(|(idx, (before, after))| TradeImpactPair {
                    pair: idx as u8 + 1,
                    before: before.iter().map(trade_slot).collect(),
                    after: after.iter().map(trade_slot).collect(),
                })
                .collect(),
            delta_pace_82,
            result_label: if delta_pace_82 > 5.0 {
                "UPGRADE".to_string()
            } else if delta_pace_82 < -5.0 {
                "DOWNGRADE".to_string()
            } else {
                "ROUGHLY EVEN".to_string()
            },
            warnings: Vec::new(),
        }
    }
}

impl TeamDepthChartView {
    pub fn from_player_views(
        team: TeamAbbr,
        season: Season,
        season_type: SeasonType,
        has_window: bool,
        skaters: &[PlayerView<'_>],
        goalies: &[PlayerView<'_>],
        scoring_mode: ScoringMode,
    ) -> Self {
        let metrics = compute_all_views_with_mode(skaters, scoring_mode);
        let metrics_map: HashMap<u32, WebFitClass> = metrics
            .iter()
            .filter_map(|metric| metric.player_nhl_id.map(|id| (id, metric.web_fit_class())))
            .collect();
        let score_of = |view: &PlayerView<'_>| -> f64 { scoring_mode_score(view, scoring_mode) };

        let mut forwards: Vec<&PlayerView<'_>> = skaters
            .iter()
            .filter(|view| view.team_display() == team.0.as_str() && view.position().is_forward())
            .collect();
        sort_depth_players(&mut forwards, &score_of);

        let mut forward_buckets: HashMap<Position, Vec<&PlayerView<'_>>> = HashMap::new();
        for view in forwards {
            let bucket = forward_buckets.entry(view.position()).or_default();
            if bucket.len() < 4 {
                bucket.push(view);
            } else {
                let natural = match view.identity.bio.shoots_catches.as_deref() {
                    Some("R") => Position::RightWing,
                    _ => Position::LeftWing,
                };
                let spill = if forward_buckets
                    .get(&natural)
                    .map_or(0, |bucket| bucket.len())
                    < 4
                {
                    natural
                } else {
                    [Position::LeftWing, Position::Center, Position::RightWing]
                        .into_iter()
                        .min_by_key(|pos| forward_buckets.get(pos).map_or(0, |bucket| bucket.len()))
                        .unwrap_or(view.position())
                };
                forward_buckets.entry(spill).or_default().push(view);
            }
        }

        let mut defense: Vec<&PlayerView<'_>> = skaters
            .iter()
            .filter(|view| {
                view.team_display() == team.0.as_str() && view.position() == Position::Defense
            })
            .collect();
        sort_depth_players(&mut defense, &score_of);
        let left_defense: Vec<&PlayerView<'_>> = defense
            .iter()
            .filter(|view| view.identity.bio.shoots_catches.as_deref() != Some("R"))
            .copied()
            .collect();
        let right_defense: Vec<&PlayerView<'_>> = defense
            .iter()
            .filter(|view| view.identity.bio.shoots_catches.as_deref() == Some("R"))
            .copied()
            .collect();

        let empty: Vec<&PlayerView<'_>> = Vec::new();
        let columns = vec![
            team_depth_chart_column(
                "lw",
                "LEFT WING",
                4,
                forward_buckets.get(&Position::LeftWing).unwrap_or(&empty),
                &score_of,
                &metrics_map,
            ),
            team_depth_chart_column(
                "c",
                "CENTER",
                4,
                forward_buckets.get(&Position::Center).unwrap_or(&empty),
                &score_of,
                &metrics_map,
            ),
            team_depth_chart_column(
                "rw",
                "RIGHT WING",
                4,
                forward_buckets.get(&Position::RightWing).unwrap_or(&empty),
                &score_of,
                &metrics_map,
            ),
            team_depth_chart_column("ld", "LD", 3, &left_defense, &score_of, &metrics_map),
            team_depth_chart_column("rd", "RD", 3, &right_defense, &score_of, &metrics_map),
        ];

        Self {
            context: view_context(season, season_type, has_window),
            team: team.clone(),
            scoring_mode: scoring_mode.label().to_string(),
            columns,
            goalies: goalies
                .iter()
                .filter(|goalie| goalie.team_display() == team.0.as_str())
                .map(goalie_slot)
                .collect(),
            warnings: Vec::new(),
        }
    }
}

fn scoring_mode_score(view: &PlayerView<'_>, scoring_mode: ScoringMode) -> f64 {
    match scoring_mode {
        ScoringMode::Fantasy => fantasy_score_view(view),
        ScoringMode::Pace => view.pace_82().unwrap_or(0.0),
        ScoringMode::Custom(stat) => stat.read(view).unwrap_or(0.0),
    }
}

fn sort_depth_players(
    players: &mut Vec<&PlayerView<'_>>,
    score_of: &impl Fn(&PlayerView<'_>) -> f64,
) {
    players.sort_by(|a, b| {
        score_of(b)
            .partial_cmp(&score_of(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id().0.cmp(&b.id().0))
    });
}

fn team_depth_chart_column(
    key: &str,
    label: &str,
    depth: usize,
    players: &[&PlayerView<'_>],
    score_of: &impl Fn(&PlayerView<'_>) -> f64,
    metrics_map: &HashMap<u32, WebFitClass>,
) -> TeamDepthChartColumn {
    TeamDepthChartColumn {
        key: key.to_string(),
        label: label.to_string(),
        depth,
        players: players
            .iter()
            .enumerate()
            .map(|(idx, player)| team_depth_chart_player(idx + 1, player, score_of, metrics_map))
            .collect(),
    }
}

fn team_depth_chart_player(
    line: usize,
    player: &PlayerView<'_>,
    score_of: &impl Fn(&PlayerView<'_>) -> f64,
    metrics_map: &HashMap<u32, WebFitClass>,
) -> TeamDepthChartPlayer {
    let score = score_of(player);
    TeamDepthChartPlayer {
        player_id: player.id(),
        display_name: player.full_name().to_string(),
        team: player
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        position: player.position(),
        line,
        score,
        fit: metrics_map.get(&player.id().0).copied(),
        metrics: vec![MetricCell {
            key: StatKey::from("score"),
            label: "Score".to_string(),
            value: MetricValue::Decimal(score),
            unit: MetricUnit::Score,
            precision: ValuePrecision::OneDecimal,
            token: None,
        }],
        tokens: vec![SemanticToken::SupportingEvidence],
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
    pub headshot_canonical_url: Option<String>,
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
        headshot_canonical_url: slot.headshot_canonical_url.clone(),
        slot: kind,
        position: slot.position,
        evidence: DeploymentEvidence::Estimated,
        metrics: vec![
            metric_decimal("pace_82", "Pace 82", slot.pace_82),
            metric_decimal("goals_per_82", "G/82", slot.goals_per_82),
            metric_optional_int("goals", "G", slot.goals, MetricUnit::Goals),
            metric_optional_int("assists", "A", slot.assists, MetricUnit::Assists),
            metric_optional_int("points", "PTS", slot.points, MetricUnit::Points),
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
            MetricCell {
                key: StatKey::from("gaa"),
                label: "GAA".to_string(),
                value: stats
                    .and_then(|g| g.goals_against_average.map(|v| v as f64))
                    .map(MetricValue::Decimal)
                    .unwrap_or(MetricValue::Missing),
                unit: MetricUnit::Score,
                precision: ValuePrecision::TwoDecimals,
                token: None,
            },
            metric_optional_int("wins", "W", stats.map(|g| g.wins), MetricUnit::Count),
            metric_optional_int("losses", "L", stats.map(|g| g.losses), MetricUnit::Count),
            metric_optional_int(
                "ot_losses",
                "OTL",
                stats.and_then(|g| g.ot_losses),
                MetricUnit::Count,
            ),
            metric_optional_int(
                "shutouts",
                "SO",
                stats.map(|g| g.shutouts),
                MetricUnit::Count,
            ),
        ],
        tokens: vec![SemanticToken::SupportingEvidence],
    }
}

fn trade_player(player: &PlayerView<'_>) -> TradeImpactPlayer {
    TradeImpactPlayer {
        player_id: player.id(),
        display_name: player.full_name().to_string(),
        team: player
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        pace_82: player.pace_82(),
        points_per_game: player.pace_82().map(|pace| pace / 82.0),
    }
}

fn trade_slot(slot: &Option<DepthChartSlot>) -> Option<TradeImpactSlot> {
    slot.as_ref().map(|slot| TradeImpactSlot {
        player_id: slot.player_id,
        display_name: slot.full_name.clone(),
        team: slot.team.clone(),
        position: slot.position,
        pace_82: slot.pace_82,
    })
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

fn metric_optional_int(key: &str, label: &str, value: Option<u32>, unit: MetricUnit) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: value
            .map(|value| MetricValue::Integer(value as i64))
            .unwrap_or(MetricValue::Missing),
        unit,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::model::{Position, Season, TeamAbbr};
    use crate::season_stats::SeasonType;
    use crate::stats_repository::StatsRepository;

    fn repo_with_depth_players() -> StatsRepository {
        let mut repo = StatsRepository::new();
        for (id, name, position, hand, team) in [
            (1, "Left Wing One", Position::LeftWing, "L", "SEA"),
            (2, "Center One", Position::Center, "L", "SEA"),
            (3, "Right Wing One", Position::RightWing, "R", "SEA"),
            (4, "Left Defense", Position::Defense, "L", "SEA"),
            (5, "Right Defense", Position::Defense, "R", "SEA"),
            (6, "Other Team Center", Position::Center, "L", "EDM"),
        ] {
            repo.upsert_identity(
                fixtures::identity(id)
                    .name(name, &name.to_ascii_lowercase())
                    .shoots(hand)
                    .build(),
            )
            .unwrap();
            repo.upsert_stats(
                fixtures::stats(id, 20252026, team)
                    .position(position)
                    .build(),
            )
            .unwrap();
        }
        repo
    }

    #[test]
    fn l0_team_depth_chart_view_projects_tui_columns() {
        let repo = repo_with_depth_players();
        let skaters: Vec<PlayerView<'_>> = repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect();
        let view = TeamDepthChartView::from_player_views(
            TeamAbbr("SEA".to_string()),
            Season(20252026),
            SeasonType::Regular,
            true,
            &skaters,
            &[],
            ScoringMode::Pace,
        );

        assert_eq!(view.scoring_mode, "Pts/82");
        assert_eq!(
            view.columns
                .iter()
                .map(|column| column.key.as_str())
                .collect::<Vec<_>>(),
            vec!["lw", "c", "rw", "ld", "rd"]
        );
        assert_eq!(view.columns[0].players[0].display_name, "Left Wing One");
        assert_eq!(view.columns[1].players[0].display_name, "Center One");
        assert_eq!(view.columns[2].players[0].display_name, "Right Wing One");
        assert_eq!(view.columns[3].players[0].display_name, "Left Defense");
        assert_eq!(view.columns[4].players[0].display_name, "Right Defense");
        assert_eq!(view.columns[0].players[0].score, 93.7);
    }

    #[test]
    fn l0_team_trade_impact_view_projects_before_after_depth() {
        let repo = repo_with_depth_players();
        let season = Season(20252026);
        let season_type = SeasonType::Regular;
        let roster = repo.team_roster(&TeamAbbr("SEA".to_string()), season, season_type);
        let player_out = repo.view(PlayerId(3), season, season_type).unwrap();
        let player_in = repo.view(PlayerId(6), season, season_type).unwrap();

        let view = TeamTradeImpactView::from_player_views(
            TeamAbbr("SEA".to_string()),
            season,
            season_type,
            &roster,
            player_in,
            player_out,
        );

        assert_eq!(view.team.as_str(), "SEA");
        assert_eq!(view.player_out.display_name, "Right Wing One");
        assert_eq!(view.player_in.display_name, "Other Team Center");
        assert!(view.forward_lines.iter().any(|line| line
            .before
            .iter()
            .flatten()
            .any(|slot| slot.display_name == "Right Wing One")));
        assert!(!view.forward_lines.iter().any(|line| line
            .after
            .iter()
            .flatten()
            .any(|slot| slot.display_name == "Right Wing One")));
        let placed_in = view
            .forward_lines
            .iter()
            .flat_map(|line| line.after.iter().flatten())
            .find(|slot| slot.display_name == "Other Team Center")
            .expect("incoming player should be placed after swap");
        assert_eq!(placed_in.team.as_str(), "SEA");
    }
}
