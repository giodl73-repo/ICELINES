# Archive boundary verification — 2026-10-06

REQ-BROWSER-001 / WP-BW-03 / WP-BW-05. Local desktop Chromium 154 exercised the
production worker from the retained, verified merged PR #75 artifact. This is
archive rejection and previous-good preservation evidence, not full resource or
release acceptance.

## Reproduction

Run the preparation script with a retained artifact's exact commit and shell hash:

```powershell
python scripts/prepare-browser-archive-stress-preview.py target/browser-master-4ae78a8c 4ae78a8c340bd3eda676633b8775f0cf18078a28 4f7f1e6de9be3772720538c7464af640dc27730dbd060c360b1bad04c7a0568b target/new-archive-stress-preview
python -m http.server 8882 --bind 127.0.0.1 --directory target/new-archive-stress-preview
```

Open `/stress.html` and run the check. Use a fresh output and browser origin, or
serve no-store headers, when switching artifacts: ordinary local HTTP caches can
retain old module/build-info bytes. The preparation step verifies clean source,
shell, worker, WASM and all 75 packages before copying the artifact. The browser
report verifies build-info against that identity. The fixture is local only;
never publish its synthetic oversized files or modified shell.

## Results

[Actual report](../evidence/archive-stress-20261006.json) records the clean source
`4ae78a8c340bd3eda676633b8775f0cf18078a28` and shell
`4f7f1e6de9be3772720538c7464af640dc27730dbd060c360b1bad04c7a0568b`.
The test first loads the 2023–24 regular package and checks its nine 100-point
leaders. After each rejected import, the entire query DTO must remain equal.

| Input | Rejection | Worker round trip |
|---|---|---|
| 102,993-byte gzip expanding past 100 MiB | Expanded archive exceeds 100 MiB | 635.3 ms |
| Tar header declaring a 33 MiB allowed file | Expanded archive file exceeds limits | 1.1 ms |
| 25 MiB plus one byte compressed input | Compressed archive exceeds 25 MiB | 9.1 ms |

All three cases retained the previous nine-row query. Timings include worker
transport, decompression and rejection; they are single desktop observations,
not latency percentiles. The expanded fixture consists of compressed zero blocks
and tests decompressed-stream accounting beyond archive end markers. No input is
extracted to disk. No live calls or saves occur.

## Remaining acceptance

This does not measure total browser peak memory, physical-phone resources, valid
near-limit normalization, actual file-picker handling, or completed downloads.
Earlier pulse 31 proves one real release regular/playoff picker import and
conversion costs. Production export buttons were invoked on Pages, but the only
connected browser did not emit a download event within an eight-second capture;
there is no verified downloaded export artifact from that attempt.

Deployed offline reopening and remote retained-byte rollback remain unverified.
The CI run for PR #79 is separate evidence for its newer acquisition guards;
these stress results must not be attributed to an unpublished browser build.