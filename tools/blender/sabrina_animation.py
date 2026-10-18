# SPDX-License-Identifier: MIT
"""Shared keyed Sabrina poses. Reauthor the saved rig without rebuilding its mesh."""

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import bpy
from mathutils import Matrix, Quaternion, Vector
from geometry import coord
from rigging import actions, rotate

ORIENTATION = Matrix.Rotation(math.radians(-55), 3, "Z") @ Matrix.Rotation(
    math.radians(65), 3, "Y"
)
CLIPS = ["Idle", "Swim", "Turn_Left", "Turn_Right", "Sing", "Wave", "Hurt", "Throw"]


def smooth(a, b, x):
    t = max(0.0, min(1.0, (x - a) / (b - a)))
    return t * t * (3 - 2 * t)


def bend_game(rig, name, angle):
    bone = rig.pose.bones[name]
    axis = bone.bone.matrix_local.to_3x3().inverted() @ coord((0, 0, 1))
    bone.rotation_quaternion = bone.rotation_quaternion @ Quaternion(axis, angle)


def aim(rig, name, direction, amount=1.0):
    bpy.context.view_layer.update()
    bone = rig.pose.bones[name]
    change = (
        (bone.tail - bone.head)
        .normalized()
        .rotation_difference(coord(direction).normalized())
    )
    target_global = change @ bone.matrix.to_quaternion()
    basis = (
        bone.parent.matrix
        @ bone.parent.bone.matrix_local.inverted()
        @ bone.bone.matrix_local
    )
    target = basis.to_quaternion().inverted() @ target_global
    bone.rotation_quaternion = bone.rotation_quaternion.slerp(target, amount)
    bpy.context.view_layer.update()


def anatomical(rig, name, angles):
    q = Quaternion()
    inverse = rig.data.bones[name].matrix_local.to_3x3().inverted()
    for axis, angle in zip(((1, 0, 0), (0, 1, 0), (0, 0, 1)), angles):
        q = q @ Quaternion(inverse @ coord(ORIENTATION @ Vector(axis)), angle)
    rig.pose.bones[name].rotation_quaternion = q


def pose(clip, u, r):
    t = math.tau * u
    swim = clip == "Swim" or clip.startswith("Turn")
    effort = 1 if swim else 0.75
    # A quiet torso and fixed elbow bend leave propulsion to the travelling tail wave.
    anatomical(r, "Spine", (-0.04 + 0.012 * math.sin(t), 0, 0))
    anatomical(r, "Chest", (-0.12 + 0.018 * math.sin(t + 0.5), 0, 0))
    anatomical(r, "Neck", (-0.27, 0, 0))
    anatomical(r, "Head", (-0.42 + 0.015 * math.sin(t + 0.3), 0.015 * math.sin(t), 0))
    # Bring the torso toward the travel axis without bending the neck backward.
    bend_game(r, "Spine", -0.24)
    for i in range(6):
        rotate(
            r,
            f"Tail_{i+1}",
            (
                0.018 * math.sin(t - i * 0.7),
                effort * (0.035 + 0.009 * i) * math.sin(t - i * 0.72),
                effort * (0.13 + 0.028 * i) * math.sin(t - i * 0.72),
            ),
        )
    for side, sign in [("L", 1), ("R", -1)]:
        rotate(
            r,
            "Fluke_" + side,
            (
                -sign * 0.07 * math.sin(t - 4),
                0.08 * math.sin(t - 4),
                0.26 * effort * math.sin(t - 4.1),
            ),
        )
        rotate(
            r,
            "FlukeTip_" + side,
            (-sign * 0.05 * math.sin(t - 4.6), 0, 0.20 * effort * math.sin(t - 4.7)),
        )
        anatomical(r, "Clavicle_" + side, (0, 0, -sign * 0.07))
        # T-pose -> relaxed arms beside the body. Elbows flex in one anatomical plane;
        # tiny palm sculls, not independent three-axis waves on every joint.
        anatomical(
            r,
            "UpperArm_" + side,
            (-0.06, sign * 0.10, -sign * (1.00 + 0.035 * math.sin(t))),
        )
        anatomical(
            r, "Forearm_" + side, (-0.30 - 0.035 * math.sin(t - 0.4), 0, -sign * 0.10)
        )
        anatomical(r, "Hand_" + side, (0.04 * math.sin(t - 0.8), 0, sign * 0.035))
        if clip == "Sing":
            anatomical(r, "UpperArm_" + side, (-0.15, sign * 0.10, -sign * 0.82))
            anatomical(r, "Forearm_" + side, (-0.55, 0, -sign * 0.12))
    for section in ["Back", "Left", "Right"]:
        for i in range(3):
            # Counter the head extension at the hair root so long hair streams aft.
            anatomical(
                r,
                f"Hair_{section}_{i+1}",
                (
                    (0.83 if i == 0 else 0.03) + 0.025 * math.sin(t - i * 0.65),
                    0.025 * math.sin(t - i * 0.6),
                    0.025 * math.sin(t - i * 0.65 - 0.7),
                ),
            )
    if clip.startswith("Turn"):
        anatomical(
            r,
            "Chest",
            (-0.12, (-1 if clip.endswith("Left") else 1) * 0.06 * math.sin(t), 0),
        )
    if clip == "Hurt":
        anatomical(r, "Chest", (-0.08, 0, 0.025 * math.sin(t)))
    if clip == "Wave":
        reach = math.sin(math.pi * u) ** 2
        bpy.context.view_layer.update()
        # Aim the two arm segments along the spell direction, with a gentle elbow
        # bend. Blend rotations, never translate/stretch the joints or wrists.
        for side, sign in [("L", 1), ("R", -1)]:
            for name, direction in [
                ("UpperArm", (1, -0.18, sign * 0.12)),
                ("Forearm", (1, 0.08, sign * 0.06)),
            ]:
                bone = r.pose.bones[name + "_" + side]
                desired = coord(direction).normalized()
                change = (
                    (bone.tail - bone.head).normalized().rotation_difference(desired)
                )
                target_global = change @ bone.matrix.to_quaternion()
                parent = bone.parent
                basis = (
                    parent.matrix
                    @ parent.bone.matrix_local.inverted()
                    @ bone.bone.matrix_local
                )
                target = basis.to_quaternion().inverted() @ target_global
                bone.rotation_quaternion = bone.rotation_quaternion.slerp(target, reach)
                bpy.context.view_layer.update()

    if clip in ("Idle", "Swim", "Turn_Left", "Turn_Right", "Throw"):
        for side, sign in (("L", 1), ("R", -1)):
            if swim:
                upper = (-0.93, -0.28, sign * 0.18)
                fore = (-0.80, -0.42 + 0.045 * math.sin(t - 0.5), sign * 0.20)
            else:
                upper = (-0.70, -0.65, sign * 0.16)
                fore = (0.10 + 0.055 * math.sin(t), -0.95, sign * 0.26)
            aim(r, "UpperArm_" + side, upper)
            aim(r, "Forearm_" + side, fore)
        if clip == "Throw":
            # Reach, cradle, then extend the elbow before release at 1.10 seconds.
            seconds = u * 1.6
            reach = smooth(0, 0.32, seconds) * (1 - smooth(1.13, 1.6, seconds))
            draw = smooth(0.32, 0.70, seconds) * (1 - smooth(0.82, 1.10, seconds))
            release = smooth(0.82, 1.10, seconds)
            upper = Vector((0.72, -0.18, -0.28)).lerp(
                Vector((0.05, -0.85, -0.22)), draw
            )
            fore = Vector((0.8, 0.28, 0.15)).lerp(Vector((0.6, 0.72, 0.16)), draw)
            upper = upper.lerp(Vector((1, 0.02, -0.10)), release)
            fore = fore.lerp(Vector((1, 0.06, 0.05)), release)
            aim(r, "UpperArm_R", upper, reach)
            aim(r, "Forearm_R", fore, reach)
            aim(r, "UpperArm_L", (-0.65, -0.4, 0.5), reach * 0.45)
            aim(r, "Forearm_L", (-0.45, -0.65, 0.3), reach * 0.45)


def author_actions(rig):
    """Replace only editable animation keys, retaining the reviewed mesh and weights."""
    rig.animation_data_clear()
    for action in list(bpy.data.actions):
        bpy.data.actions.remove(action)
    return actions(
        rig,
        [
            (
                n,
                (
                    1.6
                    if n == "Throw"
                    else (
                        0.8
                        if n == "Wave"
                        else (
                            1.8
                            if n == "Swim"
                            else 3.2 if n in ("Idle", "Sing") else 1.6
                        )
                    )
                ),
            )
            for n in CLIPS
        ],
        pose,
    )


if __name__ == "__main__":
    rig = next(
        (
            o
            for o in bpy.context.scene.objects
            if o.type == "ARMATURE" and o.get("asset_id") == "sabrina"
        ),
        None,
    )
    if rig is None or not bpy.data.filepath:
        raise RuntimeError(
            "Open the saved Sabrina Blender source before reauthoring its animations."
        )
    author_actions(rig)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=bpy.data.filepath)
