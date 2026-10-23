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
