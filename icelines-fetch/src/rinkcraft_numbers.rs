//! Bounded local source ingestion. No network, cache mutation or quote-derived scores.
use icelines_core::view_model::rinkcraft_numbers::{
    self as core, InputRow, NumericalCatalog, Source,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

pub fn from_csv_files(
    skaters: &Path,
    goalies: &Path,
    season: u32,
    selected: &[u32],
) -> anyhow::Result<NumericalCatalog> {
    anyhow::ensure!(
        (1900..=2200).contains(&(season / 10000)) && season % 10000 == season / 10000 + 1,
        "invalid YYYYZZZZ season"
    );
    let mut rows = Vec::new();
    let mut sources = Vec::new();
    for (path, goalie) in [(skaters, false), (goalies, true)] {
        anyhow::ensure!(
            std::fs::metadata(path)?.len() <= 20 * 1024 * 1024,
            "CSV exceeds 20 MiB limit"
        );
        let data = std::fs::read(path)?;
        anyhow::ensure!(data.len() <= 20 * 1024 * 1024, "CSV exceeds limit");
        rows.extend(parse(&data, season, goalie)?);
        sources.push(Source {
            url: format!(
                "https://moneypuck.com/moneypuck/playerData/seasonSummary/{}/regular/{}.csv",
                season / 10000,
                if goalie { "goalies" } else { "skaters" }
            ),
            sha256: format!("{:x}", Sha256::digest(&data)),
            bytes: data.len(),
        });
    }
    core::build(&rows, season, selected, sources).map_err(anyhow::Error::msg)
}
fn parse(data: &[u8], season: u32, goalie: bool) -> anyhow::Result<Vec<InputRow>> {
    let mut reader = csv::Reader::from_reader(data);
    let headers = reader.headers()?.clone();
    let mappings: Vec<(&str, &str)> = if goalie {
        vec![
            ("xga", "xGoals"),
            ("goals_against", "goals"),
            ("shots_against", "ongoal"),
        ]
    } else {
        vec![
            ("primary_assists", "I_F_primaryAssists"),
            ("secondary_assists", "I_F_secondaryAssists"),
            ("shots", "I_F_shotsOnGoal"),
            ("goals", "I_F_goals"),
            ("ixg", "I_F_xGoals"),
            ("unblocked_shots", "I_F_unblockedShotAttempts"),
            ("high_danger_shots", "I_F_highDangerShots"),
            ("hits", "I_F_hits"),
            ("takeaways", "I_F_takeaways"),
            ("giveaways", "I_F_giveaways"),
            ("penalties_drawn", "penaltiesDrawn"),
            ("penalties_taken", "penalties"),
            ("on_ice_xgf", "OnIce_F_xGoals"),
            ("on_ice_xga", "OnIce_A_xGoals"),
        ]
    };
    let index = |key: &str| {
        headers
            .iter()
            .position(|h| h == key)
            .ok_or_else(|| anyhow::anyhow!("MoneyPuck source missing {key}"))
    };
    for key in [
        "playerId",
        "name",
        "position",
        "season",
        "situation",
        "games_played",
        "icetime",
    ]
    .into_iter()
    .chain(mappings.iter().map(|(_, source)| *source))
    {
        index(key)?;
    }
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let text = |key: &str| -> anyhow::Result<&str> {
            record
                .get(index(key)?)
                .ok_or_else(|| anyhow::anyhow!("missing cell {key}"))
        };
        anyhow::ensure!(
            text("season")?.parse::<u32>()? == season / 10000,
            "CSV season mismatch"
        );
        if text("situation")? != if goalie { "all" } else { "5on5" } {
            continue;
        }
        let group = match text("position")? {
            "G" if goalie => "Goalies",
            "D" if !goalie => "Defense",
            "L" | "R" | "C" if !goalie => "Forwards",
            _ => anyhow::bail!("unexpected source position"),
        };
        let mut values = BTreeMap::new();
        for (key, source) in &mappings {
            let cell = text(source)?;
            if !cell.trim().is_empty() {
                values.insert((*key).into(), cell.parse::<f64>()?);
            }
        }
        rows.push(InputRow {
            player_id: text("playerId")?.parse()?,
            name: text("name")?.into(),
            group: group.into(),
            games: text("games_played")?.parse()?,
            seconds: text("icetime")?.parse()?,
            values,
        });
    }
    anyhow::ensure!(!rows.is_empty(), "no applicable source rows");
    Ok(rows)
}
pub fn load_catalog(path: &Path) -> anyhow::Result<NumericalCatalog> {
    anyhow::ensure!(
        std::fs::metadata(path)?.len() <= 2 * 1024 * 1024,
        "numerical catalog exceeds 2 MiB"
    );
    core::parse_catalog(&std::fs::read_to_string(path)?).map_err(anyhow::Error::msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numerical_csv_checks_schema_season_and_empty_cells() {
        let header =
            "playerId,name,position,season,situation,games_played,icetime,xGoals,goals,ongoal\n";
        let data = format!("{header}1,Synthetic,G,2024,all,20,72000,42,40,500\n");
        let rows = parse(data.as_bytes(), 20242025, true).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seconds, 72000.);
        assert!(parse(data.as_bytes(), 20252026, true).is_err());
        assert!(parse(b"playerId\n1\n", 20242025, true).is_err());
        let empty = format!("{header}1,Synthetic,G,2024,all,20,72000,,40,500\n");
        let rows = parse(empty.as_bytes(), 20242025, true).unwrap();
        assert!(!rows[0].values.contains_key("xga"));
    }
}
