# SPDX-License-Identifier: Apache-2.0
"""Export the opened, artist-edited .blend; never regenerate its geometry or keys.
Usage: blender --background art/characters/sabrina.blend --python tools/blender/export_character.py
"""

import bpy, json, sys
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
output = ROOT / "game/assets/models" / f"{asset}.glb"
bpy.ops.export_scene.gltf(
    filepath=str(output),
    export_format="GLB",
    use_selection=True,
    export_animations=True,
    export_animation_mode="ACTIONS",
    export_anim_single_armature=True,
    export_skins=True,
    export_def_bones=True,
    export_force_sampling=True,
    export_frame_range=False,
    export_frame_step=1,
    export_morph=False,
    export_extras=True,
    export_yup=True,
    export_materials="EXPORT",
    export_cameras=False,
    export_lights=False,
)
manifest = ROOT / "art/characters/manifest.json"
data = json.loads(manifest.read_text())
record = {
    "asset": asset,
    "source": str(Path(bpy.data.filepath).resolve().relative_to(ROOT)),
    "export": str(output.relative_to(ROOT)),
    "bones": len(rig.data.bones),
    "vertices": sum(len(o.data.vertices) for o in objects),
    "triangles": sum(
        sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objects
    ),
    "surfaces": len(objects),
    "clips": [a.name for a in bpy.data.actions],
    "bytes": output.stat().st_size,
}
data["assets"] = [r for r in data["assets"] if r["asset"] != asset] + [record]
manifest.write_text(json.dumps(data, indent=2) + "\n")
print("EXPORTED ARTIST SOURCE", output)

if asset == "sabrina":
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from export_sabrina_contacts import export_contacts

    export_contacts(rig)
