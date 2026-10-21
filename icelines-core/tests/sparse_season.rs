#[allow(dead_code)]
#[path = "support/season_read_cases.rs"]
mod cases;
use icelines_core::{
    fixtures, identity::PlayerId, model::Season, season_stats::*,
    sparse_season::SparseSeasonRepository, stats_catalog::StatId, SeasonStatsRead,
};
use std::collections::BTreeMap;
type Key = (PlayerId, Season, SeasonType);
fn key(row: &SeasonStats) -> Key {
    (row.player_id, row.season, row.season_type)
}
fn owned(read: &impl SeasonStatsRead) -> SeasonStats {
    SeasonStats {
        player_id: read.player_id(),
        season: read.season(),
        season_type: read.season_type(),
        position: read.position(),
        sweater_number: read.sweater_number(),
        totals: read.totals().clone(),
        team_stints: read.team_stints().to_vec(),
        realtime: read.realtime().cloned(),
        advanced: read.advanced().cloned(),
        goalie: read.goalie().cloned(),
        time_on_ice: read.time_on_ice().cloned(),
        goals_for_against: read.goals_for_against().cloned(),
        goalie_advanced: read.goalie_advanced().cloned(),
        goalie_saves_by_strength: read.goalie_saves_by_strength().cloned(),
        goalie_bios: read.goalie_bios().cloned(),
    }
}
#[test]
fn production_sparse_reads_preserve_all_fields_and_original_catalog_bits() {
    let input = cases::cases();
    let mut store = SparseSeasonRepository::new();
    for (identity, row) in &input {
        store.upsert_identity(identity.clone()).unwrap();
        store.upsert_stats(row.clone()).unwrap();
    }
    store.validate().unwrap();
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/season-read-legacy.json")).unwrap();
    for (j, (_, row)) in input.iter().enumerate() {
        let view = store
            .view(row.player_id, row.season, row.season_type)
            .unwrap();
        assert_eq!(owned(&view.stats), *row);
        assert_eq!(
            serde_json::to_string(&view.stats).unwrap(),
            serde_json::to_string(row).unwrap()
        );
        assert_eq!(
            serde_json::to_value(owned(&view.stats)).unwrap(),
            serde_json::to_value(row).unwrap()
        );
        for (i, stat) in StatId::all().iter().enumerate() {
            assert_eq!(
                stat.read_from(&view).map(f64::to_bits),
                golden["stats"][i]["value_bits"][j].as_u64(),
                "{} row {j}",
                stat.cli_key()
            );
        }
    }
    for (i, stat) in StatId::all().iter().enumerate() {
        let mut views: Vec<_> = input
            .iter()
            .map(|(_, r)| store.view(r.player_id, r.season, r.season_type).unwrap())
            .collect();
        views.sort_by(|a, b| stat.sort_read_cmp(a, b));
        let ids: Vec<_> = views.iter().map(|v| v.id().0).collect();
        assert_eq!(
            serde_json::to_value(ids).unwrap(),
            golden["stats"][i]["sorted_ids"],
            "{} ordering",
            stat.cli_key()
        );
    }
}
#[test]
fn refresh_remove_and_payload_relocation_preserve_reference_rows() {
    let input = cases::cases();
    let mut store = SparseSeasonRepository::new();
    let mut expected = BTreeMap::<Key, SeasonStats>::new();
    for id in 1..=40 {
        store
            .upsert_identity(fixtures::identity(id).build())
            .unwrap();
    }
    for i in 0..1200 {
        let mut row = input[(i * 17) % input.len()].1.clone();
        row.player_id = PlayerId((i % 40 + 1) as u32);
        let k = key(&row);
        if i % 7 == 0 {
            assert_eq!(store.remove(k.0, k.1, k.2), expected.remove(&k).is_some());
        } else {
            store.upsert_stats(row.clone()).unwrap();
            expected.insert(k, row);
        }
        store.validate().unwrap();
        assert_eq!(store.stats_len(), expected.len());
        for (k, row) in &expected {
            assert_eq!(owned(&store.view(k.0, k.1, k.2).unwrap().stats), *row);
        }
    }
    let keys: Vec<_> = expected.keys().copied().collect();
    for k in keys {
        assert!(store.remove(k.0, k.1, k.2));
        assert!(store.view(k.0, k.1, k.2).is_none());
        store.validate().unwrap();
    }
    assert_eq!(store.stats_len(), 0);
}
#[test]
fn existing_player_view_default_keeps_concrete_reference_and_copy_contract() {
    fn require_copy<T: Copy>(_: T) {}
    let mut row = fixtures::stats(1, 20252026, "EDM").build();
    let identity = fixtures::identity(1).build();
    let view: icelines_core::stats_repository::PlayerView<'_> =
        icelines_core::stats_repository::PlayerView {
            identity: &identity,
            stats: &row,
            contract: None,
        };
    let direct: &SeasonStats = view.stats;
    assert!(std::ptr::eq(direct, &row));
    require_copy(view);
    assert_eq!(view.gp(), row.totals.gp);
    let mutable_view = icelines_core::stats_repository::PlayerView {
        identity: &identity,
        stats: &mut row,
        contract: None,
    };
    let _: &SeasonStats = mutable_view.stats;
    require_copy(mutable_view);
    let boxed = Box::new(row);
    let boxed_view = icelines_core::stats_repository::PlayerView {
        identity: &identity,
        stats: &boxed,
        contract: None,
    };
    let _: &SeasonStats = boxed_view.stats;
    require_copy(boxed_view);
    assert!(std::ptr::eq(boxed_view.stats, &*boxed));
}
