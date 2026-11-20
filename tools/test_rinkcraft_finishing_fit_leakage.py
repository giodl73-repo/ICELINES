"""Real-executable target-goal perturbation: JOB BASE_OUTPUT EXE NEW_DIRECTORY.

Creates explicitly synthetic source bytes; never alters captured files.
"""
import csv
import hashlib
import json
import subprocess
import sys
from pathlib import Path

job_path, baseline, executable, output = map(Path,sys.argv[1:])
assert not output.exists()
output.mkdir(parents=True)
job=json.loads(job_path.read_text(encoding='utf-8-sig'))
c=json.loads(Path(job['legacy_config']['path']).read_bytes())
before=json.loads((baseline/'replay.json').read_bytes())
pid=before['profiles'][0]['player_id'];season=c['holdout_season']
for s in c['sources']:
    if s['kind']!='skaters':continue
    original=Path(job['data_root'])/s['path']
    assert hashlib.sha256(original.read_bytes()).hexdigest()==s['sha256']
    s['path']=str(original.resolve())
    if s['season']!=season:continue
    with original.open(encoding='utf-8-sig',newline='')as f:
        reader=csv.DictReader(f);fields=reader.fieldnames;rows=list(reader)
    changed=0
    for r in rows:
        if int(r['playerId'])==pid and r['situation']=='5on5':
            r['I_F_goals']=str(float(r['I_F_goals'])+1);changed+=1
    assert changed==1
    synthetic=output/'synthetic-target-goals.csv'
    with synthetic.open('w',encoding='utf-8',newline='')as f:
        writer=csv.DictWriter(f,fieldnames=fields,lineterminator='\n');writer.writeheader();writer.writerows(rows)
    raw=synthetic.read_bytes();s.update(path=str(synthetic.resolve()),bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest(),url='urn:rinkcraft:test:synthetic-target-goal-perturbation')
config_path=output/'synthetic-config.json';config_path.write_text(json.dumps(c,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
derived={**job,'legacy_config':dict(path=str(config_path.resolve()),sha256=hashlib.sha256(config_path.read_bytes()).hexdigest())}
derived_path=output/'synthetic-job.json';derived_path.write_text(json.dumps(derived,indent=2)+'\n',encoding='utf-8')
run=subprocess.run([str(executable.resolve()),str(derived_path.resolve()),str((output/'result').resolve())],capture_output=True,text=True)
assert run.returncode==0,run.stderr
after=json.loads((output/'result/replay.json').read_bytes())
assert before['prior']==after['prior'], 'held-out goals changed fitted prior'
old={p['player_id']:p for p in before['profiles']}
new={p['player_id']:p for p in after['profiles']}
assert old.keys()==new.keys()
for actor,p in old.items():
    a,b=p['forecast_from_training_only'],new[actor]['forecast_from_training_only']
    for key in ['goals','provider_xg','multiplier','interval','baseline_only','historical_rows','no_historical_rows']:
        assert a[key]==b[key]
    assert [p['expected_goals'] for p in a['policies']]==[p['expected_goals'] for p in b['policies']]
assert new[pid]['target_goals']==old[pid]['target_goals']+1
assert new[pid]['target_season_component']['posterior']['shape']==old[pid]['target_season_component']['posterior']['shape']+1
assert before['evaluation']['metrics']!=after['evaluation']['metrics']
receipt=dict(schema='rinkcraft.finishing-leakage-check.v1',synthetic_target_player=pid,target_season=season,target_goal_delta=1,prior_byte_values_unchanged=True,forecasts_unchanged=len(old),evaluation_changed=True,retrospective_component_changed=True,captured_source_bytes_untouched=True)
(output/'verification.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
print(json.dumps(receipt))
