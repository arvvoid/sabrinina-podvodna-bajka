# SPDX-License-Identifier: MIT
"""Export the saved, artist-edited reef without rebuilding geometry or textures."""

import json

import bpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
objects = [o for o in bpy.context.scene.objects if o.type == "MESH"]
if not objects:
    raise RuntimeError("No environment meshes found")
bpy.ops.object.select_all(action="DESELECT")
for obj in objects:
    obj.select_set(True)
output = ROOT / "assets/environments/reef_ruins.glb"
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
manifest = ROOT / "art/environments/manifest.json"
data = json.loads(manifest.read_text())
data.update(
    source=str(Path(bpy.data.filepath).resolve().relative_to(ROOT)),
    triangles=sum(sum(len(f.vertices) - 2 for f in o.data.polygons) for o in objects),
    vertices=sum(len(o.data.vertices) for o in objects),
    surfaces=sum(len(o.data.materials) for o in objects),
    bytes=output.stat().st_size,
)
manifest.write_text(json.dumps(data, indent=2) + "\n")
print("EXPORTED ARTIST ENVIRONMENT", output)
