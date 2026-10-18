#!/usr/bin/env python3
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Bundle the self-contained game executable with documentation and license notices."""

import argparse
import hashlib
from pathlib import Path
import shutil
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ASSET_NOTICES = (
    "fonts/LICENSE-DejaVu.txt",
    "audio/LICENSE-FluidR3.txt",
)


def package(binary, name, archive_format, output):
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
        shutil.copy2(binary, folder / binary.name)
        (folder / "licenses").mkdir()
        for relative in ASSET_NOTICES:
            source = ROOT / "assets" / relative
            if not source.is_file():
                raise FileNotFoundError(source)
            shutil.copy2(source, folder / "licenses" / source.name)
        shutil.copytree(ROOT / "docs", folder / "docs")
        for filename in ("LICENSE", "NOTICE", "ASSET_LICENSE.md", "README.md"):
            shutil.copy2(ROOT / filename, folder / filename)
        (folder / "PLAY.txt").write_text(
            "Sabrina's Underwater Fairy Tale\n\n"
            "All game assets are embedded in the executable; no assets folder is needed.\n"
            "Keep the documentation and license notices when redistributing the package.\n"
            "Code: Apache 2.0 (LICENSE and NOTICE). Game content and branding are reserved.\n"
            "Redistributing reserved content requires permission; see ASSET_LICENSE.md.\n"
            "Windows: open sabrina.exe.\n"
            "Linux/macOS: open a terminal in this folder and run ./sabrina.\n\n"
            "Linux requires Vulkan drivers, ALSA, libudev and X11 runtime libraries.\n"
            "The executable is asset-self-contained, not fully statically linked.\n"
            "Linux builds target Ubuntu 24.04 or newer compatible distributions.\n"
            "macOS builds require Apple Silicon and are unsigned/not notarized.\n"
            "Follow your system's security guidance before opening downloaded software.\n\n"
            "This is an early working version. See README.md for controls and status,\n"
            "and ASSET_LICENSE.md for artwork and audio restrictions.\n",
            encoding="utf-8",
        )
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
    args = parser.parse_args()
    print(package(args.binary, args.name, args.format, args.output))
