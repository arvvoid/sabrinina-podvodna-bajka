#!/usr/bin/env python3
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Rasterize the maintained SVG interface for Bevy's image loader."""

from pathlib import Path
import shutil
import subprocess

root = Path(__file__).resolve().parents[1] / "assets/ui"
magick = shutil.which("magick")
if not magick:
    raise SystemExit("Install ImageMagick (magick), then run this tool again.")
(root / "reef_png").mkdir(exist_ok=True)
for source in sorted((root / "reef").glob("*.svg")):
    subprocess.run(
        [
            magick,
            "-background",
            "none",
            str(source),
            str(root / "reef_png" / (source.stem + ".png")),
        ],
        check=True,
    )
