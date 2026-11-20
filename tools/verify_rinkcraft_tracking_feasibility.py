"""Independent literal census of the BDC tracking capture; no trajectory inference."""
import collections
import csv
import hashlib
import io
import itertools
import json
import math
import sys
from pathlib import Path


def main():
    source_path, report_path, receipt_path = map(Path, sys.argv[1:])
    raw = source_path.read_bytes()
    actual = json.loads(report_path.read_bytes())
    expected = dict(rows=0, frames=0, adjacent_frames_same_game_clock=0,
                    adjacent_frames_decreasing_game_clock=0, adjacent_frames_increasing_game_clock=0,
                    non_increasing_image_suffix_pairs=0, frames_without_puck=0,
                    frames_with_multiple_pucks=0, player_rows=0, puck_rows=0,
                    unidentified_jersey_rows=0, frames_with_unidentified_players=0,
                    duplicate_player_id_frames=0, player_rows_outside_rink_rectangle=0,
                    puck_rows_outside_rink_rectangle=0, rows_with_missing_coordinates=0,
                    rows_with_invalid_coordinates=0, player_rows_missing_xy=0,
                    puck_rows_missing_xy=0, frames_without_puck_position=0)
    clocks, gaps, player_counts, player_z, puck_z = (collections.Counter() for _ in range(5))
    identities = collections.defaultdict(set)
    positioned_players = collections.Counter()
    periods = set()
    previous = None
    seen = set()
    reader = csv.DictReader(io.StringIO(raw.decode('utf-8-sig')))
    for image_id, group in itertools.groupby(reader, lambda r: r['Image Id']):
        assert image_id not in seen
        seen.add(image_id)
        rows = list(group)
        first = rows[0]
        period = int(first['Period'])
        clock = first['Game Clock']
        minutes, seconds = map(int, clock.split(':'))
        remaining = minutes*60+seconds
        assert all(r['Period'] == first['Period'] and r['Game Clock'] == clock for r in rows)
        if previous is not None and previous[0] == period:
            change = previous[1]-remaining
            key = 'adjacent_frames_same_game_clock' if change == 0 else 'adjacent_frames_decreasing_game_clock' if change > 0 else 'adjacent_frames_increasing_game_clock'
            expected[key] += 1
            try:
                delta = int(image_id.rsplit('_', 1)[1])-int(previous[2].rsplit('_', 1)[1])
                if delta > 0:
                    gaps[delta] += 1
                else:
                    expected['non_increasing_image_suffix_pairs'] += 1
            except (ValueError, IndexError):
                pass
        if previous is None:
            expected['clock_first'] = clock
        previous = period, remaining, image_id
        expected['clock_last'] = clock
        expected['frames'] += 1
        expected['rows'] += len(rows)
        periods.add(period)
        clocks[(period, clock)] += 1
        players = [r for r in rows if r['Player or Puck'] == 'Player']
        pucks = [r for r in rows if r['Player or Puck'] == 'Puck']
        assert len(players)+len(pucks) == len(rows)
        expected['player_rows'] += len(players)
        expected['puck_rows'] += len(pucks)
        expected['frames_without_puck'] += not pucks
        expected['frames_with_multiple_pucks'] += len(pucks) > 1
        expected['duplicate_player_id_frames'] += len({(r['Team'], r['Player Id']) for r in players}) != len(players)
        expected['frames_with_unidentified_players'] += any(not r['Player Jersey Number'].strip() for r in players)
        player_counts[len(players)] += 1
        for row in rows:
            values = [row[k] for k in ['Rink Location X (Feet)', 'Rink Location Y (Feet)', 'Rink Location Z (Feet)']]
            expected['rows_with_missing_coordinates'] += any(not v.strip() for v in values)
            invalid = False
            for value in values:
                if value.strip():
                    try:
                        invalid |= not math.isfinite(float(value))
                    except ValueError:
                        invalid = True
            expected['rows_with_invalid_coordinates'] += invalid
        def outside(row):
            try:
                x, y = float(row['Rink Location X (Feet)']), float(row['Rink Location Y (Feet)'])
                return math.isfinite(x) and math.isfinite(y) and (abs(x) > 100 or abs(y) > 42.5)
            except ValueError:
                return False
        def positioned(row):
            try:
                return all(math.isfinite(float(row[k])) for k in ['Rink Location X (Feet)', 'Rink Location Y (Feet)'])
            except ValueError:
                return False
        positioned_players[sum(positioned(r) for r in players)] += 1
        expected['frames_without_puck_position'] += not any(positioned(r) for r in pucks)
        for r in players:
            expected['player_rows_missing_xy'] += not r['Rink Location X (Feet)'].strip() or not r['Rink Location Y (Feet)'].strip()
            identity = r['Team']+':'+r['Player Id']
            identities[identity]
            if r['Player Jersey Number'].strip():
                identities[identity].add(r['Player Jersey Number'])
            else:
                expected['unidentified_jersey_rows'] += 1
            expected['player_rows_outside_rink_rectangle'] += outside(r)
            player_z[r['Rink Location Z (Feet)']] += 1
        for r in pucks:
            expected['puck_rows_missing_xy'] += not r['Rink Location X (Feet)'].strip() or not r['Rink Location Y (Feet)'].strip()
            expected['puck_rows_outside_rink_rectangle'] += outside(r)
            puck_z[r['Rink Location Z (Feet)']] += 1
    stringify = lambda counter: {str(k): v for k, v in counter.items()}
    expected.update(periods=sorted(periods), game_clock_values=len(clocks),
                    frames_per_clock_histogram=stringify(collections.Counter(clocks.values())),
                    numeric_image_suffix_gaps=stringify(gaps), player_count_histogram=stringify(player_counts),
                    positioned_player_count_histogram=stringify(positioned_players),
                    tracked_actor_keys=len(identities), team_player_ids_with_multiple_jersey_labels=sorted(k for k, labels in identities.items() if len(labels) > 1),
                    player_z_values=dict(player_z), puck_z_values=dict(puck_z))
    for key, value in expected.items():
        assert actual[key] == value, (key, actual[key], value)
    assert actual['source_sha256'] == hashlib.sha256(raw).hexdigest()
    receipt = dict(status='passed', rows=expected['rows'], frames=expected['frames'], fields_checked=len(expected),
                   source_sha256=hashlib.sha256(raw).hexdigest(), report_sha256=hashlib.sha256(report_path.read_bytes()).hexdigest(),
                   limitation='Literal dataset census, not identity validation, FPS calibration or task/capability assessment.')
    with receipt_path.open('x', encoding='utf-8') as stream:
        json.dump(receipt, stream, indent=2)
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
