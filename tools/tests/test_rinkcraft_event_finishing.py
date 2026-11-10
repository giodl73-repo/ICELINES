import copy
import hashlib
import json
import math
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).parents[1]))
import rinkcraft_event_finishing as m

def fixture():
 c=dict(schema='icelines.event-finishing-config.v1',protocol='synthetic-only',data_kind='synthetic',evaluation_mode='prospective_metadata',input=None,
  baseline=dict(id='synthetic-model',sha256='a'*64,trained_through='2024-09-01',available_from='2024-09-02',feature_columns=['shot_distance','shot_type'],audit_locator='SYNTHETIC ONLY',immutable=True),
  scope=dict(m.SCOPE,season=20242025),windows=dict(start='2024-10-01',replication_start='2025-01-01',test_start='2025-02-01',end_exclusive='2025-07-01'),
  model=dict(league_prior_sd=2.,player_prior_sd=.5,sensitivity_prior_sd=[.25,1.],minimum_stability_attempts_per_block=2),
  bootstrap=dict(seed=42,trials=100,interval_probability=.9),limits=['Synthetic only'])
 events=[]
 for block,dates in enumerate([['2024-10-05','2024-10-06'],['2025-01-05','2025-01-06'],['2025-02-05','2025-02-06']]):
  for game,when in enumerate(dates):
   gid=100+block*10+game
   for pid in [1,2,3]:
    for shot in range(4):
     events.append(dict(c['scope'],game_id=gid,event_id=pid*10+shot,game_date=when,shooter_id=pid,goalie_id=999,position_group='D' if pid==1 else 'F',outcome='goal' if shot==pid-1 and game==0 else 'saved' if shot%2 else 'missed',baseline_id='synthetic-model',baseline_probability=.1+shot*.04))
 return c,events

class EventTests(unittest.TestCase):
 def setUp(self):self.c,self.e=fixture()
 def test_synthetic_run_is_not_empirical_validation(self):
  r=m.experiment(self.c,self.e);self.assertEqual(r['status'],'synthetic_demonstration');self.assertFalse(r['empirical_validation_established']);self.assertFalse(r['production_grades_released']);self.assertEqual(r['event_counts'],dict(train=24,replication=24,test=24));self.assertEqual(r['game_counts'],dict(train=2,replication=2,test=2))
 def test_heldout_goals_cannot_change_training_or_stability(self):
  a=m.experiment(self.c,self.e)
  for e in self.e:
   if e['game_date']>='2025-02-01':e['outcome']='goal'
  b=m.experiment(self.c,self.e);self.assertEqual(a['training_model'],b['training_model']);self.assertEqual(a['stability_pairs'],b['stability_pairs']);self.assertNotEqual(a['heldout_metrics'],b['heldout_metrics'])
 def test_frozen_baseline_vintage_and_feature_leakage(self):
  for change in [dict(trained_through='2025-01-01'),dict(available_from='2025-01-01'),dict(immutable=False),dict(feature_columns=['outcome']),dict(feature_columns=['shooter_id']),dict(sha256='unknown')]:
   c=copy.deepcopy(self.c);c['baseline'].update(change)
   with self.subTest(change=change),self.assertRaises(ValueError):m.experiment(c,self.e)
 def test_retrospective_temporal_is_not_prospective_availability(self):
  self.c['evaluation_mode']='retrospective_temporal';self.c['baseline']['available_from']='2026-11-09';r=m.experiment(self.c,self.e)
  self.assertFalse(r['baseline_available_by_start']);self.assertFalse(r['prospective_validation_established'])
  self.c['baseline']['trained_through']='2025-03-01'
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
 def test_missing_baseline_and_mixed_version_rejected(self):
  self.c['baseline']=None
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
  self.c,_=fixture();self.e[0]['baseline_id']='another'
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
 def test_duplicate_events_and_same_game_on_two_dates(self):
  self.e.append(self.e[0])
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
  self.e.pop();self.e[-1]['game_id']=self.e[0]['game_id']
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
 def test_manpower_empty_net_and_blocked_scope(self):
  for change in [dict(situation='5v4'),dict(both_goalies_present=False),dict(both_goalies_present=1),dict(outcome='blocked'),dict(season=20252026),dict(position_group='G')]:
   e=copy.deepcopy(self.e);e[0].update(change)
   with self.subTest(change=change),self.assertRaises(ValueError):m.experiment(self.c,e)
 def test_invalid_probabilities_and_ids(self):
  for value in (0,1,math.nan,math.inf,True,None):
   e=copy.deepcopy(self.e);e[0]['baseline_probability']=value
   with self.subTest(value=value),self.assertRaises(ValueError):m.experiment(self.c,e)
  self.e[0]['goalie_id']=None
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
 def test_missing_block_and_noncanonical_dates(self):
  with self.assertRaises(ValueError):m.experiment(self.c,[e for e in self.e if e['game_date']<'2025-02-01'])
  self.e[0]['game_date']='20241005'
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
 def test_unknown_player_uses_league_adjustment(self):
  for e in self.e:
   if e['game_date']>='2025-02-01':e['shooter_id']=5;e['position_group']='F'
  r=m.experiment(self.c,self.e);self.assertEqual(r['unseen_heldout_players'],[5]);self.assertEqual(r['heldout_metrics']['league_adjustment'],r['heldout_metrics']['player_MAP'])
 def test_game_cluster_bootstrap_is_paired_and_deterministic(self):
  groups={1:[-2.,10],2:[4.,20]}
  a=m.clustered_interval(groups,self.c);self.assertEqual(a,m.clustered_interval(groups,self.c));self.assertGreaterEqual(a[0],-.2);self.assertLessEqual(a[1],.2)
 def test_posterior_optimum_and_zero_data(self):
  self.assertEqual(m.fit_shift([],0,.5),0)
  beta=m.fit_shift(self.e,0,.5);g=-beta/(.5**2)+sum((e['outcome']=='goal')-m.logistic(m.logit(e['baseline_probability'])+beta) for e in self.e);self.assertAlmostEqual(g,0,places=7)
  self.assertTrue(math.isfinite(m.loss(True,0)));self.assertTrue(math.isfinite(m.loss(False,1)))
 def test_no_input_is_awaiting_not_a_successful_run(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'config.json';p.write_text(json.dumps(self.c));_,r=m.run(p);self.assertEqual(r['status'],'awaiting_synthetic_event_data');self.assertIsNone(r['heldout_metrics'])
   self.c['schema']='wrong';p.write_text(json.dumps(self.c))
   with self.assertRaises(ValueError):m.run(p)
 def test_missing_fields_and_observed_provenance_gate(self):
  self.e[0].pop('shooter_id')
  with self.assertRaises(ValueError):m.experiment(self.c,self.e)
  self.c,self.e=fixture();self.c['data_kind']='observed'
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);data=root/'events.jsonl';data.write_text('\n'.join(json.dumps(e) for e in self.e));self.c['input']={'path':'events.jsonl','sha256':hashlib.sha256(data.read_bytes()).hexdigest()};p=root/'config.json';p.write_text(json.dumps(self.c))
   with self.assertRaises(ValueError):m.run(p)
 def test_sealed_input_mutation_and_safe_report(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);data=root/'events.jsonl';data.write_text('\n'.join(json.dumps(e) for e in self.e),encoding='utf-8');self.c['input']={'path':'events.jsonl','sha256':hashlib.sha256(data.read_bytes()).hexdigest()};config=root/'config.json';config.write_text(json.dumps(self.c));_,r=m.run(config);self.assertEqual(r['status'],'synthetic_demonstration');data.write_text('modified')
   with self.assertRaises(ValueError):m.run(config)
  self.assertNotIn('<script>',m.render({'status':'test','unsafe':'<script>'},self.c))

if __name__=='__main__':unittest.main()
