//! Offline four-season trial from checksum-verified NHL goalie summary pages.
use icelines_core::view_model::rinkcraft_goalie_feat_trial as trial;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::PathBuf};
#[derive(Deserialize)]
struct Manifest {
    schema: String,
    pages: Vec<Page>,
}
#[derive(Deserialize)]
struct Page {
    season: u32,
    file: PathBuf,
    url: String,
    sha256: String,
    captured_at: String,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: rinkcraft_goalie_feat_trial MANIFEST NEW_OUTPUT".into());
    }
    let raw = fs::read(&args[0])?;
    if raw.len() > 1024 * 1024 {
        return Err("Manifest too large".into());
    }
    let m: Manifest = serde_json::from_slice(&raw)?;
    if m.schema != "rinkcraft.goalie-feat-trial-input.v1"
        || m.pages.is_empty()
        || m.pages.len() > 32
    {
        return Err("Invalid manifest".into());
    }
    let out = PathBuf::from(&args[1]);
    if out.exists() {
        return Err("Choose a new output directory".into());
    }
    let mut seasons =
        BTreeMap::<u32, Vec<(usize, usize, Vec<Value>, String, String, String)>>::new();
    for p in m.pages {
        if fs::metadata(&p.file)?.len() > 4 * 1024 * 1024 {
            return Err("Page too large".into());
        }
        let bytes = fs::read(&p.file)?;
        if format!("{:x}", Sha256::digest(&bytes)) != p.sha256 {
            return Err("Page checksum mismatch".into());
        }
        chrono::DateTime::parse_from_rfc3339(&p.captured_at)?;
        let v: Value = serde_json::from_slice(&bytes)?;
        let data = v["data"].as_array().ok_or("Missing rows")?.clone();
        let url = url::Url::parse(&p.url)?;
        let query = url.query_pairs().collect::<BTreeMap<_, _>>();
        let start = query
            .get("start")
            .ok_or("Missing page offset")?
            .parse::<usize>()?;
        let total = v["total"].as_u64().ok_or("Missing total")? as usize;
        seasons.entry(p.season).or_default().push((
            start,
            total,
            data,
            p.url,
            p.sha256,
            p.captured_at,
        ));
    }
    let mut reports = vec![];
    let mut inputs = vec![];
    let mut receipts = vec![];
    for (season, mut pages) in seasons {
        pages.sort_by_key(|p| p.0);
        let total = pages[0].1;
        if total > 2000 {
            return Err("Goalie cohort too large".into());
        }
        let mut rows = vec![];
        let mut urls = vec![];
        for (start, t, data, url, sha, date) in pages {
            if start != rows.len() || t != total || data.is_empty() {
                return Err("Incomplete/overlapping report pages".into());
            }
            rows.extend(data);
            urls.push(url.clone());
            receipts.push(json!({"season":season,"url":url,"sha256":sha,"captured_at":date}));
        }
        if rows.len() != total {
            return Err("Incomplete goalie cohort".into());
        }
        let bytes = serde_json::to_vec(&json!({"total":total,"data":rows}))?;
        let input = icelines_sources::nhl::achievements::goalie_input(&bytes, season, urls)?;
        let report = trial::evaluate(&input)?;
        println!(
            "{season}: eligible {} · recognized {} -> {} · new {}",
            report.eligible,
            report.baseline_recognized,
            report.trial_recognized,
            report.newly_recognized
        );
        reports.push(report);
        inputs.push(input);
    }
    let transitions = transitions(&reports);
    fs::create_dir_all(&out)?;
    fs::write(
        out.join("trial.json"),
        serde_json::to_vec(
            &json!({"schema":"rinkcraft.goalie-feat-study.v1","status":"opt-in trial; active registry unchanged","manifest_sha256":format!("{:x}",Sha256::digest(&raw)),"captures":receipts,"inputs":inputs,"rules":trial::rules(),"reports":reports,"transitions":transitions}),
        )?,
    )?;
    Ok(())
}
fn transitions(reports: &[trial::Report]) -> Vec<Value> {
    reports.windows(2).filter(|p|p[0].season+10001==p[1].season).flat_map(|p|{
        ["goalie-save-percentage","goalie-shot-workload"].into_iter().map(move|id|{
            let old=p[0].profiles.iter().filter(|a|a.games>=10).collect::<Vec<_>>();let mut pairs=0;let mut winners=0;let mut repeats=0;
            for a in old {if let Some(b)=p[1].profiles.iter().find(|b|b.player_id==a.player_id&&b.games>=10){pairs+=1;let won=a.feats.iter().any(|f|f.id==id&&f.state=="earned");winners+=usize::from(won);repeats+=usize::from(won&&b.feats.iter().any(|f|f.id==id&&f.state=="earned"));}}
            let prior_total=p[0].profiles.iter().filter(|a|a.feats.iter().any(|f|f.id==id&&f.state=="earned")).count();
            json!({"from":p[0].season,"to":p[1].season,"id":id,"eligible_pairs":pairs,"prior_qualifiers":winners,"repeat_qualifiers":repeats,"note":"Changing annual references; current endpoint may be partial. Descriptive repeat, not a frozen prediction or validated trait.","prior_qualifiers_all":prior_total,"without_next_eligible":prior_total-winners})
        })
    }).collect()
}
