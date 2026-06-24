use crate::state::WebState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::signal_metrics::{
    SignalEvidenceTier, SignalInput, SignalMetricUnit, SignalPolarity,
};
use icelines_core::stats_repository::StatsRepository;
use icelines_core::view_model::signals::PlayerSignalsView;
use icelines_core::view_model::signals::SignalsSourceAuthority;

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
