"""Verify actual browser downloads for the recorded 2023-24 regular slice."""
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("downloads", type=Path)
parser.add_argument("catalog", type=Path)
args = parser.parse_args()
catalog = json.loads(args.catalog.read_text(encoding="utf-8"))
entry = next(item for item in catalog["packages"]
             if item["season"] == 20232024 and item["season_type"] == "regular")
package_bytes = (args.downloads / "icelines-20232024-regular.json").read_bytes()
assert len(package_bytes) == entry["bytes"]
assert hashlib.sha256(package_bytes).hexdigest() == entry["sha256"]
package = json.loads(package_bytes)
assert (package["schema_version"], package["season"], package["season_type"]) == (1, 20232024, "regular")
result = json.loads((args.downloads / "icelines-20232024-query.json").read_text(encoding="utf-8"))
assert result["revision"] == entry["sha256"]
assert (result["season"], result["season_type"]) == (20232024, "regular")
assert result["query"]["filter"] == "p >= 100"
assert result["query"]["sort"] == "points"
assert result["minimum_games"] == 0 and result["pace_minimum_games"] == 10
assert result["observed_at"] is None and result["fetched_at"] is None
expected = [("Nikita Kucherov",144), ("Nathan MacKinnon",140),
            ("Connor McDavid",132), ("Artemi Panarin",120),
            ("David Pastrnak",110), ("Auston Matthews",107),
            ("Leon Draisaitl",106), ("Mikko Rantanen",104), ("J.T. Miller",103)]
assert [(row["name"], row["points"]) for row in result["rows"]] == expected
records = list(csv.reader(io.StringIO((args.downloads / "icelines-20232024-query.csv").read_text(encoding="utf-8"))))
metadata = [row[0] for row in records if len(row) == 1 and row[0].startswith("#")]
assert any(entry["sha256"] in item for item in metadata)
assert any('"filter":"p >= 100"' in item for item in metadata)
data = [row for row in records if not (len(row) == 1 and row[0].startswith("#"))]
header, *rows = data
assert len(rows) == len(result["rows"]) == 9
for csv_row, json_row in zip(rows, result["rows"]):
    assert len(csv_row) == len(header)
    for field, cell in zip(header, csv_row):
        value = json_row[field]
        if value is None:
            assert cell == ""
        elif isinstance(value, (int, float)):
            assert float(cell) == value
        else:
            assert cell == value
print(json.dumps({"package_bytes": len(package_bytes), "sha256": entry["sha256"],
                  "query_rows": len(rows), "json_csv_agree": True}, indent=2))
