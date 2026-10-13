"""Verify the uploader's actual tar against the composed public site before deploy."""
import argparse
import hashlib
from pathlib import Path, PurePosixPath
import tarfile


def verify(site: Path, archive: Path) -> dict:
    site = site.resolve()
    expected = {}
    for path in site.rglob("*"):
        if path.is_symlink():
            raise ValueError("staged site contains a symbolic link")
        if path.is_file():
            expected[path.relative_to(site).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    if not expected:
        raise ValueError("staged site is empty")
    seen = set()
    with tarfile.open(archive, "r:*") as contents:
        for member in contents:
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or "\\" in member.name:
                raise ValueError("unsafe archive path")
            name = path.as_posix()
            if member.isdir():
                continue
            if not member.isfile():
                raise ValueError("archive contains a link or special entry")
            if name in seen or name not in expected:
                raise ValueError("duplicate or unexpected archive file")
            seen.add(name)
            source = contents.extractfile(member)
            digest = hashlib.sha256()
            with source:
                while chunk := source.read(1024 * 1024):
                    digest.update(chunk)
            if digest.hexdigest() != expected[name]:
                raise ValueError("archive file digest mismatch")
    if seen != set(expected):
        raise ValueError("archive is missing staged files: " + ", ".join(sorted(set(expected) - seen)))
    return {"verified": True, "files": len(seen), "hidden_files": sum(any(part.startswith(".") for part in PurePosixPath(name).parts) for name in seen)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("site", type=Path)
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    print(verify(args.site, args.archive))
