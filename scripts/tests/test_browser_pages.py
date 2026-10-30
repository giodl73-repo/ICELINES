"""Deterministic publication composition tests; no network or deployment."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("pages", Path(__file__).parents[1] / "stage-browser-pages.py")
pages = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pages)


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
        (self.baseline / "workbench").mkdir()
        (self.baseline / "workbench/obsolete.js").write_bytes(b"old build")
        (self.baseline / ".git").mkdir()
        (self.baseline / ".git/config").write_bytes(b"repository metadata")
        result = pages.stage(self.baseline, self.browser, self.output)
        self.assertEqual(result["preserved_files"], 2)
        self.assertEqual((self.output / "index.html").read_bytes(), b"existing docs")
        self.assertEqual((self.output / "assets/site.css").read_bytes(), b"docs style")
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


if __name__ == "__main__":
    unittest.main()
