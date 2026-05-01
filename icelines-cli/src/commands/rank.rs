use crate::commands::output::Format;
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

    // Hart.5c.7: single load + view-based path for both Table and
    // JSON/CSV outputs. The legacy load_all_players() / &[Player]
    // bridge is gone.
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

    if format == Format::Table && out.is_none() {
        let view_refs: Vec<&_> = views.iter().collect();
        render_rank_table(&view_refs, top, false);
        return Ok(());
    }

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
