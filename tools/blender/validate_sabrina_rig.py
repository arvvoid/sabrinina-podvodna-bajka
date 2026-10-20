# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Check evaluated skin deformation in the maintained Blender source.

blender --background art/characters/sabrina.blend --python-exit-code 1 \
    --python tools/blender/validate_sabrina_rig.py
"""

import math

import bpy
import numpy as np
from mathutils import Quaternion, Vector

CLIPS = ("Idle", "Swim", "Turn_Left", "Turn_Right", "Sing", "Wave", "Hurt", "Throw", "Dive")


def elbow_sections(rig, body):
    """Measure two radial widths of a thin rest-space ring at each elbow."""
    rest = np.array([(*v.co, 1.0) for v in body.data.vertices])
    front = Vector((math.sin(math.radians(35)), 0, -math.cos(math.radians(35))))
    sections = []
    for side in ("L", "R"):
        forearm = rig.pose.bones[f"Forearm_{side}"]
        support = rig.data.bones[f"Elbow_{side}"]
        assert support.parent == forearm.bone.parent
        assert np.allclose(support.matrix_local, forearm.bone.matrix_local, atol=0.00001)
        local = (np.array(forearm.bone.matrix_local.inverted()) @ rest.T).T[:, :3]
        # Bone-local Y runs along the forearm; X/Z span its cross-section.
        ring = (np.abs(local[:, 1]) < 0.012) & (np.linalg.norm(local[:, [0, 2]], axis=1) < 0.20)
        assert ring.sum() >= 16, f"{side}: insufficient elbow cross-section samples"
        hinge = forearm.bone.matrix_local.to_quaternion().inverted() @ (
            Vector((0, 1 if side == "L" else -1, 0)).cross(front)
        )
        axes = np.array((hinge.cross(Vector((0, 1, 0))), hinge)).T
        width = np.diff(np.quantile(local[ring] @ axes, (0.05, 0.95), axis=0), axis=0)[0]
        assert (width > 0.01).all()
        sections.append((forearm, ring, axes, width))
    return sections


def check_elbows(pose, sections, clip, frame):
    for forearm, ring, axes, width in sections:
        # Measure in the half-angle frame, not camera space: rotating a healthy
        # elbow must not look like thickness loss. Quantiles ignore outliers.
        posed_rest = forearm.parent.matrix @ forearm.parent.bone.matrix_local.inverted() @ forearm.bone.matrix_local
        half_rotation = Quaternion().slerp(forearm.rotation_quaternion, 0.5)
        elbow_frame = posed_rest @ half_rotation.to_matrix().to_4x4()
        points = np.column_stack((pose[ring], np.ones(ring.sum())))
        local = (np.array(elbow_frame.inverted()) @ points.T).T[:, :3]
        deformed = np.diff(np.quantile(local @ axes, (0.05, 0.95), axis=0), axis=0)[0]
        ratio = deformed / width
        assert ((ratio >= 0.95) & (ratio <= 1.05)).all(), (
            f"{clip}/{frame}/{forearm.name}: elbow thickness collapse/bulge {ratio}"
        )


def main():
    rigs = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
    assert len(rigs) == 1 and rigs[0].get("asset_id") == "sabrina"
    rig = rigs[0]
    for side in ("L", "R"):
        upper = rig.data.bones[f"UpperArm_{side}"]
        forearm = rig.data.bones[f"Forearm_{side}"]
        clavicle = rig.data.bones[f"Clavicle_{side}"]
        assert 1.1 < upper.length / forearm.length < 1.3, "Elbow must separate anatomical arm lengths"
        assert clavicle.length < upper.length * 0.5, "Shoulder pivot must not sit halfway down the arm"
        assert (upper.tail_local - forearm.head_local).length < 0.00001
    meshes = [
        o for o in bpy.context.scene.objects
        if o.type == "MESH" and any(m.type == "ARMATURE" and m.object == rig for m in o.modifiers)
    ]
    assert len(meshes) == 2
    body = bpy.data.objects["Sabrina_Surface"]
    sections = elbow_sections(rig, body)
    geometry = []
    for obj in meshes:
        rest = np.array([v.co[:] for v in obj.data.vertices])
        edges = np.array([e.vertices[:] for e in obj.data.edges])
        lengths = np.linalg.norm(rest[edges[:, 0]] - rest[edges[:, 1]], axis=1)
        measurable = lengths > 0.00001
        geometry.append((obj, edges[measurable], lengths[measurable]))
    previous_action = rig.animation_data.action
    previous_frame = bpy.context.scene.frame_current
    previous_subframe = bpy.context.scene.frame_subframe
    try:
        for name in CLIPS:
            rig.animation_data.action = bpy.data.actions[name]
            start, end = rig.animation_data.action.frame_range
            ratios = []
            hands = []
            for frame in range(int(start), int(end) + 1):
                bpy.context.scene.frame_set(int(frame), subframe=frame - int(frame))
                hands.append([rig.pose.bones[f"Hand_{side}"].tail.copy() for side in ("L", "R")])
                if name in ("Idle", "Wave"):
                    palms = [
                        rig.pose.bones[f"Hand_{side}"].matrix.to_quaternion() @ Vector((1, 0, 0))
                        for side in ("L", "R")
                    ]
                    mirrored = Vector((palms[1].x, -palms[1].y, palms[1].z))
                    assert palms[0].dot(mirrored) > 0.95, f"{name}: reversed palm orientation"
                depsgraph = bpy.context.evaluated_depsgraph_get()
                for obj, edges, lengths in geometry:
                    evaluated = obj.evaluated_get(depsgraph)
                    pose = np.array([v.co[:] for v in evaluated.data.vertices])
                    assert np.isfinite(pose).all(), f"{name}: non-finite skin position"
                    if obj == body:
                        check_elbows(pose, sections, name, frame)
                    deformed = np.linalg.norm(pose[edges[:, 0]] - pose[edges[:, 1]], axis=1)
                    ratios.extend((deformed / lengths).tolist())
            lower, upper = np.quantile(ratios, (0.01, 0.99))
            assert max(ratios) < 2.0, f"{name}: abrupt skin-weight stretch ({max(ratios):.3f})"
            assert lower > 0.70 and upper < 1.35, f"{name}: widespread skin collapse/stretch"
            if name in ("Idle", "Swim", "Dive"):
                for side in range(2):
                    travel = max((a[side] - b[side]).length for a in hands for b in hands)
                    if name == "Idle":
                        assert travel > 0.15, f"{name}: static hand"
                    else:
                        assert 0.01 < travel < 0.4, f"{name}: moving arms should hold a quiet glide"
                    assert (hands[0][side] - hands[-1][side]).length < 0.01, f"{name}: loop seam"
                    if name == "Swim":
                        assert max(p[side].x for p in hands) < 0.2, "Swim hands must stay alongside the body"
                        assert min(abs(p[side].y) for p in hands) > 0.70, "Gliding hands must clear the hip fins"
                    if name == "Dive":
                        assert min(p[side].x for p in hands) > 1.5, "Dive hands must reach forward"
            if name == "Throw":
                assert hands[33][1].x - hands[24][1].x > 0.7, "Throw must extend at 1.1s release"
            if name == "Wave":
                for side in range(2):
                    assert hands[8][side].x - hands[0][side].x > 0.7, "Wave needs a fast forward push"
            print(
                f"{name}: all frames finite; edge-length p01={lower:.3f}, "
                f"p99={upper:.3f}, maximum={max(ratios):.3f}; elbow thickness within 5%"
            )
    finally:
        rig.animation_data.action = previous_action
        bpy.context.scene.frame_set(previous_frame, subframe=previous_subframe)
    print("Sabrina Blender deformation checks passed")


if __name__ == "__main__":
    main()
