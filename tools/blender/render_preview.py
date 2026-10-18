# SPDX-License-Identifier: MIT
"""Render an honest preview of a saved asset, without changing its source."""

import bpy, sys
from pathlib import Path
from mathutils import Vector

args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
output = args[0]
frame = int(args[1]) if len(args) > 1 else 20
clip = args[2] if len(args) > 2 else "Idle"
scene = bpy.context.scene
rig = next(o for o in scene.objects if o.type == "ARMATURE")
if clip in bpy.data.actions:
    rig.animation_data.action = bpy.data.actions[clip]
scene.frame_set(frame)
# Compute evaluated bounds, so the render reflects the deformed model.
points = []
for obj in scene.objects:
    if obj.type == "MESH":
        points.extend(obj.matrix_world @ Vector(c) for c in obj.bound_box)
lo = Vector(tuple(min(p[i] for p in points) for i in range(3)))
hi = Vector(tuple(max(p[i] for p in points) for i in range(3)))
center = (lo + hi) / 2
height = max(hi.z - lo.z, (hi.x - lo.x))
world = scene.world or bpy.data.worlds.new("Asset_preview")
scene.world = world
world.use_nodes = True
world.node_tree.nodes.get("Background").inputs[0].default_value = (
    0.035,
    0.075,
    0.09,
    1,
)
world.node_tree.nodes.get("Background").inputs[1].default_value = 0.45
bpy.ops.object.camera_add(
    location=center + Vector((height * 0.45, -height * 2.4, height * 0.25))
)
cam = bpy.context.object
cam.rotation_euler = (center - cam.location).to_track_quat("-Z", "Y").to_euler()
cam.data.type = "ORTHO"
cam.data.ortho_scale = height * 1.22
scene.camera = cam
for name, location, power, color, size in [
    ("Key", (-3, -5, 6), 650, (1, 0.88, 0.70), 5),
    ("Fill", (4, -3, 2), 450, (0.45, 0.79, 1), 4),
    ("Rim", (-2, 3, 4), 850, (0.36, 0.95, 1), 3),
]:
    bpy.ops.object.light_add(type="AREA", location=center + Vector(location))
    light = bpy.context.object
    light.name = name
    light.data.energy = power
    light.data.color = color
    light.data.shape = "DISK"
    light.data.size = size
    light.rotation_euler = (center - light.location).to_track_quat("-Z", "Y").to_euler()
scene.render.engine = "CYCLES"
scene.cycles.samples = 24
scene.cycles.use_denoising = True
scene.render.threads_mode = "FIXED"
scene.render.threads = 8
scene.render.resolution_x = 1000
scene.render.resolution_y = 1000
scene.render.resolution_percentage = 100
scene.view_settings.view_transform = "AgX"
scene.render.image_settings.file_format = "PNG"
scene.render.filepath = output
bpy.ops.render.render(write_still=True)
