# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
import hashlib
import importlib.util
import os
from pathlib import Path
import plistlib
import struct
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
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for relative in packager.ASSET_NOTICES:
            notice = self.root / "assets" / relative
            notice.parent.mkdir(parents=True, exist_ok=True)
            notice.write_text("license", encoding="utf-8")
        (self.root / "docs").mkdir()
        (self.root / "docs/ARCHITECTURE.md").write_text("not for distribution")
        for name in ("LICENSE", "NOTICE", "ASSET_LICENSE.md", "README.md"):
            (self.root / name).write_text(name, encoding="utf-8")
        (self.root / "art/icons").mkdir(parents=True)
        (self.root / "art/icons/app_icon.icns").write_bytes(b"test icon")
        self.binary = self.root / "sabrina"
        self.binary.write_bytes(b"executable")
        self.binary.chmod(0o755)
        self.root_patch = patch.object(packager, "ROOT", self.root)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)

    def expected_files(self, platform, executable):
        files = {
            "LICENSE", "NOTICE", "licenses/ASSET_LICENSE.txt",
            "licenses/LICENSE-DejaVu.txt", "licenses/LICENSE-FluidR3.txt",
            "licenses/LICENSE-Alegreya.txt",
        }
        if platform == "macos":
            files.update({
                "Sabrina.app/Contents/MacOS/sabrina",
                "Sabrina.app/Contents/Info.plist",
                "Sabrina.app/Contents/Resources/app_icon.icns",
            })
        else:
            files.add(executable)
        return files

    def test_archives_include_standalone_binary_licenses_and_checksums(self):
        for platform in ("linux", "windows", "macos"):
            for archive_format, executable in (("zip", "sabrina.exe"), ("gztar", "sabrina")):
                with self.subTest(platform=platform, format=archive_format):
                    binary = self.root / executable
                    binary.write_bytes(b"executable")
                    binary.chmod(0o755)
                    archive = packager.package(
                        binary, "sabrina-test", archive_format, self.root / "dist", platform,
                    )
                    if archive_format == "zip":
                        with zipfile.ZipFile(archive) as opened:
                            names = {item.filename for item in opened.infolist() if not item.is_dir()}
                            notice = opened.read("sabrina-test/licenses/ASSET_LICENSE.txt")
                    else:
                        with tarfile.open(archive) as opened:
                            names = {item.name for item in opened.getmembers() if item.isfile()}
                            notice = opened.extractfile(
                                "sabrina-test/licenses/ASSET_LICENSE.txt"
                            ).read()
                            if os.name == "posix":
                                path = "Sabrina.app/Contents/MacOS/sabrina" if platform == "macos" else executable
                                self.assertTrue(
                                    opened.getmember(f"sabrina-test/{path}").mode & 0o111
                                )
                    self.assertEqual(names, {
                        f"sabrina-test/{name}" for name in self.expected_files(platform, executable)
                    })
                    self.assertEqual(notice, (self.root / "ASSET_LICENSE.md").read_bytes())
                    checksum = archive.with_name(archive.name + ".sha256").read_text()
                    self.assertEqual(
                        checksum, f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n",
                    )

    def test_directory_packages_and_macos_bundle_icon(self):
        for platform in ("linux", "windows", "macos"):
            with self.subTest(platform=platform):
                directory = packager.package(
                    self.binary, f"game-{platform}", "directory", self.root / "dist", platform,
                )
                files = {
                    path.relative_to(directory).as_posix()
                    for path in directory.rglob("*") if path.is_file()
                }
                self.assertEqual(files, self.expected_files(platform, self.binary.name))
                if platform == "macos":
                    contents = directory / "Sabrina.app/Contents"
                    manifest = plistlib.loads((contents / "Info.plist").read_bytes())
                    self.assertEqual(manifest["CFBundleExecutable"], "sabrina")
                    self.assertEqual(manifest["CFBundlePackageType"], "APPL")
                    icon = contents / "Resources" / manifest["CFBundleIconFile"]
                    self.assertEqual(icon.read_bytes(), b"test icon")
                    self.assertEqual((contents / "MacOS/sabrina").read_bytes(), b"executable")

    def test_missing_macos_icon_is_an_error(self):
        (self.root / "art/icons/app_icon.icns").unlink()
        with self.assertRaises(FileNotFoundError):
            packager.package(self.binary, "game", "zip", self.root / "dist", "macos")

    def test_default_platform_matches_host_and_repackaging_removes_old_docs(self):
        for host, platform in (("linux", "linux"), ("win32", "windows"), ("darwin", "macos")):
            with self.subTest(host=host), patch.object(packager.sys, "platform", host):
                directory = packager.package(
                    self.binary, "game", "directory", self.root / "dist",
                )
                (directory / "README.md").write_text("stale documentation")
                directory = packager.package(
                    self.binary, "game", "directory", self.root / "dist",
                )
                files = {
                    path.relative_to(directory).as_posix()
                    for path in directory.rglob("*") if path.is_file()
                }
                self.assertEqual(files, self.expected_files(platform, self.binary.name))

    def test_native_icon_formats_contain_expected_sizes(self):
        ico = (ROOT / "art/icons/app_icon.ico").read_bytes()
        reserved, kind, count = struct.unpack_from("<HHH", ico)
        self.assertEqual((reserved, kind), (0, 1))
        sizes = set()
        for offset in range(6, 6 + count * 16, 16):
            width, height, _, _, _, _, length, start = struct.unpack_from("<BBBBHHII", ico, offset)
            sizes.add((width or 256, height or 256))
            self.assertLessEqual(start + length, len(ico))
        self.assertEqual(sizes, {(size, size) for size in (16, 24, 32, 48, 64, 128, 256)})
        icns = (ROOT / "art/icons/app_icon.icns").read_bytes()
        self.assertEqual(icns[:4], b"icns")
        self.assertEqual(struct.unpack_from(">I", icns, 4)[0], len(icns))
        self.assertIn(b"ic09", icns)  # 512px Finder/Dock representation.

    def test_missing_binary_and_unsafe_names_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaises(FileNotFoundError):
                packager.package(root / "missing", "game", "zip", root)
            binary = root / "sabrina"
            binary.touch()
            with self.assertRaises(ValueError):
                packager.package(binary, "../game", "zip", root)
            with self.assertRaises(ValueError):
                packager.package(binary, "game", "zip", root, "unknown")


if __name__ == "__main__":
    unittest.main()
