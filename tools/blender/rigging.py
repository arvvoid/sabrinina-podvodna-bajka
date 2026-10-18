# SPDX-License-Identifier: MIT
import bpy
from mathutils import Quaternion
from geometry import coord


def armature(name, bones):
    data = bpy.data.armatures.new(name)
    rig = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    for name, head, tail, parent in bones:
        bone = data.edit_bones.new(name)
        bone.head = coord(head)
        bone.tail = coord(tail)
        if parent:
            bone.parent = data.edit_bones[parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    rig.show_in_front = True
    data.display_type = "BBONE"
    for pb in rig.pose.bones:
        pb.rotation_mode = "QUATERNION"
    return rig


def rotate(rig, name, angles):
    # Rotation axes expressed in the character's authored game space, converted to bone space.
    bone = rig.pose.bones[name]
    inverse = bone.bone.matrix_local.to_3x3().inverted()
    q = Quaternion()
    for axis, angle in zip(((1, 0, 0), (0, 1, 0), (0, 0, 1)), angles):
        q = q @ Quaternion(inverse @ coord(axis), angle)
    bone.rotation_quaternion = q


def actions(rig, specs, pose):
    rig.animation_data_create()
    clips = []
    for name, duration in specs:
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        rig.animation_data.action = action
        frames = round(duration * 30)
        previous = {}
        for frame in range(0, frames + 1, 2):
            bpy.context.scene.frame_set(frame)
            for bone in rig.pose.bones:
                bone.rotation_quaternion = Quaternion()
                bone.location = (0, 0, 0)
                bone.scale = (1, 1, 1)
            pose(name, frame / frames, rig)
            for bone in rig.pose.bones:
                # q and -q describe the same pose, but component F-curves between
                # opposite signs pass through zero and produce an unwanted flip.
                q = bone.rotation_quaternion.copy()
                if bone.name in previous and q.dot(previous[bone.name]) < 0:
                    q.negate()
                    bone.rotation_quaternion = q
                previous[bone.name] = q
                bone.keyframe_insert(
                    "rotation_quaternion", frame=frame, group=bone.name
                )
                bone.keyframe_insert("location", frame=frame, group=bone.name)
                bone.keyframe_insert("scale", frame=frame, group=bone.name)
        action["loop"] = True
        action["description"] = (
            "Editable pose keys; in-place motion, gameplay controls world movement."
        )
        clips.append(action)
    rig.animation_data.action = clips[0]
    bpy.context.scene.frame_start = 0
    bpy.context.scene.frame_end = round(specs[0][1] * 30)
    bpy.context.scene.frame_set(0)
    return clips
