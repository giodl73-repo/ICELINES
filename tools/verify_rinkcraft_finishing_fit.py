"""SciPy/source oracle for the Rust prior-fitting replay.

Usage: JOB_JSON OUTPUT_DIRECTORY LEGACY_ASSESSMENTS_JSON NEW_RECEIPT_JSON
"""
import csv
import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path
import numpy as np
from scipy.stats import gamma

job_path, output, legacy_path, receipt = map(Path, sys.argv[1:])
job = json.loads(job_path.read_text(encoding='utf-8-sig'))
config_raw = Path(job['legacy_config']['path']).read_bytes()
assert hashlib.sha256(config_raw).hexdigest() == job['legacy_config']['sha256']
c = json.loads(config_raw)
old = json.loads(legacy_path.read_bytes())
assert old['config_sha256'] == hashlib.sha256(config_raw).hexdigest()
result = json.loads((output/'replay.json').read_bytes())
assert result['legacy_config_sha256'] == old['config_sha256']
assert result['job_sha256'] == hashlib.sha256(job_path.read_bytes()).hexdigest()
spec = importlib.util.spec_from_file_location('finishing_oracle',Path('tools/rinkcraft_capability.py'))
module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
annual = {}; raw_checks = 0
for source in c['sources']:
    if source['kind'] != 'skaters':continue
    path=Path(job['data_root'])/source['path'];raw=path.read_bytes()
    assert len(raw)==source['bytes'] and hashlib.sha256(raw).hexdigest()==source['sha256']
    rows, _=module.parse_rows(path,source['season'],'skaters')
    annual[source['season']]=rows;raw_checks+=len(rows)
    recorded_source=next(s for s in result['sources'] if s['season']==source['season'])
    assert recorded_source['sha256']==source['sha256'] and recorded_source['five_on_five_rows']==len(rows)
training_input=json.loads((output/'training-input.json').read_bytes())
expected_training={(year,pid):r for year in c['train_seasons'] for pid,r in annual[year].items()}
assert len(training_input['rows'])==len(expected_training)
for r in training_input['rows']:
    wanted=expected_training.pop((r['season'],r['player_id']))
    for k in ['goals','xg','games','minutes']:
        assert math.isclose(r[k],wanted[k],rel_tol=1e-12,abs_tol=1e-12)
assert not expected_training
training=module.aggregate([annual[y] for y in c['train_seasons']])
reference={pid:r for pid,r in training.items() if module.eligible(r,c) and r['xg']>0}
prior=module.fit_prior(reference.values(),c['prior_shape_bounds'])
rprior=result['prior']
assert rprior['peers']==prior['peers']==old['prior']['peers']==787
# Declared port tolerances: optimizer location may differ on the flat optimum;
# objective, rates and downstream predictions are tested separately.
assert math.isclose(rprior['mean'],prior['mean'],rel_tol=1e-12,abs_tol=1e-12)
assert math.isclose(rprior['shape'],prior['shape'],rel_tol=1e-4)
assert math.isclose(rprior['rate'],prior['rate'],rel_tol=1e-4)
assert abs(rprior['negative_log_marginal_likelihood']-prior['negative_log_marginal_likelihood'])<2e-6
assert rprior['at_shape_boundary']==prior['at_shape_boundary']
assert rprior['training_seasons']==c['train_seasons']
assert {r['player_id'] for r in rprior['training']}==reference.keys()
for r in rprior['training']:
    wanted=reference[r['player_id']]
    for key in ['goals','xg','games','minutes']:
        assert math.isclose(r[key],wanted[key],rel_tol=1e-12,abs_tol=1e-12)
expected={pid:r for pid,r in annual[c['holdout_season']].items() if module.eligible(r,c) and r['xg']>0}
assert len(result['profiles'])==len(expected)==678
interval_checks=prediction_checks=0;seen=set();policy_values={p:[] for p in ['league','weak_pooling','empirical_bayes']}
group_values={}
for p in result['profiles']:
    pid=p['player_id'];assert pid not in seen;seen.add(pid)
    target=expected[pid];hist=training.get(pid,dict(goals=0,xg=0.))
    assert p['target_season']==c['holdout_season'] and p['headline_scores_released'] is False
    for k,new in [('goals','target_goals'),('xg','target_provider_xg'),('games','target_games'),('minutes','target_minutes')]:
        assert math.isclose(target[k],p[new],rel_tol=1e-12,abs_tol=1e-12)
    h=p['forecast_from_training_only'];assert h['baseline_only']==(hist['xg']==0)
    assert h['no_historical_rows']==(pid not in training)
    assert h['goals']==hist['goals'];assert math.isclose(h['provider_xg'],hist['xg'],rel_tol=1e-12)
    windows=[(h,hist['goals'],hist['xg'],'multiplier'),(p['target_season_component'],target['goals'],target['xg'],'posterior'),(p['pooled_available_window_component'],hist['goals']+target['goals'],hist['xg']+target['xg'],'posterior')]
    for w,g,x,key in windows:
        a,b=rprior['shape']+g,rprior['rate']+x
        assert math.isclose(w[key]['shape'],a,rel_tol=1e-12)
        assert math.isclose(w[key]['rate'],b,rel_tol=1e-12)
        assert math.isclose(w[key]['mean'],a/b,rel_tol=1e-12)
        ci=w['interval'];q=(1-c['interval_probability'])/2
        expected_interval=gamma.ppf([q,1-q],a,scale=1/b)
        assert math.isclose(ci['lower'],expected_interval[0],rel_tol=1e-9,abs_tol=1e-11)
        assert math.isclose(ci['upper'],expected_interval[1],rel_tol=1e-9,abs_tol=1e-11)
        interval_checks+=2
        if key=='posterior':
            assert w['headline_score'] is None
            assert w['status']==('experimental_conditional_component' if x>=c['minimum_scored_xg'] else 'insufficient_xg_exposure')
            for s in w['prior_strength_sensitivity']:
                f=s['prior_strength_factor'];assert f in [.5,2.]
                assert math.isclose(s['mean'],(rprior['shape']*f+g)/(rprior['rate']*f+x),rel_tol=1e-12)
    predictions={'league':prior['mean']*target['xg'],'weak_pooling':(hist['goals']+.5)/(hist['xg']+.5/prior['mean'])*target['xg'],'empirical_bayes':(hist['goals']+prior['shape'])/(hist['xg']+prior['rate'])*target['xg']}
    for policy in h['policies']:
        name=policy['policy'];pred=predictions[name]
        assert math.isclose(policy['expected_goals'],pred,rel_tol=1e-5,abs_tol=1e-7)
        actual_pred=policy['expected_goals'];dev=float(module.poisson_deviance([target['goals']],[actual_pred])[0])
        assert math.isclose(policy['poisson_deviance'],dev,rel_tol=1e-10,abs_tol=1e-10)
        policy_values[name].append(policy['poisson_deviance']);prediction_checks+=1
        for group in ['All','Defense' if target['position']=='D' else 'Forwards']:
            group_values.setdefault((group,name),[]).append((dev,abs(target['goals']-actual_pred),target['goals'],actual_pred))
assert seen==expected.keys()
for metric in result['evaluation']['metrics']:
    group_rows=group_values[(metric['group'],metric['policy'])]
    assert metric['players']==len(group_rows)
    for key,index in [('mean_poisson_deviance',0),('mean_absolute_goal_error',1)]:
        assert math.isclose(metric[key],np.mean([r[index] for r in group_rows]),rel_tol=1e-12,abs_tol=1e-12)
    assert metric['observed_goals']==sum(r[2] for r in group_rows)
    assert math.isclose(metric['predicted_goals'],sum(r[3] for r in group_rows),rel_tol=1e-12)
    if metric['group']=='All':
        assert math.isclose(metric['mean_poisson_deviance'],np.mean(policy_values[metric['policy']]),rel_tol=1e-12)
        assert math.isclose(metric['mean_poisson_deviance'],old['evaluation']['metrics'][metric['policy']]['mean_poisson_deviance'],rel_tol=1e-5,abs_tol=1e-7)
assert result['evaluation']['new_players']==old['evaluation']['new_players']==36
assert result['evaluation']['bootstrap_recomputed'] is False and result['evaluation']['conditional_comparison_gate_reassessed'] is False
summary=dict(source_rows_parsed=raw_checks,training_rows_verified=len(training_input['rows']),training_players=len(reference),target_players=len(seen),prior_shape=rprior['shape'],prior_rate=rprior['rate'],legacy_prior_shape=prior['shape'],conditional_interval_endpoints_checked=interval_checks,predictions_checked=prediction_checks,headline_scores_released=0,bootstrap_recomputed=False,legacy_reference_sha256=hashlib.sha256(legacy_path.read_bytes()).hexdigest())
assert not receipt.exists()
receipt.write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps(summary))
