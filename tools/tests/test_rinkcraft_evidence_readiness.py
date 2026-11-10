import copy
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).parents[1]))
import rinkcraft_evidence_readiness as m

class ReadinessTests(unittest.TestCase):
 def setUp(self):
  self.reg={'version':'synthetic','attributes':[]};self.cfg={'facets':[],'attribute_context':{},'policy':'Synthetic only'}
  for aid,branch,name,ids in [('CAP-SKATING','skater','Skating',['SKA-SPEED','SKA-ACCEL']),('CAP-SHOOTING','skater','Shooting',['SHO-FINISH','SHO-VELOCITY']),('CAP-G-REBOUNDS','goalie','Rebounds',['GRE-ABSORB'])]:
   cs=[dict(id=i,label=i,inputs=['task input'],disconfirming_test='challenge') for i in ids]
   self.reg['attributes'].append(dict(id=aid,branch=branch,name=name,components=cs));self.cfg['attribute_context'][aid]=['goals']
   for i in ids:self.cfg['facets'].append(dict(attribute_id=aid,component_id=i,available_kind={'SHO-FINISH':'experimental_partial_estimate','SKA-SPEED':'historical_raw_context','SHO-VELOCITY':'historical_raw_context'}.get(i,'not_collected'),inference_limit='Synthetic limit'))
  self.pilot={'headline_scores_released':False,'profiles':[dict(player_id=1,name='Synthetic skater',position='D',components={'finishing':{'latest_season':20252026,'latest_season_evidence':{'status':'insufficient_exposure','goals':0,'candidate_score':None}}}),dict(player_id=2,name='Synthetic goalie',position='G',components={})]}
  self.tables={(s,k):{pid:{'values':{'goals':0.0},'situation':'5on5' if k=='skaters' else 'all'}} for s in (20242025,20252026) for k,pid in [('skaters',1),('goalies',2)]}
  self.edge={'player':{'id':1},'skatingSpeed':{'speedMax':{'imperial':24.,'percentile':.99},'burstsOver20':{'value':4}},'topShotSpeed':{'imperial':90.,'percentile':.7}}
 def build(self):return m.build(self.reg,self.cfg,self.pilot,self.tables,self.edge,20242025)
 def test_historical_measurement_does_not_time_travel(self):
  p=self.build()['profiles'][0];old,new=p['windows']
  self.assertEqual(old['attributes'][0]['facets'][0]['state'],'historical_raw_context')
  self.assertEqual(new['attributes'][0]['facets'][0]['state'],'not_collected')
  self.assertIsNone(new['attributes'][0]['facets'][0]['measurement'])
 def test_sparse_finishing_not_backfilled_to_past(self):
  p=self.build()['profiles'][0];old,new=p['windows']
  self.assertEqual(old['attributes'][1]['facets'][0]['state'],'not_collected')
  f=new['attributes'][1]['facets'][0];self.assertEqual(f['state'],'insufficient_exposure');self.assertIsNone(f['measurement']['candidate_score'])
 def test_missing_row_distinct_from_real_zero(self):
  self.tables[(20252026,'skaters')]={};p=self.build()['profiles'][0];old,new=p['windows']
  self.assertEqual(old['attributes'][0]['adjacent_outcomes']['goals'],0)
  self.assertIsNone(new['attributes'][0]['adjacent_outcomes']['goals']);self.assertFalse(new['source_row_present'])
  self.assertEqual(new['attributes'][1]['facets'][0]['state'],'not_collected')
 def test_goalie_scope_and_no_proxy_promotion(self):
  p=self.build()['profiles'][1]
  for w in p['windows']:
   a=w['attributes'][0];self.assertEqual(a['context_scope'],'all');self.assertEqual(a['validated_facets'],0);self.assertIsNone(a['headline_score']);self.assertFalse(a['facets'][0]['validated'])
 def test_mapping_must_be_exact_and_current(self):
  for kind in ('missing','duplicate','wrong_parent','unsupported'):
   with self.subTest(kind=kind):
    original=copy.deepcopy(self.cfg)
    if kind=='missing':self.cfg['facets'].pop()
    if kind=='duplicate':self.cfg['facets'].append(self.cfg['facets'][0])
    if kind=='wrong_parent':self.cfg['facets'][0]['attribute_id']='CAP-SHOOTING'
    if kind=='unsupported':self.cfg['facets'][0]['available_kind']='validated'
    with self.assertRaises(ValueError):self.build()
    self.cfg=original
 def test_duplicate_players_and_released_pilot_rejected(self):
  self.pilot['profiles'].append(self.pilot['profiles'][0])
  with self.assertRaises(ValueError):self.build()
  self.pilot['profiles'].pop();self.pilot['headline_scores_released']=True
  with self.assertRaises(ValueError):self.build()
 def test_acquisition_queue_cannot_omit_facets(self):
  self.cfg['next_steps']=[dict(priority=1,name='Synthetic request',request='input',acceptance='criterion',current_blocker='missing',facets=['SHO-FINISH'])]
  with self.assertRaises(ValueError):self.build()
  self.cfg['next_steps'][0]['facets']=[f['component_id'] for f in self.cfg['facets']]
  self.assertEqual(len(self.build()['next_steps']),1)
 def test_sealed_source_rejects_mutation(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'input';p.write_bytes(b'test');entry={'path':'input','sha256':hashlib.sha256(b'test').hexdigest(),'bytes':4}
   self.assertEqual(m.sealed(Path(d),entry),p)
   p.write_bytes(b'edit')
   with self.assertRaises(ValueError):m.sealed(Path(d),entry)
 def test_csv_zero_missing_duplicate_and_absent_field(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'input.csv';header='playerId,season,situation,goals\n'
   p.write_text(header+'1,2025,5on5,0\n2,2025,5on5,\n',encoding='utf-8')
   rows=m.read_rows(p,20252026,'skaters',['goals']);self.assertEqual(rows[1]['values']['goals'],0);self.assertIsNone(rows[2]['values']['goals'])
   with self.assertRaises(ValueError):m.read_rows(p,20252026,'skaters',['unknown'])
   p.write_text(header+'1,2025,5on5,0\n1,2025,5on5,0\n',encoding='utf-8')
   with self.assertRaises(ValueError):m.read_rows(p,20252026,'skaters',['goals'])
 def test_payload_cannot_close_script(self):
  v=self.build();v['profiles'][0]['name']='</script><img src=x onerror=alert(1)>'
  page=m.render(v);self.assertNotIn('</script><img',page);self.assertIn('\\u003c/script>',page)

if __name__=='__main__':unittest.main()
