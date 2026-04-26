use std::io::Write as _;
use std::path::Path;
use crate::error::SiteError;

const TEAM_NAMES: &[(&str, &str)] = &[
    ("ANA","Anaheim Ducks"),("BOS","Boston Bruins"),("BUF","Buffalo Sabres"),
    ("CAR","Carolina Hurricanes"),("CBJ","Columbus Blue Jackets"),("CGY","Calgary Flames"),
    ("CHI","Chicago Blackhawks"),("COL","Colorado Avalanche"),("DAL","Dallas Stars"),
    ("DET","Detroit Red Wings"),("EDM","Edmonton Oilers"),("FLA","Florida Panthers"),
    ("LAK","Los Angeles Kings"),("MIN","Minnesota Wild"),("MTL","Montréal Canadiens"),
    ("NJD","New Jersey Devils"),("NSH","Nashville Predators"),("NYI","New York Islanders"),
    ("NYR","New York Rangers"),("OTT","Ottawa Senators"),("PHI","Philadelphia Flyers"),
    ("PIT","Pittsburgh Penguins"),("SEA","Seattle Kraken"),("SJS","San Jose Sharks"),
    ("STL","St. Louis Blues"),("TBL","Tampa Bay Lightning"),("TOR","Toronto Maple Leafs"),
    ("UTA","Utah Hockey Club"),("VAN","Vancouver Canucks"),("VGK","Vegas Golden Knights"),
    ("WPG","Winnipeg Jets"),("WSH","Washington Capitals"),
];

/// Rewrite the nav section of mkdocs.yml with teams in ranked order.
pub fn update_nav(yml_path: &Path, ranked_teams: &[&str]) -> Result<(), SiteError> {
    let content = std::fs::read_to_string(yml_path)?;

    // Strip existing nav block
    let base = if let Some(idx) = content.find("\nnav:") {
        content[..idx].to_owned()
    } else {
        content
    };

    let mut nav = "\nnav:\n  - Tracker: index.md\n  - Teams:\n".to_owned();
    for abbrev in ranked_teams {
        let name = TEAM_NAMES.iter()
            .find(|(a, _)| a == abbrev)
            .map(|(_, n)| *n)
            .unwrap_or(abbrev);
        nav.push_str(&format!("    - {abbrev} — {name}: teams/{abbrev}.md\n"));
    }

    let mut f = std::fs::File::create(yml_path)?;
    f.write_all((base + &nav).as_bytes())?;
    Ok(())
}
