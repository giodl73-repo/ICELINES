"""Independent shooting-record oracle and calibration summary.

Usage: BASE_DIRECTORY EXPOSURE_JSON REPORT_JSON NEW_VERIFICATION_JSON
"""
import collections
import hashlib
import json
import math
import sys
from pathlib import Path

base, exposure_path, report_path, receipt_path = map(Path, sys.argv[1:])
report = json.loads(report_path.read_bytes())
exposure_bytes = exposure_path.read_bytes()
exposure = json.loads(exposure_bytes)
assert report['schema'] == 'rinkcraft.shooting-records.v1'
assert report['exposure_report_sha256'] == hashlib.sha256(exposure_bytes).hexdigest()
assert report['catalog_sha256'] == hashlib.sha256((base / 'catalog.json').read_bytes()).hexdigest()
assert report['inputs'] == exposure['inputs']
rule_bytes = json.dumps(report['rules'], sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()
assert hashlib.sha256(rule_bytes).hexdigest() == report['rules_sha256']
assert report['rules']['status'] == 'research_candidates_not_awarded'
assert report['rules']['cutoff_percentile'] == 90
catalog = json.loads((base / 'catalog.json').read_bytes())
assert len(report['inputs']) == sum(len(w['players']) for w in catalog['windows'])
assert {i['path'] for i in report['inputs']} == {p['url'] for w in catalog['windows'] for p in w['players']}
expected = {}
for item in report['inputs']:
    path = Path(item['path'])
    assert not path.is_absolute() and '..' not in path.parts
    raw = (base / path).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == item['sha256']
    p = json.loads(raw)
    f = p['numerical_evidence']
    if f['group'] == 'Goalies':
        continue
    metrics = {m['key']: m for m in p['latest_available']['metrics']}
    def count(key):
        m = metrics.get(key)
        if not m or m['status'] != 'available':
            return None
        assert m['unit'] == 'Count' and m['value'] >= 0 and m['value'] == int(m['value'])
        return int(m['value'])
    goals, shots = count('goals'), count('shots')
    e = f.get('edge_evidence')
    peaks = [] if e is None else [m['value'] for m in e['measures'] if m['key'] == 'max-shot-speed']
    assert len(peaks) <= 1
    all_row = [] if e is None else [l for l in e['locations'] if l['code'] == 'all']
    match = None
    if all_row and all(x is not None for x in [goals, shots, all_row[0]['shots'], all_row[0]['goals']]):
        match = goals == all_row[0]['goals'] and shots == all_row[0]['shots']
    r = dict(player_id=f['player_id'], season=f['season'], name=f['name'], group=f['group'], games=f['games'], minutes=f['minutes'],
             goals=goals, shots=shots, peak_mph=peaks[0] if peaks else None,
             shots_per_60=None if shots is None else shots * 60 / f['minutes'],
             conversion_pct=None if goals is None or not shots or goals > shots else goals * 100 / shots,
             edge_all_counts_match=match, edge_all_goals=all_row[0]['goals'] if all_row else None,
             edge_all_shots=all_row[0]['shots'] if all_row else None,
             edge_captured_at=None if e is None else e['captured_at'],
             source_profile_sha256=item['sha256'], edge_sha256=None if e is None else e['source']['sha256'])
    key = (r['season'], r['player_id'])
    assert key not in expected
    expected[key] = r

def close(a, b):
    if isinstance(a, float) and isinstance(b, (int, float)):
        assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12), (a, b)
    else:
        assert a == b, (a, b)

scenarios = []
rank_checks = 0
assert [s['minimum_conversion_shots'] for s in report['scenarios']] == [10, 25, 50]
for scenario in report['scenarios']:
    assert scenario['minimum_games'] == 10 and scenario['minimum_minutes'] == 100
    threshold = scenario['minimum_conversion_shots']
    def eligible(r):
        return r['games'] >= 10 and r['minutes'] >= 100
    def metric(r, k):
        if k == 'conversion_pct' and (r['shots'] is None or r['shots'] < threshold):
            return None
        return r[k]
    pools = collections.defaultdict(list)
    for r in expected.values():
        if eligible(r):
            for key in ['peak_mph', 'shots_per_60', 'conversion_pct']:
                value = metric(r, key)
                if value is not None:
                    pools[(r['season'], r['group'], key)].append(value)
    census = collections.Counter()
    seen = set()
    checked_rows = []
    for row in scenario['rows']:
        actual = row['record']
        identity = actual['season'], actual['player_id']
        assert identity not in seen
        seen.add(identity)
        r = expected[identity]
        assert actual.keys() == r.keys()
        for key in r:
            close(r[key], actual[key])
        ranks = {}
        assert [m['metric'] for m in row['measurements']] == ['peak_mph', 'shots_per_60', 'conversion_pct']
        for m in row['measurements']:
            key = m['metric']
            value = metric(r, key)
            close(value, m['value'])
            peers = pools[(r['season'], r['group'], key)]
            assert m['peers'] == len(peers)
            rank = None if value is None or not eligible(r) else 100 * (sum(v < value for v in peers) + .5 * sum(v == value for v in peers)) / len(peers)
            close(rank, m['percentile'])
            if rank is not None:
                ranks[key] = (rank, len(peers))
                rank_checks += 1
        state = 'insufficient_exposure'
        candidate = None
        if eligible(r):
            if not all(k in ranks and ranks[k][1] >= 20 for k in ['peak_mph', 'shots_per_60']):
                state = 'unassessed'
            else:
                state = 'assessed'
                p, v = ranks['peak_mph'][0] >= 90, ranks['shots_per_60'][0] >= 90
                candidate = 'power-volume' if p and v else 'shot-power' if p else 'shot-volume' if v else None
        assert (state, candidate) == (row['state'], row['candidate'])
        census[(r['season'], r['group'], candidate or state)] += 1
        checked_rows.append(row)
    assert seen == expected.keys()
    assert dict(census) == {(c['season'], c['group'], c['outcome']): c['count'] for c in scenario['census']}
    scenarios.append((threshold, checked_rows))

current = max(r['season'] for r in expected.values())
summaries = []
for threshold, rows in scenarios:
    for season in sorted({r['season'] for r in expected.values()}, reverse=True):
        for group in ['Forwards', 'Defense']:
            selected = [r for r in rows if r['record']['season'] == season and r['record']['group'] == group]
            assessed = [r for r in selected if r['state'] == 'assessed']
            converted = [r for r in selected if next(m for m in r['measurements'] if m['metric'] == 'conversion_pct')['percentile'] is not None]
            def ranks(row):
                return {m['metric']: m['percentile'] for m in row['measurements']}
            joint = [ranks(r) for r in converted if r['state'] == 'assessed']
            summaries.append(dict(minimum_conversion_shots=threshold, season=season, group=group,
                                  assessed_family=len(assessed), conversion_ranked=len(converted),
                                  shot_power=sum(r['candidate'] == 'shot-power' for r in assessed),
                                  shot_volume=sum(r['candidate'] == 'shot-volume' for r in assessed),
                                  power_volume=sum(r['candidate'] == 'power-volume' for r in assessed),
                                  peak_conversion_both_p90=sum(r['peak_mph'] >= 90 and r['conversion_pct'] >= 90 for r in joint),
                                  peak_p90_in_conversion_reference=sum(r['peak_mph'] >= 90 for r in joint)))
conversion_membership = []
for threshold, rows in scenarios:
    selected = [r for r in rows if r['record']['season'] == current]
    top = [(r['record']['name'], r['record']['shots'], r['record']['conversion_pct']) for r in selected if any(m['metric'] == 'conversion_pct' and m['percentile'] is not None and m['percentile'] >= 90 for m in r['measurements'])]
    conversion_membership.append(dict(minimum_shots=threshold, current_season=current, top_decile_count=len(top), players=top))
reference_rows = {(r['record']['season'], r['record']['player_id']): r for r in scenarios[1][1]}
for _, rows in scenarios:
    for row in rows:
        identity = row['record']['season'], row['record']['player_id']
        assert (row['state'], row['candidate']) == (reference_rows[identity]['state'], reference_rows[identity]['candidate'])
repeat = []
seasons = sorted({r['season'] for r in expected.values()})
for first, second in zip(seasons, seasons[1:]):
    pairs = [(a, reference_rows.get((second, pid))) for (season, pid), a in reference_rows.items() if season == first]
    pairs = [(a, b) for a, b in pairs if b and a['record']['group'] == b['record']['group'] and a['state'] == b['state'] == 'assessed']
    candidates = [(a, b) for a, b in pairs if a['candidate']]
    repeat.append(dict(first=first, second=second, same_group_assessed_both=len(pairs), prior_candidates=len(candidates),
                       any_candidate_again=sum(bool(b['candidate']) for a, b in candidates), same_label_again=sum(a['candidate'] == b['candidate'] for a, b in candidates)))
samples = [row for row in scenarios[1][1] if row['record']['season'] == current and any(n in row['record']['name'] for n in ('McDavid', 'MacKinnon', 'Kartye', 'Fox', 'Makar', 'Dorofeyev', 'Killorn', 'Ovechkin', 'Pastrnak'))]
result = dict(schema='rinkcraft.shooting-records-verification.v1', input_hashes=len(report['inputs']), skaters=len(expected),
              scenarios=len(scenarios), independently_verified_ranks=rank_checks,
              current_season=current, cohort_summary=summaries, conversion_top_decile_sensitivity=conversion_membership,
              family_independent_of_conversion_gate=True, consecutive_snapshot_overlap=repeat,
              source_count_agreements=sum(r['edge_all_counts_match'] is True for r in expected.values()),
              source_count_disagreements=[r for r in expected.values() if r['edge_all_counts_match'] is False],
              current_samples=samples,
              limits='Observed records and authored thresholds; raw conversion is not finishing ability. No awards or capability scores.')
assert not receipt_path.exists(), 'new receipt required'
receipt_path.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: result[k] for k in ['input_hashes','skaters','scenarios','independently_verified_ranks','source_count_agreements']}))
print(json.dumps([s for s in summaries if s['season'] == current]))
