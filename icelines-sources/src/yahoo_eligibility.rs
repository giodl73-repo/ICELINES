//! Yahoo fantasy eligibility parsing from caller-supplied CSV bytes.

/// Yahoo CSV position-eligibility record. Performance stats are deliberately
/// absent; they remain sourced from hockey-stat providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YahooEligibility {
    pub full_name: String,
    pub team: String,
    pub eligible_pos: String,
    pub photo_url: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum YahooEligibilityCsvError {
    #[error("CSV parse error at row {row}, field '{field}': {detail}")]
    CsvParse {
        row: usize,
        field: String,
        detail: String,
    },
}

const REQUIRED_COLS: &[&str] = &["First Name", "Last Name", "Team", "Eligible Positions"];

/// Parse position eligibility and optional photo URLs from a Yahoo fantasy
/// export. UTF-8 BOMs are accepted and malformed UTF-8 is replaced exactly as
/// in the legacy file loader.
pub fn parse_yahoo_eligibility_csv(
    bytes: &[u8],
) -> Result<Vec<YahooEligibility>, YahooEligibilityCsvError> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());

    let headers = reader
        .headers()
        .map_err(|error| csv_error(0, "headers", error))?
        .clone();
    let header_vec = headers.iter().collect::<Vec<_>>();
    for required in REQUIRED_COLS {
        if !header_vec.contains(required) {
            return Err(YahooEligibilityCsvError::CsvParse {
                row: 0,
                field: (*required).to_owned(),
                detail: format!("required column '{required}' not found in CSV headers"),
            });
        }
    }

    let column = |name: &str| {
        header_vec
            .iter()
            .position(|header| *header == name)
            .unwrap_or(usize::MAX)
    };
    let first = column("First Name");
    let last = column("Last Name");
    let team = column("Team");
    let eligible = column("Eligible Positions");
    let image = column("Image");

    let mut rows = Vec::new();
    for (index, result) in reader.records().enumerate() {
        let row_number = index + 2;
        let record = result.map_err(|error| csv_error(row_number, "row", error))?;
        let get = |column: usize| {
            if column == usize::MAX {
                ""
            } else {
                record.get(column).unwrap_or("").trim()
            }
        };
        let team = get(team);
        let eligible_pos = get(eligible);
        if team.is_empty() || eligible_pos.is_empty() {
            continue;
        }
        rows.push(YahooEligibility {
            full_name: format!("{} {}", get(first), get(last)),
            team: team.to_owned(),
            eligible_pos: eligible_pos.to_owned(),
            photo_url: match get(image) {
                "" => None,
                url => Some(url.to_owned()),
            },
        });
    }
    Ok(rows)
}

fn csv_error(row: usize, field: &str, error: csv::Error) -> YahooEligibilityCsvError {
    YahooEligibilityCsvError::CsvParse {
        row,
        field: field.to_owned(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bom_unicode_and_optional_image() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(
            "Last Name,First Name,Team,Eligible Positions,Image\nSlafkovský,Juraj,MTL,LW,https://example.test/juraj.png\n"
                .as_bytes(),
        );
        let rows = parse_yahoo_eligibility_csv(&bytes).unwrap();
        assert_eq!(
            rows,
            vec![YahooEligibility {
                full_name: "Juraj Slafkovský".to_owned(),
                team: "MTL".to_owned(),
                eligible_pos: "LW".to_owned(),
                photo_url: Some("https://example.test/juraj.png".to_owned()),
            }]
        );
    }

    #[test]
    fn rejects_missing_required_header() {
        let error = parse_yahoo_eligibility_csv(b"Last Name,First Name,Team\nMcDavid,Connor,EDM\n")
            .unwrap_err();
        assert!(matches!(
            error,
            YahooEligibilityCsvError::CsvParse { row: 0, .. }
        ));
    }
}
