# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Sample arm contacts from the poses used by the rendered skin."""

import json
from pathlib import Path
import sys

import bpy

ROOT = Path(__file__).resolve().parents[2]
CLIPS = ["Idle", "Swim", "Turn_Left", "Turn_Right", "Sing", "Wave", "Hurt", "Throw"]
SAMPLES = 32


def export_contacts(rig):
    previous_action = rig.animation_data.action
    previous_frame = bpy.context.scene.frame_current
    previous_subframe = bpy.context.scene.frame_subframe
    clips = []
    try:
        for clip in CLIPS:
            rig.animation_data.action = bpy.data.actions[clip]
            start, end = rig.animation_data.action.frame_range
            poses = []
            for index in range(SAMPLES):
                frame = start + (end - start) * index / (SAMPLES - 1)
                bpy.context.scene.frame_set(int(frame), subframe=frame - int(frame))
                points = []
                for side in ("L", "R"):
                    for name, endpoint in [
                        ("UpperArm", "head"),
                        ("Forearm", "head"),
                        ("Hand", "tail"),
                    ]:
                        points.append(
                            getattr(rig.pose.bones[name + "_" + side], endpoint)
                        )
                hand = rig.pose.bones["Hand_R"]
                points.append((hand.head + hand.tail) * 0.5)
                # Match glTF's Y-up axes and Sabrina's runtime scale.
                poses.append(
                    [
                        [round(v, 6) for v in (p.x * 0.88, p.z * 0.88, -p.y * 0.88)]
                        for p in points
                    ]
                )
            clips.append(poses)
        data = {
            "palm_offset": round(rig.data.bones["Hand_R"].length * 0.5, 8),
            "clips": CLIPS,
            "arms": clips,
        }
        (ROOT / "art/characters/sabrina.contacts.json").write_text(
            json.dumps(data, separators=(",", ":")) + "\n"
        )
        sys.path.insert(0, str(ROOT / "tools"))
        from export_rust_contacts import export

        export()
    finally:
        rig.animation_data.action = previous_action
        bpy.context.scene.frame_set(previous_frame, subframe=previous_subframe)
