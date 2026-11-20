"""Independent descriptive-measurement oracle; never produces player ratings."""
import collections
import hashlib
import json
import math
import statistics
import sys
from pathlib import Path


def quantile(values, q):
    values = sorted(values)
    if not values:
        return None
    x = (len(values) - 1) * q
    lo, hi = math.floor(x), math.ceil(x)
    return values[lo] * (1 - (x - lo)) + values[hi] * (x - lo)


def main():
    source_path, report_path, receipt_path = map(Path, sys.argv[1:])
    raw = source_path.read_bytes()
    source, report = json.loads(raw), json.loads(report_path.read_bytes())
    assert report['input_sha256'] == hashlib.sha256(raw).hexdigest()
    assert report['schema'] == 'rinkcraft.task-measurement-report.v1'
    assert report['status'] == 'descriptive_measurements_not_capability_scores'
    assert report['protocol'] == source['protocol']
    splits = {g['id']: g['split'] for g in source['protocol']['games']}
    lineage = {s['id']: s['lineage'] for s in source['protocol']['sources']}
    tasks = {t['component_id']: t for t in source['protocol']['tasks']}
    grouped = collections.defaultdict(list)
    for r in source['observations']:
        grouped[(r['player_id'], r['component_id'], r['context_cell'], splits[r['game_id']])].append(r)
    resolved_states = {'success', 'technical_failure', 'choice_failure', 'opponent_wins', 'teammate_failure'}
    channels_checked = 0
    assert len(report['cells']) == len(grouped)
    for cell in report['cells']:
        key = (cell['player_id'], cell['component_id'], cell['context_cell'], cell['split'])
        rows = grouped[key]
        outcomes = collections.Counter(r['outcome'] for r in rows)
        resolved = sum(outcomes[k] for k in resolved_states)
        expected = dict(outcomes=dict(outcomes), eligible=len(rows)-outcomes['out_of_scope'],
                        resolved=resolved, non_success=resolved-outcomes['success'], out_of_scope=outcomes['out_of_scope'])
        assert cell['counts'] == expected
        rate = outcomes['success']/resolved if resolved else None
        assert cell['success_per_resolved'] == rate
        games = collections.defaultdict(list)
        for row in rows:
            games[row['game_id']].append(row)
        assert cell['games_with_observations'] == len(games)
        assert cell['source_lineages'] == sorted({lineage[r['source_id']] for r in rows})
        leaving = []
        for game, game_rows in games.items():
            group_counts = collections.Counter(r['outcome'] for r in game_rows)
            group_resolved = sum(group_counts[k] for k in resolved_states)
            stored = cell['per_game'][str(game)]
            assert stored['outcomes'] == dict(group_counts)
            assert stored['resolved'] == group_resolved
            if group_resolved:
                leaving.append((outcomes['success']-group_counts['success'])/(resolved-group_resolved)
                               if resolved > group_resolved else None)
        assert cell['games_with_resolved_observations'] == len(leaving)
        if len(leaving) >= 3:
            mean = statistics.mean(leaving)
            expected_se = math.sqrt((len(leaving)-1)/len(leaving)*sum((x-mean)**2 for x in leaving))
            assert math.isclose(cell['success_rate_game_jackknife_se'], expected_se, abs_tol=1e-12)
        else:
            assert cell['success_rate_game_jackknife_se'] is None
        for name, specification in tasks[cell['component_id']]['channels'].items():
            values = [r['measurements'][name] for r in rows if name in r['measurements']]
            expected_channel = dict(unit=specification['unit'], measured=len(values),
                                    missing=sum(name in r['missing_measurements'] for r in rows),
                                    minimum=min(values) if values else None, median=quantile(values, .5),
                                    p90=quantile(values, .9), maximum=max(values) if values else None)
            actual = cell['channels'][name]
            for field, value in expected_channel.items():
                if isinstance(value, float):
                    assert math.isclose(actual[field], value, rel_tol=1e-12, abs_tol=1e-12)
                else:
                    assert actual[field] == value
            channels_checked += 1
    assert report['linked_episodes'] == len({(r['game_id'], r['episode_id']) for r in source['observations']})
    assert report['observations'] == len(source['observations'])
    expected_pairs = {(p['id'], t) for p in source['protocol']['players'] for t in tasks}
    assert {(r['player_id'], r['component_id']) for r in report['coverage']} == expected_pairs
    for row in report['coverage']:
        observations = [r for r in source['observations'] if r['player_id'] == row['player_id'] and r['component_id'] == row['component_id']]
        assert row['rows'] == len(observations)
        assert row['resolved'] == sum(r['outcome'] in resolved_states for r in observations)
        assert set(row['splits_with_resolved']) == {splits[r['game_id']] for r in observations if r['outcome'] in resolved_states}
    receipt = dict(status='passed', observations=len(source['observations']), cells=len(report['cells']),
                   channels_checked=channels_checked, component_coverage_rows=len(expected_pairs),
                   input_sha256=hashlib.sha256(raw).hexdigest(), report_sha256=hashlib.sha256(report_path.read_bytes()).hexdigest(),
                   limitation='Verifies descriptive arithmetic against supplied coding; does not validate real observations, source attribution or capability measurement.')
    with receipt_path.open('x', encoding='utf-8') as stream:
        json.dump(receipt, stream, indent=2)
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
