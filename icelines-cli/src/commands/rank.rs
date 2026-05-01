use crate::commands::output::Format;
use crate::commands::players::load_all_players;
use crate::render::terminal::render_rank_table;
use icelines_core::{position::PositionResolver, Position};

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

    // Hart.5a: routed through the centralized load boundary
    // (commands::players::load_all_players), which now uses
    // load_into_repo + flat_view_legacy internally.
    let mut all_players = load_all_players()?;

    if let Some(p) = pos_filter {
        all_players.retain(|pl| pl.position == p);
    }
    all_players.retain(|p| p.is_rankable());

    let format = Format::resolve(csv, json)?;

    if format == Format::Table && out.is_none() {
        render_rank_table(&all_players, top, pos_filter, false);
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
    let rows: Vec<Vec<String>> = all_players
        .iter()
        .take(top)
        .enumerate()
        .map(|(i, p)| {
            vec![
                (i + 1).to_string(),
                p.full_name.clone(),
                p.team.as_str().to_owned(),
                p.position.abbreviation().to_owned(),
                p.gp()
                    .map(|g| g.to_string())
                    .unwrap_or_else(|| "—".to_owned()),
                p.pace_score
                    .map(|s| format!("{:.3}", s.pace_82 / 82.0))
                    .unwrap_or_else(|| "—".to_owned()),
                p.pace_score
                    .map(|s| format!("{:.1}", s.pace_82))
                    .unwrap_or_else(|| "—".to_owned()),
                p.pace_score
                    .map(|s| format!("{:.1}", s.goals_per_82))
                    .unwrap_or_else(|| "—".to_owned()),
            ]
        })
        .collect();

    format.emit_to(headers, &rows, out.as_deref())?;
    Ok(())
}
