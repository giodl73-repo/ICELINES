use crate::error::SiteError;
use std::io::Write as _;
use std::path::Path;

const TEAM_NAMES: &[(&str, &str)] = &[
    ("ANA", "Anaheim Ducks"),
    ("BOS", "Boston Bruins"),
    ("BUF", "Buffalo Sabres"),
    ("CAR", "Carolina Hurricanes"),
    ("CBJ", "Columbus Blue Jackets"),
    ("CGY", "Calgary Flames"),
    ("CHI", "Chicago Blackhawks"),
    ("COL", "Colorado Avalanche"),
    ("DAL", "Dallas Stars"),
    ("DET", "Detroit Red Wings"),
    ("EDM", "Edmonton Oilers"),
    ("FLA", "Florida Panthers"),
    ("LAK", "Los Angeles Kings"),
    ("MIN", "Minnesota Wild"),
    ("MTL", "Montréal Canadiens"),
    ("NJD", "New Jersey Devils"),
    ("NSH", "Nashville Predators"),
    ("NYI", "New York Islanders"),
    ("NYR", "New York Rangers"),
    ("OTT", "Ottawa Senators"),
    ("PHI", "Philadelphia Flyers"),
    ("PIT", "Pittsburgh Penguins"),
    ("SEA", "Seattle Kraken"),
    ("SJS", "San Jose Sharks"),
    ("STL", "St. Louis Blues"),
    ("TBL", "Tampa Bay Lightning"),
    ("TOR", "Toronto Maple Leafs"),
    ("UTA", "Utah Hockey Club"),
    ("VAN", "Vancouver Canucks"),
    ("VGK", "Vegas Golden Knights"),
    ("WPG", "Winnipeg Jets"),
    ("WSH", "Washington Capitals"),
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
        let name = TEAM_NAMES
            .iter()
            .find(|(a, _)| a == abbrev)
            .map(|(_, n)| *n)
            .unwrap_or(abbrev);
        nav.push_str(&format!("    - {abbrev} — {name}: teams/{abbrev}.md\n"));
    }

    let mut f = std::fs::File::create(yml_path)?;
    f.write_all((base + &nav).as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Round-trip: a yml with no existing nav block gets one appended; the
    /// teams appear in the order passed in (NOT alphabetical) so the index
    /// page's ranked sort drives the sidebar.
    #[test]
    fn l1_update_nav_appends_block_when_missing_and_preserves_order() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mkdocs.yml");
        std::fs::write(&path, "site_name: IceLines\n").unwrap();

        let ranked = ["EDM", "FLA", "BOS"];
        update_nav(&path, &ranked).unwrap();

        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains("site_name: IceLines"));
        assert!(out.contains("\nnav:\n"));
        // Order preserved — find the line index of each entry and assert
        // monotonic increasing.
        let edm = out.find("teams/EDM.md").expect("EDM in nav");
        let fla = out.find("teams/FLA.md").expect("FLA in nav");
        let bos = out.find("teams/BOS.md").expect("BOS in nav");
        assert!(edm < fla && fla < bos, "ranked order must be preserved");
    }

    /// A yml with an existing nav block is rewritten — the old nav is
    /// stripped, not duplicated. Catches the bug where a re-run would
    /// pile up multiple `nav:` keys (mkdocs would refuse to load).
    #[test]
    fn l1_update_nav_strips_existing_nav_block_before_writing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mkdocs.yml");
        let initial = "\
site_name: IceLines

nav:
  - Tracker: index.md
  - Teams:
    - OLD — Old Team: teams/OLD.md
";
        std::fs::write(&path, initial).unwrap();

        update_nav(&path, &["EDM"]).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();

        // Exactly one `nav:` key.
        assert_eq!(out.matches("\nnav:\n").count(), 1, "must have exactly one nav block, got {out}");
        // Old entry must be gone.
        assert!(!out.contains("OLD"), "stale nav entry must be stripped");
        // New entry present.
        assert!(out.contains("teams/EDM.md"));
    }

    /// The teamabbrev → full-name lookup feeds into the sidebar label.
    /// Verify the canonical full names land in the output.
    #[test]
    fn l0_update_nav_uses_full_team_names_in_labels() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mkdocs.yml");
        std::fs::write(&path, "site_name: IceLines\n").unwrap();
        update_nav(&path, &["EDM", "MTL"]).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains("EDM — Edmonton Oilers"));
        assert!(out.contains("MTL — Montréal Canadiens"));
    }
}
