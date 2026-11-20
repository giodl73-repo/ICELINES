"""Independent arithmetic/rank oracle. Usage: DIRECTORY REPORT_JSON RESULT_JSON."""
import hashlib
import json
import math
import sys
from pathlib import Path

directory, report_path, result_path = map(Path, sys.argv[1:])
report = json.loads(report_path.read_text(encoding="utf-8"))
assert report["schema"] == "rinkcraft.edge-exposure.v1"
assert hashlib.sha256((directory / "catalog.json").read_bytes()).hexdigest() == report["catalog_sha256"]
catalog = json.loads((directory / "catalog.json").read_bytes())
paths = [p["url"] for w in catalog["windows"] for p in w["players"]]
assert len(paths) == len(set(paths)) == len(report["inputs"])
assert set(paths) == {item["path"] for item in report["inputs"]}
assert [(s["min_games"], s["min_minutes"]) for s in report["scenarios"]] == [(5, 100), (10, 100), (20, 100)]
expected = {}
for item in report["inputs"]:
    path = Path(item["path"])
    assert not path.is_absolute() and ".." not in path.parts
    raw = (directory / path).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == item["sha256"]
    frame = json.loads(raw)["numerical_evidence"]
    edge = frame.get("edge_evidence")
    if frame["group"] == "Goalies" or not edge:
        continue
    measures = {m["key"]: m["value"] for m in edge["measures"]}
    burst = measures.get("speed-bursts-over-20")
    key = (frame["season"], frame["player_id"])
    assert key not in expected
    expected[key] = dict(
        season=frame["season"], player_id=frame["player_id"], name=frame["name"],
        group=frame["group"], games=frame["games"], minutes=frame["minutes"],
        peak_mph=measures.get("max-skating-speed"), bursts=burst,
        bursts_per_game=None if burst is None else burst / frame["games"],
        bursts_per_60=None if burst is None else burst * 60 / frame["minutes"],
        edge_sha256=edge["source"]["sha256"],
    )

def equal(a, b):
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12), (a, b)
    else:
        assert a == b, (a, b)

checks = 0
for scenario in report["scenarios"]:
    eligible = lambda r: r["games"] >= scenario["min_games"] and r["minutes"] >= scenario["min_minutes"]
    pools = {}
    for row in expected.values():
        if eligible(row):
            for metric in ("peak_mph", "bursts", "bursts_per_60"):
                if row[metric] is not None:
                    pools.setdefault((row["season"], row["group"], metric), []).append(row[metric])
    seen = set()
    for actual in scenario["rows"]:
        r = actual["record"]
        key = (r["season"], r["player_id"])
        assert key not in seen
        seen.add(key)
        want = expected[key]
        assert r.keys() == want.keys()
        for field in want:
            equal(r[field], want[field])
        assert actual["eligible"] == eligible(want)
        wanted_metrics = {m for m in ("peak_mph", "bursts", "bursts_per_60") if eligible(want) and want[m] is not None}
        assert {c["metric"] for c in actual["comparisons"]} == wanted_metrics
        assert len(actual["comparisons"]) == len(wanted_metrics)
        for c in actual["comparisons"]:
            metric = c["metric"]
            equal(c["value"], want[metric])
            peers = pools[(r["season"], r["group"], metric)]
            rank = 100 * (sum(x < want[metric] for x in peers) + .5 * sum(x == want[metric] for x in peers)) / len(peers)
            equal(c["percentile"], rank)
            assert c["peers"] == len(peers)
            checks += 1
    assert seen == expected.keys()
result = dict(input_hashes=len(report["inputs"]), matched_skaters=len(expected), scenarios=len(report["scenarios"]), independently_checked_ranks=checks)
assert not result_path.exists(), "new output required"
result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps(result))
