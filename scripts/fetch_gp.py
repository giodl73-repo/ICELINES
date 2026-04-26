#!/usr/bin/env python
"""
fetch_gp.py — pull fresh GP data from NHL API

Writes data/gp_data.json with gamesPlayed, playerId,
skaterFullName, currentTeamAbbrev for all skaters.

Usage:
    python scripts/fetch_gp.py [--season 20252026]
"""

import json
import os
import sys
import time
import urllib.request

SEASON   = '20252026'
OUT_PATH = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'data', 'gp_data.json')
BASE_URL = ('https://api.nhle.com/stats/rest/en/skater/bios'
            '?limit=100&cayenneExp=seasonId={season}%20and%20gameTypeId=2&start={start}')
HEADERS  = {'User-Agent': 'Mozilla/5.0', 'Accept': 'application/json'}


def fetch(url):
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=20) as r:
        return json.loads(r.read())


def main():
    season = SEASON
    for arg in sys.argv[1:]:
        if arg.startswith('--season'):
            season = arg.split('=')[-1] if '=' in arg else sys.argv[sys.argv.index(arg) + 1]

    print(f'Fetching GP data for season {season}...')
    all_players = []
    start = 0
    total = None

    while True:
        url  = BASE_URL.format(season=season, start=start)
        data = fetch(url)
        batch = data.get('data', [])
        if total is None:
            total = data.get('total', '?')
        if not batch:
            break
        all_players.extend(batch)
        print(f'  {len(all_players)} / {total}')
        start += len(batch)
        if len(all_players) >= (total if isinstance(total, int) else 9999):
            break
        time.sleep(0.2)

    os.makedirs(os.path.dirname(OUT_PATH), exist_ok=True)
    with open(OUT_PATH, 'w', encoding='utf-8') as f:
        json.dump(all_players, f)

    print(f'Saved {len(all_players)} players → {OUT_PATH}')


if __name__ == '__main__':
    main()
