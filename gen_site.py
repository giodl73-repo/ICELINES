#!/usr/bin/env python
"""
gen_site.py  —  NHL Fantasy Tracker site generator

Reads Yahoo fantasy CSV, computes lineup-fit metrics,
writes docs/index.md (32-team tracker) and docs/teams/{TEAM}.md
(real 4×3 forward + 3×2 defense lineup card per team).

Usage:
    cd C:\\src\\NHL\\fantasy-tracker
    python gen_site.py
"""

import csv
import io
import json
import os
import sys
import unicodedata
from collections import defaultdict

CSV_PATH  = r'C:\Users\giodl\Downloads\Yahoo-465.l.1214-Players.csv'
GP_PATH   = os.path.join(os.path.dirname(__file__), 'gp_data.json')
DOCS_DIR  = os.path.join(os.path.dirname(__file__), 'docs')
TEAMS_DIR = os.path.join(DOCS_DIR, 'teams')

FULL_SEASON = 82   # project everything to this many games
MIN_GP      = 10   # minimum GP to use per-game projection (below this use raw)

FORWARD_POSITIONS = {'C', 'LW', 'RW'}

GOALIE_SCORING = {
    'W (G)': 5.0, 'L (G)': -2.0, 'SV (G)': 0.15,
    'GA (G)': -1.0, 'SHO (G)': 4.0,
}

FWD_LINES  = 4
DEF_PAIRS  = 3

TEAM_NAMES = {
    'ANA': 'Anaheim Ducks',     'BOS': 'Boston Bruins',
    'BUF': 'Buffalo Sabres',    'CAR': 'Carolina Hurricanes',
    'CBJ': 'Columbus Blue Jackets', 'CGY': 'Calgary Flames',
    'CHI': 'Chicago Blackhawks','COL': 'Colorado Avalanche',
    'DAL': 'Dallas Stars',      'DET': 'Detroit Red Wings',
    'EDM': 'Edmonton Oilers',   'FLA': 'Florida Panthers',
    'LA':  'Los Angeles Kings', 'MIN': 'Minnesota Wild',
    'MTL': 'Montréal Canadiens','NJ':  'New Jersey Devils',
    'NSH': 'Nashville Predators','NYI': 'New York Islanders',
    'NYR': 'New York Rangers',  'OTT': 'Ottawa Senators',
    'PHI': 'Philadelphia Flyers','PIT': 'Pittsburgh Penguins',
    'SEA': 'Seattle Kraken',    'SJ':  'San Jose Sharks',
    'STL': 'St. Louis Blues',   'TB':  'Tampa Bay Lightning',
    'TOR': 'Toronto Maple Leafs','UTA': 'Utah Hockey Club',
    'VAN': 'Vancouver Canucks', 'VGK': 'Vegas Golden Knights',
    'WPG': 'Winnipeg Jets',     'WSH': 'Washington Capitals',
}

# ─────────────────────────────────────────────────────────────────────────────
# DATA LOADING
# ─────────────────────────────────────────────────────────────────────────────

def safe_float(v):
    try: return float(v) if v and str(v).strip() else 0.0
    except: return 0.0

def normalize_name(s):
    """Strip accents for fuzzy name matching."""
    return ''.join(c for c in unicodedata.normalize('NFD', s)
                   if unicodedata.category(c) != 'Mn').lower().strip()

def load_gp_lookup(gp_path):
    """Returns {normalize_name(fullName): {'gp', 'team', 'player_id'}} from NHL API data."""
    with open(gp_path, encoding='utf-8') as f:
        records = json.load(f)
    lookup = {}
    for r in records:
        key = normalize_name(r.get('skaterFullName', ''))
        if key:
            lookup[key] = {
                'gp':        r.get('gamesPlayed', 0),
                'team':      r.get('currentTeamAbbrev', ''),
                'player_id': r.get('playerId', 0),
            }
    return lookup

# NHL CDN abbreviation overrides (Yahoo abbrev → NHL CDN abbrev)
NHL_LOGO_ABBREV = {
    'LA': 'LAK', 'NJ': 'NJD', 'TB': 'TBL',
    'SJ': 'SJS', 'UTA': 'UTA',
}

def team_logo_url(team, dark=False):
    abbrev = NHL_LOGO_ABBREV.get(team, team)
    mode   = 'dark' if dark else 'light'
    return f'https://assets.nhle.com/logos/nhl/svg/{abbrev}_{mode}.svg'

def player_photo_url(player_id):
    # NHL CDN returns generic placeholder — not used
    return ''

def elig_fwd(s):
    return [t.strip() for t in s.split(',') if t.strip() in FORWARD_POSITIONS]

def is_defense(s):
    toks = [t.strip() for t in s.split(',')]
    return 'D' in toks and not any(t in FORWARD_POSITIONS for t in toks)

def is_goalie(s):
    toks = [t.strip() for t in s.split(',')]
    return 'G' in toks and not any(t in FORWARD_POSITIONS for t in toks) and 'D' not in toks

def compute_goalie_fpts(row):
    return sum(safe_float(row.get(c, 0)) * w for c, w in GOALIE_SCORING.items())

def skater_score(g, a, gp):
    """
    Primary metric: points per 82 games.
    Tiebreaker embedded: goals per 82 games weighted at 0.001.
    Returns a single float — higher = better.
    """
    if gp >= MIN_GP:
        pts82 = (g + a) / gp * FULL_SEASON
        g82   = g       / gp * FULL_SEASON
    else:
        pts82 = g + a   # raw for very-low-GP players
        g82   = g
    return pts82 + g82 * 0.001

def load_all(csv_path, gp_lookup):
    """
    Load skaters + goalies.
    _fpts  = pts/82g + g/82g*0.001   ← drives ALL rankings
    _pts82 = points projected to 82 games  (display)
    _g82   = goals projected to 82 games   (display)
    _ppg   = raw points-per-game rate
    _gpg   = raw goals-per-game rate
    """
    skaters, goalies = [], []
    unmatched = []
    with open(csv_path, newline='', encoding='utf-8-sig') as f:
        for row in csv.DictReader(f):
            ep   = row.get('Eligible Positions', '')
            name = f"{row['First Name']} {row['Last Name']}"
            team = row.get('Team', '')
            if not team:
                continue
            if is_goalie(ep):
                row['_fpts']      = compute_goalie_fpts(row)
                row['_gp']        = 0
                row['_ppg']       = 0.0
                row['_gpg']       = 0.0
                row['_pts82']     = row['_fpts']
                row['_g82']       = 0.0
                row['_name']      = name
                row['_pos']       = 'G'
                row['_player_id'] = 0
                goalies.append(row)
            else:
                fp = elig_fwd(ep)
                if fp or is_defense(ep):
                    g   = safe_float(row.get('G (P)', 0))
                    a   = safe_float(row.get('A (P)', 0))
                    key = normalize_name(name)
                    gp_info = gp_lookup.get(key)
                    gp = gp_info['gp'] if gp_info else 0
                    if not gp_info:
                        unmatched.append(name)
                    ppg  = (g + a) / gp if gp >= MIN_GP else 0.0
                    gpg  = g       / gp if gp >= MIN_GP else 0.0
                    row['_fpts']      = skater_score(g, a, gp)
                    row['_gp']        = gp
                    row['_ppg']       = ppg
                    row['_gpg']       = gpg
                    row['_pts82']     = ppg * FULL_SEASON if gp >= MIN_GP else (g + a)
                    row['_g82']       = gpg * FULL_SEASON if gp >= MIN_GP else g
                    row['_name']      = name
                    row['_is_fwd']    = bool(fp)
                    row['_fpos']      = fp
                    row['_pos']       = None
                    row['_player_id'] = gp_info['player_id'] if gp_info else 0
                    row['_photo']     = row.get('Image', '')
                    skaters.append(row)
    if unmatched:
        print(f'  GP unmatched for {len(unmatched)} players (raw pts used)')
    return skaters, goalies

# ─────────────────────────────────────────────────────────────────────────────
# POSITION ASSIGNMENT  (greedy, best-rank first)
# ─────────────────────────────────────────────────────────────────────────────

def assign_positions(skaters):
    by_team = defaultdict(list)
    for p in skaters:
        by_team[p['Team']].append(p)
    for _, players in by_team.items():
        fwds = sorted([p for p in players if p['_is_fwd']],
                      key=lambda x: x['_fpts'], reverse=True)
        counts = defaultdict(int)
        for p in fwds:
            best = min(p['_fpos'], key=lambda pos: counts[pos])
            p['_pos'] = best
            counts[best] += 1
        for p in players:
            if not p['_is_fwd']:
                p['_pos'] = 'D'

# ─────────────────────────────────────────────────────────────────────────────
# CHARTS & CROSS-TEAM METRICS
# ─────────────────────────────────────────────────────────────────────────────

def build_charts(skaters, goalies):
    charts = defaultdict(lambda: defaultdict(list))
    for p in skaters:
        if p['Team'] and p['_pos']:
            charts[p['Team']][p['_pos']].append(p)
    for g in goalies:
        if g['Team']:
            charts[g['Team']]['G'].append(g)
    for t in charts:
        for pos in charts[t]:
            charts[t][pos].sort(key=lambda x: x['_fpts'], reverse=True)
    return charts

def compute_metrics(skaters, charts, all_teams):
    fpts_cache = {}
    for team in all_teams:
        for pos in list(FORWARD_POSITIONS) + ['D']:
            fpts_cache[(pos, team)] = [p['_fpts'] for p in charts[team].get(pos, [])]
    other_map = {t: [o for o in all_teams if o != t] for t in all_teams}
    for p in skaters:
        pos, team, fpts = p['_pos'], p['Team'], p['_fpts']
        if not pos or not team:
            continue
        own = fpts_cache.get((pos, team), [])
        p['_own_line'] = sum(1 for f in own if f > fpts) + 1
        lines = [sum(1 for f in fpts_cache.get((pos, t), []) if f > fpts) + 1
                 for t in other_map.get(team, [])]
        p['_avg_other'] = sum(lines) / len(lines) if lines else 0.0
        p['_delta'] = p['_own_line'] - p['_avg_other']

def team_strength(charts, all_teams):
    st = {}
    for team in all_teams:
        total = 0
        pos_data = {}
        for pos, depth in [('C', FWD_LINES), ('LW', FWD_LINES), ('RW', FWD_LINES), ('D', DEF_PAIRS * 2)]:
            grp  = charts[team].get(pos, [])
            pts  = sum(p['_fpts'] for p in grp[:depth])
            top1 = grp[0] if grp else None
            pos_data[pos] = {'pts': pts, 'top': top1, 'grp': grp}
            total += pts
        st[team] = {'total': total, 'pos': pos_data}
    return st

# ─────────────────────────────────────────────────────────────────────────────
# FIT CLASSIFICATION
# ─────────────────────────────────────────────────────────────────────────────

def fit_class(player):
    """
    Four tiers based on avg_other_line vs own_line:

      buried : avg < own_line − 0.75   → clearly better than their current slot (gold)
      fit    : avg ≤ own_line + 0.5    → true caliber for this line (green)
      solid  : avg ≤ own_line + 1.25   → ok but not elite for this line (yellow)
      stretch: avg > own_line + 1.25   → overextended, playing above their level (red)

    Example thresholds for Line 1:
      fit    : avg ≤ 1.5   (would be L1–2 on most teams)
      solid  : avg 1.5–2.25 (would be L2–3 on most teams)
      stretch: avg > 2.25  (3rd liner or worse playing L1)
    """
    if player is None:
        return 'empty'
    own = player.get('_own_line', 1)
    avg = player.get('_avg_other', float(own))
    if own - avg > 0.75:
        return 'buried'
    if avg <= own + 0.5:
        return 'fit'
    if avg <= own + 1.25:
        return 'solid'
    return 'stretch'

def fit_label(player):
    if player is None:
        return ''
    own = player.get('_own_line', 1)
    avg = player.get('_avg_other', float(own))
    if own - avg > 0.75:
        return f'↑ avg L{avg:.1f} — underused'
    if avg <= own + 0.5:
        return f'★ avg L{avg:.1f}'
    if avg <= own + 1.25:
        return f'~ avg L{avg:.1f}'
    return f'↓ avg L{avg:.1f} — overextended'

# ─────────────────────────────────────────────────────────────────────────────
# POSITION STATUS HELPERS
# ─────────────────────────────────────────────────────────────────────────────

def pos_status(grp, depth, threshold=2.0):
    surplus = sum(1 for p in grp if p.get('_avg_other', 99) <= threshold)
    best    = grp[0].get('_avg_other', 99) if grp else 99
    if surplus >= 3:   return 'elite',   '⭐⭐'
    if surplus >= 2:   return 'loaded',  '⭐'
    if best    <= 2.0: return 'solid',   '✓'
    if best    <= 3.0: return 'average', '—'
    if best    <= 4.0: return 'thin',    '⚠'
    return 'buy', '🔴'

def pos_rank_label(pos, team, strength, all_teams):
    ranked = sorted(all_teams, key=lambda t: strength[t]['pos'][pos]['pts'], reverse=True)
    return ranked.index(team) + 1

# ─────────────────────────────────────────────────────────────────────────────
# HTML HELPERS
# ─────────────────────────────────────────────────────────────────────────────

def player_cell(p):
    if p is None:
        return '<td class="player-cell empty"><span class="player-name">—</span></td>'
    cls       = fit_class(p)
    label     = fit_label(p)
    name      = p['_name']
    gp        = p.get('_gp', 0)
    ppg       = p.get('_ppg', 0.0)
    gpg       = p.get('_gpg', 0.0)
    pts82     = p.get('_pts82', 0.0)
    g82       = p.get('_g82', 0.0)
    photo_url = p.get('_photo', '')
    photo_tag = (f'<img class="player-photo" src="{photo_url}" '
                 f'alt="{name}" onerror="this.style.display=\'none\'">'
                 if photo_url else '')
    if gp >= MIN_GP:
        stat_str = (f'<span class="player-gp">'
                    f'{gp}gp &nbsp;·&nbsp; '
                    f'<b>{ppg:.2f}</b> pts/gp &nbsp;·&nbsp; '
                    f'{gpg:.2f} g/gp &nbsp;·&nbsp; '
                    f'{pts82:.0f} proj'
                    f'</span>')
    else:
        raw_pts = p.get('_pts82', 0.0)
        stat_str = f'<span class="player-gp">{raw_pts:.0f} pts raw</span>'
    return (
        f'<td class="player-cell {cls}">'
        f'{photo_tag}'
        f'<span class="player-name">{name}</span>'
        f'{stat_str}'
        f'<span class="player-fit-label">{label}</span>'
        f'</td>'
    )

def bar_html(pts, max_pts, width=120):
    fill = int(round(pts / max_pts * width)) if max_pts > 0 else 0
    return (
        f'<div class="tracker-bar">'
        f'<div class="tracker-bar-fill" style="width:{fill}px"></div>'
        f'</div>'
        f'<div class="tracker-score">{pts:.0f}</div>'
    )

# ─────────────────────────────────────────────────────────────────────────────
# TEAM PAGE GENERATOR
# ─────────────────────────────────────────────────────────────────────────────

def gen_team_page(team, charts, strength, all_teams, overall_rank):
    name  = TEAM_NAMES.get(team, team)
    total = strength[team]['total']
    pos   = strength[team]['pos']

    lines = []
    w = lines.append

    logo = team_logo_url(team)
    w(f'<div class="team-page-header">'
      f'<img class="team-page-logo" src="{logo}" alt="{name} logo">'
      f'<div>'
      f'<h1>{team} — {name}</h1>'
      f'<p class="team-page-meta"><strong>Overall Rank: #{overall_rank} of 32</strong> &nbsp;|&nbsp; '
      f'<strong>{total:,.0f} proj pts</strong> (pts/gp pace × 82 · top-4 F + top-6 D)</p>'
      f'</div>'
      f'</div>')
    w('')

    # Position summary pills
    w('<div class="pos-summary">')
    for p_abbr, label in [('C','C'), ('LW','LW'), ('RW','RW'), ('D','D')]:
        grp    = pos[p_abbr]['grp']
        depth  = FWD_LINES if p_abbr != 'D' else DEF_PAIRS * 2
        cls, _ = pos_status(grp, depth)
        rank   = pos_rank_label(p_abbr, team, strength, all_teams)
        pts_v  = pos[p_abbr]['pts']
        w(f'<div class="pos-pill {cls}">'
          f'<b>{label}</b> #{rank} · {pts_v:.0f} pts'
          f'</div>')
    w('</div>')
    w('')

    # ── FORWARD LINES ──────────────────────────────────────────────────────
    w('## Forward Lines')
    w('')
    w('<div class="lineup-section">')
    w('<table class="lineup-table">')
    w('<thead><tr>'
      '<th class="line-col"></th>'
      '<th class="pos-lw">LW</th>'
      '<th class="pos-c">C</th>'
      '<th class="pos-rw">RW</th>'
      '</tr></thead>')
    w('<tbody>')

    for line_num in range(1, FWD_LINES + 1):
        idx = line_num - 1
        lw  = charts[team].get('LW', [])[idx] if idx < len(charts[team].get('LW', [])) else None
        c   = charts[team].get('C',  [])[idx] if idx < len(charts[team].get('C',  [])) else None
        rw  = charts[team].get('RW', [])[idx] if idx < len(charts[team].get('RW', [])) else None
        w(f'<tr>')
        w(f'<td class="line-num-cell">Line {line_num}</td>')
        w(player_cell(lw))
        w(player_cell(c))
        w(player_cell(rw))
        w(f'</tr>')

    w('</tbody></table></div>')
    w('')

    # ── DEFENSE PAIRS ──────────────────────────────────────────────────────
    w('## Defense Pairs')
    w('')
    w('<div class="lineup-section">')
    w('<table class="lineup-table">')
    w('<thead><tr>'
      '<th class="line-col"></th>'
      '<th class="pos-d">D</th>'
      '<th class="pos-d">D</th>'
      '</tr></thead>')
    w('<tbody>')

    d_grp = charts[team].get('D', [])
    for pair_num in range(1, DEF_PAIRS + 1):
        d1_idx = (pair_num - 1) * 2
        d2_idx = d1_idx + 1
        d1 = d_grp[d1_idx] if d1_idx < len(d_grp) else None
        d2 = d_grp[d2_idx] if d2_idx < len(d_grp) else None
        w(f'<tr>')
        w(f'<td class="pair-num-cell">Pair {pair_num}</td>')
        w(player_cell(d1))
        w(player_cell(d2))
        w(f'</tr>')

    w('</tbody></table></div>')
    w('')

    # ── GOALIES ────────────────────────────────────────────────────────────
    goalies = charts[team].get('G', [])
    if goalies:
        w('## Goalies')
        w('')
        w('| Starter | fpts |')
        w('|---------|------|')
        for g in goalies[:2]:
            w(f'| {g["_name"]} | {g["_fpts"]:.0f} |')
        w('')

    # ── TRADE ANALYSIS ─────────────────────────────────────────────────────
    w('## Trade Analysis')
    w('')
    w('<div class="trade-grid">')

    # DEAL candidates (buried assets)
    all_skaters_team = []
    for p_abbr in ['C', 'LW', 'RW', 'D']:
        all_skaters_team.extend(charts[team].get(p_abbr, []))

    deal_players = sorted(
        [p for p in all_skaters_team if p.get('_avg_other', 99) <= 2.0],
        key=lambda x: x['_avg_other']
    )
    w('<div class="trade-box deal">')
    w('<h4>🟢 Can Deal (avg ≤ Line 2 elsewhere)</h4>')
    if deal_players:
        for p in deal_players[:6]:
            w(f'<div class="trade-item">'
              f'<b>{p["_name"]}</b> ({p["_pos"]}) · '
              f'{p["_fpts"]:.0f} fpts · avg L{p["_avg_other"]:.2f}'
              f'</div>')
    else:
        w('<div class="trade-item">No franchise-level assets</div>')
    w('</div>')

    # BUY positions
    buy_pos = []
    for p_abbr in ['C', 'LW', 'RW', 'D']:
        grp  = pos[p_abbr]['grp']
        depth = FWD_LINES if p_abbr != 'D' else DEF_PAIRS * 2
        cls, _ = pos_status(grp, depth)
        if cls in ('thin', 'buy'):
            best_avg = grp[0].get('_avg_other', 99) if grp else 99
            buy_pos.append((p_abbr, best_avg))

    w('<div class="trade-box buy">')
    w('<h4>🔴 Need to Acquire</h4>')
    if buy_pos:
        for p_abbr, best_avg in buy_pos:
            rk = pos_rank_label(p_abbr, team, strength, all_teams)
            w(f'<div class="trade-item">'
              f'<b>{p_abbr}</b> — ranked #{rk}, best player avg L{best_avg:.2f}'
              f'</div>')
    else:
        w('<div class="trade-item">No critical holes</div>')
    w('</div>')

    w('</div>')  # trade-grid
    w('')

    # Buried assets on this team
    buried = sorted(
        [p for p in all_skaters_team if p.get('_delta', 0) > 0.75 and p.get('_fpts', 0) >= 80],
        key=lambda x: -x['_delta']
    )
    if buried:
        w('### Buried Assets')
        w('')
        w('These players would play a higher line on most other teams:')
        w('')
        w('| Player | Pos | fpts | Own Line | Avg Elsewhere | Could Be |')
        w('|--------|-----|------|----------|---------------|----------|')
        for p in buried[:5]:
            w(f'| {p["_name"]} | {p["_pos"]} | {p["_fpts"]:.0f} | '
              f'Line {p["_own_line"]} | L{p["_avg_other"]:.2f} | '
              f'Line {round(p["_avg_other"])} on most teams |')
        w('')

    w('---')
    w('')
    w('*Back to [League Tracker](../index.md)*')

    return '\n'.join(lines)

# ─────────────────────────────────────────────────────────────────────────────
# INDEX PAGE  (2 columns × 16 teams)
# ─────────────────────────────────────────────────────────────────────────────

def gen_index(ranked_teams, strength, charts, all_teams):
    max_total = strength[ranked_teams[0]]['total']

    lines = []
    w = lines.append

    w('# NHL Fantasy Tracker — 2025–26')
    w('')
    w('Real lineup depth charts and trade analysis for all 32 teams.')
    w('')
    w(f'**Ranking metric: Points per game** (G+A / GP × 82), tiebroken by goals per game. '
      f'Players with <{MIN_GP} GP use raw totals. '
      f'Team score = sum of pace-projected pts for top-4 C + top-4 LW + top-4 RW + top-6 D.')
    w('')
    w('**Fit key:** ✓ fits role · ↑ buried (better than their line) · ↓ overextended · ⭐ loaded · 🔴 need to buy')
    w('')

    w('<div class="tracker-grid">')

    def render_col(teams_slice):
        lines2 = ['<div>']
        for rank, team in teams_slice:
            total   = strength[team]['total']
            name    = TEAM_NAMES.get(team, team)
            fill_px = int(round(total / max_total * 120))
            rank_cls = ('top5' if rank <= 5 else
                        'top10' if rank <= 10 else
                        'bot5' if rank >= 28 else '')

            badges = []
            for p_abbr in ['C', 'LW', 'RW', 'D']:
                grp   = charts[team].get(p_abbr, [])
                depth = FWD_LINES if p_abbr != 'D' else DEF_PAIRS * 2
                cls, _ = pos_status(grp, depth)
                if cls in ('elite', 'loaded'):
                    badge_cls = 'elite' if cls == 'elite' else 'loaded'
                    badges.append(f'<span class="pos-badge {badge_cls}">{p_abbr}</span>')
                elif cls in ('thin', 'buy'):
                    badges.append(f'<span class="pos-badge buy">{p_abbr}🔴</span>')
            badge_html = ''.join(badges)

            logo = team_logo_url(team)
            lines2.append(
                f'<a class="tracker-card" href="teams/{team}.md">'
                f'<span class="tracker-rank {rank_cls}">#{rank}</span>'
                f'<img class="tracker-logo" src="{logo}" alt="{team}">'
                f'<div class="tracker-bar-wrap">'
                f'<div class="tracker-bar">'
                f'<div class="tracker-bar-fill" style="width:{fill_px}px"></div>'
                f'</div>'
                f'<div style="font-size:0.7rem;color:#6b7280">{name}</div>'
                f'</div>'
                f'<div class="tracker-pos-badges">{badge_html}</div>'
                f'<span class="tracker-score">{total:.0f}</span>'
                f'</a>'
            )
        lines2.append('</div>')
        return '\n'.join(lines2)

    w(render_col([(r, t) for r, t in enumerate(ranked_teams[:16], 1)]))
    w(render_col([(r, t) for r, t in enumerate(ranked_teams[16:], 17)]))

    w('</div>')
    w('')

    # Summary table
    w('## Full Rankings')
    w('')
    w('| Rk | Team | C | LW | RW | D | Total | C# | LW# | RW# | D# |')
    w('|----|------|---|-----|-----|---|-------|-----|------|------|-----|')
    for rank, team in enumerate(ranked_teams, 1):
        s    = strength[team]
        name = TEAM_NAMES.get(team, team)
        c_rk  = pos_rank_label('C',  team, strength, all_teams)
        lw_rk = pos_rank_label('LW', team, strength, all_teams)
        rw_rk = pos_rank_label('RW', team, strength, all_teams)
        d_rk  = pos_rank_label('D',  team, strength, all_teams)
        w(f'| {rank} | [{team}](teams/{team}.md) {name} '
          f'| {s["pos"]["C"]["pts"]:.0f} '
          f'| {s["pos"]["LW"]["pts"]:.0f} '
          f'| {s["pos"]["RW"]["pts"]:.0f} '
          f'| {s["pos"]["D"]["pts"]:.0f} '
          f'| **{s["total"]:.0f}** '
          f'| #{c_rk} | #{lw_rk} | #{rw_rk} | #{d_rk} |')

    return '\n'.join(lines)

# ─────────────────────────────────────────────────────────────────────────────
# MKDOCS NAV UPDATER
# ─────────────────────────────────────────────────────────────────────────────

def update_nav(ranked_teams):
    yml_path = os.path.join(os.path.dirname(__file__), 'mkdocs.yml')
    with open(yml_path, encoding='utf-8') as f:
        content = f.read()

    # Strip existing nav block after "plugins:"
    nav_marker = '\nnav:'
    if nav_marker in content:
        content = content[:content.index(nav_marker)]

    nav_lines = ['\nnav:', '  - Tracker: index.md', '  - Teams:']
    for team in ranked_teams:
        name = TEAM_NAMES.get(team, team)
        nav_lines.append(f'    - {team} — {name}: teams/{team}.md')

    with open(yml_path, 'w', encoding='utf-8') as f:
        f.write(content + '\n'.join(nav_lines) + '\n')

# ─────────────────────────────────────────────────────────────────────────────
# MAIN
# ─────────────────────────────────────────────────────────────────────────────

def main():
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    print(f'Loading GP data from {GP_PATH}...')
    gp_lookup = load_gp_lookup(GP_PATH)
    print(f'  {len(gp_lookup)} players in GP lookup')

    print(f'Loading {CSV_PATH}...')
    skaters, goalies = load_all(CSV_PATH, gp_lookup)
    print(f'  {len(skaters)} skaters, {len(goalies)} goalies')
    print(f'  Projecting all fpts to {FULL_SEASON}-game pace (min {MIN_GP} GP)')

    print('Assigning positions...')
    assign_positions(skaters)

    charts    = build_charts(skaters, goalies)
    all_teams = sorted(charts.keys())

    print('Computing cross-team metrics...')
    compute_metrics(skaters, charts, all_teams)

    strength     = team_strength(charts, all_teams)
    ranked_teams = sorted(all_teams, key=lambda t: strength[t]['total'], reverse=True)

    os.makedirs(TEAMS_DIR, exist_ok=True)

    print('Generating team pages...')
    for rank, team in enumerate(ranked_teams, 1):
        page = gen_team_page(team, charts, strength, all_teams, rank)
        path = os.path.join(TEAMS_DIR, f'{team}.md')
        with open(path, 'w', encoding='utf-8') as f:
            f.write(page)
        print(f'  [{rank:>2}] {team} — {TEAM_NAMES.get(team, team)}')

    print('Generating index...')
    index = gen_index(ranked_teams, strength, charts, all_teams)
    with open(os.path.join(DOCS_DIR, 'index.md'), 'w', encoding='utf-8') as f:
        f.write(index)

    print('Updating mkdocs.yml nav...')
    update_nav(ranked_teams)

    print('\nDone! Run:')
    print('  cd C:\\src\\NHL\\fantasy-tracker')
    print('  mkdocs serve')

if __name__ == '__main__':
    main()
