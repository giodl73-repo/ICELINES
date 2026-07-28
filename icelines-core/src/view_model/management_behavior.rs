//! Renderer-neutral GM, manager, and opponent-specific deployment behavior.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::team_lineup::{TeamLineupPlayerView, TeamLineupProjectionView};
use super::EvidenceLabel;

pub const TEAM_DECISION_PROFILE_SCHEMA: &str = "team_decision_profile.v1";
pub const TEAM_BEHAVIOR_CALIBRATION_SCHEMA: &str = "team_behavior_calibration.v1";
pub const TEAM_BEHAVIOR_RANKING_SCHEMA: &str = "team_behavior_ranking.v1";
pub const TEAM_BEHAVIOR_RESEARCH_SCHEMA: &str = "team_behavior_research.v1";
pub const BENCH_GAME_PLAN_SCHEMA: &str = "bench_game_plan.v1";
pub const SCHEDULE_REST_PROFILE_SCHEMA: &str = "schedule_rest_profile.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorTraitView {
    /// Signed tendency from -1 (strongly avoids) through +1 (strongly favors).
    pub value: f64,
    pub evidence_games: u32,
    pub evidence_label: EvidenceLabel,
}

impl BehaviorTraitView {
    pub fn effective_value(&self) -> f64 {
        let games = f64::from(self.evidence_games);
        self.value * games / (games + 20.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralManagerBehaviorProfile {
    pub rookie_opportunity: BehaviorTraitView,
    pub veteran_preference: BehaviorTraitView,
    pub waiver_asset_protection: BehaviorTraitView,
    pub trade_aggression: BehaviorTraitView,
    pub deadline_buying_bias: BehaviorTraitView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagerBehaviorProfile {
    pub matchup_intensity: BehaviorTraitView,
    pub tactical_adaptability: BehaviorTraitView,
    pub lineup_patience: BehaviorTraitView,
    pub position_flexibility: BehaviorTraitView,
    pub physical_fourth_line_preference: BehaviorTraitView,
    pub four_line_usage: BehaviorTraitView,
    pub fatigue_rotation: BehaviorTraitView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamDecisionProfile {
    pub schema: String,
    pub id: String,
    pub team: String,
    pub season: u32,
    pub general_manager: GeneralManagerBehaviorProfile,
    pub manager: ManagerBehaviorProfile,
    #[serde(default)]
    pub disclosures: Vec<String>,
}

impl TeamDecisionProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != TEAM_DECISION_PROFILE_SCHEMA {
            return Err("team decision profile has an unsupported schema".to_owned());
        }
        if self.id.trim().is_empty() || !valid_team(&self.team) {
            return Err("team decision profile requires an id and NHL team".to_owned());
        }
        for trait_value in self.traits() {
            if !trait_value.value.is_finite() || !(-1.0..=1.0).contains(&trait_value.value) {
                return Err("behavior trait values must be finite and between -1 and 1".to_owned());
            }
        }
        Ok(())
    }

    fn traits(&self) -> [&BehaviorTraitView; 12] {
        [
            &self.general_manager.rookie_opportunity,
            &self.general_manager.veteran_preference,
            &self.general_manager.waiver_asset_protection,
            &self.general_manager.trade_aggression,
            &self.general_manager.deadline_buying_bias,
            &self.manager.matchup_intensity,
            &self.manager.tactical_adaptability,
            &self.manager.lineup_patience,
            &self.manager.position_flexibility,
            &self.manager.physical_fourth_line_preference,
            &self.manager.four_line_usage,
            &self.manager.fatigue_rotation,
        ]
    }

    fn trait_mut(&mut self, key: &str) -> Option<&mut BehaviorTraitView> {
        match key {
            "rookie_opportunity" => Some(&mut self.general_manager.rookie_opportunity),
            "veteran_preference" => Some(&mut self.general_manager.veteran_preference),
            "waiver_asset_protection" => Some(&mut self.general_manager.waiver_asset_protection),
            "trade_aggression" => Some(&mut self.general_manager.trade_aggression),
            "deadline_buying_bias" => Some(&mut self.general_manager.deadline_buying_bias),
            "matchup_intensity" => Some(&mut self.manager.matchup_intensity),
            "tactical_adaptability" => Some(&mut self.manager.tactical_adaptability),
            "lineup_patience" => Some(&mut self.manager.lineup_patience),
            "position_flexibility" => Some(&mut self.manager.position_flexibility),
            "physical_fourth_line_preference" => {
                Some(&mut self.manager.physical_fourth_line_preference)
            }
            "four_line_usage" => Some(&mut self.manager.four_line_usage),
            "fatigue_rotation" => Some(&mut self.manager.fatigue_rotation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeadershipRole {
    GeneralManager,
    HeadCoach,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeadershipTenureInput {
    pub person_id: String,
    pub person_name: String,
    pub role: LeadershipRole,
    pub team: String,
    pub started_on: String,
    #[serde(default)]
    pub ended_on: Option<String>,
    pub source_url: String,
    pub source_title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorResearchMarkerInput {
    pub marker_id: String,
    pub person_id: String,
    pub role: LeadershipRole,
    pub trait_key: String,
    /// Signed directional marker from -1 through +1.
    pub value: f64,
    /// Editorial confidence from 0 through 1. Research never becomes confirmed.
    pub confidence: f64,
    pub published_on: String,
    pub source_url: String,
    pub source_title: String,
    /// IceLines-authored paraphrase; source quotations are not stored here.
    pub evidence_summary: String,
    #[serde(default)]
    pub observed_team: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorResearchInput {
    pub schema: String,
    pub team: String,
    pub target_season: u32,
    pub as_of: String,
    pub leadership: Vec<LeadershipTenureInput>,
    pub markers: Vec<BehaviorResearchMarkerInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorResearchMarkerDecisionView {
    pub marker_id: String,
    pub accepted: bool,
    pub reason: String,
    pub effective_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorResearchTraitView {
    pub trait_key: String,
    pub base_value: Option<f64>,
    pub researched_value: Option<f64>,
    pub blended_value: Option<f64>,
    pub marker_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorResearchView {
    pub schema: String,
    pub team: String,
    pub target_season: u32,
    pub as_of: String,
    pub active_general_manager: Option<LeadershipTenureInput>,
    pub active_head_coach: Option<LeadershipTenureInput>,
    pub base_profile: TeamDecisionProfile,
    pub enriched_profile: TeamDecisionProfile,
    pub marker_decisions: Vec<BehaviorResearchMarkerDecisionView>,
    pub traits: Vec<BehaviorResearchTraitView>,
    pub disclosures: Vec<String>,
}

/// Apply citation-backed leadership research without allowing prose markers to
/// overpower observed team behavior. Markers attach to the person currently in
/// the role, so a coaching or GM change automatically retires predecessor lore.
pub fn apply_team_behavior_research(
    base: &TeamDecisionProfile,
    input: &TeamBehaviorResearchInput,
) -> Result<TeamBehaviorResearchView, String> {
    use chrono::NaiveDate;

    base.validate()?;
    if input.schema != TEAM_BEHAVIOR_RESEARCH_SCHEMA
        || input.team != base.team
        || input.target_season != base.season
    {
        return Err("behavior research must match the base team and target season".to_owned());
    }
    let as_of = NaiveDate::parse_from_str(&input.as_of, "%Y-%m-%d")
        .map_err(|_| "behavior research requires an ISO as-of date".to_owned())?;
    let active = |role| -> Result<Option<LeadershipTenureInput>, String> {
        let matches = input
            .leadership
            .iter()
            .filter(|row| {
                row.team == input.team
                    && row.role == role
                    && tenure_contains(row, as_of).unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            Err("leadership timeline has overlapping active tenures".to_owned())
        } else {
            Ok(matches.into_iter().next())
        }
    };
    for tenure in &input.leadership {
        validate_tenure(tenure)?;
    }
    let active_general_manager = active(LeadershipRole::GeneralManager)?;
    let active_head_coach = active(LeadershipRole::HeadCoach)?;
    let mut enriched_profile = base.clone();
    enriched_profile.id = format!("{}-researched-{}", base.id, input.as_of);
    let mut marker_decisions = Vec::new();
    let mut accepted: BTreeMap<String, Vec<(&BehaviorResearchMarkerInput, f64)>> = BTreeMap::new();
    for marker in &input.markers {
        validate_marker(marker, as_of)?;
        let leader = match marker.role {
            LeadershipRole::GeneralManager => active_general_manager.as_ref(),
            LeadershipRole::HeadCoach => active_head_coach.as_ref(),
        };
        let role_accepts_trait = role_trait_keys(marker.role).contains(&marker.trait_key.as_str());
        let published = NaiveDate::parse_from_str(&marker.published_on, "%Y-%m-%d")
            .expect("validated marker date");
        let age_days = (as_of - published).num_days().max(0) as f64;
        let recency = (1.0 - age_days / (365.25 * 8.0)).clamp(0.15, 1.0);
        let weight = marker.confidence * recency;
        let (accepted_marker, reason) = if !role_accepts_trait {
            (
                false,
                "trait does not belong to the marker's leadership role",
            )
        } else if leader.is_none() {
            (false, "no active leader is established for this role")
        } else if leader.is_some_and(|row| row.person_id != marker.person_id) {
            (
                false,
                "marker belongs to a predecessor, not the active leader",
            )
        } else {
            (true, "citation-backed marker applies to the active leader")
        };
        if accepted_marker {
            accepted
                .entry(marker.trait_key.clone())
                .or_default()
                .push((marker, weight));
        }
        marker_decisions.push(BehaviorResearchMarkerDecisionView {
            marker_id: marker.marker_id.clone(),
            accepted: accepted_marker,
            reason: reason.to_owned(),
            effective_weight: if accepted_marker { weight } else { 0.0 },
        });
    }
    let mut traits = Vec::new();
    for key in behavior_trait_keys() {
        let base_trait = base
            .traits()
            .into_iter()
            .zip(behavior_trait_keys())
            .find_map(|(value, candidate)| (candidate == key).then_some(value))
            .expect("known trait key");
        let markers = accepted.get(key);
        let (researched_value, marker_weight, marker_ids) =
            markers.map_or((None, 0.0, Vec::new()), |rows| {
                let total_weight = rows.iter().map(|(_, weight)| weight).sum::<f64>();
                let value = (total_weight > 0.0).then(|| {
                    rows.iter()
                        .map(|(marker, weight)| marker.value * weight)
                        .sum::<f64>()
                        / total_weight
                });
                (
                    value,
                    total_weight,
                    rows.iter()
                        .map(|(marker, _)| marker.marker_id.clone())
                        .collect(),
                )
            });
        let base_value =
            (base_trait.evidence_label != EvidenceLabel::NoRead).then_some(base_trait.value);
        let blended_value = match (base_value, researched_value) {
            (Some(base_value), Some(researched_value)) => {
                let research_share = (marker_weight * 0.12).clamp(0.0, 0.25);
                Some(base_value * (1.0 - research_share) + researched_value * research_share)
            }
            (None, Some(researched_value)) => Some(researched_value),
            (Some(base_value), None) => Some(base_value),
            (None, None) => None,
        };
        if let Some(value) = blended_value {
            let target = enriched_profile.trait_mut(key).expect("known trait key");
            if base_value.is_none() {
                target.evidence_games = (marker_weight * 20.0).round().max(1.0) as u32;
                target.evidence_label = EvidenceLabel::Reported;
            }
            target.value = value.clamp(-1.0, 1.0);
        }
        traits.push(BehaviorResearchTraitView {
            trait_key: key.to_owned(),
            base_value,
            researched_value,
            blended_value,
            marker_ids,
        });
    }
    enriched_profile.disclosures.push(
        "Citation-backed leadership markers are person-bound, recency-decayed, and capped at a 25% share when observed behavior exists."
            .to_owned(),
    );
    Ok(TeamBehaviorResearchView {
        schema: TEAM_BEHAVIOR_RESEARCH_SCHEMA.to_owned(),
        team: input.team.clone(),
        target_season: input.target_season,
        as_of: input.as_of.clone(),
        active_general_manager,
        active_head_coach,
        base_profile: base.clone(),
        enriched_profile,
        marker_decisions,
        traits,
        disclosures: vec![
            "Research summaries are IceLines paraphrases with direct source URLs; quotations are not retained in the profile.".to_owned(),
            "A new GM or head coach retires predecessor markers automatically; the new leader may carry their own dated markers from prior teams.".to_owned(),
            "Reported leadership markers describe tendencies and controversies, not moral or performance scores.".to_owned(),
        ],
    })
}

fn role_trait_keys(role: LeadershipRole) -> &'static [&'static str] {
    match role {
        LeadershipRole::GeneralManager => &[
            "rookie_opportunity",
            "veteran_preference",
            "waiver_asset_protection",
            "trade_aggression",
            "deadline_buying_bias",
        ],
        LeadershipRole::HeadCoach => &[
            "matchup_intensity",
            "tactical_adaptability",
            "lineup_patience",
            "position_flexibility",
            "physical_fourth_line_preference",
            "four_line_usage",
            "fatigue_rotation",
        ],
    }
}

fn validate_tenure(tenure: &LeadershipTenureInput) -> Result<(), String> {
    use chrono::NaiveDate;
    let start = NaiveDate::parse_from_str(&tenure.started_on, "%Y-%m-%d")
        .map_err(|_| "leadership tenure requires an ISO start date".to_owned())?;
    let end = tenure
        .ended_on
        .as_deref()
        .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| "leadership tenure requires an ISO end date".to_owned())?;
    if tenure.person_id.trim().is_empty()
        || tenure.person_name.trim().is_empty()
        || !valid_team(&tenure.team)
        || !tenure.source_url.starts_with("https://")
        || tenure.source_title.trim().is_empty()
        || end.is_some_and(|end| end < start)
    {
        return Err(
            "leadership tenure requires valid identity, dates, team, and citation".to_owned(),
        );
    }
    Ok(())
}

fn tenure_contains(
    tenure: &LeadershipTenureInput,
    date: chrono::NaiveDate,
) -> Result<bool, String> {
    use chrono::NaiveDate;
    let start = NaiveDate::parse_from_str(&tenure.started_on, "%Y-%m-%d")
        .map_err(|_| "leadership tenure requires an ISO start date".to_owned())?;
    let end = tenure
        .ended_on
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| "leadership tenure requires an ISO end date".to_owned())?;
    Ok(date >= start && end.is_none_or(|end| date <= end))
}

fn validate_marker(
    marker: &BehaviorResearchMarkerInput,
    as_of: chrono::NaiveDate,
) -> Result<(), String> {
    use chrono::NaiveDate;
    let published = NaiveDate::parse_from_str(&marker.published_on, "%Y-%m-%d")
        .map_err(|_| "research marker requires an ISO publication date".to_owned())?;
    if marker.marker_id.trim().is_empty()
        || marker.person_id.trim().is_empty()
        || !marker.value.is_finite()
        || !(-1.0..=1.0).contains(&marker.value)
        || !marker.confidence.is_finite()
        || !(0.0..=1.0).contains(&marker.confidence)
        || published > as_of
        || !marker.source_url.starts_with("https://")
        || marker.source_title.trim().is_empty()
        || marker.evidence_summary.trim().is_empty()
    {
        return Err(
            "research marker requires bounded values, an as-of-safe date, and a citation"
                .to_owned(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorSignalObservation {
    /// League-relative signed observation from -1 through +1.
    pub value: f64,
    /// Decisions, games, or deployment opportunities supporting the signal.
    pub opportunities: u32,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelativeBehaviorCountFact {
    pub team_successes: u32,
    pub team_opportunities: u32,
    pub league_successes: u32,
    pub league_opportunities: u32,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorSeasonFactsInput {
    pub season: u32,
    #[serde(default)]
    pub rookie_opening_roster_decisions: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub veteran_retention_decisions: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub waiver_protection_decisions: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub trade_activity_opportunities: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub deadline_buy_decisions: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub hard_match_deployments: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub opponent_specific_adjustments: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub lineup_continuity_decisions: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub off_position_deployments: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub physical_fourth_line_deployments: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub balanced_four_line_games: Option<RelativeBehaviorCountFact>,
    #[serde(default)]
    pub fatigue_rotation_games: Option<RelativeBehaviorCountFact>,
}

pub fn build_team_behavior_season_observation(
    facts: &TeamBehaviorSeasonFactsInput,
) -> Result<TeamBehaviorSeasonObservation, String> {
    let convert = |fact: &Option<RelativeBehaviorCountFact>| {
        fact.as_ref().map(relative_behavior_signal).transpose()
    };
    Ok(TeamBehaviorSeasonObservation {
        season: facts.season,
        rookie_opportunity: convert(&facts.rookie_opening_roster_decisions)?,
        veteran_preference: convert(&facts.veteran_retention_decisions)?,
        waiver_asset_protection: convert(&facts.waiver_protection_decisions)?,
        trade_aggression: convert(&facts.trade_activity_opportunities)?,
        deadline_buying_bias: convert(&facts.deadline_buy_decisions)?,
        matchup_intensity: convert(&facts.hard_match_deployments)?,
        tactical_adaptability: convert(&facts.opponent_specific_adjustments)?,
        lineup_patience: convert(&facts.lineup_continuity_decisions)?,
        position_flexibility: convert(&facts.off_position_deployments)?,
        physical_fourth_line_preference: convert(&facts.physical_fourth_line_deployments)?,
        four_line_usage: convert(&facts.balanced_four_line_games)?,
        fatigue_rotation: convert(&facts.fatigue_rotation_games)?,
    })
}

fn relative_behavior_signal(
    fact: &RelativeBehaviorCountFact,
) -> Result<BehaviorSignalObservation, String> {
    if fact.team_opportunities == 0
        || fact.league_opportunities == 0
        || fact.team_successes > fact.team_opportunities
        || fact.league_successes > fact.league_opportunities
        || fact.evidence_label == EvidenceLabel::Simulated
    {
        return Err(
            "behavior count facts require valid non-simulated success/opportunity counts"
                .to_owned(),
        );
    }
    let team_rate = f64::from(fact.team_successes) / f64::from(fact.team_opportunities);
    let league_rate = f64::from(fact.league_successes) / f64::from(fact.league_opportunities);
    Ok(BehaviorSignalObservation {
        value: ((team_rate - league_rate) * 2.0).clamp(-1.0, 1.0),
        opportunities: fact.team_opportunities,
        evidence_label: fact.evidence_label,
    })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorSeasonObservation {
    pub season: u32,
    #[serde(default)]
    pub rookie_opportunity: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub veteran_preference: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub waiver_asset_protection: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub trade_aggression: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub deadline_buying_bias: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub matchup_intensity: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub tactical_adaptability: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub lineup_patience: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub position_flexibility: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub physical_fourth_line_preference: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub four_line_usage: Option<BehaviorSignalObservation>,
    #[serde(default)]
    pub fatigue_rotation: Option<BehaviorSignalObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorCalibrationInput {
    pub team: String,
    pub target_season: u32,
    /// One, two, or three completed seasons before `target_season`.
    pub window_seasons: u8,
    pub observations: Vec<TeamBehaviorSeasonObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorTraitCalibrationRow {
    pub trait_key: String,
    pub seasons_used: Vec<u32>,
    pub calibrated_value: f64,
    pub effective_value: f64,
    pub opportunities: u32,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorCalibrationView {
    pub schema: String,
    pub team: String,
    pub target_season: u32,
    pub window_seasons: u8,
    pub profile: TeamDecisionProfile,
    pub traits: Vec<BehaviorTraitCalibrationRow>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorRankingRow {
    pub trait_key: String,
    pub team: String,
    pub profile_id: String,
    pub calibrated_value: Option<f64>,
    pub effective_value: Option<f64>,
    pub evidence_opportunities: u32,
    pub evidence_label: EvidenceLabel,
    pub rank: Option<usize>,
    pub percentile: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorRankingCoverageRow {
    pub team: String,
    pub ranked_traits: usize,
    pub total_traits: usize,
    pub coverage_pct: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorRankingView {
    pub schema: String,
    pub target_season: u32,
    pub teams: usize,
    pub rows: Vec<TeamBehaviorRankingRow>,
    pub coverage: Vec<TeamBehaviorRankingCoverageRow>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

pub fn rank_team_decision_profiles(
    profiles: &[TeamDecisionProfile],
) -> Result<TeamBehaviorRankingView, String> {
    if profiles.is_empty() {
        return Err("team behavior ranking requires at least one profile".to_owned());
    }
    let target_season = profiles[0].season;
    let mut teams = BTreeSet::new();
    for profile in profiles {
        profile.validate()?;
        if profile.season != target_season || !teams.insert(profile.team.clone()) {
            return Err(
                "team behavior ranking requires unique teams from one target season".to_owned(),
            );
        }
    }
    let trait_keys = behavior_trait_keys();
    let mut rows = Vec::new();
    for (trait_index, trait_key) in trait_keys.iter().enumerate() {
        let mut ranked = profiles
            .iter()
            .filter_map(|profile| {
                let value = profile.traits()[trait_index];
                (value.evidence_games > 0 && value.evidence_label != EvidenceLabel::NoRead)
                    .then_some((profile, value))
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            b.1.effective_value()
                .total_cmp(&a.1.effective_value())
                .then_with(|| a.0.team.cmp(&b.0.team))
        });
        let ranked_count = ranked.len();
        let ranks = ranked
            .iter()
            .enumerate()
            .map(|(index, (profile, _))| (profile.team.clone(), index + 1))
            .collect::<BTreeMap<_, _>>();
        for profile in profiles {
            let value = profile.traits()[trait_index];
            let rank = ranks.get(&profile.team).copied();
            let percentile = rank.map(|rank| rank_percentile(rank, ranked_count));
            let has_read = rank.is_some();
            rows.push(TeamBehaviorRankingRow {
                trait_key: (*trait_key).to_owned(),
                team: profile.team.clone(),
                profile_id: profile.id.clone(),
                calibrated_value: has_read.then_some(value.value),
                effective_value: has_read.then(|| value.effective_value()),
                evidence_opportunities: value.evidence_games,
                evidence_label: value.evidence_label,
                rank,
                percentile,
            });
        }
    }
    for (trait_key, general_manager) in [("general_manager_index", true), ("manager_index", false)]
    {
        let mut composites = profiles
            .iter()
            .filter_map(|profile| {
                composite_trait(profile, general_manager).map(|row| (profile, row))
            })
            .collect::<Vec<_>>();
        composites.sort_by(|a, b| {
            b.1 .0
                .total_cmp(&a.1 .0)
                .then_with(|| a.0.team.cmp(&b.0.team))
        });
        let count = composites.len();
        let composite_by_team = composites
            .iter()
            .enumerate()
            .map(|(index, (profile, composite))| (profile.team.clone(), (index + 1, *composite)))
            .collect::<BTreeMap<_, _>>();
        for profile in profiles {
            let composite = composite_by_team.get(&profile.team);
            rows.push(TeamBehaviorRankingRow {
                trait_key: trait_key.to_owned(),
                team: profile.team.clone(),
                profile_id: profile.id.clone(),
                calibrated_value: composite.map(|(_, value)| value.0),
                effective_value: composite.map(|(_, value)| value.0),
                evidence_opportunities: composite.map_or(0, |(_, value)| value.1),
                evidence_label: composite
                    .map_or(EvidenceLabel::NoRead, |_| EvidenceLabel::Estimated),
                rank: composite.map(|(rank, _)| *rank),
                percentile: composite.map(|(rank, _)| rank_percentile(*rank, count)),
            });
        }
    }
    rows.sort_by(|a, b| {
        a.trait_key
            .cmp(&b.trait_key)
            .then_with(|| {
                a.rank
                    .unwrap_or(usize::MAX)
                    .cmp(&b.rank.unwrap_or(usize::MAX))
            })
            .then_with(|| a.team.cmp(&b.team))
    });
    let mut coverage = profiles
        .iter()
        .map(|profile| {
            let ranked_traits = profile
                .traits()
                .iter()
                .filter(|value| {
                    value.evidence_games > 0 && value.evidence_label != EvidenceLabel::NoRead
                })
                .count();
            TeamBehaviorRankingCoverageRow {
                team: profile.team.clone(),
                ranked_traits,
                total_traits: trait_keys.len(),
                coverage_pct: ranked_traits as f64 / trait_keys.len() as f64 * 100.0,
            }
        })
        .collect::<Vec<_>>();
    coverage.sort_by(|a, b| a.team.cmp(&b.team));
    let incomplete = coverage
        .iter()
        .filter(|row| row.ranked_traits < row.total_traits)
        .map(|row| row.team.clone())
        .collect::<Vec<_>>();
    let warnings = (!incomplete.is_empty())
        .then(|| {
            format!(
                "Incomplete behavior coverage for: {}",
                incomplete.join(", ")
            )
        })
        .into_iter()
        .collect();
    Ok(TeamBehaviorRankingView {
        schema: TEAM_BEHAVIOR_RANKING_SCHEMA.to_owned(),
        target_season,
        teams: profiles.len(),
        rows,
        coverage,
        warnings,
        disclosures: vec![
            "Ranks use confidence-adjusted effective trait values; raw calibrated values remain visible.".to_owned(),
            "No-read teams remain unranked rather than receiving a league-average rank.".to_owned(),
            "GM and manager composites are equal-weight summaries of available traits, not performance or quality grades.".to_owned(),
        ],
    })
}

fn behavior_trait_keys() -> [&'static str; 12] {
    [
        "rookie_opportunity",
        "veteran_preference",
        "waiver_asset_protection",
        "trade_aggression",
        "deadline_buying_bias",
        "matchup_intensity",
        "tactical_adaptability",
        "lineup_patience",
        "position_flexibility",
        "physical_fourth_line_preference",
        "four_line_usage",
        "fatigue_rotation",
    ]
}

fn composite_trait(profile: &TeamDecisionProfile, general_manager: bool) -> Option<(f64, u32)> {
    let traits = profile.traits();
    let range = if general_manager { 0..5 } else { 5..12 };
    let values = range
        .filter_map(|index| {
            let value = traits[index];
            (value.evidence_games > 0 && value.evidence_label != EvidenceLabel::NoRead)
                .then_some((value.effective_value(), value.evidence_games))
        })
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| {
        (
            values.iter().map(|value| value.0).sum::<f64>() / values.len() as f64,
            values
                .iter()
                .fold(0u32, |total, value| total.saturating_add(value.1)),
        )
    })
}

fn rank_percentile(rank: usize, count: usize) -> f64 {
    if count <= 1 {
        100.0
    } else {
        (count - rank) as f64 / (count - 1) as f64 * 100.0
    }
}

pub fn calibrate_team_decision_profile(
    input: &TeamBehaviorCalibrationInput,
) -> Result<TeamBehaviorCalibrationView, String> {
    if !valid_team(&input.team) || !(1..=3).contains(&input.window_seasons) {
        return Err("behavior calibration requires an NHL team and a 1-3 season window".to_owned());
    }
    let mut seen = BTreeSet::new();
    for season in &input.observations {
        if season.season >= input.target_season || !seen.insert(season.season) {
            return Err(
                "behavior observations must be unique seasons before the target".to_owned(),
            );
        }
        for signal in season_signals(season).into_iter().flatten() {
            if !signal.value.is_finite()
                || !(-1.0..=1.0).contains(&signal.value)
                || signal.opportunities == 0
            {
                return Err(
                    "behavior signals require -1..1 values and positive opportunities".to_owned(),
                );
            }
            if signal.evidence_label == EvidenceLabel::Simulated {
                return Err(
                    "historical behavior calibration cannot learn from simulated evidence"
                        .to_owned(),
                );
            }
        }
    }
    let mut observations = input.observations.iter().collect::<Vec<_>>();
    observations.sort_by_key(|season| std::cmp::Reverse(season.season));
    observations.truncate(usize::from(input.window_seasons));
    if observations.is_empty() {
        return Err("behavior calibration requires at least one completed season".to_owned());
    }

    let rookie = aggregate_trait("rookie_opportunity", &observations, |row| {
        row.rookie_opportunity.as_ref()
    });
    let veteran = aggregate_trait("veteran_preference", &observations, |row| {
        row.veteran_preference.as_ref()
    });
    let waiver = aggregate_trait("waiver_asset_protection", &observations, |row| {
        row.waiver_asset_protection.as_ref()
    });
    let trades = aggregate_trait("trade_aggression", &observations, |row| {
        row.trade_aggression.as_ref()
    });
    let deadline = aggregate_trait("deadline_buying_bias", &observations, |row| {
        row.deadline_buying_bias.as_ref()
    });
    let matchup = aggregate_trait("matchup_intensity", &observations, |row| {
        row.matchup_intensity.as_ref()
    });
    let adaptability = aggregate_trait("tactical_adaptability", &observations, |row| {
        row.tactical_adaptability.as_ref()
    });
    let patience = aggregate_trait("lineup_patience", &observations, |row| {
        row.lineup_patience.as_ref()
    });
    let flexibility = aggregate_trait("position_flexibility", &observations, |row| {
        row.position_flexibility.as_ref()
    });
    let physical = aggregate_trait("physical_fourth_line_preference", &observations, |row| {
        row.physical_fourth_line_preference.as_ref()
    });
    let four_lines = aggregate_trait("four_line_usage", &observations, |row| {
        row.four_line_usage.as_ref()
    });
    let fatigue = aggregate_trait("fatigue_rotation", &observations, |row| {
        row.fatigue_rotation.as_ref()
    });
    let traits = vec![
        rookie,
        veteran,
        waiver,
        trades,
        deadline,
        matchup,
        adaptability,
        patience,
        flexibility,
        physical,
        four_lines,
        fatigue,
    ];
    let trait_view = |key: &str| {
        let row = traits
            .iter()
            .find(|row| row.trait_key == key)
            .expect("all behavior traits are calibrated");
        BehaviorTraitView {
            value: row.calibrated_value,
            evidence_games: row.opportunities,
            evidence_label: row.evidence_label,
        }
    };
    let profile = TeamDecisionProfile {
        schema: TEAM_DECISION_PROFILE_SCHEMA.to_owned(),
        id: format!(
            "{}-{}-season-calibration-{}",
            input.team.to_ascii_lowercase(),
            input.window_seasons,
            input.target_season
        ),
        team: input.team.clone(),
        season: input.target_season,
        general_manager: GeneralManagerBehaviorProfile {
            rookie_opportunity: trait_view("rookie_opportunity"),
            veteran_preference: trait_view("veteran_preference"),
            waiver_asset_protection: trait_view("waiver_asset_protection"),
            trade_aggression: trait_view("trade_aggression"),
            deadline_buying_bias: trait_view("deadline_buying_bias"),
        },
        manager: ManagerBehaviorProfile {
            matchup_intensity: trait_view("matchup_intensity"),
            tactical_adaptability: trait_view("tactical_adaptability"),
            lineup_patience: trait_view("lineup_patience"),
            position_flexibility: trait_view("position_flexibility"),
            physical_fourth_line_preference: trait_view("physical_fourth_line_preference"),
            four_line_usage: trait_view("four_line_usage"),
            fatigue_rotation: trait_view("fatigue_rotation"),
        },
        disclosures: vec![
            "Profile traits are learned estimates from completed seasons, not permanent personality labels.".to_owned(),
            "The newest season receives weight 1.0, the previous 0.65, and the third 0.40; opportunity counts also govern confidence.".to_owned(),
        ],
    };
    let missing = traits
        .iter()
        .filter(|row| row.evidence_label == EvidenceLabel::NoRead)
        .map(|row| row.trait_key.clone())
        .collect::<Vec<_>>();
    let warnings = (!missing.is_empty())
        .then(|| format!("No historical read for: {}", missing.join(", ")))
        .into_iter()
        .collect();
    Ok(TeamBehaviorCalibrationView {
        schema: TEAM_BEHAVIOR_CALIBRATION_SCHEMA.to_owned(),
        team: input.team.clone(),
        target_season: input.target_season,
        window_seasons: input.window_seasons,
        profile,
        traits,
        warnings,
        disclosures: vec![
            "Only observations before the target season are accepted, preventing retrospective leakage.".to_owned(),
            "Missing signals remain neutral with no-read evidence; they are not replaced by league reputation or prose scouting.".to_owned(),
        ],
    })
}

fn aggregate_trait(
    key: &str,
    observations: &[&TeamBehaviorSeasonObservation],
    select: impl for<'a> Fn(&'a TeamBehaviorSeasonObservation) -> Option<&'a BehaviorSignalObservation>,
) -> BehaviorTraitCalibrationRow {
    let recency = [1.0, 0.65, 0.40];
    let mut weighted_value = 0.0;
    let mut weight = 0.0;
    let mut opportunities = 0u32;
    let mut seasons_used = Vec::new();
    let mut under_review = false;
    for (index, season) in observations.iter().enumerate() {
        let Some(signal) = select(season) else {
            continue;
        };
        let sample_weight =
            f64::from(signal.opportunities) / (f64::from(signal.opportunities) + 20.0);
        let combined_weight = recency[index] * sample_weight;
        weighted_value += signal.value * combined_weight;
        weight += combined_weight;
        opportunities = opportunities.saturating_add(signal.opportunities);
        seasons_used.push(season.season);
        under_review |= signal.evidence_label == EvidenceLabel::UnderReview;
    }
    let calibrated_value = if weight == 0.0 {
        0.0
    } else {
        weighted_value / weight
    };
    let evidence_label = if seasons_used.is_empty() {
        EvidenceLabel::NoRead
    } else if under_review {
        EvidenceLabel::UnderReview
    } else {
        EvidenceLabel::Estimated
    };
    let effective_value =
        calibrated_value * f64::from(opportunities) / (f64::from(opportunities) + 20.0);
    BehaviorTraitCalibrationRow {
        trait_key: key.to_owned(),
        seasons_used,
        calibrated_value,
        effective_value,
        opportunities,
        evidence_label,
    }
}

fn season_signals(row: &TeamBehaviorSeasonObservation) -> [Option<&BehaviorSignalObservation>; 12] {
    [
        row.rookie_opportunity.as_ref(),
        row.veteran_preference.as_ref(),
        row.waiver_asset_protection.as_ref(),
        row.trade_aggression.as_ref(),
        row.deadline_buying_bias.as_ref(),
        row.matchup_intensity.as_ref(),
        row.tactical_adaptability.as_ref(),
        row.lineup_patience.as_ref(),
        row.position_flexibility.as_ref(),
        row.physical_fourth_line_preference.as_ref(),
        row.four_line_usage.as_ref(),
        row.fatigue_rotation.as_ref(),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpponentTacticalStyle {
    NorthSouthRush,
    EastWestPossession,
    DumpAndChase,
    HeavyCycle,
    Counterattack,
    Balanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchTacticalResponse {
    NeutralZoneContain,
    LayeredDisruption,
    QuickSupportExit,
    LowZoneSupport,
    PuckSecurity,
    Balanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchForwardRole {
    PrimaryScoring,
    SecondaryScoring,
    Matchup,
    CheckingEnergy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerMatchupRoleInput {
    pub player_id: u32,
    /// Each score is 0 through 100 and must retain its own upstream evidence.
    pub defensive_score: f64,
    pub transition_score: f64,
    pub forecheck_score: f64,
    pub physical_score: f64,
    pub evidence_games: u32,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BenchScheduleLoad {
    pub is_home: bool,
    pub back_to_back: bool,
    pub third_game_in_four_nights: bool,
    pub travel_km: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchGamePlanInput {
    pub opponent: String,
    pub opponent_style: OpponentTacticalStyle,
    #[serde(default)]
    pub opponent_primary_threat: Option<String>,
    pub schedule_load: BenchScheduleLoad,
    #[serde(default)]
    pub opponent_schedule_load: Option<BenchScheduleLoad>,
    pub player_roles: Vec<PlayerMatchupRoleInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchForwardAssignmentView {
    pub line: u8,
    pub role: BenchForwardRole,
    pub player_ids: Vec<u32>,
    pub suitability_score: f64,
    pub projected_five_on_five_share_pct: f64,
    pub target: Option<String>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchDefenseAssignmentView {
    pub pair: u8,
    pub primary_matchup: bool,
    pub player_ids: Vec<u32>,
    pub suitability_score: f64,
    pub target: Option<String>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchGamePlanView {
    pub schema: String,
    pub team: String,
    pub opponent: String,
    pub opponent_style: OpponentTacticalStyle,
    pub tactical_response: BenchTacticalResponse,
    pub manager_profile_id: String,
    pub hard_match_confidence: f64,
    /// Signed 0-100 team-strength adjustment from opponent-specific lineup
    /// suitability and estimated plan execution. Schedule fatigue is excluded
    /// here because IceCast already models it from the schedule.
    #[serde(default)]
    pub tactical_matchup_edge: f64,
    /// Signed 0-100 team-strength adjustment from relative schedule fatigue.
    pub schedule_fatigue_edge: f64,
    pub forward_assignments: Vec<BenchForwardAssignmentView>,
    pub defense_assignments: Vec<BenchDefenseAssignmentView>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleRestGameInput {
    pub opponent: String,
    pub team_load: BenchScheduleLoad,
    pub opponent_load: BenchScheduleLoad,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleRestProfileView {
    pub schema: String,
    pub team: String,
    pub games: usize,
    pub own_back_to_backs: usize,
    pub opponents_on_back_to_backs: usize,
    pub own_third_in_four: usize,
    pub opponents_in_third_in_four: usize,
    pub rested_vs_tired: usize,
    pub tired_vs_rested: usize,
    pub net_back_to_back_opportunity: i32,
    pub average_schedule_fatigue_edge: f64,
    pub disclosures: Vec<String>,
}

pub fn build_schedule_rest_profile(
    team: &str,
    games: &[ScheduleRestGameInput],
) -> Result<ScheduleRestProfileView, String> {
    if !valid_team(team) {
        return Err("schedule rest profile requires an NHL team".to_owned());
    }
    for game in games {
        if !valid_team(&game.opponent) || game.opponent == team {
            return Err("schedule rest games require distinct NHL opponents".to_owned());
        }
        for load in [&game.team_load, &game.opponent_load] {
            if !load.travel_km.is_finite() || load.travel_km < 0.0 {
                return Err("schedule rest travel must be finite and non-negative".to_owned());
            }
        }
    }
    let own_back_to_backs = games
        .iter()
        .filter(|game| game.team_load.back_to_back)
        .count();
    let opponents_on_back_to_backs = games
        .iter()
        .filter(|game| game.opponent_load.back_to_back)
        .count();
    let own_third_in_four = games
        .iter()
        .filter(|game| game.team_load.third_game_in_four_nights)
        .count();
    let opponents_in_third_in_four = games
        .iter()
        .filter(|game| game.opponent_load.third_game_in_four_nights)
        .count();
    let rested_vs_tired = games
        .iter()
        .filter(|game| !game.team_load.back_to_back && game.opponent_load.back_to_back)
        .count();
    let tired_vs_rested = games
        .iter()
        .filter(|game| game.team_load.back_to_back && !game.opponent_load.back_to_back)
        .count();
    let average_schedule_fatigue_edge = if games.is_empty() {
        0.0
    } else {
        games
            .iter()
            .map(|game| {
                ((schedule_fatigue_load(&game.opponent_load)
                    - schedule_fatigue_load(&game.team_load))
                    * 1.25)
                    .clamp(-3.0, 3.0)
            })
            .sum::<f64>()
            / games.len() as f64
    };
    Ok(ScheduleRestProfileView {
        schema: SCHEDULE_REST_PROFILE_SCHEMA.to_owned(),
        team: team.to_owned(),
        games: games.len(),
        own_back_to_backs,
        opponents_on_back_to_backs,
        own_third_in_four,
        opponents_in_third_in_four,
        rested_vs_tired,
        tired_vs_rested,
        net_back_to_back_opportunity: opponents_on_back_to_backs as i32
            - own_back_to_backs as i32,
        average_schedule_fatigue_edge,
        disclosures: vec![
            "Rest opportunity is schedule context, not team talent.".to_owned(),
            "Opponent back-to-back counts and own back-to-back counts are reported separately; the net is not a causal wins estimate.".to_owned(),
        ],
    })
}

pub fn build_bench_game_plan(
    lineup: &TeamLineupProjectionView,
    profile: &TeamDecisionProfile,
    input: &BenchGamePlanInput,
) -> Result<BenchGamePlanView, String> {
    profile.validate()?;
    if !profile.team.eq_ignore_ascii_case(&lineup.team) || profile.season != lineup.roster_season {
        return Err("manager profile and lineup identify different team seasons".to_owned());
    }
    if !valid_team(&input.opponent) || input.opponent.eq_ignore_ascii_case(&lineup.team) {
        return Err("Bench game plan requires a distinct NHL opponent".to_owned());
    }
    if !input.schedule_load.travel_km.is_finite() || input.schedule_load.travel_km < 0.0 {
        return Err("Bench game-plan travel must be finite and non-negative".to_owned());
    }
    let lineup_ids = lineup_player_ids(lineup);
    let mut roles = BTreeMap::new();
    for role in &input.player_roles {
        if !lineup_ids.contains(&role.player_id) || roles.insert(role.player_id, role).is_some() {
            return Err("matchup role evidence must uniquely reference lineup players".to_owned());
        }
        for score in [
            role.defensive_score,
            role.transition_score,
            role.forecheck_score,
            role.physical_score,
        ] {
            if !score.is_finite() || !(0.0..=100.0).contains(&score) {
                return Err("matchup role scores must be between 0 and 100".to_owned());
            }
        }
    }

    let mut warnings = Vec::new();
    if roles.len() < lineup_ids.len().saturating_sub(2) {
        warnings.push(
            "Matchup-role evidence is incomplete; missing players remain neutral at 50.".to_owned(),
        );
    }
    let response = tactical_response(input.opponent_style);
    let target = input.opponent_primary_threat.clone();
    let mut line_rows = lineup
        .forward_lines
        .iter()
        .map(|line| {
            let players = [&line.left_wing, &line.center, &line.right_wing]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            UnitScores::from_players(&players, &roles, input.opponent_style)
                .map(|scores| (line.line, players, scores))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "Bench game plan requires four complete forward lines".to_owned())?;
    let matchup_line = best_unit(&line_rows, |scores| scores.matchup);
    let checking_line = best_unit_excluding(&line_rows, matchup_line, |scores| scores.checking);
    let remaining = line_rows
        .iter()
        .filter(|(line, _, _)| *line != matchup_line && *line != checking_line)
        .map(|(line, _, scores)| (*line, scores.offense))
        .collect::<Vec<_>>();
    let primary_scoring = remaining
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|row| row.0)
        .unwrap_or(matchup_line);

    let shares = projected_line_shares(profile, &input.schedule_load, checking_line);
    let mut forward_assignments = line_rows
        .drain(..)
        .map(|(line, players, scores)| {
            let role = if line == matchup_line {
                BenchForwardRole::Matchup
            } else if line == checking_line {
                BenchForwardRole::CheckingEnergy
            } else if line == primary_scoring {
                BenchForwardRole::PrimaryScoring
            } else {
                BenchForwardRole::SecondaryScoring
            };
            let suitability_score = match role {
                BenchForwardRole::Matchup => scores.matchup,
                BenchForwardRole::CheckingEnergy => scores.checking,
                BenchForwardRole::PrimaryScoring | BenchForwardRole::SecondaryScoring => {
                    scores.offense
                }
            };
            BenchForwardAssignmentView {
                line,
                role,
                player_ids: players.iter().map(|player| player.player_id).collect(),
                suitability_score,
                projected_five_on_five_share_pct: shares[usize::from(line - 1)],
                target: matches!(role, BenchForwardRole::Matchup)
                    .then(|| target.clone())
                    .flatten(),
                evidence_label: EvidenceLabel::Estimated,
            }
        })
        .collect::<Vec<_>>();
    forward_assignments.sort_by_key(|row| row.line);

    let pair_rows = lineup
        .defense_pairs
        .iter()
        .map(|pair| {
            let players = [&pair.left, &pair.right]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            UnitScores::from_players(&players, &roles, input.opponent_style)
                .map(|scores| (pair.pair, players, scores))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "Bench game plan requires three complete defense pairs".to_owned())?;
    let matchup_pair = best_unit(&pair_rows, |scores| scores.matchup);
    let defense_assignments = pair_rows
        .into_iter()
        .map(|(pair, players, scores)| BenchDefenseAssignmentView {
            pair,
            primary_matchup: pair == matchup_pair,
            player_ids: players.iter().map(|player| player.player_id).collect(),
            suitability_score: scores.matchup,
            target: (pair == matchup_pair).then(|| target.clone()).flatten(),
            evidence_label: EvidenceLabel::Estimated,
        })
        .collect::<Vec<_>>();

    let intensity = profile.manager.matchup_intensity.effective_value();
    let adaptability = profile.manager.tactical_adaptability.effective_value();
    let home_change = if input.schedule_load.is_home {
        0.16
    } else {
        -0.08
    };
    let fatigue = schedule_fatigue_load(&input.schedule_load) * 0.10;
    let hard_match_confidence =
        (0.55 + intensity * 0.18 + adaptability * 0.10 + home_change - fatigue).clamp(0.05, 0.95);
    let opponent_fatigue = input
        .opponent_schedule_load
        .as_ref()
        .map(schedule_fatigue_load)
        .unwrap_or(0.0);
    let schedule_fatigue_edge =
        ((opponent_fatigue - schedule_fatigue_load(&input.schedule_load)) * 1.25).clamp(-3.0, 3.0);
    let matchup_forward = forward_assignments
        .iter()
        .find(|row| row.role == BenchForwardRole::Matchup)
        .map(|row| row.suitability_score)
        .unwrap_or(50.0);
    let primary_defense = defense_assignments
        .iter()
        .find(|row| row.primary_matchup)
        .map(|row| row.suitability_score)
        .unwrap_or(50.0);
    let primary_scoring = forward_assignments
        .iter()
        .find(|row| row.role == BenchForwardRole::PrimaryScoring)
        .map(|row| row.suitability_score)
        .unwrap_or(50.0);
    let plan_suitability = matchup_forward * 0.50 + primary_defense * 0.35 + primary_scoring * 0.15;
    let tactical_matchup_edge =
        (((plan_suitability - 50.0) / 50.0) * hard_match_confidence * 3.0).clamp(-3.0, 3.0);

    Ok(BenchGamePlanView {
        schema: BENCH_GAME_PLAN_SCHEMA.to_owned(),
        team: lineup.team.clone(),
        opponent: input.opponent.to_ascii_uppercase(),
        opponent_style: input.opponent_style,
        tactical_response: response,
        manager_profile_id: profile.id.clone(),
        hard_match_confidence,
        tactical_matchup_edge,
        schedule_fatigue_edge,
        forward_assignments,
        defense_assignments,
        warnings,
        disclosures: vec![
            "Assignments are opponent-specific simulated manager decisions, not confirmed lines.".to_owned(),
            "Home last change raises hard-match confidence; back-to-backs, third-in-four load, and travel reduce it.".to_owned(),
            "Relative fatigue compares both teams, so catching a tired opponent and playing tired are distinct schedule effects.".to_owned(),
            "The tactical matchup edge excludes the separate schedule-fatigue edge so IceCast can apply the game plan without counting its schedule model twice.".to_owned(),
            "A fatigue-rotation manager can roll the checking/energy line more on compressed schedules instead of automatically shortening the bench.".to_owned(),
        ],
    })
}

#[derive(Debug, Clone, Copy)]
struct UnitScores {
    offense: f64,
    matchup: f64,
    checking: f64,
}

impl UnitScores {
    fn from_players(
        players: &[&TeamLineupPlayerView],
        roles: &BTreeMap<u32, &PlayerMatchupRoleInput>,
        style: OpponentTacticalStyle,
    ) -> Option<Self> {
        if players.is_empty() {
            return None;
        }
        let mut offense = 0.0;
        let mut defense = 0.0;
        let mut transition = 0.0;
        let mut forecheck = 0.0;
        let mut physical = 0.0;
        for player in players {
            let games = f64::from(player.score.sample_games);
            offense += player.score.value.unwrap_or(50.0) * games / (games + 20.0)
                + 50.0 * 20.0 / (games + 20.0);
            if let Some(role) = roles.get(&player.player_id) {
                let confidence =
                    f64::from(role.evidence_games) / (f64::from(role.evidence_games) + 20.0);
                defense += 50.0 + (role.defensive_score - 50.0) * confidence;
                transition += 50.0 + (role.transition_score - 50.0) * confidence;
                forecheck += 50.0 + (role.forecheck_score - 50.0) * confidence;
                physical += 50.0 + (role.physical_score - 50.0) * confidence;
            } else {
                defense += 50.0;
                transition += 50.0;
                forecheck += 50.0;
                physical += 50.0;
            }
        }
        let count = players.len() as f64;
        let offense = offense / count;
        let defense = defense / count;
        let transition = transition / count;
        let forecheck = forecheck / count;
        let physical = physical / count;
        let matchup = match style {
            OpponentTacticalStyle::NorthSouthRush => {
                defense * 0.55 + transition * 0.35 + offense * 0.10
            }
            OpponentTacticalStyle::EastWestPossession => {
                defense * 0.60 + transition * 0.25 + offense * 0.15
            }
            OpponentTacticalStyle::DumpAndChase => {
                defense * 0.45 + physical * 0.30 + transition * 0.25
            }
            OpponentTacticalStyle::HeavyCycle => {
                defense * 0.45 + physical * 0.40 + forecheck * 0.15
            }
            OpponentTacticalStyle::Counterattack => {
                transition * 0.50 + defense * 0.35 + offense * 0.15
            }
            OpponentTacticalStyle::Balanced => defense * 0.45 + transition * 0.30 + offense * 0.25,
        };
        Some(Self {
            offense,
            matchup,
            checking: physical * 0.55 + forecheck * 0.35 + defense * 0.10,
        })
    }
}

fn tactical_response(style: OpponentTacticalStyle) -> BenchTacticalResponse {
    match style {
        OpponentTacticalStyle::NorthSouthRush => BenchTacticalResponse::NeutralZoneContain,
        OpponentTacticalStyle::EastWestPossession => BenchTacticalResponse::LayeredDisruption,
        OpponentTacticalStyle::DumpAndChase => BenchTacticalResponse::QuickSupportExit,
        OpponentTacticalStyle::HeavyCycle => BenchTacticalResponse::LowZoneSupport,
        OpponentTacticalStyle::Counterattack => BenchTacticalResponse::PuckSecurity,
        OpponentTacticalStyle::Balanced => BenchTacticalResponse::Balanced,
    }
}

fn projected_line_shares(
    profile: &TeamDecisionProfile,
    load: &BenchScheduleLoad,
    checking_line: u8,
) -> [f64; 4] {
    let mut shares = [31.0, 27.0, 23.0, 19.0];
    let fatigue = f64::from(load.back_to_back)
        + f64::from(load.third_game_in_four_nights) * 0.75
        + (load.travel_km / 4_000.0).min(0.75);
    let roll = (profile
        .manager
        .physical_fourth_line_preference
        .effective_value()
        * 1.5
        + (profile.manager.four_line_usage.effective_value()
            + profile.manager.fatigue_rotation.effective_value())
            * fatigue
            * 2.5)
        .clamp(-4.0, 4.0);
    let checking_index = usize::from(checking_line - 1);
    shares[checking_index] += roll;
    let debit = roll / 3.0;
    for (index, share) in shares.iter_mut().enumerate() {
        if index != checking_index {
            *share -= debit;
        }
    }
    shares
}

fn schedule_fatigue_load(load: &BenchScheduleLoad) -> f64 {
    f64::from(load.back_to_back)
        + f64::from(load.third_game_in_four_nights) * 0.75
        + (load.travel_km / 4_000.0).min(0.75)
}

fn best_unit<T>(rows: &[(u8, Vec<T>, UnitScores)], score: impl Fn(&UnitScores) -> f64) -> u8 {
    rows.iter()
        .max_by(|a, b| {
            score(&a.2)
                .total_cmp(&score(&b.2))
                .then_with(|| b.0.cmp(&a.0))
        })
        .map(|row| row.0)
        .unwrap_or(1)
}

fn best_unit_excluding<T>(
    rows: &[(u8, Vec<T>, UnitScores)],
    excluded: u8,
    score: impl Fn(&UnitScores) -> f64,
) -> u8 {
    rows.iter()
        .filter(|row| row.0 != excluded)
        .max_by(|a, b| {
            score(&a.2)
                .total_cmp(&score(&b.2))
                .then_with(|| b.0.cmp(&a.0))
        })
        .map(|row| row.0)
        .unwrap_or(excluded)
}

fn lineup_player_ids(lineup: &TeamLineupProjectionView) -> BTreeSet<u32> {
    lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .chain([&lineup.goalies.starter, &lineup.goalies.backup])
        .flatten()
        .map(|player| player.player_id)
        .collect()
}

fn valid_team(team: &str) -> bool {
    team.len() == 3 && team.bytes().all(|byte| byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::model::Position;

    use super::*;
    use crate::view_model::team_lineup::{
        build_team_lineup_projection, LineupAssignmentEvidence, TeamLineupPlayerInput,
    };
    use crate::view_model::TeamCeilingLens;

    fn tendency(value: f64) -> BehaviorTraitView {
        BehaviorTraitView {
            value,
            evidence_games: 82,
            evidence_label: EvidenceLabel::Estimated,
        }
    }

    fn profile() -> TeamDecisionProfile {
        TeamDecisionProfile {
            schema: TEAM_DECISION_PROFILE_SCHEMA.to_owned(),
            id: "nyr-manager-2026-27".to_owned(),
            team: "NYR".to_owned(),
            season: 20262027,
            general_manager: GeneralManagerBehaviorProfile {
                rookie_opportunity: tendency(0.2),
                veteran_preference: tendency(0.1),
                waiver_asset_protection: tendency(0.4),
                trade_aggression: tendency(0.3),
                deadline_buying_bias: tendency(0.2),
            },
            manager: ManagerBehaviorProfile {
                matchup_intensity: tendency(0.8),
                tactical_adaptability: tendency(0.7),
                lineup_patience: tendency(0.2),
                position_flexibility: tendency(0.5),
                physical_fourth_line_preference: tendency(0.8),
                four_line_usage: tendency(0.8),
                fatigue_rotation: tendency(0.9),
            },
            disclosures: vec!["Synthetic test profile".to_owned()],
        }
    }

    fn tenure(person_id: &str, role: LeadershipRole, started_on: &str) -> LeadershipTenureInput {
        LeadershipTenureInput {
            person_id: person_id.to_owned(),
            person_name: person_id.replace('-', " "),
            role,
            team: "NYR".to_owned(),
            started_on: started_on.to_owned(),
            ended_on: None,
            source_url: "https://example.test/appointment".to_owned(),
            source_title: "Official appointment".to_owned(),
        }
    }

    fn marker(person_id: &str, trait_key: &str, value: f64) -> BehaviorResearchMarkerInput {
        BehaviorResearchMarkerInput {
            marker_id: format!("{person_id}-{trait_key}"),
            person_id: person_id.to_owned(),
            role: LeadershipRole::HeadCoach,
            trait_key: trait_key.to_owned(),
            value,
            confidence: 0.8,
            published_on: "2024-06-01".to_owned(),
            source_url: "https://example.test/profile".to_owned(),
            source_title: "Dated coaching profile".to_owned(),
            evidence_summary: "The coach repeatedly used matchup deployment at a prior stop."
                .to_owned(),
            observed_team: Some("CBJ".to_owned()),
        }
    }

    #[test]
    fn leadership_change_retires_predecessor_markers() {
        let input = TeamBehaviorResearchInput {
            schema: TEAM_BEHAVIOR_RESEARCH_SCHEMA.to_owned(),
            team: "NYR".to_owned(),
            target_season: 20262027,
            as_of: "2026-07-23".to_owned(),
            leadership: vec![tenure("new-coach", LeadershipRole::HeadCoach, "2026-05-01")],
            markers: vec![marker("old-coach", "matchup_intensity", -1.0)],
        };
        let view = apply_team_behavior_research(&profile(), &input).unwrap();
        assert!(!view.marker_decisions[0].accepted);
        assert_eq!(view.enriched_profile.manager.matchup_intensity.value, 0.8);
    }

    #[test]
    fn active_leader_carries_dated_prior_team_markers_at_capped_weight() {
        let input = TeamBehaviorResearchInput {
            schema: TEAM_BEHAVIOR_RESEARCH_SCHEMA.to_owned(),
            team: "NYR".to_owned(),
            target_season: 20262027,
            as_of: "2026-07-23".to_owned(),
            leadership: vec![tenure(
                "current-coach",
                LeadershipRole::HeadCoach,
                "2026-05-01",
            )],
            markers: vec![marker("current-coach", "matchup_intensity", -1.0)],
        };
        let view = apply_team_behavior_research(&profile(), &input).unwrap();
        assert!(view.marker_decisions[0].accepted);
        let value = view.enriched_profile.manager.matchup_intensity.value;
        assert!(
            value < 0.8 && value > 0.35,
            "research is directional but capped: {value}"
        );
        assert_eq!(view.traits[5].marker_ids.len(), 1);
    }

    fn player(id: u32, position: Position) -> TeamLineupPlayerInput {
        TeamLineupPlayerInput {
            player_id: id,
            display_name: format!("Player {id}"),
            team: "NYR".to_owned(),
            prior_team: None,
            primary_position: position,
            eligible_positions: vec![position],
            headshot_canonical_url: None,
            games_played: 82,
            lens_scores: BTreeMap::from([(TeamCeilingLens::PointsPace, Some(70.0))]),
            score_evidence: EvidenceLabel::Estimated,
            power_play_role_score: None,
            penalty_kill_role_score: None,
            special_teams_evidence: None,
            requested_slot: None,
            assignment_evidence: LineupAssignmentEvidence::Scenario,
        }
    }

    fn lineup() -> TeamLineupProjectionView {
        let mut players = (0..12)
            .map(|index| {
                player(
                    100 + index,
                    [Position::LeftWing, Position::Center, Position::RightWing][index as usize % 3],
                )
            })
            .collect::<Vec<_>>();
        players.extend((0..6).map(|index| player(200 + index, Position::Defense)));
        players.extend((0..2).map(|index| player(300 + index, Position::Goalie)));
        build_team_lineup_projection("NYR", 20262027, players).unwrap()
    }

    fn input(load: BenchScheduleLoad) -> BenchGamePlanInput {
        let mut player_roles = Vec::new();
        for id in 100..112 {
            let matchup = (103..106).contains(&id);
            let checking = (109..112).contains(&id);
            player_roles.push(PlayerMatchupRoleInput {
                player_id: id,
                defensive_score: if matchup { 92.0 } else { 55.0 },
                transition_score: if matchup { 90.0 } else { 55.0 },
                forecheck_score: if checking { 94.0 } else { 50.0 },
                physical_score: if checking { 96.0 } else { 45.0 },
                evidence_games: 82,
                evidence_label: EvidenceLabel::Estimated,
            });
        }
        for id in 200..206 {
            player_roles.push(PlayerMatchupRoleInput {
                player_id: id,
                defensive_score: if id < 202 { 90.0 } else { 55.0 },
                transition_score: if id < 202 { 88.0 } else { 55.0 },
                forecheck_score: 50.0,
                physical_score: 60.0,
                evidence_games: 82,
                evidence_label: EvidenceLabel::Estimated,
            });
        }
        BenchGamePlanInput {
            opponent: "COL".to_owned(),
            opponent_style: OpponentTacticalStyle::NorthSouthRush,
            opponent_primary_threat: Some("MacKinnon line".to_owned()),
            schedule_load: load,
            opponent_schedule_load: Some(BenchScheduleLoad {
                is_home: false,
                back_to_back: false,
                third_game_in_four_nights: false,
                travel_km: 0.0,
            }),
            player_roles,
        }
    }

    #[test]
    fn assigns_opponent_specific_matchup_and_checking_jobs() {
        let plan = build_bench_game_plan(
            &lineup(),
            &profile(),
            &input(BenchScheduleLoad {
                is_home: true,
                back_to_back: false,
                third_game_in_four_nights: false,
                travel_km: 0.0,
            }),
        )
        .unwrap();

        assert_eq!(
            plan.tactical_response,
            BenchTacticalResponse::NeutralZoneContain
        );
        assert_eq!(plan.forward_assignments[1].role, BenchForwardRole::Matchup);
        assert_eq!(
            plan.forward_assignments[3].role,
            BenchForwardRole::CheckingEnergy
        );
        assert_eq!(
            plan.forward_assignments[1].target.as_deref(),
            Some("MacKinnon line")
        );
        assert!(plan.defense_assignments[0].primary_matchup);
        assert!(plan.tactical_matchup_edge > 0.0);
    }

    #[test]
    fn back_to_back_reduces_match_confidence_and_rolls_energy_line() {
        let rested = build_bench_game_plan(
            &lineup(),
            &profile(),
            &input(BenchScheduleLoad {
                is_home: true,
                back_to_back: false,
                third_game_in_four_nights: false,
                travel_km: 0.0,
            }),
        )
        .unwrap();
        let tired = build_bench_game_plan(
            &lineup(),
            &profile(),
            &input(BenchScheduleLoad {
                is_home: false,
                back_to_back: true,
                third_game_in_four_nights: true,
                travel_km: 1_500.0,
            }),
        )
        .unwrap();

        assert!(tired.hard_match_confidence < rested.hard_match_confidence);
        assert!(tired.tactical_matchup_edge < rested.tactical_matchup_edge);
        assert!(tired.schedule_fatigue_edge < 0.0);
        let rested_energy = rested
            .forward_assignments
            .iter()
            .find(|row| row.role == BenchForwardRole::CheckingEnergy)
            .unwrap();
        let tired_energy = tired
            .forward_assignments
            .iter()
            .find(|row| row.role == BenchForwardRole::CheckingEnergy)
            .unwrap();
        assert!(
            tired_energy.projected_five_on_five_share_pct
                > rested_energy.projected_five_on_five_share_pct
        );
    }

    #[test]
    fn rest_profile_exposes_opponent_back_to_back_schedule_imbalance() {
        let games = (0..19)
            .map(|index| ScheduleRestGameInput {
                opponent: if index % 2 == 0 { "MTL" } else { "TOR" }.to_owned(),
                team_load: BenchScheduleLoad {
                    is_home: true,
                    back_to_back: index < 6,
                    third_game_in_four_nights: false,
                    travel_km: 0.0,
                },
                opponent_load: BenchScheduleLoad {
                    is_home: false,
                    back_to_back: true,
                    third_game_in_four_nights: false,
                    travel_km: 500.0,
                },
            })
            .collect::<Vec<_>>();
        let view = build_schedule_rest_profile("NYR", &games).unwrap();

        assert_eq!(view.opponents_on_back_to_backs, 19);
        assert_eq!(view.own_back_to_backs, 6);
        assert_eq!(view.net_back_to_back_opportunity, 13);
        assert_eq!(view.rested_vs_tired, 13);
        assert!(view.average_schedule_fatigue_edge > 0.0);
    }

    fn signal(value: f64, opportunities: u32) -> Option<BehaviorSignalObservation> {
        Some(BehaviorSignalObservation {
            value,
            opportunities,
            evidence_label: EvidenceLabel::Confirmed,
        })
    }

    fn historical_season(season: u32, rookie: f64) -> TeamBehaviorSeasonObservation {
        TeamBehaviorSeasonObservation {
            season,
            rookie_opportunity: signal(rookie, 40),
            veteran_preference: signal(-rookie, 35),
            waiver_asset_protection: signal(0.2, 12),
            trade_aggression: signal(0.1, 20),
            deadline_buying_bias: signal(0.3, 8),
            matchup_intensity: signal(0.5, 70),
            tactical_adaptability: signal(0.25, 50),
            lineup_patience: signal(0.1, 60),
            position_flexibility: signal(0.2, 30),
            physical_fourth_line_preference: signal(0.6, 65),
            four_line_usage: signal(0.4, 75),
            fatigue_rotation: signal(0.7, 14),
        }
    }

    #[test]
    fn calibration_supports_one_two_or_three_season_recency_windows() {
        let observations = vec![
            historical_season(20232024, -0.8),
            historical_season(20242025, -0.2),
            historical_season(20252026, 0.8),
        ];
        let calibrate = |window_seasons| {
            calibrate_team_decision_profile(&TeamBehaviorCalibrationInput {
                team: "NYR".to_owned(),
                target_season: 20262027,
                window_seasons,
                observations: observations.clone(),
            })
            .unwrap()
        };

        let one = calibrate(1);
        let two = calibrate(2);
        let three = calibrate(3);
        assert_eq!(one.profile.general_manager.rookie_opportunity.value, 0.8);
        assert!(two.profile.general_manager.rookie_opportunity.value > 0.0);
        assert!(
            three.profile.general_manager.rookie_opportunity.value
                < two.profile.general_manager.rookie_opportunity.value
        );
        assert_eq!(
            three.traits[0].seasons_used,
            vec![20252026, 20242025, 20232024]
        );
        assert_eq!(three.profile.season, 20262027);
        assert!(three.warnings.is_empty());
    }

    #[test]
    fn calibration_keeps_missing_traits_no_read_and_rejects_leakage() {
        let mut observation = TeamBehaviorSeasonObservation {
            season: 20252026,
            rookie_opportunity: signal(0.5, 20),
            ..TeamBehaviorSeasonObservation::default()
        };
        let input = |observation| TeamBehaviorCalibrationInput {
            team: "SEA".to_owned(),
            target_season: 20262027,
            window_seasons: 1,
            observations: vec![observation],
        };
        let view = calibrate_team_decision_profile(&input(observation.clone())).unwrap();
        assert_eq!(
            view.profile.manager.matchup_intensity.evidence_label,
            EvidenceLabel::NoRead
        );
        assert!(view
            .warnings
            .iter()
            .any(|warning| warning.contains("matchup_intensity")));

        observation
            .rookie_opportunity
            .as_mut()
            .unwrap()
            .evidence_label = EvidenceLabel::Simulated;
        assert!(calibrate_team_decision_profile(&input(observation)).is_err());
    }

    #[test]
    fn count_facts_create_auditable_league_relative_behavior_signals() {
        let observation = build_team_behavior_season_observation(&TeamBehaviorSeasonFactsInput {
            season: 20252026,
            rookie_opening_roster_decisions: Some(RelativeBehaviorCountFact {
                team_successes: 6,
                team_opportunities: 10,
                league_successes: 100,
                league_opportunities: 250,
                evidence_label: EvidenceLabel::Confirmed,
            }),
            ..TeamBehaviorSeasonFactsInput::default()
        })
        .unwrap();
        let rookie = observation.rookie_opportunity.unwrap();
        assert!((rookie.value - 0.4).abs() < 1e-12);
        assert_eq!(rookie.opportunities, 10);
        assert_eq!(rookie.evidence_label, EvidenceLabel::Confirmed);
        assert!(observation.matchup_intensity.is_none());
    }

    #[test]
    fn league_ranking_orders_effective_traits_and_keeps_no_read_unranked() {
        let mut nyr = profile();
        nyr.id = "nyr-profile".to_owned();
        nyr.general_manager.rookie_opportunity = tendency(0.8);
        let mut sea = profile();
        sea.id = "sea-profile".to_owned();
        sea.team = "SEA".to_owned();
        sea.general_manager.rookie_opportunity = tendency(-0.2);
        let mut mtl = profile();
        mtl.id = "mtl-profile".to_owned();
        mtl.team = "MTL".to_owned();
        mtl.general_manager.rookie_opportunity = BehaviorTraitView {
            value: 0.0,
            evidence_games: 0,
            evidence_label: EvidenceLabel::NoRead,
        };

        let view = rank_team_decision_profiles(&[sea, mtl, nyr]).unwrap();
        let rookie_rows = view
            .rows
            .iter()
            .filter(|row| row.trait_key == "rookie_opportunity")
            .collect::<Vec<_>>();
        assert_eq!(rookie_rows[0].team, "NYR");
        assert_eq!(rookie_rows[0].rank, Some(1));
        assert_eq!(rookie_rows[0].percentile, Some(100.0));
        assert_eq!(rookie_rows[1].team, "SEA");
        assert_eq!(rookie_rows[1].rank, Some(2));
        assert_eq!(rookie_rows[2].team, "MTL");
        assert_eq!(rookie_rows[2].rank, None);
        assert_eq!(rookie_rows[2].effective_value, None);
        assert!(view
            .rows
            .iter()
            .any(|row| row.trait_key == "manager_index" && row.rank.is_some()));
        assert!(view
            .coverage
            .iter()
            .find(|row| row.team == "MTL")
            .is_some_and(|row| row.ranked_traits == 11));
    }
}
