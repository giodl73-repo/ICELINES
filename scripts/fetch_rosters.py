#!/usr/bin/env python
"""
fetch_rosters.py — pull full roster data from NHL API for all 32 teams.

Writes data/rosters.json: for each player, their NHL player ID,
current team, position, bio, and headshot URL from the NHL CDN.
This replaces the Yahoo CDN photo URLs with real, durable NHL photos.

Usage:
    python scripts/fetch_rosters.py [--season 20252026]
"""

import json
import os
import sys
import time
import unicodedata
import urllib.request

SEASON   = '20252026'
OUT_PATH = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'data', 'rosters.json')
HEADERS  = {'User-Agent': 'Mozilla/5.0', 'Accept': 'application/json'}

# All 32 NHL teams — NHL API abbreviations
NHL_TEAMS = [
    'ANA','BOS','BUF','CAR','CBJ','CGY','CHI','COL',
    'DAL','DET','EDM','FLA','LAK','MIN','MTL','NJD',
    'NSH','NYI','NYR','OTT','PHI','PIT','SEA','SJS',
    'STL','TBL','TOR','UTA','VAN','VGK','WPG','WSH',
]

# Yahoo abbreviation → NHL abbreviation (for matching with Yahoo CSV)
YAHOO_TO_NHL = {
    'LA': 'LAK', 'NJ': 'NJD', 'TB': 'TBL',
    'SJ': 'SJS',
}
# NHL abbreviation → Yahoo abbreviation
NHL_TO_YAHOO = {v: k for k, v in YAHOO_TO_NHL.items()}


def fetch(url):
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=15) as r:
        return json.loads(r.read())


def normalize_name(s):
    return ''.join(
        c for c in unicodedata.normalize('NFD', s)
        if unicodedata.category(c) != 'Mn'
    ).lower().strip()


def extract_name(field):
    """NHL API name fields are either a string or {'default': '...'}"""
    if isinstance(field, dict):
        return field.get('default', '')
    return str(field)


def main():
    season = SEASON
    for arg in sys.argv[1:]:
        if arg.startswith('--season'):
            season = arg.split('=')[-1] if '=' in arg else sys.argv[sys.argv.index(arg) + 1]

    print(f'Fetching rosters for season {season}...')
    all_players = {}   # normalized_name → player dict

    for team in NHL_TEAMS:
        url = f'https://api-web.nhle.com/v1/roster/{team}/{season}'
        try:
            data = fetch(url)
        except Exception as e:
            print(f'  WARN: {team} — {e}')
            continue

        count = 0
        for group in ['forwards', 'defensemen', 'goalies']:
            for p in data.get(group, []):
                first = extract_name(p.get('firstName', ''))
                last  = extract_name(p.get('lastName', ''))
                full  = f'{first} {last}'.strip()
                pid   = p.get('id')
                pos   = p.get('positionCode', '')
                headshot = p.get('headshot', '')

                # Yahoo team abbreviation for matching
                yahoo_team = NHL_TO_YAHOO.get(team, team)

                record = {
                    'player_id':   pid,
                    'full_name':   full,
                    'first_name':  first,
                    'last_name':   last,
                    'nhl_team':    team,
                    'yahoo_team':  yahoo_team,
                    'position':    pos,
                    'shoots':      p.get('shootsCatches', ''),
                    'birth_date':  p.get('birthDate', ''),
                    'birth_country': p.get('birthCountry', ''),
                    'headshot':    headshot,
                    'sweater':     p.get('sweaterNumber'),
                    'height_in':   p.get('heightInInches'),
                    'weight_lb':   p.get('weightInPounds'),
                }

                key = normalize_name(full)
                all_players[key] = record
                count += 1

        print(f'  {team}: {count} players')
        time.sleep(0.1)

    os.makedirs(os.path.dirname(OUT_PATH), exist_ok=True)
    with open(OUT_PATH, 'w', encoding='utf-8') as f:
        json.dump(all_players, f, ensure_ascii=False, indent=2)

    print(f'\nSaved {len(all_players)} players -> {OUT_PATH}')
    print('Sample headshot URL:')
    sample = next(iter(all_players.values()))
    print(f'  {sample["full_name"]}: {sample["headshot"]}')


if __name__ == '__main__':
    main()
