# Ferris Artifact Shadow

## Status

Ready for hosted evidence - 2026-08-30

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

Ferris `artifacts --request` is intentionally observation-only. It compares an
owner-declared producer with required consumers and reports compatibility and
fan-in; it does not hash, store, transfer, execute, sign, or publish artifacts.
The shadow therefore composes rather than duplicates the two systems:

1. ICELINES builds, packages, checksums, and uploads.
2. A separate same-platform job downloads the package.
3. `scripts/ferris-artifact.py` recomputes the artifact, toolchain,
   configuration, embedded-manifest, and command identities and asks Ferris to
   classify both required consumers.
4. ICELINES verifies the sidecar, archive members, source revision, binary hash,
   and packaged binary version, then executes `--version`.

## Role review

| Lens | Finding | Disposition |
| --- | --- | --- |
| ICELINES `KEEL` | The slice reuses the canonical package and release-verification chain and changes no product surface or data path. | `pass` |
| ICELINES `BENCH` | Local positive proof covers ZIP and TAR readers, two-consumer fan-in, manifest/source binding, binary hash, and binary smoke. Hosted evidence is still required on all three platforms. | `pass-with-condition` |
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
