//! Refresh selected career rows through the canonical report-aware loader.
use crate::{
    snapshot::SnapshotStore,
    stats_loader::{load_into_repo, LoadError},
};
use icelines_core::view_model::rinkcraft_profile::ProfileSourceWindow;
use icelines_core::{
    identity::PlayerId, model::Season, season_stats::SeasonType, stats_repository::StatsRepository,
};

/// Reference populations come from the same canonical local season loader.
pub fn attach_attribute_seasons(
    profile: &mut icelines_core::view_model::rinkcraft_profile::RinkcraftStatProfile,
    store: &SnapshotStore,
) -> Result<(), LoadError> {
    let mut assessments = Vec::new();
    for season in &profile.included_seasons {
        let loaded = load_into_repo(*season, profile.request.season_type, store)?;
        if let Some(assessment) =
            icelines_core::view_model::rinkcraft_attributes::build_attribute_season(
                profile.request.player_id,
                *season,
                profile.request.season_type,
                loaded
                    .repo
                    .read_league(*season, profile.request.season_type)
                    .collect(),
            )
        {
            assessments.push(assessment);
        }
    }
    profile.attribute_seasons = assessments;
    Ok(())
}

/// Shared optional source for the native CLI and HTML profile entry points.
pub fn attach_configured_stat_attributes(
    profile: &mut icelines_core::view_model::rinkcraft_profile::RinkcraftStatProfile,
) {
    if let Some(path) = std::env::var_os("ICELINES_RINKCRAFT_STAT_ATTRIBUTES") {
        if let Err(cause) = attach_stat_attribute_catalog(profile, std::path::Path::new(&path)) {
            profile.stat_attribute_seasons.clear();
            profile.stat_attribute_changes.clear();
            profile
                .limitations
                .push(format!("Stat estimates unavailable: {cause}"));
        }
    }
}

pub fn attach_stat_attribute_catalog(
    profile: &mut icelines_core::view_model::rinkcraft_profile::RinkcraftStatProfile,
    path: &std::path::Path,
) -> Result<(), String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
        return Err("Stat attribute catalog exceeds 32 MiB".into());
    }
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let catalog = icelines_core::view_model::rinkcraft_stat_scores::Catalog::from_json(&source)?;
    icelines_core::view_model::rinkcraft_profile::attach_stat_attribute_catalog(profile, &catalog)
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileHydrationError {
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error(transparent)]
    Repository(#[from] icelines_core::stats_repository::RepoError),
    #[error("player {player} is absent from the canonical {season} {season_type} window; refusing stale career-row fallback")]
    MissingPlayer {
        player: PlayerId,
        season: Season,
        season_type: SeasonType,
    },
}

/// Career summary fan-out is lean. Rehydrate selected player rows through the
/// canonical loader before profiling. No fetch/network or durable writes occur.
pub fn hydrate_profile_reports(
    repo: &mut StatsRepository,
    pid: PlayerId,
    ty: SeasonType,
    first: Option<Season>,
    last: Option<Season>,
    store: &SnapshotStore,
) -> Result<Vec<ProfileSourceWindow>, ProfileHydrationError> {
    let mut seasons: Vec<_> = repo
        .career_all(pid)
        .into_iter()
        .flatten()
        .filter(|s| {
            s.season_type == ty
                && first.is_none_or(|f| s.season >= f)
                && last.is_none_or(|l| s.season <= l)
        })
        .map(|s| s.season)
        .collect();
    seasons.sort();
    seasons.dedup();
    let mut disclosure = Vec::new();
    for season in seasons {
        let loaded = load_into_repo(season, ty, store)?;
        let row =
            loaded
                .repo
                .season(pid, season, ty)
                .ok_or(ProfileHydrationError::MissingPlayer {
                    player: pid,
                    season,
                    season_type: ty,
                })?;
        repo.upsert_stats(row.clone())?;
        disclosure.push(ProfileSourceWindow {
            season,
            season_type: ty,
            missing_reports: loaded.missing.iter().map(|m| format!("{m:?}")).collect(),
        });
    }
    Ok(disclosure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundled;
    use icelines_core::{model::Position, stats_catalog::StatId};

    #[test]
    fn rinkcraft_profile_rejects_stale_player_row_when_canonical_window_has_no_player() {
        let season = Season(bundled::BUNDLED_SEASONS[0].parse().unwrap());
        let id = PlayerId(9_000_001);
        let mut repo = StatsRepository::with_lru_cap(80);
        repo.upsert_identity(
            icelines_core::fixtures::identity(id.0)
                .name("Synthetic Skater", "synthetic skater")
                .build(),
        )
        .unwrap();
        repo.upsert_stats(icelines_core::fixtures::stats(id.0, season.0, "AAA").build())
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path());
        assert!(matches!(
            hydrate_profile_reports(&mut repo, id, SeasonType::Regular, None, None, &store),
            Err(ProfileHydrationError::MissingPlayer { .. })
        ));
    }

    #[test]
    fn rinkcraft_profile_career_fanout_includes_goalies() {
        let season = bundled::BUNDLED_SEASONS[0];
        let pid = PlayerId(bundled::get_goalie_stats(season).unwrap()[0].player_id);
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path());
        let mut repo = StatsRepository::with_lru_cap(80);
        assert!(
            crate::stats_loader::load_player_career_into_repo_with_store(&mut repo, pid, &store)
                .unwrap()
                > 0
        );
        assert!(repo.identity(pid).is_some());
        let rows: Vec<_> = repo.career_all(pid).unwrap().collect();
        assert!(!rows.is_empty());
        assert!(rows
            .iter()
            .all(|r| r.goalie.is_some() && r.position == Position::Goalie));
    }

    #[test]
    fn rinkcraft_profile_realtime_from_another_season_is_not_reused() {
        use crate::snapshot::SnapshotTier;
        let requested = bundled::BUNDLED_SEASONS[1];
        let other = bundled::BUNDLED_SEASONS[0];
        // Existing bundle identity exercises the canonical join, not team logic.
        let pid = bundled::get_bios(requested)
            .unwrap()
            .iter()
            .find(|b| {
                Position::from_api_code(&b.position_code).is_some_and(|p| p != Position::Goalie)
            })
            .unwrap()
            .player_id;
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path());
        store
            .create("other", other, SnapshotTier::Realtime, None, "2000-01-01")
            .unwrap();
        store
            .write_file(
                "other",
                &SnapshotTier::Realtime,
                "realtime.json",
                &serde_json::to_vec(&serde_json::json!([{
                    "playerId":pid,"hits":999,"blockedShots":999,"takeaways":999,"giveaways":999
                }]))
                .unwrap(),
            )
            .unwrap();
        store.seal("other").unwrap();
        store.set_active("other").unwrap();
        let loaded = load_into_repo(
            Season(requested.parse().unwrap()),
            SeasonType::Regular,
            &store,
        )
        .unwrap();
        assert_eq!(
            StatId::Hits.read(
                &loaded
                    .repo
                    .view(
                        PlayerId(pid),
                        Season(requested.parse().unwrap()),
                        SeasonType::Regular
                    )
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn rinkcraft_profile_rehydrates_canonical_optional_reports() {
        // This loader contract requires one existing bundled identity. No personal
        // team assumption: select any skater and keep the report overlay minimal.
        let season_text = bundled::BUNDLED_SEASONS[0];
        let season = Season(season_text.parse().unwrap());
        let id = PlayerId(
            bundled::get_bios(season_text)
                .unwrap()
                .iter()
                .find(|b| {
                    Position::from_api_code(&b.position_code).is_some_and(|p| p != Position::Goalie)
                })
                .unwrap()
                .player_id,
        );
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path());
        let mut repo = load_into_repo(season, SeasonType::Regular, &store)
            .unwrap()
            .repo;
        assert!(repo
            .season(id, season, SeasonType::Regular)
            .unwrap()
            .time_on_ice
            .is_none());
        let report_dir = dir.path().join(season_text).join("regular");
        std::fs::create_dir_all(&report_dir).unwrap();
        std::fs::write(report_dir.join("timeonice.json"),serde_json::to_vec(&serde_json::json!({"data":[{
            "playerId":id.0,"seasonId":season.0,"timeOnIce":1200,"timeOnIcePerGame":600.0,
            "evTimeOnIce":1000,"evTimeOnIcePerGame":500.0,"ppTimeOnIce":200,"ppTimeOnIcePerGame":100.0,
            "shTimeOnIce":0,"shTimeOnIcePerGame":0.0,"otTimeOnIce":0,"shifts":20,
            "shiftsPerGame":10.0,"timeOnIcePerShift":60.0
        }],"total":1})).unwrap()).unwrap();
        let sources = hydrate_profile_reports(
            &mut repo,
            id,
            SeasonType::Regular,
            Some(season),
            Some(season),
            &store,
        )
        .unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(
            StatId::TotalToi.read(&repo.view(id, season, SeasonType::Regular).unwrap()),
            Some(1200.0)
        );
    }
}
