# Pulse 01 - Finish Inventory and Proxy Contract

## Goal

Define the next layer of Rocket Richard player-scoring intelligence before
adding code: trend rows, streak leaderboards, and an IceLines-owned
shot-quality proxy based only on official NHL play-by-play coordinates and
existing local cache data.

## Governing roles

- **scout**: finishing context must explain player performance without implying
  lineup certainty, betting value, or proprietary expected-goals equivalence.
- **edge**: coordinates, shooter IDs, dates, and game-line cache coverage are
  incomplete in real data; the contract must model that explicitly.
- **bench**: every threshold and streak definition must have known-value tests.
- **wire**: all report routes read cache only; cache warming remains fetch/Admin
  mutation paths.

## Owned scope

1. Inspect current scoring ViewModels, streak ViewModels, and game-cache
   providers.
2. Write a short inventory artifact for available trend/streak/proxy inputs.
3. Define the first shot-quality proxy contract and its non-goals.
4. Amend this wave's later pulse list if inventory reveals a better split.

## Candidate deliverable

`FINISH-INVENTORY.md` in this wave folder, covering:

- available player scoring-event fields;
- available game-line/streak fields;
- coordinate coverage and null semantics;
- proposed proxy buckets or distance bands;
- tests required before implementation.

## Role review notes

- **scout**: use descriptive terms such as "volume", "conversion", "inside
  chances", and "finishing run"; do not use "expected goal" unless the owned
  model is explicitly defined and named as IceLines-only.
- **edge**: proxy rows must carry enough source state to distinguish "no inside
  chances" from "coordinates missing"; player ID matching must tolerate
  unresolved shooter/scorer IDs.
- **bench**: before implementation, choose fixture events with manually computed
  distances/buckets and streak lengths so tests assert known values, not
  captured output.
- **wire**: inventory should preserve the current cache boundary:
  `DataKind::PlayByPlay` and existing game-line caches are read sources; any
  cache warming remains fetch/Admin mutation flow.

## Gates

- [ ] `cargo fmt --check`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-measure-the-finish design\waves\PHASES.md --errors-only`
