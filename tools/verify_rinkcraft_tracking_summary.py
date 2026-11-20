"""Check captured tracking context against provider fields, not capability truth."""
import hashlib
import json
import math
import sys
from pathlib import Path


def read(path):
    raw = path.read_bytes()
    value = json.loads(raw)
    return raw, value[0] if isinstance(value, list) else value


def main():
    summary_path, entities_path, report_path, receipt_path = map(Path, sys.argv[1:])
    summary_bytes, summary = read(summary_path)
    entity_bytes, entities = read(entities_path)
    report = json.loads(report_path.read_bytes())
    assert str(report['game_id']) == summary['OfficialCode'] == entities['EventOfficialCode']
    assert summary['EventId'] == entities['EventId']
    assert report['summary_sha256'] == hashlib.sha256(summary_bytes).hexdigest()
    assert report['entities_sha256'] == hashlib.sha256(entity_bytes).hexdigest()
    assert report['event_time'] == summary.get('EventTime')
    assert report['as_of_utc'] == summary.get('NowUTC')
    roster = {r['EntityId']: r for r in entities['Entities'] if r['EntityType'] == 'Player'}
    data = {r['EntityId']: r for r in summary['EntitySummaries']}
    assert len(report['players']) == len(roster.keys() & data.keys())
    assert set(report['registered_players_without_summary']) == {int(r['OfficialId']) for key, r in roster.items() if key not in data}
    fields = {'time_on_seconds': 'TimeOnTotal', 'actual_time_on_seconds': 'ActualTimeOnTotal',
              'possession_seconds': 'PossessionTime', 'possession_even_strength_seconds': 'PossessionTimeEvenStrength',
              'provider_maximum_speed': 'SpeedMax', 'provider_average_speed': 'SpeedAvg', 'provider_distance': 'DistanceTotal'}
    checked = 0
    for player in report['players']:
        entity = roster[player['source_entity_id']]
        row = data[player['source_entity_id']]
        assert player['player_id'] == int(entity['OfficialId'])
        assert player['name'] == entity['FirstName'] + ' ' + entity['LastName']
        assert player['team_id'] == entity['EntityTeamId']
        assert player['position'] == {'LW': 'L', 'RW': 'R'}.get(entity['Position'], entity['Position'])
        for output, source in fields.items():
            expected = row.get(source)
            if expected is None:
                assert player[output] is None
            elif isinstance(expected, int):
                assert player[output] == expected
            else:
                # JSON f64 parsing/printing can differ by a final bit across runtimes.
                assert math.isclose(player[output], expected, rel_tol=2e-15, abs_tol=1e-12)
            checked += 1
        for output, source in [('maximum_speed_mps', 'SpeedMax'), ('average_speed_mps', 'SpeedAvg'), ('distance_metres', 'DistanceTotal')]:
            expected = row[source] * .3048 if row.get(source) is not None else None
            if expected is None:
                assert player[output] is None
            else:
                assert math.isclose(player[output], expected, rel_tol=1e-12, abs_tol=1e-12)
            checked += 1
        zones = {'offensive': row.get('PossessionTimeOZone'), 'neutral': row.get('PossessionTimeNZone'), 'defensive': row.get('PossessionTimeDZone')}
        assert player['possession_by_zone_seconds'] == zones
        expected_diagnostics = []
        if row.get('TimeOnTotal') is not None and row.get('ActualTimeOnTotal') is not None and abs(row['TimeOnTotal']-row['ActualTimeOnTotal']) > 1:
            expected_diagnostics.append('provider_time_on_clocks_differ_no_substitution')
        if row.get('PossessionTime') is not None and all(v is not None for v in zones.values()) and abs(sum(zones.values())-row['PossessionTime']) > 1e-9:
            expected_diagnostics.append('possession_zone_sum_differs_from_total_no_reconciliation')
        assert player['diagnostics'] == expected_diagnostics
    receipt = dict(status='passed', players=len(report['players']), numerical_fields_checked=checked,
                   summary_sha256=hashlib.sha256(summary_bytes).hexdigest(), entities_sha256=hashlib.sha256(entity_bytes).hexdigest(),
                   report_sha256=hashlib.sha256(report_path.read_bytes()).hexdigest(),
                   limitation='Verifies parsing, assumed-unit conversion and diagnostics only; no sensor calibration, full-game completeness or ability validation.')
    with receipt_path.open('x', encoding='utf-8') as stream:
        json.dump(receipt, stream, indent=2)
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
