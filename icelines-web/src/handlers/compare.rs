use crate::state::WebState;
use crate::templates::{ComparePlayerCard, CompareTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
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
}

#[derive(Debug)]
struct CompareResult {
    active_label: String,
    season: String,
    season_type: SeasonType,
    a: Option<ComparePlayerCard>,
    b: Option<ComparePlayerCard>,
    error: Option<String>,
    winners: crate::templates::CompareWinners,
}

#[derive(Debug, serde::Serialize)]
struct CompareEnvelope {
    schema_version: u32,
    route: &'static str,
    data: CompareData,
    meta: CompareMeta,
    error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct CompareData {
    a: Option<ComparePlayerCard>,
    b: Option<ComparePlayerCard>,
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

fn opt_u(o: Option<u32>) -> String {
    match o {
        Some(n) => n.to_string(),
        None => "—".to_owned(),
    }
}
fn opt_pct(o: Option<f32>) -> String {
    match o {
        Some(p) => {
            if p.abs() <= 1.5 {
                format!("{:.1}%", p * 100.0)
            } else {
                format!("{:.1}%", p)
            }
        }
        None => "—".to_owned(),
    }
}
fn toi_mmss(o: Option<u32>) -> String {
    match o {
        Some(secs) => {
            let m = secs / 60;
            let s = secs % 60;
            format!("{m}:{s:02}")
        }
        None => "—".to_owned(),
    }
}

async fn build_card(
    state: &WebState,
    id: u32,
    season: Season,
    season_type: SeasonType,
) -> Option<ComparePlayerCard> {
    // Lazy career fan-out so a freshly-opened player has full
    // career loaded — same pattern as the player handler.
    let pid = PlayerId(id);
    {
        let mut repo = state.repo.write().await;
        let _ = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid);
    }
    let repo = state.repo.read().await;
    let identity = repo.identity(pid)?;
    let view = repo.view(pid, season, season_type);
    let (
        gp,
        goals,
        assists,
        points,
        position,
        team,
        team_link,
        plus_minus_str,
        pim_str,
        shots_str,
        shooting_pct_str,
        hits_str,
        blocks_str,
        takeaways_str,
        giveaways_str,
        faceoff_pct_str,
        pp_goals_str,
        pp_points_str,
        sh_goals_str,
        gwg_str,
        toi_per_game_str,
    ) = match view {
        Some(v) => {
            let totals = &v.stats.totals;
            let team_display = v.team_display().to_owned();
            let team_link = if team_display.chars().all(|c| c.is_ascii_alphabetic())
                && team_display.len() <= 3
            {
                team_display.clone()
            } else {
                String::new()
            };
            (
                v.gp(),
                v.goals(),
                v.assists(),
                v.points(),
                v.position().abbreviation().to_owned(),
                team_display,
                team_link,
                format!("{:+}", v.plus_minus()),
                totals.pim.to_string(),
                totals.shots.to_string(),
                opt_pct(totals.shooting_pct),
                opt_u(v.hits()),
                opt_u(v.blocked_shots()),
                opt_u(v.takeaways()),
                opt_u(v.giveaways()),
                opt_pct(totals.faceoff_win_pct),
                totals.pp_goals.to_string(),
                totals.pp_points.to_string(),
                totals.sh_goals.to_string(),
                totals.gwg.to_string(),
                toi_mmss(totals.toi_per_game_sec),
            )
        }
        None => (
            0,
            0,
            0,
            0,
            "—".to_owned(),
            "—".to_owned(),
            String::new(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
        ),
    };
    let ppg_str = if gp > 0 {
        format!("{:.2}", points as f64 / gp as f64)
    } else {
        String::new()
    };
    Some(ComparePlayerCard {
        nhl_id: id,
        full_name: identity.full_name.clone(),
        position,
        team,
        team_link: team_link.clone(),
        headshot_url: if !team_link.is_empty() {
            Some(super::shared::build_headshot_url(season.0, &team_link, id))
        } else {
            identity.headshot_canonical_url.clone()
        },
        gp,
        goals,
        assists,
        points,
        ppg_str,
        plus_minus_str,
        pim_str,
        shots_str,
        shooting_pct_str,
        hits_str,
        blocks_str,
        takeaways_str,
        giveaways_str,
        faceoff_pct_str,
        pp_goals_str,
        pp_points_str,
        sh_goals_str,
        gwg_str,
        toi_per_game_str,
    })
}

async fn build_compare_result(state: &WebState, q: &CompareQuery) -> CompareResult {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = match cfg.active_season_type.as_str() {
            "playoff" | "playoffs" => SeasonType::Playoff,
            _ => SeasonType::Regular,
        };
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
        Some(raw) => resolve_id(&state, raw).await,
        None => None,
    };
    let b_id = match b_raw {
        Some(raw) => resolve_id(&state, raw).await,
        None => None,
    };

    let (a_card, a_missing) = match a_id {
        Some(id) => {
            let card = build_card(&state, id, season, season_type).await;
            let missing = card.is_none();
            (card, missing.then_some(id))
        }
        None => (None, None),
    };
    let (b_card, b_missing) = match b_id {
        Some(id) => {
            let card = build_card(&state, id, season, season_type).await;
            let missing = card.is_none();
            (card, missing.then_some(id))
        }
        None => (None, None),
    };

    // Distinguish "no input given" vs "input given but didn't
    // resolve to a known player". The latter is more useful to
    // surface with the typed text.
    let a_unresolved = a_raw.filter(|_| a_id.is_none());
    let b_unresolved = b_raw.filter(|_| b_id.is_none());

    let error = if a_raw.is_none() && b_raw.is_none() {
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
    let envelope = CompareEnvelope {
        schema_version: 1,
        route: "compare",
        data: CompareData {
            a: result.a,
            b: result.b,
            winners: result.winners,
        },
        meta: CompareMeta {
            season: result.season,
            season_type: match result.season_type {
                SeasonType::Regular => "regular".to_owned(),
                SeasonType::Playoff => "playoff".to_owned(),
            },
        },
        error: result.error,
    };
    axum::Json(envelope).into_response()
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
