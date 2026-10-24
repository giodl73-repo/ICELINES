# Native repository storage

`icelines-core` provides the optional `boxed-stats` Cargo feature. It stores
each `StatsRepository` season row in a `Box`, reducing the size of unused
hash-table buckets while adding one allocation and one indirection per row.
Native builds retain inline storage by default. WASM already uses boxed rows;
enabling this feature adds no WASM memory saving.

For a direct dependency:

```toml
icelines-core = { path = "../icelines-core", features = ["boxed-stats"] }
```

To exercise the native engine from this workspace:

```powershell
cargo test --locked -p icelines-wasm --lib --features icelines-core/boxed-stats
```

Cargo features are additive: this selects boxed storage for all users of the
same core package in the build graph. It is a build-time choice, not a
per-repository switch. It does not change `send-sync`, public borrowed row
types, serialized data, or eviction policy.

## Evidence and adoption limits

[MISER EXP037](https://github.com/giodl73-repo/MISER/blob/9866e3e4805386ef3a0218cd72ba8f4bf2124c2e/evidence/EXP-037.md)
compared the two representations in native Windows `BrowserEngine` processes
using 75 catalog packages and eight resident season repositories. Requested
retained bytes fell 21.34% after the first eight packages and 10.58% after all
75 packages had been processed. Construction peaks also fell. These are
allocator-requested bytes owned by the engine, not process RSS or whole-browser
memory. The measured prototype used the same boxed representation with a
host-specific selector; the measurements were not rerun against this feature.

This is an opt-in adoption candidate. Extra allocations are a real cost.
Query latency, allocator overhead, and CLI/server repository occupancy and
lifetimes need workload-specific evaluation before enabling it by default.
The existing browser memory target and deferred Android testing remain open.

[MISER EXP038](https://github.com/giodl73-repo/MISER/blob/0bf41dff256af2ea07c3e7e42fdd70d88af936f5/evidence/EXP-038.md)
subsequently measured this feature at commit
`4d5764244dd7a2239a2496961397e715cf10a50c` with production native loading and
career fan-out. One fixed twelve-process Windows release ABBA pass preserved
complete stored-data/query/roster parity. Requested retained bytes fell 27.88%
(active season), 24.99% (active plus three careers), and 37.27% (twenty-skater
career cohort), with 1,031/1,120/1,516 extra construction allocation calls.

Process private-byte deltas favored boxing in both pairs; one difference was
only 8 KiB and RSS directions were mixed. No query exceeded the >10% regression
screen in both pairs, but 18 of 42 series exceeded it in one pair. The strict
all-pairs latency screen is inconclusive, so this remains opt-in and the native
default stays inline. These are shared native query measurements, not HTTP,
whole-browser or Android acceptance. All raw outcomes and limits are retained
in the linked evidence; no repeat campaign follows this result.
