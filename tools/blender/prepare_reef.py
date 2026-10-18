# SPDX-License-Identifier: Apache-2.0
"""Adapt the owner's ruins diorama to the game's side-on swimming plane.
blender --background --python tools/blender/prepare_reef.py -- source.glb
The downloaded high-resolution source is never modified.
"""

import sys, json, hashlib
from pathlib import Path
import bpy, bmesh

sys.path.insert(0, str(Path(__file__).resolve().parent))
from reef_depth import layer_foreground

ROOT = Path(__file__).resolve().parents[2]
source = Path(sys.argv[sys.argv.index("--") + 1]).resolve()
if (
    hashlib.sha256(source.read_bytes()).hexdigest()
    != "0ff474785ded13c06d40bac9680d8c3504d50bf9ee00cf54ee6930069d9a6ac4"
):
    raise RuntimeError(
        "This preparation is specific to the supplied ruins source; adapt the crop for another model."
    )
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(source))
objects = [o for o in bpy.context.scene.objects if o.type == "MESH"]
if len(objects) != 1:
    raise RuntimeError(
        "This preparation is specific to the supplied one-mesh ruins diorama."
    )
obj = objects[0]
bpy.context.view_layer.objects.active = obj
obj.select_set(True)
bm = bmesh.new()
bm.from_mesh(obj.data)
# Remove the reconstructed blue backdrop with painted fish, and the frozen turtle/roof behind it.
remove = [
    v
    for v in bm.verts
    if (v.co.y > 0.30 and v.co.z > 0.055)
    or (0.09 < v.co.x < 0.29 and v.co.y > 0.07 and v.co.z > 0.27)
]
bmesh.ops.delete(bm, geom=remove, context="VERTS")
bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=0.0000005)
bm.to_mesh(obj.data)
bm.free()
mod = obj.modifiers.new("Reef_game_reduction", "DECIMATE")
mod.ratio = min(1, 150000 / len(obj.data.polygons))
mod.use_collapse_triangulate = True
bpy.ops.object.modifier_apply(modifier=mod.name)
# Convert the diorama into a broad stage behind the actors, with a gently rising floor.
# Blender -Y faces the camera; exported Godot +Z does too.
for v in obj.data.vertices:
    x, y, z = v.co
    depth = y + 0.48986
    v.co = (x * 44, 4 + depth * 28, z * 34 - 0.8 + depth * 2.5)
for f in obj.data.polygons:
    f.use_smooth = True
obj.name = "Reef_Ruins"
obj.data.name = "Reef_Ruins_Game_Mesh"
obj["asset_id"] = "reef_ruins"
obj["gameplay"] = "Decorative background behind the swimming plane; no collision."
layer_foreground(obj)
obj["source"] = "User-supplied underwater ruins 3d model.glb; see manifest.json"
obj.data.materials[0].name = "Reef_Stone_Coral"
for image in bpy.data.images:
    if image.size[0] > 2048:
        image.scale(2048, 2048)
    image.pack()
scene = bpy.context.scene
scene.world = bpy.data.worlds.new("Reef_preview")
scene.world.color = (0.06, 0.12, 0.15)
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type == "VIEW_3D":
            area.spaces.active.shading.type = "MATERIAL"
            area.spaces.active.region_3d.view_distance = 45
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / "art/environments/reef_ruins.blend"))
export = ROOT / "game/assets/environments/reef_ruins.glb"
bpy.ops.export_scene.gltf(
    filepath=str(export),
    export_format="GLB",
    export_animations=False,
    export_yup=True,
    export_materials="EXPORT",
    export_extras=True,
    export_cameras=False,
    export_lights=False,
)
manifest = {
    "asset": "reef_ruins",
    "source": "art/environments/reef_ruins.blend",
    "export": str(export.relative_to(ROOT)),
    "provided_source": source.name,
    "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
    "source_generator_metadata": "Tripo",
    "source_triangles": 1861166,
    "triangles": sum(
        len(f.vertices) - 2
        for o in bpy.context.scene.objects
        if o.type == "MESH"
        for f in o.data.polygons
    ),
    "vertices": sum(
        len(o.data.vertices) for o in bpy.context.scene.objects if o.type == "MESH"
    ),
    "surfaces": 2,
    "texture_max_dimension": 2048,
    "bytes": export.stat().st_size,
    "license": "Owner-supplied artwork; reserved separately from source code. See ASSET_LICENSE.md.",
    "adaptation": "Removed rear blue backdrop/painted shoal and frozen turtle region, reduced mesh, resized packed maps, arranged in depth with a lower-left foreground occlusion layer.",
}
(ROOT / "art/environments/manifest.json").write_text(
    json.dumps(manifest, indent=2) + "\n"
)
print("REEF READY", json.dumps(manifest), flush=True)
