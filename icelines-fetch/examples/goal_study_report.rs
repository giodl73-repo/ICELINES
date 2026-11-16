//! Descriptive study report. Counts selected cases; never estimates success rates.
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
#[path = "support/goal_contributors.rs"]
mod contributors;

fn increment(counts: &mut BTreeMap<String, usize>, key: Option<&str>) {
    *counts
        .entry(key.unwrap_or("unknown").to_owned())
        .or_default() += 1;
}
fn summarize(records: &[Value]) -> Value {
    let mut players: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for record in records {
        players
            .entry(
                record["selection"]["player"]["name"]
                    .as_str()
                    .unwrap_or("unknown")
                    .into(),
            )
            .or_default()
            .push(record);
    }
    let rows:Vec<_>=players.into_iter().map(|(name,cases)|{
        let mut strengths=BTreeMap::new();let mut shots=BTreeMap::new();let mut keepers=BTreeMap::new();
        let mut captured=0;let mut frames=0;let mut unassisted=0;let mut overtime=0;
        let mut personnel=BTreeSet::new();
        for case in &cases {
            if case["status"]!="captured" {continue}
            captured+=1;let s=&case["summary"];let g=&s["goal"];
            increment(&mut strengths,g["strength"].as_str());increment(&mut shots,g["shot_type"].as_str());
            increment(&mut keepers,s["goalkeeper_context"].as_str());
            frames+=s["frame_count"].as_u64().unwrap_or(0);
            if g["assists"].as_array().is_some_and(|a|a.is_empty()) {unassisted+=1;}
            if g["period_type"]=="OT" {overtime+=1;}
            if let Some(ids)=case["observed_players"].as_array(){for id in ids {if let Some(id)=id.as_u64(){personnel.insert(id);}}}
        }
        json!({"player":name,"planned_cases":cases.len(),"captured_cases":captured,"unavailable_cases":cases.len()-captured,
            "frames":frames,"strength_counts":strengths,"recorded_shot_type_counts":shots,"goalkeeper_context_counts":keepers,
            "unassisted_goals":unassisted,"overtime_goals":overtime,"unique_observed_player_ids":personnel.len()})
    }).collect();
    json!({"schema":"icelines.goal-study-summary.v0.1","planned_cases":records.len(),
        "captured_cases":records.iter().filter(|r|r["status"]=="captured").count(),
        "frames":rows.iter().map(|r|r["frames"].as_u64().unwrap()).sum::<u64>(),"players":rows,
        "interpretation":"Counts among purposively selected successful-goal cases. Not frequencies among all goals or attempts, success rates, skill estimates or tactical diagnoses. Multiple cases can share a game."})
}
fn counts(value: &Value) -> String {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join(", ")
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 2 || args.len() == 3,
        "usage: goal_study_report RECORDS_JSON OUTPUT_DIRECTORY [ANNOTATIONS_JSON]"
    );
    let source: Value = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    anyhow::ensure!(
        source["schema"] == "icelines.goal-study-records.v0.1",
        "Unsupported study records"
    );
    let mut records = source["records"].as_array().context("records")?.clone();
    if args.len() == 3 {
        let annotations: Value = serde_json::from_slice(&std::fs::read(&args[2])?)?;
        anyhow::ensure!(
            annotations["schema"] == "rinkcraft.goal-study-annotations.v0.1",
            "Unsupported annotations"
        );
        let mut seen = BTreeSet::new();
        for annotation in annotations["annotations"]
            .as_array()
            .context("annotations")?
        {
            let key = annotation["target"].as_str().context("annotation target")?;
            anyhow::ensure!(seen.insert(key), "Duplicate annotation target");
            let record = records
                .iter_mut()
                .find(|r| {
                    r["status"] == "captured"
                        && format!("{}/{}", r["target"]["game_id"], r["target"]["event_id"]) == key
                })
                .context("Annotation target is not a captured study case")?;
            record["documentary_annotation"] = annotation.clone();
        }
    }
    let source_dir = PathBuf::from(&args[0])
        .parent()
        .context("records directory")?
        .to_owned();
    for record in records.iter_mut().filter(|r| r["status"] == "captured") {
        let filename = record["report_file"].as_str().context("report_file")?;
        anyhow::ensure!(
            std::path::Path::new(filename)
                .file_name()
                .and_then(|p| p.to_str())
                == Some(filename),
            "Invalid report filename"
        );
        let bytes = std::fs::read(source_dir.join("goals").join(filename))?;
        anyhow::ensure!(
            record["report_sha256"].as_str()
                == Some(icelines_fetch::goal_visualizer::sha256_hex(&bytes).as_str()),
            "Goal report hash mismatch"
        );
        let report: Value = serde_json::from_slice(&bytes)?;
        anyhow::ensure!(
            report["target"] == record["target"] && report["summary"] == record["summary"],
            "Goal report identity/context mismatch"
        );
        record["credited_contributions"] = contributors::assess(
            &report["audit"]["events"][0],
            &record["documentary_annotation"],
        )?;
    }
    let mut summary = summarize(&records);
    summary["documentary_annotated_cases"] = json!(records
        .iter()
        .filter(|r| r["documentary_annotation"].is_object())
        .count());
    let out = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&out)?;
    icelines_fetch::atomic_write::write_json_atomic(&out.join("summary.json"), &summary)?;
    icelines_fetch::atomic_write::write_json_atomic(
        &out.join("review_records.json"),
        &json!({"schema":"icelines.goal-study-review.v0.1","records":records}),
    )?;
    let mut md=format!("# Successful-goal study: official-context pass\n\n{} selected cases; {} captured; {} frames.\n\n{}\n\n| Player | Captured / selected | Recorded strength | Recorded shot types |\n|---|---|---|---|\n",summary["planned_cases"],summary["captured_cases"],summary["frames"],summary["interpretation"].as_str().unwrap());
    for p in summary["players"].as_array().unwrap() {
        md.push_str(&format!(
            "| {} | {} / {} | {} | {} |\n",
            p["player"].as_str().unwrap(),
            p["captured_cases"],
            p["planned_cases"],
            counts(&p["strength_counts"]),
            counts(&p["recorded_shot_type_counts"])
        ));
    }
    md.push_str("\n## Unavailable cases\n\n");
    for r in records.iter().filter(|r| r["status"] != "captured") {
        md.push_str(&format!(
            "- {} · {} · game{}: {}\n",
            r["selection"]["player"]["name"].as_str().unwrap(),
            r["selection"]["game_date"].as_str().unwrap(),
            r["selection"]["game_id"],
            r["reason"].as_str().unwrap_or("unknown")
        ));
    }
    md.push_str("\nTactical mechanisms remain unassessed until recorded evidence supports an action chain. A recorded shot type does not identify a one-timer, screen, pre-shot pass or defensive failure. Goals by defensemen do not prove activation. Empty-net goals do not test goalie execution. Missing cases may introduce additional selection bias.\n");
    std::fs::write(out.join("summary.md"), md)?;
    let payload = json!({"summary":summary,"records":records})
        .to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let html = HTML.replace("__STUDY_DATA__", &payload);
    std::fs::write(out.join("index.html"), html)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
const HTML: &str = r##"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>IceLines · Successful-goal study</title>
<style>body{font:15px system-ui;background:#eff3f7;color:#18304a;margin:0}main{max-width:1200px;margin:auto;padding:24px}h1{margin:0}header,article{background:white;border:1px solid #cad7e3;border-radius:8px;padding:18px;margin-bottom:14px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,340px),1fr));gap:14px}article{margin:0}h2{font-size:18px}p{line-height:1.5}.small{font-size:12px;color:#52687a}label{display:inline-block;margin:12px 20px 12px 0}select{padding:8px;font:inherit}a{color:#145386}.bad{border-left:5px solid #ac6720}table{width:100%;border-collapse:collapse;font-size:13px}td,th{text-align:left;padding:8px;border-bottom:1px solid #dce5ed}.scroll{overflow:auto}.touch-order{font-size:13px;font-weight:bold}.touch-anchors{border-top:1px solid #dce5ed;padding-top:8px}.anchor-candidate{font-size:13px}.contributors dt{font-weight:bold;margin-top:8px;text-transform:capitalize}.contributors dd{margin:3px 0 8px;font-size:13px}.status{font-weight:bold;color:#6b510c}</style>
<main><header><p class="small">ICELINES · RINKCRAFT EXPLORATORY STUDY · 2023–24 REGULAR SEASON</p><h1>How do these goals happen?</h1><p id="coverage"></p><p>Official-context and observation pass. Selected cards include attributed NHL descriptions and provisional mechanisms. Video/release alignment and independent review remain incomplete. Scoring outcomes alone do not identify a missed assignment or player capability.</p><div class="scroll"><table><thead><tr><th>Player</th><th>Captured / selected</th><th>Strength</th><th>Recorded shot type</th></tr></thead><tbody id="summary"></tbody></table></div><label>Player <select id="player"><option value="">All players</option></select></label><label>Status <select id="status"><option value="">All cases</option><option value="captured">Captured</option><option value="unavailable">Unavailable</option></select></label><p class="small">Each case links to native IceLines analysis and the official source. Counted cases are successful goals selected by scoring-game rank, not a denominator of attempts or a representative sample of a player's goals.</p></header><div id="cases" class="grid"></div></main>
<script type="application/json" id="data">__STUDY_DATA__</script><script>(()=>{const d=JSON.parse(document.getElementById('data').textContent),$=id=>document.getElementById(id);const node=(tag,text,parent)=>{let n=document.createElement(tag);n.textContent=text;parent.append(n);return n};const counts=o=>Object.entries(o).map(([k,v])=>`${k}: ${v}`).join(', ');$('coverage').textContent=`${d.summary.planned_cases} planned cases · ${d.summary.captured_cases} captured · ${d.summary.frames} frames`;
for(const p of d.summary.players){let tr=node('tr','',$('summary'));for(const text of [p.player,`${p.captured_cases} / ${p.planned_cases}`,counts(p.strength_counts),counts(p.recorded_shot_type_counts)])node('td',text,tr);let o=node('option',p.player,$('player'));o.value=p.player}
function link(parent,text,url){try{let u=new URL(url);if(u.origin!=='http://127.0.0.1:8786'&&!(u.protocol==='https:'&&['www.nhl.com','nhl.com','wsr.nhle.com'].includes(u.hostname)))return;let a=node('a',text,parent);a.href=u.href;a.target='_blank';a.rel='noopener';parent.append(document.createElement('br'))}catch{}}
function render(){$('cases').replaceChildren();for(const r of d.records){let s=r.selection;if($('player').value&&$('player').value!==s.player.name)continue;if($('status').value&&$('status').value!==r.status)continue;let article=node('article','',$('cases'));article.dataset.status=r.status;node('h2',`${s.player.name} · ${s.game_date}`,article);node('p',`Scoring-game rank ${s.scoring_game_rank} · game ${s.game_id}`,article).className='small';if(r.status!=='captured'){article.className='bad';node('p',`Unavailable: ${r.reason}`,article);continue}let g=r.summary.goal;node('p',`${r.summary.away_team} at ${r.summary.home_team} · P${g.period} ${g.time_in_period} · ${g.strength||'unknown strength'} · ${g.shot_type||'unknown shot type'} · ${r.summary.goalkeeper_context}`,article);node('h3','Scoring credits and contributions',article);if(r.credited_contributions){let dl=node('dl','',article);dl.className='contributors';for(const p of r.credited_contributions.participants){let dt=node('dt',`${p.official_credit} · ${p.name||p.player_id}`,dl);dt.dataset.credit=p.official_credit;let description=p.action_status==='documentary-supported'?`${p.action} · attributed description (${p.action_source_ids.join(', ')})`:'Action unassessed — official credit alone does not identify the task.';node('dd',description,dl)}const c=r.credited_contributions,person=id=>c.participants.find(p=>p.player_id===id)?.name||id;node('p',`Credited touch order: ${c.credited_touch_order.map(person).join(' → ')}`,article).className='touch-order';const anchors=c.ordered_tracking_anchors;let detail=node('details','',article);detail.className='touch-anchors';node('summary',`Inspect touch-order tracking candidates (${anchors.candidate_count}${anchors.count_saturated?'+':''})`,detail);node('p','Official assists establish credited touch order. Nearest-body intervals help locate candidate contacts; they do not establish exact touches, pass types or creative importance.',detail).className='small';if(!anchors.examples.length)node('p','No complete nearest-body alignment in this clip. The official touch order is retained.',detail);for(const candidate of anchors.examples.slice(0,4))node('p',candidate.anchors.map(a=>`${person(a.player_id)} frames ${a.first_frame}–${a.last_frame}`).join(' → '),detail).className='anchor-candidate';if(anchors.candidate_count>4)node('p','Showing four alternatives in temporal enumeration order; no likelihood ranking. More are retained in JSON.',detail).className='small';node('p','Other nearby players do not establish intervening touches. Goal-frame alignment remains unverified; uncredited support can matter.',detail).className='small'}else{node('p',`Official assists: ${g.assists.map(a=>a.name||a.player_id).join(', ')||'none recorded'}.`,article);} node('p',`${r.summary.frame_count} frames · ${r.summary.unique_observed_players} observed player IDs; active membership unverified.`,article).className='small';if(r.documentary_annotation){const a=r.documentary_annotation;node('p',`Documentary hypothesis: ${a.candidate_mechanism}`,article).className='status';node('p',`NHL description: ${a.documentary_basis}`,article);node('p',`Still unknown: ${a.unresolved}`,article).className='small';for(const source of a.sources)link(article,source.id,source.url)}else{node('p','Mechanism: unassessed — review required.',article).className='status';}let ul=node('ul','',article);for(const q of r.annotation.questions)node('li',q,ul);link(article,'Open native IceLines sequence',`http://127.0.0.1:8786/goal-analysis?target=${r.target.game_id}/${r.target.event_id}`);link(article,'Official goal replay',r.source_links.visualizer);link(article,'Official video',r.source_links.video)}}$('player').onchange=render;$('status').onchange=render;render()})();</script></html>"##;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_cases_are_in_denominator_not_finish_counts() {
        let records = vec![
            json!({"selection":{"player":{"name":"Synthetic"}},"status":"captured","summary":{"goal":{"strength":"pp","shot_type":"tip-in","assists":[],"period_type":"OT"},"frame_count":5,"goalkeeper_context":"unknown"},"observed_players":[1,2]}),
            json!({"selection":{"player":{"name":"Synthetic"}},"status":"unavailable"}),
        ];
        let s = summarize(&records);
        let p = &s["players"][0];
        assert_eq!(s["planned_cases"], 2);
        assert_eq!(s["captured_cases"], 1);
        assert_eq!(p["unavailable_cases"], 1);
        assert_eq!(p["recorded_shot_type_counts"]["tip-in"], 1);
        assert_eq!(p["goalkeeper_context_counts"]["unknown"], 1);
        assert_eq!(p["overtime_goals"], 1);
        assert_eq!(p["unique_observed_player_ids"], 2);
    }
}
