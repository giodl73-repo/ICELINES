use crate::state::WebState;
use crate::templates::{ComparePlayerCard, CompareTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{CompareView, MetricCell, MetricValue, PlayerCardView, SimilarPlayersView};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct CompareQuery {
    /// Either a NHL id ("8478402") or a player name
    /// ("Connor McDavid"). UX.H — the player card's compare
    /// form posts a name selected from the autocomplete
    /// datalist; deep-linked URLs may still pass an id.
    #[serde(default)]
    pub a: Option<String>,
    #[serde(default)]
    pub b: Option<String>,
    #[serde(default)]
    pub similar: Option<usize>,
}

#[derive(Debug)]
struct CompareResult {
    active_label: String,
    season: String,
    season_type: SeasonType,
    a: Option<ComparePlayerCard>,
    b: Option<ComparePlayerCard>,
    similar: Option<SimilarPlayersView>,
    error: Option<String>,
    winners: crate::templates::CompareWinners,
}

#[derive(Debug, serde::Serialize)]
struct CompareData {
    a: Option<ComparePlayerCard>,
    b: Option<ComparePlayerCard>,
    similar: Option<SimilarPlayersView>,
    winners: crate::templates::CompareWinners,
}

#[derive(Debug, serde::Serialize)]
struct CompareMeta {
    season: String,
    season_type: String,
}

/// Resolve a `?a=` / `?b=` query value (id or name) into a
/// numeric NHL id. Pure u32 short-circuits; otherwise the
/// first repo identity whose `full_name` matches
/// case-insensitively wins. Returns None when there's no
/// match, so callers can render a friendly error instead of
/// silently picking the wrong player.
async fn resolve_id(state: &WebState, raw: &str) -> Option<u32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(id) = trimmed.parse::<u32>() {
        return Some(id);
    }
    let needle = trimmed.to_ascii_lowercase();
    let repo = state.repo.read().await;
    for identity in repo.iter_identities() {
        if identity.full_name.to_ascii_lowercase() == needle {
            return Some(identity.id.0);
        }
    }
    None
}

fn compare_card_from_view(view: &PlayerCardView, season: Season) -> ComparePlayerCard {
    let active = view.active.as_ref();
    let active_metrics = active
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);
    let (gp, goals, assists, points, position, team, team_link) = match active {
        Some(active) => {
            let team_link = team_link_for_display(&active.team_display);
            (
                metric_u32(active_metrics, "gp").unwrap_or(0),
                metric_u32(active_metrics, "goals").unwrap_or(0),
                metric_u32(active_metrics, "assists").unwrap_or(0),
                metric_u32(active_metrics, "points").unwrap_or(0),
                active.position.abbreviation().to_owned(),
                active.team_display.clone(),
                team_link,
            )
        }
        None => (0, 0, 0, 0, dash(), dash(), String::new()),
    };

    ComparePlayerCard {
        nhl_id: view.player_id.0,
        full_name: view.display_name.clone(),
        position,
        team,
        team_link: team_link.clone(),
        headshot_url: if !team_link.is_empty() {
            Some(super::shared::build_headshot_url(
                season.0,
                &team_link,
                view.player_id.0,
            ))
        } else {
            view.headshot_url.clone()
        },
        gp,
        goals,
        assists,
        points,
        ppg_str: metric_f64(active_metrics, "points_per_game")
            .map(|ppg| format!("{ppg:.2}"))
            .unwrap_or_default(),
        plus_minus_str: metric_i32(active_metrics, "plus_minus")
            .map(|value| format!("{value:+}"))
            .unwrap_or_else(dash),
        pim_str: metric_string_u32_or_dash(active_metrics, "pim"),
        shots_str: metric_string_u32_or_dash(active_metrics, "shots"),
        shooting_pct_str: metric_percent_string(active_metrics, "shooting_pct"),
        hits_str: metric_string_u32_or_dash(active_metrics, "hits"),
        blocks_str: metric_string_u32_or_dash(active_metrics, "blocks"),
        takeaways_str: metric_string_u32_or_dash(active_metrics, "takeaways"),
        giveaways_str: metric_string_u32_or_dash(active_metrics, "giveaways"),
        faceoff_pct_str: metric_percent_string(active_metrics, "faceoff_win_pct"),
        pp_goals_str: metric_string_u32_or_dash(active_metrics, "pp_goals"),
        pp_points_str: metric_string_u32_or_dash(active_metrics, "pp_points"),
        sh_goals_str: metric_string_u32_or_dash(active_metrics, "sh_goals"),
        gwg_str: metric_string_u32_or_dash(active_metrics, "gwg"),
        toi_per_game_str: metric_toi_mmss(active_metrics, "toi_per_game_sec"),
    }
}

fn team_link_for_display(team: &str) -> String {
    if team.chars().all(|c| c.is_ascii_alphabetic()) && (2..=3).contains(&team.len()) {
        team.to_owned()
    } else {
        String::new()
    }
}

fn metric_u32(metrics: &[MetricCell], key: &str) -> Option<u32> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => u32::try_from(value).ok(),
            _ => None,
        })
}

fn metric_i32(metrics: &[MetricCell], key: &str) -> Option<i32> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => i32::try_from(value).ok(),
            _ => None,
        })
}

fn metric_f64(metrics: &[MetricCell], key: &str) -> Option<f64> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Decimal(value) => Some(value),
            _ => None,
        })
}

fn metric_string_u32_or_dash(metrics: &[MetricCell], key: &str) -> String {
    metric_u32(metrics, key)
        .map(|value| value.to_string())
        .unwrap_or_else(dash)
}

fn metric_percent_string(metrics: &[MetricCell], key: &str) -> String {
    metric_f64(metrics, key)
        .map(|value| {
            if value.abs() <= 1.5 {
                format!("{:.1}%", value * 100.0)
            } else {
                format!("{value:.1}%")
            }
        })
        .unwrap_or_else(dash)
}

fn metric_toi_mmss(metrics: &[MetricCell], key: &str) -> String {
    metric_u32(metrics, key)
        .map(|secs| format!("{}:{:02}", secs / 60, secs % 60))
        .unwrap_or_else(dash)
}

fn dash() -> String {
    "\u{2014}".to_owned()
}

async fn build_compare_result(state: &WebState, q: &CompareQuery) -> CompareResult {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return CompareResult {
                active_label,
                season: season_str.clone(),
                season_type,
                a: None,
                b: None,
                similar: None,
                error: Some(format!(
                    "Active season '{season_str}' is not a valid YYYYZZZZ id"
                )),
                winners: crate::templates::CompareWinners::default(),
            };
        }
    };
    let season = Season(season_u32);

    // Resolve "a" and "b" to numeric NHL ids. Each may be
    // either a u32 (deep-linked URL like
    // /compare?a=8478402&b=8477934) or a name typed into
    // the player-card autocomplete ("Connor McDavid"). A
    // raw value that doesn't parse as u32 AND isn't a known
    // repo identity name surfaces as `unresolved` so the
    // template can name what failed.
    let a_raw = q.a.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let b_raw = q.b.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let a_id = match a_raw {
        Some(raw) => resolve_id(state, raw).await,
        None => None,
    };
    let b_id = match b_raw {
        Some(raw) => resolve_id(state, raw).await,
        None => None,
    };

    {
        let mut repo = state.repo.write().await;
        for id in [a_id, b_id].into_iter().flatten() {
            let _ =
                icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, PlayerId(id));
        }
    }

    let compare_view = {
        let repo = state.repo.read().await;
        CompareView::from_repository(
            &repo,
            a_id.map(PlayerId),
            b_id.map(PlayerId),
            season,
            season_type,
        )
    };
    let a_card = compare_view
        .a
        .as_ref()
        .map(|card| compare_card_from_view(card, season));
    let b_card = compare_view
        .b
        .as_ref()
        .map(|card| compare_card_from_view(card, season));
    let similar = if let (Some(limit), Some(target_id)) = (q.similar, a_id) {
        let limit = limit.clamp(1, 50);
        let repo = state.repo.read().await;
        let views: Vec<_> = repo.skaters(season, season_type).collect();
        views
            .iter()
            .find(|view| view.identity.id == PlayerId(target_id))
            .map(|target| {
                SimilarPlayersView::from_player_views(
                    &views,
                    target,
                    limit,
                    season,
                    season_type,
                    repo.has_window(season, season_type),
                )
            })
    } else {
        None
    };
    let a_missing = a_id.filter(|_| a_card.is_none());
    let b_missing = b_id.filter(|_| b_card.is_none());

    // Distinguish "no input given" vs "input given but didn't
    // resolve to a known player". The latter is more useful to
    // surface with the typed text.
    let a_unresolved = a_raw.filter(|_| a_id.is_none());
    let b_unresolved = b_raw.filter(|_| b_id.is_none());

    let valid_similarity_request = q.similar.is_some() && a_raw.is_some() && a_unresolved.is_none();
    let error = if valid_similarity_request || (a_raw.is_none() && b_raw.is_none()) {
        None
    } else if a_raw.is_none() {
        Some("Missing first player (?a=).".to_owned())
    } else if b_raw.is_none() {
        Some("Missing second player (?b=).".to_owned())
    } else if let (Some(a_text), Some(b_text)) = (a_unresolved, b_unresolved) {
        Some(format!(
            "Neither '{a_text}' nor '{b_text}' matches a player in the active repository."
        ))
    } else if let Some(text) = a_unresolved {
        Some(format!("No player matches '{text}'."))
    } else if let Some(text) = b_unresolved {
        Some(format!("No player matches '{text}'."))
    } else if let (Some(id_a), Some(id_b)) = (a_missing, b_missing) {
        Some(format!(
            "Neither player {id_a} nor {id_b} is in the active repository."
        ))
    } else {
        a_missing
            .or(b_missing)
            .map(|id| format!("No player with NHL id {id}."))
    };

    // Sasq.8 — compute per-stat winner flags so the template
    // can bold whichever side has the better value. Most
    // stats are higher-is-better; PIM and giveaways are
    // flipped (lower-is-better in modern hockey
    // contexts — fewer minor penalties / fewer turnovers
    // are signals of cleaner play).
    let winners = match (&a_card, &b_card) {
        (Some(pa), Some(pb)) => build_compare_winners(pa, pb),
        _ => crate::templates::CompareWinners::default(),
    };

    CompareResult {
        active_label,
        season: season_str,
        season_type,
        a: a_card,
        b: b_card,
        similar,
        error,
        winners,
    }
}

pub async fn get_compare(State(state): State<WebState>, Query(q): Query<CompareQuery>) -> Response {
    let result = build_compare_result(&state, &q).await;
    let tmpl = CompareTemplate {
        active_label: result.active_label,
        a: result.a,
        b: result.b,
        error: result.error,
        winners: result.winners,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

pub async fn get_compare_json(
    State(state): State<WebState>,
    Query(q): Query<CompareQuery>,
) -> Response {
    let result = build_compare_result(&state, &q).await;
    let data = CompareData {
        a: result.a,
        b: result.b,
        similar: result.similar,
        winners: result.winners,
    };
    let meta = CompareMeta {
        season: result.season,
        season_type: result.season_type.label().to_owned(),
    };
    match result.error {
        Some(error) => {
            crate::api::json_error_meta(StatusCode::BAD_REQUEST, "compare", data, meta, error)
        }
        None => crate::api::json_data_meta("compare", data, meta),
    }
}

/// Compute per-stat winner flags for two ComparePlayerCards.
/// Higher-is-better unless explicitly flipped (PIM, GV).
/// Numeric stats (gp, goals, etc.) compare directly; string
/// stats parse out a leading number where possible. Strings
/// containing "—" or that fail to parse skip the comparison
/// (both flags stay false → neither side bolded).
fn build_compare_winners(
    pa: &ComparePlayerCard,
    pb: &ComparePlayerCard,
) -> crate::templates::CompareWinners {
    use crate::templates::CompareWinners;
    // Parse a stat string like "+12", "1.78", "10.5%", "20:20",
    // "—" into Option<f64>. The "20:20" TOI/G case is M:SS
    // which we convert to total seconds for comparison.
    fn parse_stat(s: &str) -> Option<f64> {
        let t = s.trim();
        if t.is_empty() || t == "—" {
            return None;
        }
        if let Some((m, s)) = t.split_once(':') {
            if let (Ok(mi), Ok(se)) = (m.parse::<u32>(), s.parse::<u32>()) {
                return Some(f64::from(mi) * 60.0 + f64::from(se));
            }
        }
        let stripped = t.trim_end_matches('%');
        stripped.parse::<f64>().ok()
    }
    // Compare two values with `higher_better` bias and write
    // the (a_wins, b_wins) booleans. Equality → both false.
    fn cmp_pair(a: f64, b: f64, higher_better: bool) -> (bool, bool) {
        use std::cmp::Ordering;
        let ord = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
        if higher_better {
            (ord == Ordering::Greater, ord == Ordering::Less)
        } else {
            (ord == Ordering::Less, ord == Ordering::Greater)
        }
    }
    fn cmp_strs(sa: &str, sb: &str, higher_better: bool) -> (bool, bool) {
        match (parse_stat(sa), parse_stat(sb)) {
            (Some(a), Some(b)) => cmp_pair(a, b, higher_better),
            _ => (false, false),
        }
    }
    fn cmp_u32(a: u32, b: u32) -> (bool, bool) {
        cmp_pair(f64::from(a), f64::from(b), true)
    }
    let mut w = CompareWinners::default();
    (w.gp_a, w.gp_b) = cmp_u32(pa.gp, pb.gp);
    (w.goals_a, w.goals_b) = cmp_u32(pa.goals, pb.goals);
    (w.assists_a, w.assists_b) = cmp_u32(pa.assists, pb.assists);
    (w.points_a, w.points_b) = cmp_u32(pa.points, pb.points);
    (w.ppg_a, w.ppg_b) = cmp_strs(&pa.ppg_str, &pb.ppg_str, true);
    (w.plus_minus_a, w.plus_minus_b) = cmp_strs(&pa.plus_minus_str, &pb.plus_minus_str, true);
    (w.pim_a, w.pim_b) = cmp_strs(&pa.pim_str, &pb.pim_str, false); // lower better
    (w.shots_a, w.shots_b) = cmp_strs(&pa.shots_str, &pb.shots_str, true);
    (w.shooting_pct_a, w.shooting_pct_b) =
        cmp_strs(&pa.shooting_pct_str, &pb.shooting_pct_str, true);
    (w.hits_a, w.hits_b) = cmp_strs(&pa.hits_str, &pb.hits_str, true);
    (w.blocks_a, w.blocks_b) = cmp_strs(&pa.blocks_str, &pb.blocks_str, true);
    (w.takeaways_a, w.takeaways_b) = cmp_strs(&pa.takeaways_str, &pb.takeaways_str, true);
    (w.giveaways_a, w.giveaways_b) = cmp_strs(&pa.giveaways_str, &pb.giveaways_str, false); // lower better
    (w.faceoff_pct_a, w.faceoff_pct_b) = cmp_strs(&pa.faceoff_pct_str, &pb.faceoff_pct_str, true);
    (w.pp_goals_a, w.pp_goals_b) = cmp_strs(&pa.pp_goals_str, &pb.pp_goals_str, true);
    (w.pp_points_a, w.pp_points_b) = cmp_strs(&pa.pp_points_str, &pb.pp_points_str, true);
    (w.sh_goals_a, w.sh_goals_b) = cmp_strs(&pa.sh_goals_str, &pb.sh_goals_str, true);
    (w.gwg_a, w.gwg_b) = cmp_strs(&pa.gwg_str, &pb.gwg_str, true);
    (w.toi_per_game_a, w.toi_per_game_b) =
        cmp_strs(&pa.toi_per_game_str, &pb.toi_per_game_str, true);
    w
}
