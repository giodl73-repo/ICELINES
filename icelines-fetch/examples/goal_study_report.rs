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
        record["goal_geometry"]=icelines_fetch::goal_geometry::assess(&report["audit"]["events"][0],&report["summary"]);
        record["goalie_geometry"]=icelines_fetch::goalie_geometry::assess(&report["audit"]["events"][0],&record["goal_geometry"]);
        let event=&report["audit"]["events"][0];
        record["opposition_geometry"]=icelines_fetch::opposition_geometry::assess(event,&record["goal_geometry"]);
        let mut windows=icelines_fetch::goal_event_windows::assess(event,&record["goal_geometry"]);
        let order:Vec<_>=record["credited_contributions"]["credited_touch_order"].as_array().context("credited touch order")?.iter().filter_map(Value::as_u64).collect();
        for window in windows["candidates"].as_array_mut().context("candidate windows")? {
            let end=window["from_frame_index"].as_u64().unwrap() as usize;
            let start=window["comparison_start_frame_index"].as_u64().unwrap() as usize;
            window["pre_crossing_credited_anchors"]=contributors::ordered_anchors(&event["frames"].as_array().context("frames")?[..=end],&order);
            let mut sequences=icelines_fetch::goal_sequence_summary::assess(&record["opposition_geometry"],&window["pre_crossing_credited_anchors"]);
            for example in sequences["examples"].as_array_mut().context("sequence examples")? {
                for phase in example["phases"].as_array_mut().context("sequence phases")? {
                    let first=phase["from_frame_index"].as_u64().context("phase start")? as usize;
                    let last=phase["to_frame_index"].as_u64().context("phase end")? as usize;
                    phase["same_identity_motion"]=icelines_fetch::goal_marker_motion::assess(event,&record["goal_geometry"],first,last);
                    phase["same_identity_motion"]["goalie_endpoint_reference"]=json!({
                        "status":record["goalie_geometry"]["status"],"recorded_goalie_id":record["goalie_geometry"]["recorded_goalie_id"],
                        "recorded_goalie_name":record["goalie_geometry"]["recorded_goalie_name"],
                        "start_status":record["goalie_geometry"]["frames"][first]["status"],"end_status":record["goalie_geometry"]["frames"][last]["status"],
                        "start":record["goalie_geometry"]["frames"][first]["goalie_marker"],"end":record["goalie_geometry"]["frames"][last]["goalie_marker"]});
                }
            }
            window["opposition_sequence_summary"]=sequences;
            window["goalie_marker_at_comparison_start"]=record["goalie_geometry"]["frames"][start].clone();
            window["goalie_marker_at_comparison_end"]=record["goalie_geometry"]["frames"][end].clone();
        }
        record["goal_event_windows"]=windows;
        let last=event["frames"].as_array().context("full source frames")?.len().checked_sub(1).context("empty source clip")?;
        let scene=icelines_fetch::goal_sample_scene::assess(event,0,last);
        let history=icelines_fetch::goal_puck_distance_history::assess(&scene,&record["goal_geometry"]);
        let payload=json!({"schema":"icelines.full-puck-sequence.v0.1","target":record["target"],"frame_count":last+1,
            "candidate_windows":record["goal_event_windows"]["candidates"].as_array().context("windows")?.iter().map(|w|json!({"from_frame_index":w["from_frame_index"],"to_frame_index":w["to_frame_index"]})).collect::<Vec<_>>(),
            "exact_goal_frame_verified":false,"distance_history":history,
            "phase":{"from_frame_index":0,"to_frame_index":last,"same_identity_motion":{
                "snapshot_only":true,"show_all_observed":true,"hide_proximity":true,"markers":[],"puck_samples_native":[],
                "synchronized_scene":scene,"target_goal_reference_native":record["goal_geometry"]["target_goal_reference"],
                "goalie_endpoint_reference":{"status":record["goalie_geometry"]["status"],"recorded_goalie_id":record["goalie_geometry"]["recorded_goalie_id"],"recorded_goalie_name":record["goalie_geometry"]["recorded_goalie_name"]}}}});
        let relative=format!("full-sequence/{}-{}.json",record["target"]["game_id"].as_u64().context("game ID")?,record["target"]["event_id"].as_u64().context("event ID")?);
        let path=PathBuf::from(&args[1]).join(&relative);
        icelines_fetch::atomic_write::write_json_atomic(&path,&payload)?;
        let payload_bytes=std::fs::read(&path)?;
        record["full_puck_sequence"]=json!({"schema":"icelines.full-puck-sequence-reference.v0.1","file":relative,"sha256":icelines_fetch::goal_visualizer::sha256_hex(&payload_bytes),"frame_count":last+1});

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
    let html = HTML.replace("__FULL_SEQUENCE_SCRIPT__", include_str!("support/goal_full_puck_sequence.js")).replace("__STUDY_DATA__", &payload);
    std::fs::write(out.join("index.html"), html)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
const HTML: &str = r##"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>IceLines · Successful-goal study</title>
<style>body{font:15px system-ui;background:#eff3f7;color:#18304a;margin:0}main{max-width:1200px;margin:auto;padding:24px}h1{margin:0}header,article{background:white;border:1px solid #cad7e3;border-radius:8px;padding:18px;margin-bottom:14px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,340px),1fr));gap:14px}article{margin:0}h2{font-size:18px}p{line-height:1.5}.small{font-size:12px;color:#52687a}label{display:inline-block;margin:12px 20px 12px 0}select{padding:8px;font:inherit}a{color:#145386}.bad{border-left:5px solid #ac6720}table{width:100%;border-collapse:collapse;font-size:13px}td,th{text-align:left;padding:8px;border-bottom:1px solid #dce5ed}.scroll{overflow:auto}.touch-order{font-size:13px;font-weight:bold}.touch-anchors{border-top:1px solid #dce5ed;padding-top:8px}.anchor-candidate{font-size:13px}.contributors dt{font-weight:bold;margin-top:8px;text-transform:capitalize}.contributors dd{margin:3px 0 8px;font-size:13px}.status{font-weight:bold;color:#6b510c}.motion-rink-svg{display:block;width:100%;height:240px;border:1px solid #cad7e3;background:#fff;margin:10px 0}.rink-controls{display:flex;flex-wrap:wrap;gap:8px}.rink-controls label{min-width:0}.rink-controls select{max-width:100%}.rink-legend{display:flex;flex-wrap:wrap;gap:6px 14px;font-size:12px;font-weight:600}.motion-rink{margin:10px 0}.snapshot-controls{display:flex;flex-wrap:wrap;align-items:center;gap:6px;margin:10px 0}.sample-proximity[hidden],.snapshot-controls[hidden],.snapshot-observations[hidden],.rink-controls [hidden]{display:none}.snapshot-controls .sample-meta{flex-basis:100%;margin:0}.sample-proximity{margin:12px 0;padding:10px 0;border-top:1px solid #cad7e3}.sample-proximity p{margin:6px 0}.sample-goalie{background:#f4f0f8}.sample-proximity th,.sample-proximity td{font-size:12px;padding:5px}.snapshot-controls button{padding:6px}.snapshot-controls select{max-width:100%}summary{cursor:pointer}.full-puck-sequence{min-width:0;margin:12px 0}.distance-history-scroll{max-width:100%;overflow-x:auto;border:1px solid #cad7e3;margin:8px 0}.distance-history-svg{display:block;width:100%;height:auto}.all-player-distances td,.all-player-distances th{font-size:12px;padding:5px}.full-puck-sequence .motion-rink{border-top:1px solid #cad7e3;padding-top:8px}.full-puck-sequence>label{display:block;min-width:0;max-width:100%}.sequence-end{display:block;width:100%;max-width:100%;box-sizing:border-box}.snapshot-controls{min-width:0;max-width:100%}.snapshot-controls label{min-width:0;max-width:100%;margin-right:0}.distance-history-layout{display:grid;grid-template-columns:130px minmax(0,1fr);border:1px solid #cad7e3;margin:8px 0;min-width:0}.history-labels-svg{display:block;width:130px;box-sizing:border-box;border-right:1px solid #cad7e3;background:white}.distance-history-layout .distance-history-scroll{border:0;margin:0}</style>
<main><header><p class="small">ICELINES · RINKCRAFT EXPLORATORY STUDY · 2023–24 REGULAR SEASON</p><h1>How do these goals happen?</h1><p id="coverage"></p><p>Official-context and observation pass. Selected cards include attributed NHL descriptions and provisional mechanisms. Video/release alignment and independent review remain incomplete. Scoring outcomes alone do not identify a missed assignment or player capability.</p><div class="scroll"><table><thead><tr><th>Player</th><th>Captured / selected</th><th>Strength</th><th>Recorded shot type</th></tr></thead><tbody id="summary"></tbody></table></div><label>Player <select id="player"><option value="">All players</option></select></label><label>Status <select id="status"><option value="">All cases</option><option value="captured">Captured</option><option value="unavailable">Unavailable</option></select></label><p class="small">Each case links to native IceLines analysis and the official source. Counted cases are successful goals selected by scoring-game rank, not a denominator of attempts or a representative sample of a player's goals.</p></header><div id="cases" class="grid"></div></main>
<script type="application/json" id="data">__STUDY_DATA__</script><script>(()=>{const d=JSON.parse(document.getElementById('data').textContent),$=id=>document.getElementById(id);const node=(tag,text,parent)=>{let n=document.createElement(tag);n.textContent=text;parent.append(n);return n};const counts=o=>Object.entries(o).map(([k,v])=>`${k}: ${v}`).join(', ');$('coverage').textContent=`${d.summary.planned_cases} planned cases · ${d.summary.captured_cases} captured · ${d.summary.frames} frames`;
for(const p of d.summary.players){let tr=node('tr','',$('summary'));for(const text of [p.player,`${p.captured_cases} / ${p.planned_cases}`,counts(p.strength_counts),counts(p.recorded_shot_type_counts)])node('td',text,tr);let o=node('option',p.player,$('player'));o.value=p.player}
function link(parent,text,url){try{let u=new URL(url);if(u.origin!=='http://127.0.0.1:8786'&&!(u.protocol==='https:'&&['www.nhl.com','nhl.com','wsr.nhle.com'].includes(u.hostname)))return;let a=node('a',text,parent);a.href=u.href;a.target='_blank';a.rel='noopener';parent.append(document.createElement('br'))}catch{}}
function showOpposition(r,article){const g=r.opposition_geometry;if(!g)return;const d=node('details','',article);d.className='opposition-geometry';node('summary','Opposing skaters: puck route and scorer separation',d);node('p',g.interpretation||g.reason,d).className='small';if(g.status==='unavailable')return;const label=node('label','Adjacent sample interval ',d),select=node('select','',label);for(const row of g.intervals){const o=node('option',`${row.from_frame_index} → ${row.to_frame_index}`,select);o.value=row.from_frame_index}const body=node('div','',d);function update(){body.replaceChildren();const row=g.intervals[Number(select.value)];if(!row)return;let table=node('table','',body);let head=node('tr','',table);for(const h of ['Opponent','Puck route minimum*','Scorer at start','Scorer at end'])node('th',h,head);const ft=n=>n==null?'Unknown':n.toFixed(1)+' ft';for(const m of row.markers){let tr=node('tr','',table);node('td',m.name||m.player_id,tr);if(m.status!=='available'){let td=node('td',m.status.replaceAll('_',' '),tr);td.colSpan=3;continue}node('td',ft(m.puck_marker_minimum_separation_feet_linear_derived),tr);node('td',ft(m.scorer_marker_separation_start_feet_derived),tr);node('td',ft(m.scorer_marker_separation_end_feet_derived),tr)}node('p','*Model minimum with both puck and opponent moving linearly between samples. Endpoint scorer separation is a different measurement. Proximity does not establish responsibility or contact.',body).className='small'}select.onchange=update;update()}
__FULL_SEQUENCE_SCRIPT__
function showMotionRink(p,parent){
 const m=p.same_identity_motion;if(!m||(!m.scorer_marker_endpoints_native&&!m.synchronized_scene?.frames?.length))return;
 const detail=node('details','',parent);detail.className='motion-rink';node('summary',m.snapshot_only?'Rink positions · selected source sample':'View rink positions: samples and endpoints',detail);
 let initialized=false,receiveSample=null,pendingSample=null;const api={element:detail,setSample(index){pendingSample=String(index);detail.open=true;if(receiveSample)receiveSample(pendingSample)}};
 detail.ontoggle=()=>{if(!detail.open||initialized)return;initialized=true;
  const scene=m.synchronized_scene,sceneFrames=scene?.frames||[];
  const endpointEligible=m.markers.filter(v=>v.status==='available'&&v.marker_endpoints_native),choiceMap=new Map(endpointEligible.map(v=>[v.player_id,v]));
  for(const frame of sceneFrames)for(const v of frame.players)if(v.identity_status==='recorded_id'&&Number.isInteger(v.player_id)&&v.team&&scene.scoring_team&&v.team!==scene.scoring_team&&v.goalie===false&&!choiceMap.has(v.player_id))choiceMap.set(v.player_id,m.markers.find(o=>o.player_id===v.player_id)||{player_id:v.player_id,name:v.name});
  const eligible=[...choiceMap.values()];
  const controls=node('div','',detail);controls.className='rink-controls';
  const modeLabel=node('label','Mode ',controls),mode=node('select','',modeLabel);mode.className='rink-mode';modeLabel.hidden=!!m.snapshot_only;
  for(const [value,text] of [['sample','Same-sample snapshot'],['endpoints','Endpoint comparison']]){const option=node('option',text,mode);option.value=value;if(value==='endpoints'&&m.snapshot_only)option.disabled=true}
  if(!sceneFrames.length)mode.value='endpoints';
  const syncControls=node('div','',detail);syncControls.className='snapshot-controls';
  const previous=node('button','Previous sample',syncControls);previous.type='button';previous.className='rink-previous';
  const sampleLabel=node('label','Sample ',syncControls),sampleSelect=node('select','',sampleLabel);sampleSelect.className='rink-sample';
  for(const frame of sceneFrames){const option=node('option',String(frame.frame_index),sampleSelect);option.value=frame.frame_index}
  const next=node('button','Next sample',syncControls);next.type='button';next.className='rink-next';
  const sampleMeta=node('p','',syncControls);sampleMeta.className='sample-meta small';
  const othersLabel=node('label','',controls),othersToggle=node('input','',othersLabel);othersToggle.type='checkbox';othersToggle.checked=!!m.show_all_observed;othersToggle.className='rink-other-players';othersLabel.append(document.createTextNode(' Other observed players'));
  const label=node('label','Compare opponent ',controls),opponent=node('select','',label);opponent.className='rink-opponent';
  for(const v of eligible){const option=node('option',v.name||v.player_id,opponent);option.value=v.player_id}
  const nearest=[...(p.nearest_to_scorer_start?.markers||[]),...(p.nearest_to_scorer_end?.markers||[])];
  const preferred=nearest.find(n=>eligible.some(v=>v.player_id===n.player_id));if(preferred)opponent.value=preferred.player_id;
  if(!eligible.length){const option=node('option','No complete opponent endpoints',opponent);option.value='';opponent.disabled=true}
  const viewLabel=node('label','View ',controls),view=node('select','',viewLabel);view.className='rink-view';
  for(const [value,text] of [['focus','Focus on comparison'],['full','Full rink reference']]){const option=node('option',text,view);option.value=value}
  const samplesLabel=node('label','',controls),samplesToggle=node('input','',samplesLabel);samplesToggle.type='checkbox';samplesToggle.checked=true;samplesToggle.className='rink-puck-samples';samplesLabel.append(document.createTextNode(' Puck samples'));
  const numbersLabel=node('label','',controls),numbersToggle=node('input','',numbersLabel);numbersToggle.type='checkbox';numbersToggle.className='rink-sample-numbers';numbersLabel.append(document.createTextNode(' Sample numbers'));
  const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('role','img');svg.setAttribute('aria-label',`Observed endpoints, samples ${p.from_frame_index} and ${p.to_frame_index}`);svg.classList.add('motion-rink-svg');detail.append(svg);
  const legend=node('p','',detail);legend.className='rink-legend';
  for(const [text,color] of [['Scorer','#1764aa'],['Selected opponent','#ad4f00'],['Puck','#242d38'],['Recorded goalie','#8051aa']]){const span=node('span',text,legend);span.style.color=color}
  const caption=node('p',`Open circle = sample ${p.from_frame_index}; filled circle = sample ${p.to_frame_index}. Arrows connect endpoints only. Puck dots are recorded samples; no in-between path is drawn. Skater and goalie markers remain phase endpoints, not positions at each puck sample. The dashed connector, when shown, joins endpoints only and is not a pass or shot trajectory. Other players are omitted; active personnel are unverified. Goal line is the derived target reference; no body, stick, facing or puck height is shown.`,detail);caption.className='rink-caption small';
  const endpointCaption=caption.textContent;
  const status=node('p','',detail);status.className='rink-status small';
  const proximityPanel=node('div','',detail);proximityPanel.className='sample-proximity';
  const roster=node('details','',detail);roster.className='snapshot-observations';const rosterHeading=node('summary','Observed player list',roster),rosterList=node('ul','',roster);
  const valid=v=>v&&Number.isFinite(v.x)&&Number.isFinite(v.y);
  function draw(){
   svg.replaceChildren();const chosen=eligible.find(v=>String(v.player_id)===opponent.value),keeper=m.goalie_endpoint_reference;
   let groups=[['scorer',m.scorer_marker_endpoints_native,'#1764aa','S'],['opponent',chosen?.marker_endpoints_native,'#ad4f00','O'],['puck',m.puck_endpoints_native,'#242d38','P'],['goalie',keeper,'#8051aa','G']];
   const isSample=mode.value==='sample',frame=sceneFrames.find(v=>String(v.frame_index)===sampleSelect.value);
   syncControls.hidden=!isSample;othersLabel.hidden=!isSample;samplesToggle.disabled=isSample;numbersToggle.disabled=isSample;samplesLabel.hidden=isSample;numbersLabel.hidden=isSample;
   roster.hidden=!isSample;rosterHeading.textContent=`Observed players at sample ${frame?.frame_index??'unknown'} (${frame?.players.length??0} observations)`;rosterList.replaceChildren();
   if(isSample)for(const row of frame?.players||[]){const repeated=row.player_id!=null&&frame.players.filter(v=>v.player_id===row.player_id).length>1;node('li',`${row.source_observation_index+1}. ${row.name||row.player_id||'Unknown identity'} · ${row.team||'unknown team'} · ${row.goalie===true?'recorded goalie':row.goalie===false?'recorded skater':'role unknown'} · ${row.coordinate_status==='observed'?'coordinates observed':'coordinates unavailable'}${repeated?' · duplicate source ID':''}`,rosterList)}
   proximityPanel.hidden=!isSample||!!m.hide_proximity;proximityPanel.replaceChildren();
   if(isSample&&!m.hide_proximity){
    const proximity=m.synchronized_proximity,row=proximity?.frames?.find(v=>v.frame_index===frame?.frame_index);
    node('strong',`Observed opponent separations · source sample ${frame?.frame_index??'unknown'}`,proximityPanel);
    if(!row)node('p',`Unavailable: ${(proximity?.reason||'same-sample measurements unavailable').replaceAll('_',' ')}`,proximityPanel);
    else{
     const closest=n=>!n?.markers?.length?'Unavailable':`${n.markers.map(v=>v.name||v.player_id).join(' / ')} · ${n.distance_feet_derived.toFixed(1)} ft*${n.status==='partial_observed_skater_set'?' (partial observed set)':''}`;
     node('p',`Nearest observed skater to puck: ${closest(row.nearest_puck_skater)}`,proximityPanel).className='nearest-puck';
     node('p',`Nearest observed skater to scorer: ${closest(row.nearest_scorer_skater)}`,proximityPanel).className='nearest-scorer';
     const table=node('table','',proximityPanel),head=node('tr','',table);
     for(const h of ['Observed opponent','Puck ft*','Scorer ft*'])node('th',h,head);
     for(const r of row.opponents){const tr=node('tr','',table);tr.dataset.playerId=r.player_id??'unknown';tr.dataset.observationIndex=r.source_observation_index;if(r.goalie===true)tr.className='sample-goalie';node('td',`${r.name||r.player_id||'Unknown identity'}${r.goalie===true?' · goalie':r.goalie==null?' · role unknown':''}`,tr);for(const key of ['puck_separation_feet_derived','scorer_separation_feet_derived'])node('td',r[key]==null?'Unknown':r[key].toFixed(1),tr);if(r.status!=='available')tr.title=r.status.replaceAll('_',' ')}
     node('p',`*Feet use the regulation-derived reference. Point separations are not contact, reach, pressure or coverage grades. Observed goalies are listed but excluded from skater minima. Active membership unverified. Puck: ${row.puck_status.replaceAll('_',' ')}; scorer: ${row.scorer_status.replaceAll('_',' ')}.`,proximityPanel).className='small';
    }
   }
   previous.disabled=sampleSelect.selectedIndex<=0;next.disabled=sampleSelect.selectedIndex<0||sampleSelect.selectedIndex>=sceneFrames.length-1;
   sampleMeta.textContent=frame?`Source sample ${frame.frame_index} · native tick ${frame.elapsed_native_ticks??'unknown'} (not calibrated seconds)`:'';
   caption.textContent=isSample?'Every shown position comes from this one published source sample. No interpolation or position carry-forward. Source clock/physical synchronization, active personnel, possession, contact timing, body/stick orientation and height remain unverified. Other observed players can be shown as context. The goalie identity is recorded at the goal event, not verified occupancy at this sample.':endpointCaption;
   const sameId=(id,predicate)=>{if(id==null||!frame)return null;const rows=frame.players.filter(v=>v.player_id===id);return rows.length===1&&rows[0].identity_status==='recorded_id'&&predicate(rows[0])?rows[0]:null};
   const own=v=>!!scene?.scoring_team&&v.team===scene.scoring_team,opposing=v=>!!scene?.scoring_team&&!!v.team&&v.team!==scene.scoring_team;
   const scorerRow=sameId(scene?.scorer_id,own),opponentRow=sameId(chosen?.player_id,v=>opposing(v)&&v.goalie===false),goalieRow=sameId(keeper?.recorded_goalie_id,v=>opposing(v)&&v.goalie===true);
   const selectedRows=[scorerRow,opponentRow,goalieRow].filter(Boolean);
   const otherRows=isSample&&othersToggle.checked?(frame?.players||[]).filter(v=>!selectedRows.includes(v)&&valid(v.marker)):[];
   if(isSample)groups=[['scorer',{current:scorerRow?.marker},'#1764aa','S'],['opponent',{current:opponentRow?.marker},'#ad4f00','O'],['puck',{current:frame?.puck.marker},'#242d38','P'],['goalie',{current:goalieRow?.marker},'#8051aa','G']];
   svg.setAttribute('aria-label',isSample?`Same source sample ${frame?.frame_index??'unknown'}`:`Observed endpoints, samples ${p.from_frame_index} and ${p.to_frame_index}`);
   const samples=m.puck_samples_native||[];const observedSamples=samples.filter(v=>v.status==='observed'&&valid(v.marker));
   const points=(isSample?groups.map(([,ends])=>ends.current):groups.flatMap(([,ends])=>[ends?.start,ends?.end])).filter(valid);points.push(...otherRows.map(v=>v.marker));const goal=m.target_goal_reference_native;if(valid(goal))points.push(goal);if(!isSample&&samplesToggle.checked)points.push(...observedSamples.map(v=>v.marker));
   let xmin=points.length?Math.min(...points.map(v=>v.x))-96:0,xmax=points.length?Math.max(...points.map(v=>v.x))+96:2400,ymin=points.length?Math.min(...points.map(v=>v.y))-96:0,ymax=points.length?Math.max(...points.map(v=>v.y))+96:1020;
   if(view.value==='full'){xmin=Math.min(xmin,0);xmax=Math.max(xmax,2400);ymin=Math.min(ymin,0);ymax=Math.max(ymax,1020)}
   const width=Math.max(480,xmax-xmin),height=Math.max(360,ymax-ymin),cx=(xmin+xmax)/2,cy=(ymin+ymax)/2;
   svg.setAttribute('viewBox',`${cx-width/2} ${cy-height/2} ${width} ${height}`);
   const make=(tag,attrs,text)=>{const el=document.createElementNS('http://www.w3.org/2000/svg',tag);for(const [k,v] of Object.entries(attrs))el.setAttribute(k,v);if(text!=null)el.textContent=text;svg.append(el);return el};
   make('title',{},isSample?`Source sample ${frame?.frame_index??'unknown'}: ${chosen?.name||'unknown opponent'}`:`Same-player endpoints: scorer versus ${chosen?.name||'unknown opponent'}`);
   make('rect',{x:0,y:0,width:2400,height:1020,fill:'#f4faff',stroke:'#a7bac8','stroke-width':2,'vector-effect':'non-scaling-stroke'});
   make('line',{x1:1200,y1:0,x2:1200,y2:1020,stroke:'#d79098','stroke-width':1,'vector-effect':'non-scaling-stroke'});
   const radius=width/62,font=width/35;
   if(valid(goal)){make('line',{x1:goal.x,y1:goal.y-36,x2:goal.x,y2:goal.y+36,stroke:'#ad2834','stroke-width':4,'vector-effect':'non-scaling-stroke','data-role':'target-reference'});make('text',{x:goal.x+(goal.x>1200?-radius:radius),y:goal.y+font*1.8,fill:'#8c2530','font-size':font,'text-anchor':goal.x>1200?'end':'start'},'Goal ref')}
   for(const row of otherRows){const el=make('circle',{cx:row.marker.x,cy:row.marker.y,r:radius*.65,fill:'#c0cad3',stroke:row.goalie===true?'#8051aa':'#66798b','stroke-width':1,'vector-effect':'non-scaling-stroke','data-role':'other-observed','data-observation-index':row.source_observation_index,'data-frame-index':frame.frame_index,'data-player-id':row.player_id??'unknown','data-goalie':row.goalie??'unknown'});const title=document.createElementNS('http://www.w3.org/2000/svg','title');title.textContent=`${row.name||row.player_id||'Unknown identity'} · ${row.team||'unknown team'} · ${row.goalie===true?'recorded goalie':row.goalie===false?'recorded skater':'role unknown'}; active occupancy unverified`;el.append(title);make('text',{x:row.marker.x+radius*.8,y:row.marker.y-radius*.7,fill:'#52687a','font-size':font*.8},row.source_observation_index+1)}
   for(const [role,ends,color,letter] of groups){
    if(!ends)continue;
    if(isSample){const v=ends.current;if(valid(v)){const el=make('circle',{cx:v.x,cy:v.y,r:radius,fill:color,stroke:color,'stroke-width':2,'vector-effect':'non-scaling-stroke','data-role':role,'data-endpoint':'current','data-frame-index':frame.frame_index});const title=document.createElementNS('http://www.w3.org/2000/svg','title');title.textContent=`${role}, source sample ${frame.frame_index}, native (${v.x.toFixed(1)}, ${v.y.toFixed(1)})`;el.append(title);make('text',{x:v.x+radius*1.25,y:v.y-radius,fill:color,'font-size':font,'font-weight':600},letter)}continue}
    const a=ends.start,b=ends.end;
    if(valid(a)&&valid(b)){
     const attrs={x1:a.x,y1:a.y,x2:b.x,y2:b.y,stroke:color,'stroke-width':2,'vector-effect':'non-scaling-stroke','data-role':role,'data-kind':'endpoint-connector'};
     if(role==='puck')attrs['stroke-dasharray']='5 4';if(role!=='puck'||!samplesToggle.checked)make('line',attrs);
     const dx=b.x-a.x,dy=b.y-a.y,len=Math.hypot(dx,dy);if((role!=='puck'||!samplesToggle.checked)&&len>radius*2){const ux=dx/len,uy=dy/len,tip={x:b.x-ux*radius,y:b.y-uy*radius},base={x:tip.x-ux*radius*1.4,y:tip.y-uy*radius*1.4};make('polygon',{points:`${tip.x},${tip.y} ${base.x-uy*radius*.6},${base.y+ux*radius*.6} ${base.x+uy*radius*.6},${base.y-ux*radius*.6}`,fill:color})}
    }
    for(const [endpoint,v] of [['start',a],['end',b]])if(valid(v)){
     const el=make('circle',{cx:v.x,cy:v.y,r:radius,fill:endpoint==='start'?'white':color,stroke:color,'stroke-width':2,'vector-effect':'non-scaling-stroke','data-role':role,'data-endpoint':endpoint});
     const title=document.createElementNS('http://www.w3.org/2000/svg','title');title.textContent=`${role} ${endpoint}, native (${v.x.toFixed(1)}, ${v.y.toFixed(1)})`;el.append(title);
     make('text',{x:v.x+radius*1.25,y:v.y+(endpoint==='start'?-radius:font),fill:color,'font-size':font,'font-weight':600},letter+(endpoint==='start'?'0':'1'));
    }
   }
   if(!isSample&&samplesToggle.checked)for(const sample of observedSamples){
    const v=sample.marker;const circle=make('circle',{cx:v.x,cy:v.y,r:radius*.38,fill:'#242d38','data-role':'puck-sample','data-frame-index':sample.frame_index});
    const title=document.createElementNS('http://www.w3.org/2000/svg','title');title.textContent=`Recorded puck sample ${sample.frame_index}, native (${v.x.toFixed(1)}, ${v.y.toFixed(1)})`;circle.append(title);
    if(numbersToggle.checked)make('text',{x:v.x+radius*.55,y:v.y-radius*.6,fill:'#242d38','font-size':font*.8,'data-role':'puck-sample-label'},sample.frame_index);
   }
   if(isSample){
    const absent=groups.filter(([,ends])=>!valid(ends.current)).map(([role])=>role);
    status.textContent=`Source sample ${frame?.frame_index??'unavailable'}; ${frame?.players.length??0} published player observations, active membership unverified. Selected opponent: ${chosen?.name||'unavailable'}${absent.includes('opponent')?' (not available with verified context at this sample; no fallback)':''}. ${absent.includes('goalie')?'Event-recorded opposing goalie reference unavailable at this sample; no occupancy inference.':'Event-recorded goalie marker observed at this sample.'} ${absent.filter(v=>v!=='opponent'&&v!=='goalie').length?'Unavailable: '+absent.filter(v=>v!=='opponent'&&v!=='goalie').join(', ')+'.':''}`;
    detail.dispatchEvent(new CustomEvent('rink-source-sample',{detail:{frame_index:frame?.frame_index}}));
    return;
   }
   const missing=groups.flatMap(([role,ends])=>['start','end'].filter(key=>!valid(ends?.[key])).map(key=>`${role} ${key}`));
   status.textContent=`Puck samples: ${observedSamples.length}/${samples.length} coordinates available; ${samplesToggle.checked?'dots shown':'dots hidden'}. ${endpointEligible.length}/${m.markers.length} opponent comparisons have complete endpoints. Selected opponent: ${chosen?.name||'unavailable'}. ${keeper?.status==='inapplicable_empty_net'?'Recorded empty net; goalie endpoints inapplicable.':missing.some(v=>v.startsWith('goalie'))?'Recorded goalie endpoint geometry incomplete.':'Recorded goalie markers shown.'} ${missing.filter(v=>!v.startsWith('goalie')).length?'Other unavailable endpoints: '+missing.filter(v=>!v.startsWith('goalie')).join(', ')+'.':''}`;
  }
  opponent.onchange=draw;view.onchange=draw;samplesToggle.onchange=draw;numbersToggle.onchange=draw;mode.onchange=draw;sampleSelect.onchange=draw;othersToggle.onchange=draw;previous.onclick=()=>{sampleSelect.selectedIndex=Math.max(0,sampleSelect.selectedIndex-1);draw()};next.onclick=()=>{sampleSelect.selectedIndex=Math.min(sceneFrames.length-1,sampleSelect.selectedIndex+1);draw()};receiveSample=value=>{if([...sampleSelect.options].some(o=>o.value===String(value))){mode.value='sample';sampleSelect.value=String(value);draw()}};if(pendingSample!=null)receiveSample(pendingSample);else draw();
 };
 return api;
}
function showMarkerMotion(p,parent){const m=p.same_identity_motion;if(!m)return;const d=node('details','',parent);d.className='marker-motion';node('summary','Who moved? Compare the same players',d);node('p',m.interpretation,d).className='small';if(m.status!=='observed_endpoint_comparison'){node('p',`Unavailable: ${(m.reason||'unknown').replaceAll('_',' ')}`,d);return}showMotionRink(p,d);const ft=n=>n==null?'unknown':n.toFixed(1)+' ft';node('p',`Scorer marker net displacement: ${ft(m.scorer_marker_displacement_feet_derived)}. This is the straight-line endpoint distance, not distance travelled.`,d).className='scorer-motion';const nearestIds=new Set([...(p.nearest_to_scorer_start?.markers||[]),...(p.nearest_to_scorer_end?.markers||[])].map(v=>v.player_id));const row=(v,parent)=>{const text=v.status==='available'?`${v.name||v.player_id}: opponent net displacement ${ft(v.marker_displacement_feet_derived)}; same-pair separation ${ft(v.scorer_separation_start_feet_derived)} → ${ft(v.scorer_separation_end_feet_derived)}.`:`${v.name||v.player_id}: ${v.status.replaceAll('_',' ')}.`;node('p',text,parent).className='same-pair-motion'};node('p','Opponents nearest the scorer at either endpoint:',d).className='small';const selected=m.markers.filter(v=>nearestIds.has(v.player_id));if(!selected.length)node('p','No eligible nearest opponent identified.',d);for(const v of selected)row(v,d);const others=m.markers.filter(v=>!nearestIds.has(v.player_id));if(others.length){const more=node('details','',d);node('summary','Other observed opposing skaters',more);for(const v of others)row(v,more)}}
function showSequenceSummary(c,person,parent){const s=c.opposition_sequence_summary;if(!s?.examples.length)return;const d=node('details','',parent);d.className='sequence-summary';node('summary','Read the candidate sequence: scorer space and puck route',d);node('p',s.interpretation,d).className='small';const ft=n=>n==null?'unknown':n.toFixed(1)+' ft';const nearest=n=>!n?.markers?.length?'Unknown':`${n.markers.map(m=>m.name||m.player_id).join(' / ')} ${ft(n.distance_feet_derived)}${n.status==='partial_observed_markers'?' (partial observations)':''}`;for(const [i,e] of s.examples.slice(0,2).entries()){node('h4',`Alignment example ${i+1} of ${s.alignment_candidate_count} candidate(s)`,d);for(const p of e.phases){const label=p.kind==='actor_candidate_interval'?`${person(p.actor_id)} candidate interval`:`Between ${person(p.from_actor_id)} and ${person(p.to_actor_id)} candidates`;const box=node('div','',d);box.className='sequence-phase';node('strong',`${label}: ${p.from_frame_index} → ${p.to_frame_index}`,box);if(p.status!=='sample_comparison'){node('p','Sample comparison unavailable.',box);continue}node('p',`Nearest observed opponent to scorer: ${nearest(p.nearest_to_scorer_start)} → ${nearest(p.nearest_to_scorer_end)}.`,box);const route=p.puck_route_minimum_linear;if(route.encounters?.length)node('p',`Puck-route model minimum: ${ft(route.distance_feet_derived)}, ${route.encounters.map(m=>`${m.name||m.player_id} during ${m.from_frame_index} → ${m.to_frame_index}`).join('; ')}${route.status==='partial_observed_markers'?' (partial observations)':''}.`,box);else node('p','Puck-route comparison unavailable for this interval.',box);showMarkerMotion(p,box);if(p.same_nearest_marker_set_at_endpoints===false)node('p','Nearest opponent changes; the separation change is not one opponent’s movement.',box).className='small'}}if(s.stored_example_count>2)node('p','First two retained examples shown; remaining stored examples are in JSON. No likelihood ranking.',d).className='small'}
function showEventWindows(r,article){const w=r.goal_event_windows;if(!w)return;const detail=node('details','',article);detail.className='event-windows';node('summary',w.status==='unavailable'?'Timing reference unavailable':`Timing: ${w.candidate_count} candidate 2D net-opening crossing${w.candidate_count===1?'':'s'}`,detail);node('p','These are adjacent-sample reference-plane brackets. Puck height, full puck volume and true between-sample path are unknown; no candidate is confirmed as the goal. Repeated later crossings are retained.',detail).className='small';if(w.status==='unavailable'){node('p',w.reason,detail);return}if(!w.candidates.length){node('p','No qualifying sampled crossing. The official goal and touch order remain recorded.',detail);node('p',`Diagnostic: ${(w.missing_candidate_reason||'unknown').replaceAll('_',' ')}. Closest sampled distance to the plane: ${w.minimum_absolute_sample_plane_distance_feet_derived==null?'unavailable':w.minimum_absolute_sample_plane_distance_feet_derived.toFixed(2)+' ft (derived)'}. Forward encounters outside the opening: ${w.outside_opening_forward_encounters.length}. The reference criteria are unchanged.`,detail).className='crossing-diagnostic';} const person=id=>r.credited_contributions.participants.find(p=>p.player_id===id)?.name||id;for(const c of w.candidates.slice(0,4)){node('p',`Frames ${c.from_frame_index} → ${c.to_frame_index}: forward through the opening reference.`,detail).className='crossing-candidate';const a=c.pre_crossing_credited_anchors;node('p',`${a.candidate_count} ordered nearest-body alignment(s) in the prefix ending at frame ${c.from_frame_index}.`,detail);for(const chain of a.examples.slice(0,2))node('p',chain.anchors.map(p=>`${person(p.player_id)} ${p.first_frame}–${p.last_frame}`).join(' → '),detail).className='pre-crossing-chain';showSequenceSummary(c,person,detail);const k0=c.goalie_marker_at_comparison_start,k1=c.goalie_marker_at_comparison_end;if(k0?.status==='available'&&k1?.status==='available')node('p',`Recorded goalie marker, frames ${c.comparison_start_frame_index} → ${c.comparison_end_frame_index}: depth ${k0.goalie_marker_depth_feet_derived.toFixed(1)} → ${k1.goalie_marker_depth_feet_derived.toFixed(1)} ft; reference offset ${k0.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} → ${k1.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} ft. This comparison is not a reaction-time or positioning grade.`,detail).className='small'}if(w.candidates.length>4)node('p','Showing the first four in sample order; remaining candidates are retained in JSON.',detail).className='small'}
function render(){$('cases').replaceChildren();for(const r of d.records){let s=r.selection;if($('player').value&&$('player').value!==s.player.name)continue;if($('status').value&&$('status').value!==r.status)continue;let article=node('article','',$('cases'));article.dataset.status=r.status;node('h2',`${s.player.name} · ${s.game_date}`,article);node('p',`Scoring-game rank ${s.scoring_game_rank} · game ${s.game_id}`,article).className='small';if(r.status!=='captured'){article.className='bad';node('p',`Unavailable: ${r.reason}`,article);continue}let g=r.summary.goal;node('p',`${r.summary.away_team} at ${r.summary.home_team} · P${g.period} ${g.time_in_period} · ${g.strength||'unknown strength'} · ${g.shot_type||'unknown shot type'} · ${r.summary.goalkeeper_context}`,article);showFullPuckSequence(r,article);node('h3','Scoring credits and contributions',article);if(r.credited_contributions){let dl=node('dl','',article);dl.className='contributors';for(const p of r.credited_contributions.participants){let dt=node('dt',`${p.official_credit} · ${p.name||p.player_id}`,dl);dt.dataset.credit=p.official_credit;let description=p.action_status==='documentary-supported'?`${p.action} · attributed description (${p.action_source_ids.join(', ')})`:'Action unassessed — official credit alone does not identify the task.';node('dd',description,dl)}const c=r.credited_contributions,person=id=>c.participants.find(p=>p.player_id===id)?.name||id;if(r.goal_geometry?.status==='available')node('p',`Attacking goal: ${r.goal_geometry.attacking_goal_side}. Fixed renderer reference; distances below use a regulation-derived scale.`,article).className='small';node('p',`Credited touch order: ${c.credited_touch_order.map(person).join(' → ')}`,article).className='touch-order';const anchors=c.ordered_tracking_anchors;let detail=node('details','',article);detail.className='touch-anchors';node('summary',`Inspect touch-order tracking candidates (${anchors.candidate_count}${anchors.count_saturated?'+':''})`,detail);node('p','Official assists establish credited touch order. Nearest-body intervals help locate candidate contacts; they do not establish exact touches, pass types or creative importance.',detail).className='small';if(!anchors.examples.length)node('p','No complete nearest-body alignment in this clip. The official touch order is retained.',detail);for(const candidate of anchors.examples.slice(0,4)){node('p',candidate.anchors.map(a=>`${person(a.player_id)} frames ${a.first_frame}–${a.last_frame}`).join(' → '),detail).className='anchor-candidate';if(r.goal_geometry?.status==='available'){let list=node('ul','',detail);list.className='goal-relative-anchors';for(const a of candidate.anchors){const first=r.goal_geometry.frames[a.first_frame],last=r.goal_geometry.frames[a.last_frame];if(first?.status!=='available'||last?.status!=='available')continue;node('li',`${person(a.player_id)} candidate interval: puck ${first.distance_to_goal_center_feet_derived.toFixed(1)} → ${last.distance_to_goal_center_feet_derived.toFixed(1)} ft from goal; bearing ${first.goal_relative_bearing_degrees?.toFixed(1)??'undefined'} → ${last.goal_relative_bearing_degrees?.toFixed(1)??'undefined'}°. These are interval endpoints, not verified contact/release locations.`,list);const keeper=r.goalie_geometry;if(keeper?.status==='recorded_goalie_reference'){const k0=keeper.frames[a.first_frame],k1=keeper.frames[a.last_frame];if(k0?.status==='available'&&k1?.status==='available')node('li',`${keeper.recorded_goalie_name||keeper.recorded_goalie_id} marker during that interval: depth ${k0.goalie_marker_depth_feet_derived.toFixed(1)} → ${k1.goalie_marker_depth_feet_derived.toFixed(1)} ft; sideways offset from puck–net-centre reference ${k0.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} → ${k1.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} ft. Marker geometry only; the reference line moves with the puck.`,list).className='goalie-reference'}}}} if(anchors.candidate_count>4)node('p','Showing four alternatives in temporal enumeration order; no likelihood ranking. More are retained in JSON.',detail).className='small';node('p','Goalie marker depth/offset do not establish facing, body coverage, skate depth, readiness or optimal position. Other nearby players do not establish intervening touches. Goal-frame alignment remains unverified; uncredited support can matter.',detail).className='small'}else{node('p',`Official assists: ${g.assists.map(a=>a.name||a.player_id).join(', ')||'none recorded'}.`,article);} showEventWindows(r,article);showOpposition(r,article);node('p',`${r.summary.frame_count} frames · ${r.summary.unique_observed_players} observed player IDs; active membership unverified.`,article).className='small';if(r.documentary_annotation){const a=r.documentary_annotation;node('p',`Documentary hypothesis: ${a.candidate_mechanism}`,article).className='status';node('p',`NHL description: ${a.documentary_basis}`,article);node('p',`Still unknown: ${a.unresolved}`,article).className='small';for(const source of a.sources)link(article,source.id,source.url)}else{node('p','Mechanism: unassessed — review required.',article).className='status';}let ul=node('ul','',article);for(const q of r.annotation.questions)node('li',q,ul);link(article,'Open native IceLines sequence',`http://127.0.0.1:8786/goal-analysis?target=${r.target.game_id}/${r.target.event_id}`);link(article,'Official goal replay',r.source_links.visualizer);link(article,'Official video',r.source_links.video)}}$('player').onchange=render;$('status').onchange=render;render()})();</script></html>"##;

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
