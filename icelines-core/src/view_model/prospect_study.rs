use serde::{Deserialize, Serialize};

pub const PROSPECT_DEVELOPMENT_STUDY_SCHEMA: &str = "prospect_development_study.v1";

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
    pub hidden_value_score: f64,
    pub classification: ProspectHiddenValueClass,
    pub components: Vec<ProspectSignalComponentView>,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
    pub disclosures: Vec<String>,
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
    let low_attention = input.attention_score <= 0.4;
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
    } else if trajectory == ProspectTrajectory::Cooling {
        ProspectHiddenValueClass::Cooling
    } else {
        ProspectHiddenValueClass::Watch
    };

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
        hidden_value_score,
        classification,
        components,
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
    }
}
