"""Compare a local native profile server with saved CLI evidence snapshots.

Usage: python tools/check-rinkcraft-preview.py PATH_TO_STAT_PROFILES [LOCAL_URL]
No snapshots or server state are modified.
"""
import json
from pathlib import Path
import sys
from urllib.parse import urlencode, urlparse
from urllib.request import urlopen

root = Path(sys.argv[1])
base = sys.argv[2] if len(sys.argv) > 2 else "http://127.0.0.1:8765"
assert urlparse(base).hostname in {"127.0.0.1", "localhost", "::1"}, "local server required"
checked = []
for path in sorted(root.glob("*.json")):
    saved = json.loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(saved, dict) or saved.get("schema") != "icelines.rinkcraft-stat-profile.v1" or "request" not in saved:
        continue
    request = saved["request"]
    query = {"type": request["season_type"]}
    for key in ["first_season", "last_season"]:
        if request[key] is not None:
            query[key] = request[key]
    with urlopen(f"{base}/api/v1/player/{request['player_id']}/rinkcraft?{urlencode(query)}", timeout=30) as response:
        current = json.load(response)["data"]
    for key in ["request", "player_name", "included_seasons", "latest_available", "summary", "timeline", "changes", "source_windows"]:
        assert current[key] == saved[key], f"{path.name}: mismatch in {key}"
    checked.append(path.name)
assert checked, "no CLI evidence snapshots found"
print(json.dumps({"matched": checked, "fields": "identity, scope, metrics, changes, source gaps"}))
