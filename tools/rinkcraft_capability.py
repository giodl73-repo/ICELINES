"""Local, source-sealed exploratory finishing component; never a headline ability.
Requires the pinned research dependencies beside this file. No network or app writes.
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import html
import json
import math
import platform
import subprocess
from pathlib import Path

import numpy as np
import scipy
from scipy.optimize import minimize_scalar
from scipy.special import gammaln
from scipy.stats import gamma

SCHEMA = 'icelines.capability-component-pilot.v1'
SKATER = {'Skating':['Speed','Acceleration','Agility','Transitions'],
          'Handling':['Reception','Control','Protection','Pressure execution'],
          'Passing':['Delivery','Range','Timing','Deception'],
          'Shooting':['Finishing','Accuracy','Release','Velocity'],
          'Reading':['Recognition','Anticipation','Decision timing'],
          'Defending':['Gap control','Coverage','Denial','Recovery execution']}
GOALIE = {'Tracking':['Traffic response','Puck tracking'],
          'Positioning':['Depth','Alignment'], 'Movement':['Lateral movement','Set readiness'],
          'Rebounds':['Steering','Containment'], 'Recovery':['Repositioning','Threat response'],
          'Puck Play':['Control','Exchange','Pressure execution']}

def digest(data):
    return hashlib.sha256(data).hexdigest()

def read_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))

def finite_nonnegative(value, label):
    n = float(value)
    if not math.isfinite(n) or n < 0:
        raise ValueError(f'{label}: expected finite nonnegative number')
    return n

def parse_rows(path, season, kind):
    """Preserve source scope; fail on duplicates rather than sum traded-player totals."""
    rows, seen = {}, set()
    with Path(path).open(encoding='utf-8-sig', newline='') as file:
        reader = csv.DictReader(file)
        fields = reader.fieldnames or []
        required = ['playerId','name','season','situation','position','games_played','icetime']
        required += ['I_F_goals','I_F_xGoals'] if kind == 'skaters' else ['goals','xGoals']
        if set(required) - set(fields):
            raise ValueError(f'{path}: missing required columns')
        for raw in reader:
            if int(raw['season']) != season // 10000:
                raise ValueError('source season mismatch')
            if raw['situation'] != ('5on5' if kind == 'skaters' else 'all'):
                continue
            player = int(raw['playerId'])
            if player <= 0 or player in seen:
                raise ValueError('invalid or duplicate player-season row')
            seen.add(player)
            position = raw['position']
            if position not in (['C','L','R','D'] if kind == 'skaters' else ['G']):
                raise ValueError('position/source mismatch')
            g = finite_nonnegative(raw['I_F_goals' if kind == 'skaters' else 'goals'],'goals')
            x = finite_nonnegative(raw['I_F_xGoals' if kind == 'skaters' else 'xGoals'],'xg')
            if not g.is_integer() or (x == 0 and g > 0):
                raise ValueError('noninteger goals or positive goals with zero xG')
            rows[player] = {'player_id':player,'name':raw['name'],'position':position,
                'season':season,'goals':int(g),'xg':x,
                'games':finite_nonnegative(raw['games_played'],'games'),
                'minutes':finite_nonnegative(raw['icetime'],'seconds')/60}
    return rows, fields

def aggregate(seasons):
    result = {}
    for annual in seasons:
        for pid, row in annual.items():
            target = result.setdefault(pid, {**row,'goals':0,'xg':0.,'games':0.,'minutes':0.})
            for key in ['goals','xg','games','minutes']:
                target[key] += row[key]
            target['name'], target['position'] = row['name'], row['position']
    return result

def eligible(row, config):
    return row['games'] >= config['minimum_reference_games'] and row['minutes'] >= config['minimum_reference_minutes']

def fit_prior(rows, bounds):
    data = [r for r in rows if r['xg'] > 0]
    if len(data) < 3:
        raise ValueError('at least three positive-exposure calibration peers required')
    goals, exposure = np.array([r['goals'] for r in data]), np.array([r['xg'] for r in data])
    mu = float(goals.sum()/exposure.sum())
    if mu <= 0:
        raise ValueError('calibration requires positive total goals')
    def nll(log_shape):
        a = math.exp(log_shape)
        b = a/mu
        value = gammaln(goals+a)-gammaln(a)-gammaln(goals+1)
        value += a*np.log(b)+goals*np.log(exposure)-(goals+a)*np.log(b+exposure)
        return -float(value.sum())
    fit = minimize_scalar(nll, bounds=tuple(map(math.log,bounds)),method='bounded')
    if not fit.success:
        raise ValueError('prior optimizer failed')
    shape = math.exp(float(fit.x))
    return {'mean':mu,'shape':shape,'rate':shape/mu,'peers':len(data),
            'at_shape_boundary':shape < bounds[0]*1.01 or shape > bounds[1]/1.01,
            'negative_log_marginal_likelihood':float(fit.fun)}

def display_score(theta, prior, config):
    q = float(gamma.cdf(theta,prior['shape'],scale=1/prior['rate']))
    value = np.interp(q,config['score_quantile_knots'],config['score_value_knots'])
    return min(20,max(1,int(math.floor(float(value)+.5))))

def posterior(goals, xg, prior, config):
    if goals < 0 or xg < 0 or (xg == 0 and goals > 0):
        raise ValueError('invalid posterior evidence')
    shape, rate = prior['shape']+goals, prior['rate']+xg
    tail = (1-config['interval_probability'])/2
    lo, hi = map(float,gamma.ppf([tail,1-tail],shape,scale=1/rate))
    mean = shape/rate
    scored = xg >= config['minimum_scored_xg']
    alternatives = []
    for strength in [.5,2.]:
        estimate = (prior['shape']*strength+goals)/(prior['rate']*strength+xg)
        alternatives.append(display_score(estimate,prior,config))
    return {'goals':goals,'xg':xg,'effect_mean':mean,'effect_interval':[lo,hi],
        'interval_probability':config['interval_probability'],
        'candidate_score':display_score(mean,prior,config) if scored else None,
        'candidate_score_interval':[display_score(lo,prior,config),display_score(hi,prior,config)] if scored else None,
        'prior_sensitivity_scores':alternatives if scored else [],
        'status':'experimental_component' if scored else 'insufficient_exposure',
        'interval_scope':'Conditional on fitted prior, fixed xG and constant finishing multiplier; excludes provider/prior uncertainty and context confounding.'}

def poisson_deviance(actual, expected):
    actual, expected = np.asarray(actual,dtype=float), np.asarray(expected,dtype=float)
    if np.any(expected <= 0) or np.any(actual < 0):
        raise ValueError('deviance requires positive forecast and nonnegative outcome')
    terms = np.zeros_like(actual)
    positive = actual > 0
    terms[positive] = actual[positive]*np.log(actual[positive]/expected[positive])
    return 2*(terms-actual+expected)

def evaluate(training, heldout, prior, config):
    rows = [r for r in heldout.values() if eligible(r,config) and r['xg'] > 0]
    if not rows:
        raise ValueError('empty held-out evaluation')
    observed, forecasts = [], {name:[] for name in ['league','weak_pooling','empirical_bayes']}
    for r in rows:
        t = training.get(r['player_id'],{'goals':0,'xg':0})
        observed.append(r['goals'])
        forecasts['league'].append(prior['mean']*r['xg'])
        forecasts['weak_pooling'].append((t['goals']+.5)/(t['xg']+.5/prior['mean'])*r['xg'])
        forecasts['empirical_bayes'].append((t['goals']+prior['shape'])/(t['xg']+prior['rate'])*r['xg'])
    observed = np.array(observed)
    dev = {k:poisson_deviance(observed,v) for k,v in forecasts.items()}
    metrics = {k:{'mean_poisson_deviance':float(dev[k].mean()),
        'mean_absolute_goal_error':float(np.abs(observed-np.array(v)).mean())} for k,v in forecasts.items()}
    random = np.random.default_rng(config['bootstrap_seed'])
    indices = random.integers(0,len(rows),size=(config['bootstrap_trials'],len(rows)))
    comparisons = {}
    for name in ['league','weak_pooling']:
        delta = dev[name]-dev['empirical_bayes']
        interval = np.quantile(delta[indices].mean(axis=1),[.025,.975]).tolist()
        comparisons[name] = {'mean_deviance_improvement':float(delta.mean()),'bootstrap_95_interval':interval}
    groups={}
    for label,mask in [('Defense',[r['position']=='D' for r in rows]),('Forwards',[r['position']!='D' for r in rows])]:
        take=np.array(mask,dtype=bool)
        if take.any():
            groups[label]={'players':int(take.sum()),'observed_goals':int(observed[take].sum()),
                'predicted_goals':float(np.array(forecasts['empirical_bayes'])[take].sum()),
                'league_deviance':float(dev['league'][take].mean()),
                'empirical_bayes_deviance':float(dev['empirical_bayes'][take].mean())}
    passed = all(v['bootstrap_95_interval'][0] > 0 for v in comparisons.values())
    return {'season':config['holdout_season'],'players':len(rows),'new_players':sum(r['player_id'] not in training for r in rows),
        'metrics':metrics,'position_checks':groups,'comparisons':comparisons,'conditional_comparison_gate_passed':passed,'prospective_validation_passed':False,'feature_vintage_verified':False,
        'conditioning':'Held-out observed xG is known to every baseline; no held-out goals enter prediction.',
        'limits':'Retrospective player bootstrap; team/opponent dependence, selection, xG calibration drift and confounding remain.'}

def audit(kind, fields, row_count):
    if kind == 'skaters':
        proxies = {'Skating':[], 'Handling':['I_F_giveaways','I_F_takeaways'],
            'Passing':['I_F_primaryAssists','I_F_secondaryAssists'],
            'Shooting':['I_F_goals','I_F_xGoals','I_F_shotsOnGoal','I_F_missedShots','I_F_unblockedShotAttempts'],
            'Reading':['I_F_takeaways'], 'Defending':['OnIce_A_xGoals','shotsBlockedByPlayer','I_F_takeaways']}
    else:
        proxies = {'Tracking':['ongoal','goals'], 'Positioning':['xGoals','goals'],
            'Movement':['highDangerShots','highDangerGoals'], 'Rebounds':['rebounds','xRebounds'],
            'Recovery':['reboundGoals','rebounds'], 'Puck Play':[]}
    return {'kind':kind,'rows':row_count,'source_column_count':len(fields),
        'capabilities':{k:{'available_proxy_fields':[f for f in values if f in fields],
        'missing_candidate_fields':[f for f in values if f not in fields],
        'headline_score_supported':False} for k,values in proxies.items()}}

def read_edge(config, root):
    entry = config.get('edge_capture')
    if not entry:
        return None
    manifest_path, path = root/entry['manifest'], root/entry['path']
    manifest = read_json(manifest_path)
    if manifest.get('schema') != 'icelines.edge-capture.v1':
        raise ValueError('unsupported EDGE capture manifest')
    data = path.read_bytes()
    records = [r for r in manifest['records'] if r['sha256'] == digest(data) and r['bytes'] == len(data)]
    if len(records) != 1:
        raise ValueError('EDGE capture provenance mismatch')
    record, v = records[0], json.loads(data)
    if record['player_id'] != v['player']['id'] or f"/{manifest['season']}/2" not in record['url']:
        raise ValueError('EDGE identity/season mismatch')
    return {'player_id':record['player_id'],'season':manifest['season'],'accessed':manifest['accessed'],
        'source':record,'capture_manifest_sha256':digest(manifest_path.read_bytes()),
        'max_mph':v['skatingSpeed']['speedMax']['imperial'],
        'bursts_over_20':v['skatingSpeed']['burstsOver20']['value'],
        'top_shot_mph':v['topShotSpeed']['imperial'],
        'scope':'Historical all-strengths maxima/counts; one saved profile, not a full population or complete Skating/Shooting capability.'}

def safe_git():
    root = Path(__file__).resolve().parents[1]
    def run(*args):
        return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
    return {'commit':run('rev-parse','HEAD'),'dirty':bool(run('status','--porcelain'))}

def run(config_path):
    config_path = Path(config_path).resolve()
    config = read_json(config_path)
    if config['schema'] != 'icelines.capability-pilot-config.v1':
        raise ValueError('unsupported config schema')
    train_seasons, target = config['train_seasons'],config['holdout_season']
    if len(set(config['selected_players'])) != len(config['selected_players']):
        raise ValueError('duplicate selected player')
    for key in ['minimum_reference_minutes','minimum_reference_games','minimum_scored_xg']:
        finite_nonnegative(config[key],key)
    for year in train_seasons+[target]:
        first=year//10000
        if not 1900<=first<=2200 or year%10000!=first+1:
            raise ValueError('invalid season id')
    if not train_seasons or max(train_seasons) >= target or len(set(train_seasons)) != len(train_seasons):
        raise ValueError('training must strictly precede held-out season')
    if config['situation'] != '5on5' or config['bootstrap_trials'] <= 0:
        raise ValueError('unsupported situation or invalid bootstrap count')
    if not 0 < config['interval_probability'] < 1:
        raise ValueError('invalid interval probability')
    q, scale = config['score_quantile_knots'],config['score_value_knots']
    if len(q)!=len(scale) or any(a>=b for a,b in zip(q,q[1:])) or any(a>b for a,b in zip(scale,scale[1:])) or q[0]!=0 or q[-1]!=1 or min(scale)<1 or max(scale)>20:
        raise ValueError('invalid monotone display scale')
    root = (config_path.parent/config['data_root']).resolve()
    annual, identities, audits, sealed = {}, {}, [], []
    for source in config['sources']:
        key = (source['season'],source['kind'])
        if key[1] not in ['skaters','goalies']:
            raise ValueError('unsupported source kind')
        if key in annual:
            raise ValueError('duplicate source season/kind')
        path = root/source['path'];data = path.read_bytes()
        if digest(data)!=source['sha256'] or len(data)!=source['bytes']:
            raise ValueError('source hash/length mismatch')
        annual[key], fields = parse_rows(path,*key)
        identities.update(annual[key]);audits.append({'season':key[0],**audit(key[1],fields,len(annual[key]))})
        sealed.append({**source,'resolved_path':str(path)})
    for year in train_seasons+[target]:
        if (year,'skaters') not in annual:
            raise ValueError('missing required skater source')
    training = aggregate([annual[y,'skaters'] for y in train_seasons])
    reference = {k:v for k,v in training.items() if eligible(v,config) and v['xg']>0}
    prior = fit_prior(reference.values(),config['prior_shape_bounds'])
    evaluation = evaluate(training,annual[target,'skaters'],prior,config)
    pooled = aggregate([annual[y,'skaters'] for y in train_seasons+[target]])
    edge = read_edge(config,root)
    profiles=[]
    for pid in config['selected_players']:
        identity=identities.get(pid)
        if identity is None:
            raise ValueError(f'selected player has no audited identity: {pid}')
        is_goalie = identity['position']=='G'
        latest = annual[target,'skaters'].get(pid) if not is_goalie else None
        components = {}
        if not is_goalie:
            rows=[annual[y,'skaters'][pid] for y in train_seasons+[target] if pid in annual[y,'skaters']]
            total=pooled.get(pid)
            components['finishing']={'label':'Conditional finishing component','headline_attribute':'Shooting',
                'latest_season':target,'latest_season_evidence':posterior(latest['goals'],latest['xg'],prior,config) if latest and latest['xg']>0 else None,
                'available_three_season_window':posterior(total['goals'],total['xg'],prior,config) if total and total['xg']>0 else None,
                'latest_season_missing':latest is None,'pooled_is_historical_only':latest is None,
                'annual_raw_evidence':rows,
                'last_observed_season':max((r['season'] for r in rows),default=None),
                'limits':'Not a complete Shooting score; xG-adjusted conditional residual, not isolated causal shooting mechanics.'}
        attributes=[]
        for name, requirements in (GOALIE if is_goalie else SKATER).items():
            modelled = ['Finishing'] if name=='Shooting' and components.get('finishing',{}).get('latest_season_evidence',{} ) else []
            attributes.append({'name':name,'score':None,'status':'unrated','required_components':requirements,
                'experimental_components':modelled,'release_blockers':['Incomplete construct coverage','No validated aggregation or headline 1–20 calibration']})
        profiles.append({'player_id':pid,'name':identity['name'],'position':identity['position'],
            'attributes':attributes,'components':components,'tracking_evidence':edge if edge and edge['player_id']==pid else None})
    cohort=[{'player_id':pid,'name':r['name'],'position':r['position'],
             'training_posterior':posterior(r['goals'],r['xg'],prior,config)} for pid,r in sorted(reference.items())]
    return {'schema':SCHEMA,'protocol':config['protocol'],'config_sha256':digest(config_path.read_bytes()),
        'ice_lines_source':safe_git(),'implementation_sha256':digest(Path(__file__).read_bytes()),'environment':{'python':platform.python_version(),'numpy':np.__version__,'scipy':scipy.__version__},
        'source_credit':'MoneyPuck.com; NHL EDGE saved historical capture. Third-party source terms apply.',
        'sources':sealed,'config':config,'audit':audits,'prior':prior,'evaluation':evaluation,
        'profiles':profiles,'calibration_cohort':cohort,
        'headline_scores_released':False,
        'tracking_coverage':{'saved_profiles':1 if edge else 0,'full_population_available':False},
        'limits':['All headline capabilities remain unrated; candidate component scores are experimental.',
          'No public data/technique coverage is invented from outcome proxies.',
          'Pooled estimates assume constant effect over available seasons, not complete career or current ability.',
          'Conditional posterior intervals omit hyperparameter/model uncertainty and contextual confounding.',
          'Historical provider snapshots may share later xG-model revisions; acquisition hashes do not prove point-in-time availability.',
          'Retrospective source-vintage leakage prevents a prospective or preregistered validation claim.']}

def render_report(result):
    e=html.escape
    rows=[]
    for p in result['profiles']:
        c=p['components'].get('finishing',{})
        latest=c.get('latest_season_evidence')
        pooled=c.get('available_three_season_window')
        def value(v):
            if not v or v['candidate_score'] is None:
                return 'Unrated / insufficient exposure'
            lo,hi=v['candidate_score_interval']
            return f"{v['candidate_score']} /20 · conditional 90% range {lo}–{hi}"
        latest_text=value(latest)
        pooled_text=value(pooled)
        if p['position']=='G':
            latest_text=pooled_text='Not assessed: goalie technique requires its own evidence'
        elif c.get('latest_season_missing'):
            latest_text='No 2025–26 skater record'
            pooled_text='Historical through '+str(c.get('last_observed_season'))+' · '+pooled_text
        raw='; '.join(f"{r['season']}: {r['goals']} G / {r['xg']:.2f} xG" for r in c.get('annual_raw_evidence',[]))
        rows.append(f"<tr><th>{e(p['name'])}<small>{e(p['position'])}</small></th><td>{e(latest_text)}</td><td>{e(pooled_text)}</td><td>{e(raw or 'Goalie technique not identified by saving outcomes')}</td></tr>")
    auditrows=[]
    for a in result['audit']:
        for name,cap in a['capabilities'].items():
            auditrows.append(f"<tr><td>{a['season']} · {a['kind']}</td><td>{e(name)}</td><td>{e(', '.join(cap['available_proxy_fields']) or 'No relevant field in this summary')}</td><td>Unrated</td></tr>")
    metrics=''.join(f"<tr><th>{e(k)}</th><td>{v['mean_poisson_deviance']:.4f}</td><td>{v['mean_absolute_goal_error']:.3f}</td></tr>" for k,v in result['evaluation']['metrics'].items())
    comparisons=''.join(f"<li>Against {e(k)}: mean deviance improvement {v['mean_deviance_improvement']:.4f}; bootstrap 95% interval {v['bootstrap_95_interval'][0]:.4f} to {v['bootstrap_95_interval'][1]:.4f}.</li>" for k,v in result['evaluation']['comparisons'].items())
    return f'''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft capability pilot</title>
<style>body{{font:15px/1.5 system-ui;background:#f3f8fc;color:#183247;margin:0}}main{{max-width:1250px;padding:28px;margin:auto}}h1{{border-bottom:3px solid #286b92;padding-bottom:12px}}h2{{color:#286b92}}.notice{{padding:16px;border-left:4px solid #ac4350;background:white}}.scroll{{overflow:auto}}table{{width:100%;border-collapse:collapse;background:white;font-size:13px}}td,th{{padding:12px;text-align:left;border-bottom:1px solid #cad9e3}}th small{{display:block;color:#536574}}code{{overflow-wrap:anywhere}}summary{{cursor:pointer;padding:12px;background:#e5eff6}}li{{margin:7px 0}}</style>
<main><p>ICELINES / RINKCRAFT · RESEARCH PILOT</p><h1>From evidence to capability ratings</h1>
<p class="notice"><b>Component estimates, not released player attributes.</b> All six skater and six goalie headline attributes remain Unrated. These numbers measure a narrow conditional finishing component. Skating lacks a full tracking reference and agility/acceleration evidence. No 20+ tier is defined.</p>
<h2>1. Full-league calibration and retrospective conditional check</h2><p>{result['prior']['peers']} calibration skaters; {result['evaluation']['players']} held-out skaters. Prior finishing multiplier mean {result['prior']['mean']:.3f}; shape {result['prior']['shape']:.3f}. Train {e(str(result['config']['train_seasons']))}; held out {result['config']['holdout_season']}.</p>
<table><tr><th>Predictor</th><th>Mean Poisson deviance ↓</th><th>Mean absolute goal error ↓</th></tr>{metrics}</table><ul>{comparisons}</ul><p><b>Conditional comparison gate: {'passed' if result['evaluation']['conditional_comparison_gate_passed'] else 'not passed'}.</b> Historical xG feature vintage is unverified: prospective validation remains blocked. This comparison cannot release Shooting or validate the 1–20 scale. All forecasts condition on realized held-out xG.</p>
<h2>2. Contrasting-player component estimates</h2><p>The calibration scale is frozen from the training prior, not from these selected players. Pooled estimates cover available 2023–26 observations and assume constant finishing effect. An absent latest season is not backfilled.</p><div class="scroll"><table><tr><th>Player</th><th>2025–26 finishing candidate</th><th>Available 3-season finishing candidate</th><th>Source evidence</th></tr>{''.join(rows)}</table></div>
<h2>3. Capability coverage audit</h2><details><summary>Inspect all source seasons, positions and proxy fields</summary><div class="scroll"><table><tr><th>Source</th><th>Attribute</th><th>Available proxies — not automatic ability evidence</th><th>Headline</th></tr>{''.join(auditrows)}</table></div></details>
<h2>4. Limits and reproducibility</h2><ul>{''.join(f'<li>{e(x)}</li>' for x in result['limits'])}</ul><p>One saved 2024–25 EDGE profile supplies historical raw speed and shot-speed context, not a population skating grade. No blocked endpoint was bypassed.</p><p>{e(result['source_credit'])}</p><p>Protocol <code>{e(result['protocol'])}</code> · config SHA256 <code>{result['config_sha256']}</code>.</p><p><a href="assessments.json">Full JSON, source hashes, raw effects, intervals and sensitivity</a> · <a href="https://moneypuck.com/about.htm">Provider methods</a> · <a href="https://arxiv.org/abs/1208.0799">Hierarchical hockey-rating research</a></p></main></html>'''

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args()
    result=run(args.config)
    args.output.mkdir(parents=True,exist_ok=True)
    text=json.dumps(result,indent=2,allow_nan=False)+'\n'
    (args.output/'assessments.json').write_text(text,encoding='utf-8')
    (args.output/'report.html').write_text(render_report(result),encoding='utf-8')
    (args.output/'run.json').write_text(json.dumps({'schema':SCHEMA,'config_sha256':result['config_sha256'],
        'assessment_sha256':digest(text.encode()),'implementation_sha256':result['implementation_sha256'],'source':result['ice_lines_source'],
        'command':['python',str(Path(__file__).resolve()),'--config',str(args.config.resolve()),'--output',str(args.output.resolve())],
        'seed':result['config']['bootstrap_seed'],'trials':result['config']['bootstrap_trials'],
        'environment':result['environment']},indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'prior':result['prior'],'evaluation':result['evaluation'],
        'profiles':len(result['profiles']),'headline_scores_released':False},indent=2))

if __name__=='__main__':
    main()
