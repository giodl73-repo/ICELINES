# Ferris Artifact Shadow

## Status

Native measured three-platform evidence accepted - 2026-08-30

## Frame

ICELINES already owns a working three-platform package matrix, archive format,
checksum sidecars, embedded package manifest, and release smoke command. Ferris
does not replace those mechanisms.

The missing shared capability is an independently recorded compatibility
decision between an owner-produced package and its required integrity and smoke
consumers. The smallest falsifiable thesis is:

> An exact Ferris compatibility envelope can reject a changed artifact,
> platform, target, toolchain, configuration, manifest, or build command before
> ICELINES runs its existing downloaded-package verifier.

The deletion target is a repeated consumer rebuild performed only because the
producer package lacks reusable compatibility evidence. This slice is disproved
if incompatible bytes or metadata pass, if a missing consumer appears complete,
or if Ferris gains publication authority.

## Audit and comparison

The owner flow in `.github/workflows/ci.yml` builds `icelines-cli` for Linux,
macOS, and Windows, writes `ICELINES-PACKAGE.txt`, verifies the archive and
checksum, and uploads a one-day package artifact. The tagged release workflow
already downloads equivalent owner artifacts before publication.

Ferris first supplied an observation-only `artifacts --request` contract. The
ICELINES shadow proved that contract, then motivated Ferris commit
`73ee870fd7e7689637a93bfb835fcbf8d1ccda4e`, which adds bounded local artifact
and manifest measurement plus opt-in fail-closed qualification. Ferris still
does not store, transfer, execute, sign, or publish artifacts. The revised
shadow therefore composes rather than duplicates the two systems:

1. ICELINES builds, packages, checksums, and uploads.
2. A separate same-platform job downloads the package.
3. `scripts/ferris-artifact.py` records owner-specific toolchain,
   configuration, manifest, and command identities and asks Ferris to measure
   the downloaded archive and manifest sidecar, bind them to the producer, and
   require compatible two-consumer fan-in.
4. ICELINES verifies the checksum, byte identity between the external and
   archived manifests, archive members, source revision, binary hash, and
   packaged binary version, then executes `--version`.

## Role review

| Lens | Finding | Disposition |
| --- | --- | --- |
| ICELINES `KEEL` | The slice reuses the canonical package and release-verification chain and changes no product surface or data path. | `pass` |
| ICELINES `BENCH` | Local positive proof covers native measured qualification, two-consumer fan-in, manifest/source binding, binary hash, and binary smoke. Both structural and native measured modes passed hosted evidence on all three platforms. | `pass` |
| ICELINES `EDGE` | Artifact-byte tampering and platform/target mismatch fail closed. Missing archive, sidecar, manifest, consumer, or executable also blocks the job. | `pass` |
| Ferris `native-platform-adopter` | The same owner contract is projected on Linux, macOS, and Windows without changing ICELINES build semantics. | `pass-with-condition` |
| Ferris `ai-assurance-skeptic` | The report is structural compatibility evidence, not authenticated provenance or execution authenticity. | `pass` |
| Ferris `scope-keeper` | The workflow has no tag, release, write permission, secret, NHL API fetch, data mutation, or deployment step. | `pass` |
| Ferris `product-value-governor` | This closes named macOS and real artifact-handoff gaps rather than repeating PARLOR package selection. | `authorize` |

The first hosted run exposed a Windows integration defect before Ferris could
execute: a full nested checkout of the pinned Ferris revision exceeded Git for
Windows' default path limit on deeply named simulation evidence. The adopter
now sparsely checks out only `Cargo.toml`, `Cargo.lock`, and `crates/`, which are
the inputs required to build Ferris. This is a bounded consumer workaround;
Ferris still carries repository-layout debt for ordinary full Windows
checkouts.

The corrected run `33313971568` passed all owner and Ferris jobs:

| Platform | Package job | Consumer job | Ferris report |
| --- | ---: | ---: | --- |
| Linux x86_64 | 8m27s | 54s | `sha256:8e23c614c15b311baff0e99862e101408ca50bd0d7d71c994053e67760cad100` |
| macOS x86_64 | 9m41s | 1m02s | `sha256:3be86677e019180a7e6b70b718b0f3c76f6f254d398a28624ec7d8b74bb31a0c` |
| Windows x86_64 | 19m22s | 1m38s | `sha256:9c81e762ce79c547ed550acd32f71464f1a1ada2fe679c7ec267c7f39a18b10d` |

Each retained report has two compatible consumers and successful expected
fan-in. The owner verifier independently accepted the downloaded checksum,
archive, source revision, binary hash, and packaged binary smoke command.
Windows remains the critical path: the current all-package `needs` edge delays
the Linux and macOS consumer jobs until the 19-minute Windows package completes.

The native measured adapter passed locally against the real 17,003,262-byte
Windows package with qualification
`sha256:1b91b9e59b437784789f5ca597ea172a3971eb6c4994ad3911e7bc050460b127`.
A tampered package retained rejected qualification
`sha256:dd3c9e1016408ba4a4a6ef4dee4ca7f69b3adf065c444c3332ba939b5c4500e1`,
with only `artifact_digest_matches` false. Hosted confirmation of this native
mode was the final condition.

Run `33316361372` passed every owner job and all three native Ferris consumers:

| Platform | Package job | Consumer job | Qualification | Bytes |
| --- | ---: | ---: | --- | ---: |
| Linux x86_64 | 8m33s | 48s | `sha256:021289282ab5a352682c6fa7880f5dba066e6185303afafc62e15a765039dd82` | 31,985,400 |
| macOS x86_64 | 8m48s | 1m37s | `sha256:de78db2c61c2ab5dff67739989b7c53be394c10be40c4ba449b10d8c8fc6e4ec` | 30,833,245 |
| Windows x86_64 | 15m42s | 2m13s | `sha256:6e95248ced301473ac5be4f609488069a07671c2ad43b4806626daed59f4a753` | 28,616,879 |

Every report uses `ferris.artifact-qualification-report/v1`, has status
`qualified`, matches both producer identities, and completes the exact integrity
and smoke consumer set. The external manifest sidecar is also byte-compared
with the copy extracted from the archive by the owner verifier.

## Slice and stop conditions

The hosted slice succeeds only when all three platform packages are downloaded
by separate jobs, both required consumers are structurally compatible, and the
owner verifier accepts and executes each package. Evidence artifacts retain the
exact request and Ferris report for one day.

Stop and repair rather than weaken the contract if:

- producer and consumer toolchain identities differ;
- the downloaded digest differs from the producer envelope;
- any compatibility dimension differs;
- either required consumer is absent;
- the owner checksum, manifest, source, binary hash, or smoke check fails; or
- the workflow would require publication or data-write permission.

## Non-claims

- The GitHub artifact service and workflow identity are not authenticated by
  Ferris.
- Structural compatibility does not prove provenance, signing, or release
  eligibility.
- This does not establish cache savings or authorize reuse across platforms.
- This does not publish a package, create a release, fetch NHL data, or alter
  branch policy.
