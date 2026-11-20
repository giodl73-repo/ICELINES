use crate::WebState;
use askama::Template;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use chrono::{NaiveDate, Utc};
use icelines_core::date_range::{
    scoring_rows, select_scoring_rows, ScoringFilter, ScoringRow, SCORING_COLUMNS,
};
use icelines_fetch::date_range::{self, CurrentRosterCapture, DateRangeCapture, DateRangeQuery};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Params {
    #[serde(default, rename = "bio-column")]
    bio_column: crate::query_bio::BioColumn,
    #[serde(default)]
    selected: String,
    #[serde(default)]
    top: Option<usize>,
    #[serde(default)]
    team: String,
    #[serde(
        default,
        rename = "date-from",
        deserialize_with = "super::leaders::deserialize_optional_number"
    )]
    date_from: Option<NaiveDate>,
    #[serde(
        default,
        rename = "date-to",
        deserialize_with = "super::leaders::deserialize_optional_number"
    )]
    date_to: Option<NaiveDate>,
    #[serde(default, rename = "type")]
    season_type: String,
    #[serde(default)]
    sort: String,
    #[serde(default)]
    order: String,
    #[serde(default, rename = "then")]
    then_by: String,
    #[serde(default)]
    pos: String,
    #[serde(default)]
    roster: String,
    #[serde(default)]
    highlight: u32,
    #[serde(default)]
    format: String,
    #[serde(default)]
    theme: String,
    #[serde(default)]
    density: String,
}
impl Params {
    fn query(&self) -> anyhow::Result<DateRangeQuery> {
        anyhow::ensure!(
            matches!(self.season_type.as_str(), "" | "regular" | "playoff"),
            "type must be regular or playoff"
        );
        self.filter().validate().map_err(anyhow::Error::msg)?;
        let q = DateRangeQuery {
            team: self.team.to_ascii_uppercase(),
            date_from: self
                .date_from
                .ok_or_else(|| anyhow::anyhow!("choose a start date"))?,
            date_to: self
                .date_to
                .ok_or_else(|| anyhow::anyhow!("choose an end date"))?,
            game_type: if self.season_type == "playoff" { 3 } else { 2 },
        };
        q.validate()?;
        Ok(q)
    }
    fn filter(&self) -> ScoringFilter {
        ScoringFilter {
            then_by: self.then_by.clone(),
            position: self.pos.to_ascii_uppercase(),
            roster_scope: if self.roster.is_empty() {
                "all".into()
            } else {
                self.roster.clone()
            },
            sort: self.sort().into(),
            order: self.order().into(),
        }
    }
    fn sort(&self) -> &str {
        match self.sort.as_str() {
            "" | "pts" | "p" => "points",
            "gp" => "games",
            "ppp" => "pp-points",
            s => s,
        }
    }
    fn order(&self) -> &str {
        if self.order.is_empty() {
            "desc"
        } else {
            &self.order
        }
    }
    fn link(&self, q: &DateRangeQuery) -> String {
        format!(
            "team={}&date-from={}&date-to={}&type={}&sort={}&highlight={}&pos={}&roster={}&theme={}&density={}&order={}",
            q.team,
            q.date_from,
            q.date_to,
            if q.game_type == 3 {
                "playoff"
            } else {
                "regular"
            },
            self.sort(),
            self.highlight,
            self.filter().position,
            self.filter().roster_scope,
            if matches!(self.theme.as_str(),"night-game" | "fresh-ice" | "hockey-club" | "nhl") {&self.theme} else {""},
            if matches!(self.density.as_str(),"compact" | "comfortable") {&self.density} else {""},
            self.order()
        ) + &format!("&then={}&top={}&bio-column={}&selected={}",crate::query_report::encode(&self.then_by),self.top.unwrap_or(500),self.bio_column.token(),crate::query_report::encode(&self.selected))
    }
}
fn season_for_date(date: NaiveDate) -> u32 {
    use chrono::Datelike;
    let start = date.year() as u32 - u32::from(date.month() < 7);
    start * 10000 + start + 1
}
type ColumnHeader = crate::query_report::ReportColumn;
fn report_metrics() -> Vec<(String, String)> {
    SCORING_COLUMNS
        .iter()
        .filter(|(key, _)| !matches!(*key, "points-rank" | "ev-rank"))
        .map(|(key, label)| (key.to_string(), label.to_string()))
        .collect()
}
fn column_headers(p: &Params, q: Option<&DateRangeQuery>) -> Vec<ColumnHeader> {
    let chain = p.filter().sort_chain().unwrap_or_default();
    let base = q
        .map(|q| {
            p.link(q)
                .split('&')
                .filter(|part| {
                    !part.starts_with("sort=")
                        && !part.starts_with("order=")
                        && !part.starts_with("then=")
                })
                .collect::<Vec<_>>()
                .join("&")
        })
        .unwrap_or_default();
    let mut columns = vec![ColumnHeader {
        label: "#".into(),
        ..Default::default()
    }];
    columns.extend(crate::query_report::headers(
        &report_metrics(),
        &format!("/leaders?{base}"),
        &chain,
    ));
    columns
}
pub async fn html_from_uri(state: WebState, uri: axum::http::Uri) -> Response {
    if !icelines_query::parse_filters_from_query(uri.query().unwrap_or("")).is_empty() {
        return (StatusCode::BAD_REQUEST,"Stat-expression filters are not yet available for date-window evidence; clear them before selecting date bounds.").into_response();
    }
    match Query::<Params>::try_from_uri(&uri) {
        Ok(q) => get_html(State(state), q).await,
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
pub async fn export_from_uri(state: WebState, uri: axum::http::Uri) -> Response {
    if !icelines_query::parse_filters_from_query(uri.query().unwrap_or("")).is_empty() {
        return (StatusCode::BAD_REQUEST,"Stat-expression filters are not yet available for date-window evidence; clear them before selecting date bounds.").into_response();
    }
    match Query::<Params>::try_from_uri(&uri) {
        Ok(q) => get_export(State(state), q).await,
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
pub async fn legacy_redirect(uri: axum::http::Uri) -> Response {
    Redirect::to(&format!("/leaders?{}", uri.query().unwrap_or(""))).into_response()
}
fn root(state: &WebState) -> anyhow::Result<&std::path::Path> {
    state
        .snapshots_root
        .as_ref()
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("snapshot store is unavailable"))
}
struct ResultSet {
    capture: DateRangeCapture,
    rows: Vec<ScoringRow>,
    roster: Option<CurrentRosterCapture>,
    total: usize,
}
fn cached(state: &WebState, p: &Params) -> anyhow::Result<ResultSet> {
    let q = p.query()?;
    let c = date_range::load(root(state)?, &q)?;
    let rows = scoring_rows(
        &c.games,
        &q.team,
        q.date_from,
        q.date_to,
        q.game_type,
        p.sort(),
    )
    .map_err(anyhow::Error::msg)?;
    let filter = p.filter();
    let roster = if filter.roster_scope == "all" {
        None
    } else {
        Some(date_range::load_current_roster(root(state)?, &q.team)?)
    };
    let mut rows = select_scoring_rows(rows, &filter, roster.as_ref().map(|r| &r.player_ids))
        .map_err(anyhow::Error::msg)?;
    let total = rows.len();
    if let Some(top) = p.top {
        rows.truncate(top.clamp(1, 500));
    }
    Ok(ResultSet {
        total,
        capture: c,
        rows,
        roster,
    })
}
#[derive(Template)]
#[template(path = "date_range.html")]
struct Page {
    bio_display: crate::query_bio::BioDisplay,
    top: usize,
    total: usize,
    active_label: String,
    team: String,
    date_from: String,
    date_to: String,
    playoff: bool,
    sort: String,
    order: String,
    columns: Vec<ColumnHeader>,
    then_by: String,
    position: String,
    roster_scope: String,
    roster_evidence: String,
    highlight: u32,
    link: String,
    rows: Vec<ScoringRow>,
    error: String,
    fetched_at: String,
    latest: String,
    game_count: usize,
    focus: String,
    method: &'static str,
}
impl Page {
    fn report(&self) -> crate::query_report::QueryReport {
        use crate::query_report::{
            input, positions, select, sort_summary, QueryReport, ReportCell, ReportRow,
        };
        let filter = ScoringFilter {
            position: self.position.clone(),
            roster_scope: self.roster_scope.clone(),
            sort: self.sort.clone(),
            order: self.order.clone(),
            then_by: self.then_by.clone(),
        };
        let metrics = report_metrics();
        let context = vec![
            (
                "Date range".into(),
                format!("{} → {} (inclusive)", self.date_from, self.date_to),
            ),
            (
                "Franchise".into(),
                crate::query_report::franchise_label(&self.team),
            ),
            (
                "Position".into(),
                if self.position == "F" {
                    "Forwards (C, LW, RW)".into()
                } else if self.position.is_empty() {
                    "All skaters".into()
                } else {
                    self.position.clone()
                },
            ),
            (
                "Roster".into(),
                match self.roster_scope.as_str() {
                    "current" => "Current roster only",
                    "former" => "Not on current roster",
                    _ => "All players who appeared",
                }
                .into(),
            ),
            (
                "Games".into(),
                if self.playoff {
                    "Playoffs"
                } else {
                    "Regular season"
                }
                .into(),
            ),
            ("Scoring scope".into(), format!("{} games only", self.team)),
            (
                "Sort priorities".into(),
                sort_summary(&filter.sort_chain().unwrap_or_default(), &metrics),
            ),
        ];
        let mut context = context;
        context.push((
            "Players".into(),
            format!("{} of {} matched", self.rows.len(), self.total),
        ));
        let rows = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, r)| {
                let mut player = ReportCell::player(r.player_id, &r.name);
                player.image = super::shared::build_headshot_url(
                    icelines_core::CURRENT_SEASON,
                    &self.team,
                    r.player_id,
                );
                player.fallback = format!(
                    "https://assets.nhle.com/mugs/nhl/default/{}.png",
                    r.player_id
                );
                let mut cells = vec![
                    ReportCell::text(index + 1),
                    player,
                    ReportCell::text(&r.position),
                ];
                cells.extend(
                    [
                        r.games.to_string(),
                        r.goals.to_string(),
                        r.assists.to_string(),
                        r.points.to_string(),
                        r.ev_points.to_string(),
                        format!("{:.3}", r.points_per_game),
                        format!("{:.3}", r.ev_points_per_game),
                        r.pp_points.to_string(),
                        r.sh_points.to_string(),
                    ]
                    .into_iter()
                    .map(ReportCell::text),
                );
                ReportRow {
                    id: r.player_id,
                    name: r.name.clone(),
                    team: self.team.clone(),
                    goals: r.goals,
                    points: r.points,
                    highlight: r.player_id == self.highlight,
                    cells,
                }
            })
            .collect();
        let mut fields = vec![
            input("team", "Franchise", &self.team, "text"),
            input("date-from", "From (inclusive)", &self.date_from, "date"),
            input("date-to", "Through (inclusive)", &self.date_to, "date"),
            positions(&self.position),
            select(
                "roster",
                "Roster membership",
                &self.roster_scope,
                [
                    ("all", "All players who appeared"),
                    ("current", "Current roster only"),
                    ("former", "Not on current roster"),
                ]
                .into_iter()
                .map(|(k, l)| (k.into(), l.into())),
            ),
            select(
                "type",
                "Games",
                if self.playoff { "playoff" } else { "regular" },
                [("regular", "Regular season"), ("playoff", "Playoffs")]
                    .into_iter()
                    .map(|(k, l)| (k.into(), l.into())),
            ),
            select("sort", "Sort", &self.sort, metrics),
            select(
                "order",
                "Order",
                &self.order,
                [
                    ("desc".into(), "Descending".into()),
                    ("asc".into(), "Ascending".into()),
                ],
            ),
        ];
        fields.push(input("top", "Rows", &self.top.to_string(), "number"));
        self.bio_display.apply(QueryReport {
            context,
            columns: self.columns.clone(),
            rows,
            fields,
            hidden: vec![
                ("then".into(), self.then_by.clone()),
                ("highlight".into(), self.highlight.to_string()),
                ("theme".into(), String::new()),
                ("density".into(), String::new()),
            ],
            refresh: true,
            json_href: format!("/api/v1/leaders?{}", self.link),
            csv_href: format!("/api/v1/leaders?{}&format=csv", self.link),
            error: self.error.clone(),
        })
    }
}
pub async fn get_html(State(state): State<WebState>, Query(p): Query<Params>) -> Response {
    let today = Utc::now().date_naive();
    let q = p.query();
    let link = q.as_ref().map(|q| p.link(q)).unwrap_or_default();
    let mut page = Page {
        bio_display: Default::default(),
        top: p.top.unwrap_or(500).clamp(1, 500),
        total: 0,
        active_label: "Custom dates · team scoring".into(),
        team: p.team.to_ascii_uppercase(),
        date_from: p
            .date_from
            .map(|d| d.to_string())
            .unwrap_or_else(|| (today - chrono::Duration::days(30)).to_string()),
        date_to: p
            .date_to
            .map(|d| d.to_string())
            .unwrap_or_else(|| (today - chrono::Duration::days(1)).to_string()),
        playoff: p.season_type == "playoff",
        sort: p.sort().into(),
        order: p.order().into(),
        columns: column_headers(&p, q.as_ref().ok()),
        then_by: p.then_by.clone(),
        position: p.filter().position,
        roster_scope: p.filter().roster_scope,
        roster_evidence: String::new(),
        highlight: p.highlight,
        link,
        rows: vec![],
        error: String::new(),
        fetched_at: String::new(),
        latest: String::new(),
        game_count: 0,
        focus: String::new(),
        method: date_range::METHOD,
    };
    if !p.team.is_empty() {
        match cached(&state, &p) {
            Ok(ResultSet {
                capture: c,
                rows,
                roster,
                total,
            }) => {
                page.total = total;
                page.roster_evidence=roster.map(|r|format!("Official current roster captured {}. Membership is measured at capture time, not at the end of the historical window.",r.fetched_at)).unwrap_or_default();
                page.game_count = c.game_ids.len();
                page.fetched_at = c.fetched_at;
                page.latest = c
                    .games
                    .iter()
                    .map(|g| g.game_date)
                    .max()
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "No games".into());
                if let Some(r) = rows.iter().find(|r| r.player_id == p.highlight) {
                    page.focus = format!(
                        "{}: {} points, {} EV points, {:.3} P/G and {:.3} EVP/G in {} games",
                        r.name,
                        r.points,
                        r.ev_points,
                        r.points_per_game,
                        r.ev_points_per_game,
                        r.games
                    );
                }
                page.rows = rows;
            }
            Err(e) => page.error = e.to_string(),
        }
    }
    let season = p
        .date_to
        .map(season_for_date)
        .unwrap_or(icelines_core::CURRENT_SEASON);
    page.bio_display = crate::query_bio::BioDisplay::load(
        &state,
        p.bio_column,
        season,
        &page.team,
        page.rows.iter().map(|r| r.player_id),
    )
    .await;
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
pub async fn refresh(State(state): State<WebState>, Form(p): Form<Params>) -> Response {
    if !state.config.read().await.date_range_refresh_enabled {
        return (
            StatusCode::FORBIDDEN,
            "Live feeds are disabled; cached date-range queries remain available.",
        )
            .into_response();
    }
    let result = async {
        if p.date_from.is_none() && p.date_to.is_none() && p.filter().roster_scope != "all" {
            date_range::refresh_current_roster(root(&state)?, &p.team.to_ascii_uppercase()).await?;
            return Ok::<_, anyhow::Error>(format!(
                "/leaders?team={}&pos={}&sort={}&order={}&then={}&roster={}&top={}&bio-column={}&selected={}",
                crate::query_report::encode(&p.team),
                crate::query_report::encode(&p.pos),
                p.sort(),
                p.order(),
                crate::query_report::encode(&p.then_by),
                p.filter().roster_scope,
                p.top.unwrap_or(20),
                p.bio_column.token(),
                crate::query_report::encode(&p.selected)
            ));
        }
        let q = p.query()?;
        if p.filter().roster_scope != "all" {
            date_range::refresh_current_roster(root(&state)?, &q.team).await?;
        }
        date_range::refresh(root(&state)?, &q).await?;
        Ok::<_, anyhow::Error>(format!("/leaders?{}", p.link(&q)))
    }
    .await;
    match result {
        Ok(url) => Redirect::to(&url).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Refresh failed; previous capture retained. {e:#}"),
        )
            .into_response(),
    }
}
pub async fn get_export(State(state): State<WebState>, Query(p): Query<Params>) -> Response {
    match cached(&state, &p) {
        Ok(ResultSet {
            capture: c,
            rows,
            roster,
            total,
        }) => {
            let bio_display = crate::query_bio::BioDisplay::load(
                &state,
                p.bio_column,
                season_for_date(c.query.date_to),
                &c.query.team,
                rows.iter().map(|r| r.player_id),
            )
            .await;
            if p.format == "csv" {
                let mut csv =
                    "#,Player ID,Player,Pos,GP,G,A,P,EV P,P/G,EVP/G,PP P,SH P\r\n".to_owned();
                for (offset, label) in bio_display.column_labels().iter().enumerate() {
                    let mut parts = csv
                        .trim_end()
                        .split(',')
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    parts.insert(3 + offset, (*label).into());
                    csv = parts.join(",") + "\r\n";
                }
                for (index, r) in rows.into_iter().enumerate() {
                    let name = r.name.replace('"', "\"\"");
                    let name = if name.starts_with(['=', '+', '-', '@']) {
                        format!("'{name}")
                    } else {
                        name
                    };
                    let bio_cells = bio_display
                        .values
                        .get(&r.player_id)
                        .map(|values| {
                            values
                                .iter()
                                .map(|s| format!(",\"{}\"", s.replace('"', "\"\"")))
                                .collect::<String>()
                        })
                        .unwrap_or_default();
                    csv.push_str(&format!(
                        "{},{},\"{}\"{},{},{},{},{},{},{},{:.3},{:.3},{},{}\r\n",
                        index + 1,
                        r.player_id,
                        name,
                        bio_cells,
                        r.position,
                        r.games,
                        r.goals,
                        r.assists,
                        r.points,
                        r.ev_points,
                        r.points_per_game,
                        r.ev_points_per_game,
                        r.pp_points,
                        r.sh_points
                    ));
                }
                (
                    [
                        (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                        (
                            header::CONTENT_DISPOSITION,
                            "attachment; filename=icelines-date-range.csv",
                        ),
                    ],
                    csv,
                )
                    .into_response()
            } else {
                axum::Json(serde_json::json!({"schema_version":1,"route":"leaders","data":rows,
                    "meta":{"bio_display":bio_display,"query":c.query,"fetched_at":c.fetched_at,"sort":p.sort(),"order":p.order(),"sort_chain":p.filter().sort_chain().unwrap_or_default(),"filters":p.filter(),
                    "current_roster":roster.map(|r|serde_json::json!({"team":r.team,"fetched_at":r.fetched_at,"url":r.source.url,"sha256":r.source.sha256})),"total":total,"top":p.top,"completed_games":c.game_ids.len(),
                    "latest_game":c.games.iter().map(|g|g.game_date).max(),"methodology":date_range::METHOD,
                    "sources":c.sources.iter().map(|s|serde_json::json!({"url":s.url,"sha256":s.sha256})).collect::<Vec<_>>()}})).into_response()
            }
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn date_range_headers_toggle_order_and_preserve_result_context() {
        let empty = Query::<Params>::try_from_uri(
            &"/leaders?date-from=&date-to=&team=AAA&roster=current"
                .parse()
                .unwrap(),
        )
        .unwrap()
        .0;
        assert!(empty.date_from.is_none() && empty.date_to.is_none());
        let p = Params {
            team: "AAA".into(),
            date_from: Some("2026-10-01".parse().unwrap()),
            date_to: Some("2026-11-01".parse().unwrap()),
            pos: "F".into(),
            roster: "current".into(),
            sort: "ppg".into(),
            bio_column: crate::query_bio::BioColumn::BothAges,
            selected: "1,2".into(),
            theme: "fresh-ice".into(),
            density: "comfortable".into(),
            highlight: 42,
            ..Default::default()
        };
        let q = p.query().unwrap();
        let headers = column_headers(&p, Some(&q));
        assert_eq!(headers.len(), 12);
        assert_eq!(headers[0].label, "#");
        assert!(headers[0].href.is_empty());
        assert!(headers
            .iter()
            .all(|h| !matches!(h.key.as_str(), "points-rank" | "ev-rank")));
        let rate = headers.iter().find(|h| h.key == "ppg").unwrap();
        assert_eq!(rate.aria_sort, "descending");
        assert!(rate.href.contains("sort=ppg"));
        assert!(rate.href.contains("order=asc"));
        assert!(rate.href.contains("pos=F&roster=current"));
        assert!(rate.href.contains("theme=fresh-ice&density=comfortable"));
        assert!(rate.href.contains("highlight=42"));
        assert!(rate.href.contains("bio-column=both-ages"));
        assert!(rate.href.contains("selected=1%2C2"));
        assert!(headers
            .iter()
            .find(|h| h.key == "name")
            .unwrap()
            .href
            .contains("order=asc"));
        let p = Params {
            order: "asc".into(),
            ..p
        };
        let headers = column_headers(&p, Some(&q));
        let rate = headers.iter().find(|h| h.key == "ppg").unwrap();
        assert_eq!(rate.aria_sort, "ascending");
        assert!(rate.href.contains("order=desc"));
    }
    #[tokio::test]
    async fn date_range_exports_filter_membership_positions_and_rates_with_evidence() {
        use icelines_fetch::date_range::CapturedSource;
        use sha2::{Digest, Sha256};
        use std::collections::BTreeSet;
        let root = tempfile::tempdir().unwrap();
        let p = Params {
            team: "AAA".into(),
            date_from: Some("2026-10-01".parse().unwrap()),
            date_to: Some("2026-11-01".parse().unwrap()),
            pos: "F".into(),
            roster: "current".into(),
            sort: "ev-ppg".into(),
            highlight: 1,
            ..Default::default()
        };
        let q = p.query().unwrap();
        let games=(1..=2).map(|id|serde_json::from_value(serde_json::json!({"playerId":id,"skaterFullName":format!("Player {id}"),"positionCode":if id==1 {"L"} else {"D"},"teamAbbrev":"AAA","gameId":2026020001_u64,"gameDate":"2026-10-05","gamesPlayed":1,"goals":id,"assists":id,"points":id*2,"evPoints":id*2-1,"ppPoints":1,"shPoints":0})).unwrap()).collect();
        let source = |url: &str, body: String| CapturedSource {
            url: url.into(),
            sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
            body,
        };
        let capture = DateRangeCapture {
            schema_version: 1,
            query: q.clone(),
            fetched_at: "2026-11-12T15:00:00Z".into(),
            game_ids: BTreeSet::from([2026020001]),
            games,
            sources: vec![source("https://example.test/scoring", "{}".into())],
        };
        icelines_fetch::atomic_write::write_json_atomic(&q.path(root.path()), &capture).unwrap();
        let roster = CurrentRosterCapture {
            schema_version: 1,
            team: "AAA".into(),
            fetched_at: "2026-11-12T16:00:00Z".into(),
            player_ids: BTreeSet::from([1]),
            source: source(
                "https://api-web.nhle.com/v1/roster/AAA/current",
                serde_json::json!({"forwards":[{"id":1}],"defensemen":[],"goalies":[]}).to_string(),
            ),
        };
        icelines_fetch::atomic_write::write_json_atomic(
            &date_range::current_roster_path(root.path(), "AAA").unwrap(),
            &roster,
        )
        .unwrap();
        let state = WebState {
            snapshots_root: std::sync::Arc::new(Some(root.path().to_owned())),
            ..Default::default()
        };
        let result = cached(&state, &p).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].points_rank, 1);
        let html = get_html(State(state.clone()), Query(p)).await;
        let bytes = axum::body::to_bytes(html.into_body(), 1_000_000)
            .await
            .unwrap();
        let html = std::str::from_utf8(&bytes).unwrap();
        assert!(html.contains("Applied query filters"));
        assert!(html.contains("Forwards (C, LW, RW)"));
        assert!(html.contains("Current roster only"));
        assert!(html.contains("2.000"));
        assert!(html.contains("1.000"));
        assert!(html.contains("2026-11-12T16:00:00Z"));
        use tower::ServiceExt;
        let canonical=crate::router(state.clone()).oneshot(axum::http::Request::builder().uri("/leaders?team=AAA&date-from=2026-10-01&date-to=2026-11-01&pos=F&roster=current&sort=ppg&order=desc&then=ev-ppg:desc").body(axum::body::Body::empty()).unwrap()).await.unwrap();
        assert_eq!(canonical.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(canonical.into_body(), 1_000_000)
            .await
            .unwrap();
        let canonical = std::str::from_utf8(&bytes).unwrap();
        assert!(canonical.contains("query-controls"));
        assert!(canonical.contains("Date range"));
        assert!(canonical.contains("Franchise"));
        assert!(canonical.contains("data-sort-priority=\"2\""));
        assert!(canonical.contains("/api/v1/leaders?"));

        assert!(html.contains("pos=F&amp;roster=current"));
        let response = get_export(
            State(state),
            Query(Params {
                team: "AAA".into(),
                date_from: q.date_from.into(),
                date_to: q.date_to.into(),
                roster: "former".into(),
                ..Default::default()
            }),
        )
        .await;
        let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["data"].as_array().unwrap().len(), 1);
        assert_eq!(json["data"][0]["player_id"], 2);
        assert_eq!(json["data"][0]["points_per_game"], 4.0);
        assert_eq!(json["meta"]["filters"]["roster_scope"], "former");
        assert_eq!(
            json["meta"]["current_roster"]["fetched_at"],
            "2026-11-12T16:00:00Z"
        );
    }
    #[tokio::test]
    async fn date_range_form_and_offline_errors_are_visible() {
        let response = get_html(State(WebState::default()), Query(Params::default())).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("Refresh data &amp; run"));
        let p = Params {
            team: "AAA".into(),
            date_from: Some("2026-10-01".parse().unwrap()),
            date_to: Some("2026-11-01".parse().unwrap()),
            ..Default::default()
        };
        let response = get_export(State(WebState::default()), Query(p)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let state = WebState::default();
        state.config.write().await.date_range_refresh_enabled = false;
        assert_eq!(
            refresh(State(state), Form(Params::default()))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
}
