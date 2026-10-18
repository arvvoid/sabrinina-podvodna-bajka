# SPDX-License-Identifier: MIT
import hashlib
import importlib.util
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("package_release", ROOT / "tools/package_release.py")
packager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packager)


class PackageReleaseTests(unittest.TestCase):
    def test_archives_include_runtime_assets_licenses_and_checksums(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative in packager.RUNTIME_FOLDERS:
                folder = root / "assets" / relative
                folder.mkdir(parents=True, exist_ok=True)
                (folder / "sample").write_bytes(b"asset")
            (root / "assets/ui/boot.png").write_bytes(b"image")
            (root / "docs").mkdir()
            (root / "docs/guide.md").write_text("guide")
            for name in ("LICENSE", "ASSET_LICENSE.md", "README.md"):
                (root / name).write_text(name)
            for archive_format, executable in (("zip", "sabrina.exe"), ("gztar", "sabrina")):
                with self.subTest(format=archive_format), patch.object(packager, "ROOT", root):
                    binary = root / executable
                    binary.write_bytes(b"executable")
                    binary.chmod(0o755)
                    archive = packager.package(binary, "sabrina-test", archive_format, root / "dist")
                    if archive_format == "zip":
                        with zipfile.ZipFile(archive) as opened:
                            names = opened.namelist()
                    else:
                        with tarfile.open(archive) as opened:
                            names = opened.getnames()
                            self.assertTrue(opened.getmember(f"sabrina-test/{executable}").mode & 0o111)
                    for relative in (
                        executable, "assets/ui/boot.png", "assets/fonts/sample",
                        "docs/guide.md", "LICENSE", "ASSET_LICENSE.md", "PLAY.txt",
                    ):
                        self.assertIn(f"sabrina-test/{relative}", names)
                    checksum = archive.with_name(archive.name + ".sha256").read_text()
                    self.assertEqual(
                        checksum, f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n",
                    )

    def test_missing_binary_and_unsafe_names_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaises(FileNotFoundError):
                packager.package(root / "missing", "game", "zip", root)
            binary = root / "sabrina"
            binary.touch()
            with self.assertRaises(ValueError):
                packager.package(binary, "../game", "zip", root)


if __name__ == "__main__":
    unittest.main()
