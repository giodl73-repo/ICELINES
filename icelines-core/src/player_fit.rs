//! Opportunity evidence, separate from player quality and trade valuation.
//! Windows never cross seasons/teams. Missing minutes stay missing; role ranges
//! are supplied evidence, never inferred from scoring-sorted depth-chart rows.
use crate::view_model::context::{Completeness, ViewContext};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageGame {
    pub game_id: u64,
    pub date: NaiveDate,
    pub team: String,
    pub toi_seconds: Option<u32>,
    pub ev_seconds: Option<u32>,
    pub pp_seconds: Option<u32>,
    pub pk_seconds: Option<u32>,
    pub points: u32,
    pub shots: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleReference {
    pub label: String,
    pub evidence_url: String,
    pub comparable_players: usize,
    pub total_toi_low_seconds: f64,
    pub total_toi_high_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationOpportunity {
    pub team: String,
    pub proposed_role: String,
    pub hypothetical_total_toi_seconds: f64,
    pub rationale: String,
    pub evidence_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerFitInput {
    pub context: ViewContext,
    pub player_id: u32,
    pub team: String,
    pub cutoff: NaiveDate,
    pub window_games: usize,
    pub minimum_games: usize,
    /// Explicit analyst sensitivity threshold, not a calibrated statistical test.
    pub trend_threshold_seconds: f64,
    pub source_urls: Vec<String>,
    pub games: Vec<UsageGame>,
    pub role_reference: Option<RoleReference>,
    #[serde(default)]
    pub destinations: Vec<DestinationOpportunity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    pub games: usize,
    pub toi_observed_games: usize,
    pub first_date: Option<NaiveDate>,
    pub last_date: Option<NaiveDate>,
    pub total_toi_mean_seconds: Option<f64>,
    pub total_toi_sd_seconds: Option<f64>,
    pub total_toi_cv: Option<f64>,
    pub ev_mean_seconds: Option<f64>,
    pub pp_mean_seconds: Option<f64>,
    pub pk_mean_seconds: Option<f64>,
    pub points_per_60_all_situations: Option<f64>,
    pub shots_per_60_all_situations: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageTrend {
    Increasing,
    Stable,
    Decreasing,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleOpportunity {
    BelowReference,
    WithinReference,
    AboveReference,
    Unrated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationGap {
    pub scenario: DestinationOpportunity,
    pub hypothetical_toi_gain_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerFitView {
    pub schema: String,
    pub context: ViewContext,
    pub player_id: u32,
    pub team: String,
    pub cutoff: NaiveDate,
    pub window_games: usize,
    pub minimum_games: usize,
    pub trend_threshold_seconds: f64,
    pub source_urls: Vec<String>,
    pub recent: UsageWindow,
    pub previous: UsageWindow,
    pub total_toi_delta_seconds: Option<f64>,
    pub trend: UsageTrend,
    pub role_reference: Option<RoleReference>,
    pub role_opportunity: RoleOpportunity,
    pub destinations: Vec<DestinationGap>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}
fn complete_mean(games: &[UsageGame], field: fn(&UsageGame) -> Option<u32>) -> Option<f64> {
    let values: Option<Vec<f64>> = games.iter().map(|g| field(g).map(f64::from)).collect();
    values.and_then(|v| mean(&v))
}
fn window(games: &[UsageGame]) -> UsageWindow {
    let values: Vec<f64> = games
        .iter()
        .filter_map(|g| g.toi_seconds.map(f64::from))
        .collect();
    let avg = mean(&values);
    let sd = (values.len() >= 2).then(|| {
        let m = avg.unwrap();
        (values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64).sqrt()
    });
    let complete = !games.is_empty() && values.len() == games.len();
    let seconds = values.iter().sum::<f64>();
    let rate =
        |count: u32| (complete && seconds > 0.0).then_some(f64::from(count) * 3600.0 / seconds);
    UsageWindow {
        games: games.len(),
        toi_observed_games: values.len(),
        first_date: games.first().map(|g| g.date),
        last_date: games.last().map(|g| g.date),
        total_toi_mean_seconds: avg,
        total_toi_sd_seconds: sd,
        total_toi_cv: sd.zip(avg).and_then(|(s, m)| (m > 0.0).then_some(s / m)),
        ev_mean_seconds: complete_mean(games, |g| g.ev_seconds),
        pp_mean_seconds: complete_mean(games, |g| g.pp_seconds),
        pk_mean_seconds: complete_mean(games, |g| g.pk_seconds),
        points_per_60_all_situations: rate(games.iter().map(|g| g.points).sum()),
        shots_per_60_all_situations: rate(games.iter().map(|g| g.shots).sum()),
    }
}

pub fn build_player_fit(mut input: PlayerFitInput) -> Result<PlayerFitView, String> {
    if input.player_id == 0
        || input.team.trim().is_empty()
        || input.source_urls.is_empty()
        || input.minimum_games < 2
        || input.window_games < input.minimum_games
        || !input.trend_threshold_seconds.is_finite()
        || input.trend_threshold_seconds <= 0.0
    {
        return Err("invalid identity, sources, window, or trend threshold".into());
    }
    let start_year = input.context.window.season.0 / 10000;
    let end_year = input.context.window.season.0 % 10000;
    if end_year != start_year + 1 {
        return Err("invalid season".into());
    }
    let start = NaiveDate::from_ymd_opt(start_year as i32, 7, 1).ok_or("invalid season start")?;
    let end = NaiveDate::from_ymd_opt(end_year as i32, 7, 1).ok_or("invalid season end")?;
    let mut ids = BTreeSet::new();
    for g in &input.games {
        if g.date > input.cutoff
            || g.date < start
            || g.date >= end
            || g.team != input.team
            || !ids.insert(g.game_id)
            || g.game_id == 0
        {
            return Err("future, cross-season, cross-team, or duplicate game".into());
        }
        if let (Some(total), Some(ev), Some(pp), Some(pk)) =
            (g.toi_seconds, g.ev_seconds, g.pp_seconds, g.pk_seconds)
        {
            if u64::from(ev) + u64::from(pp) + u64::from(pk) > u64::from(total) {
                return Err("situation minutes exceed total TOI".into());
            }
        }
    }
    if let Some(r) = &input.role_reference {
        if r.label.is_empty()
            || r.evidence_url.is_empty()
            || r.comparable_players < 5
            || !r.total_toi_low_seconds.is_finite()
            || !r.total_toi_high_seconds.is_finite()
            || r.total_toi_low_seconds < 0.0
            || r.total_toi_high_seconds < r.total_toi_low_seconds
        {
            return Err(
                "role reference requires five comparable players and a valid range/source".into(),
            );
        }
    }
    for d in &input.destinations {
        if d.team.is_empty()
            || d.team == input.team
            || d.proposed_role.is_empty()
            || d.rationale.is_empty()
            || d.evidence_urls.is_empty()
            || !d.hypothetical_total_toi_seconds.is_finite()
            || d.hypothetical_total_toi_seconds < 0.0
        {
            return Err("invalid destination scenario".into());
        }
    }
    input.games.sort_by_key(|g| (g.date, g.game_id));
    let n = input.games.len();
    let split = n.saturating_sub(input.window_games);
    let recent = window(&input.games[split..]);
    let previous = window(&input.games[split.saturating_sub(input.window_games)..split]);
    let usable =
        |w: &UsageWindow| w.games >= input.minimum_games && w.toi_observed_games == w.games;
    let sources_usable = input.context.completeness != Completeness::Stale
        && input.context.completeness != Completeness::Unavailable;
    let delta = (usable(&recent) && usable(&previous) && sources_usable)
        .then(|| recent.total_toi_mean_seconds.unwrap() - previous.total_toi_mean_seconds.unwrap());
    let trend = match delta {
        Some(d) if d >= input.trend_threshold_seconds => UsageTrend::Increasing,
        Some(d) if d <= -input.trend_threshold_seconds => UsageTrend::Decreasing,
        Some(_) => UsageTrend::Stable,
        None => UsageTrend::InsufficientEvidence,
    };
    let role_opportunity = match (&input.role_reference, recent.total_toi_mean_seconds) {
        (Some(r), Some(m)) if usable(&recent) && sources_usable => {
            if m < r.total_toi_low_seconds {
                RoleOpportunity::BelowReference
            } else if m > r.total_toi_high_seconds {
                RoleOpportunity::AboveReference
            } else {
                RoleOpportunity::WithinReference
            }
        }
        _ => RoleOpportunity::Unrated,
    };
    let mut warnings = Vec::new();
    if delta.is_none() {
        warnings.push("Two complete, non-overlapping windows are required for a trend.".into());
    }
    if input.role_reference.is_none() {
        warnings.push("Observed role peer reference unavailable; depth scoring rank is not coaching deployment.".into());
    }
    if recent.ev_mean_seconds.is_none()
        || recent.pp_mean_seconds.is_none()
        || recent.pk_mean_seconds.is_none()
    {
        warnings.push(
            "Situation TOI incomplete; total minutes cannot identify EV/PP/PK opportunity changes."
                .into(),
        );
    }
    if recent.toi_observed_games < recent.games {
        warnings.push(
            "Total TOI mean covers observed games only; rates and classifications withheld.".into(),
        );
    }
    let destinations = input
        .destinations
        .into_iter()
        .map(|scenario| DestinationGap {
            hypothetical_toi_gain_seconds: if usable(&recent) && sources_usable {
                recent
                    .total_toi_mean_seconds
                    .map(|m| scenario.hypothetical_total_toi_seconds - m)
            } else {
                None
            },
            scenario,
        })
        .collect();
    Ok(PlayerFitView {
        schema: "player_team_opportunity.v1".into(), context: input.context,
        player_id: input.player_id, team: input.team, cutoff: input.cutoff,
        window_games: input.window_games, minimum_games: input.minimum_games,
        trend_threshold_seconds: input.trend_threshold_seconds, source_urls: input.source_urls,
        recent, previous, total_toi_delta_seconds: delta, trend,
        role_reference: input.role_reference, role_opportunity, destinations, warnings,
        disclosures: vec![
            "Descriptive usage change and sample SD/CV; thresholds are analyst settings, not significance tests.".into(),
            "All-situation scoring rates are descriptive, not EV talent or teammate-adjusted impact estimates.".into(),
            "Destination opportunities are hypothetical; no coaching assignment or trade return is predicted.".into(),
            "Trade decisions additionally require contract/control, replacement cost, team objectives, and an evidenced return.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::Season, season_stats::SeasonType, view_model::context::ViewWindow};
    fn input() -> PlayerFitInput {
        PlayerFitInput {
            context: ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular)),
            player_id: 1,
            team: "AAA".into(),
            cutoff: NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            window_games: 5,
            minimum_games: 5,
            trend_threshold_seconds: 60.0,
            source_urls: vec!["https://example.org/fixture".into()],
            role_reference: None,
            destinations: vec![],
            games: (1..=10)
                .map(|i| UsageGame {
                    game_id: i,
                    date: NaiveDate::from_ymd_opt(2026, 10, i as u32).unwrap(),
                    team: "AAA".into(),
                    toi_seconds: Some(if i <= 5 { 600 } else { 720 }),
                    ev_seconds: None,
                    pp_seconds: None,
                    pk_seconds: None,
                    points: 1,
                    shots: 2,
                })
                .collect(),
        }
    }
    #[test]
    fn disjoint_windows_and_order() {
        let mut i = input();
        i.games.reverse();
        let v = build_player_fit(i).unwrap();
        assert_eq!(v.total_toi_delta_seconds, Some(120.0));
        assert_eq!(v.trend, UsageTrend::Increasing);
        assert!(v.previous.last_date < v.recent.first_date);
    }
    #[test]
    fn missing_minutes_and_small_samples_do_not_classify() {
        let mut i = input();
        i.games[8].toi_seconds = None;
        let v = build_player_fit(i).unwrap();
        assert_eq!(v.trend, UsageTrend::InsufficientEvidence);
        assert_eq!(v.recent.points_per_60_all_situations, None);
        let mut i = input();
        i.games.truncate(3);
        assert_eq!(
            build_player_fit(i).unwrap().trend,
            UsageTrend::InsufficientEvidence
        );
    }
    #[test]
    fn reject_leakage_duplicates_and_cross_team() {
        let mut i = input();
        i.games[0].date = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
        assert!(build_player_fit(i).is_err());
        let mut i = input();
        i.games[0].game_id = 2;
        assert!(build_player_fit(i).is_err());
        let mut i = input();
        i.games[0].team = "BBB".into();
        assert!(build_player_fit(i).is_err());
        let mut i = input();
        i.games[0].date = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
        assert!(build_player_fit(i).is_err());
    }
    #[test]
    fn stale_data_cannot_classify() {
        let mut i = input();
        i.context.completeness = Completeness::Stale;
        assert_eq!(
            build_player_fit(i).unwrap().trend,
            UsageTrend::InsufficientEvidence
        );
    }
}
