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
                }
            }
            window["opposition_sequence_summary"]=sequences;
            window["goalie_marker_at_comparison_start"]=record["goalie_geometry"]["frames"][start].clone();
            window["goalie_marker_at_comparison_end"]=record["goalie_geometry"]["frames"][end].clone();
        }
        record["goal_event_windows"]=windows;
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
function showOpposition(r,article){const g=r.opposition_geometry;if(!g)return;const d=node('details','',article);d.className='opposition-geometry';node('summary','Opposing skaters: puck route and scorer separation',d);node('p',g.interpretation||g.reason,d).className='small';if(g.status==='unavailable')return;const label=node('label','Adjacent sample interval ',d),select=node('select','',label);for(const row of g.intervals){const o=node('option',`${row.from_frame_index} → ${row.to_frame_index}`,select);o.value=row.from_frame_index}const body=node('div','',d);function update(){body.replaceChildren();const row=g.intervals[Number(select.value)];if(!row)return;let table=node('table','',body);let head=node('tr','',table);for(const h of ['Opponent','Puck route minimum*','Scorer at start','Scorer at end'])node('th',h,head);const ft=n=>n==null?'Unknown':n.toFixed(1)+' ft';for(const m of row.markers){let tr=node('tr','',table);node('td',m.name||m.player_id,tr);if(m.status!=='available'){let td=node('td',m.status.replaceAll('_',' '),tr);td.colSpan=3;continue}node('td',ft(m.puck_marker_minimum_separation_feet_linear_derived),tr);node('td',ft(m.scorer_marker_separation_start_feet_derived),tr);node('td',ft(m.scorer_marker_separation_end_feet_derived),tr)}node('p','*Model minimum with both puck and opponent moving linearly between samples. Endpoint scorer separation is a different measurement. Proximity does not establish responsibility or contact.',body).className='small'}select.onchange=update;update()}
function showMarkerMotion(p,parent){const m=p.same_identity_motion;if(!m)return;const d=node('details','',parent);d.className='marker-motion';node('summary','Who moved? Compare the same players',d);node('p',m.interpretation,d).className='small';if(m.status!=='observed_endpoint_comparison'){node('p',`Unavailable: ${(m.reason||'unknown').replaceAll('_',' ')}`,d);return}const ft=n=>n==null?'unknown':n.toFixed(1)+' ft';node('p',`Scorer marker net displacement: ${ft(m.scorer_marker_displacement_feet_derived)}. This is the straight-line endpoint distance, not distance travelled.`,d).className='scorer-motion';const nearestIds=new Set([...(p.nearest_to_scorer_start?.markers||[]),...(p.nearest_to_scorer_end?.markers||[])].map(v=>v.player_id));const row=(v,parent)=>{const text=v.status==='available'?`${v.name||v.player_id}: opponent net displacement ${ft(v.marker_displacement_feet_derived)}; same-pair separation ${ft(v.scorer_separation_start_feet_derived)} → ${ft(v.scorer_separation_end_feet_derived)}.`:`${v.name||v.player_id}: ${v.status.replaceAll('_',' ')}.`;node('p',text,parent).className='same-pair-motion'};node('p','Opponents nearest the scorer at either endpoint:',d).className='small';const selected=m.markers.filter(v=>nearestIds.has(v.player_id));if(!selected.length)node('p','No eligible nearest opponent identified.',d);for(const v of selected)row(v,d);const others=m.markers.filter(v=>!nearestIds.has(v.player_id));if(others.length){const more=node('details','',d);node('summary','Other observed opposing skaters',more);for(const v of others)row(v,more)}}
function showSequenceSummary(c,person,parent){const s=c.opposition_sequence_summary;if(!s?.examples.length)return;const d=node('details','',parent);d.className='sequence-summary';node('summary','Read the candidate sequence: scorer space and puck route',d);node('p',s.interpretation,d).className='small';const ft=n=>n==null?'unknown':n.toFixed(1)+' ft';const nearest=n=>!n?.markers?.length?'Unknown':`${n.markers.map(m=>m.name||m.player_id).join(' / ')} ${ft(n.distance_feet_derived)}${n.status==='partial_observed_markers'?' (partial observations)':''}`;for(const [i,e] of s.examples.slice(0,2).entries()){node('h4',`Alignment example ${i+1} of ${s.alignment_candidate_count} candidate(s)`,d);for(const p of e.phases){const label=p.kind==='actor_candidate_interval'?`${person(p.actor_id)} candidate interval`:`Between ${person(p.from_actor_id)} and ${person(p.to_actor_id)} candidates`;const box=node('div','',d);box.className='sequence-phase';node('strong',`${label}: ${p.from_frame_index} → ${p.to_frame_index}`,box);if(p.status!=='sample_comparison'){node('p','Sample comparison unavailable.',box);continue}node('p',`Nearest observed opponent to scorer: ${nearest(p.nearest_to_scorer_start)} → ${nearest(p.nearest_to_scorer_end)}.`,box);const route=p.puck_route_minimum_linear;if(route.encounters?.length)node('p',`Puck-route model minimum: ${ft(route.distance_feet_derived)}, ${route.encounters.map(m=>`${m.name||m.player_id} during ${m.from_frame_index} → ${m.to_frame_index}`).join('; ')}${route.status==='partial_observed_markers'?' (partial observations)':''}.`,box);else node('p','Puck-route comparison unavailable for this interval.',box);showMarkerMotion(p,box);if(p.same_nearest_marker_set_at_endpoints===false)node('p','Nearest opponent changes; the separation change is not one opponent’s movement.',box).className='small'}}if(s.stored_example_count>2)node('p','First two retained examples shown; remaining stored examples are in JSON. No likelihood ranking.',d).className='small'}
function showEventWindows(r,article){const w=r.goal_event_windows;if(!w)return;const detail=node('details','',article);detail.className='event-windows';node('summary',w.status==='unavailable'?'Timing reference unavailable':`Timing: ${w.candidate_count} candidate 2D net-opening crossing${w.candidate_count===1?'':'s'}`,detail);node('p','These are adjacent-sample reference-plane brackets. Puck height, full puck volume and true between-sample path are unknown; no candidate is confirmed as the goal. Repeated later crossings are retained.',detail).className='small';if(w.status==='unavailable'){node('p',w.reason,detail);return}if(!w.candidates.length){node('p','No qualifying sampled crossing. The official goal and touch order remain recorded.',detail);node('p',`Diagnostic: ${(w.missing_candidate_reason||'unknown').replaceAll('_',' ')}. Closest sampled distance to the plane: ${w.minimum_absolute_sample_plane_distance_feet_derived==null?'unavailable':w.minimum_absolute_sample_plane_distance_feet_derived.toFixed(2)+' ft (derived)'}. Forward encounters outside the opening: ${w.outside_opening_forward_encounters.length}. The reference criteria are unchanged.`,detail).className='crossing-diagnostic';} const person=id=>r.credited_contributions.participants.find(p=>p.player_id===id)?.name||id;for(const c of w.candidates.slice(0,4)){node('p',`Frames ${c.from_frame_index} → ${c.to_frame_index}: forward through the opening reference.`,detail).className='crossing-candidate';const a=c.pre_crossing_credited_anchors;node('p',`${a.candidate_count} ordered nearest-body alignment(s) in the prefix ending at frame ${c.from_frame_index}.`,detail);for(const chain of a.examples.slice(0,2))node('p',chain.anchors.map(p=>`${person(p.player_id)} ${p.first_frame}–${p.last_frame}`).join(' → '),detail).className='pre-crossing-chain';showSequenceSummary(c,person,detail);const k0=c.goalie_marker_at_comparison_start,k1=c.goalie_marker_at_comparison_end;if(k0?.status==='available'&&k1?.status==='available')node('p',`Recorded goalie marker, frames ${c.comparison_start_frame_index} → ${c.comparison_end_frame_index}: depth ${k0.goalie_marker_depth_feet_derived.toFixed(1)} → ${k1.goalie_marker_depth_feet_derived.toFixed(1)} ft; reference offset ${k0.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} → ${k1.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} ft. This comparison is not a reaction-time or positioning grade.`,detail).className='small'}if(w.candidates.length>4)node('p','Showing the first four in sample order; remaining candidates are retained in JSON.',detail).className='small'}
function render(){$('cases').replaceChildren();for(const r of d.records){let s=r.selection;if($('player').value&&$('player').value!==s.player.name)continue;if($('status').value&&$('status').value!==r.status)continue;let article=node('article','',$('cases'));article.dataset.status=r.status;node('h2',`${s.player.name} · ${s.game_date}`,article);node('p',`Scoring-game rank ${s.scoring_game_rank} · game ${s.game_id}`,article).className='small';if(r.status!=='captured'){article.className='bad';node('p',`Unavailable: ${r.reason}`,article);continue}let g=r.summary.goal;node('p',`${r.summary.away_team} at ${r.summary.home_team} · P${g.period} ${g.time_in_period} · ${g.strength||'unknown strength'} · ${g.shot_type||'unknown shot type'} · ${r.summary.goalkeeper_context}`,article);node('h3','Scoring credits and contributions',article);if(r.credited_contributions){let dl=node('dl','',article);dl.className='contributors';for(const p of r.credited_contributions.participants){let dt=node('dt',`${p.official_credit} · ${p.name||p.player_id}`,dl);dt.dataset.credit=p.official_credit;let description=p.action_status==='documentary-supported'?`${p.action} · attributed description (${p.action_source_ids.join(', ')})`:'Action unassessed — official credit alone does not identify the task.';node('dd',description,dl)}const c=r.credited_contributions,person=id=>c.participants.find(p=>p.player_id===id)?.name||id;if(r.goal_geometry?.status==='available')node('p',`Attacking goal: ${r.goal_geometry.attacking_goal_side}. Fixed renderer reference; distances below use a regulation-derived scale.`,article).className='small';node('p',`Credited touch order: ${c.credited_touch_order.map(person).join(' → ')}`,article).className='touch-order';const anchors=c.ordered_tracking_anchors;let detail=node('details','',article);detail.className='touch-anchors';node('summary',`Inspect touch-order tracking candidates (${anchors.candidate_count}${anchors.count_saturated?'+':''})`,detail);node('p','Official assists establish credited touch order. Nearest-body intervals help locate candidate contacts; they do not establish exact touches, pass types or creative importance.',detail).className='small';if(!anchors.examples.length)node('p','No complete nearest-body alignment in this clip. The official touch order is retained.',detail);for(const candidate of anchors.examples.slice(0,4)){node('p',candidate.anchors.map(a=>`${person(a.player_id)} frames ${a.first_frame}–${a.last_frame}`).join(' → '),detail).className='anchor-candidate';if(r.goal_geometry?.status==='available'){let list=node('ul','',detail);list.className='goal-relative-anchors';for(const a of candidate.anchors){const first=r.goal_geometry.frames[a.first_frame],last=r.goal_geometry.frames[a.last_frame];if(first?.status!=='available'||last?.status!=='available')continue;node('li',`${person(a.player_id)} candidate interval: puck ${first.distance_to_goal_center_feet_derived.toFixed(1)} → ${last.distance_to_goal_center_feet_derived.toFixed(1)} ft from goal; bearing ${first.goal_relative_bearing_degrees?.toFixed(1)??'undefined'} → ${last.goal_relative_bearing_degrees?.toFixed(1)??'undefined'}°. These are interval endpoints, not verified contact/release locations.`,list);const keeper=r.goalie_geometry;if(keeper?.status==='recorded_goalie_reference'){const k0=keeper.frames[a.first_frame],k1=keeper.frames[a.last_frame];if(k0?.status==='available'&&k1?.status==='available')node('li',`${keeper.recorded_goalie_name||keeper.recorded_goalie_id} marker during that interval: depth ${k0.goalie_marker_depth_feet_derived.toFixed(1)} → ${k1.goalie_marker_depth_feet_derived.toFixed(1)} ft; sideways offset from puck–net-centre reference ${k0.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} → ${k1.goalie_marker_offset_from_puck_goal_line_feet_derived.toFixed(1)} ft. Marker geometry only; the reference line moves with the puck.`,list).className='goalie-reference'}}}} if(anchors.candidate_count>4)node('p','Showing four alternatives in temporal enumeration order; no likelihood ranking. More are retained in JSON.',detail).className='small';node('p','Goalie marker depth/offset do not establish facing, body coverage, skate depth, readiness or optimal position. Other nearby players do not establish intervening touches. Goal-frame alignment remains unverified; uncredited support can matter.',detail).className='small'}else{node('p',`Official assists: ${g.assists.map(a=>a.name||a.player_id).join(', ')||'none recorded'}.`,article);} showEventWindows(r,article);showOpposition(r,article);node('p',`${r.summary.frame_count} frames · ${r.summary.unique_observed_players} observed player IDs; active membership unverified.`,article).className='small';if(r.documentary_annotation){const a=r.documentary_annotation;node('p',`Documentary hypothesis: ${a.candidate_mechanism}`,article).className='status';node('p',`NHL description: ${a.documentary_basis}`,article);node('p',`Still unknown: ${a.unresolved}`,article).className='small';for(const source of a.sources)link(article,source.id,source.url)}else{node('p','Mechanism: unassessed — review required.',article).className='status';}let ul=node('ul','',article);for(const q of r.annotation.questions)node('li',q,ul);link(article,'Open native IceLines sequence',`http://127.0.0.1:8786/goal-analysis?target=${r.target.game_id}/${r.target.event_id}`);link(article,'Official goal replay',r.source_links.visualizer);link(article,'Official video',r.source_links.video)}}$('player').onchange=render;$('status').onchange=render;render()})();</script></html>"##;

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
