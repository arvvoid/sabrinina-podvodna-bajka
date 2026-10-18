# SPDX-License-Identifier: MIT
"""Prepare the owner's relic without modifying the downloaded model.
blender -b --python tools/blender/prepare_crystal.py -- source.glb
"""

import bpy
import hashlib
import json
import sys
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]
source = Path(sys.argv[sys.argv.index("--") + 1]).resolve()
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(source))
objects = [o for o in bpy.context.scene.objects if o.type == "MESH"]
if len(objects) != 1:
    raise RuntimeError("Expected the supplied single-mesh crystal")
obj = objects[0]
bpy.context.view_layer.objects.active = obj
obj.select_set(True)
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
original_triangles = sum(len(f.vertices) - 2 for f in obj.data.polygons)
modifier = obj.modifiers.new("Portable relic mesh", "DECIMATE")
modifier.ratio = min(1, 14000 / original_triangles)
modifier.use_collapse_triangulate = True
bpy.ops.object.modifier_apply(modifier=modifier.name)
lo = Vector([min(v.co[i] for v in obj.data.vertices) for i in range(3)])
hi = Vector([max(v.co[i] for v in obj.data.vertices) for i in range(3)])
center = (lo + hi) * 0.5
for v in obj.data.vertices:
    v.co = (v.co - center) * (1.35 / (hi.z - lo.z))
obj.name = "Ocean_Crystal_Relic"
for image in bpy.data.images:
    if max(image.size) > 1024:
        image.scale(1024, 1024)
    image.pack()
for material in obj.data.materials:
    for node in material.node_tree.nodes:
        if node.type == "BSDF_PRINCIPLED":
            node.inputs["Roughness"].default_value = 0.24
            node.inputs["Metallic"].default_value = 0.28
(ROOT / "art/props").mkdir(exist_ok=True)
(ROOT / "game/assets/props").mkdir(exist_ok=True)
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / "art/props/ocean_crystal.blend"))
output = ROOT / "game/assets/props/ocean_crystal.glb"
bpy.ops.export_scene.gltf(
    filepath=str(output),
    export_format="GLB",
    use_selection=True,
    export_animations=False,
    export_yup=True,
)
record = {
    "asset": "ocean_crystal",
    "source": "art/props/ocean_crystal.blend",
    "export": str(output.relative_to(ROOT)),
    "provided_source": source.name,
    "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
    "source_triangles": original_triangles,
    "triangles": sum(len(f.vertices) - 2 for f in obj.data.polygons),
    "texture_max_dimension": 1024,
    "surfaces": 1,
    "bytes": output.stat().st_size,
    "adaptation": "Reduced textured GLB, preserved filigree and water ribbons, centered and scaled to 1.35 units. Untextured 8434-triangle FBX evaluated but not used.",
    "license": "Owner-supplied artwork reserved separately from MIT code; see ASSET_LICENSE.md.",
}
(ROOT / "art/props/manifest.json").write_text(json.dumps(record, indent=2) + "\n")
