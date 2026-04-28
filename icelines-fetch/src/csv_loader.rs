use crate::error::FetchError;
use std::io::Read;
use std::path::Path;

/// Yahoo CSV position eligibility record.
/// Stats are NOT read from CSV — all stats come from the NHL API.
/// Only position eligibility and photo URL are extracted.
#[derive(Debug, Clone)]
pub struct YahooEligibility {
    pub full_name: String,
    pub team: String,
    pub eligible_pos: String, // raw string e.g. "C,LW,Util"
    pub photo_url: Option<String>,
}

/// Required column names in the Yahoo CSV header row.
const REQUIRED_COLS: &[&str] = &["First Name", "Last Name", "Team", "Eligible Positions"];

/// Load position eligibility from a Yahoo Fantasy Hockey CSV export.
/// Handles UTF-8 BOM. Validates required columns by name.
/// Rejects rows with empty Team or Eligible Positions.
pub fn load_csv_eligibility(path: &Path) -> Result<Vec<YahooEligibility>, FetchError> {
    let raw = read_file_strip_bom(path)?;

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true) // Yahoo sometimes has trailing commas
        .from_reader(raw.as_bytes());

    // Validate headers
    let headers = rdr
        .headers()
        .map_err(|e| FetchError::CsvParse {
            row: 0,
            field: "headers".into(),
            detail: e.to_string(),
        })?
        .clone();

    let header_vec: Vec<&str> = headers.iter().collect();
    for required in REQUIRED_COLS {
        if !header_vec.contains(required) {
            return Err(FetchError::CsvParse {
                row: 0,
                field: required.to_string(),
                detail: format!("required column '{required}' not found in CSV headers"),
            });
        }
    }

    let col = |name: &str| -> usize {
        header_vec
            .iter()
            .position(|h| *h == name)
            .unwrap_or(usize::MAX)
    };

    let idx_first = col("First Name");
    let idx_last = col("Last Name");
    let idx_team = col("Team");
    let idx_eligible = col("Eligible Positions");
    let idx_image = col("Image");

    let mut records = Vec::new();
    for (row_num, result) in rdr.records().enumerate() {
        let record = result.map_err(|e| FetchError::CsvParse {
            row: row_num + 2,
            field: "row".into(),
            detail: e.to_string(),
        })?;

        let get = |idx: usize| -> &str {
            if idx == usize::MAX {
                ""
            } else {
                record.get(idx).unwrap_or("").trim()
            }
        };

        let first = get(idx_first);
        let last = get(idx_last);
        let team = get(idx_team);
        let eligible = get(idx_eligible);

        if team.is_empty() || eligible.is_empty() {
            continue;
        }

        let full_name = format!("{first} {last}");
        let photo_url = {
            let raw = get(idx_image);
            if raw.is_empty() {
                None
            } else {
                Some(raw.to_owned())
            }
        };

        records.push(YahooEligibility {
            full_name,
            team: team.to_owned(),
            eligible_pos: eligible.to_owned(),
            photo_url,
        });
    }

    Ok(records)
}

fn read_file_strip_bom(path: &Path) -> Result<String, FetchError> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    // Strip UTF-8 BOM (EF BB BF) if present
    let stripped = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        &bytes
    };
    // Decode as UTF-8, replacing invalid sequences rather than failing
    Ok(String::from_utf8_lossy(stripped).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_csv(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn l1_csv_parses_basic_row() {
        let f = write_csv(
            "ID,OR,Last Name,First Name,Team,Status,Status Details,Eligible Positions,Image\n\
             p.1,1,McDavid,Connor,EDM,Available,,C,https://photo.example.com/1.png\n",
        );
        let rows = load_csv_eligibility(f.path()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].full_name, "Connor McDavid");
        assert_eq!(rows[0].team, "EDM");
        assert_eq!(rows[0].eligible_pos, "C");
    }

    #[test]
    fn l1_csv_strips_bom() {
        let mut content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        content.extend_from_slice(
            b"ID,OR,Last Name,First Name,Team,Status,Status Details,Eligible Positions,Image\n\
              p.1,1,Beniers,Matty,SEA,Available,,C,\n",
        );
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(&content).unwrap();
        let rows = load_csv_eligibility(f.path()).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn l1_csv_missing_required_column_is_error() {
        // Missing "Eligible Positions" column
        let f = write_csv("ID,Last Name,First Name,Team\np.1,McDavid,Connor,EDM\n");
        assert!(load_csv_eligibility(f.path()).is_err());
    }

    #[test]
    fn l1_csv_empty_team_row_skipped() {
        let f = write_csv(
            "ID,OR,Last Name,First Name,Team,Status,Status Details,Eligible Positions,Image\n\
             p.1,1,Ghost,Player,,Available,,C,\n",
        );
        let rows = load_csv_eligibility(f.path()).unwrap();
        assert_eq!(rows.len(), 0, "empty team rows must be skipped");
    }

    #[test]
    fn l1_csv_accented_name_preserved() {
        let f = write_csv(
            "ID,OR,Last Name,First Name,Team,Status,Status Details,Eligible Positions,Image\n\
             p.1,1,Slafkovský,Juraj,MTL,Available,,LW,\n",
        );
        let rows = load_csv_eligibility(f.path()).unwrap();
        assert_eq!(rows[0].full_name, "Juraj Slafkovský");
    }
}
