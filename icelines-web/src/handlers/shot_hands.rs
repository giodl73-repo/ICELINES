use crate::config::WebConfig;
use crate::state::WebState;
use crate::templates::{ShotHandRosterPlayerRow, ShotHandTeamRow, ShotHandsTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_fetch::schema::{RosterPlayer, RosterResponse};
use icelines_fetch::snapshot::{
    OfficialNhlRosterCaptureManifest, SnapshotStore, SnapshotTier,
    OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE,
};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

use super::leaders::{build_leaders_template, LeadersQuery};

#[derive(Debug, Default, Deserialize)]
pub struct ShotHandsQuery {
    pub sort: Option<String>,
    pub pos: Option<String>,
    pub top: Option<usize>,
    pub team: Option<String>,
}

#[derive(Default)]
struct TeamBucket {
    right: Vec<ShotHandRosterPlayerRow>,
    left: Vec<ShotHandRosterPlayerRow>,
    unknown: usize,
}

fn position_matches(position: Position, filter: &str) -> bool {
    match filter {
        "F" => matches!(
            position,
            Position::Center | Position::LeftWing | Position::RightWing
        ),
        "D" => position == Position::Defense,
        _ => true,
    }
}

fn sort_players(rows: &mut [ShotHandRosterPlayerRow], sort: &str) {
    rows.sort_by(|left, right| {
        let primary = match sort {
            "goals" => right.goals.cmp(&left.goals),
            "points" => right.points.cmp(&left.points),
            _ => right.shots.cmp(&left.shots),
        };
        primary
            .then(right.shots.cmp(&left.shots))
            .then(left.name.cmp(&right.name))
    });
}

fn roster_position_matches(position: &str, filter: &str) -> bool {
    match filter {
        "F" => position != "D",
        "D" => position == "D",
        _ => true,
    }
}

fn roster_row(
    player: &RosterPlayer,
    stats: &HashMap<u32, (u32, u32, u32)>,
) -> ShotHandRosterPlayerRow {
    let production = stats.get(&player.id);
    ShotHandRosterPlayerRow {
        name: format!(
            "{} {}",
            player.first_name.as_str(),
            player.last_name.as_str()
        ),
        position: player.position_code.clone(),
        shots: production.map_or(0, |v| v.0),
        goals: production.map_or(0, |v| v.1),
        points: production.map_or(0, |v| v.2),
        has_stats: production.is_some(),
    }
}

pub async fn get_shot_hands(
    State(state): State<WebState>,
    Query(query): Query<ShotHandsQuery>,
) -> Response {
    match build_shot_hands_template(&state, query).await {
        Ok(template) => match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("template render failed: {error}")),
            )
                .into_response(),
        },
        Err(response) => response,
    }
}

pub async fn build_shot_hands_template(
    state: &WebState,
    query: ShotHandsQuery,
) -> Result<ShotHandsTemplate, Response> {
    // Restrict this view to counting metrics. SH% is shown as context but
    // deliberately not ranked without a minimum shot threshold.
    let sort = match query.sort.as_deref() {
        Some("goals") => "goals",
        Some("points") => "points",
        _ => "shots",
    };
    let pos = match query.pos.as_deref() {
        Some("F") => "F",
        Some("D") => "D",
        _ => "",
    };
    let top = query.top.unwrap_or(15).clamp(1, 50);
    let (active_season, active_label) = {
        let config = state.config.read().await;
        (config.active_season.clone(), config.active_label.clone())
    };
    let season_id = match active_season.parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Active season is not a valid season ID".to_owned()),
            )
                .into_response());
        }
    };
    let active_has_games = {
        let repo = state.repo.read().await;
        let has_games = repo
            .skaters(Season(season_id), SeasonType::Regular)
            .any(|player| player.gp() > 0);
        has_games
    };

    // In preseason, use the last completed season so the default view
    // answers "who are the top right shots?" with observed production.
    let (comparison_state, comparison_season, season_note) = if active_has_games {
        (state.clone(), season_id, String::new())
    } else {
        let start_year = season_id / 10_000;
        let prior_id = (start_year - 1) * 10_000 + start_year;
        let loaded = state.snapshots_root.as_ref().as_ref().and_then(|root| {
            let store = icelines_fetch::snapshot::SnapshotStore::new(root.clone());
            icelines_fetch::stats_loader::load_into_repo(
                Season(prior_id),
                SeasonType::Regular,
                &store,
            )
            .ok()
        });
        match loaded {
            Some(outcome)
                if outcome
                    .repo
                    .skaters(Season(prior_id), SeasonType::Regular)
                    .any(|player| player.gp() > 0) =>
            {
                (
                    WebState::with_repo_and_config(
                        outcome.repo,
                        WebConfig::new(prior_id.to_string(), "regular"),
                    ),
                    prior_id,
                    format!(
                        "The active {active_label} season has no played-game skater stats. These are observed {}-{:02} regular-season totals.",
                        prior_id / 10_000,
                        prior_id % 100
                    ),
                )
            }
            _ => (
                state.clone(),
                season_id,
                format!(
                    "No played-game stats are available for {active_label}, and the prior-season snapshot could not be loaded."
                ),
            ),
        }
    };

    let make_query = |hand: &str| LeadersQuery {
        sort: Some(sort.to_owned()),
        pos: Some(pos.to_owned()),
        top: Some(top),
        shoots: Some(hand.to_owned()),
        ..LeadersQuery::default()
    };
    let right = build_leaders_template(&comparison_state, make_query("R"), "").await?;
    let left = build_leaders_template(&comparison_state, make_query("L"), "").await?;
    let missing_count = {
        let repo = comparison_state.repo.read().await;
        let count = repo
            .skaters(Season(comparison_season), SeasonType::Regular)
            .filter(|player| position_matches(player.position(), pos))
            .count();
        count.saturating_sub(right.total + left.total)
    };
    let mut prior_stats = HashMap::new();
    {
        let repo = comparison_state.repo.read().await;
        for player in repo.skaters(Season(comparison_season), SeasonType::Regular) {
            prior_stats.insert(
                player.id().0,
                (player.stats.totals.shots, player.goals(), player.points()),
            );
        }
    }
    let mut buckets: BTreeMap<String, TeamBucket> = BTreeMap::new();
    let roster_note = match state.snapshots_root.as_ref().as_ref() {
        Some(root) => {
            let store = SnapshotStore::new(root.clone());
            let loaded = (|| -> Result<String, String> {
                let snapshot = store.list().map_err(|e| e.to_string())?.into_iter()
                    .filter(|entry| entry.season == active_season && entry.tier == SnapshotTier::Rosters && entry.sealed)
                    .max_by(|a, b| a.name.cmp(&b.name))
                    .ok_or_else(|| format!("No sealed {active_label} roster snapshot is available."))?;
                let capture: OfficialNhlRosterCaptureManifest = store.read(&snapshot.name, &SnapshotTier::Rosters, OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE)
                    .map_err(|e| e.to_string())?;
                for (team, _) in icelines_core::CANONICAL_TEAMS {
                    let roster: RosterResponse = store.read(&snapshot.name, &SnapshotTier::Rosters, &format!("{team}.json"))
                        .map_err(|e| format!("{team} roster unavailable: {e}"))?;
                    let bucket = buckets.entry((*team).to_owned()).or_default();
                    for player in roster.forwards.iter().chain(&roster.defensemen) {
                        if !roster_position_matches(&player.position_code, pos) { continue; }
                        match player.shoots_catches.as_deref() {
                            Some("R") => bucket.right.push(roster_row(player, &prior_stats)),
                            Some("L") => bucket.left.push(roster_row(player, &prior_stats)),
                            _ => bucket.unknown += 1,
                        }
                    }
                }
                Ok(format!("Official NHL {active_label} rosters captured {}. Shooting-hand counts use this snapshot; SOG, goals, and points use {}-{:02} regular-season totals where available.", capture.observed_at, comparison_season / 10_000, comparison_season % 100))
            })();
            match loaded {
                Ok(note) => note,
                Err(error) => {
                    buckets.clear();
                    format!("Current rosters unavailable: {error} Run `icelines fetch rosters --season {active_season} --refresh` to update them.")
                }
            }
        }
        None => format!("Current rosters unavailable. Run `icelines fetch rosters --season {active_season} --refresh` to update them."),
    };
    let requested_team = query
        .team
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    let selected_team = if buckets.contains_key(&requested_team) {
        requested_team
    } else {
        String::new()
    };
    let teams = buckets
        .iter()
        .map(|(team, bucket)| {
            let known = bucket.right.len() + bucket.left.len();
            ShotHandTeamRow {
                team: team.clone(),
                right_count: bucket.right.len(),
                left_count: bucket.left.len(),
                unknown_count: bucket.unknown,
                right_pct: if known == 0 {
                    0
                } else {
                    (100 * bucket.right.len() / known) as u32
                },
                is_selected: selected_team == *team,
            }
        })
        .collect();
    let (team_right_rows, team_left_rows) = buckets
        .remove(&selected_team)
        .map(|mut bucket| {
            sort_players(&mut bucket.right, sort);
            sort_players(&mut bucket.left, sort);
            bucket.right.truncate(top);
            bucket.left.truncate(top);
            (bucket.right, bucket.left)
        })
        .unwrap_or_default();
    Ok(ShotHandsTemplate {
        active_label,
        comparison_label: format!(
            "{}-{:02} regular season",
            comparison_season / 10_000,
            comparison_season % 100
        ),
        season_note,
        sort: sort.to_owned(),
        pos: pos.to_owned(),
        top,
        right_count: right.total,
        left_count: left.total,
        missing_count,
        links_available: comparison_season == season_id,
        right_rows: right.rows,
        left_rows: left.rows,
        teams,
        selected_team,
        team_right_rows,
        team_left_rows,
        roster_note,
    })
}
