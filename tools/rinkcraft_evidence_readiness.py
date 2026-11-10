"""Join sealed local statistics to Rinkcraft facet contracts without grading players."""
from __future__ import annotations
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import subprocess
from rinkcraft_attribute_contracts import load,validate

SCHEMA='icelines.attribute-readiness.v1'

def sealed(root,entry):
    p=(root/entry['path']).resolve();raw=p.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=entry['sha256']: raise ValueError(f'Source hash mismatch: {p.name}')
    if 'bytes' in entry and len(raw)!=entry['bytes']: raise ValueError('Source byte count mismatch')
    return p

def read_rows(path,season,kind,fields):
    result={};situation='5on5' if kind=='skaters' else 'all'
    with path.open(encoding='utf-8-sig',newline='') as f:
        reader=csv.DictReader(f)
        missing=set(fields)-set(reader.fieldnames or [])
        if missing: raise ValueError(f'Missing context fields: {sorted(missing)}')
        for r in reader:
            if int(r['season'])!=season//10000: raise ValueError('Season mismatch')
            if r['situation']!=situation: continue
            pid=int(r['playerId'])
            if pid in result: raise ValueError('Duplicate player-season-situation; do not sum traded totals')
            values={}
            for field in fields:
                text=r[field];value=None if text=='' else float(text)
                if value is not None and (not math.isfinite(value) or value<0): raise ValueError('Invalid context count')
                values[field]=value
            result[pid]={'values':values,'situation':situation}
    return result

def build(registry,config,pilot,tables,edge,edge_season):
    facets={c['id']:(a,c) for a in registry['attributes'] for c in a['components']}
    mapping={m['component_id']:m for m in config['facets']}
    if len(mapping)!=len(config['facets']) or set(mapping)!=set(facets): raise ValueError('Exact unique current facet mapping required')
    for cid,m in mapping.items():
        if m['attribute_id']!=facets[cid][0]['id']: raise ValueError('Facet/attribute mismatch')
        expected={'SHO-FINISH':'experimental_partial_estimate','SKA-SPEED':'historical_raw_context','SHO-VELOCITY':'historical_raw_context'}.get(cid,'not_collected')
        if m['available_kind']!=expected: raise ValueError('Unknown or unsupported evidence adapter')
    if config.get('next_steps'):
        planned={cid for step in config['next_steps'] for cid in step['facets']}
        if planned!=set(facets):raise ValueError('Acquisition plan must cover the exact current facet registry')
        if any(not step.get(k) for step in config['next_steps'] for k in ('priority','name','request','acceptance','current_blocker')):raise ValueError('Incomplete acquisition step')
    if pilot.get('headline_scores_released') is not False: raise ValueError('Pilot must not release headlines')
    seasons=sorted({s for s,k in tables});profiles=[];seen=set()
    for p in pilot['profiles']:
        pid=p['player_id']
        if pid in seen: raise ValueError('Duplicate selected player')
        seen.add(pid);branch='goalie' if p['position']=='G' else 'skater';kind='goalies' if branch=='goalie' else 'skaters'
        windows=[]
        for season in seasons:
            source=tables.get((season,kind),{});row=source.get(pid);attributes=[]
            for a in registry['attributes']:
                if a['branch']!=branch: continue
                cs=[]
                for c in a['components']:
                    m=mapping[c['id']];state='not_collected';measurement=None
                    if c['id']=='SHO-FINISH':
                        f=p.get('components',{}).get('finishing',{})
                        if f.get('latest_season')==season and row is not None:
                            measurement=f.get('latest_season_evidence')
                            if measurement is not None: state=measurement.get('status','not_collected')
                    elif edge and pid==edge['player']['id'] and season==edge_season:
                        if c['id']=='SKA-SPEED':
                            state='historical_raw_context';measurement={'maximum_mph':edge['skatingSpeed']['speedMax']['imperial'],'provider_percentile':edge['skatingSpeed']['speedMax']['percentile'],'bursts_over_20_mph':edge['skatingSpeed']['burstsOver20']['value'],'scope':'historical season aggregate; manpower unspecified; provider percentile, not Rinkcraft calibration'}
                        elif c['id']=='SHO-VELOCITY':
                            state='historical_raw_context';measurement={'maximum_shot_mph':edge['topShotSpeed']['imperial'],'provider_percentile':edge['topShotSpeed']['percentile'],'scope':'historical season aggregate; shot-type/target and manpower unresolved'}
                    cs.append({'id':c['id'],'label':c['label'],'state':state,'measurement':measurement,'validated':False,'inference_limit':m['inference_limit'],'next_required_inputs':c['inputs'],'disconfirming_test':c['disconfirming_test']})
                context={field:None if row is None else row['values'][field] for field in config['attribute_context'][a['id']]}
                attributes.append({'id':a['id'],'name':a['name'],'headline_score':None,'validated_facets':0,'required_facets':len(cs),'adjacent_outcomes':context,'context_scope':None if row is None else row['situation'],'context_limit':'Adjacent outcomes only. Counts lack the eligible task/context denominator and cannot establish this capability.','facets':cs})
            windows.append({'season':season,'source_row_present':row is not None,'attributes':attributes})
        profiles.append({'player_id':pid,'name':p['name'],'position':p['position'],'windows':windows})
    return {'schema':SCHEMA,'definition_version':registry['version'],'policy':config['policy'],'production_grades_released':False,'profiles':profiles,'facet_map':config['facets'],'next_steps':config.get('next_steps',[]),'source_credit':pilot.get('source_credit','Synthetic test data'),'source_limits':['Uses only verified local historical inputs, not current-season data.','5on5 skater context and all-strength goalie context remain distinct; goalie context cannot calibrate a 5v5 battery.','Historical provider percentiles and experimental finishing levels are not validated attribute grades.','No actual player facet has passed the complete measurement validation gates.']}

def render(result):
    # Escape script delimiters while preserving an exact JSON round trip.
    payload=json.dumps(result,ensure_ascii=False).replace("<", "\\u003c").replace("&", "\\u0026")
    return '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft evidence workbench</title><style>body{max-width:1160px;margin:auto;padding:24px;background:#eef3f6;color:#142e40;font:15px/1.5 system-ui;overflow-wrap:anywhere}h1{border-bottom:5px solid #b8394d}header,.controls,article{background:white;border:1px solid #a2b5bf;padding:16px;margin-bottom:14px}.controls{display:flex;gap:16px;flex-wrap:wrap}label{display:flex;gap:8px;align-items:center;flex-wrap:wrap}select{max-width:100%;font:inherit;padding:6px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,470px),1fr));gap:14px}article{min-width:0;border-top:4px solid #24798f}h2{margin:0}.status{display:flex;justify-content:space-between;gap:10px}small{color:#71404d}summary{cursor:pointer;padding:7px 0;font-weight:650}.facet{border-top:1px solid #c6d4db;padding:8px 0}pre{white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.5 ui-monospace}dl{display:grid;grid-template-columns:1fr auto;gap:4px 16px}dd{margin:0}a{color:#17637a}*{box-sizing:border-box}</style><h1>Rinkcraft • Evidence workbench</h1><header><p><strong>Statistics → facet evidence → validated capability.</strong> Inspect the evidence chain for a player and season. Every headline remains Unrated.</p><p>0 validated facets is a research status, never a statement that a player has zero skill. More evidence does not mechanically raise ability.</p><p id="credit"></p><details><summary>Next evidence to collect</summary><pre id="plan"></pre></details><a href="../attributes/index.html">Attribute definitions</a> · <a href="../capabilities/index.html">Original finishing pilot</a> · <a href="readiness.json">Full evidence inventory</a></header><div class="controls"><label>Player <select id="player"></select></label><label>Season <select id="season"></select></label></div><p id="scope"></p><main id="cards"></main><details><summary>Source limits and provenance</summary><pre id="limits"></pre></details><script type="application/json" id="payload">'''+payload+'''</script><script>
const data=JSON.parse(document.getElementById('payload').textContent);
const $=id=>document.getElementById(id);$('credit').textContent=data.source_credit+' Historical tracking: NHL EDGE.';$('plan').textContent=JSON.stringify(data.next_steps,null,2);const node=(tag,text)=>{const e=document.createElement(tag);e.textContent=text;return e};
for(const p of data.profiles){const o=node('option',`${p.name} · ${p.position}`);o.value=p.player_id;$('player').append(o)}
for(const w of data.profiles[0].windows){const o=node('option',`${String(w.season).slice(0,4)}–${String(w.season).slice(4)}`);o.value=w.season;$('season').append(o)}$('season').value=data.profiles[0].windows.at(-1).season;
const labels={not_collected:'Not collected',experimental_component:'Experimental partial estimate',insufficient_exposure:'Insufficient exposure',historical_raw_context:'Historical raw measurement'};
function draw(){const p=data.profiles.find(p=>String(p.player_id)===$('player').value);const w=p.windows.find(w=>String(w.season)===$('season').value);$('cards').replaceChildren();$('scope').textContent=`${p.name} · ${p.position} · ${$('season').selectedOptions[0].textContent} · ${w.source_row_present?'Source row present':'No source row — missing is not zero'}`;
for(const a of w.attributes){const card=node('article','');card.dataset.attribute=a.id;const top=node('div','');top.className='status';top.append(node('h2',a.name),node('strong','Unrated'));card.append(top,node('small',`0 of ${a.required_facets} facets validated`));
const context=node('details','');context.append(node('summary','Adjacent outcome statistics'));context.append(node('p',a.context_limit+` Source scope: ${a.context_scope??'missing'}.`));const dl=node('dl','');for(const [key,value] of Object.entries(a.adjacent_outcomes)){dl.append(node('dt',key),node('dd',value===null?'—':Number.isInteger(value)?String(value):value.toFixed(2)))}if(!Object.keys(a.adjacent_outcomes).length)context.append(node('p','No defensible summary-field adapter for this attribute.'));context.append(dl);card.append(context);
for(const f of a.facets){const d=node('details','');d.className='facet';d.dataset.facet=f.id;d.append(node('summary',`${f.label} · ${labels[f.state]??f.state}`));d.append(node('p',f.inference_limit));if(f.measurement)d.append(node('pre',JSON.stringify(f.measurement,null,2)));d.append(node('strong','Next required inputs'));const ul=node('ul','');for(const x of f.next_required_inputs)ul.append(node('li',x));d.append(ul,node('p',`Challenge: ${f.disconfirming_test}`));card.append(d)}$('cards').append(card)}
$('limits').textContent=JSON.stringify({source_limits:data.source_limits,provenance:data.provenance},null,2)}$('player').addEventListener('change',draw);$('season').addEventListener('change',draw);draw();
</script></html>'''

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--config',required=True,type=Path);parser.add_argument('--output',required=True,type=Path);args=parser.parse_args()
    config=load(args.config);root=(args.config.parent/config['data_root']).resolve()
    if config.get('schema')!='icelines.attribute-readiness-config.v1':raise ValueError('Unknown config schema')
    inputs={k:sealed(root,e) for k,e in config['inputs'].items()};registry=load(inputs['registry']);errors=validate(registry)
    if errors: raise ValueError(errors)
    pilot_config=load(inputs['pilot_config']);pilot=load(inputs['pilot_result'])
    if hashlib.sha256(inputs['pilot_config'].read_bytes()).hexdigest()!=pilot['config_sha256']:raise ValueError('Pilot/config lineage mismatch')
    tables={};sources=[]
    for entry in pilot_config['sources']:
        path=sealed(root,entry);kind=entry['kind'];branch='goalie' if kind=='goalies' else 'skater'
        fields=sorted({f for a in registry['attributes'] if a['branch']==branch for f in config['attribute_context'][a['id']]})
        key=(entry['season'],kind)
        if key in tables:raise ValueError('Duplicate source season/kind')
        tables[key]=read_rows(path,entry['season'],kind,fields);sources.append({**entry,'observed_rows':len(tables[key])})
    capture=pilot_config['edge_capture'];manifest_path=inputs['edge_manifest'];manifest=load(manifest_path)
    if manifest['schema']!='icelines.edge-capture.v1' or len(manifest['records'])!=1:raise ValueError('Invalid saved capture manifest')
    record=manifest['records'][0];edge_path=sealed(root,dict(record,path=capture['path']));edge=load(edge_path)
    if edge['player']['id']!=record['player_id']:raise ValueError('Capture identity mismatch')
    result=build(registry,config,pilot,tables,edge,manifest['season'])
    result['provenance']={'config_sha256':hashlib.sha256(args.config.read_bytes()).hexdigest(),'inputs':config['inputs'],'sources':sources,'edge_manifest_sha256':hashlib.sha256(manifest_path.read_bytes()).hexdigest(),'edge_source':record,'edge_season':manifest['season'],'implementation_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'ice_lines_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True,cwd=Path(__file__).parent).strip()}
    args.output.mkdir(parents=True,exist_ok=True);(args.output/'readiness.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');(args.output/'index.html').write_text(render(result),encoding='utf-8')
    print(json.dumps({'players':len(result['profiles']),'seasons':len(result['profiles'][0]['windows']),'facets':len(config['facets']),'production_grades_released':False}))
if __name__=='__main__':main()
