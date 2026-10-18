# SPDX-License-Identifier: MIT
"""Export the saved crystal, preserving its geometry and packed textures."""

import json
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
manifest = ROOT / "art/props/manifest.json"
data = json.loads(manifest.read_text())
data.update(
    source=str(Path(bpy.data.filepath).resolve().relative_to(ROOT)),
    triangles=sum(
        sum(len(face.vertices) - 2 for face in obj.data.polygons) for obj in objects
    ),
    surfaces=sum(len(obj.data.materials) for obj in objects),
    bytes=output.stat().st_size,
)
manifest.write_text(json.dumps(data, indent=2) + "\n")
print("Exported", output)
