# Handling records — 2026-11-24

Handling now exposes recorded giveaways and penalty drawing separately. The front
attribute field shows both rates beneath the existing grade; selecting Handling
opens the two signals, six supporting rows and a readable formula. Native Askama
and browser rendering consume the same Rust-produced `handling-record.v1` data.
The 1–20 grade, feats, documentary class and simulator values are unchanged.

## What the measurements mean

The baseline Handling estimate combines lower giveaway-share rank and higher
penalties-drawn/60 rank at equal weights. Its share is:

`giveaways / (shots on goal + assists + giveaways)`

The new view explicitly calls this a **giveaway share proxy**. These are recorded
actions, not touches or possession opportunities. The share is not a retention
percentage or a per-touch failure probability. Giveaways/60 is also affected by
puck use and deployment, so a high rate alone does not establish poor handling.

A synthetic check demonstrates the denominator issue: two records each have
1 giveaway/60 and 0.8 penalties drawn/60. With 2 shots and 1 assist/60, the share
is 25.0%; with 9 shots and 4 assists/60 it is 7.1%. The observed giveaway rate and
penalty drawing are identical. This is an arithmetic example, not NHL player data
or a claim that either player is the better handler.

The detail also shows penalties taken and drawn-minus-taken per60. These count
events rather than penalty minutes, actual power plays or their expected value.
Coincidental penalties can cancel numerical advantages; see Rule 19.1 in the
[rulebook linked by the NHL](https://media.d3.nhle.com/image/private/t_document/prd/i9yumvyaojps5kixzaxz.pdf).
The current video-rulebook page labels its download 2026–27, but this linked
PDF's cover artwork carries 2024–25; no independent edition claim is made here.

Rates use the selected NHL all-strength time window. Displayed values are rounded;
the net and share use unrounded inputs. True zero events remain zero. A zero
share denominator stays unavailable, as do missing inputs or absent exposure.
Raw observations have no new GP cutoff; existing small-sample/grade policies remain.

## Current preview and refresh

All 787 current skaters have both raw rates in the retained NHL snapshot fetched
2026-11-23. Examples for 2026–27, per60:

| Player | Giveaways | Penalties drawn | Penalties taken |
| --- | ---: | ---: | ---: |
| McDavid | 3.70 | 1.36 | 0.39 |
| Fox | 2.85 | 0.34 | 0.68 |
| MacKinnon | 5.49 | 1.63 | 0.45 |
| Beniers | 3.18 | 0.33 | 0.00 |

These describe different recorded-event patterns, not isolated puck-control
capability. There is no new calibrated scoring recipe or achievement award.

The shared `attach_records` path now constructs both defensive and Handling
records during normal refresh and defensive-source enrichment. A new offline CLI
can rebuild these descriptive views from existing admitted measurements:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/defending-card-study/enriched/rinkcraft-stat-attributes.json --output-dir target/handling-card-study/enriched
```

Use a new output directory for replay. The command preserves other seasons,
creates no new source observations and records its input/output hashes. The
record is bound to player, season, group, GP and all-strength minutes; legacy
catalogs lacking the optional field retain the existing UI fallback.

- Input SHA-256: `d84ab01a4ecc26005f503e1d95311796f3a722c97ba08b087467525de6972eb4`
- Output SHA-256: `f551c58eeddb6ca6960e3629d5b61d8c1c749d8e2ab85267f9df0be24066eba3`

The catalog and report replay byte-identically. All 3,948 supplied six-integer-
grade arrays remain identical. Complete preview profiles are merged with the
actual WASM attachment result; unrelated profile fields and defensive records
are retained. Record text agrees exactly, with raw exposure compared within 1e-9
across JSON runtimes.

Preview: `http://127.0.0.1:8780/?player=8478402`. Select Handling to inspect the
record. This is a local generated preview; no hosted release is claimed.

## Completed checks

- Eleven focused Rust tests pass: Handling and defensive records, defensive
  source enrichment, and refresh/cache replay. New checks cover the controlled
  denominator example, zero versus missing inputs, zero opportunity, unchanged
  grades, selected-season isolation and actor/window binding.
- Locked native CLI build passes, including both Askama views. Rust 1.95.0 WASM
  and matching wasm-bindgen 0.2.118 bindings pass.
- Full browser build and all 245 browser unit tests pass.
- All 787 complete profiles receive their records through actual WASM attachment.
  No new Handling measurement is missing in this supplied snapshot.
- Browser acceptance passes McDavid, Fox, Beniers and Killorn: summary values,
  six-row detail/formula, unchanged grades, retained defensive records, actual
  worker attachment, older-record fallback and 390px page overflow.
- Four PDFs independently contain both Handling summaries on one 612×792-point
  Letter page each. The compact front screenshot was inspected.
- Import replay reproduces both catalog and report bytes exactly.
