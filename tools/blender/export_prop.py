# SPDX-License-Identifier: MIT
"""Export the saved crystal, preserving its geometry and packed textures."""

from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[2]
objects = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
if not objects:
    raise RuntimeError("No prop meshes found")
bpy.ops.object.select_all(action="DESELECT")
for obj in objects:
    obj.select_set(True)
output = ROOT / "assets/props/ocean_crystal.glb"
bpy.ops.export_scene.gltf(
    filepath=str(output),
    export_format="GLB",
    use_selection=True,
    export_animations=False,
    export_yup=True,
    export_materials="EXPORT",
    export_extras=True,
    export_cameras=False,
    export_lights=False,
)
print("Exported", output)
