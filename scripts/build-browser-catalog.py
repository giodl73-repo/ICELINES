"""Convert repository seasons or downloaded release archives into browser packages.

Never extract archive entries onto disk. Publish content-addressed, validated JSON
packages and a catalog; both native and browser loading use icelines-data.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import tarfile

MAX_ARCHIVE = 25 * 1024 * 1024
MAX_FILE = 32 * 1024 * 1024
MAX_TOTAL = 100 * 1024 * 1024
FILES = {
    "bios.json", "stats.json", "goalie-stats.json", "manifest.json",
    "playoff-bios.json", "playoff-stats.json", "playoff-goalie-stats.json",
}


def archive_files(path: Path) -> dict[str, bytes]:
    if path.stat().st_size > MAX_ARCHIVE:
        raise ValueError("compressed archive exceeds 25 MiB")
    result: dict[str, bytes] = {}
    total = 0
    with tarfile.open(path, "r:gz") as archive:
        for index, entry in enumerate(archive):
            if index >= 128:
                raise ValueError("too many archive entries")
            name = PurePosixPath(entry.name)
            if name.is_absolute() or ".." in name.parts or "\\" in entry.name or ":" in entry.name:
                raise ValueError("unsafe archive path")
            if entry.isdir():
                continue
            if not entry.isfile() or name.name not in FILES:
                raise ValueError("unsupported archive entry")
            if name.name in result:
                raise ValueError("duplicate archive file")
            total += entry.size
            if entry.size > MAX_FILE or total > MAX_TOTAL:
                raise ValueError("expanded archive exceeds limits")
            stream = archive.extractfile(entry)
            if stream is None:
                raise ValueError("missing archive bytes")
            with stream:
                data = stream.read(MAX_FILE + 1)
            if len(data) != entry.size:
                raise ValueError("archive size mismatch")
            result[name.name] = data
    return result


def packages(season: int, files: dict[str, bytes], source: str):
    start, end = divmod(season, 10000)
    if not (1900 <= start <= 2999 and end == start + 1) or season == 20042005:
        raise ValueError("invalid or lockout season")
    for kind, prefix in (("regular", ""), ("playoff", "playoff-")):
        bios_name, stats_name = prefix + "bios.json", prefix + "stats.json"
        if bios_name not in files or stats_name not in files:
            if kind == "regular":
                raise ValueError("missing required bios/stats files")
            continue
        bios, stats = json.loads(files[bios_name]), json.loads(files[stats_name])
        goalie_name = prefix + "goalie-stats.json"
        goalies = json.loads(files[goalie_name]) if goalie_name in files else []
        if not all(isinstance(rows, list) for rows in (bios, stats, goalies)):
            raise ValueError("season files must contain arrays")
        if not bios:
            if kind == "regular":
                raise ValueError("empty required regular bios")
            continue
        for row in bios + stats + goalies:
            if not isinstance(row, dict) or row.get("seasonId") not in (None, season):
                raise ValueError(f"wrong season or malformed row in {season} {kind}: {row.get('seasonId') if isinstance(row, dict) else 'not an object'}")
        yield {
            "schema_version": 1, "season": season, "season_type": kind,
            "source": source, "observed_at": None, "fetched_at": None,
            "bios": bios, "stats": stats, "goalies": goalies,
        }


def build(seasons: Path, output: Path, releases: Path | None = None) -> dict:
    data_dir = output / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    catalog = {"schema_version": 1, "packages": []}
    inputs = []
    for directory in sorted(seasons.iterdir(), reverse=True):
        if directory.is_dir() and directory.name.isdigit():
            files = {path.name: path.read_bytes() for path in directory.iterdir() if path.name in FILES}
            inputs.append((int(directory.name), files, "repository:" + directory.name))
    if releases:
        # Explicit release inputs supersede the corresponding repository season.
        release_inputs = {}
        for path in sorted(releases.glob("data-*.tar.gz")):
            season = int(path.name.removeprefix("data-").removesuffix(".tar.gz"))
            release_inputs[season] = (season, archive_files(path), "github-release:data-" + str(season))
        inputs = [item for item in inputs if item[0] not in release_inputs] + list(release_inputs.values())
    for season, files, source in inputs:
        for package in packages(season, files, source):
            encoded = json.dumps(package, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
            if len(encoded) > MAX_TOTAL:
                raise ValueError("expanded package exceeds 100 MiB")
            digest = hashlib.sha256(encoded).hexdigest()
            path = data_dir / (digest + ".json")
            path.write_bytes(encoded)
            catalog["packages"].append({
                "id": f"{season}-{package['season_type']}", "season": season,
                "season_type": package["season_type"], "source": source,
                "url": "data/" + path.name, "sha256": digest, "bytes": len(encoded),
                "skaters": len({row["playerId"] for row in package["bios"]}),
                "goalies": len(package["goalies"]),
                "observed_at": None,
                "file_sha256": {name: hashlib.sha256(data).hexdigest() for name, data in files.items()},
            })
    catalog["packages"].sort(key=lambda item: (item["season"], item["season_type"]), reverse=True)
    (output / "catalog.json").write_text(json.dumps(catalog, indent=2), encoding="utf-8")
    return catalog


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seasons", type=Path, default=root / "data/seasons")
    parser.add_argument("--output", type=Path, default=root / "icelines-browser/public")
    parser.add_argument("--release-dir", type=Path)
    args = parser.parse_args()
    result = build(args.seasons, args.output, args.release_dir)
    print(f"Published {len(result['packages'])} browser packages to {args.output}")
