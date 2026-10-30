"""Adversarial build-time archive boundary tests; no network calls."""
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("catalog", Path(__file__).parents[1] / "build-browser-catalog.py")
catalog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog)


class BrowserCatalogTests(unittest.TestCase):
    def archive(self, entries):
        temporary_root = Path(__file__).resolve().parents[2] / "target/browser-catalog-tests"
        temporary_root.mkdir(parents=True, exist_ok=True)
        directory = tempfile.TemporaryDirectory(dir=temporary_root)
        def cleanup():
            if not Path(directory.name).resolve().is_relative_to(temporary_root.resolve()):
                raise ValueError("temporary archive directory escaped test workspace")
            directory.cleanup()
        self.addCleanup(cleanup)
        path = Path(directory.name) / "data.tar.gz"
        with tarfile.open(path, "w:gz") as archive:
            for name, content, kind in entries:
                info = tarfile.TarInfo(name)
                info.type = kind
                info.size = len(content) if kind == tarfile.REGTYPE else 0
                if kind == tarfile.SYMTYPE:
                    info.linkname = "bios.json"
                archive.addfile(info, io.BytesIO(content) if info.isfile() else None)
        return path

    def test_release_layout_is_read_without_filesystem_extraction(self):
        path = self.archive([("bundle-20252026/bios.json", b"[]", tarfile.REGTYPE)])
        self.assertEqual(catalog.archive_files(path), {"bios.json": b"[]"})

    def test_traversal_is_rejected(self):
        path = self.archive([("../../bios.json", b"[]", tarfile.REGTYPE)])
        with self.assertRaisesRegex(ValueError, "unsafe"):
            catalog.archive_files(path)

    def test_duplicate_normalized_name_is_rejected(self):
        path = self.archive([("one/bios.json", b"[]", tarfile.REGTYPE), ("two/bios.json", b"[]", tarfile.REGTYPE)])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            catalog.archive_files(path)

    def test_symlink_is_rejected(self):
        path = self.archive([("bios.json", b"", tarfile.SYMTYPE)])
        with self.assertRaisesRegex(ValueError, "unsupported"):
            catalog.archive_files(path)

    def test_wrong_season_is_rejected(self):
        files = {"bios.json": json.dumps([{"playerId": 1, "seasonId": 20242025}]).encode(), "stats.json": b"[]"}
        with self.assertRaisesRegex(ValueError, "wrong season"):
            list(catalog.packages(20252026, files, "fixture"))

    def test_legacy_unknown_season_and_time_stay_unknown(self):
        files = {"bios.json": b'[{"playerId":1,"seasonId":null}]', "stats.json": b"[]"}
        package = list(catalog.packages(20252026, files, "fixture"))[0]
        self.assertIsNone(package["observed_at"])
        self.assertIsNone(package["fetched_at"])
        self.assertEqual(package["goalies"], [])

    def test_lockout_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "lockout"):
            list(catalog.packages(20042005, {}, "fixture"))


if __name__ == "__main__":
    unittest.main()
