use crate::config::{live_feeds_enabled, Config};
use icelines_fetch::date_range::{self, DateRangeQuery};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    cfg: &Config,
    query: DateRangeQuery,
    filter: &icelines_core::date_range::ScoringFilter,
    refresh: bool,
    json: bool,
    envelope: bool,
    csv: bool,
    top: Option<usize>,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    filter.validate().map_err(anyhow::Error::msg)?;
    query.validate()?;
    let roster = if filter.roster_scope != "all" {
        if refresh {
            anyhow::ensure!(
                live_feeds_enabled(),
                "live feeds are disabled; omit --refresh to query cached records"
            );
            Some(date_range::refresh_current_roster(&cfg.snapshot_dir(), &query.team).await?)
        } else {
            Some(date_range::load_current_roster(
                &cfg.snapshot_dir(),
                &query.team,
            )?)
        }
    } else {
        None
    };
    let capture = if refresh {
        anyhow::ensure!(
            live_feeds_enabled(),
            "live feeds are disabled; omit --refresh to query cached records"
        );
        date_range::refresh(&cfg.snapshot_dir(), &query).await?
    } else {
        date_range::load(&cfg.snapshot_dir(), &query)?
    };
    let rows = icelines_core::date_range::scoring_rows(
        &capture.games,
        &query.team,
        query.date_from,
        query.date_to,
        query.game_type,
        &filter.sort,
    )
    .map_err(anyhow::Error::msg)?;
    let mut rows = icelines_core::date_range::select_scoring_rows(
        rows,
        filter,
        roster.as_ref().map(|r| &r.player_ids),
    )
    .map_err(anyhow::Error::msg)?;
    let total = rows.len();
    if let Some(top) = top {
        rows.truncate(top);
    }
    let roster_meta=roster.as_ref().map(|r|serde_json::json!({"team":r.team,"fetched_at":r.fetched_at,"url":r.source.url,"sha256":r.source.sha256}));
    let rendered = if json || envelope {
        let value = serde_json::json!({
            "schema_version":1,"route":"leaders","data":rows,
            "meta":{"query":query,"sort":filter.sort,"order":filter.order,"sort_chain":filter.sort_chain().map_err(anyhow::Error::msg)?,"filters":filter,"current_roster":roster_meta,"fetched_at":capture.fetched_at,
            "total":total,"top":top,"completed_games":capture.game_ids.len(),"latest_game":capture.games.iter().map(|g|g.game_date).max(),
            "methodology":date_range::METHOD,"capture_path":query.path(&cfg.snapshot_dir()),
            "sources":capture.sources.iter().map(|s|serde_json::json!({"url":s.url,"sha256":s.sha256})).collect::<Vec<_>>()}
        });
        serde_json::to_string_pretty(if envelope { &value } else { &value["data"] })?
    } else {
        let table: Vec<_> = rows
            .iter()
            .map(|r| {
                vec![
                    r.points_rank.to_string(),
                    r.name.clone(),
                    r.position.clone(),
                    r.games.to_string(),
                    r.goals.to_string(),
                    r.assists.to_string(),
                    r.points.to_string(),
                    r.ev_points.to_string(),
                    r.ev_points_rank.to_string(),
                    format!("{:.3}", r.points_per_game),
                    format!("{:.3}", r.ev_points_per_game),
                    r.pp_points.to_string(),
                    r.sh_points.to_string(),
                ]
            })
            .collect();
        let table = super::output::Format::resolve(csv, false)?.render(
            &[
                "P rank", "Player", "Pos", "GP", "G", "A", "P", "EV P", "EV rank", "P/G", "EVP/G",
                "PP P", "SH P",
            ],
            &table,
        );
        if csv {
            table
        } else {
            format!(
                "{} · {} through {} · {} completed games · {} · position={} · roster={}\n{}\n{}\nCurrent-roster evidence: {}",
                query.team,
                query.date_from,
                query.date_to,
                capture.game_ids.len(),
                capture.fetched_at,
                if filter.position.is_empty() { "all" } else { &filter.position },
                filter.roster_scope,
                table,
                date_range::METHOD,
                roster.as_ref().map(|r|r.fetched_at.as_str()).unwrap_or("not required for all-player queries")
            )
        }
    };
    if let Some(path) = out {
        std::fs::write(path, format!("{rendered}\n"))?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}
