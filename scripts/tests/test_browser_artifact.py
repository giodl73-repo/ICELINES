"""Retained artifact integrity/refusal cases, using synthetic bytes only."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("artifact", Path(__file__).parents[1] / "verify-browser-artifact.py")
artifact = importlib.util.module_from_spec(spec)
spec.loader.exec_module(artifact)
download_spec = importlib.util.spec_from_file_location("rollback", Path(__file__).parents[1] / "download-browser-rollback.py")
rollback = importlib.util.module_from_spec(download_spec)
download_spec.loader.exec_module(rollback)


class RollbackSourceTests(unittest.TestCase):
    def test_selection_refuses_shell_or_path_input(self):
        for repo, run, commit in [("owner/repo", "1; echo bad", "a"*40), ("../repo", "123", "a"*40), ("owner/repo", "123", "master")]:
            with self.assertRaises(ValueError):
                rollback.validate_selection(repo,run,commit)

    def test_only_successful_same_repository_master_build(self):
        run = {"conclusion":"success","head_sha":"a"*40,"head_branch":"master","event":"push",
               "path":".github/workflows/browser-pages.yml","head_repository":{"full_name":"owner/repo"}}
        rollback.validate_run(run,"owner/repo","a"*40)
        for key, value in [("conclusion","failure"),("head_sha","b"*40),("head_branch","feature"),("event","pull_request"),("path","other.yml"),("head_repository",{"full_name":"fork/repo"})]:
            with self.assertRaises(ValueError):
                rollback.validate_run({**run,key:value},"owner/repo","a"*40)


class BrowserArtifactTests(unittest.TestCase):
    def setUp(self):
        parent = Path(__file__).resolve().parents[2] / "target/browser-artifact-tests"
        parent.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=parent)
        self.root = Path(self.temporary.name)
        self.commit = "a" * 40
        payload = b'{"fixture":true}'
        sha = artifact.digest(payload)
        files = {"index.html": b"fixture", "style.css": b"fixture", "src/main.js": b"fixture", "src/worker.js": b"fixture",
                 "pkg/icelines_wasm.js": b"fixture", "pkg/icelines_wasm_bg.wasm": b"\0asm\x01\0\0\0",
                 "catalog.json": json.dumps({"schema_version": 1, "packages": [{"url": f"data/{sha}.json", "sha256": sha, "bytes": len(payload)}]}).encode()}
        files["build-info.json"] = json.dumps({"commit": self.commit, "working_tree_dirty": False, "wasm_sha256": artifact.digest(files["pkg/icelines_wasm_bg.wasm"])}).encode()
        template = "const manifest = __SHELL_MANIFEST__;\n"
        assets = [{"path": name, "sha256": artifact.digest(data), "bytes": len(data), "gzip_bytes": 0} for name, data in files.items()]
        identity = {"assets": assets, "worker_sha256": artifact.digest(template.encode())}
        self.build = artifact.digest(json.dumps(identity, separators=(",", ":")).encode())
        shell = {"schema_version": 1, "build": self.build, **identity}
        files.update({"shell-manifest.json": json.dumps(shell).encode(), "sw.js": template.replace("__SHELL_MANIFEST__", json.dumps(shell)).encode(), ".nojekyll": b"", f"data/{sha}.json": payload})
        for name, data in files.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)

    def tearDown(self):
        parent = Path(__file__).resolve().parents[2] / "target/browser-artifact-tests"
        if not self.root.resolve().is_relative_to(parent.resolve()):
            raise ValueError("test cleanup escaped workspace")
        self.temporary.cleanup()

    def test_accepts_clean_selected_identity(self):
        self.assertEqual(artifact.verify(self.root, self.commit, self.build)["packages"], 1)

    def test_refuses_wrong_commit_and_shell(self):
        for commit, build in [("b"*40,self.build),(self.commit,"b"*64)]:
            with self.assertRaises(ValueError):
                artifact.verify(self.root,commit,build)

    def test_refuses_dirty_source(self):
        info = json.loads((self.root / "build-info.json").read_bytes())
        info["working_tree_dirty"] = True
        (self.root / "build-info.json").write_text(json.dumps(info))
        with self.assertRaisesRegex(ValueError,"clean reviewed"):
            artifact.verify(self.root,self.commit,self.build)

    def test_refuses_changed_shell_package_or_worker(self):
        for name in ["style.css", "sw.js", next((self.root / "data").iterdir()).relative_to(self.root)]:
            path = self.root / name
            previous = path.read_bytes()
            path.write_bytes(previous + b"changed")
            with self.assertRaises(ValueError):
                artifact.verify(self.root,self.commit,self.build)
            path.write_bytes(previous)

    def test_refuses_extra_file(self):
        (self.root / "unexpected.txt").write_text("extra")
        with self.assertRaisesRegex(ValueError,"unexpected"):
            artifact.verify(self.root,self.commit,self.build)


if __name__ == "__main__":
    unittest.main()
