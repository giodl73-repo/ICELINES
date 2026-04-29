use crate::config::Config;
use icelines_core::{filter::PlayerFilter, model::Player, position::PositionResolver};
use icelines_fetch::{snapshot::SnapshotStore, PlayerRepository, BUNDLED_SEASONS};

/// Load all skaters via PlayerRepository (snapshot → bundled fallback).
/// This is the single entry point for player data across all commands.
pub fn load_all_players() -> anyhow::Result<Vec<Player>> {
    load_all_players_for_season(None)
}

/// Load all skaters for a specific bundled season, or use the configured /
/// current season when `season` is None. Validates the season string against
/// the bundled-seasons list and produces a clear error listing valid seasons
/// when the request can't be served.
///
/// Phase 8f cherry-pick: backs the `--season` flag on query commands.
pub fn load_all_players_for_season(season: Option<&str>) -> anyhow::Result<Vec<Player>> {
    let cfg = Config::load()?;
    let resolved_season = match season {
        Some(s) => {
            validate_bundled_season(s)?;
            s.to_owned()
        }
        None => cfg.season_str(),
    };
    let repo = PlayerRepository::new(
        SnapshotStore::new(cfg.snapshot_dir()),
        resolved_season,
    );
    repo.load_all()
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))
}

/// Reject `--season` values that aren't in the bundled list. Empty string and
/// non-eight-digit strings are also rejected up front so the error message is
/// useful (rather than the deeper "no bios for season" path).
pub fn validate_bundled_season(season: &str) -> anyhow::Result<()> {
    if BUNDLED_SEASONS.contains(&season) {
        return Ok(());
    }
    let bundled = BUNDLED_SEASONS.join(", ");
    anyhow::bail!(
        "season '{season}' is not bundled.\n  Bundled seasons: {bundled}\n  \
         (use one of these, or fetch live data with `icelines fetch all`)"
    )
}

pub struct PlayersArgs {
    pub pos: Option<String>, pub team: Option<String>,
    pub age_max: Option<u8>, pub age_min: Option<u8>,
    pub nationality: Option<String>, pub draft_year: Option<u16>,
    pub draft_round: Option<u8>, pub ppg_min: Option<f64>,
    pub gp_min: Option<u32>, pub top: usize,
    #[allow(dead_code)] pub json: bool,
}

pub async fn run(args: PlayersArgs) -> anyhow::Result<()> {
    let players = load_all_players()?;
    let mut filter = PlayerFilter::new();
    if let Some(p) = args.pos {
        if let Ok((primary, _)) = PositionResolver::parse(&p) {
            filter.positions = Some(vec![primary]);
        }
    }
    if let Some(t) = args.team     { filter.teams        = Some(vec![t.to_uppercase()]); }
    filter.age_max       = args.age_max;
    filter.age_min       = args.age_min;
    filter.nationalities = args.nationality.map(|n| vec![n.to_uppercase()]);
    filter.draft_years   = args.draft_year.map(|y| vec![y]);
    filter.draft_rounds  = args.draft_round.map(|r| vec![r]);
    filter.ppg_min       = args.ppg_min;
    filter.gp_min        = args.gp_min;

    let matched = filter.apply(&players);
    println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<7} {:<8}",
        "Rank","Player","Team","Pos","Age","PPG","Proj/82");
    println!("{}", "─".repeat(62usize));
    for (i, p) in matched.iter().take(args.top).enumerate() {
        let age = p.birth_date.as_deref()
            .and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
            .map(|y| (2026u16.saturating_sub(y)).to_string())
            .unwrap_or_else(|| "—".to_owned());
        let (ppg, proj) = match p.pace_score {
            Some(s) => (format!("{:.2}", s.pace_82/82.0), format!("{:.0}", s.pace_82)),
            None    => ("—".to_owned(), "—".to_owned()),
        };
        println!("{:<4} {:<24} {:<5} {:<4} {:<4} {:<7} {:<8}",
            i+1, p.full_name, p.team.as_str(), p.position.abbreviation(), age, ppg, proj);
    }
    println!("\n{} matched, showing {}.", matched.len(), matched.len().min(args.top));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase 8f: --season validator ───────────────────────────────────────

    #[test]
    fn l0_validate_bundled_season_accepts_each_bundled_id() {
        for s in BUNDLED_SEASONS {
            assert!(validate_bundled_season(s).is_ok(),
                "bundled season {s} should validate");
        }
    }

    #[test]
    fn l0_validate_bundled_season_rejects_unbundled() {
        let err = validate_bundled_season("19951996").unwrap_err().to_string();
        assert!(err.contains("not bundled"),
            "must say 'not bundled', got: {err}");
        assert!(err.contains("Bundled seasons:"),
            "must list bundled options, got: {err}");
        // Each bundled season should appear in the hint so the user can copy.
        for s in BUNDLED_SEASONS {
            assert!(err.contains(s), "expected {s} in hint, got: {err}");
        }
    }

    #[test]
    fn l0_validate_bundled_season_rejects_garbage() {
        assert!(validate_bundled_season("").is_err());
        assert!(validate_bundled_season("hi").is_err());
        assert!(validate_bundled_season("2025").is_err());
    }
}
