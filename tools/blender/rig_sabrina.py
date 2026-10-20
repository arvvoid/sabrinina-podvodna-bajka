# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Replace the owner's Tripo humanoid rig with a mermaid rig and game actions.

blender --background --python-exit-code 1 --python tools/blender/rig_sabrina.py \
    -- mermaid.glb output.blend
This import step preserves topology, UVs and packed PBR maps, with a uniform
game-space transform and a longer tail. Subsequent edits use the saved source.
"""

import hashlib
import math
from pathlib import Path
import sys

import bmesh
import bpy
from mathutils import Quaternion, Vector

SCALE = 6.0
WAIST = 0.63
TILT = math.radians(35)
HAIR_LEVELS = (0.92, 0.85, 0.77, 0.69, 0.61, 0.55)
HAIR_LANES = (-0.085, -0.043, 0.0, 0.043, 0.085)
TAIL_LENGTH_SCALE = 1.12
# End frames at 30 FPS, in the order used by contact exports and SabrinaPose.
CLIPS = {
    "Idle": 96,
    "Swim": 54,
    "Turn_Left": 48,
    "Turn_Right": 48,
    "Sing": 96,
    "Wave": 36,
    "Hurt": 48,
    "Throw": 48,
    "Dive": 54,
}
FINGERS = ("Thumb", "Index", "Mid", "Ring", "Pinky")


def smooth(a, b, value):
    t = max(0.0, min(1.0, (value - a) / (b - a)))
    return t * t * (3 - 2 * t)


def position(p):
    """Map source Z-up anatomy into the tilted Blender game-space rest pose."""
    x, y, z = p
    return Vector((
        ((z - WAIST) * math.cos(TILT) - y * math.sin(TILT)) * SCALE,
        x * SCALE,
        ((z - WAIST) * math.sin(TILT) + y * math.cos(TILT)) * SCALE,
    ))


def pair_weights(value, stations):
    """Smoothly blend neighboring bone stations, retaining rigid plateaus."""
    ordered = sorted(stations)
    if value <= ordered[0][0]:
        return {ordered[0][1]: 1.0}
    for (start, start_bone), (end, end_bone) in zip(ordered, ordered[1:]):
        if value <= end:
            if start_bone == end_bone:
                return {start_bone: 1.0}
            blend = smooth(start, end, value)
            return {start_bone: 1 - blend, end_bone: blend}
    return {ordered[-1][1]: 1.0}


def face_region(p):
    x, y, z = p
    return (
        0.835 < z < 0.943 - 0.10 * abs(x)
        and abs(x) < 0.048 + 0.008 * smooth(0.86, 0.91, z)
        and y < -0.004
    )


def split_head(source):
    """Split by existing faces, retaining each corner's original texture UV."""
    pieces = []
    for is_face, name in ((True, "Sabrina_Face"), (False, "Sabrina_Groom")):
        obj = source.copy()
        obj.data = source.data.copy()
        obj.name = name
        bpy.context.collection.objects.link(obj)
        mesh = bmesh.new()
        mesh.from_mesh(obj.data)
        removed = [f for f in mesh.faces if face_region(f.calc_center_median()) != is_face]
        bmesh.ops.delete(mesh, geom=removed, context="FACES")
        bmesh.ops.delete(mesh, geom=[v for v in mesh.verts if not v.link_faces], context="VERTS")
        mesh.to_mesh(obj.data)
        mesh.free()
        obj.data.update()
        pieces.append(obj)
    bpy.data.objects.remove(source, do_unlink=True)
    return pieces


def imported_name(name):
    side, bone = name.split("_", 1)
    if bone == "Hand":
        return f"Hand_{side}"
    for finger in FINGERS:
        if bone.startswith(finger):
            return f"{finger}_{side}_{bone[-1]}"
    return None


def torso_weights(z):
    return pair_weights(z, [
        (0.18, "Tail_6"), (0.25, "Tail_5"), (0.33, "Tail_4"),
        (0.42, "Tail_3"), (0.51, "Tail_2"), (0.60, "Tail_1"),
        (0.655, "Spine"), (0.75, "Chest"), (0.825, "Neck"), (0.87, "Head"),
    ])


def blend_weights(a, b, amount):
    return {name: a.get(name, 0) * (1 - amount) + b.get(name, 0) * amount for name in a.keys() | b.keys()}


def weights(p, part, original):
    # Part IDs come from this Tripo model: 2 flukes, 3 torso, 4/5 arms,
    # 7/8 hip fins, 9/10 lower fins. Preserve supplied finger weights at the hands.
    x, _, z = p
    side = "L" if x >= 0 else "R"
    if part == "hair":
        if z >= 0.92 or (z > 0.835 and p.y < 0.015 and abs(x) < 0.06):
            return {"Head": 1.0}
        lanes = pair_weights(x, [(v, i) for i, v in enumerate(HAIR_LANES)])
        levels = [(0.92, "Head")] + [
            ((a + b) * 0.5, str(i + 1))
            for i, (a, b) in enumerate(zip(HAIR_LEVELS, HAIR_LEVELS[1:]))
        ]
        result = {}
        for lane, lane_weight in lanes.items():
            for segment, segment_weight in pair_weights(z, levels).items():
                name = "Head" if segment == "Head" else f"Hair_{lane}_{segment}"
                result[name] = result.get(name, 0) + lane_weight * segment_weight
        return result
    if part == "face":
        return {"Head": 1.0}
    if part in {"4", "5"} and abs(x) > 0.305:
        hand = {}
        for name, weight in original.items():
            if name.startswith(("L_", "R_")):
                mapped = imported_name(name)
                if mapped:
                    hand[mapped] = hand.get(mapped, 0) + weight
        total = sum(hand.values())
        if total > 0.5:
            return {name: weight / total for name, weight in hand.items()}
    if part in {"3", "4", "5"}:
        arm = pair_weights(abs(x), [
            (0.025, "Chest"), (0.045, f"Clavicle_{side}"),
            (0.105, f"UpperArm_{side}"), (0.15, f"UpperArm_{side}"),
            (0.1875, f"Elbow_{side}"),
            (0.225, f"Forearm_{side}"), (0.278, f"Forearm_{side}"),
            (0.305, f"Hand_{side}"),
        ])
        if part != "3":
            return arm
        shoulder_height = z + 1.2 * max(0, abs(x) - 0.055)
        torso = blend_weights(torso_weights(z), {"Chest": 1.0},
                              smooth(0.025, 0.06, abs(x)) * smooth(0.75, 0.80, z))
        return blend_weights(torso, arm,
                             smooth(0.74, 0.80, shoulder_height) * smooth(0.02, 0.06, abs(x)))
    if part == "2":
        return pair_weights(abs(x), [
            (0.02, "Tail_6"), (0.065, f"Fluke_{side}"), (0.165, f"FlukeTip_{side}"),
        ])
    if part in {"7", "8", "9", "10"}:
        upper = part in {"7", "8"}
        parent = "Tail_1" if upper else "Tail_4"
        fin = f"{'HipFin' if upper else 'LowerFin'}_{side}"
        root, tip = (0.635, 0.465) if upper else (0.36, 0.25)
        result = pair_weights(z, [(tip, fin + "_2"), ((root + tip) / 2, fin + "_1"), (root, parent)])
        attachment = result.pop(parent, 0)
        for bone, weight in torso_weights(z).items():
            result[bone] = result.get(bone, 0) + attachment * weight
        return result
    return torso_weights(z)


def prepare_mesh(obj, part, rig):
    original = [v.co.copy() for v in obj.data.vertices]
    old_groups = {g.index: g.name for g in obj.vertex_groups}
    skin = [
        weights(p, part, {old_groups[g.group]: g.weight for g in v.groups})
        for v, p in zip(obj.data.vertices, original)
    ]
    obj.vertex_groups.clear()
    for modifier in list(obj.modifiers):
        if modifier.type == "ARMATURE":
            obj.modifiers.remove(modifier)
    for vertex, p, influences in zip(obj.data.vertices, original, skin):
        influences = sorted(influences.items(), key=lambda item: item[1], reverse=True)[:4]
        total = sum(w for _, w in influences)
        if total <= 0:
            raise RuntimeError(f"Unweighted vertex {obj.name}/{vertex.index}")
        for name, weight in influences:
            if weight > 0:
                group = obj.vertex_groups.get(name) or obj.vertex_groups.new(name=name)
                group.add([vertex.index], weight / total, "REPLACE")
        vertex.co = position(p)
    colors = obj.data.color_attributes.get("Color") or obj.data.color_attributes.new(
        name="Color", type="FLOAT_COLOR", domain="CORNER"
    )
    for value in colors.data:
        value.color = (1, 1, 1, 1)
    obj.data.color_attributes.active_color = colors
    for face in obj.data.polygons:
        face.use_smooth = True
    modifier = obj.modifiers.new("Sabrina_Skin", "ARMATURE")
    modifier.object = rig
    obj.parent = rig
    return original


def upper_bones(side, sign):
    return [
        (f"Clavicle_{side}", (sign * 0.012, -0.002, 0.814), (sign * 0.0625, 0.002, 0.807), "Chest"),
        (f"UpperArm_{side}", (sign * 0.0625, 0.002, 0.807), (sign * 0.1875, 0.002, 0.795), f"Clavicle_{side}"),
        (f"Forearm_{side}", (sign * 0.1875, 0.002, 0.795), (sign * 0.293, -0.003, 0.7885), f"UpperArm_{side}"),
        (f"Elbow_{side}", (sign * 0.1875, 0.002, 0.795), (sign * 0.293, -0.003, 0.7885), f"UpperArm_{side}"),
        (f"Hand_{side}", (sign * 0.293, -0.003, 0.7885), (sign * 0.341, -0.005, 0.784), f"Forearm_{side}"),
    ]


def build_rig(landmarks):
    data = bpy.data.armatures.new("Sabrina_Skeleton")
    rig = bpy.data.objects.new("Sabrina_Rig", data)
    bpy.context.collection.objects.link(rig)
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")

    def bone(name, head, tail, parent=None):
        b = data.edit_bones.new(name)
        b.head, b.tail = position(head), position(tail)
        b.align_roll(Vector((0, 1, 0)))
        if parent:
            b.parent = data.edit_bones[parent]

    bone("Root", (0, 0, WAIST), (0, 0, WAIST + 0.03))
    bone("Spine", (0, 0, WAIST), (0, 0, 0.72), "Root")
    bone("Chest", (0, 0, 0.72), (0, 0, 0.81), "Spine")
    bone("Neck", (0, 0, 0.81), (0, 0, 0.86), "Chest")
    bone("Head", (0, 0, 0.86), (0, 0, 0.965), "Neck")
    for side, sign in (("L", 1), ("R", -1)):
        for definition in upper_bones(side, sign):
            bone(*definition)
        for finger in FINGERS:
            for i in range(1, 4):
                head, tail = landmarks[f"{side}_{finger}{i}"]
                bone(f"{finger}_{side}_{i}", head, tail,
                     f"Hand_{side}" if i == 1 else f"{finger}_{side}_{i - 1}")
    tail = (WAIST, 0.55, 0.46, 0.37, 0.29, 0.22, 0.17)
    for i, (a, b) in enumerate(zip(tail, tail[1:]), 1):
        bone(f"Tail_{i}", (0, -0.04, a), (0, -0.065, b), "Root" if i == 1 else f"Tail_{i - 1}")
    for side, sign in (("L", 1), ("R", -1)):
        bone(f"Fluke_{side}", (0, -0.065, 0.17), (sign * 0.09, -0.08, 0.09), "Tail_6")
        bone(f"FlukeTip_{side}", (sign * 0.09, -0.08, 0.09), (sign * 0.19, -0.08, 0.015), f"Fluke_{side}")
        for name, x, a, b, parent in [
            ("HipFin", 0.085, 0.635, 0.465, "Tail_1"),
            ("LowerFin", 0.045, 0.36, 0.25, "Tail_4"),
        ]:
            mid = (a + b) / 2
            bone(f"{name}_{side}_1", (sign * x, -0.025, a), (sign * (x + 0.025), -0.03, mid), parent)
            bone(f"{name}_{side}_2", (sign * (x + 0.025), -0.03, mid), (sign * (x + 0.035), -0.03, b), f"{name}_{side}_1")
    for lane, x in enumerate(HAIR_LANES):
        for i, (a, b) in enumerate(zip(HAIR_LEVELS, HAIR_LEVELS[1:]), 1):
            bone(f"Hair_{lane}_{i}", (x, 0.065, a), (x, 0.065, b),
                 "Head" if i == 1 else f"Hair_{lane}_{i - 1}")
    bpy.ops.object.mode_set(mode="OBJECT")
    rig["asset_id"] = "sabrina"
    rig["hair_chains"], rig["hair_segments"] = 5, 5
    return rig


def stroke_path(t, keys):
    for (a, start), (b, end) in zip(keys, keys[1:]):
        if t <= b:
            return Vector(start).lerp(Vector(end), smooth(a, b, t))
    return Vector(keys[-1][1])


def animate_arms(rig, name, t):
    shoulder_axis = Vector((-math.sin(TILT), 0, math.cos(TILT)))
    front = Vector((math.sin(TILT), 0, -math.cos(TILT)))
    for side, sign in (("L", 1), ("R", -1)):
        phase = math.tau * t + sign * 0.25
        idle = (72 + 5 * math.sin(phase), 5 + 7 * math.cos(phase), 22 + 10 * math.sin(phase + 0.6))
        angles = idle
        wrist = -8 + 5 * math.sin(phase + 0.8)
        curl = 0.04
        if name in ("Swim", "Turn_Left", "Turn_Right"):
            angles = (70 + 2 * math.sin(phase), 10 + 2 * math.cos(phase), 12 + 3 * math.sin(phase))
            wrist = -3 + 2 * math.sin(phase)
        elif name == "Dive":
            angles = (83 + math.sin(phase), 138 + 2 * math.cos(phase), 8 + 2 * math.sin(phase))
            wrist = 0
        elif name == "Wave":
            angles = stroke_path(t, [
                (0, idle), (0.10, (70, 60, 85)), (0.24, (78 if side == "L" else 84, 140, 8)),
                (0.43, (80, 130, 15)), (0.72, (65, 40, 45)), (1, idle),
            ])
            wrist = 38 * smooth(0.1, 0.24, t) * (1 - smooth(0.43, 0.8, t))
        elif name == "Throw" and side == "R":
            angles = stroke_path(t, [
                (0, idle), (0.25, (70, 40, 80)), (0.54, (74, 35, 95)),
                (1.1 / 1.6, (86, 140, 8)), (0.80, (80, 122, 15)), (1, idle),
            ])
            curl += 0.08 * smooth(0, 0.25, t) * (1 - smooth(0.59, 1.1 / 1.6, t))
        elif name == "Throw":
            angles = stroke_path(t, [
                (0, idle), (0.25, (55, 65, 65)), (0.54, (55, 65, 65)),
                (1.1 / 1.6, (42, 25, 35)), (0.80, (48, 20, 30)), (1, idle),
            ])
        elif name == "Sing":
            angles = (58 + 5 * math.sin(phase), 18, 45 + 8 * math.sin(phase))
        elif name == "Hurt":
            angles = (68, 20, 25 + 30 * math.sin(math.pi * t) ** 2)
        # Pose keys are degrees: shoulder drop, forward swing, elbow hinge bend.
        drop, swing, bend = map(math.radians, angles)
        clavicle_swing = max(0, swing - math.radians(55)) * 0.18
        clavicle = rig.pose.bones[f"Clavicle_{side}"]
        local = clavicle.bone.matrix_local.to_quaternion().inverted() @ Vector((0, 1, 0))
        clavicle.rotation_quaternion = Quaternion(local, -clavicle_swing)
        upper = rig.pose.bones[f"UpperArm_{side}"]
        rest = upper.bone.matrix_local.to_quaternion()
        rotation = Quaternion((0, 1, 0), -(swing - clavicle_swing)) @ Quaternion(shoulder_axis, sign * drop)
        upper.rotation_quaternion = rest.inverted() @ rotation @ rest
        hinge = Vector((0, sign, 0)).cross(front)
        for name_part, angle in (("Forearm", bend), ("Hand", math.radians(wrist))):
            bone = rig.pose.bones[f"{name_part}_{side}"]
            local = bone.bone.matrix_local.to_quaternion().inverted() @ hinge
            bone.rotation_quaternion = Quaternion(local, angle)
        # Half-angle skin support preserves elbow thickness under linear skinning.
        # It is not a parent of the forearm, so hand trajectories stay unchanged.
        rig.pose.bones[f"Elbow_{side}"].rotation_quaternion = Quaternion().slerp(
            rig.pose.bones[f"Forearm_{side}"].rotation_quaternion, 0.5
        )
        for finger in FINGERS:
            for i in range(1, 4):
                bone = rig.pose.bones[f"{finger}_{side}_{i}"]
                axis = bone.bone.matrix_local.to_quaternion().inverted() @ hinge
                bone.rotation_quaternion = Quaternion(axis, curl)


def animate(rig, clips=None):
    rig.animation_data_create()
    pitch_axis = Vector((0, 1, 0))
    selected_clips = CLIPS if clips is None else clips
    for name in selected_clips:
        end = CLIPS[name]
        if old := bpy.data.actions.get(name):
            bpy.data.actions.remove(old)
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        rig.animation_data.action = action
        previous = {}
        # Include the exact crystal release at 1.1 seconds even between regular keys.
        frames = sorted(set(range(0, end + 1, 2)) | ({33} if name == "Throw" else set()))
        for frame in frames:
            t = frame / end
            phase, pulse = t * math.tau, math.sin(math.pi * t) ** 2
            for b in rig.pose.bones:
                b.rotation_mode = "QUATERNION"
                b.rotation_quaternion = Quaternion()

            def turn(bone_name, axis, angle):
                b = rig.pose.bones[bone_name]
                local = b.bone.matrix_local.to_quaternion().inverted() @ axis
                b.rotation_quaternion = Quaternion(local, angle)

            turn("Spine", pitch_axis, 0.018 * math.sin(phase))
            turn("Chest", pitch_axis, 0.025 * math.sin(phase + 0.5))
            turn("Neck", pitch_axis, -0.20)
            turn("Head", pitch_axis, -0.47 + 0.012 * math.sin(phase))
            for i in range(1, 7):
                amplitude = (0.11 if name == "Idle" else 0.17) * (0.55 + i * 0.12)
                resting_bend = TILT * {1: 0.35, 2: 0.35, 3: 0.30}.get(i, 0.0)
                turn(f"Tail_{i}", pitch_axis, resting_bend + amplitude * math.sin(phase - i * 0.58))
                if i in (5, 6):
                    b = rig.pose.bones[f"Tail_{i}"]
                    b.rotation_quaternion @= Quaternion((0, 1, 0), math.radians(35))
            for side, sign in (("L", 1), ("R", -1)):
                turn(f"Fluke_{side}", pitch_axis, 0.12 * math.sin(phase - 3.6 + sign * 0.25))
                rig.pose.bones[f"Fluke_{side}"].rotation_quaternion @= Quaternion(
                    (0, 0, 1), sign * 0.045 * math.sin(phase - 3.0)
                )
                turn(f"FlukeTip_{side}", pitch_axis, 0.16 * math.sin(phase - 4.2 + sign * 0.25))
                for fin in ("HipFin", "LowerFin"):
                    for i in (1, 2):
                        turn(f"{fin}_{side}_{i}", pitch_axis,
                             (0.06 + i * 0.025) * math.sin(phase - i * 0.8 + sign * 0.5))
            if name == "Hurt":
                turn("Chest", pitch_axis, -0.18 * pulse)
            if name.startswith("Turn"):
                turn("Chest", Vector((0, 0, 1)), (0.16 if name == "Turn_Left" else -0.16) * pulse)
            if name == "Wave":
                turn("Chest", pitch_axis, 0.055 * smooth(0, 0.1, t)
                     - 0.11 * smooth(0.1, 0.22, t) + 0.055 * smooth(0.43, 1, t))
            if name == "Throw":
                turn("Chest", Vector((0, 0, 1)), 0.07 * smooth(0, 0.5, t)
                     - 0.13 * smooth(0.57, 1.1 / 1.6, t) + 0.06 * smooth(0.78, 1, t))
            animate_arms(rig, name, t)
            for b in rig.pose.bones:
                if not b.name.startswith("Hair_"):
                    if b.name in previous:
                        b.rotation_quaternion.make_compatible(previous[b.name])
                    previous[b.name] = b.rotation_quaternion.copy()
                    b.keyframe_insert("rotation_quaternion", frame=frame, group=b.name)
        for layer in action.layers:
            for strip in layer.strips:
                for bag in strip.channelbags:
                    for curve in bag.fcurves:
                        for key in curve.keyframe_points:
                            key.interpolation = "LINEAR"
    rig.animation_data.action = bpy.data.actions["Idle"]
    bpy.context.scene.frame_set(0)


def lengthen_tail(rig):
    previous = rig.get("tail_length_scale", 1.0)
    if abs(previous - TAIL_LENGTH_SCALE) < 0.00001:
        return
    axis = Vector((math.cos(TILT), 0, math.sin(TILT)))

    def extend(point):
        distance = min(0.0, point.dot(axis))
        return point + axis * distance * (TAIL_LENGTH_SCALE / previous - 1)

    body = bpy.data.objects["Sabrina_Surface"]
    for key in body.data.shape_keys.key_blocks:
        for vertex in key.data:
            vertex.co = extend(vertex.co)
    for vertex, basis in zip(body.data.vertices, body.data.shape_keys.key_blocks[0].data):
        vertex.co = basis.co
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for bone in rig.data.edit_bones:
        if bone.name.startswith(("Tail_", "Fluke", "HipFin_", "LowerFin_")):
            bone.head, bone.tail = extend(bone.head), extend(bone.tail)
    bpy.ops.object.mode_set(mode="OBJECT")
    rig["tail_length_scale"] = TAIL_LENGTH_SCALE


def refine_head(rig):
    if rig.get("adult_head_refined"):
        return

    def refine(point, face=False):
        x = point.y / SCALE
        y = (-point.x * math.sin(TILT) + point.z * math.cos(TILT)) / SCALE
        z = (point.x * math.cos(TILT) + point.z * math.sin(TILT)) / SCALE + WAIST
        blend = smooth(0.83, 0.87, z)
        if face and y < -0.004:
            eye = math.exp(-((abs(x) - 0.028) / 0.018) ** 4 - ((z - 0.913) / 0.012) ** 4)
            x -= (x - math.copysign(0.028, x)) * eye * 0.05
            z -= (z - 0.913) * eye * 0.05
        return position((x * (1 - 0.05 * blend), y * (1 - 0.05 * blend),
                         z - (z - 0.84) * 0.02 * blend))

    for obj in (bpy.data.objects["Sabrina_Surface"], bpy.data.objects["Sabrina_Groom"]):
        face_vertices = set()
        for polygon in obj.data.polygons:
            if obj.data.materials[polygon.material_index].name == "Sabrina_Face":
                face_vertices.update(polygon.vertices)
        if obj.data.shape_keys:
            for key in obj.data.shape_keys.key_blocks:
                for i, vertex in enumerate(key.data):
                    vertex.co = refine(vertex.co, i in face_vertices)
            for vertex, basis in zip(obj.data.vertices, obj.data.shape_keys.key_blocks[0].data):
                vertex.co = basis.co
        else:
            for vertex in obj.data.vertices:
                vertex.co = refine(vertex.co)
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for bone in rig.data.edit_bones:
        if bone.name in ("Neck", "Head") or bone.name.startswith("Hair_"):
            bone.head, bone.tail = refine(bone.head), refine(bone.tail)
    bpy.ops.object.mode_set(mode="OBJECT")
    rig["adult_head_refined"] = True


def repair_elbows(rig):
    """Preserve existing artwork/actions; add only elbow support weights and tracks."""
    previous_action = rig.animation_data.action
    previous_frame = bpy.context.scene.frame_current
    previous_subframe = bpy.context.scene.frame_subframe
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for side in ("L", "R"):
        forearm = rig.data.edit_bones[f"Forearm_{side}"]
        support = rig.data.edit_bones.get(f"Elbow_{side}") or rig.data.edit_bones.new(f"Elbow_{side}")
        support.head, support.tail, support.roll = forearm.head, forearm.tail, forearm.roll
        support.parent = forearm.parent
    bpy.ops.object.mode_set(mode="OBJECT")
    body = bpy.data.objects["Sabrina_Surface"]
    for side in ("L", "R"):
        name = f"Elbow_{side}"
        if name not in body.vertex_groups:
            body.vertex_groups.new(name=name)
    for vertex in body.data.vertices:
        x = vertex.co.y / SCALE
        if not 0.15 <= abs(x) <= 0.225:
            continue
        side = "L" if x >= 0 else "R"
        expected = {f"UpperArm_{side}", f"Forearm_{side}", f"Elbow_{side}"}
        if not any(body.vertex_groups[g.group].name in expected for g in vertex.groups):
            continue
        if not all(body.vertex_groups[g.group].name in expected for g in vertex.groups):
            raise RuntimeError(f"Unexpected non-arm weights at elbow vertex {vertex.index}")
        for group in list(vertex.groups):
            body.vertex_groups[group.group].remove([vertex.index])
        for name, weight in weights(Vector((x, 0, 0.795)), "4", {}).items():
            if weight > 0:
                body.vertex_groups[name].add([vertex.index], weight, "REPLACE")
    try:
        for name in CLIPS:
            if name not in bpy.data.actions:
                continue
            rig.animation_data.action = bpy.data.actions[name]
            start, end = rig.animation_data.action.frame_range
            for frame in range(int(start), int(end) + 1):
                bpy.context.scene.frame_set(frame)
                for side in ("L", "R"):
                    support = rig.pose.bones[f"Elbow_{side}"]
                    support.rotation_mode = "QUATERNION"
                    support.rotation_quaternion = Quaternion().slerp(
                        rig.pose.bones[f"Forearm_{side}"].rotation_quaternion, 0.5
                    )
                    support.keyframe_insert("rotation_quaternion", frame=frame, group=support.name)
            for layer in rig.animation_data.action.layers:
                for strip in layer.strips:
                    for bag in strip.channelbags:
                        for curve in bag.fcurves:
                            if curve.data_path.startswith('pose.bones["Elbow_'):
                                for key in curve.keyframe_points:
                                    key.interpolation = "LINEAR"
    finally:
        rig.animation_data.action = previous_action
        bpy.context.scene.frame_set(previous_frame, subframe=previous_subframe)


def main():
    arguments = sys.argv[sys.argv.index("--") + 1:]
    if arguments == ["--swimming-poses"]:
        rigs = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
        if len(rigs) != 1 or rigs[0].get("asset_id") != "sabrina" or not bpy.data.filepath:
            raise RuntimeError("Open the maintained Sabrina source before updating swimming poses")
        animate(rigs[0], ("Swim", "Turn_Left", "Turn_Right", "Dive"))
        bpy.ops.wm.save_as_mainfile(filepath=bpy.data.filepath)
        return
    if arguments == ["--repair-elbows"]:
        rigs = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
        if len(rigs) != 1 or rigs[0].get("asset_id") != "sabrina" or not bpy.data.filepath:
            raise RuntimeError("Open the maintained Sabrina source before repairing elbows")
        repair_elbows(rigs[0])
        bpy.ops.wm.save_as_mainfile(filepath=bpy.data.filepath)
        return
    if arguments in (["--actions-only"], ["--repair-arms"], ["--refine-silhouette"]):
        rigs = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
        if len(rigs) != 1 or rigs[0].get("asset_id") != "sabrina" or not bpy.data.filepath:
            raise RuntimeError("Open the maintained Sabrina source before replacing its game actions")
        if arguments == ["--repair-arms"]:
            rig = rigs[0]
            bpy.ops.object.select_all(action="DESELECT")
            rig.select_set(True)
            bpy.context.view_layer.objects.active = rig
            bpy.ops.object.mode_set(mode="EDIT")
            for side, sign in (("L", 1), ("R", -1)):
                for name, head, tail, parent in upper_bones(side, sign):
                    bone = rig.data.edit_bones.get(name) or rig.data.edit_bones.new(name)
                    bone.head, bone.tail = position(head), position(tail)
                    bone.parent = rig.data.edit_bones[parent]
                    bone.align_roll(Vector((0, 1, 0)))
            bpy.ops.object.mode_set(mode="OBJECT")
            body = bpy.data.objects["Sabrina_Surface"]
            parts = {}
            for polygon in body.data.polygons:
                material = body.data.materials[polygon.material_index].name
                if material in ("Sabrina_Part_3", "Sabrina_Part_4", "Sabrina_Part_5"):
                    for index in polygon.vertices:
                        parts[index] = material[-1]
            for index, part in parts.items():
                vertex = body.data.vertices[index]
                x = vertex.co.y / SCALE
                if abs(x) > 0.305:
                    continue
                p = Vector((x, (-vertex.co.x * math.sin(TILT) + vertex.co.z * math.cos(TILT)) / SCALE,
                            (vertex.co.x * math.cos(TILT) + vertex.co.z * math.sin(TILT)) / SCALE + WAIST))
                if p.z < WAIST:
                    p.z = WAIST + (p.z - WAIST) / rigs[0].get("tail_length_scale", 1.0)
                for group in list(vertex.groups):
                    body.vertex_groups[group.group].remove([index])
                for name, weight in weights(p, part, {}).items():
                    if weight > 0:
                        group = body.vertex_groups.get(name) or body.vertex_groups.new(name=name)
                        group.add([index], weight, "REPLACE")
        if arguments == ["--refine-silhouette"]:
            lengthen_tail(rigs[0])
            refine_head(rigs[0])
        animate(rigs[0])
        bpy.ops.wm.save_as_mainfile(filepath=bpy.data.filepath)
        return
    source, output = [Path(p).resolve() for p in arguments]
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(source))
    parts = {int(o.name.split("_")[-1]): o for o in bpy.context.scene.objects
             if o.type == "MESH" and o.name.startswith("tripo_part_")}
    if set(parts) != set(range(11)):
        raise RuntimeError("Expected the inspected eleven-part textured T-pose mermaid")
    imported = next(o for o in bpy.context.scene.objects if o.type == "ARMATURE")
    landmarks = {
        b.name: (imported.matrix_world @ b.head_local, imported.matrix_world @ b.tail_local)
        for b in imported.data.bones
    }
    for obj in parts.values():
        transform = obj.matrix_world.copy()
        obj.parent = None
        obj.matrix_world = transform
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    for obj in list(bpy.data.objects):
        if obj not in parts.values():
            bpy.data.objects.remove(obj, do_unlink=True)
    for action in list(bpy.data.actions):
        bpy.data.actions.remove(action)
    face, hair = split_head(parts.pop(0))
    hair_mat = hair.data.materials[0].copy()
    hair_mat.name = "Sabrina_Hair"
    hair.data.materials[0] = hair_mat
    face.data.materials[0].name = "Sabrina_Face"
    for i, obj in parts.items():
        obj.data.materials[0].name = f"Sabrina_Part_{i}"
    rig = build_rig(landmarks)
    body_objects = []
    for i, obj in parts.items():
        prepare_mesh(obj, str(i), rig)
        body_objects.append(obj)
    face_original = prepare_mesh(face, "face", rig)
    prepare_mesh(hair, "hair", rig)
    body_objects.append(face)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in body_objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = face
    bpy.ops.object.join()
    body = bpy.context.object
    body.name = "Sabrina_Surface"
    body.shape_key_add(name="Basis")
    for name in ("Blink", "SoftSmile", "Singing"):
        key = body.shape_key_add(name=name)
        for i, p in enumerate(face_original):
            x, _, z = p
            delta = Vector()
            if name == "Blink":
                influence = math.exp(-((abs(x) - 0.028) / 0.018) ** 4 - ((z - 0.913) / 0.009) ** 2)
                delta.z = (0.913 - z) * influence * 0.8
            elif name == "SoftSmile":
                influence = math.exp(-(x / 0.025) ** 4 - ((z - 0.866) / 0.008) ** 2)
                delta.z = influence * abs(x) * 0.07
            else:
                influence = math.exp(-(x / 0.019) ** 4 - ((z - 0.866) / 0.008) ** 2)
                delta.z = (z - 0.866) * influence * 0.55
            key.data[i].co += position(p + delta) - position(p)
        key.value = 0
    lengthen_tail(rig)
    refine_head(rig)
    animate(rig)
    bpy.context.scene.render.fps = 30
    bpy.context.scene.frame_start, bpy.context.scene.frame_end = 0, 96
    rig["source_name"] = source.name
    rig["source_sha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
    bpy.context.scene["authoring_notes"] = (
        "Original textured Tripo topology and UVs; uniform game-space transform and 12% longer tail. "
        "No human legs: six tail joints, independent flukes, four paired small-fin joints, "
        "thirty finger joints, five five-joint hair chains. Hair dynamics run in the game."
    )
    for image in bpy.data.images:
        if image.has_data:
            image.pack()
    bpy.context.preferences.filepaths.save_version = 0
    output.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(output))
    print("RIGGED SABRINA", output, "bones", len(rig.data.bones), "clips", len(CLIPS))


if __name__ == "__main__":
    main()
