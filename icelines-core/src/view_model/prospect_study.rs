use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const PROSPECT_DEVELOPMENT_STUDY_SCHEMA: &str = "prospect_development_study.v1";
pub const PROSPECT_DISCOVERY_BOARD_SCHEMA: &str = "prospect_discovery_board.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectOpportunityStatus {
    None,
    Monitoring,
    RecallCandidate,
    DebutPlanned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectAvailabilityStatus {
    Healthy,
    InjuryInterrupted,
    Recovered,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectTrajectory {
    Rising,
    Stable,
    Cooling,
    Insufficient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectHiddenValueClass {
    InjuryObscuredRiser,
    InjuryRecoveryWatch,
    HiddenRiser,
    VisibleRiser,
    Watch,
    Cooling,
    OverexposedCooling,
    HypeAheadOfEvidence,
    SmallSampleHypeRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectMarketPosition {
    Underrecognized,
    Aligned,
    Overexposed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectDiscoveryLensKind {
    ProductionRiser,
    InjuryObscured,
    RecoveryUnproven,
    OpportunityBacked,
    AttentionLag,
    AttentionAheadOfEvidence,
    WorkloadUncertain,
    CoolingSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectDiscoveryLensDirection {
    Upside,
    Risk,
    Context,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentSeasonInput {
    pub season: u32,
    pub league: String,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectStudyEvidenceInput {
    pub label: String,
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentStudyInput {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub age: u8,
    pub nhl_games_played: u32,
    pub seasons: Vec<ProspectDevelopmentSeasonInput>,
    pub opportunity: ProspectOpportunityStatus,
    pub availability: ProspectAvailabilityStatus,
    /// Explicitly authored 0..1 estimate. Zero means little public attention;
    /// one means extensive attention. `attention_basis` must explain it.
    pub attention_score: f64,
    pub attention_basis: String,
    #[serde(default)]
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentStudyConfig {
    /// Latest same-league points/game that represents a strong pro season.
    pub production_benchmark_ppg: f64,
    /// Same-league points/game gain that represents a clear rise.
    pub rising_delta_ppg: f64,
    /// Games required before production receives full workload confidence.
    pub full_confidence_games: u32,
    /// Both same-league seasons must reach this workload before trajectory is classified.
    pub minimum_comparison_games: u32,
    pub production_weight: f64,
    pub trajectory_weight: f64,
    pub opportunity_weight: f64,
    pub attention_gap_weight: f64,
}

impl Default for ProspectDevelopmentStudyConfig {
    fn default() -> Self {
        Self {
            production_benchmark_ppg: 0.8,
            rising_delta_ppg: 0.15,
            full_confidence_games: 40,
            minimum_comparison_games: 10,
            production_weight: 0.4,
            trajectory_weight: 0.3,
            opportunity_weight: 0.2,
            attention_gap_weight: 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentSeasonView {
    pub season: u32,
    pub league: String,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: f64,
    pub same_league_ppg_delta: Option<f64>,
    pub same_league_ppg_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectSignalComponentView {
    pub id: String,
    pub score: f64,
    pub weight: f64,
    pub weighted_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDiscoveryLensView {
    pub kind: ProspectDiscoveryLensKind,
    pub direction: ProspectDiscoveryLensDirection,
    pub strength: f64,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentStudyView {
    pub schema: String,
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub age: u8,
    pub nhl_games_played: u32,
    pub seasons: Vec<ProspectDevelopmentSeasonView>,
    pub trajectory: ProspectTrajectory,
    pub workload_confidence: f64,
    pub opportunity: ProspectOpportunityStatus,
    pub availability: ProspectAvailabilityStatus,
    pub attention_score: f64,
    pub attention_basis: String,
    pub performance_attention_gap: f64,
    pub market_position: ProspectMarketPosition,
    pub hidden_value_score: f64,
    pub classification: ProspectHiddenValueClass,
    pub components: Vec<ProspectSignalComponentView>,
    pub lenses: Vec<ProspectDiscoveryLensView>,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectDiscoveryBoardLane {
    HiddenGem,
    BuyerBeware,
    Watch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDiscoveryBoardRow {
    pub rank: usize,
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub lane: ProspectDiscoveryBoardLane,
    pub classification: ProspectHiddenValueClass,
    pub market_position: ProspectMarketPosition,
    pub hidden_value_score: f64,
    pub performance_attention_gap: f64,
    /// Lane-relative 0..100 ordering signal. Hidden gems use hidden value;
    /// buyer-beware rows use their strongest supported risk signal.
    pub lane_score: f64,
    pub lenses: Vec<ProspectDiscoveryLensView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDiscoveryBoardView {
    pub schema: String,
    pub studies: usize,
    pub hidden_gems: Vec<ProspectDiscoveryBoardRow>,
    pub buyer_beware: Vec<ProspectDiscoveryBoardRow>,
    pub watch: Vec<ProspectDiscoveryBoardRow>,
    pub disclosures: Vec<String>,
}

pub fn build_prospect_discovery_board(
    studies: Vec<ProspectDevelopmentStudyView>,
) -> Result<ProspectDiscoveryBoardView, String> {
    if studies.is_empty() {
        return Err("prospect discovery board requires at least one study".to_owned());
    }

    let mut player_ids = BTreeSet::new();
    let mut hidden_gems = Vec::new();
    let mut buyer_beware = Vec::new();
    let mut watch = Vec::new();
    for study in studies {
        if study.schema != PROSPECT_DEVELOPMENT_STUDY_SCHEMA
            || study.player_id == 0
            || study.player.trim().is_empty()
            || study.organization.trim().is_empty()
            || study.position.trim().is_empty()
            || !study.hidden_value_score.is_finite()
            || !(0.0..=100.0).contains(&study.hidden_value_score)
            || !study.performance_attention_gap.is_finite()
            || study.lenses.iter().any(|lens| {
                !lens.strength.is_finite()
                    || !(0.0..=1.0).contains(&lens.strength)
                    || lens.summary.trim().is_empty()
            })
        {
            return Err("invalid prospect development study supplied to board".to_owned());
        }
        if !player_ids.insert(study.player_id) {
            return Err(format!(
                "duplicate prospect development study for player {}",
                study.player_id
            ));
        }

        let strongest_risk = study
            .lenses
            .iter()
            .filter(|lens| lens.direction == ProspectDiscoveryLensDirection::Risk)
            .map(|lens| lens.strength)
            .fold(0.0_f64, f64::max);
        let has_upside = study
            .lenses
            .iter()
            .any(|lens| lens.direction == ProspectDiscoveryLensDirection::Upside);
        let buyer_beware_signal = study.market_position == ProspectMarketPosition::Overexposed
            || matches!(
                study.classification,
                ProspectHiddenValueClass::OverexposedCooling
                    | ProspectHiddenValueClass::HypeAheadOfEvidence
                    | ProspectHiddenValueClass::SmallSampleHypeRisk
            );
        let hidden_gem_signal =
            matches!(
                study.classification,
                ProspectHiddenValueClass::InjuryObscuredRiser
                    | ProspectHiddenValueClass::InjuryRecoveryWatch
                    | ProspectHiddenValueClass::HiddenRiser
            ) || (study.market_position == ProspectMarketPosition::Underrecognized && has_upside);
        let lane = if buyer_beware_signal {
            ProspectDiscoveryBoardLane::BuyerBeware
        } else if hidden_gem_signal {
            ProspectDiscoveryBoardLane::HiddenGem
        } else {
            ProspectDiscoveryBoardLane::Watch
        };
        let lane_score = match lane {
            ProspectDiscoveryBoardLane::BuyerBeware => {
                strongest_risk.max((-study.performance_attention_gap).clamp(0.0, 1.0)) * 100.0
            }
            ProspectDiscoveryBoardLane::HiddenGem | ProspectDiscoveryBoardLane::Watch => {
                study.hidden_value_score
            }
        };
        let row = ProspectDiscoveryBoardRow {
            rank: 0,
            player_id: study.player_id,
            player: study.player,
            organization: study.organization,
            position: study.position,
            lane,
            classification: study.classification,
            market_position: study.market_position,
            hidden_value_score: study.hidden_value_score,
            performance_attention_gap: study.performance_attention_gap,
            lane_score,
            lenses: study.lenses,
        };
        match lane {
            ProspectDiscoveryBoardLane::HiddenGem => hidden_gems.push(row),
            ProspectDiscoveryBoardLane::BuyerBeware => buyer_beware.push(row),
            ProspectDiscoveryBoardLane::Watch => watch.push(row),
        }
    }

    for rows in [&mut hidden_gems, &mut buyer_beware, &mut watch] {
        rows.sort_by(|left, right| {
            right
                .lane_score
                .total_cmp(&left.lane_score)
                .then_with(|| left.player.cmp(&right.player))
                .then_with(|| left.player_id.cmp(&right.player_id))
        });
        for (index, row) in rows.iter_mut().enumerate() {
            row.rank = index + 1;
        }
    }

    Ok(ProspectDiscoveryBoardView {
        schema: PROSPECT_DISCOVERY_BOARD_SCHEMA.to_owned(),
        studies: player_ids.len(),
        hidden_gems,
        buyer_beware,
        watch,
        disclosures: vec![
            "The board composes validated prospect-development studies and preserves their active discovery lenses; it does not rescore raw source data.".to_owned(),
            "Hidden Gems require supported upside plus underrecognition or a hidden-value classification. Buyer Beware requires overexposure or an explicit hype/cooling classification.".to_owned(),
            "Uncertain workload alone remains Watch and is never treated as negative evidence.".to_owned(),
            "Lane scores rank candidates within a lane and are not comparable across lanes.".to_owned(),
        ],
    })
}

pub fn build_prospect_development_study(
    mut input: ProspectDevelopmentStudyInput,
    config: ProspectDevelopmentStudyConfig,
) -> Result<ProspectDevelopmentStudyView, String> {
    let weight_sum = config.production_weight
        + config.trajectory_weight
        + config.opportunity_weight
        + config.attention_gap_weight;
    let weights = [
        config.production_weight,
        config.trajectory_weight,
        config.opportunity_weight,
        config.attention_gap_weight,
    ];
    if input.player_id == 0
        || input.player.trim().is_empty()
        || input.organization.trim().is_empty()
        || input.position.trim().is_empty()
        || input.seasons.len() < 2
        || !input.attention_score.is_finite()
        || !(0.0..=1.0).contains(&input.attention_score)
        || input.attention_basis.trim().is_empty()
        || !config.production_benchmark_ppg.is_finite()
        || config.production_benchmark_ppg <= 0.0
        || !config.rising_delta_ppg.is_finite()
        || config.rising_delta_ppg <= 0.0
        || config.full_confidence_games == 0
        || config.minimum_comparison_games == 0
        || config.minimum_comparison_games > config.full_confidence_games
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
        || !weight_sum.is_finite()
        || (weight_sum - 1.0).abs() > 1e-9
        || input.seasons.iter().any(|row| {
            row.season == 0
                || row.league.trim().is_empty()
                || row.games_played == 0
                || row.goals.saturating_add(row.assists) < row.goals
        })
        || input.evidence.iter().any(|item| {
            item.label.trim().is_empty()
                || !(item.source_url.starts_with("https://")
                    || item.source_url.starts_with("http://"))
        })
    {
        return Err("invalid prospect development study input or configuration".to_owned());
    }
    input.seasons.sort_by_key(|row| row.season);
    if input
        .seasons
        .windows(2)
        .any(|rows| rows[0].season == rows[1].season)
    {
        return Err("prospect development seasons must be unique".to_owned());
    }

    let mut seasons = Vec::with_capacity(input.seasons.len());
    for (index, row) in input.seasons.iter().enumerate() {
        let points = row.goals.saturating_add(row.assists);
        let ppg = f64::from(points) / f64::from(row.games_played);
        let prior = (row.games_played >= config.minimum_comparison_games)
            .then(|| {
                input.seasons[..index].iter().rev().find(|prior| {
                    prior.league.eq_ignore_ascii_case(&row.league)
                        && prior.games_played >= config.minimum_comparison_games
                })
            })
            .flatten();
        let prior_ppg = prior.map(|prior| {
            f64::from(prior.goals.saturating_add(prior.assists)) / f64::from(prior.games_played)
        });
        let delta = prior_ppg.map(|prior| ppg - prior);
        seasons.push(ProspectDevelopmentSeasonView {
            season: row.season,
            league: row.league.clone(),
            games_played: row.games_played,
            goals: row.goals,
            assists: row.assists,
            points,
            points_per_game: ppg,
            same_league_ppg_delta: delta,
            same_league_ppg_change: prior_ppg
                .filter(|prior| *prior > 0.0)
                .map(|prior| delta.unwrap_or(0.0) / prior),
        });
    }

    let latest = seasons.last().expect("validated seasons");
    let delta = latest.same_league_ppg_delta;
    let trajectory = match delta {
        Some(value) if value >= config.rising_delta_ppg => ProspectTrajectory::Rising,
        Some(value) if value <= -config.rising_delta_ppg => ProspectTrajectory::Cooling,
        Some(_) => ProspectTrajectory::Stable,
        None => ProspectTrajectory::Insufficient,
    };
    let latest_confidence =
        (f64::from(latest.games_played) / f64::from(config.full_confidence_games)).min(1.0);
    let prior_confidence = input.seasons[..input.seasons.len() - 1]
        .iter()
        .rev()
        .find(|row| row.league.eq_ignore_ascii_case(&latest.league))
        .map(|row| (f64::from(row.games_played) / f64::from(config.full_confidence_games)).min(1.0))
        .unwrap_or(0.0);
    let workload_confidence = latest_confidence.min(prior_confidence);
    let production_score = (latest.points_per_game / config.production_benchmark_ppg)
        .clamp(0.0, 1.0)
        * latest_confidence;
    let trajectory_score = delta
        .map(|value| {
            (0.5 + value / (2.0 * config.rising_delta_ppg)).clamp(0.0, 1.0) * workload_confidence
        })
        .unwrap_or(0.0);
    let opportunity_score = match input.opportunity {
        ProspectOpportunityStatus::None => 0.0,
        ProspectOpportunityStatus::Monitoring => 0.35,
        ProspectOpportunityStatus::RecallCandidate => 0.7,
        ProspectOpportunityStatus::DebutPlanned => 1.0,
    };
    let attention_gap_score = 1.0 - input.attention_score;
    let component_values = [
        ("production", production_score, config.production_weight),
        ("trajectory", trajectory_score, config.trajectory_weight),
        ("opportunity", opportunity_score, config.opportunity_weight),
        (
            "attention_gap",
            attention_gap_score,
            config.attention_gap_weight,
        ),
    ];
    let components = component_values
        .iter()
        .map(|(id, score, weight)| ProspectSignalComponentView {
            id: (*id).to_owned(),
            score: *score,
            weight: *weight,
            weighted_points: score * weight * 100.0,
        })
        .collect::<Vec<_>>();
    let hidden_value_score = components
        .iter()
        .map(|component| component.weighted_points)
        .sum::<f64>();
    let performance_attention_gap =
        ((production_score + trajectory_score + opportunity_score) / 3.0) - input.attention_score;
    let market_position = if performance_attention_gap >= 0.2 {
        ProspectMarketPosition::Underrecognized
    } else if performance_attention_gap <= -0.2 {
        ProspectMarketPosition::Overexposed
    } else {
        ProspectMarketPosition::Aligned
    };
    let low_attention = input.attention_score <= 0.4;
    let high_attention = input.attention_score >= 0.65;
    let classification = if trajectory == ProspectTrajectory::Rising
        && low_attention
        && input.availability == ProspectAvailabilityStatus::InjuryInterrupted
        && input.opportunity == ProspectOpportunityStatus::DebutPlanned
    {
        ProspectHiddenValueClass::InjuryObscuredRiser
    } else if trajectory == ProspectTrajectory::Rising
        && low_attention
        && hidden_value_score >= 70.0
    {
        ProspectHiddenValueClass::HiddenRiser
    } else if trajectory == ProspectTrajectory::Rising {
        ProspectHiddenValueClass::VisibleRiser
    } else if input.availability == ProspectAvailabilityStatus::Recovered
        && matches!(
            input.opportunity,
            ProspectOpportunityStatus::RecallCandidate | ProspectOpportunityStatus::DebutPlanned
        )
        && production_score >= 0.75
    {
        ProspectHiddenValueClass::InjuryRecoveryWatch
    } else if trajectory == ProspectTrajectory::Cooling && high_attention {
        ProspectHiddenValueClass::OverexposedCooling
    } else if trajectory == ProspectTrajectory::Insufficient
        && latest_confidence < 0.75
        && high_attention
    {
        ProspectHiddenValueClass::SmallSampleHypeRisk
    } else if market_position == ProspectMarketPosition::Overexposed
        && production_score < 0.65
        && matches!(
            input.opportunity,
            ProspectOpportunityStatus::None | ProspectOpportunityStatus::Monitoring
        )
    {
        ProspectHiddenValueClass::HypeAheadOfEvidence
    } else if trajectory == ProspectTrajectory::Cooling {
        ProspectHiddenValueClass::Cooling
    } else {
        ProspectHiddenValueClass::Watch
    };
    let mut lenses = Vec::new();
    if trajectory == ProspectTrajectory::Rising {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::ProductionRiser,
            direction: ProspectDiscoveryLensDirection::Upside,
            strength: trajectory_score,
            summary: "Same-league scoring rate cleared the configured rising threshold.".to_owned(),
        });
    }
    if input.availability == ProspectAvailabilityStatus::InjuryInterrupted {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::InjuryObscured,
            direction: ProspectDiscoveryLensDirection::Context,
            strength: opportunity_score,
            summary: "Injury interrupted documented opportunity; it does not reduce the development signal or add score.".to_owned(),
        });
    }
    if input.availability == ProspectAvailabilityStatus::Recovered
        && trajectory == ProspectTrajectory::Insufficient
    {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::RecoveryUnproven,
            direction: ProspectDiscoveryLensDirection::Context,
            strength: 1.0 - workload_confidence,
            summary: "The return is productive, but the injured comparison season is too small to prove a trend.".to_owned(),
        });
    }
    if matches!(
        input.opportunity,
        ProspectOpportunityStatus::RecallCandidate | ProspectOpportunityStatus::DebutPlanned
    ) {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::OpportunityBacked,
            direction: ProspectDiscoveryLensDirection::Upside,
            strength: opportunity_score,
            summary: "Documented recall or debut intent supports the performance signal."
                .to_owned(),
        });
    }
    if market_position == ProspectMarketPosition::Underrecognized {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::AttentionLag,
            direction: ProspectDiscoveryLensDirection::Upside,
            strength: performance_attention_gap.clamp(0.0, 1.0),
            summary: "The authored attention estimate trails the combined performance and opportunity evidence.".to_owned(),
        });
    } else if market_position == ProspectMarketPosition::Overexposed {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::AttentionAheadOfEvidence,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: (-performance_attention_gap).clamp(0.0, 1.0),
            summary: "The authored attention estimate is ahead of the combined performance and opportunity evidence.".to_owned(),
        });
    }
    if workload_confidence < 0.75 {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::WorkloadUncertain,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: 1.0 - workload_confidence,
            summary: "The comparable same-league workload is below the confidence gate.".to_owned(),
        });
    }
    if trajectory == ProspectTrajectory::Cooling {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::CoolingSignal,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: (0.5 - trajectory_score).max(0.0) * 2.0,
            summary: "Same-league scoring rate declined beyond the configured cooling threshold."
                .to_owned(),
        });
    }

    Ok(ProspectDevelopmentStudyView {
        schema: PROSPECT_DEVELOPMENT_STUDY_SCHEMA.to_owned(),
        player_id: input.player_id,
        player: input.player,
        organization: input.organization,
        position: input.position,
        age: input.age,
        nhl_games_played: input.nhl_games_played,
        seasons,
        trajectory,
        workload_confidence,
        opportunity: input.opportunity,
        availability: input.availability,
        attention_score: input.attention_score,
        attention_basis: input.attention_basis,
        performance_attention_gap,
        market_position,
        hidden_value_score,
        classification,
        components,
        lenses,
        evidence: input.evidence,
        disclosures: vec![
            "The hidden-value score combines latest production, same-league trajectory, documented opportunity, and an explicitly authored attention estimate; it is a discovery signal, not an NHL-equivalency projection.".to_owned(),
            "Injury explains interrupted opportunity but does not add points to the score; availability remains a separate labeled state.".to_owned(),
            "Raw scoring is compared only with an earlier season in the same league, preventing junior-to-pro league changes from masquerading as development decline.".to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firkus_is_an_injury_obscured_riser_with_transparent_components() {
        let view = build_prospect_development_study(
            ProspectDevelopmentStudyInput {
                player_id: 8_483_442,
                player: "Jagger Firkus".to_owned(),
                organization: "SEA".to_owned(),
                position: "RW".to_owned(),
                age: 22,
                nhl_games_played: 0,
                seasons: vec![
                    ProspectDevelopmentSeasonInput {
                        season: 20242025,
                        league: "AHL".to_owned(),
                        games_played: 69,
                        goals: 15,
                        assists: 21,
                    },
                    ProspectDevelopmentSeasonInput {
                        season: 20252026,
                        league: "AHL".to_owned(),
                        games_played: 63,
                        goals: 21,
                        assists: 35,
                    },
                ],
                opportunity: ProspectOpportunityStatus::DebutPlanned,
                availability: ProspectAvailabilityStatus::InjuryInterrupted,
                attention_score: 0.2,
                attention_basis: "Low public visibility after missing his planned NHL debut."
                    .to_owned(),
                evidence: vec![ProspectStudyEvidenceInput {
                    label: "Kraken GM said injury prevented a planned NHL debut.".to_owned(),
                    source_url: "https://www.nhl.com/kraken/news/2026-nhl-draft-behind-the-scenes-seattle-kraken-draft-room".to_owned(),
                }],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();

        assert_eq!(view.trajectory, ProspectTrajectory::Rising);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::InjuryObscuredRiser
        );
        assert!((view.seasons[1].points_per_game - 56.0 / 63.0).abs() < 1e-9);
        assert!(view.seasons[1].same_league_ppg_change.unwrap() > 0.70);
        assert!(view.hidden_value_score > 90.0);
        assert_eq!(view.components.len(), 4);
        assert_eq!(
            view.market_position,
            ProspectMarketPosition::Underrecognized
        );
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::InjuryObscured));
    }

    #[test]
    fn cross_league_seasons_do_not_invent_a_trajectory() {
        let input = ProspectDevelopmentStudyInput {
            player_id: 1,
            player: "Prospect".to_owned(),
            organization: "SEA".to_owned(),
            position: "C".to_owned(),
            age: 20,
            nhl_games_played: 0,
            seasons: vec![
                ProspectDevelopmentSeasonInput {
                    season: 20232024,
                    league: "WHL".to_owned(),
                    games_played: 60,
                    goals: 40,
                    assists: 50,
                },
                ProspectDevelopmentSeasonInput {
                    season: 20242025,
                    league: "AHL".to_owned(),
                    games_played: 60,
                    goals: 12,
                    assists: 18,
                },
            ],
            opportunity: ProspectOpportunityStatus::Monitoring,
            availability: ProspectAvailabilityStatus::Healthy,
            attention_score: 0.5,
            attention_basis: "Estimated from current coverage.".to_owned(),
            evidence: Vec::new(),
        };
        let view =
            build_prospect_development_study(input, ProspectDevelopmentStudyConfig::default())
                .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Insufficient);
        assert_eq!(view.seasons[1].same_league_ppg_delta, None);
    }

    #[test]
    fn tiny_injury_season_does_not_invent_a_recovery_decline() {
        let input = ProspectDevelopmentStudyInput {
            player_id: 8_482_162,
            player: "Roby Jarventie".to_owned(),
            organization: "EDM".to_owned(),
            position: "LW".to_owned(),
            age: 23,
            nhl_games_played: 10,
            seasons: vec![
                ProspectDevelopmentSeasonInput {
                    season: 20242025,
                    league: "AHL".to_owned(),
                    games_played: 2,
                    goals: 0,
                    assists: 2,
                },
                ProspectDevelopmentSeasonInput {
                    season: 20252026,
                    league: "AHL".to_owned(),
                    games_played: 52,
                    goals: 17,
                    assists: 19,
                },
            ],
            opportunity: ProspectOpportunityStatus::RecallCandidate,
            availability: ProspectAvailabilityStatus::Recovered,
            attention_score: 0.25,
            attention_basis: "Low visibility after two long-term injuries.".to_owned(),
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Oilers documented two long-term injury setbacks and his recall."
                    .to_owned(),
                source_url: "https://www.nhl.com/oilers/news/blog-jarventie-ready-for-second-nhl-opportunity-after-overcoming-injuries".to_owned(),
            }],
        };
        let view =
            build_prospect_development_study(input, ProspectDevelopmentStudyConfig::default())
                .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Insufficient);
        assert_eq!(view.seasons[1].same_league_ppg_delta, None);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::InjuryRecoveryWatch
        );
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::RecoveryUnproven));
    }

    #[test]
    fn attention_on_a_tiny_flash_is_flagged_as_hype_risk() {
        let view = build_prospect_development_study(
            study_input(
                5,
                6,
                4,
                10,
                ProspectOpportunityStatus::Monitoring,
                ProspectAvailabilityStatus::Healthy,
                0.85,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Insufficient);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::SmallSampleHypeRisk
        );
        assert_eq!(view.market_position, ProspectMarketPosition::Overexposed);
        assert!(view
            .lenses
            .iter()
            .any(|lens| { lens.kind == ProspectDiscoveryLensKind::AttentionAheadOfEvidence }));
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::WorkloadUncertain));
    }

    #[test]
    fn high_attention_and_real_decline_is_overexposed_cooling() {
        let view = build_prospect_development_study(
            study_input(
                60,
                60,
                60,
                30,
                ProspectOpportunityStatus::Monitoring,
                ProspectAvailabilityStatus::Healthy,
                0.8,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Cooling);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::OverexposedCooling
        );
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::CoolingSignal));
    }

    #[test]
    fn discovery_board_separates_upside_risk_and_uncertainty() {
        let mut hidden = build_prospect_development_study(
            study_input(
                60,
                30,
                60,
                60,
                ProspectOpportunityStatus::RecallCandidate,
                ProspectAvailabilityStatus::Healthy,
                0.2,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        hidden.player_id = 1;
        hidden.player = "Hidden Prospect".to_owned();

        let mut buyer = build_prospect_development_study(
            study_input(
                60,
                60,
                60,
                30,
                ProspectOpportunityStatus::Monitoring,
                ProspectAvailabilityStatus::Healthy,
                0.8,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        buyer.player_id = 2;
        buyer.player = "Overexposed Prospect".to_owned();

        let mut uncertain = hidden.clone();
        uncertain.player_id = 3;
        uncertain.player = "Uncertain Prospect".to_owned();
        uncertain.classification = ProspectHiddenValueClass::Watch;
        uncertain.market_position = ProspectMarketPosition::Aligned;
        uncertain.performance_attention_gap = 0.0;
        uncertain.lenses = vec![ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::WorkloadUncertain,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: 0.8,
            summary: "The comparable workload is uncertain.".to_owned(),
        }];

        let mut lower_hidden = hidden.clone();
        lower_hidden.player_id = 4;
        lower_hidden.player = "Lower Hidden Prospect".to_owned();
        lower_hidden.hidden_value_score -= 10.0;

        let board =
            build_prospect_discovery_board(vec![uncertain, lower_hidden, buyer, hidden]).unwrap();
        assert_eq!(board.schema, PROSPECT_DISCOVERY_BOARD_SCHEMA);
        assert_eq!(board.studies, 4);
        assert_eq!(board.hidden_gems[0].player, "Hidden Prospect");
        assert_eq!(board.hidden_gems[0].rank, 1);
        assert_eq!(board.hidden_gems[1].player, "Lower Hidden Prospect");
        assert_eq!(board.hidden_gems[1].rank, 2);
        assert_eq!(board.buyer_beware[0].player, "Overexposed Prospect");
        assert_eq!(board.buyer_beware[0].rank, 1);
        assert_eq!(board.watch[0].player, "Uncertain Prospect");
        assert_eq!(board.watch[0].rank, 1);
    }

    #[test]
    fn discovery_board_rejects_duplicate_player_studies() {
        let study = build_prospect_development_study(
            study_input(
                60,
                30,
                60,
                60,
                ProspectOpportunityStatus::RecallCandidate,
                ProspectAvailabilityStatus::Healthy,
                0.2,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();

        let error = build_prospect_discovery_board(vec![study.clone(), study]).unwrap_err();
        assert!(error.contains("duplicate"));
    }

    fn study_input(
        prior_games: u32,
        prior_points: u32,
        latest_games: u32,
        latest_points: u32,
        opportunity: ProspectOpportunityStatus,
        availability: ProspectAvailabilityStatus,
        attention_score: f64,
    ) -> ProspectDevelopmentStudyInput {
        ProspectDevelopmentStudyInput {
            player_id: 9,
            player: "Test Prospect".to_owned(),
            organization: "TST".to_owned(),
            position: "C".to_owned(),
            age: 21,
            nhl_games_played: 0,
            seasons: vec![
                ProspectDevelopmentSeasonInput {
                    season: 20242025,
                    league: "AHL".to_owned(),
                    games_played: prior_games,
                    goals: prior_points / 2,
                    assists: prior_points - prior_points / 2,
                },
                ProspectDevelopmentSeasonInput {
                    season: 20252026,
                    league: "AHL".to_owned(),
                    games_played: latest_games,
                    goals: latest_points / 2,
                    assists: latest_points - latest_points / 2,
                },
            ],
            opportunity,
            availability,
            attention_score,
            attention_basis: "Test attention basis.".to_owned(),
            evidence: Vec::new(),
        }
    }
}
