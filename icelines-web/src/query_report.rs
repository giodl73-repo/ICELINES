//! Shared presentation model for season and date-window leader queries.
use crate::templates::{LeaderRow, LeadersTemplate};
use icelines_core::date_range::{parse_sort_chain, SortTerm};

#[derive(Clone, Default)]
pub struct ReportColumn {
    pub key: String,
    pub label: String,
    pub href: String,
    pub aria_sort: String,
    pub indicator: String,
    pub priority: usize,
    pub order: String,
}
#[derive(Clone, Default)]
pub struct ReportCell {
    pub text: String,
    pub href: String,
    pub image: String,
    pub fallback: String,
}
impl ReportCell {
    pub fn text(text: impl ToString) -> Self {
        Self {
            text: {
                let value = text.to_string();
                if value.is_empty() {
                    "—".into()
                } else {
                    value
                }
            },
            ..Default::default()
        }
    }
    pub fn player(id: u32, name: &str) -> Self {
        Self {
            text: name.into(),
            href: format!("/player/{id}"),
            ..Default::default()
        }
    }
}
#[derive(Clone, Default)]
pub struct ReportRow {
    pub id: u32,
    pub name: String,
    pub team: String,
    pub goals: u32,
    pub points: u32,
    pub highlight: bool,
    pub cells: Vec<ReportCell>,
}
#[derive(Clone, Default)]
pub struct ReportOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}
#[derive(Clone, Default)]
pub struct ReportField {
    pub name: String,
    pub label: String,
    pub value: String,
    pub kind: String,
    pub options: Vec<ReportOption>,
}
pub fn input(name: &str, label: &str, value: &str, kind: &str) -> ReportField {
    ReportField {
        name: name.into(),
        label: label.into(),
        value: value.into(),
        kind: kind.into(),
        options: vec![],
    }
}
pub fn select(
    name: &str,
    label: &str,
    value: &str,
    options: impl IntoIterator<Item = (String, String)>,
) -> ReportField {
    ReportField {
        name: name.into(),
        label: label.into(),
        value: value.into(),
        kind: "select".into(),
        options: options
            .into_iter()
            .map(|(v, l)| ReportOption {
                selected: v == value,
                value: v,
                label: l,
            })
            .collect(),
    }
}
pub fn franchise_label(team: &str) -> String {
    icelines_core::TeamAbbr::parse(team)
        .ok()
        .and_then(|t| t.full_name())
        .map(|name| format!("{team} — {name}"))
        .unwrap_or_else(|| team.into())
}
pub fn positions(value: &str) -> ReportField {
    select(
        "pos",
        "Position",
        value,
        [
            ("", "All skaters"),
            ("F", "Forwards"),
            ("C", "Centers"),
            ("LW", "Left wings"),
            ("RW", "Right wings"),
            ("D", "Defensemen"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into())),
    )
}
#[derive(Clone, Default)]
pub struct QueryReport {
    pub context: Vec<(String, String)>,
    pub columns: Vec<ReportColumn>,
    pub rows: Vec<ReportRow>,
    pub fields: Vec<ReportField>,
    pub hidden: Vec<(String, String)>,
    pub refresh: bool,
    pub json_href: String,
    pub csv_href: String,
    pub error: String,
}
impl QueryReport {
    pub fn csv(&self) -> String {
        fn field(s: &str) -> String {
            let safe = if s.starts_with(['=', '+', '-', '@']) {
                format!("'{s}")
            } else {
                s.into()
            };
            format!("\"{}\"", safe.replace('"', "\"\""))
        }
        let mut lines = vec![self
            .columns
            .iter()
            .map(|c| field(&c.label))
            .collect::<Vec<_>>()
            .join(",")];
        lines.extend(self.rows.iter().map(|r| {
            r.cells
                .iter()
                .map(|c| field(&c.text))
                .collect::<Vec<_>>()
                .join(",")
        }));
        format!("{}\r\n", lines.join("\r\n"))
    }
}
pub fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
pub fn headers(columns: &[(String, String)], base: &str, chain: &[SortTerm]) -> Vec<ReportColumn> {
    columns
        .iter()
        .map(|(key, label)| {
            let index = chain.iter().position(|t| &t.key == key);
            let order = index.map(|i| chain[i].order.as_str()).unwrap_or(
                if matches!(
                    key.as_str(),
                    "name" | "position" | "points-rank" | "ev-rank"
                ) {
                    "asc"
                } else {
                    "desc"
                },
            );
            let next = if chain.len() == 1 && index == Some(0) {
                if order == "desc" {
                    "asc"
                } else {
                    "desc"
                }
            } else {
                order
            };
            ReportColumn {
                key: key.clone(),
                label: label.clone(),
                href: format!("{base}&sort={key}&order={next}"),
                aria_sort: if index != Some(0) {
                    "none"
                } else if order == "asc" {
                    "ascending"
                } else {
                    "descending"
                }
                .into(),
                indicator: if index.is_none() {
                    "↕"
                } else if order == "asc" {
                    "▲"
                } else {
                    "▼"
                }
                .into(),
                priority: index.map(|i| i + 1).unwrap_or(0),
                order: order.into(),
            }
        })
        .collect()
}
pub fn sort_summary(chain: &[SortTerm], columns: &[(String, String)]) -> String {
    chain
        .iter()
        .enumerate()
        .map(|(i, t)| {
            format!(
                "{}. {} {}",
                i + 1,
                columns
                    .iter()
                    .find(|(k, _)| k == &t.key)
                    .map(|(_, l)| l.as_str())
                    .unwrap_or(&t.key),
                if t.order == "desc" { "↓" } else { "↑" }
            )
        })
        .collect::<Vec<_>>()
        .join(" → ")
}
pub fn regular_chain(sort: &str, order: &str, then: &str) -> Result<Vec<SortTerm>, String> {
    use crate::handlers::leaders::SortKey;
    parse_sort_chain(sort, order, then, |key| {
        SortKey::ALL.iter().any(|k| k.url_token() == key)
            || matches!(key, "name" | "position" | "team")
    })
}
pub fn sort_regular(rows: &mut [LeaderRow], chain: &[SortTerm]) {
    fn number(r: &LeaderRow, key: &str) -> Option<f64> {
        Some(match key {
            "gp" => r.gp as f64,
            "goals" => r.goals as f64,
            "assists" => r.assists as f64,
            "points" => r.points as f64,
            "ppg" => {
                if r.gp > 0 {
                    r.points as f64 / r.gp as f64
                } else {
                    return None;
                }
            }
            "plus-minus" => r.plus_minus as f64,
            "pim" => r.pim as f64,
            "shots" => r.shots as f64,
            "shooting-pct" => r.shooting_pct? as f64,
            "hits" => r.hits? as f64,
            "blocks" => r.blocks? as f64,
            "faceoff" => r.faceoff_pct? as f64,
            "ppp" => r.pp_points as f64,
            "p60" => r.points_per_60?,
            "g60" => r.goals_per_60?,
            "a60" => r.assists_per_60?,
            "hits60" => r.hits_per_60?,
            "blocks60" => r.blocks_per_60?,
            "breakout" | "decline" => r.points_delta? as f64,
            _ => return None,
        })
    }
    rows.sort_by(|a, b| {
        for t in chain {
            let c = match t.key.as_str() {
                "name" => a.name.cmp(&b.name),
                "position" => a.position.cmp(&b.position),
                "team" => a.team.cmp(&b.team),
                key => match (number(a, key), number(b, key)) {
                    (Some(a), Some(b)) => a.total_cmp(&b),
                    (Some(_), None) => return std::cmp::Ordering::Less,
                    (None, Some(_)) => return std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                },
            };
            let c = if t.order == "desc" { c.reverse() } else { c };
            if c != std::cmp::Ordering::Equal {
                return c;
            }
        }
        a.nhl_id.cmp(&b.nhl_id)
    });
}
impl LeadersTemplate {
    pub fn report(&self) -> QueryReport {
        let metrics = self
            .col_headers
            .iter()
            .map(|c| (c.url_token.clone(), c.label.clone()))
            .collect::<Vec<_>>();
        let chain = regular_chain(&self.active_sort, &self.query_order, &self.query_then)
            .unwrap_or_default();
        let mut suffix = self.bio_query_suffix.clone();
        for f in &self.active_filters {
            suffix.push_str(&format!("&filter={}", encode(f)))
        }
        let base = format!(
            "/leaders?pos={}&top={}&team={}{}&roster={}",
            self.active_pos,
            self.active_top,
            encode(&self.franchise),
            suffix,
            self.roster_scope
        );
        let mut columns = vec![ReportColumn {
            label: "#".into(),
            ..Default::default()
        }];
        columns.extend(headers(
            &[
                ("name".into(), "Player".into()),
                ("position".into(), "Pos".into()),
                ("team".into(), "Team".into()),
            ],
            &base,
            &chain,
        ));
        columns.extend(headers(&metrics, &base, &chain));
        let rows = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let mut player = ReportCell::player(r.nhl_id, &r.name);
                player.image = r.headshot_url.clone();
                player.fallback = r.headshot_fallback_url.clone();
                let mut cells = vec![
                    ReportCell::text(i + 1),
                    player,
                    ReportCell::text(&r.position),
                    ReportCell {
                        text: r.team.clone(),
                        href: format!("/team/{}", r.team),
                        ..Default::default()
                    },
                ];
                cells.extend(
                    [
                        r.gp.to_string(),
                        r.goals.to_string(),
                        r.assists.to_string(),
                        r.points.to_string(),
                        r.ppg_str.clone(),
                        r.plus_minus_str.clone(),
                        r.pim.to_string(),
                        r.shots.to_string(),
                        r.shooting_pct_str.clone(),
                        r.hits_str.clone(),
                        r.blocks_str.clone(),
                        r.faceoff_pct_str.clone(),
                        r.pp_points.to_string(),
                        r.points_per_60_str.clone(),
                        r.goals_per_60_str.clone(),
                        r.assists_per_60_str.clone(),
                        r.hits_per_60_str.clone(),
                        r.blocks_per_60_str.clone(),
                        r.points_delta_str.clone(),
                        r.points_delta_str.clone(),
                    ]
                    .into_iter()
                    .map(ReportCell::text),
                );
                ReportRow {
                    id: r.nhl_id,
                    name: r.name.clone(),
                    team: r.team.clone(),
                    goals: r.goals,
                    points: r.points,
                    cells,
                    ..Default::default()
                }
            })
            .collect();
        let mut context = vec![
            ("Window".into(), self.active_label.clone()),
            (
                "Franchise".into(),
                if self.franchise.is_empty() {
                    "All franchises".into()
                } else {
                    franchise_label(&self.franchise)
                },
            ),
            (
                "Position".into(),
                if self.active_pos == "F" {
                    "Forwards".into()
                } else if self.active_pos.is_empty() {
                    "All skaters".into()
                } else {
                    self.active_pos.clone()
                },
            ),
            (
                "Scoring scope".into(),
                "Full-season totals (all team stints)".into(),
            ),
            ("Filters".into(), self.result_active_filters.clone()),
            ("Sort".into(), sort_summary(&chain, &metrics)),
        ];
        for (label, low, high) in [
            ("Age", &self.bio_age_min_str, &self.bio_age_max_str),
            (
                "Draft year",
                &self.bio_draft_min_str,
                &self.bio_draft_max_str,
            ),
            ("Height", &self.bio_height_min_str, &self.bio_height_max_str),
            ("Weight", &self.bio_weight_min_str, &self.bio_weight_max_str),
        ] {
            if !low.is_empty() || !high.is_empty() {
                context.push((
                    label.into(),
                    format!(
                        "{} – {}",
                        if low.is_empty() { "Any" } else { low },
                        if high.is_empty() { "Any" } else { high }
                    ),
                ));
            }
        }
        context.push((
            "Roster".into(),
            match self.roster_scope.as_str() {
                "current" => "Current roster only",
                "former" => "Not on current roster",
                _ => "All players who appeared",
            }
            .into(),
        ));
        if !self.roster_fetched_at.is_empty() {
            context.push(("Roster captured".into(), self.roster_fetched_at.clone()))
        }
        if !self.bio_country.is_empty() {
            context.push(("Country".into(), self.bio_country.clone()))
        }
        if !self.bio_shoots.is_empty() {
            context.push(("Handedness".into(), self.bio_shoots.clone()))
        }
        let mut position = positions(&self.active_pos);
        position.options.push(ReportOption {
            value: "G".into(),
            label: "Goalies (The Crease)".into(),
            selected: self.active_pos == "G",
        });
        let mut fields = vec![
            input("team", "Franchise", &self.franchise, "text"),
            input("date-from", "From (inclusive)", "", "date"),
            input("date-to", "Through (inclusive)", "", "date"),
            position,
            select("sort", "Sort", &self.active_sort, metrics),
            select(
                "order",
                "Order",
                &self.query_order,
                [
                    ("desc".into(), "Descending".into()),
                    ("asc".into(), "Ascending".into()),
                ],
            ),
            input("top", "Rows", &self.active_top.to_string(), "number"),
        ];
        fields.insert(
            4,
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
        );
        let mut hidden = vec![
            ("then".into(), self.query_then.clone()),
            ("theme".into(), String::new()),
            ("density".into(), String::new()),
        ];
        hidden.extend(
            self.active_filters
                .iter()
                .map(|f| ("filter".into(), f.clone())),
        );
        for (key, value) in [
            ("age-min", &self.bio_age_min_str),
            ("age-max", &self.bio_age_max_str),
            ("draft-min", &self.bio_draft_min_str),
            ("draft-max", &self.bio_draft_max_str),
            ("height-min", &self.bio_height_min_str),
            ("height-max", &self.bio_height_max_str),
            ("weight-min", &self.bio_weight_min_str),
            ("weight-max", &self.bio_weight_max_str),
            ("country", &self.bio_country),
            ("shoots", &self.bio_shoots),
        ] {
            if !value.is_empty() {
                hidden.push((key.into(), value.clone()))
            }
        }
        let json_href = format!(
            "/api/v1/leaders?{}&sort={}&order={}&then={}",
            base.split_once('?').unwrap().1,
            self.active_sort,
            self.query_order,
            encode(&self.query_then)
        );
        self.bio_display.apply(QueryReport {
            context,
            columns,
            rows,
            fields,
            hidden,
            csv_href: format!("{json_href}&format=csv"),
            json_href,
            ..Default::default()
        })
    }
}
