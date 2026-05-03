use crate::config::Config;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{filter::PlayerFilter, position::PositionResolver};
use icelines_fetch::{
    snapshot::SnapshotStore,
    stats_loader::{load_into_repo, LoadOutcome},
    BUNDLED_SEASONS,
};

/// Hart.5c.3: load a `LoadOutcome` for the requested season (or the
/// configured / current season when `season` is None). Hart.6.9 added
/// the `season_type` parameter so playoff loads ride the same path.
/// `load_repo_for_season(s, None)` is a regular-season shortcut for
/// existing callers; pass `Some(SeasonType::Playoff)` for the playoff
/// window.
pub fn load_repo_for_season(
    season: Option<&str>,
    season_type: Option<SeasonType>,
) -> anyhow::Result<(LoadOutcome, Season, SeasonType)> {
    let cfg = Config::load()?;
    let resolved_season = match season {
        Some(s) => {
            validate_bundled_season(s)?;
            s.to_owned()
        }
        None => cfg.season_str(),
    };
    let season_u32: u32 = resolved_season
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{resolved_season}' is not a YYYYZZZZ id"))?;
    let season = Season(season_u32);
    let ty = season_type.unwrap_or(SeasonType::Regular);
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(season, ty, &store).map_err(|e| {
        let hint = match ty {
            SeasonType::Regular => "Try: icelines fetch all",
            SeasonType::Playoff => "Try: icelines fetch stats --type playoff",
        };
        anyhow::anyhow!("{e}\n  {hint}")
    })?;
    Ok((outcome, season, ty))
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
    pub pos: Option<String>,
    pub team: Option<String>,
    pub age_max: Option<u8>,
    pub age_min: Option<u8>,
    pub nationality: Option<String>,
    pub draft_year: Option<u16>,
    pub draft_round: Option<u8>,
    pub ppg_min: Option<f64>,
    pub gp_min: Option<u32>,
    pub top: usize,
    pub json: bool,
    pub csv: bool,
    pub out: Option<std::path::PathBuf>,
}

pub async fn run(args: PlayersArgs) -> anyhow::Result<()> {
    // Hart.5b2: refactored off Vec<Player> onto PlayerView via
    // load_into_repo + apply_views. Identity bio access (birth_date)
    // goes through view.identity.bio per the new model.
    let cfg = Config::load()?;
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{}' is not a YYYYZZZZ id", cfg.season_str()))?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(Season(season_u32), SeasonType::Regular, &store)
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))?;

    let mut filter = PlayerFilter::new();
    if let Some(p) = args.pos {
        if let Ok((primary, _)) = PositionResolver::parse(&p) {
            filter.positions = Some(vec![primary]);
        }
    }
    if let Some(t) = args.team {
        filter.teams = Some(vec![t.to_uppercase()]);
    }
    filter.age_max = args.age_max;
    filter.age_min = args.age_min;
    filter.nationalities = args.nationality.map(|n| vec![n.to_uppercase()]);
    filter.draft_years = args.draft_year.map(|y| vec![y]);
    filter.draft_rounds = args.draft_round.map(|r| vec![r]);
    filter.ppg_min = args.ppg_min;
    filter.gp_min = args.gp_min;

    let matched = filter.apply_views(
        outcome
            .repo
            .skaters(Season(season_u32), SeasonType::Regular),
    );
    let total = matched.len();
    let take = total.min(args.top);

    let headers = &["rank", "player", "team", "pos", "age", "ppg", "proj_82"];
    let rows: Vec<Vec<String>> = matched
        .iter()
        .take(args.top)
        .enumerate()
        .map(|(i, v)| {
            let age = v
                .identity
                .bio
                .birth_date
                .as_deref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<u16>().ok())
                .map(|y| (2026u16.saturating_sub(y)).to_string())
                .unwrap_or_else(|| "—".to_owned());
            let (ppg, proj) = match v.pace_score() {
                Some(s) => (
                    format!("{:.2}", s.pace_82 / 82.0),
                    format!("{:.0}", s.pace_82),
                ),
                None => ("—".to_owned(), "—".to_owned()),
            };
            vec![
                (i + 1).to_string(),
                v.full_name().to_owned(),
                v.team_display().to_owned(),
                v.position().abbreviation().to_owned(),
                age,
                ppg,
                proj,
            ]
        })
        .collect();

    let format = crate::commands::output::Format::resolve(args.csv, args.json)?;
    format.emit_to(headers, &rows, args.out.as_deref())?;
    if format == crate::commands::output::Format::Table && args.out.is_none() {
        println!("\n{total} matched, showing {take}.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase 8f: --season validator ───────────────────────────────────────

    #[test]
    fn l0_validate_bundled_season_accepts_each_bundled_id() {
        for s in BUNDLED_SEASONS {
            assert!(
                validate_bundled_season(s).is_ok(),
                "bundled season {s} should validate"
            );
        }
    }

    #[test]
    fn l0_validate_bundled_season_rejects_unbundled() {
        let err = validate_bundled_season("19951996").unwrap_err().to_string();
        assert!(
            err.contains("not bundled"),
            "must say 'not bundled', got: {err}"
        );
        assert!(
            err.contains("Bundled seasons:"),
            "must list bundled options, got: {err}"
        );
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
