"""Research-only Rinkcraft definition validation and reference rendering.
No player-data ingestion, network access, fitted calibration or production writes.
Synthetic score-policy helpers do not establish empirical release eligibility.
"""
from __future__ import annotations
import argparse
import hashlib
import html
import json
import math
from pathlib import Path
from statistics import NormalDist

EXPECTED = {'skater': {'Skating','Handling','Passing','Shooting','Reading','Defending'},
            'goalie': {'Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play'}}
ATTRIBUTE_FIELDS = ('construct','exclusions','components','context','evidence_contract','overlap_rules',
                    'behavior_anchors','current_coverage','validation_plan','counterexamples','sources')
COMPONENT_FIELDS = ('definition','opportunity','start','end','success','failure','censoring','measure','unit',
                    'direction','inputs','adjustments','confounds','disconfirming_test')
GATES = ('all_required_components_measured','target_scope_and_opportunities_complete',
         'source_lineages_and_vintages_verified','measurement_reliability_passed',
         'context_and_position_support_passed','frozen_reference_calibrated',
         'joint_uncertainty_calibrated','heldout_task_validation_passed',
         'composite_construct_validation_passed','weight_sensitivity_passed',
         'independent_methods_review_recorded')

def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))

def validate(v):
    errors=[]
    if v.get('schema') != 'rinkcraft.attribute-contracts.v1': errors.append('Unknown schema')
    if v.get('production_grades_released') is not False: errors.append('Definition registry must not release grades')
    for key in ('version','scope','observation_policy','evidence_policy','scoring_policy','sources','migration'):
        if not v.get(key): errors.append(f'Missing shared {key}')
    attrs=v.get('attributes',[])
    if len(attrs)!=12: errors.append('Expected twelve separate contracts')
    ids=[a.get('id') for a in attrs]
    if len(set(ids))!=len(ids): errors.append('Duplicate attribute ID')
    components=[]
    for branch,names in EXPECTED.items():
        actual=[a.get('name') for a in attrs if a.get('branch')==branch]
        if set(actual)!=names or len(actual)!=6: errors.append(f'Incomplete {branch} names')
    for a in attrs:
        aid=a.get('id','?')
        for key in ATTRIBUTE_FIELDS:
            if not a.get(key): errors.append(f'{aid}: missing {key}')
        if a.get('measurement_validation_status')!='unvalidated' or a.get('player_grades_released') is not False:
            errors.append(f'{aid}: definition is not measurement validation')
        for key in ('construct','current_coverage'):
            if not isinstance(a.get(key),str): errors.append(f'{aid}: {key} must be text')
        cs=a.get('components',[]); keys=[c.get('id') for c in cs];components.extend(keys)
        if len(cs)<2: errors.append(f'{aid}: incomplete facet battery')
        for c in cs:
            for key in COMPONENT_FIELDS:
                if not c.get(key): errors.append(f'{c.get("id")}: missing {key}')
            if c.get('required') is not True: errors.append(f'{aid}: optional facet silently changes construct')
            if c.get('direction') not in ('higher','lower','task_model'): errors.append(f'{aid}: unknown polarity')
        if set(a.get('evidence_contract',{}).get('required_components',[]))!=set(keys):
            errors.append(f'{aid}: required-facet mismatch')
        weights=a.get('candidate_weights',{})
        if set(weights)!=set(keys) or not weights or any(not isinstance(w,(int,float)) or isinstance(w,bool) or not math.isfinite(w) or w<=0 for w in weights.values()):
            errors.append(f'{aid}: invalid weights')
        elif not math.isclose(sum(weights.values()),1,abs_tol=1e-10) or any(not math.isclose(w,1/len(keys),abs_tol=1e-10) for w in weights.values()):
            errors.append(f'{aid}: versioned balanced-domain weights changed')
        for source in a.get('sources',[]):
            if source.get('id') not in v.get('sources',{}) or not source.get('use'): errors.append(f'{aid}: missing source or use boundary')
        anchors=a.get('behavior_anchors',{})
        if set(anchors)!= {'low','typical','high'}: errors.append(f'{aid}: behavior anchors incomplete')
        if not any(c.get('attribute_id')==aid for c in v.get('challenge_cases',[])): errors.append(f'{aid}: no challenge')
    if len(set(components))!=len(components): errors.append('Duplicate component ID')
    for sid,s in v.get('sources',{}).items():
        if s.get('id')!=sid: errors.append(f'{sid}: source key/ID mismatch')
        for key in ('title','authors','published','accessed','url','locator','inspection','scope','limitations','rights'):
            if key=='published' and s.get(key) is None and s.get('publication_note'): continue
            if not s.get(key): errors.append(f'{sid}: missing source {key}')
        if not s.get('url','').startswith('https://'): errors.append(f'{sid}: invalid source URL')
    cases=v.get('challenge_cases',[])
    if len({c.get('id') for c in cases})!=len(cases): errors.append('Duplicate challenge ID')
    for c in cases:
        if c.get('attribute_id') not in ids: errors.append('Unknown challenge attribute')
        for key in ('scenario','expected','forbidden','case_kind'):
            if not c.get(key): errors.append(f'{c.get("id")}: missing {key}')
    policy=v.get('scoring_policy',{})
    if tuple(policy.get('release_gates',[]))!=GATES: errors.append('Release gates changed')
    q=policy.get('quantile_knots',[]);s=policy.get('score_knots',[])
    if len(q)!=len(s) or len(q)<2 or q[0]!=0 or q[-1]!=1 or any(x>=y for x,y in zip(q,q[1:])):
        errors.append('Invalid calibration knots')
    elif s[0]!=1 or s[-1]!=20 or any(x>y for x,y in zip(s,s[1:])): errors.append('Invalid score knots')
    return errors

def unit_quantile(x):
    if isinstance(x,bool) or not isinstance(x,(int,float)) or not math.isfinite(x) or not 0<=x<=1:
        raise ValueError('Expected finite quantile in [0,1]')
    return x

def balanced_index(attribute, quantiles):
    """Candidate H from already calibrated facet quantiles; never fills gaps."""
    required={c['id'] for c in attribute['components']}
    if set(quantiles)!=required: raise ValueError('Complete exact facet battery required; no imputation')
    normal=NormalDist()
    return sum(attribute['candidate_weights'][key]*normal.inv_cdf(max(.005,min(.995,unit_quantile(q)))) for key,q in quantiles.items())

def reference_quantile(value, reference):
    if not reference or not all(math.isfinite(x) for x in [value,*reference]): raise ValueError('Finite full reference required')
    return (sum(x<value for x in reference)+.5*sum(x==value for x in reference))/len(reference)

def score_quantile(q, policy):
    q=unit_quantile(q);xs=policy['quantile_knots'];ys=policy['score_knots']
    for i in range(1,len(xs)):
        if q<=xs[i]:
            value=ys[i-1]+(q-xs[i-1])/(xs[i]-xs[i-1])*(ys[i]-ys[i-1])
            return max(1,min(20,math.floor(value+.5)))
    raise ValueError('Malformed policy')

def publication_state(policy, gate_records, interval, sensitivity_delta=0):
    """Mechanical gate evaluation only; caller must substantiate real audit records.
    weight_sensitivity_passed concerns an evaluable registered test. A >2-point
    change blocks the point even if all basic calibration gates passed.
    """
    for gate in policy['release_gates']:
        record=gate_records.get(gate,{})
        if record.get('passed') is not True or not record.get('evidence') or not record.get('method_version'):
            return 'unrated'
    if interval is None or len(interval)!=2: return 'unrated'
    lo,hi=interval
    if any(isinstance(x,bool) or not isinstance(x,int) or not 1<=x<=20 for x in interval) or lo>hi:
        return 'unrated'
    if not isinstance(sensitivity_delta,(int,float)) or not math.isfinite(sensitivity_delta) or sensitivity_delta<0: return 'unrated'
    if hi-lo>policy['point_precision_gate']['maximum_integer_score_span'] or sensitivity_delta>2:
        return 'range_only'
    return 'released_point_with_interval'

def markdown_value(v,depth=0):
    if isinstance(v,dict):
        return '\n'.join(f'**{k.replace("_"," ").capitalize()}:** {markdown_value(x,depth+1)}\n' for k,x in v.items())
    if isinstance(v,list):
        return '\n'.join(f'- {markdown_value(x,depth+1)}' for x in v)
    return str(v)

def render(v,out,registry_hash,markdown_only=False):
    out=Path(out);out.mkdir(parents=True,exist_ok=True)
    intro=f'Attribute contracts {v["version"]}. Operational proposals; empirical measurement validation is unvalidated. All production grades remain unrated. Registry SHA256: {registry_hash}.'
    index=['# Rinkcraft attribute definitions','',intro,'','Generated from the authored registry; edit the registry and regenerate.','','| Branch | Attribute | Required facets |','|---|---|---|']
    sections=[]
    for a in v['attributes']:
        filename=a['id'].lower()+'.md'
        index.append(f'| {a["branch"]} | [{a["name"]}]({filename}) | '+', '.join(c['label'] for c in a['components'])+' |')
        lines=[f'# {a["name"]} — {a["branch"]}','',intro,'','[Shared policy](policy.md) · [All attributes](README.md)','']
        for key in ATTRIBUTE_FIELDS:
            lines.extend([f'## {key.replace("_"," ").capitalize()}',''])
            if key=='components':
                for c in a['components']:
                    lines.extend([f'### {c["label"]} ({c["id"]})','',markdown_value(c),''])
            elif key=='sources':
                for ref in a[key]:
                    s=v['sources'][ref['id']]
                    lines.extend([f'- [{s["id"]}: {s["title"]}]({s["url"]}) — {ref["use"]}',''])
            else: lines.extend([markdown_value(a[key]),''])
        lines.extend(['## Challenge cases','',markdown_value([c for c in v['challenge_cases'] if c['attribute_id']==a['id']]),''])
        (out/filename).write_text('\n'.join(lines),encoding='utf-8')
        esc=html.escape
        components=''.join(f'<details><summary>{esc(c["label"])}</summary><dl>'+''.join(f'<dt>{esc(k.replace("_"," "))}</dt><dd>{esc(str(x))}</dd>' for k,x in c.items())+'</dl></details>' for c in a['components'])
        sources=''.join(f'<li><a href="{esc(v["sources"][r["id"]]["url"],quote=True)}">{esc(r["id"])}</a>: {esc(r["use"])}</li>' for r in a['sources'])
        sections.append(f'<article id="{a["id"]}"><small>{a["branch"]} · Unvalidated · Unrated</small><h2>{esc(a["name"])}</h2><p>{esc(a["construct"])}</p>{components}<details><summary>Evidence, boundaries and challenges</summary><pre>{esc(json.dumps({k:a[k] for k in ATTRIBUTE_FIELDS if k not in ("construct","components","sources")},indent=2,ensure_ascii=False))}</pre></details><ul>{sources}</ul></article>')
    (out/'README.md').write_text('\n'.join(index)+'\n\n[Shared policy and full source scopes](policy.md)\n',encoding='utf-8')
    policy={k:v[k] for k in ('scope','observation_policy','evidence_policy','scoring_policy','migration','sources')}
    (out/'policy.md').write_text('# Shared evidence and scoring policy\n\n'+intro+'\n\n'+markdown_value(policy)+'\n',encoding='utf-8')
    if markdown_only: return
    nav=''.join(f'<a href="#{a["id"]}">{html.escape(a["name"])} ({a["branch"]})</a>' for a in v['attributes'])
    page='<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft attribute contracts</title><style>body{margin:0 auto;max-width:1100px;padding:24px;font:16px/1.5 system-ui;overflow-wrap:anywhere;background:#edf3f6;color:#122c3d}h1{border-bottom:5px solid #be3144}nav{display:flex;flex-wrap:wrap;gap:8px}nav a{background:#fff;padding:6px 10px;border:1px solid #9baeb8;border-radius:4px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,450px),1fr));gap:16px;margin-top:24px}article{background:#fff;border:1px solid #9baeb8;border-top:4px solid #247a94;padding:18px;min-width:0;scroll-margin-top:16px}summary{cursor:pointer;font-weight:650;padding:8px 0}dt{font-weight:650;margin-top:8px}dd{margin-left:12px}pre{white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.5 ui-monospace}a{color:#175a72;overflow-wrap:anywhere}small{color:#5d3342}*{box-sizing:border-box}</style><h1>Rinkcraft • Evidence before attributes</h1><p>'+html.escape(intro)+'</p><p>Six skater attributes, six separate goalie attributes. Expand any facet to inspect its opportunities, measurement, failures and disconfirming test.</p><nav>'+nav+'</nav><main>'+''.join(sections)+'</main><details><summary>Shared policy, score semantics and full source scopes</summary><pre>'+html.escape(json.dumps(policy,indent=2,ensure_ascii=False))+'</pre></details></html>'
    (out/'index.html').write_text(page,encoding='utf-8')
    (out/'registry.json').write_text(json.dumps(v,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--registry',required=True,type=Path)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--markdown-only',action='store_true')
    args=parser.parse_args();raw=args.registry.read_bytes();v=load(args.registry);errors=validate(v)
    if errors: raise SystemExit('\n'.join(errors))
    digest=hashlib.sha256(raw).hexdigest()
    if args.output: render(v,args.output,digest,args.markdown_only)
    print(json.dumps({'definition_contracts':len(v['attributes']),'components':sum(len(a['components']) for a in v['attributes']),'challenge_cases':len(v['challenge_cases']),'registry_sha256':digest,'structural_validation':'passed','empirical_validation':'not_established','production_grades_released':False}))
if __name__=='__main__': main()
