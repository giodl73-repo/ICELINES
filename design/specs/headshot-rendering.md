# Headshot Rendering — Specification (Reference)

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented (reference doc)

---

## Purpose

Render player face photos as **ASCII art** inside the terminal — the
small portrait shown on the TUI player card. Uses Unicode braille
characters for 2×4-pixel resolution per cell, fetched live from the
NHL CDN, cached in-process for the session, and persisted as rendered
braille rows under the user cache directory.

This is a reference doc for the existing implementation in
`icelines-cli/src/tui/headshot.rs`. Behavior is intentionally
unchanged in v1; this spec captures the algorithm so future
contributors can reason about it.

---

## URL derivation

Headshots live on the NHL CDN at:

```
https://assets.nhle.com/mugs/nhl/{SEASON}/{TEAM}/{NHL_ID}.png
```

- `SEASON` — 8-digit season ID (e.g. `20252026`)
- `TEAM` — three-letter abbrev (`EDM`, `SEA`, ...)
- `NHL_ID` — the player's `nhl_id` integer

URL is built at the screen-render call site (in `screens/player.rs`)
from the `Player` struct, which already has all three fields. No
roster fetch is required — the URL pattern is stable.

If a player changes teams mid-season, both old and new URLs may
exist on the CDN; the season's roster fetch determines which is
canonical. For traded players, the photo may briefly show the old
team's jersey until the CDN updates.

---

## Cache

```rust
pub struct HeadshotCache {
    inner: Arc<Mutex<HashMap<u32, Vec<String>>>>,  // nhl_id → rows of braille
}
```

- Keyed by `nhl_id` (not by URL).
- Value is `Vec<String>` — one string per terminal row, already
  rendered as braille characters.
- `Arc<Mutex<HashMap>>` is locked briefly for each get / insert.
  Contention is negligible: cache hits are ~microseconds and a
  fetch only writes once per player. A lock-free map (e.g. DashMap)
  was considered but rejected — the simpler dependency-free path
  wins for this volume.

`get(id)` returns `Some(rows)` if cached, `None` if missing or in flight.
`set(id, rows)` writes the rendered output.

Two sentinel row patterns let the renderer show progress:
- `is_loading(rows)` — true if the rows match a "loading…" placeholder
- `is_error(rows)` — true if the rows match an "error" placeholder

The in-memory cache is session-scoped. Successful fetches also write the
rendered rows to `~/.icelines/cache/headshots/{nhl_id}.txt`; a fresh TUI launch
checks that disk cache before making an NHL CDN request. Disk reads are
best-effort: missing, empty, or unreadable files fall through to the network
path.

---

## Fetch + dither pipeline

`spawn_fetch(nhl_id, url, cache, target_cols, target_rows)`:

1. **Insert "loading" placeholder** into the cache so the renderer can
   show a spinner / skeleton.
2. **Spawn a tokio task**:
   - GET the URL via reqwest (default 30s timeout).
   - Decode the PNG → RGB image.
   - Convert to grayscale.
   - Resize to `(target_cols * 2)` × `(target_rows * 4)` pixels —
     because each braille char is 2 wide × 4 tall.
   - **Stretch the histogram** (linear contrast remap from the actual
     [min,max] range of pixels to the full [0,255]) so faint photos
     reach both extremes.
   - **Threshold dither** at the constant `THRESHOLD = 128` —
     pixels above the threshold are "on", below are "off". One-pass,
     no error diffusion. (Floyd-Steinberg was prototyped; the
     contrast-stretched threshold gave better recognition on small
     terminal grids.)
   - Walk the 1-bit image in 2×4 blocks; for each block, set the
     corresponding 8 dot bits in U+2800–U+28FF and emit the resulting
     character.
   - Collect rows of characters into `Vec<String>`.
3. **Insert the result** into the cache (or an error placeholder on
   any failure).

Total time: ~200–500 ms on a typical home connection. The TUI keeps
rendering at 10 fps with the loading placeholder until the rows land.

---

## Braille dot layout

Unicode braille codepoints have eight dots arranged 2×4:

| Row | Left dot | Right dot |
|---|---:|---:|
| 1 | 1 (`0x01`) | 4 (`0x08`) |
| 2 | 2 (`0x02`) | 5 (`0x10`) |
| 3 | 3 (`0x04`) | 6 (`0x20`) |
| 4 | 7 (`0x40`) | 8 (`0x80`) |

A rendered character is `U+2800 + Σ(bit for each set dot)`. All
ranges from `U+2800` (blank) to `U+28FF` (all eight dots) are valid.

---

## Why braille over half-blocks

The TUI uses **braille** for player headshots and **half-blocks** for
team logos. Trade-off:

| Mode | Resolution per cell | Best for |
|------|---------------------|----------|
| Half-block (▀) | 1 × 2 pixels | High-contrast graphics (logos) |
| Braille | 2 × 4 pixels | Photographic detail (faces) |

A 24-row × 18-col face image is 48 × 72 effective pixels in braille —
enough resolution to recognize a player's face. Half-blocks at the
same character grid would be 18 × 48, often too low to be useful.

Braille requires a font with full Unicode block coverage — Cascadia
Code, JetBrains Mono, and most modern monospace fonts support it.
Older fonts (terminal default Consolas pre-2020) may render boxes;
the TUI does not detect this in v1.

---

## Render integration

The player card (`screens/player.rs`) reserves a 24-row × 18-col
window in the upper-left for the headshot. On screen open:

1. Check `headshot_cache.get(player.nhl_id)`.
2. If `None`: insert loading placeholder, spawn fetch.
3. If `Some(rows)` and not error: render rows into the reserved area.
4. If `Some(rows)` and error placeholder: render a generic silhouette.

The fetch task communicates only via the cache — no channels,
mpscs, or callbacks. The TUI's per-frame redraw picks up the new
state automatically.

---

## Decisions (Open Questions resolved)

1. **Persist rendered rows, not source photos**: Source photos are 50–80 KB each,
   but dithered rows are about 1 KB per player. Persisting the rendered text
   avoids repeated CDN requests without creating a large photo cache.

2. **No fallback URL**: If the CDN fetch fails (404 for traded
   player, network error), show the error placeholder. Don't try
   alternate URLs. Simpler.

3. **PNG-only**: NHL CDN serves PNG; no JPG / WebP handling needed.

4. **Histogram-stretch + threshold dither**, not Floyd-Steinberg:
   Tested both; on small (24-row) face crops the stretched
   threshold preserves more facial structure (eyes / nose / chin
   stay distinct) than the noisier error-diffusion output. One
   pass; cheap.

5. **No color**: 1-bit dithered grayscale. Color braille is
   possible (foreground RGB per cell) but doubles the byte cost per
   frame for marginal recognition improvement. Punt.

6. **Reuses proof's dither algorithm**: The braille walker mirrors
   `proof::ascii::braille` for consistency with future
   DASHBOARD-SPEC integration.

---

## Test coverage

The pure rendering and cache helpers have L0 coverage in
`icelines-cli/src/tui/headshot.rs::tests`.

Covered:
- `l0_braille_dot_bit_layout_matches_unicode` — every dot maps to the expected
  Unicode braille bit.
- `l0_threshold_dither_solid_black_sets_all_dots` and
  `l0_threshold_dither_solid_white_clears_all_dots` — all-on/all-off contracts.
- `l0_threshold_constant_is_midpoint` — threshold stays at the 8-bit midpoint.
- `l0_cache_get_set_roundtrip` and `l0_cache_clone_shares_storage` — in-memory
  cache behavior.
- `l0_is_loading_detects_placeholder` and `l0_is_error_detects_placeholder` —
  sentinel detection.
- `l0_disk_write_and_read_roundtrip`,
  `l0_disk_write_creates_parent_directory_on_first_use`,
  `l0_disk_read_returns_none_when_file_missing`, and
  `l0_disk_read_treats_empty_file_as_miss` — disk-cache behavior in an isolated
  temp home.

Live NHL CDN requests remain intentionally untested in CI.

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| Color braille | LOW | Marginal recognition gain |
| Team logo rendering tweaks | LOW | Half-block already good |
| Local SVG → braille for team logos | LOW | Already works via PNG |
| Configurable target dimensions | LOW | 24×18 is the right size |
| Animated transition on cache miss | LOW | Cosmetic |
