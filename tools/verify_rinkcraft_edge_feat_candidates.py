"""Independently verify the candidate family and summarize sensitivity.

Usage: EXPOSURE_JSON CANDIDATE_JSON NEW_VERIFICATION_JSON
"""
import collections
import hashlib
import json
import math
import sys
from pathlib import Path

source_path, candidate_path, output_path = map(Path, sys.argv[1:])
source_bytes = source_path.read_bytes()
source = json.loads(source_bytes)
report = json.loads(candidate_path.read_bytes())
assert report['schema'] == 'rinkcraft.edge-feat-candidates.v1'
assert report['exposure_report_sha256'] == hashlib.sha256(source_bytes).hexdigest()
assert report['catalog_sha256'] == source['catalog_sha256']
rule_bytes = json.dumps(report['rules'], sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()
assert hashlib.sha256(rule_bytes).hexdigest() == report['rules_sha256']
assert report['rules']['cutoff_percentile'] == 90
assert report['rules']['minimum_measured_peers'] == 20
assert report['rules']['status'] == 'research_candidates_not_awarded'
assert [s['min_games'] for s in report['scenarios']] == [s['min_games'] for s in source['scenarios']]
checked = 0
all_cases = {}
for original, scenario in zip(source['scenarios'], report['scenarios']):
    assert len(original['rows']) == len(scenario['rows'])
    assert scenario['min_minutes'] == original['min_minutes'] == 100
    census = collections.Counter()
    cases = {}
    for before, after in zip(original['rows'], scenario['rows']):
        r = before['record']
        key = r['season'], r['player_id']
        assert key not in cases
        assert all(after[k] == r[k] for k in ('season', 'player_id', 'name', 'group', 'edge_sha256'))
        assert len(after['measurements']) == len(before['comparisons'])
        for actual_measure, expected_measure in zip(after['measurements'], before['comparisons']):
            assert actual_measure.keys() == expected_measure.keys()
            for field, expected_value in expected_measure.items():
                actual_value = actual_measure[field]
                if isinstance(expected_value, float):
                    assert math.isclose(actual_value, expected_value, rel_tol=1e-12, abs_tol=1e-12)
                else:
                    assert actual_value == expected_value
        metrics = {c['metric']: c for c in before['comparisons']}
        peak = metrics.get('peak_mph')
        rate = metrics.get('bursts_per_60')
        candidate = None
        if not before['eligible']:
            state = 'insufficient_exposure'
        elif not peak or not rate or min(peak['peers'], rate['peers']) < 20:
            state = 'unassessed'
        else:
            state = 'assessed'
            p, b = peak['percentile'] >= 90, rate['percentile'] >= 90
            if p and b:
                candidate = 'two-gears'
            elif p:
                candidate = 'top-gear'
            elif b:
                candidate = 'burst-engine'
        assert (after['state'], after['candidate']) == (state, candidate)
        census[(r['season'], r['group'], candidate or state)] += 1
        cases[key] = after
        checked += 1
    actual = {(c['season'], c['group'], c['outcome']): c['count'] for c in scenario['census']}
    assert dict(census) == actual and sum(actual.values()) == len(cases)
    all_cases[scenario['min_games']] = cases
assert all(set(c) == set(all_cases[10]) for c in all_cases.values())

baseline = all_cases[10]
current = max(s for s, _ in baseline)
sensitivity = []
for cutoff in (5, 20):
    other = all_cases[cutoff]
    matched = [(a, other[k]) for k, a in baseline.items() if a['season'] == current]
    both = [(a, b) for a, b in matched if a['state'] == b['state'] == 'assessed']
    sensitivity.append(dict(
        min_games=cutoff, current_season=current,
        assessed_at_both=len(both),
        qualification_changes_among_assessed=sum(bool(a['candidate']) != bool(b['candidate']) for a, b in both),
        family_label_changes_among_assessed=sum(a['candidate'] != b['candidate'] for a, b in both),
        baseline_assessed_losing_exposure=sum(a['state'] == 'assessed' and b['state'] == 'insufficient_exposure' for a, b in matched),
        baseline_candidates_losing_exposure=sum(bool(a['candidate']) and b['state'] == 'insufficient_exposure' for a, b in matched),
    ))

repeat = []
seasons = sorted({s for s, _ in baseline})
for first, second in zip(seasons, seasons[1:]):
    pairs = [(a, baseline.get((second, pid))) for (season, pid), a in baseline.items() if season == first]
    pairs = [(a, b) for a, b in pairs if b and a['group'] == b['group'] and a['state'] == b['state'] == 'assessed']
    prior = [(a, b) for a, b in pairs if a['candidate']]
    repeat.append(dict(first=first, second=second, same_group_assessed_both=len(pairs),
                       prior_candidates=len(prior), any_candidate_again=sum(bool(b['candidate']) for a, b in prior),
                       same_label_again=sum(a['candidate'] == b['candidate'] for a, b in prior)))

samples = [a for a in baseline.values() if a['season'] == current and any(n in a['name'] for n in ('McDavid', 'MacKinnon', 'Kartye', 'Makar', 'Fox', 'Emberson', 'Killorn'))]
summary = dict(
    schema='rinkcraft.edge-feat-verification.v1', independently_checked_classifications=checked,
    census_10gp=next(s['census'] for s in report['scenarios'] if s['min_games'] == 10),
    sensitivity=sensitivity, consecutive_snapshot_overlap=repeat, current_samples=samples,
    limits='Authored P90 hypotheses; recorded activity only. Current season is partial. Consecutive snapshot overlap is not prospective validation, corrected reliability or evidence of ability change.',
)
assert not output_path.exists(), 'new output required'
output_path.write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: summary[k] for k in ('independently_checked_classifications', 'sensitivity', 'consecutive_snapshot_overlap')}))
