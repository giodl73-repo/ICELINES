"""Neutral synthetic policy tests; no tests assert a real player's ability."""
import copy
import importlib.util
import json
import math
from pathlib import Path
import tempfile
import unittest

SPEC=importlib.util.spec_from_file_location('contracts',Path(__file__).parents[1]/'rinkcraft_attribute_contracts.py')
m=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(m)


def fixture():
    v={'schema':'rinkcraft.attribute-contracts.v1','version':'test-only','production_grades_released':False,
       'scope':{'population':'synthetic'},'observation_policy':{'unit':'synthetic task'},
       'evidence_policy':{'lineage':'synthetic'},'migration':{'policy':'test-only'},
       'sources':{'TEST':{k:'Synthetic source' for k in ('title','authors','published','accessed','locator','inspection','scope','limitations','rights')}},
       'attributes':[],'challenge_cases':[],
       'scoring_policy':{'release_gates':list(m.GATES),'quantile_knots':[0,.05,.25,.5,.7,.9,.97,.995,1],
                         'score_knots':[1,4,8,10,12,16,18,20,20],
                         'point_precision_gate':{'maximum_integer_score_span':4}}}
    v['sources']['TEST']['id']='TEST'
    v['sources']['TEST']['url']='https://example.org/synthetic'
    for branch,names in m.EXPECTED.items():
        for name in sorted(names):
            aid=f'{branch}-{name}'
            cs=[dict(id=f'{aid}-{i}',label=f'Task {i}',required=True,**{k:'Synthetic task boundary' for k in m.COMPONENT_FIELDS}) for i in range(3)]
            for c in cs: c['direction']='task_model'
            a={k:['Synthetic definition'] for k in m.ATTRIBUTE_FIELDS}
            a.update(id=aid,name=name,branch=branch,components=cs,construct='Synthetic construct',current_coverage='Synthetic coverage',
                     evidence_contract={'required_components':[c['id'] for c in cs]},
                     candidate_weights={c['id']:1/3 for c in cs},sources=[{'id':'TEST','use':'Synthetic test only'}],
                     behavior_anchors=dict(low='Low',typical='Typical',high='High'),
                     measurement_validation_status='unvalidated',player_grades_released=False)
            v['attributes'].append(a)
            v['challenge_cases'].append(dict(id=f'case-{aid}',attribute_id=aid,scenario='Synthetic ambiguous task',expected='Withhold',forbidden='Invent score',case_kind='test'))
    return v

class ContractTests(unittest.TestCase):
    def setUp(self): self.v=fixture();self.p=self.v['scoring_policy'];self.a=self.v['attributes'][0]
    def test_neutral_complete_fixture(self): self.assertEqual(m.validate(self.v),[])
    def test_each_required_field_is_enforced(self):
        for field in m.COMPONENT_FIELDS:
            v=copy.deepcopy(self.v);v['attributes'][0]['components'][0].pop(field)
            self.assertTrue(m.validate(v),field)
    def test_branch_completeness_and_duplicate_ids(self):
        v=copy.deepcopy(self.v);v['attributes'].pop();self.assertTrue(m.validate(v))
        v=copy.deepcopy(self.v);v['attributes'][1]['id']=v['attributes'][0]['id'];self.assertTrue(m.validate(v))
    def test_source_scope_and_challenge_required(self):
        for mutate in (lambda v:v['sources'].clear(),lambda v:v['challenge_cases'].clear(),lambda v:v['sources']['TEST'].pop('limitations')):
            v=copy.deepcopy(self.v);mutate(v);self.assertTrue(m.validate(v))
    def test_unknown_publication_date_is_explicit(self):
        self.v['sources']['TEST']['published']=None
        self.assertTrue(m.validate(self.v))
        self.v['sources']['TEST']['publication_note']='Unknown from inspected material'
        self.assertEqual(m.validate(self.v),[])
    def test_registry_cannot_claim_measurement_validity(self):
        self.v['production_grades_released']=True;self.assertTrue(m.validate(self.v))
        self.v=fixture();self.v['attributes'][0]['measurement_validation_status']='validated';self.assertTrue(m.validate(self.v))
    def test_no_missing_facet_imputation_or_extra_vote(self):
        qs={c['id']:.5 for c in self.a['components']}
        self.assertAlmostEqual(m.balanced_index(self.a,qs),0)
        qs.pop(next(iter(qs)))
        with self.assertRaises(ValueError):m.balanced_index(self.a,qs)
        qs={c['id']:.5 for c in self.a['components']};qs['duplicate-transform']=.99
        with self.assertRaises(ValueError):m.balanced_index(self.a,qs)
    def test_probability_validation_and_bounded_extremes(self):
        for bad in (-.1,1.1,math.nan,math.inf,True,None):
            with self.assertRaises(ValueError):m.score_quantile(bad,self.p)
        for q in (0,1):
            self.assertTrue(math.isfinite(m.balanced_index(self.a,{c['id']:q for c in self.a['components']})))
    def test_mapping_knots_and_round_half_up(self):
        for q,s in zip(self.p['quantile_knots'],self.p['score_knots']): self.assertEqual(m.score_quantile(q,self.p),s)
        self.assertEqual(m.score_quantile(.025,self.p),3)
        points=[m.score_quantile(i/1000,self.p) for i in range(1001)]
        self.assertEqual(points,sorted(points));self.assertEqual((min(points),max(points)),(1,20))
    def test_reference_ties_and_no_empty_reference(self):
        self.assertEqual(m.reference_quantile(2,[1,2,2,3]),.5)
        self.assertEqual(m.reference_quantile(0,[1,2]),0)
        self.assertEqual(m.reference_quantile(3,[1,2]),1)
        with self.assertRaises(ValueError):m.reference_quantile(1,[])
    def test_every_gate_needs_substantiated_audit(self):
        records={g:dict(passed=True,evidence='SYNTHETIC ONLY',method_version='test') for g in m.GATES}
        self.assertEqual(m.publication_state(self.p,records,(8,12)),'released_point_with_interval')
        for gate in m.GATES:
            for bad in ({},{'passed':True},{'passed':False,'evidence':'test','method_version':'test'}):
                r=copy.deepcopy(records);r[gate]=bad
                self.assertEqual(m.publication_state(self.p,r,(8,12)),'unrated',gate)
    def test_uncertainty_and_sensitivity_withhold_point(self):
        r={g:dict(passed=True,evidence='SYNTHETIC ONLY',method_version='test') for g in m.GATES}
        self.assertEqual(m.publication_state(self.p,r,(5,15)),'range_only')
        self.assertEqual(m.publication_state(self.p,r,(9,11),3),'range_only')
        for interval in (None,(12,8),(0,8),(8,21),(8.,12),(True,10)):
            self.assertEqual(m.publication_state(self.p,r,interval),'unrated')
    def test_render_preserves_contracts_and_escapes_html(self):
        self.v['attributes'][0]['construct']='<script>bad()</script>'
        with tempfile.TemporaryDirectory() as d:
            m.render(self.v,d,'synthetic-hash')
            page=(Path(d)/'index.html').read_text(encoding='utf-8')
            self.assertNotIn('<script>',page);self.assertIn('&lt;script&gt;',page)
            self.assertEqual(page.count('<article '),12)
            self.assertEqual(json.loads((Path(d)/'registry.json').read_text(encoding='utf-8')),self.v)
            self.assertIn('Synthetic definition',(Path(d)/(self.a['id'].lower()+'.md')).read_text(encoding='utf-8'))

if __name__=='__main__': unittest.main()
