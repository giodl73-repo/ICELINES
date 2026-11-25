//! Repeatable public-source refresh; generated catalogs remain separate from source code.
pub mod puck;
pub mod calibration;
use crate::{error::FetchError, nhl_api::NhlApiClient};
use icelines_core::view_model::rinkcraft_stat_scores::{self as scores, Catalog, Player};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Request {
    pub season: u32,
    pub kind: String,
    pub cache_dir: PathBuf,
    pub output_dir: PathBuf,
    pub previous_catalog: Option<PathBuf>,
    pub edge_captures: Option<PathBuf>,
    pub game_cache_dir: Option<PathBuf>,
    pub offline: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capture {
    pub url: String,
    pub sha256: String,
    pub captured_at: String,
    pub status: u16,
}
#[derive(Clone)]
struct Store {
    root: PathBuf,
    offline: bool,
    client: NhlApiClient,
}
impl Store {
    async fn get(
        &self,
        url: &str,
        reuse: bool,
        optional: bool,
    ) -> Result<(Option<Vec<u8>>, Capture), String> {
        let key = format!("{:x}", Sha256::digest(url.as_bytes()));
        let raw = self.root.join(format!("{key}.bin"));
        let meta = self.root.join(format!("{key}.json"));
        if meta.exists() && (self.offline || reuse) {
            let c: Capture = serde_json::from_slice(&fs::read(&meta).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if c.url != url {
                return Err("Cache source URL differs".into());
            }
            chrono::DateTime::parse_from_rfc3339(&c.captured_at).map_err(|e| e.to_string())?;
            if c.status == 404 && optional && self.offline {
                return Ok((None, c));
            }
            if c.status != 404 || !optional {
                let b = fs::read(raw).map_err(|e| e.to_string())?;
                if c.status != 200 || c.sha256 != format!("{:x}", Sha256::digest(&b)) {
                    return Err("Cached public source checksum/status differs".into());
                }
                return Ok((Some(b), c));
            }
        }
        if self.offline {
            return Err(format!("Offline public source missing: {url}"));
        }
        let (body, status) = match self.client.fetch_source_bytes(url).await {
            Ok(b) => (Some(b), 200),
            Err(FetchError::Http { status: 404, .. }) if optional => (None, 404),
            Err(e) => return Err(e.to_string()),
        };
        if body.as_ref().is_some_and(|b| b.len() > 100 * 1024 * 1024) {
            return Err("Public source exceeds 100 MiB".into());
        }
        let c = Capture {
            url: url.into(),
            sha256: body
                .as_ref()
                .map(|b| format!("{:x}", Sha256::digest(b)))
                .unwrap_or_default(),
            captured_at: chrono::Utc::now().to_rfc3339(),
            status,
        };
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        if let Some(b) = &body {
            fs::write(raw, b).map_err(|e| e.to_string())?;
        }
        fs::write(meta, serde_json::to_vec(&c).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok((body, c))
    }
    async fn nhl(
        &self,
        report: &str,
        season: u32,
        receipts: &mut Vec<Capture>,
    ) -> Result<Vec<Value>, String> {
        let mut rows = vec![];
        let mut expected = None;
        loop {
            let url=format!("https://api.nhle.com/stats/rest/en/{report}?isAggregate=false&isGame=false&limit=100&start={}&sort=%5B%7B%22property%22%3A%22playerId%22%2C%22direction%22%3A%22ASC%22%7D%5D&cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2",rows.len());
            let (b, c) = self.get(&url, false, false).await?;
            receipts.push(c);
            let v: Value = serde_json::from_slice(&b.ok_or("Missing NHL report")?)
                .map_err(|e| e.to_string())?;
            let total = v["total"]
                .as_u64()
                .filter(|n| *n <= 3000)
                .ok_or("Invalid NHL total")? as usize;
            if expected.replace(total).is_some_and(|old| old != total) {
                return Err("NHL pagination population changed; run refresh again".into());
            }
            let page = v["data"].as_array().ok_or("NHL report rows absent")?;
            if page.is_empty() && rows.len() < total {
                return Err("NHL pagination truncated".into());
            }
            rows.extend(page.iter().cloned());
            if rows.len() >= total {
                if rows.len() != total {
                    return Err("NHL pagination exceeds total".into());
                }
                break;
            }
        }
        Ok(rows)
    }
}
async fn edge(
    store: Store,
    p: Player,
    season: u32,
    retained: Option<(PathBuf, Capture)>,
) -> Result<(Player, Vec<Capture>, Option<String>), String> {
    let fallback = p.clone();
    edge_observation(store, p, season, retained)
        .await
        .or_else(|e| {
            Ok((
                fallback,
                vec![],
                Some(format!("EDGE observation unavailable: {e}")),
            ))
        })
}
async fn edge_observation(
    store: Store,
    mut p: Player,
    season: u32,
    retained: Option<(PathBuf, Capture)>,
) -> Result<(Player, Vec<Capture>, Option<String>), String> {
    let url = format!(
        "https://api-web.nhle.com/v1/edge/skater-detail/{}/{season}/2",
        p.player_id
    );
    let mut receipts = vec![];
    let mut selected = None;
    let mut reason = "NHL EDGE unavailable".to_string();
    let parse = |b: &[u8], c: &Capture| {
        icelines_sources::nhl::edge::parse(
            b,
            p.player_id,
            season,
            &p.group,
            p.games,
            &url,
            &c.captured_at,
        )
    };
    // A newer GP-matched cache must win over an explicitly retained older capture.
    let key = format!("{:x}", Sha256::digest(url.as_bytes()));
    if store.root.join(format!("{key}.json")).exists() {
        let mut cache = store.clone();
        cache.offline = true;
        let (body, c) = cache.get(&url, true, true).await?;
        if let Some(body) = body {
            match parse(&body, &c) {
                Ok(e) => selected = Some(e),
                Err(e) => reason = e,
            }
        }
        receipts.push(c);
    }
    if selected.is_none() {
        if let Some((path, c)) = retained {
            let b = fs::read(path).map_err(|e| e.to_string())?;
            if c.url != url || c.sha256 != format!("{:x}", Sha256::digest(&b)) {
                return Err("Retained EDGE capture source/checksum differs".into());
            }
            match parse(&b, &c) {
                Ok(e) => selected = Some(e),
                Err(e) => reason = e,
            }
            receipts.push(c);
        }
    }
    if selected.is_none() && !store.offline {
        let (body, c) = store.get(&url, false, true).await?;
        if let Some(body) = body {
            match parse(&body, &c) {
                Ok(e) => selected = Some(e),
                Err(e) => reason = e,
            }
        }
        receipts.push(c);
    }
    let note = if let Some(e) = selected {
        for m in e.measures {
            match m.key.as_str() {
                "max-skating-speed" => {
                    p.metrics.insert("peak-speed".into(), m.value);
                }
                "speed-bursts-over-20" if p.minutes > 0. => {
                    p.metrics.insert("bursts".into(), m.value * 60. / p.minutes);
                }
                "skating-distance" => {
                    p.metrics.insert("edge-distance".into(), m.value);
                }
                "max-shot-speed" => {
                    p.metrics.insert("edge-shot-speed".into(), m.value);
                }
                "defensive-zone-puck-time" => {
                    p.metrics.insert("edge-dz".into(), m.value);
                }
                _ => {}
            }
        }
        None
    } else {
        Some(reason)
    };
    Ok((p, receipts, note))
}

fn write(path: &Path, v: &impl Serialize) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
pub async fn refresh(request: Request) -> Result<Value, String> {
    let year = request.season / 10000;
    if !(2008..=2200).contains(&year)
        || request.season % 10000 != year + 1
        || !["all", "skaters", "goalies"].contains(&request.kind.as_str())
    {
        return Err("Use a valid YYYYZZZZ season and all/skaters/goalies kind".into());
    }
    if request.output_dir.exists() {
        return Err(
            "Refresh requires a new output directory; existing catalogs are preserved".into(),
        );
    }
    let store = Store {
        root: request.cache_dir.join(request.season.to_string()),
        offline: request.offline,
        client: NhlApiClient::production(),
    };
    let mut package = icelines_sources::schema::SeasonPackage {
        schema_version: 1,
        season: request.season,
        season_type: icelines_core::season_stats::SeasonType::Regular,
        source: "NHL public reports · attribute refresh".into(),
        observed_at: None,
        fetched_at: None,
        bios: vec![],
        stats: vec![],
        goalies: vec![],
    };
    let mut receipts = vec![];
    let mut observations = vec![];
    let mut shot_contexts = BTreeMap::new();
    let mut skater_sources = None;
    let mut defending_source=None;
    let mut notes = BTreeMap::<u32, Vec<String>>::new();
    if request.kind != "skaters" {
        eprintln!("Refreshing goalie public sources for {}", request.season);
        let rows = store
            .nhl("goalie/summary", request.season, &mut receipts)
            .await?;
        package.goalies =
            serde_json::from_value(json!(rows)).map_err(|e| format!("Goalie package rows: {e}"))?;
        let url = format!(
            "https://moneypuck.com/moneypuck/playerData/seasonSummary/{year}/regular/goalies.csv"
        );
        let (csv, c) = store.get(&url, false, false).await?;
        receipts.push(c);
        let csv = String::from_utf8(csv.ok_or("Missing goalie CSV")?).map_err(|e| e.to_string())?;
        let url = format!("https://peter-tanner.com/moneypuck/downloads/shots_{year}.zip");
        let (zip, c) = store.get(&url, false, false).await?;
        let shot_capture = c.clone();
        receipts.push(c);
        let mut archive = zip::ZipArchive::new(Cursor::new(zip.ok_or("Missing shots ZIP")?))
            .map_err(|e| e.to_string())?;
        if archive.len() != 1 {
            return Err("Expected one CSV in MoneyPuck shots archive".into());
        }
        let file = archive.by_index(0).map_err(|e| e.to_string())?;
        if file.name().map_err(|e| e.to_string())?.as_ref() != format!("shots_{year}.csv").as_str()
            || file.size() > 256 * 1024 * 1024
        {
            return Err("Unexpected/oversized MoneyPuck shot member".into());
        }
        for o in icelines_sources::rinkcraft_stat_inputs::goalie_inputs(
            request.season,
            &rows,
            &csv,
            file,
        )? {
            if let Some(data) = o.shot_context {
                shot_contexts.insert(
                    o.player.player_id,
                    icelines_core::view_model::rinkcraft_goalie_shots::Context::new(
                        data,
                        shot_capture.url.clone(),
                        shot_capture.captured_at.clone(),
                        shot_capture.sha256.clone(),
                    ),
                );
            }
            notes.insert(o.player.player_id, o.notes);
            observations.push(o.player);
        }
    }
    if request.kind != "goalies" {
        eprintln!("Refreshing skater public sources for {}", request.season);
        let summary = store
            .nhl("skater/summary", request.season, &mut receipts)
            .await?;
        let bios = store
            .nhl("skater/bios", request.season, &mut receipts)
            .await?;
        package.bios = serde_json::from_value(json!(bios))
            .map_err(|e| format!("Skater bio package rows: {e}"))?;
        package.stats = serde_json::from_value(json!(summary))
            .map_err(|e| format!("Skater stat package rows: {e}"))?;
        let ids = package
            .stats
            .iter()
            .map(|r| (r.player_id, r.games_played))
            .collect::<BTreeMap<_, _>>();
        if package.bios.len() != package.stats.len()
            || package
                .bios
                .iter()
                .any(|r| ids.get(&r.player_id) != Some(&r.games_played))
        {
            return Err("NHL bios and stats disagree on player/GP coverage; refresh again".into());
        }
        let realtime = store
            .nhl("skater/realtime", request.season, &mut receipts)
            .await?;
        let penalties = store
            .nhl("skater/penalties", request.season, &mut receipts)
            .await?;
        let toi = store
            .nhl("skater/timeonice", request.season, &mut receipts)
            .await?;
        let players = icelines_sources::rinkcraft_stat_inputs::skater_inputs(
            request.season,
            &summary,
            &realtime,
            &penalties,
            &toi,
        )?;
        skater_sources = Some((summary, realtime, toi));
        let mut retained = BTreeMap::<u32, Capture>::new();
        if let Some(root) = &request.edge_captures {
            let manifest: Value = serde_json::from_slice(
                &fs::read(root.join("capture.json")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            for r in manifest["records"]
                .as_array()
                .ok_or("Retained EDGE records missing")?
            {
                if r["season"].as_u64() != Some(request.season as u64)
                    || r["status"] != "captured"
                    || r["group"] == "Goalies"
                {
                    continue;
                }
                let id = r["player_id"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or("Retained EDGE id missing")?;
                let c = Capture {
                    url: r["url"].as_str().ok_or("Retained EDGE URL missing")?.into(),
                    sha256: r["sha256"]
                        .as_str()
                        .ok_or("Retained EDGE hash missing")?
                        .into(),
                    captured_at: r["accessed_utc"]
                        .as_str()
                        .ok_or("Retained EDGE date missing")?
                        .into(),
                    status: 200,
                };
                if retained.insert(id, c).is_some() {
                    return Err("Duplicate retained EDGE actor".into());
                }
            }
        }
        let total = players.len();
        let mut tasks = tokio::task::JoinSet::new();
        let mut done = 0;
        for p in players {
            let retained = request
                .edge_captures
                .as_ref()
                .map(|d| d.join(format!("{}/{}.json", request.season, p.player_id)))
                .zip(retained.get(&p.player_id).cloned());
            tasks.spawn(edge(store.clone(), p, request.season, retained));
            if tasks.len() >= 4 {
                let (p, rs, note) = tasks
                    .join_next()
                    .await
                    .ok_or("Missing EDGE task")?
                    .map_err(|e| e.to_string())??;
                if let Some(note) = note {
                    notes.entry(p.player_id).or_default().push(note);
                }
                receipts.extend(rs);
                observations.push(p);
                done += 1;
                if done % 50 == 0 {
                    eprintln!("EDGE {done}/{total}")
                }
            }
        }
        while let Some(result) = tasks.join_next().await {
            let (p, rs, note) = result.map_err(|e| e.to_string())??;
            if let Some(note) = note {
                notes.entry(p.player_id).or_default().push(note);
            }
            receipts.extend(rs);
            observations.push(p);
        }
    }
    observations.sort_by_key(|p| p.player_id);
    let mut skater_contexts = BTreeMap::new();
    let mut faceoff_samples = BTreeMap::<u32,icelines_core::view_model::rinkcraft_under_pressure::Faceoffs>::new();
    if let Some((summary, realtime, toi)) = skater_sources {
        let mut assists = None;
        if let Some(root) = &request.game_cache_dir {
            let games_store = Store {
                root: root.join(request.season.to_string()),
                ..store.clone()
            };
            let teams = crate::teams::nhl_teams_for_season(&request.season.to_string())
                .into_iter()
                .map(str::to_owned)
                .collect();
            let (games, captures) = puck::capture_games(
                games_store,
                request.season,
                teams,
                |bytes,id,season| Ok((icelines_sources::rinkcraft_skater_context::goal_assists(bytes,id,season)?,icelines_sources::rinkcraft_pressure::parse_game(bytes,id,season)?)),
            )
            .await?;
            let coverage_games=games.len() as u32;
            let mut event_captures=captures.iter().filter(|c|c.url.ends_with("/play-by-play")).collect::<Vec<_>>();
            event_captures.sort_by(|a,b|a.url.cmp(&b.url));
            let captured_at=event_captures.iter().map(|c|c.captured_at.as_str()).max().ok_or("Missing pressure captures")?.to_string();
            let source_sha256=format!("{:x}",Sha256::digest(serde_json::to_vec(&serde_json::to_value(&event_captures).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?));
            receipts.extend(captures);
            let mut totals =
                BTreeMap::<u32, icelines_sources::rinkcraft_skater_context::Assists>::new();
            for (game,draws) in games {
                for (id,c) in draws {
                    let f=faceoff_samples.entry(id).or_insert_with(||icelines_core::view_model::rinkcraft_under_pressure::Faceoffs {season:request.season,player_id:id,taken:0,won:0,late_taken:0,late_won:0,coverage_games,captured_at:captured_at.clone(),source_sha256:source_sha256.clone()});
                    f.taken+=c.taken;f.won+=c.won;f.late_taken+=c.late_taken;f.late_won+=c.late_won;
                }
                for (id, a) in game {
                    let sum = totals.entry(id).or_default();
                    sum.primary += a.primary;
                    sum.secondary += a.secondary;
                }
            }
            assists = Some(totals);
        }
        let url = format!(
            "https://peter-tanner.com/moneypuck/downloads/shots_{}.zip",
            request.season / 10000
        );
        let mut shooting = None;
        let mut shot_date = None;
        if let Ok((Some(raw), capture)) = store.get(&url, request.kind != "skaters", false).await {
            let mut zip = zip::ZipArchive::new(Cursor::new(raw)).map_err(|e| e.to_string())?;
            if zip.len() != 1 {
                return Err("Expected one shot CSV".into());
            }
            let file = zip.by_index(0).map_err(|e| e.to_string())?;
            if file.size() > 256 * 1024 * 1024 {
                return Err("Shot CSV exceeds bound".into());
            }
            shooting = Some(icelines_sources::rinkcraft_skater_context::shooting(
                file,
                request.season,
            )?);
            shot_date = Some(capture.captured_at.clone());
            receipts.push(capture);
        }
        let players = observations
            .iter()
            .filter(|p| p.group != "Goalies")
            .cloned()
            .collect::<Vec<_>>();
        skater_contexts = icelines_sources::rinkcraft_skater_context::build(
            &players,
            &summary,
            &realtime,
            &toi,
            assists.as_ref(),
            shooting.as_ref(),
        )?;
        for context in skater_contexts.values_mut() {
            if let Some(date) = &shot_date {
                context.notes.push(format!("MoneyPuck.com shot archive captured {date}; shot-quality counts use its own goalie-in-net attempts and goals, independently of the NHL time/production snapshot."));
            }
        }
        let url=format!("https://moneypuck.com/moneypuck/playerData/seasonSummary/{year}/regular/skaters.csv");
        if let Ok((Some(raw),capture))=store.get(&url,false,true).await {
            defending_source=Some((raw,capture.clone()));receipts.push(capture);
        }
    }
    let mut profiles = scores::build(request.season, "all strengths · rates per60", &observations)?;
    for p in &mut profiles {
        p.skater_context = skater_contexts.remove(&p.player_id);
        if p.group != "Goalies" {p.under_pressure=Some(icelines_core::view_model::rinkcraft_under_pressure::build(p,faceoff_samples.remove(&p.player_id)));}
        if p.group == "Goalies" {
            p.shot_context = shot_contexts.remove(&p.player_id);
            p.scope = "all strengths · NHL and MoneyPuck observations".into();
        }
        if let Some(ns) = notes.get(&p.player_id) {
            for s in &mut p.scores {
                for n in ns {
                    s.note.push_str(" ");
                    s.note.push_str(n);
                }
            }
        }
    }
    if let Some((raw,capture))=defending_source {
        icelines_sources::rinkcraft_defending_context::enrich(&mut profiles,request.season,Cursor::new(raw),&format!("{} · {} · SHA-256 {}",capture.captured_at,capture.url,capture.sha256))?;
    }else{icelines_core::view_model::rinkcraft_skater_context::attach_records(&mut profiles,request.season);}
    let refreshed = profiles.len();
    let complete = profiles
        .iter()
        .filter(|p| p.scores.iter().all(|s| s.score.is_some()))
        .count();
    let groups = profiles
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut m, p| {
            *m.entry(p.group.clone()).or_default() += 1;
            m
        });
    let mut previous = if let Some(path) = &request.previous_catalog {
        Catalog::from_json(&fs::read_to_string(path).map_err(|e| e.to_string())?)?.profiles
    } else {
        vec![]
    };
    previous.retain(|p| {
        p.season != request.season
            || match request.kind.as_str() {
                "goalies" => p.group != "Goalies",
                "skaters" => p.group == "Goalies",
                _ => false,
            }
    });
    previous.extend(profiles);
    previous.sort_by_key(|p| (p.season, p.player_id));
    let catalog = Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: previous,
    };
    catalog.validate()?;
    let catalog_bytes = serde_json::to_vec(&catalog).map_err(|e| e.to_string())?;
    if catalog_bytes.len() > 32 * 1024 * 1024 {
        return Err("Catalog exceeds 32 MiB; retain fewer seasons in this portable catalog".into());
    }
    let report = json!({"schema":"rinkcraft.attribute-refresh.v1","refreshed_at":chrono::Utc::now().to_rfc3339(),"season":request.season,"kind":request.kind,"offline":request.offline,"refreshed_profiles":refreshed,"complete_profiles":complete,"groups":groups,"source_credit":"NHL public reports; MoneyPuck.com public shot and goalie data","source_notes":notes,"captures":receipts,"input_observations":observations});
    package.fetched_at = receipts
        .iter()
        .filter(|c| c.url.starts_with("https://api.nhle.com/stats/rest/"))
        .map(|c| c.captured_at.clone())
        .max();
    fs::create_dir_all(&request.output_dir).map_err(|e| e.to_string())?;
    fs::write(
        request.output_dir.join("rinkcraft-stat-attributes.json"),
        catalog_bytes,
    )
    .map_err(|e| e.to_string())?;
    write(&request.output_dir.join("refresh-report.json"), &report)?;
    write(&request.output_dir.join("season-package.json"), &package)?;
    Ok(
        json!({"season":request.season,"refreshed_profiles":refreshed,"complete_profiles":complete,"output_dir":request.output_dir}),
    )
}
