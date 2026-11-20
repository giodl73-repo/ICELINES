"""Independent arithmetic/scope audit of exported NHL preview cards; no network."""
import bisect
import hashlib
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

manifest_path, directory, result_path = map(Path, sys.argv[1:])
manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
catalog = json.loads((directory / 'catalog.json').read_text(encoding='utf-8'))
checks = 0
conditions_checked = 0
low_checks = 0
whitelist = {'points', 'goals', 'assists', 'shots', 'hits', 'blocks', 'takeaways'}
for window in manifest['windows']:
    def read(spec):
        raw = Path(spec['path']).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == spec['sha256']
        return json.loads(raw)
    source = read(window['reports'])
    toi = read(window['timeonice'])
    year = window['season']
    selected = next(w for w in catalog['windows'] if w['season'] == year)
    raw_rows = {r['playerId']: r for key in ['skaters', 'goalies'] for r in source[key]['data']}
    real = {r['playerId']: r for r in source['realtime']['data']}
    pen = {r['playerId']: r for r in source['penalties']['data']}
    times = {r['playerId']: r for r in toi['data']}
    goalie_ids = {r['playerId'] for r in source['goalies']['data']}
    assert {p['id'] for p in selected['players']} == raw_rows.keys()
    assert selected['captured_through'] == window['captured_through']
    values = {}
    groups = {}
    for actor, row in raw_rows.items():
        goalie = actor in goalie_ids
        group = 'Goalies' if goalie else 'Defense' if row['positionCode'] == 'D' else 'Forwards'
        groups[actor] = group
        gp = row['gamesPlayed']
        fields = [('starts', row.get('gamesStarted'), 1), ('shutouts', row.get('shutouts'), 1), ('shots-faced', row.get('shotsAgainst'), gp), ('save-fraction', row.get('saves'), row.get('shotsAgainst'))] if goalie else [
            *[(k, row.get(k), gp) for k in ['points', 'goals', 'assists', 'shots']],
            *[(k, real.get(actor, {}).get(src), gp) for k, src in [('hits','hits'), ('blocks','blockedShots'), ('takeaways','takeaways'), ('giveaways','giveaways')]],
            ('pim', row.get('penaltyMinutes'), gp), ('drawn', pen.get(actor, {}).get('penaltiesDrawn'), gp), ('taken', pen.get(actor, {}).get('penalties'), gp),
            *[(k, row.get(src), 1) for k, src in [('pp-goals','ppGoals'), ('pp-points','ppPoints'), ('sh-goals','shGoals'), ('sh-points','shPoints')]],
            ('pp-assists', None if row.get('ppPoints') is None or row.get('ppGoals') is None else row['ppPoints']-row['ppGoals'], 1)]
        values[actor] = {key: None if num is None or den is None or den <= 0 else Fraction(num, den) for key, num, den in fields}
    references = {}
    for actor, row in raw_rows.items():
        if row['gamesPlayed'] < (15 if actor in goalie_ids else 20):
            continue
        for key, value in values[actor].items():
            if value is not None:
                references.setdefault((groups[actor], key), []).append(value)
    for ref in references.values():
        ref.sort()
    for entry in selected['players']:
        actor = entry['id']
        row = raw_rows[actor]
        p = json.loads((directory / entry['url']).read_text(encoding='utf-8'))
        a = p['achievements']
        assert p['request']['player_id'] == a['player_id'] == actor
        assert p['request']['first_season'] == p['request']['last_season'] == p['latest_available']['season'] == a['season'] == year
        assert p['latest_available']['games'] == entry['games'] == a['games'] == row['gamesPlayed']
        assert a['source_sha256'] == window['reports']['sha256']
        assert a['group'] == groups[actor]
        assert set(entry['teams']) == {t.strip() for t in row['teamAbbrevs'].split(',') if t.strip()}
        observed = {c['key']: c for f in a['feats'] for c in f['conditions']}
        eligible = row['gamesPlayed'] >= (15 if actor in goalie_ids else 20)
        for key, c in observed.items():
            value = values[actor][key]
            ref = references.get((groups[actor], key), [])
            expected = None if not eligible or len(ref) < 20 or value is None else 100 * (bisect.bisect_left(ref, value) + bisect.bisect_right(ref, value)) / (2 * len(ref))
            assert (c['value'] is None) == (value is None)
            if value is not None:
                assert math.isclose(c['value'], float(value), rel_tol=1e-12, abs_tol=1e-12)
            assert c['peers'] == len(ref)
            assert (c['percentile'] is None) == (expected is None)
            if expected is not None:
                assert math.isclose(c['percentile'], expected, rel_tol=1e-12, abs_tol=1e-12)
            conditions_checked += 1
        o = a.get('profile_observations')
        if groups[actor] != 'Goalies' and row['gamesPlayed'] >= 20:
            assert o
            expected = {'low-' + k for k, c in observed.items() if k in whitelist and c['percentile'] is not None and c['percentile'] <= 20}
            assert {c['id'] for c in o['low_callouts']} == expected
            positives = sum(f['state'] == 'earned' and 'risk_flags' not in f['placements'] for f in a['feats'])
            cautions = sum(f['state'] == 'earned' and 'risk_flags' in f['placements'] for f in a['feats'])
            assert o['positive_count'] == positives and o['caution_count'] == cautions
            low_checks += 1
        else:
            assert o is None
        usage = p.get('deployment_evidence')
        if usage:
            t = row if actor in goalie_ids else times[actor]
            assert usage['rows'][0]['seconds'] == t['timeOnIce']
            assert usage['games'] == row['gamesPlayed']
        checks += 1
result = {'player_seasons_checked': checks, 'independent_rational_metric_rank_checks': conditions_checked, 'lower_tail_profile_checks': low_checks, 'source_identity_scope_workload_checks': 'passed'}
result_path.write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
print(json.dumps(result))
