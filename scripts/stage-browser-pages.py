"""Preserve the existing Pages site and add the browser under workbench/."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil


def stage(baseline: Path, browser: Path, output: Path) -> dict:
    baseline, browser, output = baseline.resolve(), browser.resolve(), output.resolve()
    if output == baseline or output == browser or output.is_relative_to(baseline) or output.is_relative_to(browser):
        raise ValueError("output must be separate from both input trees")
    if not (baseline / "index.html").is_file() or not (browser / "shell-manifest.json").is_file():
        raise ValueError("existing site index and verified browser shell are required")
    if output.exists():
        raise ValueError("output must be new; refusing to replace an existing tree")
    preserved = {}
    additions = {}
    for source, target, inventory, excluded in [
        (baseline, output, preserved, {".git", "workbench"}),
        (browser, output / "workbench", additions, {".git"}),
    ]:
        for path in source.rglob("*"):
            relative = path.relative_to(source)
            if relative.parts[0] in excluded:
                continue
            if path.is_symlink():
                raise ValueError("publication cannot contain symbolic links")
            if path.is_file():
                inventory[relative.as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    output.mkdir(parents=True)
    for source, target, inventory in [(baseline, output, preserved), (browser, output / "workbench", additions)]:
        for name, digest in inventory.items():
            destination = target / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source / name, destination)
            if hashlib.sha256(destination.read_bytes()).hexdigest() != digest:
                raise ValueError("publication copy integrity failed")
    (output / ".nojekyll").touch()
    return {"browser_path": "workbench/", "preserved_files": len(preserved),
            "browser_files": len(additions), "preserved_sha256": preserved}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("browser", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(json.dumps(stage(args.baseline, args.browser, args.output), indent=2))
