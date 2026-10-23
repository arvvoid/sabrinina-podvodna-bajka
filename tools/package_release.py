#!/usr/bin/env python3
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Bundle the self-contained game executable/app with only license notices."""

import argparse
import hashlib
from pathlib import Path
import plistlib
import shutil
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ASSET_NOTICES = (
    "fonts/LICENSE-DejaVu.txt",
    "fonts/LICENSE-Alegreya.txt",
    "audio/LICENSE-FluidR3.txt",
)


def package(binary, name, archive_format, output, platform=None):
    if platform is None:
        platform = {"linux": "linux", "win32": "windows", "darwin": "macos"}.get(sys.platform)
    if platform not in {"linux", "windows", "macos"}:
        raise ValueError(f"Unsupported package platform: {platform}")
    if not binary.is_file():
        raise FileNotFoundError(f"Build the game first: {binary}")
    if not name or name in {".", ".."} or any(
        char not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
        for char in name
    ):
        raise ValueError("Package name must contain only letters, digits, dots, _ or -")
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="sabrina-package-") as temporary:
        folder = Path(temporary) / name
        folder.mkdir()
        if platform == "macos":
            contents = folder / "Sabrina.app" / "Contents"
            (contents / "MacOS").mkdir(parents=True)
            (contents / "Resources").mkdir()
            shutil.copy2(binary, contents / "MacOS" / "sabrina")
            shutil.copy2(ROOT / "art/icons/app_icon.icns", contents / "Resources/app_icon.icns")
            with (contents / "Info.plist").open("wb") as manifest:
                plistlib.dump({
                    "CFBundleExecutable": "sabrina",
                    "CFBundleIconFile": "app_icon.icns",
                    "CFBundleIdentifier": "dev.arvvoid.sabrina",
                    "CFBundleName": "Sabrina",
                    "CFBundleDisplayName": "Sabrina's Underwater Fairy Tale",
                    "CFBundlePackageType": "APPL",
                    "NSHighResolutionCapable": True,
                    "LSMinimumSystemVersion": "11.0",
                }, manifest)
        else:
            shutil.copy2(binary, folder / binary.name)
        (folder / "licenses").mkdir()
        for relative in ASSET_NOTICES:
            source = ROOT / "assets" / relative
            if not source.is_file():
                raise FileNotFoundError(source)
            shutil.copy2(source, folder / "licenses" / source.name)
        for filename in ("LICENSE", "NOTICE"):
            shutil.copy2(ROOT / filename, folder / filename)
        shutil.copy2(ROOT / "ASSET_LICENSE.md", folder / "licenses/ASSET_LICENSE.txt")
        if archive_format == "directory":
            destination = output / name
            if destination.exists():
                shutil.rmtree(destination)
            shutil.copytree(folder, destination)
        else:
            destination = Path(shutil.make_archive(
                str(output / name), archive_format, root_dir=temporary, base_dir=name,
            ))
            checksum = hashlib.sha256(destination.read_bytes()).hexdigest()
            destination.with_name(destination.name + ".sha256").write_text(
                f"{checksum}  {destination.name}\n", encoding="ascii",
            )
    return destination


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--name", required=True)
    parser.add_argument("--format", choices=("zip", "gztar", "directory"), required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--platform", choices=("linux", "windows", "macos"))
    args = parser.parse_args()
    print(package(args.binary, args.name, args.format, args.output, args.platform))
