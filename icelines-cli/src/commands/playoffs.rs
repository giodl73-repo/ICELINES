//! Phase Lester Patrick (LP.2) — `icelines playoffs` text bracket.
//!
//! Mirrors the data the TUI Playoffs tab + web /playoffs route already
//! consume (`icelines_fetch::bundled::load_playoffs`). Default season
//! is the most recent COMPLETED playoff in the bundle so the user gets
//! a populated bracket year-round, not an empty frame in the offseason.

use anyhow::{Context, Result};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    PlayoffsBracketInput, PlayoffsRoundInput, PlayoffsSeriesInput, PlayoffsView, ViewContext,
    ViewWindow,
};
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
    let view = playoffs_view_from_bundle(bundle);
    view.rounds
        .iter()
        .filter(|round| {
            round_filter
                .map(|want| round.round_number == want)
                .unwrap_or(true)
        })
        .flat_map(|round| {
            round.series.iter().map(|series| PlayoffRow {
                round: round.round_number,
                round_label: round.label.clone(),
                top_seed: series.top_abbrev.clone(),
                bottom_seed: series.bottom_abbrev.clone(),
                top_wins: series.top_wins,
                bottom_wins: series.bottom_wins,
                winner: series.winner_abbrev.clone(),
                games_played: series.games_played,
            })
        })
        .collect()
}

fn playoffs_view_from_bundle(bundle: &PlayoffsBundle) -> PlayoffsView {
    let season = bundle
        .season
        .parse::<u32>()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    PlayoffsView::from_bracket(
        ViewContext::new(ViewWindow::new(Season(season), SeasonType::Playoff)),
        bundle.season.clone(),
        "historical bundle".to_string(),
        playoff_bracket_input(bundle.to_bracket()),
    )
}

fn playoff_bracket_input(bracket: icelines_fetch::nhl_api::PlayoffBracket) -> PlayoffsBracketInput {
    PlayoffsBracketInput {
        rounds: bracket
            .rounds
            .into_iter()
            .map(|round| PlayoffsRoundInput {
                round_number: round.round_number,
                label: round.label,
                series: round
                    .series
                    .into_iter()
                    .map(|series| PlayoffsSeriesInput {
                        top_abbrev: series.top_seed_abbrev,
                        top_name: series.top_seed_name,
                        top_wins: series.top_seed_wins,
                        bottom_abbrev: series.bottom_seed_abbrev,
                        bottom_name: series.bottom_seed_name,
                        bottom_wins: series.bottom_seed_wins,
                        winner_abbrev: series.winner_abbrev,
                        conference: series.conference,
                    })
                    .collect(),
            })
            .collect(),
    }
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
    // Post-LP review fix #2: bundled_playoff_seasons() is newest-first
    // (matching BUNDLED_SEASONS convention). When no season has a
    // completed Cup Final, fall back to the FIRST entry (newest) — not
    // last(), which would jump back to the oldest in the list.
    newest_complete.or_else(|| seasons.first().map(|s| (*s).to_owned()))
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
    // Post-LP review fix #9 — drop the empty-row separator between
    // rounds. comfy-table renders an all-empty row as cell borders
    // around blank space (looks like missing data), not as a section
    // divider. The Round column already changes value per round so
    // the structure is visible without artificial gaps.
    for r in &rows {
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

// ── Phase Conn Smythe C.1 — series momentum drill-down ──────────────────────

pub async fn run_series_momentum(
    season: Option<String>,
    series_letter: String,
    json: bool,
) -> Result<()> {
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

    let bracket = bundle.to_bracket();
    // Find the round + series matching the requested letter.
    let needle = series_letter.to_uppercase();
    let found = bracket
        .rounds
        .iter()
        .find_map(|r| {
            r.series
                .iter()
                .find(|s| {
                    s.letter
                        .as_deref()
                        .map(|l| l.eq_ignore_ascii_case(&needle))
                        .unwrap_or(false)
                })
                .map(|s| (r, s))
        })
        .with_context(|| {
            format!(
                "no series '{}' in {season_id} playoffs — try `icelines playoffs --season {season_id}` to see series letters",
                series_letter
            )
        })?;
    let (round, series) = found;

    let season_u32: u32 = season_id
        .parse()
        .context("season ID must be 8-digit YYYYZZZZ")?;
    let momentum = icelines_fetch::series_momentum_builder::compute_series_momentum(
        icelines_core::model::Season(season_u32),
        round,
        series,
    );

    if json {
        return emit_momentum_json(&season_id, &momentum);
    }
    print_momentum_text(&season_id, &momentum);
    Ok(())
}

fn print_momentum_text(season_id: &str, m: &icelines_core::series_momentum::SeriesMomentum) {
    println!(
        "SERIES {} — {}  ·  {} (top seed) vs {}",
        m.series_letter, m.round_label, m.top_seed_abbrev.0, m.bottom_seed_abbrev.0,
    );
    println!(
        "{}  ·  {} games played, {} remaining",
        m.summary_line(),
        m.games_played,
        m.games_remaining,
    );
    if let Some(last) = &m.last_result {
        println!(
            "Last game ({}): {} {}-{} {}{}",
            last.date,
            last.winner.0,
            last.winner_score,
            last.loser_score,
            other_team(&m.top_seed_abbrev, &m.bottom_seed_abbrev, &last.winner).0,
            if last.ot { " (OT)" } else { "" }
        );
    }
    if !m.series_complete {
        let next_at = if m.home_advantage {
            &m.top_seed_abbrev.0
        } else {
            &m.bottom_seed_abbrev.0
        };
        println!("Next game: G{} at {}", m.games_played + 1, next_at,);
    }
    let _ = season_id; // available for future season-aware rendering
}

fn other_team<'a>(
    top: &'a icelines_core::model::TeamAbbr,
    bottom: &'a icelines_core::model::TeamAbbr,
    winner: &icelines_core::model::TeamAbbr,
) -> &'a icelines_core::model::TeamAbbr {
    if winner.0 == top.0 {
        bottom
    } else {
        top
    }
}

fn emit_momentum_json(
    season_id: &str,
    m: &icelines_core::series_momentum::SeriesMomentum,
) -> Result<()> {
    let env = serde_json::json!({
        "schema_version": 1,
        "route": "playoffs.series",
        "data": m,
        "meta": {
            "season_id": season_id,
        },
    });
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

fn emit_json(bundle: &PlayoffsBundle, rows: &[PlayoffRow], round_filter: Option<u8>) -> Result<()> {
    // Post-LP review fix #5 — envelope shape matches King.2.4 web
    // convention `{schema_version, route, data, meta}`. Bundle context
    // (season, champion, Conn Smythe, round filter) lives under `meta`.
    #[derive(serde::Serialize)]
    struct Meta<'a> {
        season: &'a str,
        champion: Option<&'a str>,
        conn_smythe: Option<&'a str>,
        round_filter: Option<u8>,
        count: usize,
    }
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        schema_version: u32,
        route: &'static str,
        data: &'a [PlayoffRow],
        meta: Meta<'a>,
    }
    let env = Envelope {
        schema_version: 1,
        route: "playoffs",
        data: rows,
        meta: Meta {
            season: &bundle.season,
            champion: bundle.champion.as_deref(),
            conn_smythe: bundle.conn_smythe.as_deref(),
            round_filter,
            count: rows.len(),
        },
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
