"""Offline F/D nuisance-baseline experiment; no capability scores or network."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import subprocess
import numpy as np
import rinkcraft_temporal_finishing as temporal
import rinkcraft_capability as legacy

POLICIES=['global_baseline','global_player','position_baseline','position_player']
def group(row):
    if row['position'] not in ('C','L','R','D'):raise ValueError('Skater position required')
    return 'D' if row['position']=='D' else 'F'

def fit(rows,source):
    reference=[r for r in rows.values() if legacy.eligible(r,source) and r['xg']>0]
    prior=legacy.fit_prior(reference,source['prior_shape_bounds']);factors={}
    for g in ('F','D'):
        subset=[r for r in reference if group(r)==g]
        if not subset or sum(r['goals'] for r in subset)<=0:raise ValueError('Positive F and D calibration support required')
        factors[g]={'factor':sum(r['goals'] for r in subset)/sum(r['xg'] for r in subset),'players':len(subset),'goals':sum(r['goals'] for r in subset),'xg':sum(r['xg'] for r in subset)}
    return {'prior':prior,'position_factors':factors}

def predict(row,history,model):
    p=model['prior'];f=model['position_factors'][group(row)]['factor'];g=sum(r['goals'] for r in history);x=sum(r['xg'] for r in history);adjusted=sum(r['xg']*model['position_factors'][group(r)]['factor'] for r in history)
    return {'global_baseline':p['mean']*row['xg'],'global_player':(p['shape']+g)/(p['rate']+x)*row['xg'],'position_baseline':f*row['xg'],'position_player':(p['shape']+g)/(p['shape']+adjusted)*f*row['xg']}

def evaluate(rows,forecasts,c):
    actual=np.array([r['goals'] for r in rows]);dev={k:legacy.poisson_deviance(actual,v) for k,v in forecasts.items()};rng=np.random.default_rng(c['bootstrap_seed']);idx=rng.integers(0,len(rows),size=(c['bootstrap_trials'],len(rows)));tail=(1-c['interval_probability'])/2;comparisons={}
    for left,right in [('position_baseline','global_baseline'),('position_player','global_player'),('position_player','position_baseline')]:
        d=dev[left]-dev[right];comparisons[left+'_minus_'+right]={'mean_difference':float(d.mean()),'paired_player_interval':np.quantile(d[idx].mean(axis=1),[tail,1-tail]).tolist(),'negative_favors':left,'interval_probability':c['interval_probability']}
    metrics={k:{'mean_deviance':float(dev[k].mean()),'predicted_goals':float(sum(v))} for k,v in forecasts.items()};groups={}
    for g in ('F','D'):
        mask=np.array([group(r)==g for r in rows])
        if mask.any():groups[g]={'players':int(mask.sum()),'observed_goals':int(actual[mask].sum()),'models':{k:{'predicted_goals':float(np.array(v)[mask].sum()),'mean_deviance':float(dev[k][mask].mean()),'relative_calibration_error':float(np.array(v)[mask].sum()/actual[mask].sum()-1) if actual[mask].sum() else None} for k,v in forecasts.items()}}
    return {'players':len(rows),'observed_goals':int(actual.sum()),'metrics':metrics,'comparisons':comparisons,'positions':groups,'position_comparability_validated':False,'prospective_validation_passed':False}

def analyze(annual,identities,source,c):
    if c['schema']!='icelines.position-finishing-config.v1' or c['policies']!=POLICIES:raise ValueError('Unknown fixed experiment')
    years=[c[k] for k in ('calibration_season','replication_season','target_season')]
    if years!=sorted(set(years)) or len(years)!=3:raise ValueError('Three chronological seasons required')
    if set(annual)!=set(years):raise ValueError('Exact declared seasons required')
    if not isinstance(c['bootstrap_trials'],int) or isinstance(c['bootstrap_trials'],bool) or c['bootstrap_trials']<=0 or not 0<c['interval_probability']<1:raise ValueError('Invalid resampling policy')
    model=fit(annual[years[0]],source);evaluations=[];records={}
    for target,history_years in [(years[1],years[:1]),(years[2],years[:2])]:
        rows=[r for r in annual[target].values() if legacy.eligible(r,source) and r['xg']>0]
        if not rows:raise ValueError('Empty evaluation cohort')
        forecasts={k:[] for k in POLICIES};players={}
        for r in rows:
            history=[annual[y][r['player_id']] for y in history_years if r['player_id'] in annual[y]];values=predict(r,history,model)
            for k,v in values.items():forecasts[k].append(v)
            players[r['player_id']]={'player_id':r['player_id'],'position':r['position'],'observed_goals':r['goals'],'target_xg':r['xg'],'predictions':values,'historical_rows':len(history),'position_group_switch':len({group(r),*[group(h) for h in history]})>1}
        evaluations.append({'season':target,'history_seasons':history_years,**evaluate(rows,forecasts,c)});records[target]=players
    profiles=[{'player_id':p['player_id'],'name':p['name'],'position':p['position'],'headline_score':None,'status':'not_applicable_goalie' if p['position']=='G' else 'conditional_model_comparison','windows':[{'season':y,'evidence':records[y].get(p['player_id'])} for y in years[1:]]} for p in identities]
    return {'schema':'icelines.position-finishing.v1','protocol':c['protocol'],'model':model,'evaluations':evaluations,'profiles':profiles,'production_grades_released':False,'empirical_capability_validation_established':False,'limits':c['limits'],'interpretation':'F/D factors adjust a count-model nuisance baseline. They are not within-position skill ranks, a validated correction to raw xG, or a causal explanation of its residuals.'}

def run(path):
    c=temporal.load(path);root=(path.parent/c['data_root']).resolve();sp=temporal.sealed(root,c['inputs']['source_config']);source=temporal.load(sp);identity=temporal.load(temporal.sealed(root,c['inputs']['identity_export']))
    if identity['config_sha256']!=hashlib.sha256(sp.read_bytes()).hexdigest():raise ValueError('Identity source lineage mismatch')
    annual={};sources=[]
    for entry in source['sources']:
        if entry['kind']!='skaters':continue
        if entry['season'] in annual:raise ValueError('Duplicate season source')
        annual[entry['season']],_=legacy.parse_rows(temporal.sealed(root,entry),entry['season'],'skaters');sources.append(entry)
    r=analyze(annual,identity['profiles'],source,c);r['provenance']={'config_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'inputs':c['inputs'],'sources':sources,'implementation_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'shared_estimator_sha256':hashlib.sha256(Path(legacy.__file__).read_bytes()).hexdigest(),'shared_loader_sha256':hashlib.sha256(Path(temporal.__file__).read_bytes()).hexdigest(),'ice_lines_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=Path(__file__).parent,text=True).strip(),'bootstrap':{'seed':c['bootstrap_seed'],'trials':c['bootstrap_trials']},'environment':{'numpy':np.__version__,'scipy':temporal.scipy.__version__}}
    return r

def render(r):
    esc=html.escape;tables=[]
    for e in r['evaluations']:
        rows=''.join('<tr><td>'+esc(g)+'</td><td>'+str(p['observed_goals'])+'</td>'+''.join('<td>'+format(p['models'][k]['predicted_goals'],'.1f')+'</td>' for k in POLICIES)+'</tr>' for g,p in e['positions'].items())
        tables.append('<article><h2>'+str(e['season'])+'</h2><p>Goals scored by skaters. All models receive realized target-season xG.</p><div class="table"><table><tr><th>Group</th><th>Observed</th>'+''.join('<th>'+k.replace('_',' ')+'</th>' for k in POLICIES)+'</tr>'+rows+'</table></div><details><summary>Errors, comparisons and conditional intervals</summary><pre>'+esc(json.dumps(e,indent=2))+'</pre></details></article>')
    latest=r['evaluations'][-1];comparison=latest['comparisons']['position_player_minus_global_player'];lo,hi=comparison['paired_player_interval']
    finding='<article><h2>Calibration improves; overall advantage remains uncertain</h2><p>Position-player minus global-player mean deviance: '+format(comparison['mean_difference'],'.5f')+'; '+str(round(comparison['interval_probability']*100))+'% paired-player interval ['+format(lo,'.5f')+', '+format(hi,'.5f')+']. Negative favors the position adjustment. The interval must be read alongside the point estimate.</p><p>A group total matching observed goals does not demonstrate accurate individual predictions. Inspect subgroup errors and the simpler position baseline too.</p></article>'
    return '<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft position calibration</title><style>body{max-width:1100px;margin:auto;padding:24px;font:16px/1.5 system-ui;background:#edf3f6;color:#163144;overflow-wrap:anywhere}h1{border-bottom:5px solid #b83d50}article{background:white;border:1px solid #9bafb9;border-top:4px solid #277d95;padding:18px;margin:18px 0}table{border-collapse:collapse;width:100%;font-size:14px}th,td{padding:8px;text-align:left;border-bottom:1px solid #cad5dd}.table{overflow:auto}pre{white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.5 monospace}summary{cursor:pointer;font-weight:bold}a{color:#175b72}</style><h1>Rinkcraft • Position and calibration</h1><p>Can an earlier-data F/D baseline explain part of the finishing residual? Exploratory conditional comparison; all capability grades stay Unrated.</p><p>MoneyPuck.com historical data · local noncommercial research · <a href="../temporal-finishing/index.html">Temporal study</a> · <a href="result.json">Full result</a></p>'+finding+''.join(tables)+'<article><h2>What the adjustment means</h2><p>'+esc(r['interpretation'])+'</p><details><summary>Frozen model, limits and provenance</summary><pre>'+esc(json.dumps({k:r[k] for k in ('model','limits','provenance')},indent=2))+'</pre></details></article></html>'

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--config',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();r=run(a.config);a.output.mkdir(parents=True,exist_ok=True);(a.output/'result.json').write_text(json.dumps(r,indent=2)+'\n',encoding='utf-8');(a.output/'index.html').write_text(render(r),encoding='utf-8');print(json.dumps({'model':r['model'],'evaluations':r['evaluations']},indent=2))
if __name__=='__main__':main()
