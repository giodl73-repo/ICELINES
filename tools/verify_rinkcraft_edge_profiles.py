"""Independent EDGE capture/field audit, including exact baseline preservation."""
import copy
import hashlib
import json
import math
import sys
from pathlib import Path

manifest_path, output, result_path = map(Path, sys.argv[1:])
manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
def read(spec):
    raw = Path(spec['path']).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == spec['sha256']
    return raw
catalog_bytes = read(manifest['catalog'])
assert (output/'catalog.json').read_bytes() == catalog_bytes
catalog = json.loads(catalog_bytes)
capture = json.loads(read(manifest['capture']))
inventory = json.loads(read(manifest['base_inventory']))
records = {(r['season'], r['player_id']): r for r in capture['records']}
base = Path(manifest['base_directory'])
profiles = available = measures = locations = replays = 0
for window in catalog['windows']:
    for player in window['players']:
        path = player['url']
        original_raw = (base/path).read_bytes()
        assert hashlib.sha256(original_raw).hexdigest() == inventory[path]
        original = json.loads(original_raw)
        new_raw = (output/path).read_bytes()
        new = json.loads(new_raw)
        clean = copy.deepcopy(new)
        edge = clean['numerical_evidence'].pop('edge_evidence', None)
        status = clean['numerical_evidence'].pop('edge_capture_status')
        assert clean == original, path
        record = records[(window['season'], player['id'])]
        assert status['source_url'] == record['url']
        assert status['attempted_at'] == record['accessed_utc']
        if edge:
            assert status['state'] == 'available'
            raw = read(record)
            source = json.loads(raw)
            assert edge['source']['sha256'] == record['sha256']
            assert edge['source']['bytes'] == len(raw) == record['bytes']
            assert edge['player_id'] == player['id'] == source['player']['id']
            assert edge['season'] == window['season']
            assert edge['games'] == player['games'] == source['player']['gamesPlayed']
            for m in edge['measures']:
                field = source
                for key in m['source_field'].split('.'):
                    field = field[key]
                value = field.get('imperial' if m['unit'] in ['Mph', 'Miles'] else 'value') if isinstance(field, dict) else field
                assert math.isclose(m['value'], value, rel_tol=1e-12, abs_tol=1e-12)
                p = field.get('percentile') if isinstance(field, dict) else None
                assert (p is None) == (m['provider_percentile'] is None)
                if p is not None:
                    assert 0 <= p <= 1 and math.isclose(m['provider_percentile'], p, abs_tol=1e-12)
                measures += 1
            raw_locations = {r['locationCode']: r for r in source.get('shotLocationSummary', source.get('sogSummary', []))}
            for row in edge['locations']:
                actual = raw_locations[row['code']]
                for key, raw_key in [('shots','shots'), ('goals','goals'), ('saves','saves'), ('goals_against','goalsAgainst')]:
                    assert row[key] == actual.get(raw_key)
                goalie = player['group'] == 'Goalies'
                den = None if goalie and (actual.get('saves') is None or actual.get('goalsAgainst') is None) else actual['saves'] + actual['goalsAgainst'] if goalie else actual.get('shots')
                expected = None if den == 0 else actual.get('savePctg' if goalie else 'shootingPctg')
                assert (row['fraction'] is None) == (expected is None)
                if expected is not None:
                    assert math.isclose(row['fraction'], expected, abs_tol=1e-12)
                locations += 1
            available += 1
        else:
            assert status['state'] == 'unavailable' and status['reason']
        replay_path = output.parent/'catalog-v1'/path
        if replay_path.exists():
            assert new_raw == replay_path.read_bytes()
            replays += 1
        profiles += 1
result = {'profiles_preserved_exactly': profiles, 'matched_edge_profiles': available, 'unavailable_explicit': profiles-available, 'raw_measurement_checks': measures, 'raw_location_checks': locations, 'byte_identical_replay_profiles': replays, 'catalog_unchanged': True}
result_path.write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
print(json.dumps(result))
