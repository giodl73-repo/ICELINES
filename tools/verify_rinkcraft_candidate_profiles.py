"""Verify preservation: BASE NEW SKATING_CANDIDATES NEW_RECEIPT [SHOOTING_REPORT]."""
import hashlib
import json
import math
import sys
from pathlib import Path

base, output, report_path, receipt = map(Path, sys.argv[1:5])
shooting_report = json.loads(Path(sys.argv[5]).read_bytes()) if len(sys.argv) == 6 else None
shooting_hash = hashlib.sha256(Path(sys.argv[5]).read_bytes()).hexdigest() if shooting_report else None
shooting_rows = {(r['record']['season'], r['record']['player_id']): r for s in shooting_report['scenarios'] if s['minimum_conversion_shots'] == 25 for r in s['rows']} if shooting_report else {}
shooting_attached = shooting_qualified = 0
def nested_equal(a, b):
    if isinstance(a, dict):
        assert a.keys() == b.keys()
        for key in a:
            nested_equal(a[key], b[key])
    elif isinstance(a, list):
        assert len(a) == len(b)
        for x, y in zip(a, b):
            nested_equal(x, y)
    elif isinstance(a, float) and isinstance(b, (int, float)):
        assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12)
    else:
        assert a == b
raw_report = report_path.read_bytes()
report_hash = hashlib.sha256(raw_report).hexdigest()
report = json.loads(raw_report)
catalog = json.loads((base / 'catalog.json').read_bytes())
assert (base / 'catalog.json').read_bytes() == (output / 'catalog.json').read_bytes()
assert (base / 'edge-coverage.json').read_bytes() == (output / 'edge-coverage.json').read_bytes()
rows = {(r['season'], r['player_id']): r for s in report['scenarios'] if s['min_games'] == 10 for r in s['rows']}
profiles = attached = qualified = 0
for window in catalog['windows']:
    for actor in window['players']:
        path = actor['url']
        before_raw = (base / path).read_bytes()
        after_raw = (output / path).read_bytes()
        before, after = json.loads(before_raw), json.loads(after_raw)
        candidate = after['numerical_evidence'].pop('edge_skating_candidate', None)
        shooting = after['numerical_evidence'].pop('edge_shooting_candidate', None)
        assert before == after, path
        expected = rows.pop((window['season'], actor['id']), None)
        if expected is None:
            assert candidate is None
            if shooting is None:
                assert before_raw == after_raw
        else:
            assert candidate is not None
            for key, value in expected.items():
                if key == 'measurements':
                    assert len(value) == len(candidate[key])
                    for a, b in zip(value, candidate[key]):
                        assert a.keys() == b.keys()
                        for k in a:
                            if isinstance(a[k], float):
                                assert math.isclose(a[k], b[k], rel_tol=1e-12, abs_tol=1e-12)
                            else:
                                assert a[k] == b[k]
                else:
                    assert candidate[key] == value
            frame = before['numerical_evidence']
            assert candidate['edge_sha256'] == frame['edge_evidence']['source']['sha256']
            assert candidate['rules_sha256'] == report['rules_sha256']
            assert candidate['source_profile_sha256'] == hashlib.sha256(before_raw).hexdigest()
            assert candidate['candidate_report_sha256'] == report_hash
            assert candidate['games'] == frame['games']
            assert math.isclose(candidate['minutes'], frame['minutes'], rel_tol=1e-12)
            attached += 1
            qualified += candidate['candidate'] is not None
        shot_expected = shooting_rows.pop((window['season'], actor['id']), None)
        if shot_expected is None:
            assert shooting is None
        else:
            assert shooting is not None
            for key, value in shot_expected.items():
                nested_equal(value, shooting[key])
            assert shooting['schema'] == 'icelines.shooting-candidate.v1'
            assert shooting['status'] == 'research_candidate'
            assert shooting['minimum_conversion_shots'] == 25
            assert shooting['minimum_games'] == 10 and shooting['minimum_minutes'] == 100
            assert shooting['rules_sha256'] == shooting_report['rules_sha256']
            assert shooting['candidate_report_sha256'] == shooting_hash
            assert shooting['record']['source_profile_sha256'] == hashlib.sha256(before_raw).hexdigest()
            shooting_attached += 1
            shooting_qualified += shooting['candidate'] is not None
        profiles += 1
assert not rows
assert not shooting_rows
result = dict(profiles_preserved=profiles, candidate_contexts_bound=attached, candidate_labels=qualified, shooting_contexts_bound=shooting_attached, shooting_labels=shooting_qualified, catalog_unchanged=True, coverage_unchanged=True)
assert not receipt.exists()
receipt.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result))
