//! AHL affiliate dressed-lineup projection with the official development rule.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::model::Position;

pub const AHL_AFFILIATE_PROJECTION_SCHEMA: &str = "ahl_affiliate_projection.v1";
pub const AHL_AFFILIATION_CATALOG_SCHEMA: &str = "ahl_affiliation_catalog.v1";
pub const CURRENT_AHL_AFFILIATION_SEASON: u32 = 20262027;
pub const AHL_AFFILIATION_SOURCE_URL: &str = "https://theahl.com/nhl-affiliations";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlAffiliationView {
    pub nhl_team: String,
    pub ahl_team: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlAffiliationCatalogView {
    pub schema: String,
    pub season: u32,
    pub checked_at: String,
    pub source_url: String,
    pub affiliations: Vec<AhlAffiliationView>,
}

pub fn current_ahl_affiliation_catalog() -> AhlAffiliationCatalogView {
    let affiliations = [
        ("ANA", "San Diego Gulls"),
        ("BOS", "Providence Bruins"),
        ("BUF", "Rochester Americans"),
        ("CAR", "Chicago Wolves"),
        ("CBJ", "Cleveland Monsters"),
        ("CGY", "Calgary Wranglers"),
        ("CHI", "Rockford IceHogs"),
        ("COL", "Colorado Eagles"),
        ("DAL", "Texas Stars"),
        ("DET", "Grand Rapids Griffins"),
        ("EDM", "Bakersfield Condors"),
        ("FLA", "Charlotte Checkers"),
        ("LAK", "Ontario Reign"),
        ("MIN", "Iowa Wild"),
        ("MTL", "Laval Rocket"),
        ("NJD", "Utica Comets"),
        ("NSH", "Milwaukee Admirals"),
        ("NYI", "Hamilton Hammers"),
        ("NYR", "Hartford Wolf Pack"),
        ("OTT", "Belleville Senators"),
        ("PHI", "Lehigh Valley Phantoms"),
        ("PIT", "Wilkes-Barre/Scranton Penguins"),
        ("SEA", "Coachella Valley Firebirds"),
        ("SJS", "San Jose Barracuda"),
        ("STL", "Springfield Thunderbirds"),
        ("TBL", "Syracuse Crunch"),
        ("TOR", "Toronto Marlies"),
        ("UTA", "Tucson Roadrunners"),
        ("VAN", "Abbotsford Canucks"),
        ("VGK", "Henderson Silver Knights"),
        ("WPG", "Manitoba Moose"),
        ("WSH", "Hershey Bears"),
    ]
    .into_iter()
    .map(|(nhl_team, ahl_team)| AhlAffiliationView {
        nhl_team: nhl_team.to_owned(),
        ahl_team: ahl_team.to_owned(),
    })
    .collect();
    AhlAffiliationCatalogView {
        schema: AHL_AFFILIATION_CATALOG_SCHEMA.to_owned(),
        season: CURRENT_AHL_AFFILIATION_SEASON,
        checked_at: "2026-07-24".to_owned(),
        source_url: AHL_AFFILIATION_SOURCE_URL.to_owned(),
        affiliations,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlDevelopmentRuleInput {
    pub dressed_skaters: usize,
    pub minimum_development_skaters: usize,
    pub professional_game_threshold: u32,
    pub source_url: String,
    pub checked_at: String,
}

impl Default for AhlDevelopmentRuleInput {
    fn default() -> Self {
        Self {
            dressed_skaters: 18,
            minimum_development_skaters: 12,
            professional_game_threshold: 260,
            source_url: "https://theahl.com/faq".to_owned(),
            checked_at: "2026-07-24T00:00:00Z".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlDevelopmentClassification {
    Development,
    Veteran,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlAffiliatePlayerInput {
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    pub eligible_positions: Vec<Position>,
    pub projected_score: f64,
    /// Organizational prospect status. This is deliberately independent of
    /// the AHL development-rule classification.
    #[serde(default)]
    pub prospect: bool,
    /// Optional 0..1 organization-supplied readiness for an NHL recall.
    #[serde(default)]
    pub recall_readiness: Option<f64>,
    /// Regular-season NHL, AHL, and European elite professional games at the
    /// start of the season, following the supplied AHL rule authority.
    #[serde(default)]
    pub professional_games_at_season_start: Option<u32>,
    /// Whether this scenario assigns the player to the affiliate. NHL roster
    /// branches set this explicitly; IceLines never equates a camp cut with a
    /// successful AHL assignment.
    pub assigned_to_affiliate: bool,
    #[serde(default)]
    pub waiver_required: bool,
    pub source_league: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlAffiliateProjectionInput {
    pub nhl_team: String,
    pub ahl_team: String,
    pub season: u32,
    pub rule: AhlDevelopmentRuleInput,
    pub players: Vec<AhlAffiliatePlayerInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlLineUnitKind {
    Forward,
    Defense,
    Goalie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlLineUnitView {
    pub kind: AhlLineUnitKind,
    pub unit: usize,
    pub player_ids: Vec<u32>,
    pub player_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlAffiliatePlayerView {
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    pub projected_score: f64,
    pub prospect: bool,
    pub recall_readiness: Option<f64>,
    pub professional_games_at_season_start: Option<u32>,
    pub classification: Option<AhlDevelopmentClassification>,
    pub assigned_to_affiliate: bool,
    pub waiver_required: bool,
    pub dressed: bool,
    #[serde(default)]
    pub line_assignment: Option<String>,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    pub source_league: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlProspectPoolRowView {
    pub rank: usize,
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    pub projected_score: f64,
    pub recall_readiness: Option<f64>,
    pub dressed: bool,
    pub line_assignment: Option<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlAffiliateProjectionView {
    pub schema: String,
    pub nhl_team: String,
    pub ahl_team: String,
    pub season: u32,
    pub rule: AhlDevelopmentRuleInput,
    pub dressed_skaters: usize,
    pub dressed_goalies: usize,
    pub development_skaters: usize,
    pub veteran_skaters: usize,
    pub maximum_veteran_skaters: usize,
    pub unused_veteran_slots: usize,
    pub available_veterans_not_dressed: usize,
    pub development_rule_compliant: bool,
    pub assigned_prospects: usize,
    pub dressed_prospects: usize,
    pub prospect_pool: Vec<AhlProspectPoolRowView>,
    pub players: Vec<AhlAffiliatePlayerView>,
    pub lines: Vec<AhlLineUnitView>,
    pub disclosures: Vec<String>,
}

pub fn classify_ahl_development_player(
    professional_games_at_season_start: u32,
    threshold: u32,
) -> AhlDevelopmentClassification {
    if professional_games_at_season_start <= threshold {
        AhlDevelopmentClassification::Development
    } else {
        AhlDevelopmentClassification::Veteran
    }
}

pub fn build_ahl_affiliate_projection(
    input: &AhlAffiliateProjectionInput,
) -> Result<AhlAffiliateProjectionView, String> {
    validate(input)?;
    let classify = |player: &AhlAffiliatePlayerInput| {
        player.professional_games_at_season_start.map(|games| {
            classify_ahl_development_player(games, input.rule.professional_game_threshold)
        })
    };
    let available = input
        .players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.assigned_to_affiliate)
        .collect::<Vec<_>>();
    let forwards = available
        .iter()
        .copied()
        .filter(|(_, player)| player.primary_position.is_forward())
        .collect::<Vec<_>>();
    let defense = available
        .iter()
        .copied()
        .filter(|(_, player)| player.primary_position == Position::Defense)
        .collect::<Vec<_>>();
    let mut goalies = available
        .iter()
        .copied()
        .filter(|(_, player)| player.primary_position == Position::Goalie)
        .collect::<Vec<_>>();
    goalies.sort_by(|a, b| player_order(a.1, b.1));

    let maximum_veterans = input
        .rule
        .dressed_skaters
        .saturating_sub(input.rule.minimum_development_skaters);
    let mut best: Option<(f64, Vec<usize>, Vec<usize>)> = None;
    for veteran_forwards in 0..=12 {
        for veteran_defense in 0..=6 {
            if veteran_forwards + veteran_defense > maximum_veterans {
                continue;
            }
            let Some(selected_forwards) =
                select_position_group(&forwards, 12, veteran_forwards, classify, true)
            else {
                continue;
            };
            let Some(selected_defense) =
                select_position_group(&defense, 6, veteran_defense, classify, false)
            else {
                continue;
            };
            let score = selected_forwards
                .iter()
                .chain(&selected_defense)
                .map(|index| input.players[*index].projected_score)
                .sum::<f64>();
            if best.as_ref().is_none_or(|best| score > best.0) {
                best = Some((score, selected_forwards, selected_defense));
            }
        }
    }
    let Some((_, mut selected_forwards, mut selected_defense)) = best else {
        return Err(format!(
            "{} cannot dress 12F/6D while satisfying the AHL development rule",
            input.ahl_team
        ));
    };
    if goalies.len() < 2 {
        return Err(format!(
            "{} cannot dress two assigned goalies",
            input.ahl_team
        ));
    }
    selected_forwards.sort_by(|a, b| player_order(&input.players[*a], &input.players[*b]));
    selected_defense.sort_by(|a, b| player_order(&input.players[*a], &input.players[*b]));
    let selected_goalies = vec![goalies[0].0, goalies[1].0];

    let selected = selected_forwards
        .iter()
        .chain(&selected_defense)
        .chain(&selected_goalies)
        .copied()
        .collect::<BTreeSet<_>>();
    let mut assignments = vec![None; input.players.len()];
    let mut lines = Vec::new();
    let centers = selected_forwards
        .iter()
        .copied()
        .filter(|index| {
            input.players[*index]
                .eligible_positions
                .contains(&Position::Center)
        })
        .take(4)
        .collect::<Vec<_>>();
    let wings = selected_forwards
        .iter()
        .copied()
        .filter(|index| !centers.contains(index))
        .collect::<Vec<_>>();
    let forward_lines = (0..4)
        .map(|line| vec![wings[line * 2], centers[line], wings[line * 2 + 1]])
        .collect::<Vec<_>>();
    for (line_index, line) in forward_lines.iter().enumerate() {
        for (slot, index) in line.iter().enumerate() {
            assignments[*index] = Some(format!("F{}{}", line_index + 1, ["LW", "C", "RW"][slot]));
        }
        lines.push(line_view(
            input,
            AhlLineUnitKind::Forward,
            line_index + 1,
            line,
        ));
    }
    for (pair_index, chunk) in selected_defense.chunks(2).enumerate() {
        for (slot, index) in chunk.iter().enumerate() {
            assignments[*index] = Some(format!("D{}{}", pair_index + 1, ["L", "R"][slot]));
        }
        lines.push(line_view(
            input,
            AhlLineUnitKind::Defense,
            pair_index + 1,
            chunk,
        ));
    }
    for (goalie_index, index) in selected_goalies.iter().enumerate() {
        assignments[*index] = Some(if goalie_index == 0 { "G1" } else { "G2" }.to_owned());
    }
    lines.push(line_view(
        input,
        AhlLineUnitKind::Goalie,
        1,
        &selected_goalies,
    ));

    let development_skaters = selected_forwards
        .iter()
        .chain(&selected_defense)
        .filter(|index| {
            classify(&input.players[**index]) == Some(AhlDevelopmentClassification::Development)
        })
        .count();
    let veteran_skaters = input.rule.dressed_skaters - development_skaters;
    let available_veterans = available
        .iter()
        .filter(|(_, player)| {
            player.primary_position != Position::Goalie
                && classify(player) == Some(AhlDevelopmentClassification::Veteran)
        })
        .count();
    let mut players = input
        .players
        .iter()
        .enumerate()
        .map(|(index, player)| {
            let classification = classify(player);
            let dressed = selected.contains(&index);
            let blocked_reason = if dressed {
                None
            } else if !player.assigned_to_affiliate {
                Some("not_assigned_to_affiliate".to_owned())
            } else if player.primary_position != Position::Goalie
                && classification == Some(AhlDevelopmentClassification::Veteran)
                && veteran_skaters == maximum_veterans
            {
                Some("development_rule_veteran_limit".to_owned())
            } else {
                Some("lineup_competition".to_owned())
            };
            AhlAffiliatePlayerView {
                player_id: player.player_id,
                display_name: player.display_name.clone(),
                primary_position: player.primary_position,
                projected_score: player.projected_score,
                prospect: player.prospect,
                recall_readiness: player.recall_readiness,
                professional_games_at_season_start: player.professional_games_at_season_start,
                classification,
                assigned_to_affiliate: player.assigned_to_affiliate,
                waiver_required: player.waiver_required,
                dressed,
                line_assignment: assignments[index].clone(),
                blocked_reason,
                source_league: player.source_league.clone(),
            }
        })
        .collect::<Vec<_>>();
    players.sort_by(|a, b| {
        b.dressed
            .cmp(&a.dressed)
            .then_with(|| b.projected_score.total_cmp(&a.projected_score))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    let mut prospect_pool = players
        .iter()
        .filter(|player| player.assigned_to_affiliate && player.prospect)
        .map(|player| AhlProspectPoolRowView {
            rank: 0,
            player_id: player.player_id,
            display_name: player.display_name.clone(),
            primary_position: player.primary_position,
            projected_score: player.projected_score,
            recall_readiness: player.recall_readiness,
            dressed: player.dressed,
            line_assignment: player.line_assignment.clone(),
            blocked_reason: player.blocked_reason.clone(),
        })
        .collect::<Vec<_>>();
    prospect_pool.sort_by(|a, b| {
        b.projected_score
            .total_cmp(&a.projected_score)
            .then_with(|| {
                b.recall_readiness
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.recall_readiness.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    for (index, prospect) in prospect_pool.iter_mut().enumerate() {
        prospect.rank = index + 1;
    }
    let dressed_prospects = prospect_pool
        .iter()
        .filter(|prospect| prospect.dressed)
        .count();

    Ok(AhlAffiliateProjectionView {
        schema: AHL_AFFILIATE_PROJECTION_SCHEMA.to_owned(),
        nhl_team: input.nhl_team.clone(),
        ahl_team: input.ahl_team.clone(),
        season: input.season,
        rule: input.rule.clone(),
        dressed_skaters: input.rule.dressed_skaters,
        dressed_goalies: 2,
        development_skaters,
        veteran_skaters,
        maximum_veteran_skaters: maximum_veterans,
        unused_veteran_slots: maximum_veterans.saturating_sub(veteran_skaters),
        available_veterans_not_dressed: available_veterans.saturating_sub(veteran_skaters),
        development_rule_compliant: development_skaters
            >= input.rule.minimum_development_skaters,
        assigned_prospects: prospect_pool.len(),
        dressed_prospects,
        prospect_pool,
        players,
        lines,
        disclosures: vec![
            "The AHL development rule is enforced on the 18 dressed skaters; goaltenders do not count toward the twelve-player development minimum.".to_owned(),
            "Organizational prospect status and recall readiness are explicit inputs; AHL development-rule eligibility does not automatically make a player a prospect.".to_owned(),
            "Professional-game classification uses regular-season totals fixed at the start of the season and must be supplied from authority; IceLines does not infer missing totals from age or NHL games alone.".to_owned(),
            "This is an affiliate assignment scenario. Waiver clearance, contract consent, injuries, recalls, and players assigned to other leagues remain separate gates.".to_owned(),
        ],
    })
}

fn select_position_group(
    players: &[(usize, &AhlAffiliatePlayerInput)],
    slots: usize,
    veteran_slots: usize,
    classify: impl Fn(&AhlAffiliatePlayerInput) -> Option<AhlDevelopmentClassification> + Copy,
    require_centers: bool,
) -> Option<Vec<usize>> {
    let mut veterans = players
        .iter()
        .copied()
        .filter(|(_, player)| classify(player) == Some(AhlDevelopmentClassification::Veteran))
        .collect::<Vec<_>>();
    let mut development = players
        .iter()
        .copied()
        .filter(|(_, player)| classify(player) == Some(AhlDevelopmentClassification::Development))
        .collect::<Vec<_>>();
    veterans.sort_by(|a, b| player_order(a.1, b.1));
    development.sort_by(|a, b| player_order(a.1, b.1));
    let development_slots = slots.checked_sub(veteran_slots)?;
    if veterans.len() < veteran_slots || development.len() < development_slots {
        return None;
    }
    let mut chosen = veterans[..veteran_slots]
        .iter()
        .chain(&development[..development_slots])
        .map(|(index, _)| *index)
        .collect::<Vec<_>>();
    if require_centers {
        let mut centers = chosen
            .iter()
            .filter(|index| {
                players_for_index(players, **index)
                    .eligible_positions
                    .contains(&Position::Center)
            })
            .count();
        for pool in [&veterans, &development] {
            while centers < 4 {
                let replacement = pool.iter().find(|(index, player)| {
                    !chosen.contains(index) && player.eligible_positions.contains(&Position::Center)
                });
                let Some((replacement_index, replacement_player)) = replacement else {
                    break;
                };
                let replacement_class = classify(replacement_player);
                let outgoing = chosen
                    .iter()
                    .enumerate()
                    .filter(|(_, index)| {
                        let player = players_for_index(players, **index);
                        !player.eligible_positions.contains(&Position::Center)
                            && classify(player) == replacement_class
                    })
                    .min_by(|(_, a), (_, b)| {
                        players_for_index(players, **a)
                            .projected_score
                            .total_cmp(&players_for_index(players, **b).projected_score)
                    })
                    .map(|(position, _)| position);
                let Some(outgoing) = outgoing else {
                    break;
                };
                chosen[outgoing] = *replacement_index;
                centers += 1;
            }
        }
        if centers < 4 {
            return None;
        }
    }
    Some(chosen)
}

fn players_for_index<'a>(
    players: &[(usize, &'a AhlAffiliatePlayerInput)],
    index: usize,
) -> &'a AhlAffiliatePlayerInput {
    players
        .iter()
        .find(|(candidate, _)| *candidate == index)
        .expect("selected index belongs to position group")
        .1
}

fn player_order(a: &AhlAffiliatePlayerInput, b: &AhlAffiliatePlayerInput) -> std::cmp::Ordering {
    b.projected_score
        .total_cmp(&a.projected_score)
        .then_with(|| a.display_name.cmp(&b.display_name))
}

fn line_view(
    input: &AhlAffiliateProjectionInput,
    kind: AhlLineUnitKind,
    unit: usize,
    indices: &[usize],
) -> AhlLineUnitView {
    AhlLineUnitView {
        kind,
        unit,
        player_ids: indices
            .iter()
            .map(|index| input.players[*index].player_id)
            .collect(),
        player_names: indices
            .iter()
            .map(|index| input.players[*index].display_name.clone())
            .collect(),
    }
}

fn validate(input: &AhlAffiliateProjectionInput) -> Result<(), String> {
    if input.nhl_team.trim().is_empty() || input.ahl_team.trim().is_empty() {
        return Err("affiliate projection requires NHL and AHL team labels".to_owned());
    }
    if input.rule.dressed_skaters != 18
        || input.rule.minimum_development_skaters > input.rule.dressed_skaters
        || input.rule.professional_game_threshold == 0
    {
        return Err("affiliate projection has an unsupported development rule shape".to_owned());
    }
    if !(input.rule.source_url.starts_with("https://")
        || input.rule.source_url.starts_with("http://"))
        || input.rule.checked_at.trim().is_empty()
    {
        return Err("affiliate development rule requires dated absolute authority".to_owned());
    }
    if input.season == CURRENT_AHL_AFFILIATION_SEASON {
        let catalog = current_ahl_affiliation_catalog();
        let expected = catalog
            .affiliations
            .iter()
            .find(|row| row.nhl_team == input.nhl_team)
            .ok_or_else(|| {
                format!(
                    "affiliate projection has unknown {} NHL team {}",
                    input.season, input.nhl_team
                )
            })?;
        if expected.ahl_team != input.ahl_team {
            return Err(format!(
                "{} is affiliated with {} in {}, not {}",
                input.nhl_team, expected.ahl_team, input.season, input.ahl_team
            ));
        }
    }
    let mut ids = BTreeSet::new();
    for player in &input.players {
        if player.player_id == 0
            || !ids.insert(player.player_id)
            || player.display_name.trim().is_empty()
            || !player.projected_score.is_finite()
            || player.recall_readiness.is_some_and(|readiness| {
                !readiness.is_finite() || !(0.0..=1.0).contains(&readiness)
            })
            || !player.eligible_positions.contains(&player.primary_position)
        {
            return Err("affiliate projection contains an invalid player row".to_owned());
        }
        if player.assigned_to_affiliate
            && player.primary_position != Position::Goalie
            && player.professional_games_at_season_start.is_none()
        {
            return Err(format!(
                "affiliate player {} lacks start-of-season professional games required by the development rule",
                player.player_id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(id: u32, position: Position, score: f64, games: u32) -> AhlAffiliatePlayerInput {
        AhlAffiliatePlayerInput {
            player_id: id,
            display_name: format!("Player {id}"),
            primary_position: position,
            eligible_positions: if position.is_forward() {
                vec![position, Position::Center]
            } else {
                vec![position]
            },
            projected_score: score,
            prospect: games <= 260,
            recall_readiness: Some((score / 100.0).clamp(0.0, 1.0)),
            professional_games_at_season_start: Some(games),
            assigned_to_affiliate: true,
            waiver_required: games > 260,
            source_league: "test".to_owned(),
        }
    }

    #[test]
    fn development_rule_can_bench_a_higher_scored_veteran() {
        let mut players = Vec::new();
        for id in 1..=16 {
            players.push(player(
                id,
                if id <= 8 {
                    Position::Center
                } else {
                    Position::LeftWing
                },
                100.0 - f64::from(id),
                if id <= 8 { 400 } else { 100 },
            ));
        }
        for id in 20..=28 {
            players.push(player(
                id,
                Position::Defense,
                100.0 - f64::from(id),
                if id <= 22 { 400 } else { 100 },
            ));
        }
        players.push(player(30, Position::Goalie, 60.0, 500));
        players.push(player(31, Position::Goalie, 55.0, 50));
        let view = build_ahl_affiliate_projection(&AhlAffiliateProjectionInput {
            nhl_team: "NYR".to_owned(),
            ahl_team: "Hartford Wolf Pack".to_owned(),
            season: 20262027,
            rule: AhlDevelopmentRuleInput::default(),
            players,
        })
        .unwrap();
        assert_eq!(view.dressed_skaters, 18);
        assert_eq!(view.development_skaters, 12);
        assert_eq!(view.veteran_skaters, 6);
        assert!(view.development_rule_compliant);
        assert!(view.assigned_prospects >= view.dressed_prospects);
        assert!(view
            .prospect_pool
            .windows(2)
            .all(|rows| rows[0].projected_score >= rows[1].projected_score));
        assert!(view.players.iter().any(|player| {
            !player.dressed
                && player.blocked_reason.as_deref() == Some("development_rule_veteran_limit")
        }));
        assert_eq!(
            view.lines
                .iter()
                .filter(|line| line.kind == AhlLineUnitKind::Forward)
                .count(),
            4
        );
    }

    #[test]
    fn missing_professional_games_fail_closed() {
        let mut row = player(1, Position::Center, 10.0, 10);
        row.professional_games_at_season_start = None;
        let error = build_ahl_affiliate_projection(&AhlAffiliateProjectionInput {
            nhl_team: "SEA".to_owned(),
            ahl_team: "Coachella Valley Firebirds".to_owned(),
            season: 20262027,
            rule: AhlDevelopmentRuleInput::default(),
            players: vec![row],
        })
        .unwrap_err();
        assert!(error.contains("professional games"));
    }

    #[test]
    fn official_threshold_is_inclusive() {
        assert_eq!(
            classify_ahl_development_player(260, 260),
            AhlDevelopmentClassification::Development
        );
        assert_eq!(
            classify_ahl_development_player(261, 260),
            AhlDevelopmentClassification::Veteran
        );
    }

    #[test]
    fn current_affiliation_catalog_covers_32_unique_organizations() {
        let catalog = current_ahl_affiliation_catalog();
        assert_eq!(catalog.schema, AHL_AFFILIATION_CATALOG_SCHEMA);
        assert_eq!(catalog.affiliations.len(), 32);
        assert_eq!(
            catalog
                .affiliations
                .iter()
                .map(|row| row.nhl_team.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            32
        );
        assert!(catalog
            .affiliations
            .iter()
            .any(|row| { row.nhl_team == "NYR" && row.ahl_team == "Hartford Wolf Pack" }));
        assert!(catalog
            .affiliations
            .iter()
            .any(|row| { row.nhl_team == "SEA" && row.ahl_team == "Coachella Valley Firebirds" }));
    }

    #[test]
    fn current_projection_rejects_the_wrong_affiliate() {
        let error = build_ahl_affiliate_projection(&AhlAffiliateProjectionInput {
            nhl_team: "NYR".to_owned(),
            ahl_team: "Coachella Valley Firebirds".to_owned(),
            season: CURRENT_AHL_AFFILIATION_SEASON,
            rule: AhlDevelopmentRuleInput::default(),
            players: Vec::new(),
        })
        .unwrap_err();
        assert!(error.contains("Hartford Wolf Pack"));
    }

    #[test]
    fn recall_readiness_outside_probability_range_fails_closed() {
        let mut row = player(1, Position::Center, 10.0, 10);
        row.recall_readiness = Some(1.01);
        let error = build_ahl_affiliate_projection(&AhlAffiliateProjectionInput {
            nhl_team: "SEA".to_owned(),
            ahl_team: "Coachella Valley Firebirds".to_owned(),
            season: CURRENT_AHL_AFFILIATION_SEASON,
            rule: AhlDevelopmentRuleInput::default(),
            players: vec![row],
        })
        .unwrap_err();
        assert!(error.contains("invalid player row"));
    }
}
