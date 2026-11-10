import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
import numpy as np
sys.path.insert(0,str(Path(__file__).parents[1]))
import rinkcraft_temporal_finishing as m

def fixture():
 c=dict(schema='icelines.temporal-finishing-config.v1',protocol='synthetic-test',calibration_season=20232024,recent_season=20242025,target_season=20252026,policies=['league','older','recent','pooled'],interval_probability=.9,minimum_change_xg_per_season=10.,meaningful_ratio=1.2,bootstrap_seed=42,bootstrap_trials=100,prior_strength_factors=[.5,1.,2.],limits=['Synthetic only'])
 sc=dict(minimum_reference_games=10,minimum_reference_minutes=300,prior_shape_bounds=[.05,100000])
 annual={y:{i:dict(player_id=i,name=f'Synthetic {i}',position='D' if i%3==0 else 'C',season=y,goals=i+offset,xg=10.+i,games=40,minutes=500) for i in range(1,9)} for y,offset in [(20232024,0),(20242025,2),(20252026,3)]}
 identities=[dict(player_id=i,name=f'Synthetic {i}',position='D' if i%3==0 else 'C') for i in range(1,9)]+[dict(player_id=99,name='Synthetic goalie',position='G')]
 return c,sc,annual,identities

class TemporalTests(unittest.TestCase):
 def setUp(self):self.c,self.sc,self.a,self.ids=fixture()
 def run_model(self):return m.analyze(self.a,self.ids,self.sc,self.c)
 def test_target_goals_do_not_change_prior_or_forecasts(self):
  a=self.run_model()
  for r in self.a[20252026].values():r['goals']+=10
  b=self.run_model();self.assertEqual(a['prior'],b['prior']);self.assertEqual(a['replication'],b['replication'])
  for k in a['target']['metrics']:self.assertEqual(a['target']['metrics'][k]['predicted_goals'],b['target']['metrics'][k]['predicted_goals'])
  self.assertNotEqual(a['target']['metrics']['pooled']['mean_poisson_deviance'],b['target']['metrics']['pooled']['mean_poisson_deviance'])
 def test_recent_outcomes_update_recent_and_pool_only(self):
  a=self.run_model()
  for r in self.a[20242025].values():r['goals']+=5
  b=self.run_model();self.assertEqual(a['prior'],b['prior']);self.assertEqual(a['target']['metrics']['older']['predicted_goals'],b['target']['metrics']['older']['predicted_goals']);self.assertNotEqual(a['target']['metrics']['recent']['predicted_goals'],b['target']['metrics']['recent']['predicted_goals'])
 def test_showcase_does_not_fit_reference_or_evaluation(self):
  a=self.run_model();self.ids=self.ids[:1];b=self.run_model();self.assertEqual(a['prior'],b['prior']);self.assertEqual(a['target'],b['target']);self.assertEqual(len(b['profiles']),1)
 def test_ratio_distribution_matches_independent_gamma_draws(self):
  prior={'shape':4.,'rate':4.};older=dict(goals=12,xg=10.,position='C');newer=dict(goals=25,xg=12.,position='C');r=m.change(older,newer,prior,self.c);rng=np.random.default_rng(123);ratio=rng.gamma(29.,1/16.,120000)/rng.gamma(16.,1/14.,120000)
  self.assertAlmostEqual(r['probability_increase_beyond_threshold'],float(np.mean(ratio>1.2)),delta=.006)
  self.assertAlmostEqual(r['probability_decrease_beyond_threshold'],float(np.mean(ratio<.8)),delta=.006)
  self.assertAlmostEqual(r['ratio_median'],float(np.median(ratio)),delta=.012)
  self.assertTrue(np.allclose(r['ratio_interval'],np.quantile(ratio,[.05,.95]),atol=.025))
 def test_ratio_symmetry_and_inverse_intervals(self):
  p={'shape':8.,'rate':8.};x=dict(goals=10,xg=10.,position='D');y=dict(goals=25,xg=15.,position='D')
  r=m.change(x,x,p,self.c);self.assertAlmostEqual(r['probability_newer_gt_older'],.5)
  a=m.change(x,y,p,self.c);b=m.change(y,x,p,self.c);self.assertAlmostEqual(a['ratio_median']*b['ratio_median'],1);self.assertAlmostEqual(a['ratio_interval'][0]*b['ratio_interval'][1],1)
 def test_missing_low_exposure_and_real_zero(self):
  p={'shape':4.,'rate':4.};x=dict(goals=0,xg=10.,games=10,minutes=300,position='D')
  self.assertEqual(m.change(None,x,p,self.c)['status'],'missing_season');self.assertEqual(m.change({**x,'xg':2},x,p,self.c)['status'],'insufficient_exposure');self.assertIsNone(m.estimate(None,p,.9));self.assertEqual(m.estimate(x,p,.9)['goals'],0)
 def test_all_sensitivity_factors_reported_central_unchanged(self):
  r=self.run_model();self.assertEqual([x['factor'] for x in r['prior_strength_sensitivity']],[.5,1.,2.]);central=r['prior_strength_sensitivity'][1];self.assertEqual(central['target_metrics'],r['target']['metrics']);self.assertFalse(r['production_grades_released']);self.assertFalse(r['empirical_capability_validation_established'])
 def test_goalies_not_given_skater_component(self):
  p=self.run_model()['profiles'][-1];self.assertEqual(p['status'],'not_applicable_goalie');self.assertEqual(p['annual'],[]);self.assertIsNone(p['change'])
 def test_paired_bootstrap_deterministic_and_position_totals(self):
  a=self.run_model();b=self.run_model();self.assertEqual(a['target']['comparisons'],b['target']['comparisons']);groups=a['target']['position_checks'];self.assertEqual(sum(g['players'] for g in groups.values()),8);self.assertEqual(sum(g['observed_goals'] for g in groups.values()),a['target']['observed_goals'])
 def test_invalid_config_and_source_mutation(self):
  for change in [dict(recent_season=20252026),dict(meaningful_ratio=2.),dict(meaningful_ratio=1.),dict(prior_strength_factors=[0,1]),dict(bootstrap_trials=True),dict(policies=['recent'])]:
   c=copy.deepcopy(self.c);c.update(change)
   with self.subTest(change=change),self.assertRaises(ValueError):m.validate(c)
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'input';p.write_bytes(b'sealed');entry={'path':'input','sha256':hashlib.sha256(b'sealed').hexdigest(),'bytes':6};self.assertEqual(m.sealed(Path(d),entry),p);p.write_bytes(b'edited')
   with self.assertRaises(ValueError):m.sealed(Path(d),entry)
 def test_report_json_roundtrip_and_no_script_injection(self):
  r=self.run_model();r['profiles'][0]['name']='</script><img> &quot;';page=m.render(r);payload=page.split('id="payload">')[1].split('</script>')[0];self.assertEqual(json.loads(payload),r);self.assertNotIn('</script><img>',page)

if __name__=='__main__':unittest.main()
