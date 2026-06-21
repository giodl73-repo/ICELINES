# Phase Mammoth Compare Inventory

## Purpose

Inventory Compare route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Mammoth Compare posture |
|---|---|---|
| Compare HTML | `GET /compare` | Keep read-only `CompareView`, similarity mode through `SimilarPlayersView`, and career trend SVG when both cards have enough loaded career rows. |
| Compare JSON | `GET /api/v1/compare` | Keep stable data/meta envelopes, selected-card row identity, similarity rows, and shared bad-input error envelopes. |

## Risks to Avoid

- Claiming career data creation from read routes.
- Pulling scoring, streak, records, or fantasy route claims into compare rows.
- Claiming new comparison modes.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused compare tests cover envelopes, row
   identity, similarity rows, career SVG, and bad-input boundaries.
3. Matrix wording. Result: passed; compare rows now carry scoped wording.
4. Closeout. Result: passed; Phase Mammoth Compare is closed with final
   route-row claims and non-claims recorded.
