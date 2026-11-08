//! Thin adapter over the shared Rinkcraft statistical evidence view.
use anyhow::{bail, Context};
use icelines_core::view_model::rinkcraft_profile::{
    build_rinkcraft_stat_profile, ProfileMetricStatus, RinkcraftProfileRequest,
};
use icelines_core::{
    identity::PlayerId, model::Season, name::normalize_name, season_stats::SeasonType,
};

pub async fn run(
    player: String,
    first: Option<String>,
    last: Option<String>,
    season_type: SeasonType,
    json: bool,
) -> anyhow::Result<()> {
    let parse_season = |s: String| -> anyhow::Result<Season> {
        let n = s
            .parse::<u32>()
            .with_context(|| format!("invalid season {s:?}; use YYYYZZZZ"))?;
        Ok(Season::try_new(n)?)
    };
    let first_season = first.map(parse_season).transpose()?;
    let last_season = last.map(parse_season).transpose()?;
    if first_season.zip(last_season).is_some_and(|(a, b)| a > b) {
        bail!("first season must not be later than last season");
    }
    // Identity bootstrap must not require a current playoff bundle. Historical
    // postseason profiles can exist when the newest season has no playoff data.
    let (mut loaded, active, _) =
        super::players::load_repo_for_season(None, Some(SeasonType::Regular))?;
    let numeric = player.parse::<u32>().ok();
    let pid = match numeric {
        Some(id) => PlayerId::try_new(id)?,
        None => {
            let normalized = normalize_name(&player);
            let mut candidates: Vec<_> = loaded
                .repo
                .skaters(active, SeasonType::Regular)
                .chain(loaded.repo.goalies(active, SeasonType::Regular))
                .filter(|v| v.name_normalized() == normalized)
                .map(|v| v.id())
                .collect();
            candidates.extend(
                icelines_fetch::stats_loader::find_player_candidates(&player)
                    .into_iter()
                    .filter(|c| normalize_name(&c.full_name) == normalized)
                    .map(|c| PlayerId(c.pid)),
            );
            candidates.sort();
            candidates.dedup();
            if candidates.len() > 1 {
                bail!("multiple players share this name; use the numeric NHL player id");
            }
            candidates
                .first()
                .copied()
                .with_context(|| format!("player {player:?} not found in loaded/bundled data"))?
        }
    };
    let cfg = crate::config::Config::load()?;
    let store = icelines_fetch::snapshot::SnapshotStore::new(cfg.snapshot_dir());
    icelines_fetch::stats_loader::load_player_career_into_repo_with_store(
        &mut loaded.repo,
        pid,
        &store,
    )?;
    let identity = loaded
        .repo
        .identity(pid)
        .with_context(|| format!("player {pid} has no loaded identity"))?;
    if numeric.is_none() && normalize_name(&identity.full_name) != normalize_name(&player) {
        bail!(
            "use a full player name or numeric NHL player id; partial name resolved to {:?}",
            identity.full_name
        );
    }
    let source_windows = icelines_fetch::rinkcraft_profile::hydrate_profile_reports(
        &mut loaded.repo,
        pid,
        season_type,
        first_season,
        last_season,
        &store,
    )?;
    let mut profile = build_rinkcraft_stat_profile(
        &loaded.repo,
        RinkcraftProfileRequest {
            player_id: pid,
            season_type,
            first_season,
            last_season,
        },
    )?;
    profile.source_windows = source_windows;
    profile.limitations.extend(
        loaded
            .missing
            .iter()
            .map(|m| format!("Regular-season identity/bootstrap loader: {m:?}")),
    );
    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
        return Ok(());
    }
    println!(
        "{} — statistical evidence profile ({})",
        profile.player_name, season_type
    );
    println!(
        "Scope: {}; {} loaded seasons",
        profile.scope,
        profile.timeline.len()
    );
    if let Some(latest) = &profile.latest_available {
        println!(
            "Latest available in scope: {} — {} games",
            latest.season, latest.games
        );
        for m in latest
            .metrics
            .iter()
            .filter(|m| m.status == ProfileMetricStatus::Available)
        {
            println!("  {}: {:.4}", m.key, m.value.unwrap());
        }
    } else {
        println!("No player rows in this scope.");
    }
    println!("\nSelected-window summary:");
    for m in profile
        .summary
        .iter()
        .filter(|m| m.status == ProfileMetricStatus::Available)
    {
        println!("  {}: {:.4} [{}]", m.key, m.value.unwrap(), m.aggregation);
    }
    println!("\nUse --json for season history, coverage, arithmetic changes and job-family evidence links.");
    for limitation in &profile.limitations {
        println!("Note: {limitation}");
    }
    Ok(())
}
