use crate::state::WebState;
use crate::templates::ScoutingTemplate;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    scouting_report_sections, MetricCell, MetricValue, PlayerCardView, ReportFormat, ReportKind,
    ReportView, ViewContext, ViewWindow,
};

pub async fn get_scouting(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    let report = match build_scouting_report(&state, id).await {
        Ok(report) => report,
        Err(resp) => return resp,
    };

    let active_label = {
        let cfg = state.config.read().await;
        cfg.active_label.clone()
    };
    let tmpl = ScoutingTemplate {
        active_label,
        title: report.context.title.clone(),
        player_href: format!("/player/{id}"),
        rendered_html: render_markdown(&report.rendered_body),
        markdown_body: report.rendered_body,
    };

    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => error_500(format!("template render failed: {e}")),
    }
}

pub async fn get_scouting_json(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    let report = match build_scouting_report(&state, id).await {
        Ok(report) => report,
        Err(resp) => return resp,
    };

    let meta = ScoutingMeta {
        report_id: report.context.report_id.clone(),
        title: report.context.title.clone(),
        format: report.format.label().to_owned(),
        sections: report.context.sections.len(),
    };
    crate::api::json_data_meta("scouting", report, meta)
}

async fn build_scouting_report(state: &WebState, id: u32) -> Result<ReportView, Response> {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = season_str
        .parse()
        .map_err(|_| error_500(format!("active season '{season_str}' is not a YYYYZZZZ id")))?;
    let season = Season(season_u32);
    let pid = PlayerId(id);

    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!(
                "warn: career fan-out for pid={id} failed: {e} - scouting report will show only seasons already loaded"
            );
        }
    }

    let mut card = {
        let repo = state.repo.read().await;
        PlayerCardView::from_repository(&repo, pid, season, season_type).ok_or_else(|| {
            error_404(format!(
                "No player with NHL id {id} in the active repository."
            ))
        })?
    };
    let pre_nhl_stints = {
        let store = icelines_fetch::career_landing::load_local_store();
        store
            .get(id)
            .map(icelines_fetch::career_landing::extract_pre_nhl_stints)
            .unwrap_or_default()
    };
    card = card.with_pre_nhl_stints(&pre_nhl_stints);

    Ok(scouting_report_from_card(card, season, season_type))
}

fn scouting_report_from_card(
    card: PlayerCardView,
    season: Season,
    season_type: SeasonType,
) -> ReportView {
    let title = format!("Scouting Report - {}", card.display_name);
    let body = render_scouting_markdown(&card, season_type);
    ReportView::rendered(
        ViewContext::new(ViewWindow::new(season, season_type)),
        ReportKind::Scouting,
        format!("scouting-{}", card.player_id.0),
        title,
        ReportFormat::Markdown,
        scouting_report_sections(),
        body,
    )
}

fn render_scouting_markdown(card: &PlayerCardView, season_type: SeasonType) -> String {
    let active_metrics = card
        .active
        .as_ref()
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);
    let gp = metric_u32(active_metrics, "gp").unwrap_or(0);
    let goals = metric_u32(active_metrics, "goals").unwrap_or(0);
    let assists = metric_u32(active_metrics, "assists").unwrap_or(0);
    let points = metric_u32(active_metrics, "points").unwrap_or(0);
    let ppg = metric_f64(active_metrics, "points_per_game")
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "-".to_owned());
    let team = card
        .active
        .as_ref()
        .map(|active| active.team_display.clone())
        .unwrap_or_else(|| "-".to_owned());
    let position = card
        .active
        .as_ref()
        .map(|active| active.position.abbreviation().to_owned())
        .unwrap_or_else(|| "-".to_owned());

    let career_points: u32 = card
        .career
        .iter()
        .filter(|row| row.season_type == season_type)
        .filter_map(|row| metric_u32(&row.metrics, "points"))
        .sum();
    let pre_nhl_line = card
        .pre_nhl_career
        .first()
        .map(|row| {
            format!(
                "{} {}: {} points in {} GP",
                row.season_label,
                row.league,
                row.points
                    .map(|points| points.to_string())
                    .unwrap_or_else(|| "-".to_owned()),
                row.games
            )
        })
        .unwrap_or_else(|| "No local pre-NHL stints loaded.".to_owned());

    format!(
        "# Scouting Report - {name}\n\n\
         ## Bio\n\
         - Position: {position}\n\
         - Team: {team}\n\n\
         ## Current Season\n\
         - {gp} GP, {goals} G, {assists} A, {points} P, {ppg} P/GP\n\n\
         ## Career Trajectory\n\
         - {career_rows} NHL {season_type_label} rows in the loaded repository.\n\
         - {career_points} points across those loaded rows.\n\n\
         ## Peer Group Rank\n\
         - Use leaders and compare surfaces for active peer ordering.\n\n\
         ## Linemates\n\
         - Not populated by the current repository snapshot.\n\n\
         ## Depth Chart Position\n\
         - Use team and depth surfaces for current roster context.\n\n\
         ## Cross-Team Value\n\
         - Cross-team comparison is available through depth and compare views.\n\n\
         ## Fit Interpretation\n\
         - Pre-NHL context: {pre_nhl_line}\n",
        name = card.display_name,
        position = position,
        team = team,
        gp = gp,
        goals = goals,
        assists = assists,
        points = points,
        ppg = ppg,
        career_rows = card
            .career
            .iter()
            .filter(|row| row.season_type == season_type)
            .count(),
        season_type_label = season_type.label(),
        career_points = career_points,
        pre_nhl_line = pre_nhl_line
    )
}

fn render_markdown(markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown, opts);
    let mut out = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut out, parser);
    out
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

fn metric_f64(metrics: &[MetricCell], key: &str) -> Option<f64> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Decimal(value) => Some(value),
            _ => None,
        })
}

#[derive(Debug, serde::Serialize)]
struct ScoutingMeta {
    report_id: String,
    title: String,
    format: String,
    sections: usize,
}

fn error_404(msg: String) -> Response {
    (
        StatusCode::NOT_FOUND,
        Html(format!(
            "<!doctype html><html><body><h1>Scouting report not found</h1><p>{msg}</p></body></html>"
        )),
    )
        .into_response()
}

fn error_500(msg: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html(format!(
            "<!doctype html><html><body><h1>500</h1><p>{msg}</p></body></html>"
        )),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_scouting_report_wraps_player_card_in_report_view() {
        let (identity, stats) = icelines_core::fixtures::stat_catalog_variants::skater_modern();
        let repo = icelines_core::fixtures::test_repo_with(identity, stats);
        let card = PlayerCardView::from_repository(
            &repo,
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
        )
        .expect("fixture card");

        let report = scouting_report_from_card(card, Season(20242025), SeasonType::Regular);

        assert_eq!(report.context.kind, ReportKind::Scouting);
        assert_eq!(report.context.sections.len(), 8);
        assert!(report.rendered_body.contains("# Scouting Report"));
        assert!(report.rendered_body.contains("## Current Season"));
    }

    #[test]
    fn l0_scouting_markdown_renders_to_html() {
        let html = render_markdown("# Scouting Report\n\n## Current Season\n");
        assert!(html.contains("<h1>Scouting Report</h1>"));
        assert!(html.contains("<h2>Current Season</h2>"));
    }
}
