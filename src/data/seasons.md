# IceLines Season Coverage

IceLines covers 38 NHL seasons spanning 1987-88 through 2025-26.

<!-- proof:figure id="seasons-bundled" kind="table.reference" -->
## Bundled in binary (zero config required)

| Season | ID | Notes |
|--------|----|-------|
| 2025-26 | 20252026 | Current season — refreshed weekly |
| 2024-25 | 20242025 | |
| 2023-24 | 20232024 | |
| 2022-23 | 20222023 | |
| 2021-22 | 20212022 | |

These 5 seasons are compiled directly into the `icelines` binary.
`icelines query leaders` works immediately after install with no network access.

<!-- proof:figure id="seasons-installable" kind="table.reference" -->
## Installable via `icelines data install`

**Salary-cap era (2005-06 → 2020-21)**

| Seasons | Era |
|---------|-----|
| 20202021, 20192020, 20182019 | COVID/bubble seasons |
| 20172018, 20162017, 20152016 | Matthews/Laine/Marner draft class |
| 20142015, 20132014, 20122013 | McDavid pre-draft years (2013 lockout-shortened) |
| 20112012, 20102011, 20092010 | Crosby/Ovechkin peak |
| 20082009, 20072008, 20062007 | Stamkos draft, Crosby Cup |
| 20052006 | Ovechkin + Crosby rookie year |

**Pre-cap era (2000-01 → 2003-04)**

| Seasons | Era |
|---------|-----|
| 20032004, 20022003, 20012002, 20002001 | Final pre-lockout seasons |

**Gretzky-trade era (1987-88 → 1999-2000)**

| Seasons | Era |
|---------|-----|
| 19992000, 19981999, 19971998 | Lemieux returns; Gretzky retires (1999) |
| 19961997, 19951996 | Post-lockout return |
| 19941995 | 48-game lockout season |
| 19931994, 19921993, 19911992 | Gretzky final LA years + Quebec/Nordiques |
| 19901991, 19891990 | Lemieux back-to-back scoring titles |
| 19881989, 19871988 | Gretzky to LA Kings (1988 trade) |

Note: 20042005 omitted — full lockout, zero games played.

<!-- proof:figure id="seasons-install-commands" kind="table.reference" -->
## Install commands

| Command | Effect |
|---------|--------|
| `icelines data install --seasons 1` | Refresh current season |
| `icelines data install --seasons 5` | Current + 4 prior seasons |
| `icelines data install --seasons 25` | All salary-cap era seasons |
| `icelines data install --seasons 38` | Complete 1987–2025 history |
| `icelines data install --season 19881989` | Specific season (Gretzky's first LA year) |
| `icelines data list` | Show installed seasons |
| `icelines data remove 20002001` | Uninstall a season |
