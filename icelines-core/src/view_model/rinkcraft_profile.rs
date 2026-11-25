//! Deterministic statistical evidence for Rinkcraft player sheets.
//! Catalog reads remain authoritative. Families are navigation, not class scores.
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::identity::PlayerId;
use crate::model::{Position, Season};
use crate::season_stats::SeasonType;
use crate::stats_catalog::{StatId, StatUnit};
use crate::stats_repository::ReadPlayerView;
use crate::stats_repository::StatsRepository;
use crate::SeasonStatsRead;

pub const RINKCRAFT_PROFILE_SCHEMA: &str = "icelines.rinkcraft-stat-profile.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RinkcraftProfileRequest {
    pub player_id: PlayerId,
    pub season_type: SeasonType,
    pub first_season: Option<Season>,
    pub last_season: Option<Season>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RinkcraftProfileError {
    #[error("player {0} is not loaded")]
    UnknownPlayer(PlayerId),
    #[error("first season must not be later than last season")]
    ReversedWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMetricStatus {
    Available,
    IncompleteCoverage,
    Unavailable,
    NotApplicable,
    AggregationUnsupported,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileMetric {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub value: Option<f64>,
    pub status: ProfileMetricStatus,
    pub aggregation: String,
    pub observed_seasons: usize,
    pub missing_seasons: Vec<Season>,
    pub report_source: Option<String>,
    pub ratio: Option<ProfileRatio>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileRatio {
    pub numerator_key: String,
    pub denominator_key: String,
    pub numerator: f64,
    pub denominator: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileSeason {
    pub season: Season,
    pub position: Position,
    pub teams: Vec<String>,
    pub traded: bool,
    pub games: u32,
    pub metrics: Vec<ProfileMetric>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileMetricChange {
    pub key: String,
    pub previous: Option<f64>,
    pub latest: Option<f64>,
    /// Arithmetic difference only; not a significance test or role-change claim.
    pub difference: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileSeasonChange {
    pub from: Season,
    pub to: Season,
    pub consecutive: bool,
    pub previous_games: u32,
    pub latest_games: u32,
    pub position_changed: bool,
    pub metrics: Vec<ProfileMetricChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileJobFamily {
    pub name: String,
    pub catalog_keys: Vec<String>,
    pub interpretation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RinkcraftStatProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_puck_events: Option<super::rinkcraft_goalie_puck::Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stat_attribute_changes: Vec<super::rinkcraft_stat_scores::SeasonChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stat_attribute_seasons: Vec<super::rinkcraft_stat_scores::Profile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achievements: Option<super::rinkcraft_achievements::AchievementProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment_evidence: Option<super::rinkcraft_deployment::DeploymentEvidence>,
    /// Authored reading help; never counted as evidence about this player.
    #[serde(default)]
    pub evidence_guide: Vec<super::rinkcraft_evidence_guide::EvidenceGuideTopic>,
    #[serde(default)]
    pub numerical_history: Option<super::rinkcraft_history::HistoryCard>,
    #[serde(default)]
    pub numerical_snapshot: Option<super::rinkcraft_history::SnapshotSelection>,
    #[serde(default)]
    pub numerical_evidence: Option<super::rinkcraft_numbers::EvidenceCard>,
    #[serde(default)]
    pub character_sheet: Option<super::rinkcraft_character::CharacterSheet>,
    #[serde(default)]
    pub attribute_seasons: Vec<super::rinkcraft_attributes::AttributeSeason>,
    pub schema: String,
    pub request: RinkcraftProfileRequest,
    pub player_name: String,
    pub scope: String,
    pub league_current_season: Season,
    pub current_season_available: bool,
    pub source_windows: Vec<ProfileSourceWindow>,
    pub included_seasons: Vec<Season>,
    /// An absent row is not evidence that the player did not play.
    pub absent_seasons_in_requested_range: Vec<Season>,
    pub latest_available: Option<ProfileSeason>,
    pub summary: Vec<ProfileMetric>,
    pub timeline: Vec<ProfileSeason>,
    pub changes: Vec<ProfileSeasonChange>,
    pub job_families: Vec<ProfileJobFamily>,
    pub unmeasured_dimensions: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileSourceWindow {
    pub season: Season,
    pub season_type: SeasonType,
    pub missing_reports: Vec<String>,
}

fn read<R: SeasonStatsRead>(stat: StatId, view: &ReadPlayerView<'_, R>) -> Option<f64> {
    if !stat.applies_to(view.position(), view.is_goalie()) || !stat.applies_to_era(view.season()) {
        return None;
    }
    stat.read_from(&view.stats).filter(|v| v.is_finite())
}

fn cell(
    stat: StatId,
    values: &[Option<f64>],
    seasons: &[Season],
    value: Option<f64>,
    method: &str,
    applicable: bool,
) -> ProfileMetric {
    let observed = values.iter().filter(|v| v.is_some()).count();
    let status = if !applicable {
        ProfileMetricStatus::NotApplicable
    } else if value.is_some() {
        ProfileMetricStatus::Available
    } else if method == "unsupported" {
        ProfileMetricStatus::AggregationUnsupported
    } else if observed > 0 && observed < values.len() {
        ProfileMetricStatus::IncompleteCoverage
    } else {
        ProfileMetricStatus::Unavailable
    };
    ProfileMetric {
        key: stat.cli_key().into(),
        label: stat.label().into(),
        unit: format!("{:?}", stat.unit()),
        value,
        status,
        aggregation: method.into(),
        observed_seasons: observed,
        missing_seasons: seasons
            .iter()
            .zip(values)
            .filter_map(|(s, v)| v.is_none().then_some(*s))
            .collect(),
        report_source: stat.report_source().map(|r| format!("{r:?}")),
        ratio: None,
    }
}

/// Recompose supported rates from their actual denominators, never average percentages.
fn ratio_components(stat: StatId) -> Option<(StatId, StatId, f64)> {
    use StatId::*;
    Some(match stat {
        ShootingPct => (Goals, Shots, 1.0),
        SavePct => (Saves, ShotsAgainst, 1.0),
        GoalsPerGame => (Goals, Games, 1.0),
        AssistsPerGame => (Assists, Games, 1.0),
        PointsPerGame => (Points, Games, 1.0),
        GoalsPer82 => (Goals, Games, 82.0),
        AssistsPer82 => (Assists, Games, 82.0),
        TotalToiPerGame => (TotalToi, Games, 1.0),
        EvToiPerGame | EvenStrengthTimeOnIcePerGame => (EvToi, Games, 1.0),
        PpToiPerGame => (PpToi, Games, 1.0),
        ShToiPerGame => (ShToi, Games, 1.0),
        ShiftsPerGame => (Shifts, Games, 1.0),
        ToiPerShift => (TotalToi, Shifts, 1.0),
        HitsPer60 => (Hits, TotalToi, 3600.0),
        BlockedShotsPer60 => (BlockedShots, TotalToi, 3600.0),
        TakeawaysPer60 => (Takeaways, TotalToi, 3600.0),
        GiveawaysPer60 => (Giveaways, TotalToi, 3600.0),
        _ => return None,
    })
}

fn summary_metric<R: SeasonStatsRead>(
    stat: StatId,
    views: &[ReadPlayerView<'_, R>],
    seasons: &[Season],
) -> ProfileMetric {
    let applicable = views
        .iter()
        .any(|v| stat.applies_to(v.position(), v.is_goalie()));
    let raw: Vec<_> = views.iter().map(|v| read(stat, v)).collect();
    if views.is_empty() {
        return cell(stat, &raw, seasons, None, "no_rows", true);
    }
    if let Some((numerator, denominator, scale)) = ratio_components(stat) {
        let pairs: Vec<_> = views
            .iter()
            .map(|v| read(numerator, v).zip(read(denominator, v)))
            .collect();
        let observed: Vec<_> = pairs
            .iter()
            .map(|p| p.map(|(n, d)| if d > 0.0 { n / d * scale } else { 0.0 }))
            .collect();
        let totals: Option<(f64, f64)> = pairs
            .iter()
            .try_fold((0.0, 0.0), |(n, d), p| p.map(|(a, b)| (n + a, d + b)));
        let value = totals
            .and_then(|(n, d)| (d > 0.0).then_some(n / d * scale))
            .filter(|v| v.is_finite());
        let mut result = cell(
            stat,
            &observed,
            seasons,
            value,
            "ratio_of_summed_components",
            applicable,
        );
        result.ratio = totals.map(|(n, d)| ProfileRatio {
            numerator_key: numerator.cli_key().into(),
            denominator_key: denominator.cli_key().into(),
            numerator: n,
            denominator: d,
            scale,
        });
        return result;
    }
    let additive = stat.unit() == StatUnit::Count
        || matches!(
            stat,
            StatId::TotalToi | StatId::EvToi | StatId::PpToi | StatId::ShToi
        );
    if additive {
        let value: Option<f64> = raw.iter().copied().sum();
        return cell(
            stat,
            &raw,
            seasons,
            value.filter(|v| v.is_finite()),
            "sum_strict_coverage",
            applicable,
        );
    }
    // Provider/model versions and denominators are not encoded for every catalog
    // rate. Preserve season observations instead of inventing career arithmetic.
    if views.len() == 1 {
        return cell(
            stat,
            &raw,
            seasons,
            raw[0],
            "single_season_catalog_value",
            applicable,
        );
    }
    cell(stat, &raw, seasons, None, "unsupported", applicable)
}

fn families() -> Vec<ProfileJobFamily> {
    use StatId::*;
    [
        ("Win possession", vec![Takeaways, Hits], "Recorded takeaways/contact are context; pressure and controlled acquisition are unmeasured."),
        ("Advance possession", vec![], "Boundary advancement, carries and exits need suitable event/tracking evidence."),
        ("Sustain possession", vec![Giveaways], "Recorded losses are context; possession exposure, retention and support are unmeasured."),
        ("Build threats", vec![Assists, PpPoints], "Production proxies do not isolate passing, off-puck work or causal creation."),
        ("Execute scoring chances", vec![Shots, Goals, ShootingPct, IxG], "Shooting involvement/results and provider xG; no persistent finishing-skill claim."),
        ("Prevent threats", vec![BlockedShots, EvGoalsAgainst, OnIceXgAgainst, SavePct, GoalsSavedAboveExpected], "Coverage-related context and goalie outcomes; no individual duty or causal blame inferred."),
    ].into_iter().map(|(name,ids,interpretation)|ProfileJobFamily {
        name:name.into(), catalog_keys:ids.into_iter().map(|s|s.cli_key().into()).collect(), interpretation:interpretation.into()
    }).collect()
}

pub fn build_rinkcraft_stat_profile(
    repo: &StatsRepository,
    request: RinkcraftProfileRequest,
) -> Result<RinkcraftStatProfile, RinkcraftProfileError> {
    if request
        .first_season
        .zip(request.last_season)
        .is_some_and(|(a, b)| a > b)
    {
        return Err(RinkcraftProfileError::ReversedWindow);
    }
    let identity = repo
        .identity(request.player_id)
        .ok_or(RinkcraftProfileError::UnknownPlayer(request.player_id))?;
    let views: Vec<_> = repo
        .career_all(request.player_id)
        .into_iter()
        .flatten()
        .filter(|s| s.season_type == request.season_type)
        .filter(|s| {
            request.first_season.is_none_or(|first| s.season >= first)
                && request.last_season.is_none_or(|last| s.season <= last)
        })
        .filter_map(|s| repo.view(request.player_id, s.season, s.season_type))
        .collect();
    let mut profile = build_rinkcraft_stat_profile_from_views(&identity.full_name, request, views)?;
    if request.season_type == SeasonType::Regular {
        if let Some(s) = profile.latest_available.as_ref() {
            profile.achievements = super::rinkcraft_achievements::from_league(
                repo.read_league(s.season, request.season_type).collect(),
                s.season.0,
                request.player_id.0,
            )
            .ok()
            .flatten();
        }
    }
    Ok(profile)
}

/// Attach estimates only to canonical rows from the same observed window.
pub fn attach_stat_attribute_catalog(
    profile: &mut RinkcraftStatProfile,
    catalog: &super::rinkcraft_stat_scores::Catalog,
) -> Result<(), String> {
    catalog.validate()?;
    profile.stat_attribute_seasons.clear();
    profile.stat_attribute_changes.clear();
    if profile.request.season_type != SeasonType::Regular {
        return Ok(());
    }
    for row in &profile.timeline {
        if profile
            .request
            .first_season
            .is_some_and(|first| row.season < first)
            || profile
                .request
                .last_season
                .is_some_and(|last| row.season > last)
        {
            continue;
        }
        let selected = profile
            .numerical_evidence
            .as_ref()
            .filter(|n| n.player_id == profile.request.player_id.0 && n.season == row.season.0);
        let group = match row.position {
            Position::Goalie => "Goalies",
            Position::Defense => "Defense",
            _ => "Forwards",
        };
        let games = selected
            .filter(|n| n.group == group)
            .map(|n| n.games)
            .unwrap_or(row.games);
        if let Some(scores) = catalog
            .profile(profile.request.player_id.0, row.season.0, games)
            .filter(|p| p.group == group)
        {
            profile.stat_attribute_seasons.push(scores.clone());
        } else if catalog
            .profiles
            .iter()
            .any(|p| p.player_id == profile.request.player_id.0 && p.season == row.season.0)
        {
            let note=format!("Stat estimates for {} withheld: supplied position/GP does not match the selected {}-GP row.",row.season.0,games);
            if !profile.limitations.contains(&note) {
                profile.limitations.push(note);
            }
        }
    }
    if let Some(n) = profile.numerical_evidence.as_ref().filter(|n| {
        n.player_id == profile.request.player_id.0
            && !profile.timeline.iter().any(|row| row.season.0 == n.season)
            && profile
                .request
                .first_season
                .is_none_or(|first| n.season >= first.0)
            && profile
                .request
                .last_season
                .is_none_or(|last| n.season <= last.0)
    }) {
        if !profile
            .stat_attribute_seasons
            .iter()
            .any(|p| p.season == n.season)
        {
            if let Some(scores) = catalog
                .profile(n.player_id, n.season, n.games)
                .filter(|p| p.group == n.group)
            {
                profile.stat_attribute_seasons.push(scores.clone());
            }
        }
    }
    profile.stat_attribute_changes =
        super::rinkcraft_stat_scores::season_changes(&profile.stat_attribute_seasons);
    let target = profile.numerical_evidence.as_ref().map(|n| n.season)
        .or_else(|| profile.latest_available.as_ref().map(|s| s.season.0));
    if let Some(scores) = profile.stat_attribute_seasons.iter().find(|p| Some(p.season)==target) {
        let current = profile.deployment_evidence.as_ref().is_some_and(|d|
            d.player_id.0==scores.player_id && d.season.0==scores.season && d.games==scores.games
            && d.season_type==SeasonType::Regular && d.goalie==(scores.group=="Goalies"));
        if !current {
            profile.deployment_evidence = super::rinkcraft_deployment::from_stat_profile(scores);
        }
    }

    Ok(())
}

/// Storage-independent builder for prepared native and browser windows.
/// No loading occurs. For duplicate windows the first supplied view wins.
pub fn build_rinkcraft_stat_profile_from_views<R: SeasonStatsRead>(
    player_name: &str,
    request: RinkcraftProfileRequest,
    mut views: Vec<ReadPlayerView<'_, R>>,
) -> Result<RinkcraftStatProfile, RinkcraftProfileError> {
    if request
        .first_season
        .zip(request.last_season)
        .is_some_and(|(a, b)| a > b)
    {
        return Err(RinkcraftProfileError::ReversedWindow);
    }
    views.retain(|v| {
        v.id() == request.player_id
            && v.season_type() == request.season_type
            && request.first_season.is_none_or(|s| v.season() >= s)
            && request.last_season.is_none_or(|s| v.season() <= s)
    });
    views.sort_by_key(|v| v.season());
    views.dedup_by_key(|v| v.season());
    let seasons: Vec<_> = views.iter().map(|v| v.season()).collect();
    let timeline: Vec<_> = views
        .iter()
        .map(|v| ProfileSeason {
            season: v.season(),
            position: v.position(),
            teams: v
                .stats
                .team_stints()
                .iter()
                .map(|s| s.team.0.clone())
                .collect(),
            traded: v.was_traded_in_window(),
            games: v.gp(),
            metrics: StatId::all()
                .iter()
                .map(|&s| {
                    let value = read(s, v);
                    cell(
                        s,
                        &[value],
                        &[v.season()],
                        value,
                        "catalog_read",
                        s.applies_to(v.position(), v.is_goalie()),
                    )
                })
                .collect(),
        })
        .collect();
    let changes = timeline
        .windows(2)
        .map(|pair| ProfileSeasonChange {
            from: pair[0].season,
            to: pair[1].season,
            consecutive: pair[1].season.start_year() == pair[0].season.start_year() + 1,
            previous_games: pair[0].games,
            latest_games: pair[1].games,
            position_changed: pair[0].position != pair[1].position,
            metrics: pair[0]
                .metrics
                .iter()
                .zip(&pair[1].metrics)
                .map(|(a, b)| ProfileMetricChange {
                    key: a.key.clone(),
                    previous: a.value,
                    latest: b.value,
                    difference: a
                        .value
                        .zip(b.value)
                        .map(|(x, y)| y - x)
                        .filter(|v| v.is_finite()),
                })
                .collect(),
        })
        .collect();
    let absent_seasons_in_requested_range = request
        .first_season
        .zip(request.last_season)
        .map(|(first, last)| {
            (first.start_year()..=last.start_year())
                .map(|y| Season(u32::from(y) * 10_000 + u32::from(y) + 1))
                .filter(|s| !seasons.contains(s))
                .collect()
        })
        .unwrap_or_default();
    Ok(RinkcraftStatProfile { goalie_puck_events:None,
        stat_attribute_changes:Vec::new(),stat_attribute_seasons:Vec::new(),achievements:None,
        deployment_evidence:views.last().and_then(|v|super::rinkcraft_deployment::from_read(&v.stats)),
        evidence_guide:super::rinkcraft_evidence_guide::topics(),
        schema:RINKCRAFT_PROFILE_SCHEMA.into(),request,player_name:player_name.into(),
        scope:if request.first_season.is_none() && request.last_season.is_none(){"available_career"}else{"selected_seasons"}.into(),
        league_current_season:Season(crate::CURRENT_SEASON),current_season_available:seasons.contains(&Season(crate::CURRENT_SEASON)),source_windows:Vec::new(),
        included_seasons:seasons.clone(),absent_seasons_in_requested_range,
        latest_available:timeline.last().cloned(),summary:StatId::all().iter().map(|&s|summary_metric(s,&views,&seasons)).collect(),
        timeline,changes,job_families:families(),attribute_seasons:Vec::new(),character_sheet:None,numerical_evidence:None,numerical_snapshot:None,numerical_history:None,
        unmeasured_dimensions:vec!["north_south_east_west_style".into(),"pressure_and_controlled_recovery".into(),"off_puck_availability".into(),"system_assignment_and_error_responsibility".into(),"goalie_depth_stance_and_commitment".into(),"causal_lineup_fit".into()],
        limitations:vec![
            "Available career means loaded repository rows, not verified complete career coverage. Latest means latest loaded season within the selected scope, not necessarily the league's current season.".into(),
            "Season type is isolated. Catalog source/era/trade missingness is preserved; source report names identify catalog routes, not an audited provider snapshot.".into(),
            "Summary values require every selected row's components. Unsupported career rates/model outputs remain available by season; no GP-weighted percentage average is substituted.".into(),
            "Changes are arithmetic catalog differences, not statistical change-point detection, exposure adjustment or evidence of changed role/ability. Counts depend on games and deployment.".into(),
            "Job families are nonadditive evidence navigation, not validated class scores. Exact date ranges require game-level inputs; this contract selects whole seasons.".into()
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fixtures,
        season_stats::{GoalieSeasonStats, RealtimeStats, StatTotals, TeamStint},
    };

    fn repo_row(repo: &mut StatsRepository, season: Season, gp: u32, goals: u32, shots: u32) {
        let mut row = fixtures::stats(1, season.0, "AAA").build();
        row.totals = StatTotals {
            gp,
            goals,
            points: goals,
            shots,
            ..Default::default()
        };
        row.team_stints = vec![TeamStint {
            team: crate::model::TeamAbbr("AAA".into()),
            started: None,
            ended: None,
            gp,
            goals,
            assists: 0,
            points: goals,
            goalie: None,
        }];
        row.realtime = None;
        row.advanced = None;
        row.time_on_ice = None;
        row.goals_for_against = None;
        repo.upsert_stats(row).unwrap();
    }
    fn request() -> RinkcraftProfileRequest {
        RinkcraftProfileRequest {
            player_id: PlayerId(1),
            season_type: SeasonType::Regular,
            first_season: None,
            last_season: None,
        }
    }
    fn metric(profile: &RinkcraftStatProfile, stat: StatId) -> &ProfileMetric {
        profile
            .summary
            .iter()
            .find(|m| m.key == stat.cli_key())
            .unwrap()
    }
    fn repo() -> StatsRepository {
        let mut r = StatsRepository::with_lru_cap(80);
        r.upsert_identity(
            fixtures::identity(1)
                .name("Test Skater", "test skater")
                .build(),
        )
        .unwrap();
        r
    }

    #[test]
    fn rinkcraft_profile_recomposes_shooting_percentage_and_keeps_missing_optional_reports() {
        let mut r = repo();
        let y = Season(crate::CURRENT_SEASON);
        repo_row(&mut r, y, 10, 5, 10);
        repo_row(&mut r, Season(y.0 - 10001), 80, 10, 100);
        let p = build_rinkcraft_stat_profile(&r, request()).unwrap();
        assert!((metric(&p, StatId::ShootingPct).value.unwrap() - 15.0 / 110.0).abs() < 1e-12);
        assert_eq!(metric(&p, StatId::Goals).value, Some(15.0));
        assert_eq!(metric(&p, StatId::Hits).value, None);
        assert_eq!(p.latest_available.unwrap().season, y);
    }
    #[test]
    fn rinkcraft_profile_excludes_playoffs_and_marks_unloaded_seasons() {
        let mut r = repo();
        let y = Season(crate::CURRENT_SEASON);
        repo_row(&mut r, y, 10, 3, 30);
        let mut playoff = fixtures::stats(1, y.0, "AAA").build();
        playoff.season_type = SeasonType::Playoff;
        r.upsert_stats(playoff).unwrap();
        let p = build_rinkcraft_stat_profile(
            &r,
            RinkcraftProfileRequest {
                first_season: Some(Season(y.0 - 10001)),
                last_season: Some(y),
                ..request()
            },
        )
        .unwrap();
        assert_eq!(metric(&p, StatId::Goals).value, Some(3.0));
        assert_eq!(
            p.absent_seasons_in_requested_range,
            vec![Season(y.0 - 10001)]
        );
        assert_eq!(p.timeline.len(), 1);
    }

    #[test]
    fn rinkcraft_profile_time_rates_use_ice_time_and_report_numeric_changes_only() {
        use crate::season_stats::TimeOnIceStats;
        let mut r = repo();
        let y = Season(crate::CURRENT_SEASON);
        for (season, gp, hits) in [(Season(y.0 - 10001), 1, 10), (y, 100, 20)] {
            repo_row(&mut r, season, gp, 0, 0);
            let mut row = r
                .season(PlayerId(1), season, SeasonType::Regular)
                .unwrap()
                .clone();
            row.realtime = Some(RealtimeStats {
                hits,
                ..Default::default()
            });
            row.time_on_ice = Some(TimeOnIceStats {
                time_on_ice_sec: 3600,
                ..Default::default()
            });
            r.upsert_stats(row).unwrap();
        }
        let p = build_rinkcraft_stat_profile(&r, request()).unwrap();
        assert_eq!(metric(&p, StatId::HitsPer60).value, Some(15.0));
        assert!(
            (metric(&p, StatId::TotalToiPerGame).value.unwrap() - 7200.0 / 101.0).abs() < 1e-12
        );
        assert_eq!(
            metric(&p, StatId::HitsPer60)
                .ratio
                .as_ref()
                .unwrap()
                .denominator,
            7200.0
        );
        assert!(p.changes[0].consecutive);
        assert_eq!(
            p.changes[0]
                .metrics
                .iter()
                .find(|m| m.key == StatId::Hits.cli_key())
                .unwrap()
                .difference,
            Some(10.0)
        );
        assert!(p
            .unmeasured_dimensions
            .iter()
            .any(|s| s.contains("system_assignment")));
    }
    #[test]
    fn rinkcraft_profile_does_not_fill_missing_season_or_average_model_rates() {
        let mut r = repo();
        let y = Season(crate::CURRENT_SEASON);
        repo_row(&mut r, y, 10, 0, 0);
        repo_row(&mut r, Season(y.0 - 10001), 20, 0, 0);
        let mut row = r
            .view(PlayerId(1), y, SeasonType::Regular)
            .unwrap()
            .stats
            .clone();
        row.realtime = Some(RealtimeStats {
            hits: 0,
            ..Default::default()
        });
        r.upsert_stats(row).unwrap();
        let p = build_rinkcraft_stat_profile(&r, request()).unwrap();
        assert_eq!(
            metric(&p, StatId::Hits).status,
            ProfileMetricStatus::IncompleteCoverage
        );
        assert_eq!(metric(&p, StatId::Hits).value, None);
        assert_eq!(metric(&p, StatId::ShootingPct).value, None);
        assert_eq!(
            metric(&p, StatId::IxgPer60).status,
            ProfileMetricStatus::AggregationUnsupported
        );
    }
    #[test]
    fn rinkcraft_profile_goalie_save_percentage_uses_shots_not_games() {
        let mut r = repo();
        let y = Season(crate::CURRENT_SEASON);
        for (season, gp, saves, shots) in [(y, 2, 9, 10), (Season(y.0 - 10001), 1, 80, 100)] {
            let mut row = fixtures::stats(1, season.0, "AAA").build();
            row.position = Position::Goalie;
            row.goalie = Some(GoalieSeasonStats {
                saves,
                shots_against: shots,
                goals_against: shots - saves,
                ..Default::default()
            });
            row.totals.gp = gp;
            row.team_stints[0].gp = gp;
            r.upsert_stats(row).unwrap();
        }
        let p = build_rinkcraft_stat_profile(&r, request()).unwrap();
        assert!((metric(&p, StatId::SavePct).value.unwrap() - 89.0 / 110.0).abs() < 1e-12);
        assert_eq!(
            metric(&p, StatId::Goals).status,
            ProfileMetricStatus::NotApplicable
        );
    }

    #[test]
    fn rinkcraft_profile_reading_guide_is_additive_to_legacy_records() {
        let mut r = repo();
        repo_row(&mut r, Season(crate::CURRENT_SEASON), 10, 2, 3);
        let p = build_rinkcraft_stat_profile(&r, request()).unwrap();
        assert!(!p.evidence_guide.is_empty());
        let mut legacy = serde_json::to_value(&p).unwrap();
        legacy.as_object_mut().unwrap().remove("evidence_guide");
        let restored: RinkcraftStatProfile = serde_json::from_value(legacy).unwrap();
        assert!(restored.evidence_guide.is_empty());
        assert_eq!(restored.summary, p.summary);
        assert_eq!(restored.timeline, p.timeline);
        assert_eq!(restored.character_sheet, p.character_sheet);
        assert_eq!(restored.attribute_seasons, p.attribute_seasons);
    }
    #[test]
    fn rinkcraft_profile_empty_unknown_and_reversed_windows_are_explicit() {
        let r = repo();
        let y = Season(crate::CURRENT_SEASON);
        assert!(build_rinkcraft_stat_profile(&r, request())
            .unwrap()
            .latest_available
            .is_none());
        assert_eq!(
            build_rinkcraft_stat_profile(
                &r,
                RinkcraftProfileRequest {
                    player_id: PlayerId(2),
                    ..request()
                }
            )
            .unwrap_err(),
            RinkcraftProfileError::UnknownPlayer(PlayerId(2))
        );
        assert_eq!(
            build_rinkcraft_stat_profile(
                &r,
                RinkcraftProfileRequest {
                    first_season: Some(y),
                    last_season: Some(Season(y.0 - 10001)),
                    ..request()
                }
            )
            .unwrap_err(),
            RinkcraftProfileError::ReversedWindow
        );
    }
}
