#!/usr/bin/env python3
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Generate Rust collision samples from the Blender contact export."""

import argparse
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def export(check=False):
    data = json.loads((ROOT / "art/characters/sabrina.contacts.json").read_text(encoding="utf-8"))
    assert data["clips"] == [
        "Idle",
        "Swim",
        "Turn_Left",
        "Turn_Right",
        "Sing",
        "Wave",
        "Hurt",
        "Throw",
    ]
    assert len(data["arms"]) == 8
    lines = [
        "// Copyright (c) 2026 Luka Dežulović",
        "// SPDX-License-Identifier: Apache-2.0",
        "// Baked Blender poses; regenerate with tools/export_rust_contacts.py.",
        f"pub const PALM_OFFSET: f32 = {data['palm_offset']:.8f};",
        "pub static ARMS: [[[[f32; 3]; 7]; 32]; 8] = [",
    ]
    for clip in data["arms"]:
        assert len(clip) == 32
        lines.append("[")
        for pose in clip:
            assert len(pose) == 7 and all(len(point) == 3 for point in pose)
            lines.append(
                "["
                + ", ".join(
                    "[" + ", ".join(f"{v:.6f}" for v in point) + "]" for point in pose
                )
                + "],"
            )
        lines.append("],")
    lines.append("];")
    rustfmt = shutil.which("rustfmt") or str(Path.home() / ".cargo/bin/rustfmt")
    formatted = subprocess.run(
        [rustfmt, "--edition", "2024"],
        input="\n".join(lines),
        text=True,
        encoding="utf-8",
        capture_output=True,
        check=True,
        cwd=ROOT,
    ).stdout
    target = ROOT / "src/reef_sim/src/generated_contacts.rs"
    if check:
        assert (
            target.read_text(encoding="utf-8") == formatted
        ), "Run python3 tools/export_rust_contacts.py"
    else:
        target.write_text(formatted, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    export(parser.parse_args().check)
