//! UI-neutral NHL/AHL organization lineup and recall forecast.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::ahl_affiliate::{
    current_ahl_affiliation_catalog, AhlAffiliatePlayerView, AhlAffiliateProjectionView,
    AhlLineUnitKind, AhlRosterPoolAuthority, AHL_AFFILIATE_PROJECTION_SCHEMA,
    CURRENT_AHL_AFFILIATION_SEASON,
};
use super::team_lineup::{
    TeamLineupPlayerView, TeamLineupProjectionView, TeamLineupSpecialTeamsView,
    TEAM_LINEUP_PROJECTION_SCHEMA,
};

pub const ORGANIZATION_LINEUP_FORECAST_SCHEMA: &str = "organization_lineup_forecast.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationLineupForecastInput {
    pub nhl_lineup: TeamLineupProjectionView,
    pub ahl_affiliate: AhlAffiliateProjectionView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationLevel {
    Nhl,
    Ahl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationUnitKind {
    ForwardLine,
    DefensePair,
    Goalies,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationUnitView {
    pub level: OrganizationLevel,
    pub team: String,
    pub kind: OrganizationUnitKind,
    pub unit: u8,
    pub label: String,
    pub player_ids: Vec<u32>,
    pub player_names: Vec<String>,
    pub average_score: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationPositionGroup {
    Forward,
    Defense,
    Goalie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationRecallCandidateView {
    pub position_group: OrganizationPositionGroup,
    pub rank: usize,
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    pub projected_score: f64,
    pub recall_readiness: Option<f64>,
    pub dressed_for_affiliate: bool,
    pub affiliate_assignment: Option<String>,
    pub waiver_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationRecallPlanView {
    pub position_group: OrganizationPositionGroup,
    pub first_recall_player_id: Option<u32>,
    pub first_recall_name: Option<String>,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationBlockedPlayerView {
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    pub projected_score: f64,
    pub blocked_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationLineupCountsView {
    pub forward_lines: usize,
    pub defense_pairs: usize,
    pub goalies: usize,
    pub nhl_extras: usize,
    pub ahl_assigned_not_dressed: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationLineupForecastView {
    pub schema: String,
    pub season: u32,
    pub nhl_team: String,
    pub ahl_team: String,
    pub counts: OrganizationLineupCountsView,
    pub units: Vec<OrganizationUnitView>,
    pub recall_ladder: Vec<OrganizationRecallCandidateView>,
    pub recall_plan: Vec<OrganizationRecallPlanView>,
    pub blocked_players: Vec<OrganizationBlockedPlayerView>,
    pub nhl_special_teams: TeamLineupSpecialTeamsView,
    pub ahl_development_rule_compliant: bool,
    #[serde(default)]
    pub ahl_pool_authority: AhlRosterPoolAuthority,
    pub nhl_lineup: TeamLineupProjectionView,
    pub ahl_affiliate: AhlAffiliateProjectionView,
    pub disclosures: Vec<String>,
}

pub fn build_organization_lineup_forecast(
    input: &OrganizationLineupForecastInput,
) -> Result<OrganizationLineupForecastView, String> {
    validate(input)?;

    let mut units = nhl_units(&input.nhl_lineup);
    units.extend(ahl_units(&input.ahl_affiliate)?);

    let mut recall_ladder = input
        .ahl_affiliate
        .players
        .iter()
        .filter(|player| player.assigned_to_affiliate)
        .map(recall_candidate)
        .collect::<Vec<_>>();
    recall_ladder.sort_by(|a, b| {
        a.position_group
            .cmp(&b.position_group)
            .then_with(|| {
                b.recall_readiness
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.recall_readiness.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| b.projected_score.total_cmp(&a.projected_score))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    let mut ranks = BTreeMap::<OrganizationPositionGroup, usize>::new();
    for candidate in &mut recall_ladder {
        let rank = ranks.entry(candidate.position_group).or_default();
        *rank += 1;
        candidate.rank = *rank;
    }

    let recall_plan = [
        OrganizationPositionGroup::Forward,
        OrganizationPositionGroup::Defense,
        OrganizationPositionGroup::Goalie,
    ]
    .into_iter()
    .map(|position_group| {
        let candidates = recall_ladder
            .iter()
            .filter(|row| row.position_group == position_group)
            .collect::<Vec<_>>();
        OrganizationRecallPlanView {
            position_group,
            first_recall_player_id: candidates.first().map(|row| row.player_id),
            first_recall_name: candidates.first().map(|row| row.display_name.clone()),
            candidate_count: candidates.len(),
        }
    })
    .collect();

    let mut blocked_players = input
        .ahl_affiliate
        .players
        .iter()
        .filter(|player| player.assigned_to_affiliate && !player.dressed)
        .map(|player| OrganizationBlockedPlayerView {
            player_id: player.player_id,
            display_name: player.display_name.clone(),
            primary_position: player.primary_position,
            projected_score: player.projected_score,
            blocked_reason: player
                .blocked_reason
                .clone()
                .unwrap_or_else(|| "lineup_competition".to_owned()),
        })
        .collect::<Vec<_>>();
    blocked_players.sort_by(|a, b| {
        b.projected_score
            .total_cmp(&a.projected_score)
            .then_with(|| a.display_name.cmp(&b.display_name))
    });

    Ok(OrganizationLineupForecastView {
        schema: ORGANIZATION_LINEUP_FORECAST_SCHEMA.to_owned(),
        season: input.nhl_lineup.roster_season,
        nhl_team: input.nhl_lineup.team.clone(),
        ahl_team: input.ahl_affiliate.ahl_team.clone(),
        counts: OrganizationLineupCountsView {
            forward_lines: units
                .iter()
                .filter(|unit| unit.kind == OrganizationUnitKind::ForwardLine)
                .count(),
            defense_pairs: units
                .iter()
                .filter(|unit| unit.kind == OrganizationUnitKind::DefensePair)
                .count(),
            goalies: units
                .iter()
                .filter(|unit| unit.kind == OrganizationUnitKind::Goalies)
                .map(|unit| unit.player_ids.len())
                .sum(),
            nhl_extras: input.nhl_lineup.extras.len(),
            ahl_assigned_not_dressed: blocked_players.len(),
        },
        units,
        recall_ladder,
        recall_plan,
        blocked_players,
        nhl_special_teams: input.nhl_lineup.special_teams.clone(),
        ahl_development_rule_compliant: input.ahl_affiliate.development_rule_compliant,
        ahl_pool_authority: input.ahl_affiliate.pool_authority.clone(),
        nhl_lineup: input.nhl_lineup.clone(),
        ahl_affiliate: input.ahl_affiliate.clone(),
        disclosures: vec![
            "The System contains four NHL and four AHL forward lines, three defense pairs at each level, and two goalies at each level.".to_owned(),
            "Recall order ranks explicit recall-readiness evidence before projected player score and never treats AHL development-rule eligibility as automatic NHL readiness.".to_owned(),
            "NHL and AHL assignments are mutually exclusive for dressed and affiliate-assigned players; duplicate organizational identity fails closed.".to_owned(),
            "NHL special teams are preserved from the lineup projection. AHL PP/PK forecasting remains unavailable until affiliate role scores and evidence are supplied.".to_owned(),
            "Waivers, cap space, contract consent, emergency-recall rules, and injury status remain transaction-time gates even when a player leads the recall ladder.".to_owned(),
        ],
    })
}

fn validate(input: &OrganizationLineupForecastInput) -> Result<(), String> {
    let nhl = &input.nhl_lineup;
    let ahl = &input.ahl_affiliate;
    if nhl.schema != TEAM_LINEUP_PROJECTION_SCHEMA {
        return Err(format!("unsupported NHL lineup schema {}", nhl.schema));
    }
    if ahl.schema != AHL_AFFILIATE_PROJECTION_SCHEMA {
        return Err(format!("unsupported AHL affiliate schema {}", ahl.schema));
    }
    if nhl.team != ahl.nhl_team {
        return Err(format!(
            "NHL lineup team {} does not match affiliate parent {}",
            nhl.team, ahl.nhl_team
        ));
    }
    if nhl.roster_season != ahl.season {
        return Err(format!(
            "NHL lineup season {} does not match affiliate season {}",
            nhl.roster_season, ahl.season
        ));
    }
    if !ahl.development_rule_compliant {
        return Err("AHL affiliate projection is not development-rule compliant".to_owned());
    }
    if ahl.season == CURRENT_AHL_AFFILIATION_SEASON {
        let expected = current_ahl_affiliation_catalog()
            .affiliations
            .into_iter()
            .find(|row| row.nhl_team == nhl.team)
            .map(|row| row.ahl_team)
            .ok_or_else(|| format!("no current AHL affiliation for {}", nhl.team))?;
        if expected != ahl.ahl_team {
            return Err(format!(
                "{} is affiliated with {}, not {}",
                nhl.team, expected, ahl.ahl_team
            ));
        }
    }

    let nhl_dressed_players = exact_nhl_dressed_players(nhl)?;
    let mut nhl_assigned_players = nhl_dressed_players.clone();
    for player in &nhl.extras {
        if !nhl_assigned_players.insert(player.player_id) {
            return Err(format!(
                "duplicate NHL roster player {} across dressed lineup and extras",
                player.player_id
            ));
        }
    }
    let ahl_players = exact_ahl_dressed_players(ahl)?;
    let ahl_by_id = ahl
        .players
        .iter()
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    if ahl_by_id.len() != ahl.players.len() {
        return Err("AHL affiliate projection contains duplicate player identities".to_owned());
    }
    for id in &ahl_players {
        let player = ahl_by_id
            .get(id)
            .ok_or_else(|| format!("AHL dressed player {id} is absent from player rows"))?;
        if !player.assigned_to_affiliate || !player.dressed {
            return Err(format!(
                "AHL unit player {id} is not both assigned and dressed"
            ));
        }
    }
    let assigned_ahl_ids = ahl
        .players
        .iter()
        .filter(|player| player.assigned_to_affiliate)
        .map(|player| player.player_id)
        .collect::<BTreeSet<_>>();
    let overlap = nhl_assigned_players
        .intersection(&assigned_ahl_ids)
        .copied()
        .collect::<Vec<_>>();
    if !overlap.is_empty() {
        return Err(format!(
            "players assigned to both NHL and AHL organizations: {overlap:?}"
        ));
    }
    Ok(())
}

fn exact_nhl_dressed_players(lineup: &TeamLineupProjectionView) -> Result<BTreeSet<u32>, String> {
    if lineup.forward_lines.len() != 4 || lineup.defense_pairs.len() != 3 {
        return Err("NHL lineup must contain exactly 4F/3D units".to_owned());
    }
    let mut ids = BTreeSet::new();
    for line in &lineup.forward_lines {
        for player in [&line.left_wing, &line.center, &line.right_wing] {
            let player = player
                .as_ref()
                .ok_or_else(|| format!("NHL forward line {} is incomplete", line.line))?;
            if !ids.insert(player.player_id) {
                return Err(format!("duplicate NHL dressed player {}", player.player_id));
            }
        }
    }
    for pair in &lineup.defense_pairs {
        for player in [&pair.left, &pair.right] {
            let player = player
                .as_ref()
                .ok_or_else(|| format!("NHL defense pair {} is incomplete", pair.pair))?;
            if !ids.insert(player.player_id) {
                return Err(format!("duplicate NHL dressed player {}", player.player_id));
            }
        }
    }
    for player in [&lineup.goalies.starter, &lineup.goalies.backup] {
        let player = player
            .as_ref()
            .ok_or_else(|| "NHL lineup must contain starter and backup goalies".to_owned())?;
        if !ids.insert(player.player_id) {
            return Err(format!("duplicate NHL dressed player {}", player.player_id));
        }
    }
    Ok(ids)
}

fn exact_ahl_dressed_players(
    affiliate: &AhlAffiliateProjectionView,
) -> Result<BTreeSet<u32>, String> {
    let forwards = affiliate
        .lines
        .iter()
        .filter(|line| line.kind == AhlLineUnitKind::Forward)
        .collect::<Vec<_>>();
    let defense = affiliate
        .lines
        .iter()
        .filter(|line| line.kind == AhlLineUnitKind::Defense)
        .collect::<Vec<_>>();
    let goalies = affiliate
        .lines
        .iter()
        .filter(|line| line.kind == AhlLineUnitKind::Goalie)
        .collect::<Vec<_>>();
    if forwards.len() != 4
        || forwards.iter().any(|line| line.player_ids.len() != 3)
        || defense.len() != 3
        || defense.iter().any(|line| line.player_ids.len() != 2)
        || goalies.len() != 1
        || goalies[0].player_ids.len() != 2
    {
        return Err("AHL lineup must contain exactly 4F/3D/2G".to_owned());
    }
    let mut ids = BTreeSet::new();
    for id in affiliate
        .lines
        .iter()
        .flat_map(|line| line.player_ids.iter().copied())
    {
        if !ids.insert(id) {
            return Err(format!("duplicate AHL dressed player {id}"));
        }
    }
    Ok(ids)
}

fn nhl_units(lineup: &TeamLineupProjectionView) -> Vec<OrganizationUnitView> {
    let mut units = Vec::new();
    for line in &lineup.forward_lines {
        let players = [&line.left_wing, &line.center, &line.right_wing]
            .into_iter()
            .filter_map(Option::as_ref)
            .collect::<Vec<_>>();
        units.push(nhl_unit(
            lineup,
            OrganizationUnitKind::ForwardLine,
            line.line,
            players,
        ));
    }
    for pair in &lineup.defense_pairs {
        let players = [&pair.left, &pair.right]
            .into_iter()
            .filter_map(Option::as_ref)
            .collect::<Vec<_>>();
        units.push(nhl_unit(
            lineup,
            OrganizationUnitKind::DefensePair,
            pair.pair,
            players,
        ));
    }
    let goalies = [&lineup.goalies.starter, &lineup.goalies.backup]
        .into_iter()
        .filter_map(Option::as_ref)
        .collect::<Vec<_>>();
    units.push(nhl_unit(lineup, OrganizationUnitKind::Goalies, 1, goalies));
    units
}

fn nhl_unit(
    lineup: &TeamLineupProjectionView,
    kind: OrganizationUnitKind,
    unit: u8,
    players: Vec<&TeamLineupPlayerView>,
) -> OrganizationUnitView {
    let scores = players
        .iter()
        .filter_map(|player| player.score.value)
        .collect::<Vec<_>>();
    OrganizationUnitView {
        level: OrganizationLevel::Nhl,
        team: lineup.team.clone(),
        kind,
        unit,
        label: unit_label(&lineup.team, kind, unit),
        player_ids: players.iter().map(|player| player.player_id).collect(),
        player_names: players
            .iter()
            .map(|player| player.display_name.clone())
            .collect(),
        average_score: average(&scores),
    }
}

fn ahl_units(affiliate: &AhlAffiliateProjectionView) -> Result<Vec<OrganizationUnitView>, String> {
    let by_id = affiliate
        .players
        .iter()
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    affiliate
        .lines
        .iter()
        .map(|line| {
            let players = line
                .player_ids
                .iter()
                .map(|id| {
                    by_id
                        .get(id)
                        .copied()
                        .ok_or_else(|| format!("AHL line references unknown player {id}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let kind = match line.kind {
                AhlLineUnitKind::Forward => OrganizationUnitKind::ForwardLine,
                AhlLineUnitKind::Defense => OrganizationUnitKind::DefensePair,
                AhlLineUnitKind::Goalie => OrganizationUnitKind::Goalies,
            };
            let scores = players
                .iter()
                .map(|player| player.projected_score)
                .collect::<Vec<_>>();
            Ok(OrganizationUnitView {
                level: OrganizationLevel::Ahl,
                team: affiliate.ahl_team.clone(),
                kind,
                unit: line.unit as u8,
                label: unit_label(&affiliate.ahl_team, kind, line.unit as u8),
                player_ids: line.player_ids.clone(),
                player_names: players
                    .iter()
                    .map(|player| player.display_name.clone())
                    .collect(),
                average_score: average(&scores),
            })
        })
        .collect()
}

fn recall_candidate(player: &AhlAffiliatePlayerView) -> OrganizationRecallCandidateView {
    OrganizationRecallCandidateView {
        position_group: position_group(player.primary_position),
        rank: 0,
        player_id: player.player_id,
        display_name: player.display_name.clone(),
        primary_position: player.primary_position,
        projected_score: player.projected_score,
        recall_readiness: player.recall_readiness,
        dressed_for_affiliate: player.dressed,
        affiliate_assignment: player.line_assignment.clone(),
        waiver_required: player.waiver_required,
    }
}

fn position_group(position: Position) -> OrganizationPositionGroup {
    match position {
        Position::Defense => OrganizationPositionGroup::Defense,
        Position::Goalie => OrganizationPositionGroup::Goalie,
        _ => OrganizationPositionGroup::Forward,
    }
}

fn unit_label(team: &str, kind: OrganizationUnitKind, unit: u8) -> String {
    match kind {
        OrganizationUnitKind::ForwardLine => format!("{team} F{unit}"),
        OrganizationUnitKind::DefensePair => format!("{team} D{unit}"),
        OrganizationUnitKind::Goalies => format!("{team} goalies"),
    }
}

fn average(scores: &[f64]) -> Option<f64> {
    (!scores.is_empty()).then(|| scores.iter().sum::<f64>() / scores.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::ahl_affiliate::{
        build_ahl_affiliate_projection, AhlAffiliatePlayerInput, AhlAffiliateProjectionInput,
        AhlDevelopmentRuleInput,
    };

    fn load_input() -> OrganizationLineupForecastInput {
        let nhl_lineup = serde_json::from_str(include_str!(
            "../../../examples/team-lineup-nyr-2026-27.json"
        ))
        .unwrap();
        let mut players = Vec::new();
        for index in 0..14 {
            let primary_position = if index < 4 {
                Position::Center
            } else if index % 2 == 0 {
                Position::LeftWing
            } else {
                Position::RightWing
            };
            players.push(AhlAffiliatePlayerInput {
                player_id: 9_100_000 + index,
                display_name: format!("Hartford Forward {}", index + 1),
                primary_position,
                eligible_positions: vec![primary_position],
                projected_score: 70.0 - index as f64,
                prospect: index < 10,
                recall_readiness: Some(0.90 - index as f64 * 0.02),
                professional_games_at_season_start: Some(100),
                assigned_to_affiliate: true,
                waiver_required: false,
                source_league: "AHL".to_owned(),
            });
        }
        for index in 0..7 {
            players.push(AhlAffiliatePlayerInput {
                player_id: 9_200_000 + index,
                display_name: format!("Hartford Defense {}", index + 1),
                primary_position: Position::Defense,
                eligible_positions: vec![Position::Defense],
                projected_score: 65.0 - index as f64,
                prospect: index < 5,
                recall_readiness: Some(0.82 - index as f64 * 0.03),
                professional_games_at_season_start: Some(120),
                assigned_to_affiliate: true,
                waiver_required: false,
                source_league: "AHL".to_owned(),
            });
        }
        for index in 0..3 {
            players.push(AhlAffiliatePlayerInput {
                player_id: 9_300_000 + index,
                display_name: format!("Hartford Goalie {}", index + 1),
                primary_position: Position::Goalie,
                eligible_positions: vec![Position::Goalie],
                projected_score: 60.0 - index as f64,
                prospect: true,
                recall_readiness: Some(0.75 - index as f64 * 0.05),
                professional_games_at_season_start: None,
                assigned_to_affiliate: true,
                waiver_required: false,
                source_league: "AHL".to_owned(),
            });
        }
        let ahl_affiliate = build_ahl_affiliate_projection(&AhlAffiliateProjectionInput {
            nhl_team: "NYR".to_owned(),
            ahl_team: "Hartford Wolf Pack".to_owned(),
            season: 20262027,
            rule: AhlDevelopmentRuleInput::default(),
            pool_authority: Default::default(),
            players,
        })
        .unwrap();
        OrganizationLineupForecastInput {
            nhl_lineup,
            ahl_affiliate,
        }
    }

    #[test]
    fn builds_all_eight_forward_lines_six_pairs_and_four_goalies() {
        let view = build_organization_lineup_forecast(&load_input()).unwrap();
        assert_eq!(view.schema, ORGANIZATION_LINEUP_FORECAST_SCHEMA);
        assert_eq!(view.counts.forward_lines, 8);
        assert_eq!(view.counts.defense_pairs, 6);
        assert_eq!(view.counts.goalies, 4);
        assert_eq!(view.units.len(), 16);
        assert_eq!(view.recall_plan.len(), 3);
        assert!(view
            .recall_plan
            .iter()
            .all(|plan| plan.first_recall_player_id.is_some()));
    }

    #[test]
    fn rejects_double_assignment_across_nhl_and_ahl() {
        let mut input = load_input();
        let nhl_id = input.nhl_lineup.forward_lines[0]
            .center
            .as_ref()
            .unwrap()
            .player_id;
        let ahl_id = input.ahl_affiliate.lines[0].player_ids[0];
        input
            .ahl_affiliate
            .players
            .iter_mut()
            .find(|player| player.player_id == ahl_id)
            .unwrap()
            .player_id = nhl_id;
        input.ahl_affiliate.lines[0].player_ids[0] = nhl_id;
        let error = build_organization_lineup_forecast(&input).unwrap_err();
        assert!(error.contains("both NHL and AHL"));
    }

    #[test]
    fn rejects_nhl_extra_assigned_to_affiliate() {
        let mut input = load_input();
        let nhl_id = input.nhl_lineup.extras[0].player_id;
        let ahl_id = input.ahl_affiliate.lines[0].player_ids[0];
        input
            .ahl_affiliate
            .players
            .iter_mut()
            .find(|player| player.player_id == ahl_id)
            .unwrap()
            .player_id = nhl_id;
        input.ahl_affiliate.lines[0].player_ids[0] = nhl_id;
        let error = build_organization_lineup_forecast(&input).unwrap_err();
        assert!(error.contains("both NHL and AHL"));
    }

    #[test]
    fn rejects_incomplete_affiliate_units() {
        let mut input = load_input();
        input.ahl_affiliate.lines[0].player_ids.pop();
        let error = build_organization_lineup_forecast(&input).unwrap_err();
        assert!(error.contains("4F/3D/2G"));
    }
}
