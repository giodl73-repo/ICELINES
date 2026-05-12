use serde::{Deserialize, Serialize};

use crate::model::Season;
use crate::name::normalize_name;
use crate::scheme::Scheme;
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{Completeness, SourceKind, SourceState, ViewContext, ViewWindow};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyRosterGapView {
    pub context: ViewContext,
    pub league: String,
    pub team: String,
    pub scoring_scheme: String,
    pub categories: Vec<String>,
    pub rows: Vec<FantasyRosterGapRow>,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyRosterGapRow {
    pub category: String,
    pub user_total: f64,
    pub weight: f64,
    pub action: FantasyRosterGapAction,
    pub action_reason: String,
    pub best_available: Option<FantasyRosterGapCandidate>,
    pub gap_score: f64,
    pub weighted_gap_score: f64,
    pub replacement_target: Option<FantasyRosterGapReplacement>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyRosterGapAction {
    AddNow,
    Watch,
    NoAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyRosterGapCandidate {
    pub player_id: u32,
    pub display_name: String,
    pub team: String,
    pub position: String,
    pub value: f64,
    pub weighted_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyRosterGapReplacement {
    pub player_id: u32,
    pub display_name: String,
    pub team: String,
    pub position: String,
    pub value: f64,
    pub weighted_value: f64,
    pub delta: f64,
    pub weighted_delta: f64,
}

pub struct FantasyRosterGapInput<'a> {
    pub season: Season,
    pub season_type: SeasonType,
    pub league: &'a str,
    pub team: &'a str,
    pub scoring_scheme: &'a str,
    pub categories: Vec<String>,
    pub user_roster_keys: Vec<String>,
    pub all_rostered_keys: Vec<String>,
    pub limit: usize,
}

impl FantasyRosterGapView {
    pub fn from_repository(repo: &StatsRepository, input: FantasyRosterGapInput<'_>) -> Self {
        let mut context = ViewContext::new(ViewWindow::new(input.season, input.season_type));
        let has_window = repo.has_window(input.season, input.season_type);
        context.completeness = if has_window {
            Completeness::Partial
        } else {
            Completeness::Unavailable
        };
        context.source_state = vec![
            SourceState::complete(SourceKind::Roster),
            SourceState::complete(SourceKind::FantasyImport),
        ];

        let scheme = Scheme::builtin_named(input.scoring_scheme);
        let categories = effective_categories(input.scoring_scheme, input.categories);
        let user_keys = input
            .user_roster_keys
            .iter()
            .map(|key| normalize_name(key))
            .collect::<Vec<_>>();
        let all_rostered = input
            .all_rostered_keys
            .iter()
            .map(|key| normalize_name(key))
            .collect::<std::collections::BTreeSet<_>>();
        let skaters = repo
            .skaters(input.season, input.season_type)
            .collect::<Vec<_>>();
        let user_roster = skaters
            .iter()
            .copied()
            .filter(|player| contains_player(&user_keys, player))
            .collect::<Vec<_>>();
        let available = skaters
            .iter()
            .copied()
            .filter(|player| !all_rostered.contains(&player.identity.name_normalized))
            .collect::<Vec<_>>();

        let mut warnings = Vec::new();
        if !has_window {
            warnings.push("requested season/type window is not loaded".to_string());
        }
        if user_roster.is_empty() {
            warnings.push("no marked user roster players resolved in the active pool".to_string());
        }
        if available.is_empty() {
            warnings.push("no available skater pool resolved from imported rosters".to_string());
        }

        let mut rows = categories
            .iter()
            .map(|category| {
                let weight = category_weight(scheme.as_ref(), category);
                let user_total = user_roster
                    .iter()
                    .map(|player| category_value(player, category))
                    .sum::<f64>();
                let best_available = best_candidate(&available, category, weight);
                let gap_score = best_available
                    .as_ref()
                    .map(|candidate| candidate.value)
                    .unwrap_or(0.0);
                let replacement_target = best_available.as_ref().and_then(|candidate| {
                    replacement_target(&user_roster, category, &candidate.position, weight)
                        .map(|target| FantasyRosterGapReplacement {
                            delta: candidate.value - target.value,
                            weighted_delta: candidate.weighted_value - target.weighted_value,
                            ..target
                        })
                });
                let weighted_gap_score = replacement_target
                    .as_ref()
                    .map(|target| target.weighted_delta)
                    .or_else(|| {
                        best_available
                            .as_ref()
                            .map(|candidate| candidate.weighted_value)
                    })
                    .unwrap_or(0.0);
                let (action, action_reason) =
                    classify_action(best_available.as_ref(), replacement_target.as_ref(), weighted_gap_score);
                let recommendation = match &best_available {
                    Some(candidate) if action == FantasyRosterGapAction::NoAction => format!(
                        "{} is the best available {} contributor, but the resolved replacement delta is not positive.",
                        candidate.display_name, category
                    ),
                    Some(candidate) if replacement_target.is_some() => {
                        let target = replacement_target.as_ref().expect("checked above");
                        format!(
                            "{} adds {:.1} weighted {} over {} at {}.",
                            candidate.display_name,
                            target.weighted_delta,
                            category,
                            target.display_name,
                            candidate.position
                        )
                    }
                    Some(candidate) if user_total <= f64::EPSILON => format!(
                        "{} is the best available {} contributor; your marked roster has no resolved {} total.",
                        candidate.display_name, category, category
                    ),
                    Some(candidate) => format!(
                        "{} leads available {} help at {:.1} ({:.1} weighted); your roster total is {:.1}.",
                        candidate.display_name,
                        category,
                        candidate.value,
                        candidate.weighted_value,
                        user_total
                    ),
                    None => format!("No available candidate resolved for {category}."),
                };
                FantasyRosterGapRow {
                    category: category.clone(),
                    user_total,
                    weight,
                    action,
                    action_reason,
                    best_available,
                    gap_score,
                    weighted_gap_score,
                    replacement_target,
                    recommendation,
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            b.weighted_gap_score
                .total_cmp(&a.weighted_gap_score)
                .then_with(|| b.gap_score.total_cmp(&a.gap_score))
                .then_with(|| a.category.cmp(&b.category))
        });
        rows.truncate(input.limit.max(1));

        Self {
            context: context.clone(),
            league: input.league.to_string(),
            team: input.team.to_string(),
            scoring_scheme: input.scoring_scheme.to_string(),
            categories,
            rows,
            source_state: context.source_state,
            warnings,
        }
    }
}

const ADD_NOW_WEIGHTED_DELTA: f64 = 5.0;

fn classify_action(
    candidate: Option<&FantasyRosterGapCandidate>,
    replacement: Option<&FantasyRosterGapReplacement>,
    weighted_gap_score: f64,
) -> (FantasyRosterGapAction, String) {
    if candidate.is_none() {
        return (
            FantasyRosterGapAction::NoAction,
            "no available candidate resolved".to_string(),
        );
    }
    if let Some(replacement) = replacement {
        if replacement.weighted_delta >= ADD_NOW_WEIGHTED_DELTA {
            return (
                FantasyRosterGapAction::AddNow,
                format!(
                    "same-position replacement improves weighted contribution by {:.1}",
                    replacement.weighted_delta
                ),
            );
        }
        if replacement.weighted_delta > 0.0 {
            return (
                FantasyRosterGapAction::Watch,
                format!(
                    "positive but small same-position weighted delta ({:.1})",
                    replacement.weighted_delta
                ),
            );
        }
        return (
            FantasyRosterGapAction::NoAction,
            format!(
                "same-position replacement delta is not positive ({:.1})",
                replacement.weighted_delta
            ),
        );
    }
    if weighted_gap_score > 0.0 {
        return (
            FantasyRosterGapAction::Watch,
            "no same-position replacement resolved; evaluate roster fit manually".to_string(),
        );
    }
    (
        FantasyRosterGapAction::NoAction,
        "weighted contribution is not positive".to_string(),
    )
}

fn contains_player(keys: &[String], player: &PlayerView<'_>) -> bool {
    keys.iter()
        .any(|key| player.identity.name_normalized.contains(key.as_str()))
}

fn best_candidate(
    players: &[PlayerView<'_>],
    category: &str,
    weight: f64,
) -> Option<FantasyRosterGapCandidate> {
    players
        .iter()
        .map(|player| (*player, category_value(player, category)))
        .filter(|(_, value)| value.abs() > f64::EPSILON || weight < 0.0)
        .max_by(|(a_player, a_value), (b_player, b_value)| {
            (a_value * weight)
                .total_cmp(&(b_value * weight))
                .then_with(|| b_player.id().0.cmp(&a_player.id().0))
        })
        .map(|(player, value)| candidate_from_player(player, weight, value))
}

fn replacement_target(
    players: &[PlayerView<'_>],
    category: &str,
    position: &str,
    weight: f64,
) -> Option<FantasyRosterGapReplacement> {
    players
        .iter()
        .filter(|player| player.position().abbreviation() == position)
        .map(|player| (*player, category_value(player, category)))
        .min_by(|(a_player, a_value), (b_player, b_value)| {
            (a_value * weight)
                .total_cmp(&(b_value * weight))
                .then_with(|| a_player.id().0.cmp(&b_player.id().0))
        })
        .map(|(player, value)| {
            let candidate = candidate_from_player(player, weight, value);
            FantasyRosterGapReplacement {
                player_id: candidate.player_id,
                display_name: candidate.display_name,
                team: candidate.team,
                position: candidate.position,
                value: candidate.value,
                weighted_value: candidate.weighted_value,
                delta: 0.0,
                weighted_delta: 0.0,
            }
        })
}

fn candidate_from_player(
    player: PlayerView<'_>,
    weight: f64,
    value: f64,
) -> FantasyRosterGapCandidate {
    FantasyRosterGapCandidate {
        player_id: player.id().0,
        display_name: player.full_name().to_string(),
        team: player.team_display().to_string(),
        position: player.position().abbreviation().to_string(),
        value,
        weighted_value: value * weight,
    }
}

fn effective_categories(scoring_scheme: &str, categories: Vec<String>) -> Vec<String> {
    let explicit = categories
        .into_iter()
        .map(|category| normalize_category(&category))
        .filter(|category| !category.is_empty())
        .collect::<Vec<_>>();
    if !explicit.is_empty() {
        return explicit;
    }
    Scheme::builtin_named(scoring_scheme)
        .map(|scheme| {
            scheme
                .skater_category_keys()
                .into_iter()
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                "hits".to_string(),
                "blocks".to_string(),
                "shots".to_string(),
            ]
        })
}

fn normalize_category(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "shot" | "sog" | "shots_on_goal" | "shots-on-goal" => "shots".to_string(),
        "blocked_shots" | "blocked-shots" | "blk" => "blocks".to_string(),
        "hit" => "hits".to_string(),
        other => other.to_string(),
    }
}

fn category_weight(scheme: Option<&Scheme>, category: &str) -> f64 {
    let Some(scheme) = scheme else {
        return 1.0;
    };
    let weights = &scheme.skater;
    (match category {
        "goals" => weights.goals,
        "assists" => weights.assists,
        "pp_goals" => weights.pp_goals,
        "pp_assists" => weights.pp_assists,
        "sh_goals" => weights.sh_goals,
        "sh_assists" => weights.sh_assists,
        "gwg" => weights.gwg,
        "ot_goals" => weights.ot_goals,
        "hits" => weights.hits,
        "blocks" => weights.blocks,
        "shots" | "shots_on_goal" => weights.shots_on_goal,
        "plus_minus" => weights.plus_minus,
        "takeaways" => weights.takeaways,
        "giveaways" => weights.giveaways,
        "faceoff_wins" => weights.faceoff_wins,
        "points" => weights.goals.max(weights.assists),
        _ => 1.0,
    }) as f64
}

fn category_value(player: &PlayerView<'_>, category: &str) -> f64 {
    match category {
        "goals" => player.stats.totals.goals as f64,
        "assists" => player.stats.totals.assists as f64,
        "points" => player.stats.totals.points as f64,
        "pp_goals" => player.stats.totals.pp_goals as f64,
        "pp_assists" => player
            .stats
            .totals
            .pp_points
            .saturating_sub(player.stats.totals.pp_goals) as f64,
        "sh_goals" => player.stats.totals.sh_goals as f64,
        "sh_assists" => player
            .stats
            .totals
            .sh_points
            .saturating_sub(player.stats.totals.sh_goals) as f64,
        "gwg" => player.stats.totals.gwg as f64,
        "ot_goals" => player.stats.totals.ot_goals as f64,
        "hits" => player.hits().unwrap_or(0) as f64,
        "blocks" => player.blocked_shots().unwrap_or(0) as f64,
        "shots" => player.shots() as f64,
        "plus_minus" => player.plus_minus() as f64,
        "takeaways" => player.takeaways().unwrap_or(0) as f64,
        "giveaways" => player.giveaways().unwrap_or(0) as f64,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn fantasy_roster_gap_ranks_best_available_category_help() {
        let (identity_a, stats_a) = fixtures::stat_catalog_variants::skater_modern();
        let identity_b = fixtures::identity(8479999)
            .name("Block Helper", "block helper")
            .build();
        let stats_b = fixtures::stats(8479999, 20242025, "SEA")
            .realtime(20, 80, 2, 1)
            .build();
        let mut repo = fixtures::test_repo_with(identity_a, stats_a);
        repo.upsert_identity(identity_b).unwrap();
        repo.upsert_stats(stats_b).unwrap();

        let view = FantasyRosterGapView::from_repository(
            &repo,
            FantasyRosterGapInput {
                season: Season(20242025),
                season_type: SeasonType::Regular,
                league: "Main",
                team: "Mine",
                scoring_scheme: "yahoo-standard",
                categories: vec!["blocks".to_string()],
                user_roster_keys: vec!["connor mcdavid".to_string()],
                all_rostered_keys: vec!["connor mcdavid".to_string()],
                limit: 5,
            },
        );

        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].category, "blocks");
        assert_eq!(view.rows[0].action, FantasyRosterGapAction::AddNow);
        assert_eq!(view.rows[0].weight, 0.5);
        assert_eq!(
            view.rows[0]
                .best_available
                .as_ref()
                .map(|candidate| candidate.display_name.as_str()),
            Some("Block Helper")
        );
        assert_eq!(
            view.rows[0]
                .best_available
                .as_ref()
                .map(|candidate| candidate.weighted_value),
            Some(40.0)
        );
        assert_eq!(
            view.rows[0]
                .replacement_target
                .as_ref()
                .map(|target| target.display_name.as_str()),
            Some("Connor McDavid")
        );
        assert_eq!(
            view.rows[0]
                .replacement_target
                .as_ref()
                .map(|target| target.delta),
            Some(62.0)
        );
        assert_eq!(
            view.rows[0]
                .replacement_target
                .as_ref()
                .map(|target| target.weighted_delta),
            Some(31.0)
        );
        assert!(view.rows[0].recommendation.contains("Block Helper"));
    }

    #[test]
    fn fantasy_roster_gap_marks_small_positive_delta_as_watch() {
        let (identity_a, stats_a) = fixtures::stat_catalog_variants::skater_modern();
        let identity_b = fixtures::identity(8479998)
            .name("Small Helper", "small helper")
            .build();
        let stats_b = fixtures::stats(8479998, 20242025, "SEA")
            .realtime(20, 19, 2, 1)
            .build();
        let mut repo = fixtures::test_repo_with(identity_a, stats_a);
        repo.upsert_identity(identity_b).unwrap();
        repo.upsert_stats(stats_b).unwrap();

        let view = FantasyRosterGapView::from_repository(
            &repo,
            FantasyRosterGapInput {
                season: Season(20242025),
                season_type: SeasonType::Regular,
                league: "Main",
                team: "Mine",
                scoring_scheme: "yahoo-standard",
                categories: vec!["blocks".to_string()],
                user_roster_keys: vec!["connor mcdavid".to_string()],
                all_rostered_keys: vec!["connor mcdavid".to_string()],
                limit: 5,
            },
        );

        assert_eq!(view.rows[0].action, FantasyRosterGapAction::Watch);
        assert_eq!(
            view.rows[0]
                .replacement_target
                .as_ref()
                .map(|target| target.weighted_delta),
            Some(0.5)
        );
        assert!(view.rows[0].action_reason.contains("small"));
    }

    #[test]
    fn fantasy_roster_gap_marks_non_positive_delta_as_no_action() {
        let (identity_a, stats_a) = fixtures::stat_catalog_variants::skater_modern();
        let identity_b = fixtures::identity(8479997)
            .name("No Upgrade", "no upgrade")
            .build();
        let stats_b = fixtures::stats(8479997, 20242025, "SEA")
            .realtime(20, 18, 2, 1)
            .build();
        let mut repo = fixtures::test_repo_with(identity_a, stats_a);
        repo.upsert_identity(identity_b).unwrap();
        repo.upsert_stats(stats_b).unwrap();

        let view = FantasyRosterGapView::from_repository(
            &repo,
            FantasyRosterGapInput {
                season: Season(20242025),
                season_type: SeasonType::Regular,
                league: "Main",
                team: "Mine",
                scoring_scheme: "yahoo-standard",
                categories: vec!["blocks".to_string()],
                user_roster_keys: vec!["connor mcdavid".to_string()],
                all_rostered_keys: vec!["connor mcdavid".to_string()],
                limit: 5,
            },
        );

        assert_eq!(view.rows[0].action, FantasyRosterGapAction::NoAction);
        assert_eq!(
            view.rows[0]
                .replacement_target
                .as_ref()
                .map(|target| target.weighted_delta),
            Some(0.0)
        );
        assert!(view.rows[0].action_reason.contains("not positive"));
    }
}
