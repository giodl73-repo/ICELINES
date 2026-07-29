//! Authoritative player display scores and renderer-neutral NHL lineup projection.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::Position;

use super::card::{
    CardAssetFallback, CardAssetKind, CardAssetReference, CardAssetState, CardAssetView,
    CardLineupGroupKind, CardLineupGroupView, CardLineupSlotView, CardMetricView,
    LineupSectionView,
};
use super::tokens::{MetricCell, MetricUnit, MetricValue, StatKey, ValuePrecision};
use super::{EvidenceLabel, SourceKind, TeamCeilingLens};

pub const ICELINES_PLAYER_SCORE_SCHEMA: &str = "icelines_player_score.v1";
pub const ICELINES_PLAYER_SCORE_METHOD: &str = "team_ceiling_multilens.v1";
pub const TEAM_LINEUP_PROJECTION_SCHEMA: &str = "team_lineup_projection.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerScorePositionGroup {
    Forward,
    Defense,
    Goalie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IceLinesPlayerScoreComponent {
    pub lens: TeamCeilingLens,
    pub label: String,
    pub raw_value: Option<f64>,
    pub normalized_value: Option<f64>,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IceLinesPlayerScoreView {
    pub schema: String,
    pub method: String,
    pub position_group: PlayerScorePositionGroup,
    pub value: Option<f64>,
    /// Integer display score or `NR`; renderers must not reinterpret it.
    pub display: String,
    pub sample_games: u32,
    pub coverage_pct: f64,
    pub evidence_label: EvidenceLabel,
    pub components: Vec<IceLinesPlayerScoreComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineupAssignmentEvidence {
    Actual,
    Reported,
    Estimated,
    Scenario,
}

impl LineupAssignmentEvidence {
    pub fn card_evidence_label(self) -> EvidenceLabel {
        match self {
            Self::Actual => EvidenceLabel::Confirmed,
            Self::Reported => EvidenceLabel::Reported,
            Self::Estimated => EvidenceLabel::Estimated,
            Self::Scenario => EvidenceLabel::Simulated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineupForwardPosition {
    LeftWing,
    Center,
    RightWing,
}

impl LineupForwardPosition {
    fn position(self) -> Position {
        match self {
            Self::LeftWing => Position::LeftWing,
            Self::Center => Position::Center,
            Self::RightWing => Position::RightWing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TeamLineupRequestedSlot {
    Forward {
        line: u8,
        position: LineupForwardPosition,
    },
    /// Explicit manager/scenario decision to deploy a forward away from the
    /// player's natural eligibility while retaining the natural positions in
    /// the player record.
    FlexibleForward {
        line: u8,
        position: LineupForwardPosition,
    },
    Defense {
        pair: u8,
        right_side: bool,
    },
    Goalie {
        starter: bool,
    },
    Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupPlayerInput {
    pub player_id: u32,
    pub display_name: String,
    /// Current roster membership at the declared roster cutoff.
    pub team: String,
    /// Previous/statistical team is provenance only and never controls assignment.
    pub prior_team: Option<String>,
    pub primary_position: Position,
    pub eligible_positions: Vec<Position>,
    pub headshot_canonical_url: Option<String>,
    pub games_played: u32,
    pub lens_scores: BTreeMap<TeamCeilingLens, Option<f64>>,
    pub score_evidence: EvidenceLabel,
    /// Prior-season PP deployment/role score. Official PP TOI per game is the
    /// preferred input; translated prospect estimates must remain labeled.
    #[serde(default)]
    pub power_play_role_score: Option<f64>,
    /// Prior-season PK deployment/role score. Official shorthanded TOI per
    /// game is the preferred input.
    #[serde(default)]
    pub penalty_kill_role_score: Option<f64>,
    #[serde(default)]
    pub special_teams_evidence: Option<EvidenceLabel>,
    pub requested_slot: Option<TeamLineupRequestedSlot>,
    pub assignment_evidence: LineupAssignmentEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamLineupPortraitView {
    pub asset_id: String,
    pub headshot_canonical_url: Option<String>,
    pub fallback_initials: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupPlayerView {
    pub player_id: u32,
    pub display_name: String,
    pub team: String,
    pub prior_team: Option<String>,
    pub primary_position: Position,
    pub eligible_positions: Vec<Position>,
    pub portrait: TeamLineupPortraitView,
    pub score: IceLinesPlayerScoreView,
    #[serde(default)]
    pub power_play_role_score: Option<f64>,
    #[serde(default)]
    pub penalty_kill_role_score: Option<f64>,
    #[serde(default)]
    pub special_teams_evidence: Option<EvidenceLabel>,
    pub assignment_evidence: LineupAssignmentEvidence,
    #[serde(skip)]
    requested_slot: Option<TeamLineupRequestedSlot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupForwardLineView {
    pub line: u8,
    pub left_wing: Option<TeamLineupPlayerView>,
    pub center: Option<TeamLineupPlayerView>,
    pub right_wing: Option<TeamLineupPlayerView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupDefensePairView {
    pub pair: u8,
    pub left: Option<TeamLineupPlayerView>,
    pub right: Option<TeamLineupPlayerView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupGoaliesView {
    pub starter: Option<TeamLineupPlayerView>,
    pub backup: Option<TeamLineupPlayerView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamLineupSpecialTeamsKind {
    PowerPlay,
    PenaltyKill,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupSpecialTeamsUnitView {
    pub kind: TeamLineupSpecialTeamsKind,
    pub unit: u8,
    /// Required on each estimated power-play unit; absent on PK units.
    pub quarterback_id: Option<u32>,
    pub player_ids: Vec<u32>,
    pub average_role_score: Option<f64>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupSpecialTeamsView {
    pub power_play: Vec<TeamLineupSpecialTeamsUnitView>,
    pub penalty_kill: Vec<TeamLineupSpecialTeamsUnitView>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamLineupWarningView {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamLineupProjectionView {
    pub schema: String,
    pub score_schema: String,
    pub score_method: String,
    pub team: String,
    pub roster_season: u32,
    pub assignment_evidence: Vec<LineupAssignmentEvidence>,
    pub forward_lines: Vec<TeamLineupForwardLineView>,
    pub defense_pairs: Vec<TeamLineupDefensePairView>,
    pub goalies: TeamLineupGoaliesView,
    #[serde(default)]
    pub special_teams: TeamLineupSpecialTeamsView,
    pub extras: Vec<TeamLineupPlayerView>,
    pub warnings: Vec<TeamLineupWarningView>,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum TeamLineupProjectionError {
    #[error("lineup projection requires at least one player")]
    EmptyRoster,
    #[error("invalid lineup team: {0}")]
    InvalidTeam(String),
    #[error("player {player_id} belongs to {actual}, not roster team {expected}")]
    WrongRosterTeam {
        player_id: u32,
        expected: String,
        actual: String,
    },
    #[error("duplicate lineup player id: {0}")]
    DuplicatePlayer(u32),
    #[error("player {0} has no valid position eligibility")]
    MissingEligibility(u32),
    #[error("player {0} mixes goalie and skater eligibility")]
    MixedGoalieEligibility(u32),
    #[error("player {0} primary position is absent from eligibility")]
    PrimaryPositionNotEligible(u32),
    #[error("headshot URL for player {0} does not contain the stable player id")]
    HeadshotIdentityMismatch(u32),
    #[error("invalid requested lineup slot for player {0}")]
    InvalidRequestedSlot(u32),
    #[error("requested lineup slot is not eligible for player {0}")]
    IneligibleRequestedSlot(u32),
    #[error("requested lineup slot is assigned more than once")]
    DuplicateRequestedSlot,
    #[error("player {0} has an invalid special-teams role score")]
    InvalidSpecialTeamsScore(u32),
}

pub fn build_team_lineup_projection(
    team: &str,
    roster_season: u32,
    players: Vec<TeamLineupPlayerInput>,
) -> Result<TeamLineupProjectionView, TeamLineupProjectionError> {
    if players.is_empty() {
        return Err(TeamLineupProjectionError::EmptyRoster);
    }
    let team = team.trim().to_ascii_uppercase();
    if !valid_team(&team) {
        return Err(TeamLineupProjectionError::InvalidTeam(team));
    }
    validate_players(&team, &players)?;

    let mut views = players.into_iter().map(player_view).collect::<Vec<_>>();
    views.sort_by(player_order);

    let mut forwards: Vec<[Option<TeamLineupPlayerView>; 3]> =
        (0..4).map(|_| [None, None, None]).collect();
    let mut defense: Vec<[Option<TeamLineupPlayerView>; 2]> =
        (0..3).map(|_| [None, None]).collect();
    let mut goalies: [Option<TeamLineupPlayerView>; 2] = [None, None];
    let mut extras = Vec::new();
    let mut remaining = Vec::new();

    for player in views {
        match player.requested_slot() {
            Some(
                TeamLineupRequestedSlot::Forward { line, position }
                | TeamLineupRequestedSlot::FlexibleForward { line, position },
            ) => {
                let target = &mut forwards[(line - 1) as usize][forward_index(position)];
                if target.replace(player).is_some() {
                    return Err(TeamLineupProjectionError::DuplicateRequestedSlot);
                }
            }
            Some(TeamLineupRequestedSlot::Defense { pair, right_side }) => {
                let target = &mut defense[(pair - 1) as usize][usize::from(right_side)];
                if target.replace(player).is_some() {
                    return Err(TeamLineupProjectionError::DuplicateRequestedSlot);
                }
            }
            Some(TeamLineupRequestedSlot::Goalie { starter }) => {
                let target = &mut goalies[usize::from(!starter)];
                if target.replace(player).is_some() {
                    return Err(TeamLineupProjectionError::DuplicateRequestedSlot);
                }
            }
            Some(TeamLineupRequestedSlot::Extra) => extras.push(player),
            None => remaining.push(player),
        }
    }

    for player in remaining {
        if player.primary_position == Position::Goalie {
            if let Some(target) = goalies.iter_mut().find(|slot| slot.is_none()) {
                *target = Some(player.with_estimated_assignment());
            } else {
                extras.push(player.with_estimated_assignment());
            }
        } else if player.primary_position == Position::Defense {
            if let Some(target) = defense
                .iter_mut()
                .flat_map(|pair| pair.iter_mut())
                .find(|slot| slot.is_none())
            {
                *target = Some(player.with_estimated_assignment());
            } else {
                extras.push(player.with_estimated_assignment());
            }
        } else if let Some((line, position)) = best_forward_slot(&forwards, &player) {
            forwards[line][position] = Some(player.with_estimated_assignment());
        } else {
            extras.push(player.with_estimated_assignment());
        }
    }

    extras.sort_by(player_order);
    let missing_forwards = forwards
        .iter()
        .flatten()
        .filter(|slot| slot.is_none())
        .count();
    let missing_defense = defense
        .iter()
        .flatten()
        .filter(|slot| slot.is_none())
        .count();
    let missing_goalies = goalies.iter().filter(|slot| slot.is_none()).count();
    let mut warnings = Vec::new();
    if missing_forwards + missing_defense + missing_goalies > 0 {
        warnings.push(TeamLineupWarningView {
            code: "incomplete_roster_shape".to_string(),
            message: format!(
                "Projected shape is missing {missing_forwards} forward, {missing_defense} defense, and {missing_goalies} goalie slots."
            ),
        });
    }
    let assigned_players = forwards
        .iter()
        .flatten()
        .chain(defense.iter().flatten())
        .chain(goalies.iter())
        .filter_map(Option::as_ref)
        .chain(extras.iter());
    let assignment_evidence = assigned_players
        .clone()
        .map(|player| player.assignment_evidence)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let unrated = assigned_players
        .filter(|player| player.score.value.is_none())
        .count();
    if unrated > 0 {
        warnings.push(TeamLineupWarningView {
            code: "unrated_players".to_string(),
            message: format!("{unrated} players have no qualifying NHL sample and display NR."),
        });
    }

    let mut projection = TeamLineupProjectionView {
        schema: TEAM_LINEUP_PROJECTION_SCHEMA.to_string(),
        score_schema: ICELINES_PLAYER_SCORE_SCHEMA.to_string(),
        score_method: ICELINES_PLAYER_SCORE_METHOD.to_string(),
        team,
        roster_season,
        assignment_evidence,
        forward_lines: forwards
            .into_iter()
            .enumerate()
            .map(
                |(index, [left_wing, center, right_wing])| TeamLineupForwardLineView {
                    line: (index + 1) as u8,
                    left_wing,
                    center,
                    right_wing,
                },
            )
            .collect(),
        defense_pairs: defense
            .into_iter()
            .enumerate()
            .map(|(index, [left, right])| TeamLineupDefensePairView {
                pair: (index + 1) as u8,
                left,
                right,
            })
            .collect(),
        goalies: TeamLineupGoaliesView {
            starter: goalies[0].take(),
            backup: goalies[1].take(),
        },
        special_teams: TeamLineupSpecialTeamsView::default(),
        extras,
        warnings,
    };
    projection.special_teams = build_special_teams(&projection);
    Ok(projection)
}

pub fn team_lineup_card_section(view: &TeamLineupProjectionView) -> LineupSectionView {
    let mut groups = view
        .forward_lines
        .iter()
        .map(|line| CardLineupGroupView {
            id: format!("forward-line-{}", line.line),
            label: format!("Line {}", line.line),
            kind: CardLineupGroupKind::ForwardLine,
            slots: vec![
                card_slot(
                    &format!("line-{}-lw", line.line),
                    "LW",
                    line.left_wing.as_ref(),
                ),
                card_slot(&format!("line-{}-c", line.line), "C", line.center.as_ref()),
                card_slot(
                    &format!("line-{}-rw", line.line),
                    "RW",
                    line.right_wing.as_ref(),
                ),
            ],
        })
        .collect::<Vec<_>>();
    groups.extend(view.defense_pairs.iter().map(|pair| CardLineupGroupView {
        id: format!("defense-pair-{}", pair.pair),
        label: format!("Pair {}", pair.pair),
        kind: CardLineupGroupKind::DefensePair,
        slots: vec![
            card_slot(
                &format!("pair-{}-left", pair.pair),
                "LD",
                pair.left.as_ref(),
            ),
            card_slot(
                &format!("pair-{}-right", pair.pair),
                "RD",
                pair.right.as_ref(),
            ),
        ],
    }));
    groups.push(CardLineupGroupView {
        id: "goalies".to_string(),
        label: "Goalies".to_string(),
        kind: CardLineupGroupKind::Goalies,
        slots: vec![
            card_slot("goalie-starter", "Starter", view.goalies.starter.as_ref()),
            card_slot("goalie-backup", "Backup", view.goalies.backup.as_ref()),
        ],
    });
    if !view.extras.is_empty() {
        groups.push(CardLineupGroupView {
            id: "extras".to_string(),
            label: "Extras".to_string(),
            kind: CardLineupGroupKind::Extras,
            slots: view
                .extras
                .iter()
                .enumerate()
                .map(|(index, player)| {
                    card_slot(&format!("extra-{}", index + 1), "Extra", Some(player))
                })
                .collect(),
        });
    }
    LineupSectionView {
        id: "projected-lineup".to_string(),
        title: "Projected team lineup and IceLines player scores".to_string(),
        groups,
    }
}

fn build_special_teams(view: &TeamLineupProjectionView) -> TeamLineupSpecialTeamsView {
    let mut forwards = view
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .flatten()
        .collect::<Vec<_>>();
    let mut defense = view
        .defense_pairs
        .iter()
        .flat_map(|pair| [&pair.left, &pair.right])
        .flatten()
        .collect::<Vec<_>>();
    let rank = |players: &mut Vec<&TeamLineupPlayerView>, power_play: bool| {
        players.sort_by(|a, b| {
            special_teams_effective_score(b, power_play)
                .unwrap_or(-1.0)
                .total_cmp(&special_teams_effective_score(a, power_play).unwrap_or(-1.0))
                .then_with(|| b.score.sample_games.cmp(&a.score.sample_games))
                .then_with(|| a.player_id.cmp(&b.player_id))
        });
    };

    rank(&mut forwards, true);
    rank(&mut defense, true);
    let pp_forwards = forwards
        .iter()
        .copied()
        .filter(|player| {
            player
                .power_play_role_score
                .is_some_and(|score| score > 0.0)
        })
        .collect::<Vec<_>>();
    let pp_defense = defense
        .iter()
        .copied()
        .filter(|player| {
            player
                .power_play_role_score
                .is_some_and(|score| score > 0.0)
        })
        .collect::<Vec<_>>();
    let mut power_play = Vec::new();
    for unit_index in 0..2 {
        let quarterback = pp_defense.get(unit_index).copied();
        let unit_forwards = pp_forwards
            .iter()
            .skip(unit_index * 4)
            .take(4)
            .copied()
            .collect::<Vec<_>>();
        let mut players = quarterback.into_iter().collect::<Vec<_>>();
        players.extend(unit_forwards);
        power_play.push(special_teams_unit(
            TeamLineupSpecialTeamsKind::PowerPlay,
            (unit_index + 1) as u8,
            quarterback.map(|player| player.player_id),
            players,
            true,
        ));
    }

    rank(&mut forwards, false);
    rank(&mut defense, false);
    let pk_forwards = forwards
        .iter()
        .copied()
        .filter(|player| {
            player
                .penalty_kill_role_score
                .is_some_and(|score| score > 0.0)
        })
        .collect::<Vec<_>>();
    let pk_defense = defense
        .iter()
        .copied()
        .filter(|player| {
            player
                .penalty_kill_role_score
                .is_some_and(|score| score > 0.0)
        })
        .collect::<Vec<_>>();
    let mut penalty_kill = Vec::new();
    for unit_index in 0..2 {
        let mut players = pk_defense
            .iter()
            .skip(unit_index * 2)
            .take(2)
            .copied()
            .collect::<Vec<_>>();
        players.extend(pk_forwards.iter().skip(unit_index * 2).take(2).copied());
        penalty_kill.push(special_teams_unit(
            TeamLineupSpecialTeamsKind::PenaltyKill,
            (unit_index + 1) as u8,
            None,
            players,
            false,
        ));
    }

    let mut warnings = Vec::new();
    for unit in &power_play {
        if unit.quarterback_id.is_none() || unit.player_ids.len() != 5 {
            warnings.push(format!(
                "PP{} lacks enough rated deployment evidence for one quarterback and four forwards.",
                unit.unit
            ));
        }
    }
    for unit in &penalty_kill {
        if unit.player_ids.len() != 4 {
            warnings.push(format!(
                "PK{} lacks enough rated shorthanded deployment evidence for two defensemen and two forwards.",
                unit.unit
            ));
        }
    }
    TeamLineupSpecialTeamsView {
        power_play,
        penalty_kill,
        warnings,
    }
}

pub(crate) fn rebuild_special_teams(view: &mut TeamLineupProjectionView) {
    view.special_teams = build_special_teams(view);
}

fn special_teams_unit(
    kind: TeamLineupSpecialTeamsKind,
    unit: u8,
    quarterback_id: Option<u32>,
    players: Vec<&TeamLineupPlayerView>,
    power_play: bool,
) -> TeamLineupSpecialTeamsUnitView {
    let scores = players
        .iter()
        .filter_map(|player| special_teams_effective_score(player, power_play))
        .collect::<Vec<_>>();
    TeamLineupSpecialTeamsUnitView {
        kind,
        unit,
        quarterback_id,
        player_ids: players.iter().map(|player| player.player_id).collect(),
        average_role_score: (!scores.is_empty())
            .then(|| scores.iter().sum::<f64>() / scores.len() as f64),
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn special_teams_effective_score(player: &TeamLineupPlayerView, power_play: bool) -> Option<f64> {
    let raw = if power_play {
        player.power_play_role_score
    } else {
        player.penalty_kill_role_score
    }?;
    let games = f64::from(player.score.sample_games);
    Some(raw * games / (games + 20.0))
}

pub fn team_lineup_card_assets(view: &TeamLineupProjectionView) -> Vec<CardAssetView> {
    lineup_players(view)
        .map(|player| {
            let reference = player
                .portrait
                .headshot_canonical_url
                .clone()
                .map(CardAssetReference::ExternalUrl);
            CardAssetView {
                id: player.portrait.asset_id.clone(),
                subject_id: format!("player:{}", player.player_id),
                kind: CardAssetKind::Headshot,
                state: if reference.is_some() {
                    CardAssetState::Available
                } else {
                    CardAssetState::Missing
                },
                reference,
                source: SourceKind::Roster,
                observed_at: None,
                integrity_sha256: None,
                alt: format!("{} headshot", player.display_name),
                fallback: CardAssetFallback::Initials(player.portrait.fallback_initials.clone()),
            }
        })
        .collect()
}

fn card_slot(id: &str, label: &str, player: Option<&TeamLineupPlayerView>) -> CardLineupSlotView {
    CardLineupSlotView {
        id: id.to_string(),
        label: label.to_string(),
        subject_id: player.map(|player| format!("player:{}", player.player_id)),
        subject_label: player.map(|player| player.display_name.clone()),
        asset_id: player.map(|player| player.portrait.asset_id.clone()),
        metrics: player.map_or_else(Vec::new, |player| vec![score_card_metric(player)]),
        evidence_label: player.map_or(EvidenceLabel::NoRead, |player| {
            player.assignment_evidence.card_evidence_label()
        }),
    }
}

fn score_card_metric(player: &TeamLineupPlayerView) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey::from("icelines_player_score"),
            label: "IceLines score".to_string(),
            value: player
                .score
                .value
                .map(MetricValue::Decimal)
                .unwrap_or(MetricValue::Missing),
            unit: MetricUnit::Score,
            precision: ValuePrecision::Integer,
            token: None,
        },
        display_text: player.score.display.clone(),
        accessible_text: player.score.value.map_or_else(
            || format!("{} is not rated", player.display_name),
            |score| {
                format!(
                    "{} IceLines player score {:.0} out of 100",
                    player.display_name, score
                )
            },
        ),
        comparison: None,
        evidence_label: player.score.evidence_label,
    }
}

fn lineup_players(view: &TeamLineupProjectionView) -> impl Iterator<Item = &TeamLineupPlayerView> {
    view.forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            view.defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .chain([&view.goalies.starter, &view.goalies.backup])
        .filter_map(Option::as_ref)
        .chain(view.extras.iter())
}

impl TeamLineupPlayerView {
    fn requested_slot(&self) -> Option<TeamLineupRequestedSlot> {
        self.requested_slot
    }

    fn with_estimated_assignment(mut self) -> Self {
        self.assignment_evidence = LineupAssignmentEvidence::Estimated;
        self
    }
}

fn player_view(input: TeamLineupPlayerInput) -> TeamLineupPlayerView {
    TeamLineupPlayerView {
        player_id: input.player_id,
        display_name: input.display_name.clone(),
        team: input.team.trim().to_ascii_uppercase(),
        prior_team: input.prior_team,
        primary_position: input.primary_position,
        eligible_positions: canonical_positions(input.eligible_positions),
        portrait: TeamLineupPortraitView {
            asset_id: format!("player:{}:headshot", input.player_id),
            headshot_canonical_url: input.headshot_canonical_url,
            fallback_initials: initials(&input.display_name),
        },
        score: player_score(
            input.primary_position,
            input.games_played,
            &input.lens_scores,
            input.score_evidence,
        ),
        power_play_role_score: input.power_play_role_score,
        penalty_kill_role_score: input.penalty_kill_role_score,
        special_teams_evidence: input.special_teams_evidence,
        assignment_evidence: input.assignment_evidence,
        requested_slot: input.requested_slot,
    }
}

fn player_score(
    position: Position,
    games_played: u32,
    lenses: &BTreeMap<TeamCeilingLens, Option<f64>>,
    evidence_label: EvidenceLabel,
) -> IceLinesPlayerScoreView {
    let position_group = if position == Position::Goalie {
        PlayerScorePositionGroup::Goalie
    } else if position == Position::Defense {
        PlayerScorePositionGroup::Defense
    } else {
        PlayerScorePositionGroup::Forward
    };
    let specs: &[(TeamCeilingLens, f64, f64)] = match position_group {
        PlayerScorePositionGroup::Forward => &[
            (TeamCeilingLens::PointsPace, 120.0, 0.40),
            (TeamCeilingLens::GoalScoring, 60.0, 0.20),
            (TeamCeilingLens::Fantasy, 450.0, 0.25),
            (TeamCeilingLens::Upside, 120.0, 0.15),
        ],
        PlayerScorePositionGroup::Defense => &[
            (TeamCeilingLens::PointsPace, 90.0, 0.30),
            (TeamCeilingLens::GoalScoring, 30.0, 0.10),
            (TeamCeilingLens::Fantasy, 400.0, 0.40),
            (TeamCeilingLens::Upside, 100.0, 0.20),
        ],
        PlayerScorePositionGroup::Goalie => &[(TeamCeilingLens::PointsPace, 100.0, 1.0)],
    };
    let components = specs
        .iter()
        .map(|(lens, ceiling, weight)| {
            let raw_value = if position_group == PlayerScorePositionGroup::Goalie {
                lenses
                    .get(&TeamCeilingLens::PointsPace)
                    .copied()
                    .flatten()
                    .or_else(|| lenses.values().copied().flatten().next())
            } else {
                lenses.get(lens).copied().flatten()
            };
            IceLinesPlayerScoreComponent {
                lens: *lens,
                label: if position_group == PlayerScorePositionGroup::Goalie {
                    "Goalie quality".to_string()
                } else {
                    lens.label().to_string()
                },
                raw_value,
                normalized_value: raw_value
                    .map(|value| (value / ceiling * 100.0).clamp(0.0, 100.0)),
                weight: *weight,
            }
        })
        .collect::<Vec<_>>();
    let has_primary_read = games_played > 0
        && components
            .first()
            .and_then(|component| component.normalized_value)
            .is_some();
    let value = has_primary_read.then(|| {
        let (weighted, weights) = components.iter().fold((0.0, 0.0), |acc, component| {
            component.normalized_value.map_or(acc, |value| {
                (acc.0 + value * component.weight, acc.1 + component.weight)
            })
        });
        if weights == 0.0 {
            0.0
        } else {
            ((weighted / weights) * 10.0).round() / 10.0
        }
    });
    let coverage_pct = components
        .iter()
        .filter(|component| component.raw_value.is_some())
        .count() as f64
        / components.len() as f64
        * 100.0;
    IceLinesPlayerScoreView {
        schema: ICELINES_PLAYER_SCORE_SCHEMA.to_string(),
        method: ICELINES_PLAYER_SCORE_METHOD.to_string(),
        position_group,
        value,
        display: value.map_or_else(|| "NR".to_string(), |score| format!("{score:.0}")),
        sample_games: games_played,
        coverage_pct,
        evidence_label,
        components,
    }
}

fn validate_players(
    team: &str,
    players: &[TeamLineupPlayerInput],
) -> Result<(), TeamLineupProjectionError> {
    let mut ids = BTreeSet::new();
    let mut slots = std::collections::HashSet::new();
    for player in players {
        if !ids.insert(player.player_id) {
            return Err(TeamLineupProjectionError::DuplicatePlayer(player.player_id));
        }
        let actual = player.team.trim().to_ascii_uppercase();
        if actual != team {
            return Err(TeamLineupProjectionError::WrongRosterTeam {
                player_id: player.player_id,
                expected: team.to_string(),
                actual,
            });
        }
        if player.eligible_positions.is_empty() {
            return Err(TeamLineupProjectionError::MissingEligibility(
                player.player_id,
            ));
        }
        let eligible = canonical_positions(player.eligible_positions.clone());
        if !eligible.contains(&player.primary_position) {
            return Err(TeamLineupProjectionError::PrimaryPositionNotEligible(
                player.player_id,
            ));
        }
        let has_goalie = eligible.contains(&Position::Goalie);
        if has_goalie && eligible.len() != 1 {
            return Err(TeamLineupProjectionError::MixedGoalieEligibility(
                player.player_id,
            ));
        }
        if player
            .headshot_canonical_url
            .as_ref()
            .is_some_and(|url| !url.contains(&player.player_id.to_string()))
        {
            return Err(TeamLineupProjectionError::HeadshotIdentityMismatch(
                player.player_id,
            ));
        }
        if [player.power_play_role_score, player.penalty_kill_role_score]
            .into_iter()
            .flatten()
            .any(|score| !score.is_finite() || score < 0.0)
        {
            return Err(TeamLineupProjectionError::InvalidSpecialTeamsScore(
                player.player_id,
            ));
        }
        if let Some(slot) = player.requested_slot {
            validate_requested_slot(player, slot)?;
            if slot != TeamLineupRequestedSlot::Extra && !slots.insert(slot) {
                return Err(TeamLineupProjectionError::DuplicateRequestedSlot);
            }
        }
    }
    Ok(())
}

fn validate_requested_slot(
    player: &TeamLineupPlayerInput,
    slot: TeamLineupRequestedSlot,
) -> Result<(), TeamLineupProjectionError> {
    let valid = match slot {
        TeamLineupRequestedSlot::Forward { line, position } => {
            (1..=4).contains(&line) && player.eligible_positions.contains(&position.position())
        }
        TeamLineupRequestedSlot::FlexibleForward { line, .. } => {
            (1..=4).contains(&line) && player.primary_position.is_forward()
        }
        TeamLineupRequestedSlot::Defense { pair, .. } => {
            (1..=3).contains(&pair) && player.eligible_positions.contains(&Position::Defense)
        }
        TeamLineupRequestedSlot::Goalie { .. } => {
            player.eligible_positions.contains(&Position::Goalie)
        }
        TeamLineupRequestedSlot::Extra => true,
    };
    if valid {
        Ok(())
    } else if match slot {
        TeamLineupRequestedSlot::Forward { line, .. }
        | TeamLineupRequestedSlot::FlexibleForward { line, .. } => !(1..=4).contains(&line),
        TeamLineupRequestedSlot::Defense { pair, .. } => !(1..=3).contains(&pair),
        _ => false,
    } {
        Err(TeamLineupProjectionError::InvalidRequestedSlot(
            player.player_id,
        ))
    } else {
        Err(TeamLineupProjectionError::IneligibleRequestedSlot(
            player.player_id,
        ))
    }
}

fn best_forward_slot(
    forwards: &[[Option<TeamLineupPlayerView>; 3]],
    player: &TeamLineupPlayerView,
) -> Option<(usize, usize)> {
    let preferred = forward_position(player.primary_position);
    let mut eligible = player
        .eligible_positions
        .iter()
        .filter_map(|position| forward_position(*position))
        .collect::<Vec<_>>();
    eligible.sort_by_key(|position| {
        let index = forward_index(*position);
        let filled = forwards.iter().filter(|line| line[index].is_some()).count();
        (filled, usize::from(Some(*position) != preferred), index)
    });
    eligible.into_iter().find_map(|position| {
        let index = forward_index(position);
        forwards
            .iter()
            .position(|line| line[index].is_none())
            .map(|line| (line, index))
    })
}

fn forward_position(position: Position) -> Option<LineupForwardPosition> {
    match position {
        Position::LeftWing => Some(LineupForwardPosition::LeftWing),
        Position::Center => Some(LineupForwardPosition::Center),
        Position::RightWing => Some(LineupForwardPosition::RightWing),
        _ => None,
    }
}

fn forward_index(position: LineupForwardPosition) -> usize {
    match position {
        LineupForwardPosition::LeftWing => 0,
        LineupForwardPosition::Center => 1,
        LineupForwardPosition::RightWing => 2,
    }
}

fn canonical_positions(mut positions: Vec<Position>) -> Vec<Position> {
    positions.sort_by_key(|position| match position {
        Position::LeftWing => 0,
        Position::Center => 1,
        Position::RightWing => 2,
        Position::Defense => 3,
        Position::Goalie => 4,
    });
    positions.dedup();
    positions
}

fn player_order(a: &TeamLineupPlayerView, b: &TeamLineupPlayerView) -> std::cmp::Ordering {
    b.score
        .value
        .unwrap_or(-1.0)
        .total_cmp(&a.score.value.unwrap_or(-1.0))
        .then_with(|| a.display_name.cmp(&b.display_name))
        .then_with(|| a.player_id.cmp(&b.player_id))
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|part| part.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

fn valid_team(team: &str) -> bool {
    team.len() == 3
        && team
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() && byte.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(id: u32, name: &str, position: Position) -> TeamLineupPlayerInput {
        TeamLineupPlayerInput {
            player_id: id,
            display_name: name.to_string(),
            team: "NYR".to_string(),
            prior_team: None,
            primary_position: position,
            eligible_positions: vec![position],
            headshot_canonical_url: Some(format!(
                "https://assets.nhle.com/mugs/nhl/20262027/NYR/{id}.png"
            )),
            games_played: 82,
            lens_scores: BTreeMap::from([
                (TeamCeilingLens::PointsPace, Some(70.0)),
                (TeamCeilingLens::GoalScoring, Some(30.0)),
                (TeamCeilingLens::Fantasy, Some(250.0)),
                (TeamCeilingLens::Upside, Some(75.0)),
            ]),
            score_evidence: EvidenceLabel::Confirmed,
            power_play_role_score: Some(if position == Position::Defense {
                90.0
            } else {
                60.0
            }),
            penalty_kill_role_score: Some(if position == Position::Defense {
                80.0
            } else {
                50.0
            }),
            special_teams_evidence: Some(EvidenceLabel::Confirmed),
            requested_slot: None,
            assignment_evidence: LineupAssignmentEvidence::Estimated,
        }
    }

    #[test]
    fn projects_complete_shape_preserves_multi_position_and_faces() {
        let mut players = Vec::new();
        for index in 0..12 {
            let position = [Position::LeftWing, Position::Center, Position::RightWing][index % 3];
            players.push(player(
                100 + index as u32,
                &format!("Forward {index}"),
                position,
            ));
        }
        players[0].display_name = "Alexis Lafrenière".to_string();
        players[0].eligible_positions = vec![Position::LeftWing, Position::RightWing];
        for index in 0..6 {
            players.push(player(
                200 + index,
                &format!("Defense {index}"),
                Position::Defense,
            ));
        }
        for index in 0..3 {
            let mut goalie = player(300 + index, &format!("Goalie {index}"), Position::Goalie);
            goalie.lens_scores =
                BTreeMap::from([(TeamCeilingLens::PointsPace, Some(80.0 - index as f64))]);
            players.push(goalie);
        }

        let view = build_team_lineup_projection("NYR", 20262027, players).unwrap();
        assert_eq!(view.forward_lines.len(), 4);
        assert_eq!(view.defense_pairs.len(), 3);
        assert!(view.goalies.starter.is_some());
        assert!(view.goalies.backup.is_some());
        assert_eq!(view.extras.len(), 1);
        let lafreniere = view.forward_lines[0].left_wing.as_ref().unwrap();
        assert_eq!(lafreniere.display_name, "Alexis Lafrenière");
        assert_eq!(lafreniere.eligible_positions.len(), 2);
        assert_eq!(lafreniere.portrait.fallback_initials, "AL");
        assert!(lafreniere.portrait.headshot_canonical_url.is_some());
        assert!(view.warnings.is_empty());
        assert_eq!(view.special_teams.power_play.len(), 2);
        assert_eq!(view.special_teams.penalty_kill.len(), 2);
        assert!(view.special_teams.warnings.is_empty());
        assert!(view
            .special_teams
            .power_play
            .iter()
            .all(|unit| { unit.quarterback_id.is_some() && unit.player_ids.len() == 5 }));
        assert!(view
            .special_teams
            .penalty_kill
            .iter()
            .all(|unit| unit.quarterback_id.is_none() && unit.player_ids.len() == 4));
        let section = team_lineup_card_section(&view);
        assert_eq!(section.groups.len(), 9);
        let assets = team_lineup_card_assets(&view);
        assert_eq!(assets.len(), 21);
        assert!(assets
            .iter()
            .all(|asset| asset.subject_id.starts_with("player:")));
    }

    #[test]
    fn requested_scenario_assignment_and_missing_score_are_explicit() {
        let mut prospect = player(8480001, "Noah Laba", Position::Center);
        prospect.games_played = 0;
        prospect.lens_scores = BTreeMap::new();
        prospect.requested_slot = Some(TeamLineupRequestedSlot::Forward {
            line: 2,
            position: LineupForwardPosition::Center,
        });
        prospect.assignment_evidence = LineupAssignmentEvidence::Scenario;
        let view = build_team_lineup_projection("NYR", 20262027, vec![prospect]).unwrap();
        let prospect = view.forward_lines[1].center.as_ref().unwrap();
        assert_eq!(prospect.score.value, None);
        assert_eq!(prospect.score.display, "NR");
        assert_eq!(
            prospect.assignment_evidence,
            LineupAssignmentEvidence::Scenario
        );
        assert_eq!(
            prospect.assignment_evidence.card_evidence_label(),
            EvidenceLabel::Simulated
        );
        assert!(view
            .warnings
            .iter()
            .any(|warning| warning.code == "unrated_players"));
    }

    #[test]
    fn flexible_forward_replays_an_off_natural_side_assignment() {
        let mut center = player(8480002, "Flexible Center", Position::Center);
        center.requested_slot = Some(TeamLineupRequestedSlot::FlexibleForward {
            line: 1,
            position: LineupForwardPosition::LeftWing,
        });
        center.assignment_evidence = LineupAssignmentEvidence::Scenario;

        let view = build_team_lineup_projection("NYR", 20262027, vec![center]).unwrap();
        let assigned = view.forward_lines[0].left_wing.as_ref().unwrap();
        assert_eq!(assigned.player_id, 8480002);
        assert_eq!(assigned.primary_position, Position::Center);
        assert_eq!(assigned.eligible_positions, vec![Position::Center]);
        assert_eq!(
            assigned.assignment_evidence,
            LineupAssignmentEvidence::Scenario
        );
    }

    #[test]
    fn refuses_duplicate_wrong_team_goalie_mix_and_headshot_mismatch() {
        let first = player(1, "One Player", Position::Center);
        assert!(matches!(
            build_team_lineup_projection("NYR", 20262027, vec![first.clone(), first]),
            Err(TeamLineupProjectionError::DuplicatePlayer(1))
        ));
        let mut wrong_team = player(2, "Two Player", Position::Center);
        wrong_team.team = "SEA".to_string();
        assert!(matches!(
            build_team_lineup_projection("NYR", 20262027, vec![wrong_team]),
            Err(TeamLineupProjectionError::WrongRosterTeam { .. })
        ));
        let mut mixed = player(3, "Three Player", Position::Goalie);
        mixed.eligible_positions.push(Position::Center);
        assert!(matches!(
            build_team_lineup_projection("NYR", 20262027, vec![mixed]),
            Err(TeamLineupProjectionError::MixedGoalieEligibility(3))
        ));
        let mut bad_face = player(4, "Four Player", Position::Defense);
        bad_face.headshot_canonical_url = Some("https://assets.nhle.com/999.png".to_string());
        assert!(matches!(
            build_team_lineup_projection("NYR", 20262027, vec![bad_face]),
            Err(TeamLineupProjectionError::HeadshotIdentityMismatch(4))
        ));

        let mut bad_deployment = player(5, "Five Player", Position::Center);
        bad_deployment.power_play_role_score = Some(-1.0);
        assert!(matches!(
            build_team_lineup_projection("NYR", 20262027, vec![bad_deployment]),
            Err(TeamLineupProjectionError::InvalidSpecialTeamsScore(5))
        ));
    }

    #[test]
    fn special_teams_do_not_invent_usage_when_evidence_is_missing() {
        let mut forward = player(1, "Rated Forward", Position::Center);
        forward.power_play_role_score = None;
        forward.penalty_kill_role_score = None;
        forward.special_teams_evidence = None;
        let mut defense = player(2, "Rated Defense", Position::Defense);
        defense.power_play_role_score = None;
        defense.penalty_kill_role_score = None;
        defense.special_teams_evidence = None;

        let view = build_team_lineup_projection("NYR", 20262027, vec![forward, defense]).unwrap();

        assert!(view
            .special_teams
            .power_play
            .iter()
            .all(|unit| unit.player_ids.is_empty() && unit.quarterback_id.is_none()));
        assert!(view
            .special_teams
            .penalty_kill
            .iter()
            .all(|unit| unit.player_ids.is_empty()));
        assert_eq!(view.special_teams.warnings.len(), 4);
    }
}
