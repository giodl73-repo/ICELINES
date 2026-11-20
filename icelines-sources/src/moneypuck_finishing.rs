//! Exact-scope f64 parsing for the finishing pilot; no source-local defaults.
use icelines_core::view_model::{rinkcraft_finishing::Position, rinkcraft_finishing_fit::Row};
use serde::Deserialize;
use std::collections::BTreeSet;
#[derive(Debug)]
pub struct PlayerRow {
    pub name: String,
    pub row: Row,
}
#[derive(Deserialize)]
struct Raw {
    #[serde(rename = "playerId")]
    id: u32,
    name: String,
    season: u32,
    situation: String,
    position: String,
    games_played: f64,
    icetime: f64,
    #[serde(rename = "I_F_goals")]
    goals: f64,
    #[serde(rename = "I_F_xGoals")]
    xg: f64,
}
pub fn parse(bytes: &[u8], season: u32) -> Result<Vec<PlayerRow>, String> {
    if season / 10000 + 1 != season % 10000 {
        return Err("invalid requested season".into());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader.headers().map_err(|e| e.to_string())?;
    for key in [
        "playerId",
        "name",
        "season",
        "situation",
        "position",
        "games_played",
        "icetime",
        "I_F_goals",
        "I_F_xGoals",
    ] {
        if !headers.iter().any(|h| h == key) {
            return Err(format!("missing finishing column: {key}"));
        }
    }
    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    for raw in reader.deserialize::<Raw>() {
        let r = raw.map_err(|e| e.to_string())?;
        if r.season != season / 10000 {
            return Err("CSV season mismatch".into());
        }
        if r.situation != "5on5" {
            continue;
        }
        let position = match r.position.as_str() {
            "C" => Position::C,
            "L" => Position::L,
            "R" => Position::R,
            "D" => Position::D,
            _ => return Err("non-skater position".into()),
        };
        if r.id == 0
            || !ids.insert(r.id)
            || !r.games_played.is_finite()
            || r.games_played < 0.
            || r.games_played.fract() != 0.
            || r.games_played > f64::from(u32::MAX)
            || !r.goals.is_finite()
            || r.goals < 0.
            || r.goals.fract() != 0.
            || r.goals > f64::from(u32::MAX)
            || !r.icetime.is_finite()
            || r.icetime < 0.
            || !r.xg.is_finite()
            || r.xg < 0.
            || (r.xg == 0. && r.goals > 0.)
        {
            return Err("duplicate identity or invalid finishing evidence".into());
        }
        rows.push(PlayerRow {
            name: r.name,
            row: Row {
                player_id: r.id,
                season,
                position,
                goals: r.goals as u32,
                xg: r.xg,
                games: r.games_played as u32,
                minutes: r.icetime / 60.,
            },
        });
    }
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str =
        "playerId,name,season,situation,position,games_played,icetime,I_F_goals,I_F_xGoals\n";
    #[test]
    fn exact_scope_zero_and_double_precision() {
        let raw=format!("{HEADER}1,Example,2023,all,C,20,6000,4.0,5.12345678901234\n1,Example,2023,5on5,C,20,6000,0.0,5.12345678901234\n");
        let rows = parse(raw.as_bytes(), 20232024).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].row.goals, 0);
        assert_eq!(rows[0].row.xg, 5.12345678901234);
        assert_eq!(rows[0].row.minutes, 100.);
    }
    #[test]
    fn wrong_year_duplicates_and_invalid_counts_fail() {
        assert!(parse(b"playerId,name\n", 20232024).is_err());
        for body in [
            "1,A,2024,5on5,C,20,6000,1,2\n",
            "1,A,2023,5on5,C,20,6000,0.5,2\n",
            "1,A,2023,5on5,G,20,6000,1,2\n",
            "1,A,2023,5on5,C,20,6000,1,0\n",
            "1,A,2023,5on5,C,20,6000,1,2\n1,A,2023,5on5,C,20,6000,1,2\n",
        ] {
            assert!(parse(format!("{HEADER}{body}").as_bytes(), 20232024).is_err());
        }
    }
}
