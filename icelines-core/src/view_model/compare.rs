use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Position, Season};
use crate::season_stats::SeasonType;
use crate::stats_catalog::StatId;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
    ViewWindow,
};
use crate::view_model::player_card::PlayerCardView;
use crate::view_model::tokens::{MetricCell, MetricUnit, MetricValue, StatKey, ValuePrecision};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompareView {
    pub context: ViewContext,
    pub a: Option<PlayerCardView>,
    pub b: Option<PlayerCardView>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarPlayersView {
    pub context: ViewContext,
    pub target: SimilarPlayerTarget,
    pub cohort_count: usize,
    pub rows: Vec<SimilarPlayerRow>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarPlayerTarget {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team_display: String,
    pub position: Position,
    pub age: Option<u8>,
    pub draft_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarPlayerRow {
    pub rank: usize,
    pub player_id: PlayerId,
    pub display_name: String,
    pub team_display: String,
    pub position: Position,
    pub age: Option<u8>,
    pub draft_label: String,
    pub similarity_pct: u32,
    pub distance: f64,
    pub metrics: Vec<MetricCell>,
}

impl CompareView {
    pub fn from_repository(
        repo: &StatsRepository,
        a: Option<PlayerId>,
        b: Option<PlayerId>,
        season: Season,
        season_type: SeasonType,
    ) -> Self {
        let has_window = repo.has_window(season, season_type);
        let mut context = ViewContext::new(ViewWindow::new(season, season_type));
        if !has_window {
            context.completeness = Completeness::Unavailable;
            context
                .source_state
                .push(SourceState::missing(SourceKind::Roster));
        }

        let a = a.and_then(|id| PlayerCardView::from_repository(repo, id, season, season_type));
        let b = b.and_then(|id| PlayerCardView::from_repository(repo, id, season, season_type));
        let empty_state = if a.is_none() && b.is_none() {
            Some(EmptyState {
                kind: if has_window {
                    EmptyKind::NoRows
                } else {
                    EmptyKind::MissingSource
                },
                title: "No comparable players".to_string(),
                detail: Some("No resolved player ids produced compare cards.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            a,
            b,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

impl SimilarPlayersView {
    pub fn from_player_views(
        views: &[PlayerView<'_>],
        target: &PlayerView<'_>,
        n: usize,
        season: Season,
        season_type: SeasonType,
        has_window: bool,
    ) -> Self {
        let target_age = view_age(target);
        let cohort: Vec<&PlayerView<'_>> = views
            .iter()
            .filter(|view| {
                view.position() == target.position()
                    && view.is_rankable()
                    && view_age(view)
                        .zip(target_age)
                        .map(|(age, target_age)| (age as i32 - target_age as i32).abs() <= 2)
                        .unwrap_or(false)
            })
            .collect();

        let target_summary = SimilarPlayerTarget {
            player_id: target.identity.id,
            display_name: target.identity.full_name.clone(),
            team_display: target.team_display().to_string(),
            position: target.position(),
            age: target_age,
            draft_label: draft_label(target),
        };

        let mut context = ViewContext::new(ViewWindow::new(season, season_type));
        if !has_window {
            context.completeness = Completeness::Unavailable;
            context
                .source_state
                .push(SourceState::missing(SourceKind::Roster));
        }

        if cohort.len() < 3 {
            return Self {
                context,
                target: target_summary,
                cohort_count: cohort.len(),
                rows: Vec::new(),
                warnings: Vec::new(),
                empty_state: Some(EmptyState {
                    kind: if has_window {
                        EmptyKind::NoRows
                    } else {
                        EmptyKind::MissingSource
                    },
                    title: "Similarity cohort too small".to_string(),
                    detail: Some(format!(
                        "Need at least 3 same-position players aged within 2 years; found {}.",
                        cohort.len()
                    )),
                    recovery: Vec::new(),
                }),
            };
        }

        let ppgs: Vec<f64> = cohort
            .iter()
            .map(|view| StatId::PointsPerGame.read(view).unwrap_or(0.0))
            .collect();
        let gpgs: Vec<f64> = cohort
            .iter()
            .map(|view| StatId::GoalsPerGame.read(view).unwrap_or(0.0))
            .collect();
        let picks: Vec<f64> = cohort.iter().map(|view| draft_pick_score(view)).collect();

        let (ppg_mu, ppg_sd) = mean_std(&ppgs);
        let (gpg_mu, gpg_sd) = mean_std(&gpgs);
        let (pick_mu, pick_sd) = mean_std(&picks);

        let target_norm = &target.identity.name_normalized;
        let target_index = cohort
            .iter()
            .position(|view| &view.identity.name_normalized == target_norm)
            .unwrap_or(0);
        let target_z_ppg = zscore(ppgs[target_index], ppg_mu, ppg_sd);
        let target_z_gpg = zscore(gpgs[target_index], gpg_mu, gpg_sd);
        let target_z_pick = zscore(picks[target_index], pick_mu, pick_sd);

        let mut scored: Vec<(&PlayerView<'_>, f64)> = cohort
            .iter()
            .zip(ppgs.iter())
            .zip(gpgs.iter())
            .zip(picks.iter())
            .map(|(((view, &ppg), &gpg), &pick)| {
                let dz_ppg = zscore(ppg, ppg_mu, ppg_sd) - target_z_ppg;
                let dz_gpg = zscore(gpg, gpg_mu, gpg_sd) - target_z_gpg;
                let dz_pick = zscore(pick, pick_mu, pick_sd) - target_z_pick;
                let distance = (dz_ppg * dz_ppg + dz_gpg * dz_gpg + dz_pick * dz_pick).sqrt();
                (*view, distance)
            })
            .collect();

        scored.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.identity.id.0.cmp(&b.0.identity.id.0))
        });
        scored.retain(|(view, _)| &view.identity.name_normalized != target_norm);

        let rows: Vec<SimilarPlayerRow> = scored
            .into_iter()
            .take(n)
            .enumerate()
            .map(|(idx, (view, distance))| {
                let points_per_game = StatId::PointsPerGame.read(view);
                SimilarPlayerRow {
                    rank: idx + 1,
                    player_id: view.identity.id,
                    display_name: view.identity.full_name.clone(),
                    team_display: view.team_display().to_string(),
                    position: view.position(),
                    age: view_age(view),
                    draft_label: draft_label(view),
                    similarity_pct: (100.0 / (1.0 + distance)) as u32,
                    distance,
                    metrics: vec![MetricCell {
                        key: StatKey::from("points_per_game"),
                        label: "PPG".to_string(),
                        value: points_per_game
                            .map(MetricValue::Decimal)
                            .unwrap_or(MetricValue::Missing),
                        unit: MetricUnit::PerGame,
                        precision: ValuePrecision::ThreeDecimals,
                        token: None,
                    }],
                }
            })
            .collect();

        Self {
            context,
            target: target_summary,
            cohort_count: cohort.len(),
            rows,
            warnings: Vec::new(),
            empty_state: None,
        }
    }
}

fn view_age(view: &PlayerView<'_>) -> Option<u8> {
    view.identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<u32>().ok())
        .map(|birth_year| 2026u32.saturating_sub(birth_year) as u8)
}

fn draft_label(view: &PlayerView<'_>) -> String {
    let bio = &view.identity.bio;
    match (bio.draft_year, bio.draft_round, bio.draft_overall) {
        (Some(year), Some(round), Some(overall)) => format!("{year} R{round}#{overall}"),
        (Some(year), _, _) => year.to_string(),
        _ => "UD".to_string(),
    }
}

fn draft_pick_score(view: &PlayerView<'_>) -> f64 {
    view.identity
        .bio
        .draft_overall
        .map(|pick| 1.0 - (pick as f64 - 1.0) / 399.0)
        .unwrap_or(0.0)
}

fn mean_std(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n == 0.0 {
        return (0.0, 1.0);
    }
    let mean = values.iter().sum::<f64>() / n;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / n;
    let std = variance.sqrt();
    (mean, if std < 1e-10 { 1.0 } else { std })
}

fn zscore(value: f64, mean: f64, std: f64) -> f64 {
    (value - mean) / std
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::model::{PaceScore, Position, Season};
    use crate::season_stats::SeasonType;
    use crate::stats_repository::StatsRepository;

    fn repo_with_players(seeds: &[(u32, &str, Position, u32, u32)]) -> StatsRepository {
        let mut repo = StatsRepository::new();
        for &(id, name, position, goals, points) in seeds {
            let normalized = crate::name::normalize_name(name);
            let identity = fixtures::identity(id).name(name, &normalized).build();
            let mut stats = fixtures::stats(id, 20242025, "EDM")
                .position(position)
                .build();
            stats.totals.goals = goals;
            stats.totals.assists = points.saturating_sub(goals);
            stats.totals.points = points;
            stats.totals.pace_score = Some(PaceScore {
                pace_82: points as f64 / stats.totals.gp as f64 * 82.0,
                goals_per_82: goals as f64 / stats.totals.gp as f64 * 82.0,
                raw_points: points,
                gp: stats.totals.gp,
            });
            repo.upsert_identity(identity).expect("identity fixture");
            repo.upsert_stats(stats).expect("stats fixture");
        }
        repo
    }

    #[test]
    fn similar_players_view_excludes_target_and_uses_same_position_rows() {
        let repo = repo_with_players(&[
            (1, "Target", Position::Center, 30, 90),
            (2, "Close Center", Position::Center, 29, 88),
            (3, "Far Center", Position::Center, 12, 40),
            (4, "Wing", Position::LeftWing, 30, 90),
        ]);
        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let target = views.iter().find(|view| view.identity.id.0 == 1).unwrap();

        let view = SimilarPlayersView::from_player_views(
            &views,
            target,
            20,
            Season(20242025),
            SeasonType::Regular,
            true,
        );

        assert!(view.empty_state.is_none());
        assert!(view.rows.iter().all(|row| row.player_id.0 != 1));
        assert!(view.rows.iter().all(|row| row.position == Position::Center));
        assert_eq!(view.rows[0].display_name, "Close Center");
    }
}
