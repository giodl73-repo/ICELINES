use crate::config::Config;
use crate::render::terminal::render_rank_table;
use icelines_core::{position::PositionResolver, Position};
use icelines_fetch::{snapshot::SnapshotStore, PlayerRepository};

pub async fn run(top: usize, pos: Option<String>, _scheme: Option<String>) -> anyhow::Result<()> {
    let cfg  = Config::load()?;
    let repo = PlayerRepository::new(
        SnapshotStore::new(cfg.snapshot_dir()),
        cfg.season_str(),
    );

    let pos_filter: Option<Position> = pos.as_deref()
        .and_then(|p| PositionResolver::parse(p).ok().map(|(primary, _)| primary));

    let mut all_players = repo.load_all()
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))?;

    if let Some(p) = pos_filter {
        all_players.retain(|pl| pl.position == p);
    }
    all_players.retain(|p| p.is_rankable());

    render_rank_table(&all_players, top, pos_filter, false);
    Ok(())
}
