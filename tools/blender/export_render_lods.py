# SPDX-License-Identifier: MIT
"""Derive texture-free render LODs from maintained sources; never edit the sources.
Run: blender --background --python tools/blender/export_render_lods.py
The regular GLBs keep textures, skeletons and animations. These smaller meshes
reuse their material and skin handles at runtime.
"""

import hashlib
import json

import bpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
records = []
characters = json.loads((ROOT / "art/characters/manifest.json").read_text())["assets"]
sources = [r for r in characters if r["asset"] not in ["sabrina", "jellyfish"]]
sources.append(json.loads((ROOT / "art/environments/manifest.json").read_text()))
for record in sources:
    asset = record["asset"]
    bpy.ops.wm.open_mainfile(filepath=str(ROOT / record["source"]))
    rigs = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
    for rig in rigs:
        rig.animation_data_clear()
        rig.data.pose_position = "REST"
    sources_mesh = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    copies = []
    counts = {}
    for source in sources_mesh:
        source_name = source.data.name
        base_triangles = sum(len(p.vertices) - 2 for p in source.data.polygons)
        for level, ratio in [(1, 0.30 if rigs else 0.40), (2, 0.12 if rigs else 0.16)]:
            obj = source.copy()
            obj.data = source.data.copy()
            obj.name = source.name + "__LOD" + str(level)
            obj.data.name = source_name + "__LOD" + str(level)
            bpy.context.collection.objects.link(obj)
            for modifier in list(obj.modifiers):
                obj.modifiers.remove(modifier)
            bpy.ops.object.select_all(action="DESELECT")
            obj.select_set(True)
            bpy.context.view_layer.objects.active = obj
            decimate = obj.modifiers.new("Screen-sized geometry", "DECIMATE")
            decimate.ratio = ratio
            bpy.ops.object.modifier_apply(modifier=decimate.name)
            if rigs:
                bpy.ops.object.vertex_group_limit_total(limit=4)
                bpy.ops.object.vertex_group_normalize_all(lock_active=False)
                obj.modifiers.new("Shared skin", "ARMATURE").object = rigs[0]
            copies.append(obj)
            counts[obj.data.name] = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in copies + rigs:
        obj.select_set(True)
    output = ROOT / "assets/render_lods" / (asset + ".glb")
    output.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(output),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_skins=True,
        export_def_bones=True,
        export_morph=False,
        export_yup=True,
        export_materials="NONE",
        export_texcoords=True,
        export_cameras=False,
        export_lights=False,
    )
    records.append(
        {
            "asset": asset,
            "source": record["source"],
            "base_export": record["export"],
            "base_sha256": hashlib.sha256(
                (ROOT / record["export"]).read_bytes()
            ).hexdigest(),
            "export": str(output.relative_to(ROOT)),
            "triangles": counts,
        }
    )
    print("LOD EXPORTED", asset, counts, flush=True)
(ROOT / "assets/render_lods/manifest.json").write_text(
    json.dumps(records, indent=2) + "\n"
)
