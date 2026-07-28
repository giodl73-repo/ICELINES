//! Historical player-role and opponent-style evidence for The Bench.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::model::{Position, Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};

use super::management_behavior::{OpponentTacticalStyle, PlayerMatchupRoleInput};
use super::EvidenceLabel;

pub const PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA: &str = "player_matchup_role_evidence.v1";
pub const OPPONENT_STYLE_EVIDENCE_SCHEMA: &str = "opponent_style_evidence.v1";
pub const TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA: &str = "team_player_matchup_role_evidence.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerRoleSeasonFactsInput {
    pub player_id: u32,
    pub is_defenseman: bool,
    pub games_played: u32,
    pub even_strength_toi_seconds: u32,
    pub short_handed_toi_seconds: u32,
    pub hits: u32,
    pub blocked_shots: u32,
    pub takeaways: u32,
    pub giveaways: u32,
    /// Offensive-zone start percentage. Absence remains a missing component.
    #[serde(default)]
    pub offensive_zone_start_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerMatchupRoleEvidenceRow {
    pub schema: String,
    pub role: PlayerMatchupRoleInput,
    pub peer_group: String,
    pub component_coverage_pct: f64,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamPlayerMatchupRoleEvidenceView {
    pub schema: String,
    pub team: String,
    pub season: u32,
    pub season_type: SeasonType,
    pub roster_skaters: usize,
    pub rated_skaters: usize,
    pub league_forward_peers: usize,
    pub league_defense_peers: usize,
    pub roles: Vec<PlayerMatchupRoleEvidenceRow>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

/// Build one current-roster team's role evidence against the complete league
/// peer population resident in the selected StatsRepository window.
pub fn build_team_player_matchup_role_evidence(
    repository: &StatsRepository,
    team: &TeamAbbr,
    season: Season,
    season_type: SeasonType,
) -> Result<TeamPlayerMatchupRoleEvidenceView, String> {
    let roster = repository.team_roster(team, season, season_type);
    let roster_skaters = roster
        .iter()
        .filter(|player| !player.is_goalie())
        .map(|player| player.id().0)
        .collect::<BTreeSet<_>>();
    if roster_skaters.is_empty() {
        return Err(format!(
            "matchup-role repository adapter found no roster skaters for {team}"
        ));
    }
    let facts = repository
        .skaters(season, season_type)
        .filter_map(player_role_facts)
        .collect::<Vec<_>>();
    let league_forward_peers = facts.iter().filter(|row| !row.is_defenseman).count();
    let league_defense_peers = facts.iter().filter(|row| row.is_defenseman).count();
    if league_forward_peers < 2 || league_defense_peers < 2 {
        return Err(
            "matchup-role repository adapter requires at least two forward and defense peers"
                .to_owned(),
        );
    }
    let by_player = build_player_matchup_role_evidence(&facts)?
        .into_iter()
        .map(|row| (row.role.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let roles = roster_skaters
        .iter()
        .filter_map(|player_id| by_player.get(player_id).cloned())
        .collect::<Vec<_>>();
    let missing = roster_skaters
        .iter()
        .filter(|player_id| !by_player.contains_key(player_id))
        .copied()
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if !missing.is_empty() {
        warnings.push(format!(
            "{} roster skater(s) lack complete realtime and time-on-ice facts: {}",
            missing.len(),
            missing
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(TeamPlayerMatchupRoleEvidenceView {
        schema: TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA.to_owned(),
        team: team.0.clone(),
        season: season.0,
        season_type,
        roster_skaters: roster_skaters.len(),
        rated_skaters: roles.len(),
        league_forward_peers,
        league_defense_peers,
        roles,
        warnings,
        disclosures: vec![
            "Current-roster membership selects the output; the full resident league window supplies position-group percentiles.".to_owned(),
            "Players require realtime counts plus even-strength and shorthanded TOI; incomplete players remain unrated and are named in warnings.".to_owned(),
            "Zone-start coverage is currently absent from the repository adapter, so role component coverage is 90%.".to_owned(),
        ],
    })
}

fn player_role_facts(player: PlayerView<'_>) -> Option<PlayerRoleSeasonFactsInput> {
    if player.position() == Position::Goalie {
        return None;
    }
    let time_on_ice = player.stats.time_on_ice.as_ref()?;
    Some(PlayerRoleSeasonFactsInput {
        player_id: player.id().0,
        is_defenseman: player.position() == Position::Defense,
        games_played: player.gp(),
        even_strength_toi_seconds: time_on_ice.ev_time_on_ice_sec,
        short_handed_toi_seconds: time_on_ice.sh_time_on_ice_sec,
        hits: player.hits()?,
        blocked_shots: player.blocked_shots()?,
        takeaways: player.takeaways()?,
        giveaways: player.giveaways()?,
        offensive_zone_start_pct: None,
    })
}

/// Rank observed rates within forward/defense peer groups. These are
/// descriptive role indicators; downstream game-plan construction applies its
/// own games-played confidence shrinkage.
pub fn build_player_matchup_role_evidence(
    facts: &[PlayerRoleSeasonFactsInput],
) -> Result<Vec<PlayerMatchupRoleEvidenceRow>, String> {
    if facts.is_empty() {
        return Err("matchup-role evidence requires player season facts".to_owned());
    }
    let mut ids = BTreeSet::new();
    for row in facts {
        if row.player_id == 0
            || !ids.insert(row.player_id)
            || row.games_played == 0
            || row.even_strength_toi_seconds == 0
        {
            return Err(
                "matchup-role facts require unique players with games and even-strength TOI"
                    .to_owned(),
            );
        }
        if row
            .offensive_zone_start_pct
            .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
        {
            return Err("offensive-zone start percentage must be between 0 and 100".to_owned());
        }
    }

    let mut output = Vec::with_capacity(facts.len());
    for row in facts {
        let peers = facts
            .iter()
            .filter(|peer| peer.is_defenseman == row.is_defenseman)
            .collect::<Vec<_>>();
        let hits = percentile(rate_per_60(row.hits, row), &peers, |peer| {
            rate_per_60(peer.hits, peer)
        });
        let blocks = percentile(rate_per_60(row.blocked_shots, row), &peers, |peer| {
            rate_per_60(peer.blocked_shots, peer)
        });
        let takeaways = percentile(rate_per_60(row.takeaways, row), &peers, |peer| {
            rate_per_60(peer.takeaways, peer)
        });
        let giveaway_security = 100.0
            - percentile(rate_per_60(row.giveaways, row), &peers, |peer| {
                rate_per_60(peer.giveaways, peer)
            });
        let net_takeaways = percentile(
            rate_per_60(row.takeaways, row) - rate_per_60(row.giveaways, row),
            &peers,
            |peer| rate_per_60(peer.takeaways, peer) - rate_per_60(peer.giveaways, peer),
        );
        let penalty_kill = percentile(
            f64::from(row.short_handed_toi_seconds) / f64::from(row.games_played),
            &peers,
            |peer| f64::from(peer.short_handed_toi_seconds) / f64::from(peer.games_played),
        );
        let defensive_zone = row.offensive_zone_start_pct.map(|value| 100.0 - value);
        let defensive_score = weighted(&[
            (blocks, 0.30),
            (takeaways, 0.20),
            (net_takeaways, 0.20),
            (penalty_kill, 0.20),
            (defensive_zone.unwrap_or(50.0), 0.10),
        ]);
        let transition_score = weighted(&[
            (net_takeaways, 0.45),
            (takeaways, 0.30),
            (giveaway_security, 0.25),
        ]);
        let forecheck_score = weighted(&[(hits, 0.55), (takeaways, 0.30), (net_takeaways, 0.15)]);
        let physical_score = weighted(&[(hits, 0.65), (blocks, 0.35)]);
        output.push(PlayerMatchupRoleEvidenceRow {
            schema: PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA.to_owned(),
            role: PlayerMatchupRoleInput {
                player_id: row.player_id,
                defensive_score,
                transition_score,
                forecheck_score,
                physical_score,
                evidence_games: row.games_played,
                evidence_label: EvidenceLabel::Estimated,
            },
            peer_group: if row.is_defenseman { "defense" } else { "forward" }.to_owned(),
            component_coverage_pct: if defensive_zone.is_some() { 100.0 } else { 90.0 },
            disclosures: vec![
                "Scores are peer-group percentiles from descriptive rates, not isolated causal player effects.".to_owned(),
                "Transition is a takeaway/giveaway proxy until controlled entry and exit events are available.".to_owned(),
                "The Bench independently shrinks these scores toward neutral using evidence games.".to_owned(),
            ],
        });
    }
    output.sort_by_key(|row| row.role.player_id);
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamStyleSeasonFactsInput {
    pub team: String,
    pub season: u32,
    pub games_played: u32,
    /// Fraction of scheduled games with the required event feed, from 0 to 1.
    pub event_coverage: f64,
    pub rush_attempts: u32,
    pub east_west_sequences: u32,
    pub dump_ins: u32,
    pub forecheck_recoveries: u32,
    pub cycle_sequences: u32,
    pub counterattack_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpponentStyleScoreView {
    pub style: OpponentTacticalStyle,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpponentStyleEvidenceRow {
    pub schema: String,
    pub team: String,
    pub season: u32,
    pub style: Option<OpponentTacticalStyle>,
    pub confidence: f64,
    pub evidence_games: u32,
    pub event_coverage: f64,
    pub scores: Vec<OpponentStyleScoreView>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

pub fn build_opponent_style_evidence(
    facts: &[TeamStyleSeasonFactsInput],
) -> Result<Vec<OpponentStyleEvidenceRow>, String> {
    if facts.is_empty() {
        return Err("opponent-style evidence requires team season facts".to_owned());
    }
    let mut teams = BTreeSet::new();
    let season = facts[0].season;
    for row in facts {
        if row.team.len() != 3
            || !row.team.bytes().all(|byte| byte.is_ascii_uppercase())
            || !teams.insert(row.team.clone())
            || row.season != season
            || row.games_played == 0
            || !row.event_coverage.is_finite()
            || !(0.0..=1.0).contains(&row.event_coverage)
        {
            return Err(
                "opponent-style facts require one season, unique NHL teams, games, and 0-1 coverage".to_owned(),
            );
        }
    }
    let mut rows = Vec::with_capacity(facts.len());
    for row in facts {
        let score = |value: f64, metric: fn(&TeamStyleSeasonFactsInput) -> f64| {
            percentile(value, &facts.iter().collect::<Vec<_>>(), |peer| {
                metric(peer)
            })
        };
        let north_south = score(rate_per_game(row.rush_attempts, row.games_played), |peer| {
            rate_per_game(peer.rush_attempts, peer.games_played)
        });
        let east_west = score(
            rate_per_game(row.east_west_sequences, row.games_played),
            |peer| rate_per_game(peer.east_west_sequences, peer.games_played),
        );
        let dump = score(rate_per_game(row.dump_ins, row.games_played), |peer| {
            rate_per_game(peer.dump_ins, peer.games_played)
        });
        let recoveries = score(
            rate_per_game(row.forecheck_recoveries, row.games_played),
            |peer| rate_per_game(peer.forecheck_recoveries, peer.games_played),
        );
        let heavy_cycle = score(
            rate_per_game(row.cycle_sequences, row.games_played),
            |peer| rate_per_game(peer.cycle_sequences, peer.games_played),
        );
        let counterattack = score(
            rate_per_game(row.counterattack_attempts, row.games_played),
            |peer| rate_per_game(peer.counterattack_attempts, peer.games_played),
        );
        let mut scores = vec![
            OpponentStyleScoreView {
                style: OpponentTacticalStyle::NorthSouthRush,
                score: north_south,
            },
            OpponentStyleScoreView {
                style: OpponentTacticalStyle::EastWestPossession,
                score: east_west,
            },
            OpponentStyleScoreView {
                style: OpponentTacticalStyle::DumpAndChase,
                score: weighted(&[(dump, 0.60), (recoveries, 0.40)]),
            },
            OpponentStyleScoreView {
                style: OpponentTacticalStyle::HeavyCycle,
                score: heavy_cycle,
            },
            OpponentStyleScoreView {
                style: OpponentTacticalStyle::Counterattack,
                score: counterattack,
            },
        ];
        scores.sort_by(|a, b| b.score.total_cmp(&a.score));
        let separation = scores[0].score - scores[1].score;
        let sufficient = row.games_played >= 10 && row.event_coverage >= 0.50;
        let style = if !sufficient {
            None
        } else if separation <= 5.0 {
            Some(OpponentTacticalStyle::Balanced)
        } else {
            Some(scores[0].style)
        };
        let evidence =
            f64::from(row.games_played) / (f64::from(row.games_played) + 20.0) * row.event_coverage;
        let confidence = (evidence * (0.70 + 0.30 * separation / 100.0)).clamp(0.0, 1.0);
        rows.push(OpponentStyleEvidenceRow {
            schema: OPPONENT_STYLE_EVIDENCE_SCHEMA.to_owned(),
            team: row.team.clone(),
            season: row.season,
            style,
            confidence,
            evidence_games: row.games_played,
            event_coverage: row.event_coverage,
            scores,
            warnings: (!sufficient)
                .then(|| {
                    "Opponent style is no-read: at least 10 games and 50% event coverage are required."
                        .to_owned()
                })
                .into_iter()
                .collect(),
            disclosures: vec![
                "Styles are league-relative descriptive event archetypes, not permanent team identities.".to_owned(),
                "Balanced means the two leading archetype scores are within five percentile points.".to_owned(),
            ],
        });
    }
    rows.sort_by(|a, b| a.team.cmp(&b.team));
    Ok(rows)
}

fn rate_per_60(count: u32, row: &PlayerRoleSeasonFactsInput) -> f64 {
    f64::from(count) * 3_600.0 / f64::from(row.even_strength_toi_seconds)
}

fn rate_per_game(count: u32, games: u32) -> f64 {
    f64::from(count) / f64::from(games)
}

fn percentile<T>(value: f64, peers: &[&T], metric: impl Fn(&T) -> f64) -> f64 {
    let lower = peers.iter().filter(|peer| metric(peer) < value).count() as f64;
    let equal = peers
        .iter()
        .filter(|peer| (metric(peer) - value).abs() < 1e-9)
        .count() as f64;
    ((lower + equal * 0.5) / peers.len() as f64 * 100.0).clamp(0.0, 100.0)
}

fn weighted(values: &[(f64, f64)]) -> f64 {
    let denominator = values.iter().map(|(_, weight)| weight).sum::<f64>();
    values
        .iter()
        .map(|(value, weight)| value * weight)
        .sum::<f64>()
        / denominator
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::stat_catalog_variants::skater_modern;
    use crate::identity::PlayerId;

    fn player(
        player_id: u32,
        hits: u32,
        blocks: u32,
        takeaways: u32,
        giveaways: u32,
    ) -> PlayerRoleSeasonFactsInput {
        PlayerRoleSeasonFactsInput {
            player_id,
            is_defenseman: false,
            games_played: 82,
            even_strength_toi_seconds: 60_000,
            short_handed_toi_seconds: 1_000,
            hits,
            blocked_shots: blocks,
            takeaways,
            giveaways,
            offensive_zone_start_pct: Some(50.0),
        }
    }

    #[test]
    fn player_roles_reward_physical_pressure_and_puck_security() {
        let rows = build_player_matchup_role_evidence(&[
            player(1, 220, 80, 60, 20),
            player(2, 20, 30, 20, 90),
        ])
        .unwrap();
        assert!(rows[0].role.physical_score > rows[1].role.physical_score);
        assert!(rows[0].role.transition_score > rows[1].role.transition_score);
        assert_eq!(rows[0].role.evidence_games, 82);
    }

    #[test]
    fn style_classifier_keeps_low_coverage_as_no_read() {
        let facts = vec![
            TeamStyleSeasonFactsInput {
                team: "NYR".to_owned(),
                season: 20252026,
                games_played: 82,
                event_coverage: 1.0,
                rush_attempts: 500,
                east_west_sequences: 100,
                dump_ins: 100,
                forecheck_recoveries: 40,
                cycle_sequences: 100,
                counterattack_attempts: 100,
            },
            TeamStyleSeasonFactsInput {
                team: "SEA".to_owned(),
                season: 20252026,
                games_played: 8,
                event_coverage: 0.40,
                rush_attempts: 10,
                east_west_sequences: 10,
                dump_ins: 10,
                forecheck_recoveries: 10,
                cycle_sequences: 10,
                counterattack_attempts: 10,
            },
        ];
        let rows = build_opponent_style_evidence(&facts).unwrap();
        assert_eq!(rows[0].style, Some(OpponentTacticalStyle::NorthSouthRush));
        assert_eq!(rows[1].style, None);
        assert!(!rows[1].warnings.is_empty());
    }

    #[test]
    fn tied_archetypes_are_reported_as_balanced() {
        let facts = ["NYR", "SEA"]
            .into_iter()
            .map(|team| TeamStyleSeasonFactsInput {
                team: team.to_owned(),
                season: 20252026,
                games_played: 82,
                event_coverage: 1.0,
                rush_attempts: 100,
                east_west_sequences: 100,
                dump_ins: 100,
                forecheck_recoveries: 100,
                cycle_sequences: 100,
                counterattack_attempts: 100,
            })
            .collect::<Vec<_>>();
        let rows = build_opponent_style_evidence(&facts).unwrap();
        assert!(rows
            .iter()
            .all(|row| row.style == Some(OpponentTacticalStyle::Balanced)));
    }

    fn repository_player(
        player_id: u32,
        position: Position,
        team: &str,
        complete: bool,
    ) -> (
        crate::identity::PlayerIdentity,
        crate::season_stats::SeasonStats,
    ) {
        let (mut identity, mut stats) = skater_modern();
        identity.id = PlayerId(player_id);
        identity.full_name = format!("Player {player_id}");
        identity.name_normalized = format!("player {player_id}");
        stats.player_id = PlayerId(player_id);
        stats.position = position;
        stats.team_stints[0].team = TeamAbbr(team.to_owned());
        if !complete {
            stats.time_on_ice = None;
        }
        (identity, stats)
    }

    #[test]
    fn repository_adapter_rates_current_roster_against_league_peers() {
        let mut repository = StatsRepository::new();
        for (id, position, team, complete) in [
            (1, Position::Center, "NYR", true),
            (2, Position::Defense, "NYR", true),
            (3, Position::LeftWing, "SEA", true),
            (4, Position::Defense, "SEA", true),
            (5, Position::RightWing, "NYR", false),
        ] {
            let (identity, stats) = repository_player(id, position, team, complete);
            repository.upsert_identity(identity).unwrap();
            repository.upsert_stats(stats).unwrap();
        }
        repository.set_current_roster(
            TeamAbbr("NYR".to_owned()),
            Season(20242025),
            SeasonType::Regular,
            [PlayerId(1), PlayerId(2), PlayerId(5)],
        );

        let view = build_team_player_matchup_role_evidence(
            &repository,
            &TeamAbbr("NYR".to_owned()),
            Season(20242025),
            SeasonType::Regular,
        )
        .unwrap();
        assert_eq!(view.roster_skaters, 3);
        assert_eq!(view.rated_skaters, 2);
        assert_eq!(view.league_forward_peers, 2);
        assert_eq!(view.league_defense_peers, 2);
        assert!(view
            .roles
            .iter()
            .all(|row| [1, 2].contains(&row.role.player_id)));
        assert!(view.warnings[0].contains('5'));
    }
}
