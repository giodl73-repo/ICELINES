#!/usr/bin/env python3
"""
Fetch NHL historical season data (bios + stats) from the public NHL API.

Saves bios.json and stats.json to data/seasons/{SEASON}/ for each season.
Skips seasons that already have both files.

Usage:
    python3 scripts/fetch_history.py
    python3 scripts/fetch_history.py --seasons 20002001 20012002
    python3 scripts/fetch_history.py --from 20002001 --to 20202021
"""

import argparse
import json
import sys
import time
from pathlib import Path

try:
    import urllib.request as req
    import urllib.error
except ImportError:
    print("Python 3.x required")
    sys.exit(1)

HEADERS = {
    "User-Agent": "icelines/0.1 (https://github.com/giodl73-repo/ICELINES)",
    "Accept": "application/json",
}

BASE = "https://api.nhle.com/stats/rest/en"

# All seasons 2000-01 through 2020-21 (exclusive of current 5 bundled)
# Skip 20042005 — lockout, zero games played
ALL_HISTORICAL = [
    "20002001", "20012002", "20022003", "20032004",
    # 20042005 lockout — skip
    "20052006", "20062007", "20072008", "20082009", "20092010",
    "20102011", "20112012", "20122013",
    "20132014", "20142015", "20152016", "20162017", "20172018",
    "20182019", "20192020", "20202021",
]


def fetch_paged(endpoint: str, delay_ms: int = 300) -> list:
    """Fetch all pages from a paginated NHL stats endpoint."""
    all_rows = []
    start = 0
    limit = 100
    total = None

    while True:
        url = f"{endpoint}&limit={limit}&start={start}"
        try:
            request = req.Request(url, headers=HEADERS)
            with req.urlopen(request, timeout=30) as r:
                data = json.loads(r.read())
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return []
            raise

        rows = data.get("data", [])
        if total is None:
            total = data.get("total", 0)

        all_rows.extend(rows)
        start += len(rows)

        if start >= total or len(rows) == 0:
            break

        time.sleep(delay_ms / 1000.0)

    return all_rows


def fetch_season(season: str, data_root: Path, force: bool = False) -> bool:
    """Fetch bios + stats for one season. Returns True if fetched, False if skipped."""
    dest = data_root / season
    bios_path  = dest / "bios.json"
    stats_path = dest / "stats.json"

    if not force and bios_path.exists() and stats_path.exists():
        bios_count  = len(json.loads(bios_path.read_text()))
        stats_count = len(json.loads(stats_path.read_text()))
        print(f"  {season}: already exists ({bios_count} bios, {stats_count} stats) — skipping")
        return False

    dest.mkdir(parents=True, exist_ok=True)
    print(f"  {season}: fetching bios...", end="", flush=True)

    bios_ep  = f"{BASE}/skater/bios?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2"
    stats_ep = f"{BASE}/skater/summary?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2"

    bios = fetch_paged(bios_ep)
    print(f" {len(bios)} players", end="", flush=True)
    bios_path.write_text(json.dumps(bios, separators=(",", ":")))

    time.sleep(0.5)
    print(f"  | stats...", end="", flush=True)

    stats = fetch_paged(stats_ep)
    print(f" {len(stats)} players")
    stats_path.write_text(json.dumps(stats, separators=(",", ":")))

    return True


def main():
    parser = argparse.ArgumentParser(description="Fetch historical NHL season data")
    parser.add_argument("--seasons", nargs="+", help="Specific season IDs to fetch")
    parser.add_argument("--from", dest="from_season", help="Start season (inclusive)")
    parser.add_argument("--to",   dest="to_season",   help="End season (inclusive)")
    parser.add_argument("--force", action="store_true", help="Re-fetch even if data exists")
    parser.add_argument("--data-dir", default="data/seasons", help="Output directory")
    args = parser.parse_args()

    data_root = Path(args.data_dir)
    data_root.mkdir(parents=True, exist_ok=True)

    if args.seasons:
        seasons = args.seasons
    elif args.from_season or args.to_season:
        start = args.from_season or ALL_HISTORICAL[0]
        end   = args.to_season   or ALL_HISTORICAL[-1]
        seasons = [s for s in ALL_HISTORICAL if start <= s <= end]
    else:
        seasons = ALL_HISTORICAL

    print(f"IceLines Historical Season Fetch")
    print(f"  Seasons: {len(seasons)}  ({seasons[0]}–{seasons[-1]})")
    print(f"  Output:  {data_root.resolve()}")
    print()

    fetched = 0
    skipped = 0
    errors  = 0

    for season in seasons:
        try:
            if fetch_season(season, data_root, force=args.force):
                fetched += 1
            else:
                skipped += 1
        except Exception as e:
            print(f"  {season}: ERROR — {e}")
            errors += 1
            time.sleep(2)

    print()
    print(f"Done. Fetched {fetched}, skipped {skipped}, errors {errors}.")
    if fetched > 0:
        print(f"Commit data/seasons/ and push to make available via GitHub Releases.")


if __name__ == "__main__":
    main()
