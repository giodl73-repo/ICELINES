use crate::state::WebState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::signal_metrics::{
    SignalEvidenceTier, SignalInput, SignalMetricUnit, SignalPolarity,
};
use icelines_core::stats_repository::StatsRepository;
use icelines_core::view_model::signals::{
    PlayerSignalsView, SignalRosterEvidenceFilter, SignalsRosterView, SignalsSourceAuthority,
};

#[derive(Debug, serde::Deserialize)]
pub struct TeamSignalsQuery {
    evidence: Option<String>,
}

pub async fn get_player_signals(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    match build_player_signals_view(&state, id).await {
        Ok((active_label, view)) => Html(render_signals_html(&active_label, &view)).into_response(),
        Err(response) => response,
    }
}

pub async fn get_player_signals_json(
    State(state): State<WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_player_signals_view(&state, id).await {
        Ok((_active_label, view)) => {
            let meta = SignalsMeta {
                season: view.context.window.season.0.to_string(),
                season_type: view.context.window.season_type.label().to_owned(),
                player_id: view.player_id,
                player_name: view.player_name.clone(),
                signal_count: view.rows.len(),
                source_authority: view.source_authority.clone(),
            };
            crate::api::json_data_meta("player-signals", view, meta)
        }
        Err(response) => response,
    }
}

pub async fn get_team_signals(
    State(state): State<WebState>,
    Path(abbrev): Path<String>,
    Query(query): Query<TeamSignalsQuery>,
) -> Response {
    match build_team_signals_roster_view(&state, &abbrev, query.evidence.as_deref()).await {
        Ok((active_label, view)) => {
            Html(render_team_signals_roster_html(&active_label, &view)).into_response()
        }
        Err(response) => response,
    }
}

pub async fn get_team_signals_json(
    State(state): State<WebState>,
    Path(abbrev): Path<String>,
    Query(query): Query<TeamSignalsQuery>,
) -> Response {
    match build_team_signals_roster_view(&state, &abbrev, query.evidence.as_deref()).await {
        Ok((_active_label, view)) => {
            let meta = SignalsRosterMeta {
                season: view.season.to_string(),
                season_type: view.season_type.clone(),
                team: view.team.clone(),
                evidence_filter: view.evidence_filter.label().to_string(),
                evidence_filter_scope:
                    "team-scoped Signals roster inspection only; not a leaderboard, StatId promotion, filter catalog, or analytics-cache metric family",
                evidence_filter_docs: signals_roster_evidence_filter_docs(&view.team),
                matched_count: view.rows.len(),
                total_player_count: view.total_player_count,
                filtered_out_count: view.filtered_out_count(),
                source_authority: view.source_authority.clone(),
            };
            crate::api::json_data_meta("team-signals-roster", view, meta)
        }
        Err(response) => response,
    }
}

async fn build_player_signals_view(
    state: &WebState,
    id: u32,
) -> Result<(String, PlayerSignalsView), Response> {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season_u32: u32 = season_str.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Html(format!("Season '{season_str}' is not a valid YYYYZZZZ id")),
        )
            .into_response()
    })?;
    let season = Season(season_u32);
    let pid = PlayerId(id);

    let view = {
        let repo = state.repo.read().await;
        let mut local_repo = player_local_repo(&repo, pid);
        if let Err(e) =
            icelines_fetch::stats_loader::load_player_career_into_repo(&mut local_repo, pid)
        {
            eprintln!(
                "warn: signals career fan-out for pid={id} failed: {e} - \
                 player signals will use only seasons already loaded"
            );
        }
        let player = local_repo.view(pid, season, season_type).or_else(|| {
            let career: Vec<_> = local_repo.career_all(pid)?.collect();
            let latest = career.last()?;
            local_repo.view(pid, latest.season, latest.season_type)
        });
        let Some(player) = player else {
            return Err((
                StatusCode::NOT_FOUND,
                Html(format!(
                    "No player with NHL id {id} in the active repository."
                )),
            )
                .into_response());
        };
        PlayerSignalsView::from_player(PlayerSignalsView::context_for_player(&player), &player)
    };

    Ok((active_label, view))
}

fn player_local_repo(shared: &StatsRepository, pid: PlayerId) -> StatsRepository {
    let mut local = StatsRepository::with_lru_cap(shared.lru_cap());
    if let Some(identity) = shared.identity(pid) {
        if let Err(err) = local.upsert_identity(identity.clone()) {
            eprintln!(
                "warn: local signals repo identity merge for pid={} failed: {err}",
                pid.0
            );
        }
    }
    if let Some(contract) = shared.contract(pid) {
        local.upsert_contract(pid, contract.clone());
    }
    for stats in shared.iter_stats().filter(|stats| stats.player_id == pid) {
        if let Err(err) = local.upsert_stats(stats.clone()) {
            eprintln!(
                "warn: local signals repo stat copy for pid={} failed: {err}",
                pid.0
            );
        }
    }
    local
}

async fn build_team_signals_roster_view(
    state: &WebState,
    abbrev: &str,
    evidence: Option<&str>,
) -> Result<(String, SignalsRosterView), Response> {
    let team = TeamAbbr::parse(abbrev).map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Html(format!("Unknown team '{}'.", html_escape(abbrev))),
        )
            .into_response()
    })?;
    let evidence_filter = parse_evidence_filter(evidence).map_err(|message| {
        (StatusCode::BAD_REQUEST, Html(html_escape(&message))).into_response()
    })?;
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season_u32: u32 = season_str.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Html(format!("Season '{season_str}' is not a valid YYYYZZZZ id")),
        )
            .into_response()
    })?;
    let season = Season(season_u32);
    let view = {
        let repo = state.repo.read().await;
        SignalsRosterView::from_players(
            team.clone(),
            season,
            season_type,
            repo.skaters(season, season_type).collect::<Vec<_>>(),
            evidence_filter,
        )
    };
    if view.rows.is_empty() {
        let message = if view.total_player_count > 0 {
            format!(
                "No Signals roster rows matched evidence filter '{}' for {}. \
                 Try <a href=\"/team/{}/signals?evidence=all\">all</a>, \
                 <a href=\"/team/{}/signals?evidence=partial\">partial</a>, or \
                 <a href=\"/team/{}/signals?evidence=missing\">missing</a>. \
                 These recovery links keep Signals team-scoped and do not rank players or promote Signals to cache, StatId, or leaderboard surfaces.",
                evidence_filter.label(),
                html_escape(&team.0),
                html_escape(&team.0),
                html_escape(&team.0),
                html_escape(&team.0)
            )
        } else {
            format!(
                "No skaters found for {} in {} {}.",
                team.0,
                season.0,
                season_type.label()
            )
        };
        return Err((StatusCode::NOT_FOUND, Html(message)).into_response());
    }
    Ok((active_label, view))
}

fn parse_evidence_filter(value: Option<&str>) -> Result<SignalRosterEvidenceFilter, String> {
    match value.unwrap_or("all").trim().to_ascii_lowercase().as_str() {
        "all" => Ok(SignalRosterEvidenceFilter::All),
        "full" => Ok(SignalRosterEvidenceFilter::Full),
        "partial" => Ok(SignalRosterEvidenceFilter::Partial),
        "missing" => Ok(SignalRosterEvidenceFilter::Missing),
        other => Err(format!(
            "Unknown Signals evidence filter '{other}'. Use all, full, partial, or missing."
        )),
    }
}

fn render_signals_html(active_label: &str, view: &PlayerSignalsView) -> String {
    let rows = view
        .rows
        .iter()
        .map(|row| {
            let value = row
                .value
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "unavailable".to_owned());
            let missing = if row.missing_inputs.is_empty() {
                "none".to_owned()
            } else {
                row.missing_inputs
                    .iter()
                    .map(|input| input_label(*input))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "<tr><td><strong>{}</strong><br><code>{}</code></td>\
                 <td class=\"numeric\">{}</td><td>{}</td><td>{}</td>\
                 <td>{}</td><td>{}</td></tr>",
                html_escape(&row.label),
                html_escape(&row.cli_key),
                html_escape(&value),
                unit_label(row.unit),
                polarity_label(row.polarity),
                tier_label(row.evidence_tier),
                html_escape(&missing)
            )
        })
        .collect::<String>();
    let methodology = view
        .rows
        .iter()
        .map(|row| {
            format!(
                "<li><strong>{}</strong>: {}</li>",
                html_escape(&row.short_label),
                html_escape(&row.methodology)
            )
        })
        .collect::<String>();
    let limitations = view
        .rows
        .iter()
        .map(|row| {
            format!(
                "<li><strong>{}</strong>: {}</li>",
                html_escape(&row.short_label),
                html_escape(&row.limitations)
            )
        })
        .collect::<String>();
    let disclosures = view
        .disclosures
        .iter()
        .map(|line| format!("<p class=\"meta-line\">{}</p>", html_escape(line)))
        .collect::<String>();
    let non_claims = view
        .non_claims
        .iter()
        .map(|line| {
            format!(
                "<p class=\"meta-line\"><strong>Disclaimer:</strong> {}</p>",
                html_escape(line)
            )
        })
        .collect::<String>();
    let source_authority = format!(
        "<section aria-labelledby=\"signals-source-authority\"><h2 id=\"signals-source-authority\">Source authority</h2>\
         <p class=\"meta-line\" data-signals-source-authority=\"{}\" data-coverage-state=\"{}\">{}</p>\
         <p class=\"meta-line\"><strong>Authority source:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Coverage state:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Covered inputs:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Covered metrics:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Blocked claims:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Authority limitations:</strong> {}</p></section>",
        html_escape(&view.source_authority.source),
        html_escape(&view.source_authority.coverage_state),
        html_escape(&view.source_authority.label),
        html_escape(&view.source_authority.source),
        html_escape(&view.source_authority.coverage_state),
        html_escape(&view.source_authority.covered_inputs.join(", ")),
        html_escape(&view.source_authority.covered_metrics.join(", ")),
        html_escape(&view.source_authority.blocked_claims.join(", ")),
        html_escape(&view.source_authority.limitations.join(", "))
    );

    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <title>{name} Signals</title><link rel=\"stylesheet\" href=\"/static/style.css\">\
         </head><body><header><a href=\"/\">IceLines</a> <span>{active}</span></header>\
         <main id=\"main\"><p><a href=\"/player/{pid}\">Back to player card</a> | \
         <a href=\"/api/v1/player/{pid}/signals\">JSON</a></p>\
         <h1>{name} Signals</h1>\
         <p>{position} · {team} · {gp} GP · {active}</p>\
         <p class=\"meta-line\">Descriptive derived metrics from PlayerSignalsView; unavailable values are missing evidence, not zero value truth.</p>\
         {source_authority}\
         <div class=\"table-scroll\"><table><thead><tr><th>Signal</th><th class=\"numeric\">Value</th><th>Unit</th><th>Polarity</th><th>Evidence</th><th>Missing inputs</th></tr></thead><tbody>{rows}</tbody></table></div>\
         <h2>Methodology</h2><ul>{methodology}</ul><h2>Limitations</h2><ul>{limitations}</ul>\
         {disclosures}{non_claims}</main></body></html>",
        name = html_escape(&view.player_name),
        active = html_escape(active_label),
        pid = view.player_id,
        position = html_escape(&view.position),
        team = html_escape(&view.team),
        gp = view.games_played,
    )
}

fn render_team_signals_roster_html(active_label: &str, view: &SignalsRosterView) -> String {
    let rows = view
        .rows
        .iter()
        .map(|player| {
            let phys = signal_cell(player, "physical-engagement-rate");
            let pmd = signal_cell(player, "puck-management-differential");
            let pim = signal_cell(player, "penalty-drag-rate");
            format!(
                "<tr><td><a href=\"/player/{pid}/signals\">{name}</a><br><span class=\"muted\">{pos} · {gp} GP</span></td>\
                 <td>{phys}</td><td>{pmd}</td><td>{pim}</td><td>{evidence}</td></tr>",
                pid = player.player_id,
                name = html_escape(&player.player_name),
                pos = html_escape(&player.position),
                gp = player.games_played,
                phys = html_escape(&phys),
                pmd = html_escape(&pmd),
                pim = html_escape(&pim),
                evidence = html_escape(&row_evidence_summary(player))
            )
        })
        .collect::<String>();
    let disclosures = view
        .disclosures
        .iter()
        .map(|line| format!("<p class=\"meta-line\">{}</p>", html_escape(line)))
        .collect::<String>();
    let non_claims = view
        .non_claims
        .iter()
        .map(|line| {
            format!(
                "<p class=\"meta-line\"><strong>Non-claim:</strong> {}</p>",
                html_escape(line)
            )
        })
        .collect::<String>();
    let source_authority = format!(
        "<section aria-labelledby=\"signals-roster-source-authority\"><h2 id=\"signals-roster-source-authority\">Source authority</h2>\
         <p class=\"meta-line\" data-signals-roster-source-authority=\"{}\" data-coverage-state=\"{}\">{}</p>\
         <p class=\"meta-line\"><strong>Authority source:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Coverage state:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Covered inputs:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Covered metrics:</strong> {}</p>\
         <p class=\"meta-line\"><strong>Blocked claims:</strong> {}</p></section>",
        html_escape(&view.source_authority.source),
        html_escape(&view.source_authority.coverage_state),
        html_escape(&view.source_authority.label),
        html_escape(&view.source_authority.source),
        html_escape(&view.source_authority.coverage_state),
        html_escape(&view.source_authority.covered_inputs.join(", ")),
        html_escape(&view.source_authority.covered_metrics.join(", ")),
        html_escape(&view.source_authority.blocked_claims.join(", "))
    );
    let filter_links = ["all", "full", "partial", "missing"]
        .into_iter()
        .map(|filter| {
            let current = if filter == view.evidence_filter.label() {
                " aria-current=\"page\""
            } else {
                ""
            };
            format!(
                "<li><a href=\"/team/{team}/signals?evidence={filter}\"{current}>{filter}</a> \
                 <a class=\"muted\" href=\"/api/v1/team/{team}/signals?evidence={filter}\">JSON</a></li>",
                team = html_escape(&view.team),
                filter = filter,
                current = current
            )
        })
        .collect::<String>();

    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <title>{team} Signals roster</title><link rel=\"stylesheet\" href=\"/static/style.css\">\
         </head><body><header><a href=\"/\">IceLines</a> <span>{active}</span></header>\
         <main id=\"main\"><p><a href=\"/team/{team}\">Back to team page</a> | \
         <a href=\"/api/v1/team/{team}/signals?evidence={filter}\">JSON</a></p>\
         <h1>{team} Signals roster</h1>\
         <p class=\"meta-line\">{note}</p>\
         <p class=\"meta-line\">Evidence filter: {filter}; rows: {matched} matched / {total} total / {filtered} filtered out.</p>\
         <section aria-labelledby=\"signals-roster-evidence-filters\"><h2 id=\"signals-roster-evidence-filters\">Evidence filters</h2>\
         <p class=\"meta-line\">Filter links narrow this team-scoped discovery matrix only; they do not rank players or promote Signals to cache, StatId, or leaderboard surfaces.</p>\
         <ul>{filter_links}</ul></section>\
         {source_authority}{disclosures}{non_claims}\
         <div class=\"table-scroll\"><table><thead><tr><th>Player</th><th>Phys/60</th><th>PMD/60</th><th>PIM/60</th><th>Evidence</th></tr></thead><tbody>{rows}</tbody></table></div>\
         <p class=\"meta-line\">Legend: unavailable means missing/below-threshold evidence, never zero value truth.</p>\
         </main></body></html>",
        team = html_escape(&view.team),
        active = html_escape(active_label),
        filter = view.evidence_filter.label(),
        note = html_escape(&view.schema_note),
        matched = view.rows.len(),
        total = view.total_player_count,
        filtered = view.filtered_out_count(),
        filter_links = filter_links,
    )
}

fn signal_cell(view: &PlayerSignalsView, key: &str) -> String {
    let row = view
        .rows
        .iter()
        .find(|row| row.cli_key == key)
        .expect("current signal key");
    match row.value {
        Some(value) => format!("{value:.2} {}", tier_label(row.evidence_tier)),
        None => format!("unavailable {}", tier_label(row.evidence_tier)),
    }
}

fn row_evidence_summary(view: &PlayerSignalsView) -> String {
    let mut parts: Vec<String> = Vec::new();
    for row in &view.rows {
        if row.evidence_tier != SignalEvidenceTier::Full || !row.missing_inputs.is_empty() {
            parts.push(format!(
                "{}: {} missing {}",
                row.short_label,
                tier_label(row.evidence_tier),
                missing_inputs_label(&row.missing_inputs)
            ));
        }
    }
    if parts.is_empty() {
        "all full".to_string()
    } else {
        parts.join("; ")
    }
}

fn missing_inputs_label(inputs: &[SignalInput]) -> String {
    if inputs.is_empty() {
        "none".to_string()
    } else {
        inputs
            .iter()
            .map(|input| input_label(*input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn unit_label(unit: SignalMetricUnit) -> &'static str {
    match unit {
        SignalMetricUnit::Per60 => "per 60",
    }
}

fn polarity_label(polarity: SignalPolarity) -> &'static str {
    match polarity {
        SignalPolarity::HigherIsBetter => "higher is better",
        SignalPolarity::LowerIsBetter => "lower is better",
        SignalPolarity::Neutral => "neutral",
    }
}

fn tier_label(tier: SignalEvidenceTier) -> &'static str {
    match tier {
        SignalEvidenceTier::Full => "full",
        SignalEvidenceTier::Partial => "partial",
        SignalEvidenceTier::Missing => "missing",
    }
}

fn input_label(input: SignalInput) -> &'static str {
    match input {
        SignalInput::SampleSize => "sample size",
        SignalInput::Realtime => "realtime",
        SignalInput::IceTime => "ice time",
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Debug, serde::Serialize)]
struct SignalsMeta {
    season: String,
    season_type: String,
    player_id: u32,
    player_name: String,
    signal_count: usize,
    source_authority: SignalsSourceAuthority,
}

#[derive(Debug, serde::Serialize)]
struct SignalsRosterMeta {
    season: String,
    season_type: String,
    team: String,
    evidence_filter: String,
    evidence_filter_scope: &'static str,
    evidence_filter_docs: Vec<SignalsRosterEvidenceFilterDoc>,
    matched_count: usize,
    total_player_count: usize,
    filtered_out_count: usize,
    source_authority: SignalsSourceAuthority,
}

#[derive(Debug, serde::Serialize)]
struct SignalsRosterEvidenceFilterDoc {
    value: &'static str,
    meaning: &'static str,
    html_url: String,
    json_url: String,
}

fn signals_roster_evidence_filter_docs(team: &str) -> Vec<SignalsRosterEvidenceFilterDoc> {
    [
        (
            "all",
            "include every player on the team Signals roster regardless of evidence tier",
        ),
        (
            "full",
            "include only players whose Signals rows are all full-evidence rows",
        ),
        (
            "partial",
            "include players with at least one partial-evidence Signals row",
        ),
        (
            "missing",
            "include players with at least one missing-evidence Signals row",
        ),
    ]
    .into_iter()
    .map(|(value, meaning)| SignalsRosterEvidenceFilterDoc {
        value,
        meaning,
        html_url: format!("/team/{team}/signals?evidence={value}"),
        json_url: format!("/api/v1/team/{team}/signals?evidence={value}"),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::fixtures::{identity, stats, test_repo_with};

    #[test]
    fn l0_web_signals_html_never_zero_fills_missing() {
        let identity = identity(8478402).build();
        let stats = stats(8478402, 20252026, "EDM").build();
        let repo = test_repo_with(identity, stats);
        let player = repo
            .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
            .expect("player view");
        let view =
            PlayerSignalsView::from_player(PlayerSignalsView::context_for_player(&player), &player);

        let html = render_signals_html("25-26 Regular", &view);
        assert!(html.contains("unavailable"));
        assert!(html.contains("missing evidence"));
        assert!(html.contains("data-signals-source-authority=\"PlayerSignalsView stat inputs\""));
        assert!(html.contains("Signals authority: descriptive derived metrics"));
        assert!(!html.contains(">0.00</td><td>per 60</td><td>neutral"));
    }
}
