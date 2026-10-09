"""Compare a live Pages deployment with a verified retained artifact.

This checks distribution bytes and MIME types, not browser execution, CORS,
offline reopening, device performance, or saved-data compatibility.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
from urllib.parse import quote, urljoin, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener


spec = importlib.util.spec_from_file_location("browser_artifact", Path(__file__).with_name("verify-browser-artifact.py"))
artifact = importlib.util.module_from_spec(spec)
spec.loader.exec_module(artifact)


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("deployment asset redirected")


def verify_file(url, expected_sha, expected_bytes=None, expected_mime=None):
    request = Request(url, headers={"Accept-Encoding": "identity", "User-Agent": "IceLines-deployment-verifier"})
    with build_opener(NoRedirect()).open(request, timeout=20) as response:
        mime = response.headers.get_content_type()
        if expected_mime and mime not in expected_mime:
            raise ValueError(f"unexpected MIME {mime}")
        limit = expected_bytes if expected_bytes is not None else 16 * 1024 * 1024
        payload = response.read(limit + 1)
        if len(payload) > limit or (expected_bytes is not None and len(payload) != expected_bytes):
            raise ValueError("deployment asset size mismatch")
        if hashlib.sha256(payload).hexdigest() != expected_sha:
            raise ValueError("deployment asset digest mismatch")
        return {"url": url, "bytes": len(payload), "sha256": expected_sha, "mime": mime}


def mime_for(name):
    if name.endswith(".wasm"):
        return {"application/wasm"}
    if name.endswith(".js"):
        return {"application/javascript", "text/javascript"}
    return {"html": {"text/html"}, "css": {"text/css"}, "json": {"application/json"}}.get(name.rsplit(".", 1)[-1])


def verify(root, base, commit, build, preservation=None):
    parsed = urlsplit(base)
    if parsed.scheme not in {"https", "http"} or not parsed.netloc or parsed.username or parsed.password or parsed.query or parsed.fragment or not base.endswith("/"):
        raise ValueError("provide a deployment base URL ending in / without credentials, query, or fragment")
    if parsed.scheme != "https" and parsed.hostname not in {"localhost", "127.0.0.1", "::1"}:
        raise ValueError("remote deployments require HTTPS; HTTP is allowed only for local test servers")
    identity = artifact.verify(root, commit, build)
    # Type declarations and the publisher marker are not runtime browser assets.
    files = sorted(p for p in root.rglob("*") if p.is_file() and p.name != ".nojekyll" and not p.name.endswith(".d.ts"))
    jobs = []
    for path in files:
        name = path.relative_to(root).as_posix()
        payload = path.read_bytes()
        jobs.append(("workbench", urljoin(base, quote(name, safe="/")), hashlib.sha256(payload).hexdigest(), len(payload), mime_for(name)))
    if preservation:
        inventory = json.loads(preservation.read_text(encoding="utf-8"))
        for name, sha in sorted(inventory["preserved_sha256"].items()):
            if name == ".nojekyll":
                continue
            parts = name.split("/")
            if any(part in {"", ".", ".."} for part in parts) or "\\" in name or ":" in name:
                raise ValueError("unsafe preserved documentation path")
            jobs.append(("documentation", urljoin(base, "../" + quote(name, safe="/")), sha, None, mime_for(name)))

    def probe(job):
        group, url, sha, size, mime = job
        try:
            return {"group": group, **verify_file(url, sha, size, mime)}
        except Exception as error:
            return {"group": group, "url": url, "error": str(error)}

    with ThreadPoolExecutor(max_workers=6) as executor:
        results = list(executor.map(probe, jobs))
    failures = [row for row in results if "error" in row]
    return {**identity, "base_url": base, "captured_at": datetime.now(timezone.utc).isoformat(),
            "transport": parsed.scheme, "verified": not failures, "runtime_files": len(files),
            "documentation_files": len(jobs) - len(files), "matched_files": len(jobs) - len(failures),
            "failures": failures, "files": results,
            "scope": "Static-byte and MIME verification only; no browser/offline/device/resource acceptance claim."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("base")
    parser.add_argument("commit")
    parser.add_argument("build")
    parser.add_argument("--preservation", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = verify(args.root, args.base, args.commit, args.build, args.preservation)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "files"}, indent=2))
    raise SystemExit(0 if report["verified"] else 1)
