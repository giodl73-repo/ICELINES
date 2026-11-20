"""Independent frozen-CDF/coverage oracle: MANIFEST OUTPUT_DIRECTORY NEW_RECEIPT."""
import bisect
import collections
import hashlib
import json
import math
import sys
from pathlib import Path

manifest_path, output, receipt = map(Path, sys.argv[1:])
m = json.loads(manifest_path.read_text(encoding='utf-8-sig'))
def read(spec):
    raw = Path(spec['path']).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == spec['sha256']
    return json.loads(raw)
source, exposure = read(m['observations']), read(m['exposure'])
records = {(r['season'], r['player_id']): r for r in source['records']}
assert len(records) == len(source['records'])
ledger = json.loads((output/'coverage-ledger.json').read_bytes())
model = json.loads((output/'epoch-model.json').read_bytes())
report = json.loads((output/'epoch-evaluation.json').read_bytes())
assert model['training_seasons'] == sorted(m['training_seasons'])
assert model['weighting'] == 'equal season; within-season empirical midrank CDF'
assert report['manifest_sha256'] == hashlib.sha256(manifest_path.read_bytes()).hexdigest()
settings = source['settings']
def eligible(r):
    return r['games'] >= settings['minimum_games'] and r['minutes'] >= settings['minimum_minutes']
def values(r):
    edge = r.get('edge')
    es = {} if edge is None else {m['key']:m['value'] for m in edge['measures']}
    shots, goals = r['shots'], r['goals']
    return dict(skating_peak_mph=es.get('max-skating-speed'),
                bursts_per_60=None if 'speed-bursts-over-20' not in es else es['speed-bursts-over-20']*60/r['minutes'],
                shot_peak_mph=es.get('max-shot-speed'),shots_per_60=None if shots is None else shots*60/r['minutes'],
                conversion_pct=None if shots is None or goals is None or shots<settings['minimum_conversion_shots'] or not shots or goals>shots else goals*100/shots)
def close(a,b):
    if isinstance(a,dict):
        assert a.keys()==b.keys()
        for k in a: close(a[k],b[k])
    elif isinstance(a,list):
        assert len(a)==len(b)
        for x,y in zip(a,b):close(x,y)
    elif isinstance(a,(float,int)) and isinstance(b,(float,int)):
        assert math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-12),(a,b)
    else:assert a==b,(a,b)

members_checked = cdfs_checked = decisions_checked = 0
def check_epoch(epoch,evaluation):
    global members_checked,cdfs_checked,decisions_checked
    assert evaluation['season']>max(epoch['training_seasons'])
    assert evaluation['epoch_sha256']==epoch['sha256']
    refs={}
    for ref in epoch['references']:
        assert [y['season'] for y in ref['years']]==epoch['training_seasons']
        ids=set();total=0
        for year in ref['years']:
            expected={r['player_id']:values(r)[ref['key']] for r in records.values() if r['season']==year['season'] and r['group']==ref['group'] and eligible(r) and values(r)[ref['key']] is not None}
            actual=dict(year['members']);assert actual.keys()==expected.keys()
            assert len(actual)==len(year['members'])
            assert year['members']==sorted(year['members'],key=lambda p:(p[1],p[0]))
            for pid,value in actual.items():close(expected[pid],value)
            ids.update(actual);total+=len(actual);members_checked+=len(actual)
        assert ref['player_seasons']==total and ref['unique_players']==len(ids)
        assert ref['usable']==all(len(y['members'])>=20 for y in ref['years'])
        refs[(ref['group'],ref['key'])]=ref
    expected_rows={k:r for k,r in records.items() if k[0]==evaluation['season']}
    seen=set();summary=collections.defaultdict(lambda:[0,0,0.]);census=collections.Counter()
    for row in evaluation['rows']:
        key=row['season'],row['player_id'];assert key not in seen;seen.add(key)
        r=expected_rows[key];assert row['group']==r['group'] and row['name']==r['name'] and row['eligible']==eligible(r)
        rs={}
        for metric in row['measurements']:
            ref=refs[(r['group'],metric['key'])];close(metric['value'],values(r)[metric['key']])
            assert metric['reference_usable']==ref['usable']
            assert metric['training_player_seasons']==ref['player_seasons'] and metric['training_unique_players']==ref['unique_players']
            rank=None
            if eligible(r) and metric['value'] is not None and ref['usable']:
                per_year=[]
                for year in ref['years']:
                    pool=[v for _,v in year['members']]
                    lo,hi=bisect.bisect_left(pool,metric['value']),bisect.bisect_right(pool,metric['value'])
                    per_year.append(100*(lo+.5*(hi-lo))/len(pool))
                rank=sum(per_year)/len(per_year)
            close(metric['frozen_percentile'],rank)
            if rank is not None:
                rs[metric['key']]=(rank,ref['player_seasons']);cdfs_checked+=1
                s=summary[(r['group'],metric['key'])];s[0]+=1;s[1]+=rank>=90
                if metric['same_season_percentile'] is not None:s[2]+=abs(rank-metric['same_season_percentile'])
        pairs=[('recorded-skating-speed','skating_peak_mph','bursts_per_60',('top-gear','burst-engine','two-gears')),('recorded-shot-power-volume','shot_peak_mph','shots_per_60',('shot-power','shot-volume','power-volume'))]
        for family,pk,ak,names in pairs:
            candidate=None
            if not eligible(r):state='insufficient_exposure'
            elif pk not in rs or ak not in rs or min(rs[pk][1],rs[ak][1])<20:state='unassessed'
            else:
                state='assessed';p,a=rs[pk][0]>=90,rs[ak][0]>=90
                candidate=names[2] if p and a else names[0] if p else names[1] if a else None
            actual=next(d for d in row['frozen_families'] if d['family']==family)
            assert (actual['state'],actual['candidate'])==(state,candidate)
            census[(r['group'],family,candidate or state)]+=1;decisions_checked+=1
    assert seen==expected_rows.keys()
    for s in evaluation['summary']:
        n,top,shift=summary[(s['group'],s['key'])]
        assert (s['compared'],s['frozen_p90_count'])==(n,top)
        close(s['frozen_p90_share'],top/n);close(s['mean_absolute_percentile_shift'],shift/n)
    assert dict(census)=={(c['group'],c['family'],c['outcome']):c['count'] for c in evaluation['census']}

check_epoch(model,report['validation']);check_epoch(model,report['monitor'])
if report['rolling_origin']:
    check_epoch(report['rolling_origin']['epoch'],report['rolling_origin']['evaluation'])
base=Path(m['base_directory']);profile_hashes={i['path']:i['sha256'] for i in exposure['inputs']}
assert len(profile_hashes)==len(ledger['profiles'])
definitions={a['id']:a for a in ledger['definitions']}
components=context_rows=goalies=0;seen=set()
for p in ledger['profiles']:
    identity=p['season'],p['player_id'];assert identity not in seen;seen.add(identity)
    raw=(base/f"data/{p['season']}/{p['player_id']}.json").read_bytes()
    assert hashlib.sha256(raw).hexdigest()==p['source_profile_sha256']==profile_hashes[f"data/{p['season']}/{p['player_id']}.json"]
    old=json.loads(raw);f=old['numerical_evidence'];goalie=f['group']=='Goalies';goalies+=goalie
    expected={a['id']:a for a in definitions.values() if a['branch']==('goalie' if goalie else 'skater')}
    assert {a['attribute_id'] for a in p['attributes']}==expected.keys()
    for a in p['attributes']:
        assert a['headline_state']=='unrated'
        assert {c['component_id'] for c in a['facets']}=={c['id'] for c in expected[a['attribute_id']]['components']}
        for c in a['facets']:
            components+=1;assert c['measurement_state']=='unmeasured_task' and c['release_state']=='not_calibrated'
            if c['context']:
                context_rows+=1;assert not goalie and c['component_id'] in {'SKA-SPEED','SHO-VELOCITY','SHO-FINISH'}
                for x in c['context']:
                    assert x['scope']=='all strengths'
                    if x['key']=='raw_conversion_pct':
                        r=records[identity];assert x['source_sha256']==r['official_source_sha256'];close(x['value'],r['goals']*100/r['shots'])
                    else:
                        edge=f['edge_evidence'];assert x['source_sha256']==edge['source']['sha256'];actual=next(e for e in edge['measures'] if e['key']==x['key']);close(x['value'],actual['value'])
    close(p['goalie_stopping_context'],f.get('edge_evidence',{}).get('locations') if goalie else None)
assert ledger['summary']==dict(player_seasons=len(seen),goalie_player_seasons=goalies,required_component_rows=components,component_rows_with_context=context_rows,loaded_calibrated_task_estimates=0,released_headline_scores=0)
result=dict(profile_hashes=len(seen),component_rows=components,contextual_component_rows=context_rows,goalie_player_seasons=goalies,reference_members_checked=members_checked,frozen_cdfs_checked=cdfs_checked,family_decisions_checked=decisions_checked,headline_scores_released=0)
assert not receipt.exists()
receipt.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps(result))
for phase in ['validation','monitor']:
    print(phase,json.dumps(report[phase]['summary']))
