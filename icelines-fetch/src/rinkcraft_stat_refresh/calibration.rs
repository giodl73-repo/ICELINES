//! Single-season diagnostic inputs. Skating/EDGE is intentionally not acquired.
use super::{write, Store};
use crate::nhl_api::NhlApiClient;
use icelines_core::view_model::{
    rinkcraft_skater_context::Context, rinkcraft_stat_scores as scores,
};
use icelines_sources::{rinkcraft_skater_context as context, schema::SeasonPackage};
use serde_json::{json, Value};
use std::{fs, io::Cursor, path::PathBuf};

pub struct Request {
    pub season: u32,
    pub cache_dir: PathBuf,
    pub output_dir: PathBuf,
    pub offline: bool,
}
pub async fn capture(r: Request) -> Result<Value, String> {
    let year = r.season / 10000;
    if !(2008..=2200).contains(&year) || r.season % 10000 != year + 1 {
        return Err("Use a YYYYZZZZ season since 2008-09".into());
    }
    if r.output_dir.exists() {
        return Err("Choose a new output directory".into());
    }
    let store = Store {
        root: r.cache_dir.join(r.season.to_string()),
        offline: r.offline,
        client: NhlApiClient::production(),
    };
    let mut receipts = vec![];
    let summary = store.nhl("skater/summary", r.season, &mut receipts).await?;
    let bios = store.nhl("skater/bios", r.season, &mut receipts).await?;
    let realtime = store
        .nhl("skater/realtime", r.season, &mut receipts)
        .await?;
    let penalties = store
        .nhl("skater/penalties", r.season, &mut receipts)
        .await?;
    let toi = store
        .nhl("skater/timeonice", r.season, &mut receipts)
        .await?;
    let players = icelines_sources::rinkcraft_stat_inputs::skater_inputs(
        r.season, &summary, &realtime, &penalties, &toi,
    )?;
    let url = format!(
        "https://moneypuck.com/moneypuck/playerData/seasonSummary/{year}/regular/skaters.csv"
    );
    let (csv, capture) = store.get(&url, true, false).await?;
    receipts.push(capture);
    let csv = csv.ok_or("Missing skater CSV")?;
    let assists = context::season_assists(Cursor::new(&csv), r.season, &summary)?;
    let mut defending = icelines_sources::rinkcraft_defending_context::parse(
        Cursor::new(&csv),
        r.season,
        &players,
    )?;
    let defending_matched = defending.len();
    let url = format!("https://peter-tanner.com/moneypuck/downloads/shots_{year}.zip");
    let (raw, capture) = store.get(&url, true, false).await?;
    receipts.push(capture);
    let mut archive = zip::ZipArchive::new(Cursor::new(raw.ok_or("Missing shots archive")?))
        .map_err(|e| e.to_string())?;
    if archive.len() != 1 {
        return Err("Expected one shot CSV".into());
    }
    let file = archive.by_index(0).map_err(|e| e.to_string())?;
    if file.size() > 256 * 1024 * 1024 {
        return Err("Shot CSV exceeds bound".into());
    }
    let shots = context::shooting(file, r.season)?;
    let mut contexts = context::build(&players, &summary, &realtime, &toi, None, Some(&shots))?;
    let mut profiles = scores::build(
        r.season,
        "diagnostic inputs · all strengths · no EDGE acquisition",
        &players,
    )?;
    let mut matched = 0;
    for p in &mut profiles {
        let mut c = contexts
            .remove(&p.player_id)
            .ok_or("Missing skater context")?;
        if let Some(values) = defending.remove(&p.player_id) {
            c.values.extend(values);
        }
        if let Some(seconds) = toi
            .iter()
            .find(|t| t["playerId"].as_u64() == Some(p.player_id as u64))
            .and_then(|t| t["shTimeOnIce"].as_f64())
            .filter(|v| v.is_finite() && *v >= 0. && p.minutes > 0.)
        {
            c.values
                .insert("def-pk-share".into(), seconds / (p.minutes * 60.));
        }
        if let Some(a) = assists.get(&p.player_id) {
            c.values.insert("primary".into(), a.primary as f64);
            c.values.insert("secondary".into(), a.secondary as f64);
            if p.minutes > 0. {
                c.values
                    .insert("primary-rate".into(), a.primary as f64 * 60. / p.minutes);
            }
            matched += 1;
        } else {
            c.notes.push("Assist-type detail unavailable: missing MoneyPuck row or GP/total-assist mismatch with NHL.".into());
        }
        c.source_credit = "NHL public season reports; MoneyPuck.com assist credits, goalie-in-net shot data and five-on-five on/off results".into();
        p.skater_context = Some(Context::new(c.values, c.source_credit, c.notes));
    }
    let catalog = scores::Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    };
    catalog.validate()?;
    let package = SeasonPackage {
        schema_version: 1,
        season: r.season,
        season_type: icelines_core::season_stats::SeasonType::Regular,
        source: "NHL public reports · calibration inputs".into(),
        observed_at: None,
        fetched_at: None,
        bios: serde_json::from_value(json!(bios)).map_err(|e| e.to_string())?,
        stats: serde_json::from_value(json!(summary)).map_err(|e| e.to_string())?,
        goalies: vec![],
    };
    let report = json!({"schema":"rinkcraft.calibration-inputs.v1","season":r.season,"players":players.len(),"reconciled_assist_players":matched,"matched_defending_players":defending_matched,"source_credit":"NHL; MoneyPuck.com","captures":receipts,"notes":["Diagnostic catalog only: no EDGE acquisition. Baseline recipes unchanged. Missing assist detail is not zero. Five-on-five defense is on-ice team performance; on/off differences do not isolate player impact."]});
    fs::create_dir_all(&r.output_dir).map_err(|e| e.to_string())?;
    write(
        &r.output_dir.join("rinkcraft-stat-attributes.json"),
        &catalog,
    )?;
    write(&r.output_dir.join("season-package.json"), &package)?;
    write(&r.output_dir.join("capture-report.json"), &report)?;
    Ok(
        json!({"season":r.season,"players":players.len(),"reconciled_assist_players":matched,"output":r.output_dir}),
    )
}
