use std::path::Path;
use crate::error::FetchError;

/// Yahoo CSV position eligibility record — the only thing we read from the CSV.
#[derive(Debug)]
pub struct YahooEligibility {
    pub full_name:    String,
    pub team:         String,
    pub eligible_pos: String,  // raw string e.g. "C,LW,Util"
    pub photo_url:    Option<String>,
}

/// Load position eligibility from a Yahoo Fantasy Hockey CSV export.
/// Stats are NOT read — all stats come from the NHL API.
pub fn load_csv_eligibility(_path: &Path) -> Result<Vec<YahooEligibility>, FetchError> {
    // Task 7: full implementation
    Ok(vec![])
}
