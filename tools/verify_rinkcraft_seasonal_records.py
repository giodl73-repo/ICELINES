"""Prepare a checksummed test fixture or compare shared-engine results to frozen exports.

prepare BASE EXPOSURE_JSON NEW_INPUT_JSON NEW_MANIFEST_JSON
check INPUT_JSON OUTPUT_JSON EXPOSURE_JSON SKATING_JSON SHOOTING_JSON NEW_RECEIPT_JSON
"""
import hashlib
import json
import math
import sys
from pathlib import Path

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

if sys.argv[1] == 'prepare':
    base, exposure_path, dest, manifest_path = map(Path, sys.argv[2:])
    assert not dest.exists() and not manifest_path.exists()
    exposure_raw = exposure_path.read_bytes()
    exposure = json.loads(exposure_raw)
    catalog_raw = (base / 'catalog.json').read_bytes()
    assert sha(catalog_raw) == exposure['catalog_sha256']
    catalog = json.loads(catalog_raw)
    paths = {p['url'] for w in catalog['windows'] for p in w['players']}
    assert len(paths) == len(exposure['inputs']) and paths == {i['path'] for i in exposure['inputs']}
    records = []
    for item in exposure['inputs']:
        path = Path(item['path'])
        assert not path.is_absolute() and '..' not in path.parts
        raw = (base / path).read_bytes()
        assert sha(raw) == item['sha256']
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
        records.append(dict(player_id=f['player_id'], season=f['season'], name=f['name'], group=f['group'], games=f['games'], minutes=f['minutes'],
                            goals=count('goals'), shots=count('shots'), official_source_sha256=item['sha256'], edge=f.get('edge_evidence')))
    prepared = dict(schema='icelines.seasonal-record-input.v1', reference_label='Frozen NHL public capture 2026-11-19; four supplied seasons', season_type='regular', strength_scope='all strengths',
                    settings=dict(minimum_games=10, minimum_minutes=100, minimum_conversion_shots=25), records=records)
    raw = json.dumps(prepared, ensure_ascii=False, separators=(',', ':')).encode()
    dest.write_bytes(raw)
    manifest = dict(input_sha256=sha(raw), bytes=len(raw), records=len(records), parent_exposure_sha256=sha(exposure_raw), catalog_sha256=sha(catalog_raw), verified_profile_hashes=len(paths))
    manifest_path.write_text(json.dumps(manifest, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(manifest))
elif sys.argv[1] == 'check':
    input_path, output_path, exposure_path, skating_path, shooting_path, receipt_path = map(Path, sys.argv[2:])
    assert not receipt_path.exists()
    source = json.loads(input_path.read_bytes())
    output = json.loads(output_path.read_bytes())
    exposure = json.loads(exposure_path.read_bytes())
    skating = json.loads(skating_path.read_bytes())
    shooting = json.loads(shooting_path.read_bytes())
    assert output['schema'] == 'icelines.seasonal-record-catalog.v1'
    assert output['status'] == 'research_candidates_not_awarded'
    assert 'caller-supplied' in output['reference_scope'] and 'not certified NHL-wide' in output['reference_scope']
    canonical = dict(source, records=sorted(source['records'], key=lambda r: (r['season'], r['player_id'])))
    # Native and WASM must match the full catalog byte-for-byte. This oracle checks
    # values and identity independently; Rust canonical float rendering owns its hash.
    assert len(output['input_sha256']) == 64
    assert output['settings'] == source['settings']
    settings = source['settings']
    old_exposure = next(s for s in exposure['scenarios'] if s['min_games'] == settings['minimum_games'])
    old_skating = next(s for s in skating['scenarios'] if s['min_games'] == settings['minimum_games'])
    old_shooting = next(s for s in shooting['scenarios'] if s['minimum_conversion_shots'] == settings['minimum_conversion_shots'])
    old_sk = {(r['season'], r['player_id']): r for r in old_skating['rows']}
    old_exp = {(r['record']['season'], r['record']['player_id']): r for r in old_exposure['rows']}
    old_sh = {(r['record']['season'], r['record']['player_id']): r for r in old_shooting['rows']}
    expected_ids = {(r['season'], r['player_id']) for r in source['records']}
    assert len(expected_ids) == len(source['records']) == len(output['profiles'])
    actual_ids = set()
    ranks = decisions = 0
    candidates = {'recorded-skating-speed': 0, 'recorded-shot-power-volume': 0}
    def close(a, b):
        if isinstance(a, (float, int)) and isinstance(b, (float, int)):
            assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12), (a, b)
        else:
            assert a == b, (a, b)
    source_rows = {(r['season'], r['player_id']): r for r in source['records']}
    for p in output['profiles']:
        key = p['season'], p['player_id']
        assert key not in actual_ids
        actual_ids.add(key)
        r = source_rows[key]
        assert p['name'] == r['name'] and p['group'] == r['group']
        assert p['eligible'] == (r['games'] >= settings['minimum_games'] and r['minutes'] >= settings['minimum_minutes'])
        ms = {m['key']: m for m in p['measurements']}
        assert len(ms) == len(p['measurements']) == 6
        for m in ms.values():
            assert m['reference_sha256'] is None or len(m['reference_sha256']) == 64
        if key in old_exp:
            for c in old_exp[key]['comparisons']:
                new_key = 'skating_peak_mph' if c['metric'] == 'peak_mph' else c['metric']
                close(c['value'], ms[new_key]['value']); close(c['percentile'], ms[new_key]['percentile']); assert c['peers'] == ms[new_key]['peers']; ranks += 1
        # Conversion diagnostics and shooting reference use the 10-GP exposure
        # setting in the frozen report; other GP scenarios are checked for skating.
        if settings['minimum_games'] == 10:
            for c in old_sh[key]['measurements']:
                new_key = 'shot_peak_mph' if c['metric'] == 'peak_mph' else c['metric']
                close(c['value'], ms[new_key]['value']); close(c['percentile'], ms[new_key]['percentile']); assert c['peers'] == ms[new_key]['peers']; ranks += c['percentile'] is not None
        families = {d['family']: d for d in p['families']}
        assert set(families) == set(candidates)
        wanted = old_sk.get(key)
        if wanted:
            assert (families['recorded-skating-speed']['state'], families['recorded-skating-speed']['candidate']) == (wanted['state'], wanted['candidate']); decisions += 1
        else:
            assert families['recorded-skating-speed']['candidate'] is None
        if settings['minimum_games'] == 10:
            assert (families['recorded-shot-power-volume']['state'], families['recorded-shot-power-volume']['candidate']) == (old_sh[key]['state'], old_sh[key]['candidate']); decisions += 1
        for family, d in families.items():
            candidates[family] += d['candidate'] is not None
    assert actual_ids == expected_ids
    result = dict(profiles=len(actual_ids), verified_legacy_ranks=ranks, verified_legacy_decisions=decisions, candidates=candidates, settings=settings)
    receipt_path.write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(result))
else:
    raise SystemExit(__doc__)
