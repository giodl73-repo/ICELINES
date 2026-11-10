"""Exploratory temporal checks on already sealed annual finishing data. No network or grades."""
from __future__ import annotations
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import numpy as np
import scipy
from scipy.stats import gamma,betaprime
import rinkcraft_capability as legacy

def load(p):return json.loads(Path(p).read_text(encoding='utf-8-sig'))
def sealed(root,entry):
    p=(root/entry['path']).resolve();raw=p.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=entry['sha256'] or ('bytes' in entry and len(raw)!=entry['bytes']):raise ValueError('Sealed input hash/length mismatch')
    return p

def validate(c):
    if c.get('schema')!='icelines.temporal-finishing-config.v1':raise ValueError('Unknown config')
    years=[c[k] for k in ('calibration_season','recent_season','target_season')]
    if len(set(years))!=3 or years!=sorted(years) or any(not isinstance(y,int) or isinstance(y,bool) or y%10000!=y//10000+1 for y in years):raise ValueError('Three chronological season IDs required')
    if c['policies']!=['league','older','recent','pooled']:raise ValueError('Fixed declared policy comparison required')
    if not 0<c['interval_probability']<1:raise ValueError('Invalid interval')
    if not isinstance(c['bootstrap_trials'],int) or isinstance(c['bootstrap_trials'],bool) or c['bootstrap_trials']<=0:raise ValueError('Invalid trial count')
    factors=c.get('prior_strength_factors',[1.])
    if not factors or 1. not in factors or len(factors)!=len(set(factors)) or any(isinstance(x,bool) or not math.isfinite(x) or x<=0 for x in factors):raise ValueError('Explicit positive unique prior-strength factors including 1 required')
    if c['meaningful_ratio']>=2:raise ValueError('Relative change threshold must be below 100%')
    for k,minimum in [('minimum_change_xg_per_season',0),('meaningful_ratio',1)]:
        if isinstance(c[k],bool) or not math.isfinite(c[k]) or c[k]<=minimum:raise ValueError('Invalid change policy')

def moments(row,prior):
    g=0 if row is None else row['goals'];x=0 if row is None else row['xg']
    return prior['shape']+g,prior['rate']+x

def estimate(row,prior,prob):
    if row is None:return None
    shape,rate=moments(row,prior);tail=(1-prob)/2
    return {'goals':row['goals'],'xg':row['xg'],'games':row['games'],'minutes':row['minutes'],'effect_mean':shape/rate,'effect_interval':gamma.ppf([tail,1-tail],shape,scale=1/rate).tolist(),'position':row['position'],'scope':'Conditional finishing multiplier, not a Shooting grade.'}

def change(older,newer,prior,c):
    if older is None or newer is None:return {'status':'missing_season','ratio_interval':None}
    if min(older['xg'],newer['xg'])<c['minimum_change_xg_per_season']:return {'status':'insufficient_exposure','ratio_interval':None}
    a,b=moments(older,prior);aa,bb=moments(newer,prior);tail=(1-c['interval_probability'])/2;scale=b/bb;factor=c['meaningful_ratio']
    lo,median,hi=map(float,betaprime.ppf([tail,.5,1-tail],aa,a,scale=scale))
    return {'status':'conditional_comparison','ratio_median':median,'ratio_interval':[lo,hi],'interval_probability':c['interval_probability'],'probability_newer_gt_older':float(betaprime.sf(1,aa,a,scale=scale)),'probability_increase_beyond_threshold':float(betaprime.sf(factor,aa,a,scale=scale)),'probability_decrease_beyond_threshold':float(betaprime.cdf(2-factor,aa,a,scale=scale)),'threshold_factor':factor,'decrease_ratio_threshold':2-factor,'position_group_changed':(older['position']=='D')!=(newer['position']=='D'),'interval_scope':'Independent season effects conditional on common fixed empirical prior and supplied xG; omits prior/provider/context uncertainty. Not a causal skill-change probability.'}

def means(pid,annual,years,prior):
    r1=annual[years[0]].get(pid);r2=annual[years[1]].get(pid)
    a,b=moments(r1,prior);aa,bb=moments(r2,prior)
    pooled=(prior['shape']+sum(r['goals'] for r in [r1,r2] if r))/(prior['rate']+sum(r['xg'] for r in [r1,r2] if r))
    return {'league':prior['mean'],'older':a/b,'recent':aa/bb,'pooled':pooled}

def evaluation(rows,forecasts,c):
    if not rows:raise ValueError('Empty conditional evaluation cohort')
    actual=np.array([r['goals'] for r in rows]);dev={k:legacy.poisson_deviance(actual,p) for k,p in forecasts.items()};metrics={k:{'mean_poisson_deviance':float(d.mean()),'mean_absolute_goal_error':float(np.abs(actual-np.array(forecasts[k])).mean()),'predicted_goals':float(sum(forecasts[k]))} for k,d in dev.items()}
    pairs=[('older','league')] if 'pooled' not in dev else [('pooled','league'),('recent','pooled'),('older','pooled')]
    rng=np.random.default_rng(c['bootstrap_seed']);indices=rng.integers(0,len(rows),size=(c['bootstrap_trials'],len(rows)));tail=(1-c['interval_probability'])/2;comparisons={}
    for left,right in pairs:
        delta=dev[left]-dev[right];comparisons[f'{left}_minus_{right}']={'mean_deviance_difference':float(delta.mean()),'paired_player_bootstrap_interval':np.quantile(delta[indices].mean(axis=1),[tail,1-tail]).tolist(),'interval_probability':c['interval_probability'],'negative_favors':left,'positive_favors':right}
    groups={}
    for group in ('F','D'):
        mask=np.array([('D' if r['position']=='D' else 'F')==group for r in rows])
        if mask.any():groups[group]={'players':int(mask.sum()),'observed_goals':int(actual[mask].sum()),'predicted_goals':{k:float(np.array(v)[mask].sum()) for k,v in forecasts.items()}}
    return {'players':len(rows),'observed_goals':int(actual.sum()),'metrics':metrics,'comparisons':comparisons,'position_checks':groups,'interval_scope':'Paired player resampling, fixed training prior/estimates and realized target xG. No game/team/temporal clustering or full source/model uncertainty.','prospective_validation_passed':False,'position_comparability_validated':False,'feature_vintage_verified':False}

def analyze(annual,identities,source_config,c):
    validate(c);y0,y1,y2=[c[k] for k in ('calibration_season','recent_season','target_season')]
    reference=[r for r in annual[y0].values() if legacy.eligible(r,source_config) and r['xg']>0]
    prior=legacy.fit_prior(reference,source_config['prior_shape_bounds'])
    cohorts={y:[r for r in annual[y].values() if legacy.eligible(r,source_config) and r['xg']>0] for y in (y1,y2)}
    forecasts={k:[] for k in c['policies']}
    for r in cohorts[y2]:
        for k,v in means(r['player_id'],annual,[y0,y1],prior).items():forecasts[k].append(v*r['xg'])
    repeat={'league':[],'older':[]}
    for r in cohorts[y1]:
        a,b=moments(annual[y0].get(r['player_id']),prior);repeat['league'].append(prior['mean']*r['xg']);repeat['older'].append(a/b*r['xg'])
    profiles=[]
    for p in identities:
        if p['position']=='G':profiles.append({'player_id':p['player_id'],'name':p['name'],'position':'G','status':'not_applicable_goalie','annual':[],'change':None});continue
        pid=p['player_id'];records=[{'season':year,'evidence':estimate(annual[year].get(pid),prior,c['interval_probability'])} for year in [y0,y1,y2]]
        profiles.append({'player_id':pid,'name':p['name'],'position':p['position'],'status':'component_only','annual':records,'change':change(annual[y1].get(pid),annual[y2].get(pid),prior,c),'headline_score':None})
    repeat_change=[change(annual[y1].get(pid),annual[y2].get(pid),prior,c) for pid in set(annual[y1])|set(annual[y2])]
    eligible_changes=[x for x in repeat_change if x['status']=='conditional_comparison']
    strength_sensitivity=[]
    for factor in c.get('prior_strength_factors',[1.]):
        altered={**prior,'shape':prior['shape']*factor,'rate':prior['rate']*factor}
        fs={k:[] for k in c['policies']}
        for row in cohorts[y2]:
            for key,value in means(row['player_id'],annual,[y0,y1],altered).items():fs[key].append(value*row['xg'])
        ev=evaluation(cohorts[y2],fs,c)
        changes=[change(annual[y1].get(pid),annual[y2].get(pid),altered,c) for pid in set(annual[y1])|set(annual[y2])]
        complete=[x for x in changes if x['status']=='conditional_comparison']
        strength_sensitivity.append({'factor':factor,'target_metrics':ev['metrics'],'recent_minus_pooled':ev['comparisons']['recent_minus_pooled'],'compared_players':len(complete),'ratio_interval_excludes_one':sum(x['ratio_interval'][0]>1 or x['ratio_interval'][1]<1 for x in complete)})
    return {'schema':'icelines.temporal-finishing.v1','protocol':c['protocol'],'prior':prior,'prior_season':y0,'change_policy':{k:c[k] for k in ('interval_probability','minimum_change_xg_per_season','meaningful_ratio')},'prior_strength_sensitivity':strength_sensitivity,'replication':{'season':y1,**evaluation(cohorts[y1],repeat,c)},'target':{'season':y2,**evaluation(cohorts[y2],forecasts,c)},'profiles':profiles,'change_population':{'compared_players':len(eligible_changes),'ratio_interval_excludes_one':sum(x['ratio_interval'][0]>1 or x['ratio_interval'][1]<1 for x in eligible_changes),'interpretation':'Descriptive model intervals, not multiple-testing-controlled discoveries or established skill changes.'},'production_grades_released':False,'empirical_capability_validation_established':False,'source_credit':'MoneyPuck.com — historical data used for local noncommercial research.','limits':c['limits']}

def run(path):
    c=load(path);validate(c);root=(path.parent/c['data_root']).resolve();sp=sealed(root,c['inputs']['source_config']);sc=load(sp);export=load(sealed(root,c['inputs']['identity_export']))
    if export['config_sha256']!=hashlib.sha256(sp.read_bytes()).hexdigest():raise ValueError('Identity/source lineage mismatch')
    annual={};sources=[]
    for source in sc['sources']:
        if source['kind']!='skaters':continue
        year=source['season']
        if year in annual:raise ValueError('Duplicate annual source')
        data=sealed(root,source);annual[year],_=legacy.parse_rows(data,year,'skaters');sources.append(source)
    required={c[k] for k in ('calibration_season','recent_season','target_season')}
    if set(annual)!=required:raise ValueError('Exact declared season sources required')
    result=analyze(annual,export['profiles'],sc,c)
    result['provenance']={'config_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'inputs':c['inputs'],'sources':sources,'implementation_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'shared_estimator_sha256':hashlib.sha256(Path(legacy.__file__).read_bytes()).hexdigest(),'ice_lines_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=Path(__file__).parent,text=True).strip(),'environment':{'numpy':np.__version__,'scipy':scipy.__version__},'bootstrap':{'seed':c['bootstrap_seed'],'trials':c['bootstrap_trials']}}
    return result

def render(result):
    payload=json.dumps(result,ensure_ascii=False).replace('<',chr(92)+'u003c').replace('&',chr(92)+'u0026')
    return '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft temporal finishing</title><style>body{max-width:1100px;margin:auto;padding:24px;background:#edf3f6;color:#163144;font:15px/1.5 system-ui;overflow-wrap:anywhere}h1{border-bottom:5px solid #b73b50}article,header{background:white;border:1px solid #9cb0bc;border-top:4px solid #277c94;margin:16px 0;padding:18px}select{font:inherit;padding:7px;max-width:100%}table{width:100%;border-collapse:collapse;font-size:14px}th,td{text-align:left;border-bottom:1px solid #cad6dd;padding:7px}pre{white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.5 Consolas,monospace}summary{cursor:pointer;font-weight:bold}.table{overflow:auto}a{color:#145e77}*{box-sizing:border-box}</style><h1>Rinkcraft • Change or noise?</h1><header><p>Real historical season aggregates; conditional finishing only. This is a retrospective component experiment, with unknown provider model vintage. Every capability grade stays Unrated.</p><p>MoneyPuck.com data · local noncommercial research · <a href="../readiness/index.html">Evidence workbench</a> · <a href="result.json">Full results and provenance</a></p></header><article><h2>Does the earlier result repeat?</h2><div id="replication"></div><h2>Recent season or pooled history?</h2><div id="target"></div><p id="comparison"></p><details><summary>Uncertainty and F/D checks</summary><pre id="evaluation"></pre></details></article><article><h2>Inspect a player</h2><label>Player <select id="player"></select></label><p id="scope"></p><p>1.00× means conversion matching the supplied xG baseline under this model. It is a finishing component, not a percentile or Shooting grade.</p><div class="table" id="annual"></div><h3>Last two seasons</h3><p id="change"></p><details><summary>Conditional change evidence</summary><pre id="change-record"></pre></details></article><article><details><summary>Study limits and source lineage</summary><pre id="limits"></pre></details></article><script type="application/json" id="payload">'''+payload+'''</script><script>
const data=JSON.parse(document.getElementById('payload').textContent);const $=x=>document.getElementById(x);const el=(t,s)=>{const e=document.createElement(t);e.textContent=s;return e};const num=(x,n=3)=>x==null?'—':x.toFixed(n);const season=y=>String(y).slice(0,4)+'–'+String(y).slice(6);const policy={league:'League baseline',older:season(data.prior_season)+' only',recent:season(data.replication.season)+' only',pooled:season(data.prior_season)+' + '+season(data.replication.season)};
function metrics(value,container){const table=el('table','');const tr=el('tr','');for(const x of ['Evidence window / policy','Mean goal-count deviance','Predicted goals'])tr.append(el('th',x));table.append(tr);for(const [name,m] of Object.entries(value.metrics)){const r=el('tr','');for(const x of [season(value.season)+' / '+policy[name],num(m.mean_poisson_deviance),num(m.predicted_goals,1)])r.append(el('td',x));table.append(r)}$(container).append(table);$(container).append(el('p','Lower deviance is better on this conditional comparison; it does not establish a validated capability.'))}metrics(data.replication,'replication');metrics(data.target,'target');const difference=data.target.comparisons.recent_minus_pooled;$('comparison').textContent='Recent minus pooled deviance: '+num(difference.mean_deviance_difference,4)+'; '+(100*difference.interval_probability)+'% paired-player interval '+difference.paired_player_bootstrap_interval.map(x=>num(x,4)).join('–')+'. Positive favors pooled history. Conditional on realized target xG and fixed training model.';$('evaluation').textContent=JSON.stringify({prior_strength_sensitivity:data.prior_strength_sensitivity,replication:data.replication.comparisons,target:data.target.comparisons,positions:data.target.position_checks},null,2);
for(const p of data.profiles){const o=el('option',p.name+' · '+p.position);o.value=p.player_id;$('player').append(o)}function draw(){const p=data.profiles.find(x=>String(x.player_id)===$('player').value);$('annual').replaceChildren();$('scope').textContent=p.name+' · '+p.position+' · Component evidence; headline Unrated';$('change-record').textContent=JSON.stringify(p.change,null,2);if(p.position==='G'){$('change').textContent='Finishing is a skater component. Goalie attributes require their own task evidence; no goalie grade is inferred.';return}const table=el('table','');const head=el('tr','');for(const x of ['Season','Goals / xG','Conditional multiplier',(100*data.change_policy.interval_probability)+'% interval'])head.append(el('th',x));table.append(head);for(const a of p.annual){const v=a.evidence;const r=el('tr','');for(const x of [season(a.season),v?v.goals+' / '+num(v.xg,2):'Missing season',v?num(v.effect_mean)+'×':'—',v?v.effect_interval.map(x=>num(x)).join('–'):'—'])r.append(el('td',String(x)));table.append(r)}$('annual').append(table);const c=p.change;if(c.status==='conditional_comparison'){$('change').textContent='Later / earlier multiplier: '+num(c.ratio_median)+'×; '+(100*c.interval_probability)+'% interval '+c.ratio_interval.map(x=>num(x)).join('–')+'. Under this fixed-prior model, probability of a >'+num(100*(c.threshold_factor-1),0)+'% increase: '+num(100*c.probability_increase_beyond_threshold,1)+'%; of a >'+num(100*(c.threshold_factor-1),0)+'% decrease: '+num(100*c.probability_decrease_beyond_threshold,1)+'%. This is not a probability of true skill change.'}else{$('change').textContent=c.status==='missing_season'?'Comparison withheld: a season is missing.':'Comparison withheld: at least one season has less than the declared '+data.change_policy.minimum_change_xg_per_season+' xG exposure threshold.'}}$('player').addEventListener('change',draw);draw();$('limits').textContent=JSON.stringify({prior:data.prior,change_population:data.change_population,limits:data.limits,provenance:data.provenance},null,2);
</script></html>'''

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--config',required=True,type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args();result=run(args.config);args.output.mkdir(parents=True,exist_ok=True);(args.output/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');(args.output/'index.html').write_text(render(result),encoding='utf-8');print(json.dumps({'prior':result['prior'],'replication':result['replication'],'target':result['target'],'change_population':result['change_population']},indent=2))
if __name__=='__main__':main()
