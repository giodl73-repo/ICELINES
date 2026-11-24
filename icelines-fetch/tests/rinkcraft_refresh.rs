use icelines_core::view_model::rinkcraft_stat_scores::{self as s, Catalog, Player};
use icelines_fetch::rinkcraft_stat_refresh::{refresh, Capture, Request};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Write},
    path::Path,
};
fn cache(root: &Path, url: &str, body: &[u8]) {
    fs::create_dir_all(root).unwrap();
    let key = format!("{:x}", Sha256::digest(url.as_bytes()));
    fs::write(root.join(format!("{key}.bin")), body).unwrap();
    fs::write(
        root.join(format!("{key}.json")),
        serde_json::to_vec(&Capture {
            url: url.into(),
            sha256: format!("{:x}", Sha256::digest(body)),
            captured_at: "2026-11-23T00:00:00Z".into(),
            status: 200,
        })
        .unwrap(),
    )
    .unwrap();
}
fn zip(name: &str, csv: &str) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    z.start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    z.write_all(csv.as_bytes()).unwrap();
    z.finish().unwrap().into_inner()
}
fn fixture(root: &Path, member: &str) {
    let url="https://api.nhle.com/stats/rest/en/goalie/summary?isAggregate=false&isGame=false&limit=100&start=0&sort=%5B%7B%22property%22%3A%22playerId%22%2C%22direction%22%3A%22ASC%22%7D%5D&cayenneExp=seasonId%3D20252026%20and%20gameTypeId%3D2";
    let rows=(1..=20).map(|id|json!({"playerId":id,"seasonId":20252026,"gamesPlayed":20,"timeOnIce":12000,"saves":1,"shotsAgainst":2,"assists":0,"goalieFullName":format!("Synthetic Keeper {id}"),"lastName":"Keeper","teamAbbrevs":"MTL","gamesStarted":20,"wins":10,"losses":10,"goalsAgainst":1,"shutouts":0,"goals":0,"points":0,"penaltyMinutes":0})).collect::<Vec<_>>();
    cache(
        root,
        url,
        &serde_json::to_vec(&json!({"total":20,"data":rows})).unwrap(),
    );
    let mut summary = "playerId,season,situation,games_played,ongoal\n".to_string();
    let mut shots="shotID,game_id,season,isPlayoffGame,period,goalieIdForShot,shotOnEmptyNet,shotWasOnGoal,goal,shotRebound,shotAngleReboundRoyalRoad,shotGeneratedRebound,xGoal,xRebound\n".to_string();
    for id in 1..=20 {
        summary.push_str(&format!("{id},2025,all,20,2\n"));
        shots.push_str(&format!("{},20001,2025,0,1,{id},0,1,0,0,1,1,0.02,0.1\n{},20001,2025,0,1,{id},0,1,1,1,1,0,0.3,0.1\n",id*2,id*2+1));
    }
    cache(
        root,
        "https://moneypuck.com/moneypuck/playerData/seasonSummary/2025/regular/goalies.csv",
        summary.as_bytes(),
    );
    cache(
        root,
        "https://peter-tanner.com/moneypuck/downloads/shots_2025.zip",
        &zip(member, &shots),
    );
}
fn request(root: &Path, name: &str) -> Request {
    Request {
        season: 20252026,
        kind: "goalies".into(),
        cache_dir: root.join("cache"),
        output_dir: root.join(name),
        previous_catalog: None,
        edge_captures: None,
        game_cache_dir: None,
        offline: true,
    }
}
#[tokio::test]
async fn refresh_is_replayable_preserves_other_windows_and_checks_cached_bytes() {
    let root = tempfile::tempdir().unwrap();
    fixture(&root.path().join("cache/20252026"), "shots_2025.csv");
    let old = s::build(
        20242025,
        "all strengths",
        &[Player {
            player_id: 999,
            group: "Defense".into(),
            games: 20,
            minutes: 200.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap();
    let previous = root.path().join("previous.json");
    fs::write(
        &previous,
        serde_json::to_vec(&Catalog {
            schema: "rinkcraft.stat-attribute-catalog.v1".into(),
            profiles: old,
        })
        .unwrap(),
    )
    .unwrap();
    let mut r = request(root.path(), "first");
    r.previous_catalog = Some(previous.clone());
    let report = refresh(r.clone()).await.unwrap();
    assert_eq!(report["complete_profiles"], 20);
    let first = fs::read(r.output_dir.join("rinkcraft-stat-attributes.json")).unwrap();
    let package = fs::read(r.output_dir.join("season-package.json")).unwrap();
    let mut engine = icelines_wasm::BrowserEngine::new();
    engine.load_bytes(&package).unwrap();
    let c = Catalog::from_json(std::str::from_utf8(&first).unwrap()).unwrap();
    assert_eq!(c.profiles.len(), 21);
    assert_eq!(c.profiles[1].group, "Goalies");
    assert!(c.profiles[1].scores.iter().all(|s| s.score == Some(10)));
    assert!(refresh(r).await.unwrap_err().contains("new output"));
    let mut replay = request(root.path(), "replay");
    replay.previous_catalog = Some(previous);
    refresh(replay.clone()).await.unwrap();
    assert_eq!(
        first,
        fs::read(replay.output_dir.join("rinkcraft-stat-attributes.json")).unwrap()
    );
    let raw = fs::read_dir(root.path().join("cache/20252026"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().unwrap() == "bin")
        .unwrap();
    fs::write(raw, b"corrupt").unwrap();
    assert!(refresh(request(root.path(), "bad"))
        .await
        .unwrap_err()
        .contains("checksum"));
    assert!(!root.path().join("bad").exists());
}
#[tokio::test]
async fn refresh_never_extracts_unexpected_archive_members() {
    let root = tempfile::tempdir().unwrap();
    fixture(&root.path().join("cache/20252026"), "../shots_2025.csv");
    assert!(refresh(request(root.path(), "bad"))
        .await
        .unwrap_err()
        .contains("Unexpected"));
    assert!(!root.path().join("shots_2025.csv").exists());
}
#[tokio::test]
async fn gp_matched_cache_wins_over_retained_older_edge_and_missing_edge_is_optional() {
    let root = tempfile::tempdir().unwrap();
    let cache_root = root.path().join("cache/20252026");
    let retained = root.path().join("retained");
    fs::create_dir_all(retained.join("20252026")).unwrap();
    let mut records = vec![];
    let rows = |report: &str| {
        (1..=20).map(|id|match report {
  "summary"=>json!({"playerId":id,"seasonId":20252026,"positionCode":"C","gamesPlayed":20,"goals":1,"shots":10,"assists":2,"points":3,"pointsPerGame":0.15,"ppGoals":0,"ppPoints":0,"shGoals":0,"shPoints":0,"gameWinningGoals":0,"otGoals":0,"plusMinus":0,"timeOnIcePerGame":600,"teamAbbrevs":"MTL"}),
  "bios"=>json!({"playerId":id,"seasonId":20252026,"skaterFullName":format!("Synthetic Skater {id}"),"lastName":"Skater","positionCode":"C","gamesPlayed":20,"goals":1,"assists":2,"points":3,"currentTeamAbbrev":"MTL"}),
  "realtime"=>json!({"playerId":id,"seasonId":20252026,"gamesPlayed":20,"giveaways":0,"takeaways":3,"blockedShots":4}),
  "penalties"=>json!({"playerId":id,"seasonId":20252026,"gamesPlayed":20,"penaltiesDrawn":1,"penalties":2}),
  _=>json!({"playerId":id,"seasonId":20252026,"gamesPlayed":20,"timeOnIce":12000})}).collect::<Vec<_>>()
    };
    for report in ["summary", "bios", "realtime", "penalties", "timeonice"] {
        let url=format!("https://api.nhle.com/stats/rest/en/skater/{report}?isAggregate=false&isGame=false&limit=100&start=0&sort=%5B%7B%22property%22%3A%22playerId%22%2C%22direction%22%3A%22ASC%22%7D%5D&cayenneExp=seasonId%3D20252026%20and%20gameTypeId%3D2");
        cache(
            &cache_root,
            &url,
            &serde_json::to_vec(&json!({"total":20,"data":rows(report)})).unwrap(),
        );
    }
    for id in 1..=20 {
        let url = format!("https://api-web.nhle.com/v1/edge/skater-detail/{id}/20252026/2");
        let edge = |gp| {
            serde_json::to_vec(&json!({"player":{"id":id,"position":"C","gamesPlayed":gp},"seasonsWithEdgeStats":[{"id":20252026,"gameTypes":[2]}],"skatingSpeed":{"speedMax":{"imperial":20.0,"metric":32.18688},"burstsOver20":{"value":10}}})).unwrap()
        };
        cache(&cache_root, &url, &edge(20));
        let old = edge(19);
        fs::write(retained.join(format!("20252026/{id}.json")), &old).unwrap();
        records.push(json!({"season":20252026,"player_id":id,"group":"Forwards","status":"captured","url":url,"sha256":format!("{:x}",Sha256::digest(&old)),"accessed_utc":"2026-11-19T00:00:00Z"}));
    }
    fs::write(
        retained.join("capture.json"),
        serde_json::to_vec(&json!({"records":records})).unwrap(),
    )
    .unwrap();
    let mut r = request(root.path(), "skaters");
    r.kind = "skaters".into();
    r.edge_captures = Some(retained);
    assert_eq!(refresh(r).await.unwrap()["complete_profiles"], 20);
    let mut r = request(root.path(), "without-edge");
    r.kind = "skaters".into();
    for id in 1..=20 {
        let url = format!("https://api-web.nhle.com/v1/edge/skater-detail/{id}/20252026/2");
        let key = format!("{:x}", Sha256::digest(url.as_bytes()));
        fs::remove_file(cache_root.join(format!("{key}.json"))).unwrap();
    }
    assert_eq!(refresh(r.clone()).await.unwrap()["complete_profiles"], 0);
    let c = Catalog::from_json(
        &fs::read_to_string(r.output_dir.join("rinkcraft-stat-attributes.json")).unwrap(),
    )
    .unwrap();
    assert!(c
        .profiles
        .iter()
        .all(|p| p.scores[0].score.is_none() && p.scores[1..].iter().all(|s| s.score.is_some())));
}
