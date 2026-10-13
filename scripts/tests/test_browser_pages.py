"""Deterministic publication composition tests; no network or deployment."""
import importlib.util
from pathlib import Path
import tempfile
import tarfile
import io
import unittest

spec = importlib.util.spec_from_file_location("pages", Path(__file__).parents[1] / "stage-browser-pages.py")
pages = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pages)
archive_spec = importlib.util.spec_from_file_location("pages_archive", Path(__file__).parents[1] / "verify-browser-pages-archive.py")
pages_archive = importlib.util.module_from_spec(archive_spec)
archive_spec.loader.exec_module(pages_archive)


class BrowserPagesTests(unittest.TestCase):
    def setUp(self):
        root = Path(__file__).resolve().parents[2] / "target/browser-pages-tests"
        root.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=root)
        self.root = Path(self.temporary.name)
        self.baseline, self.browser, self.output = [self.root / x for x in ("baseline", "browser", "output")]
        for path in [self.baseline, self.browser]:
            path.mkdir()
        (self.baseline / "index.html").write_bytes(b"existing docs")
        (self.baseline / "assets").mkdir()
        (self.baseline / "assets/site.css").write_bytes(b"docs style")
        (self.browser / "shell-manifest.json").write_bytes(b"{}")
        (self.browser / "index.html").write_bytes(b"browser")
        (self.browser / ".nojekyll").touch()

    def tearDown(self):
        root = Path(__file__).resolve().parents[2] / "target/browser-pages-tests"
        if not self.root.resolve().is_relative_to(root.resolve()):
            raise ValueError("test cleanup escaped workspace")
        self.temporary.cleanup()

    def test_preserves_docs_and_replaces_only_workbench(self):
        (self.baseline / ".well-known").mkdir()
        (self.baseline / ".well-known/security.txt").write_bytes(b"existing public contact")
        (self.baseline / "workbench").mkdir()
        (self.baseline / "workbench/obsolete.js").write_bytes(b"old build")
        (self.baseline / ".git").mkdir()
        (self.baseline / ".git/config").write_bytes(b"repository metadata")
        result = pages.stage(self.baseline, self.browser, self.output)
        self.assertEqual(result["preserved_files"], 3)
        self.assertEqual((self.output / "index.html").read_bytes(), b"existing docs")
        self.assertEqual((self.output / "assets/site.css").read_bytes(), b"docs style")
        self.assertEqual((self.output / ".well-known/security.txt").read_bytes(), b"existing public contact")
        self.assertEqual((self.output / "workbench/index.html").read_bytes(), b"browser")
        self.assertFalse((self.output / "workbench/obsolete.js").exists())
        self.assertFalse((self.output / ".git").exists())
        self.assertTrue((self.output / ".nojekyll").is_file())

    def test_refuses_existing_output(self):
        self.output.mkdir()
        with self.assertRaisesRegex(ValueError, "must be new"):
            pages.stage(self.baseline, self.browser, self.output)

    def test_refuses_nested_output(self):
        for output in [self.browser / "nested", self.baseline / "nested", self.baseline]:
            with self.assertRaisesRegex(ValueError, "separate"):
                pages.stage(self.baseline, self.browser, output)

    def test_requires_existing_site_and_browser(self):
        (self.baseline / "index.html").unlink()
        with self.assertRaisesRegex(ValueError, "required"):
            pages.stage(self.baseline, self.browser, self.output)

    def archive(self, entries):
        target = self.root / "artifact.tar"
        with tarfile.open(target, "w") as archive:
            for name, payload in entries:
                entry = tarfile.TarInfo(name)
                entry.size = len(payload)
                archive.addfile(entry, io.BytesIO(payload))
        return target

    def test_actual_archive_preserves_hidden_files(self):
        pages.stage(self.baseline, self.browser, self.output)
        entries = [("./" + path.relative_to(self.output).as_posix(), path.read_bytes()) for path in self.output.rglob("*") if path.is_file()]
        result = pages_archive.verify(self.output, self.archive(entries))
        self.assertTrue(result["verified"])
        self.assertEqual(result["hidden_files"], 2)

    def test_archive_refuses_hidden_file_omission(self):
        pages.stage(self.baseline, self.browser, self.output)
        entries = [(path.relative_to(self.output).as_posix(), path.read_bytes()) for path in self.output.rglob("*") if path.is_file() and path.name != ".nojekyll"]
        with self.assertRaisesRegex(ValueError, "missing"):
            pages_archive.verify(self.output, self.archive(entries))

    def test_archive_refuses_changed_documentation(self):
        with self.assertRaisesRegex(ValueError, "digest"):
            pages_archive.verify(self.baseline, self.archive([("index.html", b"wrong bytes")]))

    def test_archive_refuses_duplicate_and_unexpected_files(self):
        for entries in [[("index.html", b"existing docs"), ("./index.html", b"existing docs")], [("extra.txt", b"extra")]]:
            with self.assertRaisesRegex(ValueError, "duplicate or unexpected"):
                pages_archive.verify(self.baseline, self.archive(entries))

    def test_archive_refuses_traversal(self):
        with self.assertRaisesRegex(ValueError, "unsafe"):
            pages_archive.verify(self.baseline, self.archive([("../index.html", b"existing docs")]))

    def test_archive_refuses_link(self):
        target = self.root / "artifact.tar"
        with tarfile.open(target, "w") as archive:
            entry = tarfile.TarInfo("index.html")
            entry.type = tarfile.SYMTYPE
            entry.linkname = "elsewhere"
            archive.addfile(entry)
        with self.assertRaisesRegex(ValueError, "link"):
            pages_archive.verify(self.baseline, target)


if __name__ == "__main__":
    unittest.main()
