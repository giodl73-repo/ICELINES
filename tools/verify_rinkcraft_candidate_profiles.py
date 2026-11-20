"""Verify preservation and candidate binding: BASE NEW CANDIDATES NEW_RECEIPT."""
import hashlib
import json
import math
import sys
from pathlib import Path

base, output, report_path, receipt = map(Path, sys.argv[1:])
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
        assert before == after, path
        expected = rows.pop((window['season'], actor['id']), None)
        if expected is None:
            assert candidate is None and before_raw == after_raw
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
        profiles += 1
assert not rows
result = dict(profiles_preserved=profiles, candidate_contexts_bound=attached, candidate_labels=qualified, catalog_unchanged=True, coverage_unchanged=True)
assert not receipt.exists()
receipt.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result))
