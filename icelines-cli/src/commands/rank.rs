use crate::commands::output::Format;
use crate::commands::players::load_all_players;
use crate::config::Config;
use crate::render::terminal::render_rank_table;
use icelines_core::model::Season;
use icelines_core::scoring::sort_views_by_pace;
use icelines_core::season_stats::SeasonType;
use icelines_core::{position::PositionResolver, Position};
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::load_into_repo;

pub async fn run(
    top: usize,
    pos: Option<String>,
    _scheme: Option<String>,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let pos_filter: Option<Position> = pos
        .as_deref()
        .and_then(|p| PositionResolver::parse(p).ok().map(|(primary, _)| primary));

    let format = Format::resolve(csv, json)?;

    // Hart.5b2 (rank.rs proof-of-concept): table render uses the
    // legacy Player-shaped path via `load_all_players()` because the
    // `render_rank_table` helper still consumes `&[Player]`. Non-table
    // export goes through the new PlayerView path directly — proves
    // the per-consumer mechanical-refactor pattern works.
    if format == Format::Table && out.is_none() {
        let mut all_players = load_all_players()?;
        if let Some(p) = pos_filter {
            all_players.retain(|pl| pl.position == p);
        }
        all_players.retain(|p| p.is_rankable());
        render_rank_table(&all_players, top, pos_filter, false);
        return Ok(());
    }

    // PlayerView path — load via the new repo, iterate views, sort,
    // render. Hart.5b2 demonstrates the pattern future consumer
    // refactors will follow.
    let cfg = Config::load()?;
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{}' is not a YYYYZZZZ id", cfg.season_str()))?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(Season(season_u32), SeasonType::Regular, &store)
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))?;

    let mut views: Vec<_> = outcome
        .repo
        .skaters(Season(season_u32), SeasonType::Regular)
        .filter(|v| pos_filter.is_none_or(|p| v.position() == p))
        .filter(|v| v.is_rankable())
        .collect();
    sort_views_by_pace(&mut views);

    let headers = &[
        "rank",
        "player",
        "team",
        "pos",
        "gp",
        "ppg",
        "pts_per_82",
        "goals_per_82",
    ];
    let rows: Vec<Vec<String>> = views
        .iter()
        .take(top)
        .enumerate()
        .map(|(i, v)| {
            vec![
                (i + 1).to_string(),
                v.full_name().to_owned(),
                v.team_display().to_owned(),
                v.position().abbreviation().to_owned(),
                v.gp().to_string(),
                v.pace_score()
                    .map(|s| format!("{:.3}", s.pace_82 / 82.0))
                    .unwrap_or_else(|| "—".to_owned()),
                v.pace_82()
                    .map(|p| format!("{p:.1}"))
                    .unwrap_or_else(|| "—".to_owned()),
                v.goals_per_82()
                    .map(|g| format!("{g:.1}"))
                    .unwrap_or_else(|| "—".to_owned()),
            ]
        })
        .collect();

    format.emit_to(headers, &rows, out.as_deref())?;
    Ok(())
}
