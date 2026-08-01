use crate::error::FetchError;
pub use icelines_sources::yahoo_eligibility::YahooEligibility;
use icelines_sources::yahoo_eligibility::{parse_yahoo_eligibility_csv, YahooEligibilityCsvError};
use std::path::Path;

/// Load position eligibility from a Yahoo Fantasy Hockey CSV export.
/// Handles UTF-8 BOM. Validates required columns by name.
/// Rejects rows with empty Team or Eligible Positions.
pub fn load_csv_eligibility(path: &Path) -> Result<Vec<YahooEligibility>, FetchError> {
    let bytes = std::fs::read(path)?;
    parse_yahoo_eligibility_csv(&bytes).map_err(|error| match error {
        YahooEligibilityCsvError::CsvParse { row, field, detail } => {
            FetchError::CsvParse { row, field, detail }
        }
    })
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
