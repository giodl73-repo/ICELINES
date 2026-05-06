//! Phase Lester Patrick (LP.2) — `icelines playoffs` text bracket.
//!
//! Mirrors the data the TUI Playoffs tab + web /playoffs route already
//! consume (`icelines_fetch::bundled::load_playoffs`). Default season
//! is the most recent COMPLETED playoff in the bundle so the user gets
//! a populated bracket year-round, not an empty frame in the offseason.

use anyhow::{Context, Result};
use icelines_fetch::bundled;
use icelines_fetch::playoffs_bundle::PlayoffsBundle;

/// LP.2 — `PlayoffRow` is the projection used for table / JSON / CSV
/// output. Pure data — formatting decisions live in the emitters.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlayoffRow {
    pub round: u8,
    pub round_label: String,
    pub top_seed: String,
    pub bottom_seed: String,
    pub top_wins: u8,
    pub bottom_wins: u8,
    pub winner: Option<String>,
    pub games_played: u8,
}

/// Project a `PlayoffsBundle` into table rows, optionally filtered to
/// one round. Pure — unit-testable from a JSON fixture.
pub fn project_playoff_rows(bundle: &PlayoffsBundle, round_filter: Option<u8>) -> Vec<PlayoffRow> {
    let bracket = bundle.to_bracket();
    let mut out = Vec::new();
    for round in &bracket.rounds {
        if let Some(want) = round_filter {
            if round.round_number != want {
                continue;
            }
        }
        for series in &round.series {
            out.push(PlayoffRow {
                round: round.round_number,
                round_label: round.label.clone(),
                top_seed: series.top_seed_abbrev.clone(),
                bottom_seed: series.bottom_seed_abbrev.clone(),
                top_wins: series.top_seed_wins,
                bottom_wins: series.bottom_seed_wins,
                winner: series.winner_abbrev.clone(),
                games_played: series.top_seed_wins + series.bottom_seed_wins,
            });
        }
    }
    out
}

/// Pick a default season — most recent COMPLETED playoff. We walk
/// `bundled_playoff_seasons()` newest-first and pick the first whose
/// final-round series has a winner. Falls back to the newest available
/// if none look complete.
fn default_season() -> Option<String> {
    let seasons = bundled::bundled_playoff_seasons();
    let mut newest_complete: Option<String> = None;
    for s in &seasons {
        if let Some(b) = bundled::load_playoffs(s) {
            let bracket = b.to_bracket();
            let final_complete = bracket
                .rounds
                .iter()
                .find(|r| r.round_number == 4)
                .map(|r| r.series.iter().any(|x| x.winner_abbrev.is_some()))
                .unwrap_or(false);
            if final_complete {
                newest_complete = Some((*s).to_owned());
                break;
            }
        }
    }
    newest_complete.or_else(|| seasons.last().map(|s| (*s).to_owned()))
}

pub async fn run(season: Option<String>, round: Option<u8>, json: bool, csv: bool) -> Result<()> {
    let season_id = match season {
        Some(s) => s,
        None => default_season().context(
            "no playoff seasons available in the bundle — run `icelines fetch all` first",
        )?,
    };
    let bundle = bundled::load_playoffs(&season_id).with_context(|| {
        format!(
            "no playoff bundle for season '{season_id}' — \
             try `icelines data list` to see installed seasons"
        )
    })?;

    let rows = project_playoff_rows(&bundle, round);

    if json {
        return emit_json(&bundle, &rows, round);
    }
    if csv {
        return emit_csv(&rows);
    }

    if rows.is_empty() {
        println!("No series match the filter.");
        return Ok(());
    }

    println!(
        "PLAYOFFS — {}{}",
        format_season(&season_id),
        bundle
            .champion
            .as_deref()
            .map(|c| format!("  ·  Champion: {c}"))
            .unwrap_or_default(),
    );
    if let Some(cs) = &bundle.conn_smythe {
        println!("Conn Smythe: {cs}");
    }
    println!();

    use comfy_table::{ContentArrangement, Table};
    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Round", "Series", "Top", "Bottom", "Result", "Winner"]);
    let mut current_round = 0;
    for r in &rows {
        if r.round != current_round {
            table.add_row(vec!["", "", "", "", "", ""]);
            current_round = r.round;
        }
        let result = if r.winner.is_some() {
            format!("{}-{}", r.top_wins, r.bottom_wins)
        } else {
            format!("{}-{} (in progress)", r.top_wins, r.bottom_wins)
        };
        table.add_row(vec![
            r.round_label.as_str(),
            "",
            r.top_seed.as_str(),
            r.bottom_seed.as_str(),
            &result,
            r.winner.as_deref().unwrap_or("—"),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn emit_json(bundle: &PlayoffsBundle, rows: &[PlayoffRow], round_filter: Option<u8>) -> Result<()> {
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        schema_version: u32,
        season: &'a str,
        champion: Option<&'a str>,
        conn_smythe: Option<&'a str>,
        round_filter: Option<u8>,
        count: usize,
        series: &'a [PlayoffRow],
    }
    let env = Envelope {
        schema_version: 1,
        season: &bundle.season,
        champion: bundle.champion.as_deref(),
        conn_smythe: bundle.conn_smythe.as_deref(),
        round_filter,
        count: rows.len(),
        series: rows,
    };
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

fn emit_csv(rows: &[PlayoffRow]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    wtr.write_record([
        "round",
        "round_label",
        "top_seed",
        "bottom_seed",
        "top_wins",
        "bottom_wins",
        "winner",
        "games_played",
    ])?;
    for r in rows {
        wtr.write_record([
            r.round.to_string(),
            r.round_label.clone(),
            r.top_seed.clone(),
            r.bottom_seed.clone(),
            r.top_wins.to_string(),
            r.bottom_wins.to_string(),
            r.winner.clone().unwrap_or_default(),
            r.games_played.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

/// "19921993" → "1992-93".
fn format_season(season_id: &str) -> String {
    if season_id.len() == 8 {
        let start = &season_id[..4];
        let end = &season_id[6..];
        format!("{start}-{end}")
    } else {
        season_id.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LP.2 / l0_default_season_falls_back_to_completed
    /// — Picks a season whose Cup Final has a winner. With 38 bundled
    ///   seasons (1987-88 → 2025-26) this is guaranteed non-None.
    #[test]
    fn l0_default_season_falls_back_to_completed() {
        let s = default_season();
        assert!(
            s.is_some(),
            "default_season must not be None with 38 bundled"
        );
        let s = s.unwrap();
        let bundle = bundled::load_playoffs(&s).expect("default season must load");
        let bracket = bundle.to_bracket();
        let final_winner = bracket
            .rounds
            .iter()
            .find(|r| r.round_number == 4)
            .and_then(|r| r.series.iter().find_map(|x| x.winner_abbrev.clone()));
        assert!(
            final_winner.is_some(),
            "default season {s} must have a Cup winner"
        );
    }

    /// LP.2 / l0_project_filters_round
    #[test]
    fn l0_project_filters_round() {
        let bundle = bundled::load_playoffs("19931994").expect("1993-94 must be in the bundle");
        let all = project_playoff_rows(&bundle, None);
        let r4 = project_playoff_rows(&bundle, Some(4));
        assert_eq!(r4.len(), 1, "Cup Final has exactly one series");
        assert!(all.len() > r4.len());
        assert!(r4.iter().all(|r| r.round == 4));
    }

    /// LP.2 / l0_project_winner_carries_through
    /// — 1993-94: NYR won the Cup. Our row for the Cup Final must
    ///   have winner=Some("NYR").
    #[test]
    fn l0_project_winner_carries_through() {
        let bundle = bundled::load_playoffs("19931994").unwrap();
        let r4 = project_playoff_rows(&bundle, Some(4));
        assert_eq!(r4.len(), 1);
        assert_eq!(r4[0].winner.as_deref(), Some("NYR"));
    }

    /// LP.2 / l0_format_season
    #[test]
    fn l0_format_season() {
        assert_eq!(format_season("19921993"), "1992-93");
        assert_eq!(format_season("20242025"), "2024-25");
        assert_eq!(format_season("garbage"), "garbage");
    }
}
