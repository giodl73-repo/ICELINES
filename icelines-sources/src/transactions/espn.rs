use crate::schema::{RawTransaction, RawTransactionTeam};

pub fn season_to_date_range(season: &str) -> Option<String> {
    let (start_year, end_year) = season_years(season)?;
    Some(format!("{start_year}0901-{end_year}0731"))
}

pub fn season_month_windows(season: &str) -> Option<Vec<String>> {
    let (start_year, end_year) = season_years(season)?;
    let months = [
        (start_year, 9),
        (start_year, 10),
        (start_year, 11),
        (start_year, 12),
        (end_year, 1),
        (end_year, 2),
        (end_year, 3),
        (end_year, 4),
        (end_year, 5),
        (end_year, 6),
        (end_year, 7),
    ];
    Some(
        months
            .into_iter()
            .map(|(year, month)| {
                let last = days_in_month(year, month);
                format!("{year}{month:02}01-{year}{month:02}{last:02}")
            })
            .collect(),
    )
}

fn season_years(season: &str) -> Option<(u32, u32)> {
    if season.len() != 8 || !season.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let start_year: u32 = season[..4].parse().ok()?;
    let end_year: u32 = season[4..].parse().ok()?;
    (end_year == start_year + 1).then_some((start_year, end_year))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => {
            29
        }
        2 => 28,
        _ => 30,
    }
}

/// Parse one ESPN page while recording additive schema drift.
pub fn parse_page_with_fallback(body: &serde_json::Value) -> (Vec<RawTransaction>, Vec<String>) {
    let mut rows = Vec::new();
    let mut dropped = Vec::new();

    let Some(transactions) = body.get("transactions").and_then(|value| value.as_array()) else {
        dropped.push("(missing top-level 'transactions' array)".to_owned());
        return (rows, dropped);
    };

    for raw in transactions {
        for (key, _) in raw.as_object().into_iter().flat_map(|map| map.iter()) {
            if !matches!(key.as_str(), "date" | "description" | "team") {
                push_unique(&mut dropped, format!("transactions[].{key}"));
            }
        }
        let date = raw
            .get("date")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned();
        let description = raw
            .get("description")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned();
        let team = raw.get("team").and_then(|team_value| {
            let id = team_value
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let abbreviation = team_value
                .get("abbreviation")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let display_name = team_value
                .get("displayName")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            for (key, _) in team_value
                .as_object()
                .into_iter()
                .flat_map(|map| map.iter())
            {
                if !matches!(key.as_str(), "id" | "abbreviation" | "displayName") {
                    push_unique(&mut dropped, format!("transactions[].team.{key}"));
                }
            }
            if id.is_empty() && abbreviation.is_empty() && display_name.is_empty() {
                None
            } else {
                Some(RawTransactionTeam {
                    id: id.to_owned(),
                    abbreviation: abbreviation.to_owned(),
                    display_name: display_name.to_owned(),
                })
            }
        });
        if date.is_empty() && description.is_empty() {
            continue;
        }
        rows.push(RawTransaction {
            date,
            description,
            team,
        });
    }
    (rows, dropped)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_page_with_fallback, season_month_windows};
    use serde_json::json;

    #[test]
    fn creates_full_hockey_calendar_windows() {
        let windows = season_month_windows("20252026").expect("valid season");
        assert_eq!(windows.len(), 11);
        assert_eq!(windows[0], "20250901-20250930");
        assert_eq!(windows[10], "20260701-20260731");
    }

    #[test]
    fn preserves_rows_and_records_additive_drift() {
        let (rows, dropped) = parse_page_with_fallback(&json!({
            "transactions": [{
                "date": "2026-04-29",
                "description": "Recalled F X",
                "unexpected": true,
                "team": {"id":"3", "abbreviation":"NYR", "displayName":"Rangers", "logo":"x"}
            }]
        }));
        assert_eq!(rows.len(), 1);
        assert!(dropped.contains(&"transactions[].unexpected".to_owned()));
        assert!(dropped.contains(&"transactions[].team.logo".to_owned()));
    }
}
