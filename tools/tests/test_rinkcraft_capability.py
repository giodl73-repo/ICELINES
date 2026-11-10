"""Neutral mathematical/provenance checks, not hockey construct validation."""
import csv
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('pilot',Path(__file__).resolve().parents[1]/'rinkcraft_capability.py')
pilot=importlib.util.module_from_spec(spec);spec.loader.exec_module(pilot)

class CapabilityPilotTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        self.config={'schema':'icelines.capability-pilot-config.v1','protocol':'synthetic-test',
          'data_root':'.','sources':[],'train_seasons':[20232024,20242025],'holdout_season':20252026,
          'situation':'5on5','minimum_reference_minutes':0,'minimum_reference_games':0,
          'minimum_scored_xg':10,'prior_shape_bounds':[.05,100000],'interval_probability':.9,
          'bootstrap_seed':7,'bootstrap_trials':100,'score_quantile_knots':[0,.5,1],
          'score_value_knots':[1,10,20],'selected_players':[1,2,3]}
        for year in self.config['train_seasons']+[self.config['holdout_season']]:
            file=self.root/f'{year}.csv'
            with file.open('w',newline='',encoding='utf-8') as f:
                writer=csv.DictWriter(f,fieldnames=['playerId','name','position','season','situation','games_played','icetime','I_F_goals','I_F_xGoals'])
                writer.writeheader()
                for i in range(1,9):
                    writer.writerow({'playerId':i,'name':f'Synthetic {i}','position':'D' if i==1 else 'C','season':year//10000,'situation':'5on5','games_played':30,'icetime':30000,'I_F_goals':i*2,'I_F_xGoals':10})
            self.config['sources'].append({'season':year,'kind':'skaters','path':file.name,'url':'https://example.com/synthetic','sha256':pilot.digest(file.read_bytes()),'bytes':file.stat().st_size})
        self.path=self.root/'config.json';self.save()
    def save(self):
        self.path.write_text(json.dumps(self.config),encoding='utf-8')
    def test_holdout_goals_do_not_change_prior_or_calibration_scores(self):
        first=pilot.run(self.path)
        source=self.config['sources'][-1];file=self.root/source['path']
        with file.open(newline='',encoding='utf-8') as f:reader=csv.DictReader(f);fields=reader.fieldnames;rows=list(reader)
        for row in rows:row['I_F_goals']=str(int(row['I_F_goals'])+10)
        with file.open('w',newline='',encoding='utf-8') as f:writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader();writer.writerows(rows)
        source['sha256']=pilot.digest(file.read_bytes());source['bytes']=file.stat().st_size;self.save()
        second=pilot.run(self.path)
        self.assertEqual(first['prior'],second['prior']);self.assertEqual(first['calibration_cohort'],second['calibration_cohort'])
        self.assertNotEqual(first['evaluation']['metrics'],second['evaluation']['metrics'])
    def test_source_mutation_rejected(self):
        file=self.root/self.config['sources'][0]['path'];file.write_text('changed',encoding='utf-8')
        with self.assertRaisesRegex(ValueError,'hash/length'):pilot.run(self.path)
    def test_temporal_overlap_and_duplicate_source_rejected(self):
        self.config['holdout_season']=20242025;self.save()
        with self.assertRaisesRegex(ValueError,'strictly precede'):pilot.run(self.path)
        self.config['holdout_season']=20252026;self.config['sources'].append(self.config['sources'][0]);self.save()
        with self.assertRaisesRegex(ValueError,'duplicate source'):pilot.run(self.path)
    def test_duplicate_player_and_invalid_outcomes_rejected(self):
        file=self.root/self.config['sources'][0]['path'];line=file.read_text(encoding='utf-8').splitlines()[1]
        with file.open('a',encoding='utf-8') as f:f.write(line+'\n')
        with self.assertRaisesRegex(ValueError,'duplicate'):pilot.parse_rows(file,20232024,'skaters')
        with self.assertRaisesRegex(ValueError,'invalid'):pilot.posterior(1,0,{'shape':2,'rate':2},self.config)
    def test_posterior_shrinkage_precision_and_sparse_gate(self):
        prior={'shape':10,'rate':10}
        small=pilot.posterior(10,10,prior,self.config);large=pilot.posterior(1000,1000,prior,self.config)
        self.assertEqual(small['effect_mean'],1)
        self.assertLess(large['effect_interval'][1]-large['effect_interval'][0],small['effect_interval'][1]-small['effect_interval'][0])
        self.assertIsNone(pilot.posterior(0,0,prior,self.config)['candidate_score'])
        shrunk=pilot.posterior(5,1,prior,self.config)['effect_mean'];self.assertLess(shrunk,5);self.assertGreater(shrunk,1)
    def test_scale_monotone_and_bounded(self):
        prior={'shape':10,'rate':10};values=[pilot.display_score(i/100,prior,self.config) for i in range(501)]
        self.assertEqual(values,sorted(values));self.assertGreaterEqual(min(values),1);self.assertLessEqual(max(values),20)
    def test_headline_unrated_and_prospective_gate_closed(self):
        result=pilot.run(self.path)
        self.assertFalse(result['headline_scores_released']);self.assertFalse(result['evaluation']['prospective_validation_passed'])
        self.assertTrue(all(a['score'] is None for p in result['profiles'] for a in p['attributes']))
        self.assertEqual(pilot.evaluate({}, {1:{'player_id':1,'position':'C','goals':0,'xg':2,'games':20,'minutes':300}},result['prior'],self.config)['new_players'],1)
    def test_bootstrap_deterministic_and_zero_outcome_defined(self):
        a=pilot.run(self.path);b=pilot.run(self.path);self.assertEqual(a['evaluation'],b['evaluation'])
        self.assertEqual(pilot.poisson_deviance([0],[2]).tolist(),[4])
    def test_goalie_outcomes_never_fill_goalie_capabilities(self):
        file=self.root/'goalie.csv'
        file.write_text('playerId,name,position,season,situation,games_played,icetime,goals,xGoals\n9,Synthetic goalie,G,2025,all,30,100000,80,90\n',encoding='utf-8')
        self.config['sources'].append({'season':20252026,'kind':'goalies','path':file.name,'url':'https://example.com/synthetic','sha256':pilot.digest(file.read_bytes()),'bytes':file.stat().st_size})
        self.config['selected_players'].append(9);self.save()
        result=pilot.run(self.path);goalie=result['profiles'][-1]
        self.assertEqual(goalie['components'],{})
        self.assertEqual([a['name'] for a in goalie['attributes']],list(pilot.GOALIE))
        self.assertTrue(all(a['score'] is None for a in goalie['attributes']))
        self.assertIn('Not assessed: goalie technique',pilot.render_report(result))
    def test_report_escapes_names(self):
        result=pilot.run(self.path);result['profiles'][0]['name']='<img src=x onerror=x>'
        rendered=pilot.render_report(result);self.assertIn('&lt;img',rendered);self.assertNotIn('<img src=x',rendered)

if __name__=='__main__':unittest.main()
