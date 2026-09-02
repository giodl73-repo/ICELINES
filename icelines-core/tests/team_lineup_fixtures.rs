use std::collections::BTreeSet;
use std::path::PathBuf;

use icelines_core::{
    TeamLineupPlayerView, TeamLineupProjectionView, TEAM_LINEUP_PROJECTION_SCHEMA,
};

fn fixture(name: &str) -> TeamLineupProjectionView {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join(name);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|error| {
        panic!("read canonical lineup fixture {}: {error}", path.display())
    }))
    .unwrap_or_else(|error| panic!("parse canonical lineup fixture {}: {error}", path.display()))
}

fn players(view: &TeamLineupProjectionView) -> Vec<&TeamLineupPlayerView> {
    view.forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            view.defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .chain([&view.goalies.starter, &view.goalies.backup])
        .filter_map(Option::as_ref)
        .chain(view.extras.iter())
        .collect()
}

fn assert_fixture(view: &TeamLineupProjectionView, team: &str) {
    assert_eq!(view.schema, TEAM_LINEUP_PROJECTION_SCHEMA);
    assert_eq!(view.team, team);
    assert_eq!(view.forward_lines.len(), 4);
    assert_eq!(view.defense_pairs.len(), 3);
    let players = players(view);
    let ids = players
        .iter()
        .map(|player| player.player_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), players.len());
    assert!(players.iter().all(|player| player.team == team));
    assert!(players.iter().all(|player| {
        player
            .portrait
            .headshot_canonical_url
            .as_ref()
            .is_some_and(|url| url.contains(&player.player_id.to_string()))
    }));
    assert!(players
        .iter()
        .all(|player| player.score.display == "NR" || player.score.value.is_some()));
}

#[test]
fn alpha_lineup_keeps_complete_roster_and_authoritative_faces() {
    let view = fixture("team-lineup-alp-2026-27.json");
    assert_fixture(&view, "NYR");
    assert!(players(&view)
        .iter()
        .all(|player| player.display_name.starts_with("Sample Player ")));
}

#[test]
fn bravo_lineup_matches_reported_complete_shape() {
    let view = fixture("team-lineup-brv-2026-27.json");
    assert_fixture(&view, "SEA");
    assert!(!view
        .warnings
        .iter()
        .any(|warning| warning.code == "incomplete_roster_shape"));
    assert_eq!(
        view.forward_lines[0].center.as_ref().unwrap().display_name,
        "Sample Player 248"
    );
    assert_eq!(
        view.goalies.starter.as_ref().unwrap().display_name,
        "Sample Player 188"
    );
}
