#!/usr/bin/env python3

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys


REPOSITORY_ID = "giodl73-repo/ICELINES"
SCHEMA = "icelines.ferris-artifact-producer/v1"
REQUEST_SCHEMA = "ferris.artifact-reuse-request/v1"
QUALIFICATION_SCHEMA = "ferris.artifact-qualification-report/v1"
FEATURES = ["default"]


class ArtifactFailure(RuntimeError):
    pass


def sha256_bytes(value):
    return f"sha256:{hashlib.sha256(value).hexdigest()}"


def sha256_json(value):
    return sha256_bytes(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    )


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return f"sha256:{digest.hexdigest()}"


def rustc_identity():
    rustc = shutil.which("rustc")
    rustup = shutil.which("rustup")
    if rustup is None:
        candidate = Path.home() / ".cargo" / (
            "bin/rustup.exe" if Path.home().drive else "bin/rustup"
        )
        if candidate.is_file():
            rustup = str(candidate)
    if rustc is None and rustup:
        resolved = subprocess.run(
            [rustup, "which", "rustc"],
            check=False,
            capture_output=True,
            text=True,
        )
        if resolved.returncode == 0:
            candidate = Path(resolved.stdout.strip())
            if candidate.is_file():
                rustc = str(candidate)
    if rustc is None:
        raise ArtifactFailure("Rust compiler is unavailable")
    result = subprocess.run(
        [rustc, "-Vv"],
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        raise ArtifactFailure(
            f"rustc -Vv failed with exit code {result.returncode}"
        )
    return sha256_bytes(result.stdout)


def compatibility(manifest, args):
    return {
        "repository_id": REPOSITORY_ID,
        "source_revision": args.source_revision,
        "toolchain_identity": rustc_identity(),
        "platform_os": args.platform_os,
        "platform_architecture": args.platform_architecture,
        "target": args.target,
        "profile": "release",
        "features": FEATURES,
        "configuration_identity": sha256_json(
            {
                "features": FEATURES,
                "profile": "release",
                "target": args.target,
            }
        ),
        "manifest_identity": sha256_file(manifest),
        "command_identity": sha256_json(
            [
                "cargo",
                "build",
                "--release",
                "--locked",
                "--target",
                args.target,
                "-p",
                "icelines-cli",
            ]
        ),
    }


def strict_object(value, expected_keys, label):
    if not isinstance(value, dict) or set(value) != set(expected_keys):
        raise ArtifactFailure(f"{label} does not match its strict shape")
    return value


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"{json.dumps(value, indent=2, sort_keys=True)}\n", encoding="utf-8")


def produce(args):
    artifact = args.artifact.resolve(strict=True)
    manifest = args.manifest.resolve(strict=True)
    producer = {
        "schema": SCHEMA,
        "producer_node_id": f"package-{args.platform_os}",
        "attempt_id": args.attempt_id,
        "terminal_status": "succeeded",
        "artifact_id": args.artifact_id,
        "artifact_digest": sha256_file(artifact),
        "compatibility": compatibility(manifest, args),
    }
    write_json(args.output, producer)
    return {
        "artifact_id": producer["artifact_id"],
        "artifact_digest": producer["artifact_digest"],
        "producer_path": str(args.output),
    }


def consume(args):
    artifact = args.artifact.resolve(strict=True)
    manifest = args.manifest.resolve(strict=True)
    producer = json.loads(args.producer.read_text(encoding="utf-8"))
    strict_object(
        producer,
        {
            "schema",
            "producer_node_id",
            "attempt_id",
            "terminal_status",
            "artifact_id",
            "artifact_digest",
            "compatibility",
        },
        "producer envelope",
    )
    if producer["schema"] != SCHEMA:
        raise ArtifactFailure(f"unsupported producer schema: {producer['schema']}")
    required_compatibility = compatibility(manifest, args)
    request = {
        "schema": REQUEST_SCHEMA,
        "producer": {
            key: producer[key]
            for key in (
                "producer_node_id",
                "attempt_id",
                "terminal_status",
                "artifact_id",
                "artifact_digest",
                "compatibility",
            )
        },
        "consumers": [
            {
                "consumer_node_id": consumer_id,
                "required": True,
                "expected_artifact_id": producer["artifact_id"],
                "expected_artifact_digest": producer["artifact_digest"],
                "required_compatibility": required_compatibility,
            }
            for consumer_id in args.consumer_ids
        ],
        "expected_consumer_ids": args.consumer_ids,
    }
    write_json(args.request_output, request)

    result = subprocess.run(
        [
            str(args.ferris),
            "artifacts",
            "--request",
            str(args.request_output),
            "--artifact-path",
            str(artifact),
            "--manifest-path",
            str(manifest),
            "--require-compatible",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ArtifactFailure(
            f"ferris artifacts emitted invalid JSON ({result.returncode}): {error}"
        ) from error
    write_json(args.report_output, report)
    if result.returncode != 0:
        raise ArtifactFailure(
            f"ferris artifacts rejected qualification ({result.returncode})\n{result.stderr}"
        )
    if report.get("schema") != QUALIFICATION_SCHEMA or report.get("status") != "qualified":
        raise ArtifactFailure("Ferris did not emit a qualified artifact report")
    reuse_report = report.get("reuse_report", {})
    if not isinstance(reuse_report, dict):
        raise ArtifactFailure("Ferris qualification has an invalid reuse report")
    consumers = reuse_report.get("consumers", [])
    if not isinstance(consumers, list):
        raise ArtifactFailure("Ferris qualification has an invalid reuse report")
    reported_consumers = []
    for consumer in consumers:
        if not isinstance(consumer, dict) or not isinstance(
            consumer.get("consumer_node_id"), str
        ):
            raise ArtifactFailure("Ferris qualification has an invalid consumer result")
        reported_consumers.append(consumer["consumer_node_id"])
    requested_consumers = sorted(args.consumer_ids)
    reported_consumers.sort()
    if (
        reported_consumers != requested_consumers
        or reuse_report.get("fan_in", {}).get("expected_consumer_ids")
        != requested_consumers
        or any(consumer.get("classification") != "compatible" for consumer in consumers)
    ):
        raise ArtifactFailure(
            "Ferris qualification did not account for every requested consumer"
        )
    return {
        "artifact_id": producer["artifact_id"],
        "artifact_digest": report["measurement"]["artifact_digest"],
        "qualification_id": report["qualification_id"],
        "reuse_report_id": report["reuse_report"]["report_id"],
        "consumers": reported_consumers,
        "classification": "qualified",
    }


def add_compatibility_arguments(parser):
    parser.add_argument("--artifact", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--platform-os", required=True)
    parser.add_argument("--platform-architecture", required=True)
    parser.add_argument("--source-revision", required=True)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Bind ICELINES owner packages to Ferris artifact compatibility"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    producer = subparsers.add_parser("produce")
    add_compatibility_arguments(producer)
    producer.add_argument("--artifact-id", required=True)
    producer.add_argument("--attempt-id", required=True)
    producer.add_argument("--output", required=True, type=Path)

    consumer = subparsers.add_parser("consume")
    add_compatibility_arguments(consumer)
    consumer.add_argument("--producer", required=True, type=Path)
    consumer.add_argument("--ferris", required=True, type=Path)
    consumer.add_argument(
        "--consumer-id", dest="consumer_ids", required=True, action="append"
    )
    consumer.add_argument("--request-output", required=True, type=Path)
    consumer.add_argument("--report-output", required=True, type=Path)
    return parser.parse_args()


def main():
    args = parse_args()
    try:
        result = produce(args) if args.command == "produce" else consume(args)
    except (ArtifactFailure, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"ICELINES Ferris artifact qualification failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
