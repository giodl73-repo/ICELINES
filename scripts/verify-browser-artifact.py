"""Validate retained distribution bytes without running the artifact's code."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def digest(data):
    return hashlib.sha256(data).hexdigest()


def verify(root: Path, commit: str, build: str) -> dict:
    if not re.fullmatch(r"[a-f0-9]{40}", commit) or not re.fullmatch(r"[a-f0-9]{64}", build):
        raise ValueError("full commit and shell SHA-256 are required")
    root = root.resolve()
    paths = list(root.rglob("*"))
    if any(path.is_symlink() for path in paths):
        raise ValueError("artifact contains symbolic links")
    shell = json.loads((root / "shell-manifest.json").read_text(encoding="utf-8"))
    info = json.loads((root / "build-info.json").read_text(encoding="utf-8"))
    if info["commit"] != commit or info["working_tree_dirty"] is not False:
        raise ValueError("artifact source identity does not match a clean reviewed commit")
    identity = json.dumps({"assets": shell["assets"], "worker_sha256": shell["worker_sha256"]}, separators=(",", ":"), ensure_ascii=False).encode()
    if shell["schema_version"] != 1 or shell["build"] != build or digest(identity) != build:
        raise ValueError("shell identity mismatch")
    allowed = {".nojekyll", "sw.js", "shell-manifest.json", "pkg/icelines_wasm.d.ts", "pkg/icelines_wasm_bg.wasm.d.ts"}
    for asset in shell["assets"]:
        name = asset["path"]
        if not re.fullmatch(r"(?:index\.html|style\.css|catalog\.json|build-info\.json|src/[a-z-]+\.js|pkg/icelines_wasm(?:_bg)?\.(?:js|wasm))", name):
            raise ValueError("unsafe shell path")
        if name in allowed:
            raise ValueError("duplicate shell path")
        payload = (root / name).read_bytes()
        if len(payload) != asset["bytes"] or digest(payload) != asset["sha256"]:
            raise ValueError("shell asset integrity mismatch")
        allowed.add(name)
    for name in ["index.html", "style.css", "catalog.json", "build-info.json", "src/main.js", "src/worker.js", "pkg/icelines_wasm.js", "pkg/icelines_wasm_bg.wasm"]:
        if name not in allowed:
            raise ValueError("required shell asset missing")
    wasm = (root / "pkg/icelines_wasm_bg.wasm").read_bytes()
    if not wasm.startswith(b"\0asm\x01\0\0\0") or digest(wasm) != info["wasm_sha256"]:
        raise ValueError("WASM identity mismatch")
    worker = (root / "sw.js").read_text(encoding="utf-8")
    embedded = re.search(r"^const manifest = (.+);$", worker, re.MULTILINE)
    if embedded is None or json.loads(embedded.group(1)) != shell:
        raise ValueError("worker manifest mismatch")
    template = worker[:embedded.start(1)] + "__SHELL_MANIFEST__" + worker[embedded.end(1):]
    if digest(template.encode()) != shell["worker_sha256"]:
        raise ValueError("worker template integrity mismatch")
    catalog = json.loads((root / "catalog.json").read_text(encoding="utf-8"))
    if catalog["schema_version"] != 1 or not catalog["packages"]:
        raise ValueError("invalid catalog")
    for entry in catalog["packages"]:
        if not re.fullmatch(r"[a-f0-9]{64}", entry["sha256"]) or entry["url"] != f"data/{entry['sha256']}.json":
            raise ValueError("unsafe package path")
        payload = (root / entry["url"]).read_bytes()
        if len(payload) != entry["bytes"] or digest(payload) != entry["sha256"]:
            raise ValueError("package integrity mismatch")
        allowed.add(entry["url"])
    if any(path.relative_to(root).as_posix() not in allowed for path in paths if path.is_file()):
        raise ValueError("unexpected artifact file")
    if (root / ".nojekyll").read_bytes() != b"":
        raise ValueError("invalid nojekyll marker")
    return {"commit": commit, "shell_build": build, "verified_files": len([p for p in paths if p.is_file()]), "packages": len(catalog["packages"])}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("commit")
    parser.add_argument("build")
    args = parser.parse_args()
    print(json.dumps(verify(args.root, args.commit, args.build), indent=2))
