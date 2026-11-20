import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).parents[1]))
from test_rinkcraft_temporal_finishing import fixture
import rinkcraft_position_finishing as m

class PositionTests(unittest.TestCase):
 def setUp(self):
  c,self.source,self.annual,self.identities=fixture();self.c={**c,'schema':'icelines.position-finishing-config.v1','protocol':'synthetic','replication_season':20242025,'policies':m.POLICIES}
 def run_model(self):return m.analyze(self.annual,self.identities,self.source,self.c)
 def test_later_outcomes_cannot_fit_model_or_change_target_predictions(self):
  a=self.run_model()
  for row in self.annual[20252026].values():row['goals']+=5
  b=self.run_model();self.assertEqual(a['model'],b['model']);self.assertEqual(a['evaluations'][0],b['evaluations'][0]);self.assertEqual(a['profiles'][0]['windows'][1]['evidence']['predictions'],b['profiles'][0]['windows'][1]['evidence']['predictions']);self.assertNotEqual(a['evaluations'][1]['metrics'],b['evaluations'][1]['metrics'])
 def test_middle_outcomes_update_history_but_not_frozen_factors(self):
  a=self.run_model()
  for row in self.annual[20242025].values():row['goals']+=4
  b=self.run_model();self.assertEqual(a['model'],b['model']);self.assertNotEqual(a['profiles'][0]['windows'][1]['evidence']['predictions']['position_player'],b['profiles'][0]['windows'][1]['evidence']['predictions']['position_player'])
 def test_factor_definition_and_normalized_exposure(self):
  r=self.run_model();f=r['model']['position_factors']['D'];self.assertAlmostEqual(f['factor'],f['goals']/f['xg'])
  row=dict(player_id=1,position='D',xg=10,goals=5);model={'prior':{'mean':1,'shape':4,'rate':4},'position_factors':{'D':{'factor':2},'F':{'factor':.5}}}
  p=m.predict(row,[dict(position='C',xg=6,goals=3),dict(position='D',xg=2,goals=1)],model)
  self.assertAlmostEqual(p['position_player'],(4+4)/(4+3+4)*20);self.assertEqual(p['position_baseline'],20)
 def test_new_player_fallback_and_no_group_support(self):
  r=self.run_model();row=dict(position='D',xg=10);p=m.predict(row,[],r['model']);self.assertEqual(p['global_player'],p['global_baseline']);self.assertEqual(p['position_player'],p['position_baseline'])
  self.annual[20232024]={i:r for i,r in self.annual[20232024].items() if r['position']!='D'}
  with self.assertRaises(ValueError):self.run_model()
 def test_showcase_does_not_change_fit_or_evaluation(self):
  a=self.run_model();self.identities=self.identities[:1];b=self.run_model();self.assertEqual(a['model'],b['model']);self.assertEqual(a['evaluations'],b['evaluations'])
 def test_switch_flag_and_goalie_exclusion(self):
  self.annual[20242025][1]['position']='D';r=self.run_model();self.assertTrue(r['profiles'][0]['windows'][0]['evidence']['position_group_switch']);self.assertEqual(r['profiles'][-1]['status'],'not_applicable_goalie');self.assertTrue(all(p['headline_score'] is None for p in r['profiles']))
 def test_paired_bootstrap_and_actual_totals(self):
  a=self.run_model();b=self.run_model();self.assertEqual(a['evaluations'],b['evaluations'])
  for e in a['evaluations']:self.assertEqual(sum(g['observed_goals'] for g in e['positions'].values()),e['observed_goals']);self.assertFalse(e['position_comparability_validated'])
 def test_invalid_seasons_and_scope(self):
  self.c['target_season']=20242025
  with self.assertRaises(ValueError):self.run_model()
  with self.assertRaises(ValueError):m.group({'position':'G'})
 def test_html_escapes_untrusted_text(self):
  r=self.run_model();r['provenance']={};r['interpretation']='<script>unsafe</script>';self.assertNotIn('<script>',m.render(r))

if __name__=='__main__':unittest.main()
