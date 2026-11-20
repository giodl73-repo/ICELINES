"""Offline event-level finishing experiment; neutral input contract, no downloads.
No player grades. Real results require actual sealed events and audited baseline.
"""
from __future__ import annotations
import argparse
from collections import Counter,defaultdict
from datetime import date
import hashlib
import html
import json
import math
from pathlib import Path
import random
import re
import subprocess

FEATURES={'shot_distance','shot_angle','shot_type','previous_event_type','previous_event_elapsed_seconds','pre_shot_manpower','shooter_handedness','home_away'}
SCOPE={'competition':'NHL','season_type':'regular','situation':'5v5','attempt_scope':'unblocked','both_goalies_present':True}

def load(path):return json.loads(Path(path).read_text(encoding='utf-8-sig'))
def day(value):
    parsed=date.fromisoformat(value)
    if parsed.isoformat()!=value:raise ValueError('Canonical YYYY-MM-DD dates required')
    return parsed
def positive_int(value):return isinstance(value,int) and not isinstance(value,bool) and value>0

def validate_config(config):
    if config.get('schema')!='icelines.event-finishing-config.v1':raise ValueError('Unknown config schema')
    if config.get('data_kind') not in ('observed','synthetic'):raise ValueError('Explicit data kind required')
    if config.get('evaluation_mode') not in ('retrospective_temporal','prospective_metadata'):raise ValueError('Explicit retrospective or prospective-metadata mode required')
    w=config['windows'];dates=[day(w[k]) for k in ('start','replication_start','test_start','end_exclusive')]
    if dates!=sorted(set(dates)):raise ValueError('Chronological nonempty windows required')
    season=config['scope'].get('season')
    if not positive_int(season) or len(str(season))!=8 or season//10000+1!=season%10000:raise ValueError('Consecutive YYYYyyyy season required')
    for k,value in SCOPE.items():
        if config['scope'].get(k)!=value or type(config['scope'].get(k)) is not type(value):raise ValueError('Unsupported target scope')
    for k in ('league_prior_sd','player_prior_sd'):
        value=config['model'][k]
        if isinstance(value,bool) or not isinstance(value,(int,float)) or not math.isfinite(value) or value<=0:raise ValueError('Positive finite prior SD required')
    if not positive_int(config['model']['minimum_stability_attempts_per_block']):raise ValueError('Invalid stability threshold')
    if any(isinstance(x,bool) or not isinstance(x,(int,float)) or not math.isfinite(x) or x<=0 for x in config['model']['sensitivity_prior_sd']):raise ValueError('Invalid sensitivity priors')
    boot=config['bootstrap']
    if not positive_int(boot['trials']) or not isinstance(boot['seed'],int) or not 0<boot['interval_probability']<1:raise ValueError('Invalid bootstrap settings')
    return dates

def validate(config,events):
    dates=validate_config(config)
    if not events:raise ValueError('No events; no empirical run')
    baseline=config.get('baseline')
    if not baseline or any(not baseline.get(k) for k in ('id','sha256','trained_through','available_from','feature_columns','audit_locator')):raise ValueError('Audited frozen baseline metadata required')
    if not re.fullmatch('[0-9a-f]{64}',baseline['sha256']) or baseline.get('immutable') is not True:raise ValueError('Immutable baseline hash required')
    if day(baseline['trained_through'])>=dates[0]:raise ValueError('Baseline training leaks into experiment')
    available_by_start=day(baseline['available_from'])<=dates[0]
    if config['evaluation_mode']=='prospective_metadata' and not available_by_start:raise ValueError('Baseline unavailable at prospective start')
    if not set(baseline['feature_columns'])<=FEATURES:raise ValueError('Unsupported or outcome/identity-derived baseline features')
    seen=set();games={};positions={};counts=Counter()
    for e in events:
        required={'game_id','event_id','game_date','shooter_id','goalie_id','position_group','outcome','baseline_id','baseline_probability',*SCOPE,'season'}
        if required-set(e):raise ValueError('Missing required event fields: '+str(sorted(required-set(e))))
        gid=e['game_id'];eid=e['event_id'];when=day(e['game_date']);key=(gid,eid)
        if not all(positive_int(e[k]) for k in ('game_id','event_id','shooter_id','goalie_id')):raise ValueError('Positive actor/game/event IDs required')
        if key in seen:raise ValueError('Duplicate game-event; replay cannot add evidence')
        seen.add(key)
        if gid in games and games[gid]!=when:raise ValueError('One game cannot cross date/window boundaries')
        games[gid]=when
        if not dates[0]<=when<dates[-1]:raise ValueError('Out-of-window events must be excluded and documented upstream')
        for k,value in {**SCOPE,'season':config['scope']['season']}.items():
            if e.get(k)!=value or type(e.get(k)) is not type(value):raise ValueError('Mixed strength/season/goalie/attempt scope')
        if e['outcome'] not in ('goal','saved','missed'):raise ValueError('Only documented unblocked outcomes supported')
        if e['position_group'] not in ('F','D'):raise ValueError('Explicit F/D support required')
        pid=e['shooter_id']
        if pid in positions and positions[pid]!=e['position_group']:raise ValueError('Inconsistent F/D mapping')
        positions[pid]=e['position_group']
        p=e['baseline_probability']
        if isinstance(p,bool) or not isinstance(p,(int,float)) or not math.isfinite(p) or not 0<p<1:raise ValueError('Finite nondegenerate baseline probabilities required')
        if e.get('baseline_id')!=baseline['id']:raise ValueError('Mixed baseline versions')
        block='train' if when<dates[1] else 'replication' if when<dates[2] else 'test';counts[block]+=1
    if set(counts)!= {'train','replication','test'}:raise ValueError('Events required in both training blocks and held-out test')
    return dict(counts)

def logit(p):return math.log(p)-math.log1p(-p)
def logistic(x):
    if x>=0:return 1/(1+math.exp(-x))
    z=math.exp(x);return z/(1+z)

def fit_shift(events,offset,sd):
    """Unique penalized Bernoulli optimum with fixed logit offsets and N(0,sd²)."""
    shift=0.;precision=1/(sd*sd)
    for _ in range(100):
        gradient=-shift*precision;curvature=precision
        for e in events:
            p=logistic(logit(e['baseline_probability'])+offset+shift)
            gradient+=(e['outcome']=='goal')-p;curvature+=p*(1-p)
        step=max(-2.,min(2.,gradient/curvature));shift+=step
        if abs(step)<1e-10:return shift
    raise ValueError('MAP optimizer did not converge')

def fit(events,config):
    delta=fit_shift(events,0,config['model']['league_prior_sd']);by=defaultdict(list)
    for e in events:by[e['shooter_id']].append(e)
    beta={pid:fit_shift(rows,delta,config['model']['player_prior_sd']) for pid,rows in by.items()}
    return {'league_shift':delta,'player_shifts':beta}

def probability(e,model,player=True):return logistic(logit(e['baseline_probability'])+model['league_shift']+(model['player_shifts'].get(e['shooter_id'],0) if player else 0))
def loss(y,p):
    # Numerical safeguard for floating-point saturation, not probability calibration.
    p=max(1e-15,min(1-1e-15,p))
    return -math.log(p) if y else -math.log1p(-p)

def quantile(values,q):
    a=sorted(values);pos=(len(a)-1)*q;i=int(pos);j=min(i+1,len(a)-1);return a[i]+(pos-i)*(a[j]-a[i])

def clustered_interval(game_differences,config):
    # Paired game resampling: all events in a selected game travel together.
    groups=list(game_differences.values());boot=config['bootstrap'];rng=random.Random(boot['seed']);samples=[]
    for _ in range(boot['trials']):
        selected=[rng.choice(groups) for _ in groups];samples.append(sum(x[0] for x in selected)/sum(x[1] for x in selected))
    tail=(1-boot['interval_probability'])/2
    return [quantile(samples,tail),quantile(samples,1-tail)]

def experiment(config,events):
    counts=validate(config,events);w=config['windows'];early=[e for e in events if e['game_date']<w['replication_start']];replication=[e for e in events if w['replication_start']<=e['game_date']<w['test_start']];train=early+replication;test=[e for e in events if e['game_date']>=w['test_start']]
    model=fit(train,config);groups=defaultdict(lambda:[0.,0]);results={};bins=defaultdict(lambda:[0,0,0.]);unseen=set();sens=[]
    forecasts={name:[] for name in ('frozen_baseline','league_adjustment','player_MAP')};observed=[];by_position=defaultdict(lambda:defaultdict(float))
    for e in test:
        y=int(e['outcome']=='goal');observed.append(y);pred={'frozen_baseline':e['baseline_probability'],'league_adjustment':probability(e,model,False),'player_MAP':probability(e,model)}
        for name,p in pred.items():forecasts[name].append(p)
        difference=loss(y,pred['player_MAP'])-loss(y,pred['league_adjustment']);groups[e['game_id']][0]+=difference;groups[e['game_id']][1]+=1
        pos=by_position[e['position_group']];pos['attempts']+=1;pos['goals']+=y;pos['player_MAP_expected_goals']+=pred['player_MAP'];pos['league_expected_goals']+=pred['league_adjustment']
        b=bins[min(9,int(pred['player_MAP']*10))];b[0]+=1;b[1]+=y;b[2]+=pred['player_MAP']
        if e['shooter_id'] not in model['player_shifts']:unseen.add(e['shooter_id'])
    for name,ps in forecasts.items():results[name]={'mean_log_loss':sum(loss(y,p) for y,p in zip(observed,ps))/len(ps),'brier_score':sum((y-p)**2 for y,p in zip(observed,ps))/len(ps),'predicted_goals':sum(ps)}
    for sd in config['model']['sensitivity_prior_sd']:
        alternate={**config,'model':{**config['model'],'player_prior_sd':sd}};m=fit(train,alternate);sens.append({'prior_sd':sd,'mean_log_loss':sum(loss(e['outcome']=='goal',probability(e,m)) for e in test)/len(test)})
    # A common league offset learned only in early games; neither block borrows later outcomes.
    first=fit(early,config);early_by=defaultdict(list);rep_by=defaultdict(list)
    for e in early:early_by[e['shooter_id']].append(e)
    for e in replication:rep_by[e['shooter_id']].append(e)
    minimum=config['model']['minimum_stability_attempts_per_block'];stable=[]
    for pid in sorted(set(early_by)&set(rep_by)):
        if min(len(early_by[pid]),len(rep_by[pid]))>=minimum:stable.append({'shooter_id':pid,'early_attempts':len(early_by[pid]),'replication_attempts':len(rep_by[pid]),'early_log_odds_shift':first['player_shifts'][pid],'replication_log_odds_shift':fit_shift(rep_by[pid],first['league_shift'],config['model']['player_prior_sd'])})
    interval=clustered_interval(groups,config) if len(groups)>=2 else None
    return {'schema':'icelines.event-finishing-result.v1','status':'synthetic_demonstration' if config['data_kind']=='synthetic' else 'exploratory_observed_run','data_kind':config['data_kind'],'evaluation_mode':config['evaluation_mode'],'baseline_available_by_start':day(config['baseline']['available_from'])<=day(w['start']),'prospective_validation_established':False,'event_count':len(events),'observed_event_count':len(events) if config['data_kind']=='observed' else 0,'event_counts':counts,'game_counts':dict(Counter('train' if d<day(w['replication_start']) else 'replication' if d<day(w['test_start']) else 'test' for d in {e['game_id']:day(e['game_date']) for e in events}.values())),'training_model':model,'heldout_metrics':results,'player_MAP_minus_league_log_loss_interval':interval,'interval_scope':'Paired game-clustered evaluation interval conditional on fitted training model and fixed baseline; not a player-ability interval, prospective proof or complete model uncertainty. Negative difference favors player MAP.','heldout_positions':dict(by_position),'calibration_bins':[dict(probability_range=[k/10,(k+1)/10],attempts=v[0],goals=v[1],predicted_goals=v[2]) for k,v in sorted(bins.items())],'prior_sensitivity':sens,'stability_pairs':stable,'unseen_heldout_players':sorted(unseen),'production_grades_released':False,'empirical_validation_established':False,'limits':config.get('limits',[])}

def render(result,config):
    esc=html.escape;status=result['status'].replace('_',' ')
    return '<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft event validation</title><style>body{max-width:950px;margin:auto;padding:24px;background:#edf3f6;color:#143044;font:16px/1.5 system-ui;overflow-wrap:anywhere}article{background:white;border:1px solid #9cafbb;border-top:5px solid #267e95;padding:20px}h1{border-bottom:5px solid #b23e52}pre{white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.5 ui-monospace}a{color:#176078}summary{cursor:pointer;font-weight:bold}</style><h1>Rinkcraft • Event validation</h1><article><h2>'+esc(status.capitalize())+'</h2><p>No player attribute grades are released. Synthetic demonstrations are software checks, never NHL findings.</p><p><a href="../readiness/index.html">Evidence workbench</a> · <a href="result.json">Full run record</a></p><h3>Experiment</h3><p>Two chronological training blocks, followed by held-out games. Compare a frozen shot baseline, league adjustment and shrunk player effect. Resample complete games for evaluation uncertainty; report F/D calibration separately.</p><h3>Current result</h3><pre>'+esc(json.dumps(result,indent=2,ensure_ascii=False))+'</pre><details><summary>Configured protocol and data requirements</summary><pre>'+esc(json.dumps(config,indent=2,ensure_ascii=False))+'</pre></details></article></html>'

def run(path):
    config=load(path);validate_config(config);entry=config.get('input')
    if not entry:return config,{'schema':'icelines.event-finishing-result.v1','status':f"awaiting_{config['data_kind']}_event_data",'data_kind':config['data_kind'],'observed_event_count':0,'heldout_metrics':None,'acquisition':config.get('acquisition',[]),'production_grades_released':False,'empirical_validation_established':False,'limits':config.get('limits',[])}
    if config['data_kind']=='observed' and any(not entry.get(k) for k in ('source_url','accessed','adapter_version','rights','coverage_note')):raise ValueError('Observed input provenance/rights/coverage metadata required')
    source=(Path(path).parent/entry['path']).resolve();raw=source.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=entry['sha256']:raise ValueError('Event source hash mismatch')
    events=[json.loads(line) for line in raw.decode('utf-8-sig').splitlines() if line.strip()]
    return config,experiment(config,events)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--config',required=True,type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args();config,result=run(args.config)
    result['provenance']={'config_sha256':hashlib.sha256(args.config.read_bytes()).hexdigest(),'input':config.get('input'),'protocol_configuration':{k:config.get(k) for k in ('protocol','evaluation_mode','baseline','scope','windows','model','bootstrap')},'implementation_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'ice_lines_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=Path(__file__).parent,text=True).strip()}
    args.output.mkdir(parents=True,exist_ok=True);(args.output/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');(args.output/'index.html').write_text(render(result,config),encoding='utf-8');print(json.dumps({'status':result['status'],'observed_event_count':result.get('observed_event_count'),'production_grades_released':False}))
if __name__=='__main__':main()
