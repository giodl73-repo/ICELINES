# 2026-27 preseason division forecast

IceCast's frozen production baseline places the New York Rangers third in the
Metropolitan Division by average points. The Rangers project to 94.76 points,
with a 49.7% playoff probability and an 84-106 point P10-P90 interval. New
Jersey is less than one expected point behind, so third place is not a strong
separation.

## Atlantic

| Rank | Team | Average points | P10-P90 | Playoffs | Stanley Cup |
|---:|---|---:|---:|---:|---:|
| 1 | TBL | 101.58 | 91-112 | 76.6% | 8.43% |
| 2 | MTL | 100.63 | 90-111 | 73.6% | 7.00% |
| 3 | BUF | 97.70 | 86-109 | 60.7% | 4.23% |
| 4 | OTT | 95.92 | 85-107 | 52.6% | 3.10% |
| 5 | BOS | 95.19 | 84-106 | 48.9% | 2.46% |
| 6 | FLA | 95.00 | 84-106 | 48.2% | 2.95% |
| 7 | TOR | 93.16 | 82-104 | 39.5% | 1.83% |
| 8 | DET | 89.94 | 79-101 | 25.8% | 0.70% |

## Metropolitan

| Rank | Team | Average points | P10-P90 | Playoffs | Stanley Cup |
|---:|---|---:|---:|---:|---:|
| 1 | WSH | 101.00 | 90-112 | 76.6% | 8.32% |
| 2 | CAR | 97.33 | 86-108 | 61.8% | 4.11% |
| 3 | NYR | 94.76 | 84-106 | 49.7% | 2.52% |
| 4 | NJD | 93.80 | 83-105 | 45.0% | 2.24% |
| 5 | PHI | 93.36 | 82-105 | 42.3% | 1.88% |
| 6 | PIT | 92.68 | 82-104 | 39.6% | 1.60% |
| 7 | CBJ | 92.41 | 81-103 | 38.6% | 1.53% |
| 8 | NYI | 87.81 | 77-99 | 20.4% | 0.35% |

## Central

| Rank | Team | Average points | P10-P90 | Playoffs | Stanley Cup |
|---:|---|---:|---:|---:|---:|
| 1 | COL | 101.42 | 91-112 | 85.1% | 9.89% |
| 2 | MIN | 99.61 | 88-111 | 79.0% | 6.67% |
| 3 | DAL | 98.61 | 88-109 | 76.3% | 6.08% |
| 4 | UTA | 95.59 | 85-107 | 62.9% | 3.55% |
| 5 | STL | 90.44 | 79-101 | 39.6% | 0.98% |
| 6 | NSH | 90.36 | 79-101 | 39.0% | 1.14% |
| 7 | WPG | 89.11 | 78-100 | 33.2% | 0.77% |
| 8 | CHI | 87.89 | 77-99 | 28.9% | 0.65% |

## Pacific

| Rank | Team | Average points | P10-P90 | Playoffs | Stanley Cup |
|---:|---|---:|---:|---:|---:|
| 1 | EDM | 98.80 | 88-110 | 80.4% | 6.69% |
| 2 | LAK | 95.20 | 84-106 | 66.1% | 3.80% |
| 3 | VGK | 92.79 | 82-104 | 54.5% | 2.31% |
| 4 | SJS | 92.11 | 81-103 | 51.3% | 1.95% |
| 5 | SEA | 89.54 | 78-101 | 39.7% | 1.16% |
| 6 | ANA | 88.57 | 77-100 | 35.6% | 0.84% |
| 7 | CGY | 83.40 | 72-94 | 16.7% | 0.15% |
| 8 | VAN | 81.43 | 70-93 | 11.8% | 0.12% |

## Method and authority

- Generated from Icelines commit `659cbd59` with `icecast season`, season
  `20262027`, stats season `20252026`, 10,000 trials, and seed `20262027`.
- The run contains 1,344 scheduled games and 84 games per team. It selected the
  authoritative July 29 official-roster snapshot, verified all 32 teams,
  enabled player-value effects, and emitted no warnings.
- Full-league simulation fingerprint:
  `1afdc479e23aaa46f7bfdb4e84f600c1c479a5e42b9291118d9b0aa63bd6c8a0`.
- Rankings sort teams by average simulated points. They are summaries of the
  distribution, not direct probabilities of each exact division finish.
- These model estimates are not betting odds or guarantees. The baseline uses
  frozen roster/depth strength, home ice, rest, congestion, travel, and
  timezone context. It does not assume unmodeled trades or personnel events.
- The stronger game-prediction challenger remains evaluation-only until the
  prospectively registered 2026-27 holdout can be scored after April 11, 2027.
