# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Export the opened, artist-edited .blend; never regenerate its geometry or keys.
Usage: blender --background art/characters/sabrina.blend --python tools/blender/export_character.py
"""

import sys

import bpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
rigs = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
if len(rigs) != 1:
    raise RuntimeError("Expected one character armature in this source file.")
rig = rigs[0]
asset = rig.get("asset_id")
if not asset or not asset.replace("_", "").isalnum():
    raise RuntimeError("Armature needs a valid asset_id property.")
objects = [
    o
    for o in bpy.context.scene.objects
    if o.type == "MESH"
    and any(m.type == "ARMATURE" and m.object == rig for m in o.modifiers)
]
if not objects:
    raise RuntimeError("No skinned meshes found.")
bpy.ops.object.select_all(action="DESELECT")
rig.select_set(True)
for obj in objects:
    obj.select_set(True)
bpy.context.view_layer.objects.active = rig
output = ROOT / "assets/models" / f"{asset}.glb"
bpy.ops.export_scene.gltf(
    filepath=str(output),
    export_format="GLB",
    use_selection=True,
    export_animations=True,
    export_animation_mode="ACTIONS",
    export_anim_single_armature=True,
    export_skins=True,
    export_tangents=asset == "sabrina",
    export_optimize_animation_keep_anim_armature=asset != "sabrina",
    export_def_bones=True,
    export_force_sampling=True,
    export_frame_range=False,
    export_frame_step=1,
    export_morph=True,
    export_extras=True,
    export_yup=True,
    export_materials="EXPORT",
    export_cameras=False,
    export_lights=False,
)
print("EXPORTED ARTIST SOURCE", output)

if asset == "sabrina":
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from export_sabrina_contacts import export_contacts

    export_contacts(rig)
