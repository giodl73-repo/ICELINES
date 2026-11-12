//! Optional biographical columns shared by both leaders reports.
use crate::query_report::{select, QueryReport, ReportCell, ReportColumn};
use crate::WebState;
use chrono::{NaiveDate, Utc};
use icelines_core::identity::{PlayerBio, PlayerId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BioColumn {
    #[default]
    Off,
    #[serde(alias = "age")]
    SeasonAge,
    RealAge,
    BothAges,
    Height,
    Weight,
    Nationality,
}
impl BioColumn {
    pub fn token(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::SeasonAge => "season-age",
            Self::RealAge => "real-age",
            Self::BothAges => "both-ages",
            Self::Height => "height",
            Self::Weight => "weight",
            Self::Nationality => "nationality",
        }
    }
    fn columns(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Off => &[],
            Self::SeasonAge => &[("season-age", "Season age")],
            Self::RealAge => &[("real-age", "Age")],
            Self::BothAges => &[("season-age", "Season age"), ("real-age", "Age")],
            Self::Height => &[("height", "Height (in)")],
            Self::Weight => &[("weight", "Weight (lb)")],
            Self::Nationality => &[("nationality", "Nationality")],
        }
    }
}

#[derive(Clone, Default, Serialize)]
pub struct BioDisplay {
    pub column: BioColumn,
    pub season: u32,
    pub real_age_as_of: String,
    pub values: BTreeMap<u32, Vec<String>>,
}
impl BioDisplay {
    pub fn column_labels(&self) -> Vec<&'static str> {
        self.column
            .columns()
            .iter()
            .map(|(_, label)| *label)
            .collect()
    }
    pub async fn load(
        state: &WebState,
        column: BioColumn,
        season: u32,
        team: &str,
        ids: impl IntoIterator<Item = u32>,
    ) -> Self {
        let today = Utc::now().date_naive();
        let mut result = Self {
            column,
            season,
            real_age_as_of: today.to_string(),
            values: BTreeMap::new(),
        };
        if column == BioColumn::Off {
            return result;
        }
        // The saved official roster supplies bios for newly appearing players
        // even when their season identity has not yet entered the stats repo.
        let roster = state
            .snapshots_root
            .as_ref()
            .as_deref()
            .and_then(|root| icelines_fetch::date_range::load_current_roster(root, team).ok())
            .and_then(|r| serde_json::from_str::<serde_json::Value>(&r.source.body).ok());
        let repo = state.repo.read().await;
        for id in ids {
            let mut bio = repo
                .identity(PlayerId(id))
                .map(|p| p.bio.clone())
                .unwrap_or_default();
            if let Some(player) = roster.as_ref().and_then(|r| r.as_object()).and_then(|r| {
                r.values()
                    .filter_map(|v| v.as_array())
                    .flatten()
                    .find(|v| v["id"].as_u64() == Some(u64::from(id)))
            }) {
                if bio.birth_date.is_none() {
                    bio.birth_date = player["birthDate"].as_str().map(str::to_owned);
                }
                if bio.height_in_inches.is_none() {
                    bio.height_in_inches = player["heightInInches"]
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok());
                }
                if bio.weight_lbs.is_none() {
                    bio.weight_lbs = player["weightInPounds"]
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok());
                }
                if bio.nationality_code.is_none() {
                    bio.nationality_code = player["nationalityCode"].as_str().map(str::to_owned);
                }
            }
            result
                .values
                .insert(id, values(column, &bio, season, today));
        }
        result
    }
    pub fn apply(&self, mut report: QueryReport) -> QueryReport {
        report.fields.push(select(
            "bio-column",
            "Bio column",
            self.column.token(),
            [
                ("off", "Off"),
                ("season-age", "Season age"),
                ("real-age", "Age"),
                ("both-ages", "Both ages"),
                ("height", "Height"),
                ("weight", "Weight"),
                ("nationality", "Nationality"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into())),
        ));
        for (offset, (key, label)) in self.column.columns().iter().enumerate() {
            report.columns.insert(
                2 + offset,
                ReportColumn {
                    key: (*key).into(),
                    label: (*label).into(),
                    ..Default::default()
                },
            );
            for row in &mut report.rows {
                let text = self
                    .values
                    .get(&row.id)
                    .and_then(|v| v.get(offset))
                    .map(String::as_str)
                    .unwrap_or("—");
                row.cells.insert(2 + offset, ReportCell::text(text));
            }
        }
        if matches!(self.column, BioColumn::SeasonAge | BioColumn::BothAges) {
            report.context.push((
                "Season age".into(),
                format!(
                    "as of Jan 31, {} ({}–{:02} season)",
                    self.season % 10000,
                    self.season / 10000,
                    self.season % 100
                ),
            ));
        }
        if matches!(self.column, BioColumn::RealAge | BioColumn::BothAges) {
            report
                .context
                .push(("Age".into(), format!("as of {}", self.real_age_as_of)));
        }
        if self.column == BioColumn::Nationality {
            report.context.push((
                "Nationality".into(),
                "Recorded nationality; unavailable values shown as —".into(),
            ));
        }
        report
    }
}
fn values(column: BioColumn, bio: &PlayerBio, season: u32, today: NaiveDate) -> Vec<String> {
    column
        .columns()
        .iter()
        .map(|(key, _)| {
            match *key {
                "season-age" => bio
                    .birth_date
                    .as_deref()
                    .and_then(|d| icelines_query::compute_age(d, season))
                    .map(|n| n.to_string()),
                "real-age" => bio
                    .birth_date
                    .as_deref()
                    .and_then(|d| d.parse::<NaiveDate>().ok())
                    .and_then(|d| today.years_since(d))
                    .map(|n| n.to_string()),
                "height" => bio.height_in_inches.map(|n| n.to_string()),
                "weight" => bio.weight_lbs.map(|n| n.to_string()),
                "nationality" => bio
                    .nationality_code
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| {
                        (2..=3).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphabetic())
                    })
                    .map(str::to_ascii_uppercase),
                _ => None,
            }
            .unwrap_or_else(|| "—".into())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_columns_follow_player_and_off_removes_them_without_changing_rows() {
        use crate::query_report::ReportRow;
        let report = QueryReport {
            columns: ["#", "Player", "GP"]
                .into_iter()
                .map(|label| ReportColumn {
                    label: label.into(),
                    ..Default::default()
                })
                .collect(),
            rows: vec![ReportRow {
                id: 7,
                cells: ["1", "Example Player", "10"]
                    .into_iter()
                    .map(ReportCell::text)
                    .collect(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let selected = BioDisplay {
            column: BioColumn::BothAges,
            season: 20262027,
            real_age_as_of: "2026-11-12".into(),
            values: BTreeMap::from([(7, vec!["27".into(), "26".into()])]),
        }
        .apply(report.clone());
        assert_eq!(
            selected
                .columns
                .iter()
                .map(|c| c.label.as_str())
                .collect::<Vec<_>>(),
            ["#", "Player", "Season age", "Age", "GP"]
        );
        assert_eq!(
            selected.rows[0]
                .cells
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>(),
            ["1", "Example Player", "27", "26", "10"]
        );
        assert!(selected
            .csv()
            .contains("\"Player\",\"Season age\",\"Age\""));
        let off = BioDisplay::default().apply(report);
        assert_eq!(off.columns.len(), 3);
        assert_eq!(off.rows[0].cells.len(), 3);
    }
    #[test]
    fn season_and_real_age_have_distinct_reference_dates_and_missing_bios_stay_missing() {
        let bio = PlayerBio {
            birth_date: Some("2000-01-31".into()),
            birth_country: Some("CAN".into()),
            ..Default::default()
        };
        assert_eq!(
            values(
                BioColumn::BothAges,
                &bio,
                20262027,
                "2026-11-12".parse().unwrap()
            ),
            ["27", "26"]
        );
        assert_eq!(
            values(
                BioColumn::Nationality,
                &bio,
                20262027,
                "2026-11-12".parse().unwrap()
            ),
            ["—"]
        );
        assert_eq!(
            values(
                BioColumn::BothAges,
                &PlayerBio::default(),
                20262027,
                "2026-11-12".parse().unwrap()
            ),
            ["—", "—"]
        );
    }
}
