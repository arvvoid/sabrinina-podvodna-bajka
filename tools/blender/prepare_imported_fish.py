# SPDX-License-Identifier: Apache-2.0
"""Separate the owner's twelve-fish collection, bake compact maps and author swim rigs.
Source-specific, destructive rebuild of the generated sources; use export_character.py
for subsequent artist edits. The supplied GLB is never modified.
"""

import sys, math, json, hashlib
from pathlib import Path
import bpy, bmesh
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
from geometry import coord, chain_weights
from rigging import armature, actions, rotate

SOURCE_SHA = "c6aec1183f5d02a0baabb835f2e140e5bb1b14923acd1c63c54060f509ab79db"
# Source sheet centers (Blender X/Z), independent of imported object ordering.
FISH = [
    ("clownfish", -0.346, 0.616),
    ("blue_tang", -0.005, 0.622),
    ("butterflyfish", 0.026, 0.392),
    ("bannerfish", 0.344, 0.358),
    ("parrotfish", -0.375, 0.207),
    ("blue_wrasse", -0.116, 0.205),
    ("fire_goby", -0.311, 0.085),
    ("royal_tang", 0.360, 0.098),
    ("yellow_tang", 0.328, 0.616),
    ("emperor_angelfish", -0.311, 0.395),
    ("coral_anthias", 0.156, 0.206),
    ("cardinalfish", 0.074, 0.102),
]


def separate(source, cache):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(source))
    obj = next(o for o in bpy.context.scene.objects if o.type == "MESH")
    bpy.context.view_layer.objects.active = obj
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=5e-7)
    bm.to_mesh(obj.data)
    bm.free()
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.separate(type="LOOSE")
    bpy.ops.object.mode_set(mode="OBJECT")
    objects = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    if len(objects) != 12:
        raise RuntimeError("Expected twelve connected fish in this exact source")
    for name, x, z in FISH:

        def distance(o):
            c = (
                np.min([v.co[:] for v in o.data.vertices], axis=0)
                + np.max([v.co[:] for v in o.data.vertices], axis=0)
            ) / 2
            return (c[0] - x) ** 2 + (c[2] - z) ** 2

        obj = min(objects, key=distance)
        objects.remove(obj)
        obj.name = name
        bpy.context.view_layer.objects.active = obj
        mod = obj.modifiers.new("Game_budget", "DECIMATE")
        mod.ratio = 7800 / len(obj.data.polygons)
        mod.use_collapse_triangulate = True
        bpy.ops.object.modifier_apply(modifier=mod.name)
        obj.data.normals_split_custom_set([(0, 0, 0)] * len(obj.data.loops))
        for f in obj.data.polygons:
            f.use_smooth = True
    bpy.ops.wm.save_as_mainfile(filepath=str(cache))


def bake(obj, name):
    material = obj.data.materials[0].copy()
    obj.data.materials[0] = material
    ns = material.node_tree.nodes
    ls = material.node_tree.links
    bs = next(n for n in ns if n.type == "BSDF_PRINCIPLED")
    out = next(n for n in ns if n.type == "OUTPUT_MATERIAL")
    old_uv = obj.data.uv_layers.active.name
    uv = ns.new("ShaderNodeUVMap")
    uv.uv_map = old_uv
    for node in list(ns):
        if node.type == "TEX_IMAGE":
            ls.new(uv.outputs["UV"], node.inputs["Vector"])
        if node.type == "NORMAL_MAP":
            node.uv_map = old_uv
            node.inputs["Strength"].default_value = 0.55
    color = bs.inputs["Base Color"].links[0].from_socket
    obj.data.uv_layers.new(name="GameUV")
    obj.data.uv_layers.active_index = 1
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(76), island_margin=0.016)
    bpy.ops.object.mode_set(mode="OBJECT")
    obj.data.uv_layers.active.active_render = True
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 4
    scene.render.threads_mode = "FIXED"
    scene.render.threads = 8
    scene.render.bake.margin = 4
    target = ns.new("ShaderNodeTexImage")
    emit = ns.new("ShaderNodeEmission")
    maps = {}
    for kind in ["Color", "Normal"]:
        img = bpy.data.images.new(name + "_" + kind, width=512, height=512, alpha=False)
        img.colorspace_settings.name = "sRGB" if kind == "Color" else "Non-Color"
        target.image = img
        ns.active = target
        if kind == "Color":
            ls.new(color, emit.inputs[0])
            ls.new(emit.outputs[0], out.inputs[0])
            bpy.ops.object.bake(type="EMIT")
        else:
            ls.new(bs.outputs[0], out.inputs[0])
            bpy.ops.object.bake(type="NORMAL")
        img.pack()
        maps[kind] = img
    mat = bpy.data.materials.new(name + "_Pearl")
    mat.use_nodes = True
    ns = mat.node_tree.nodes
    ls = mat.node_tree.links
    bs = ns.get("Principled BSDF")
    bs.inputs["Roughness"].default_value = 0.48
    bs.inputs["Metallic"].default_value = 0.08
    for kind, img in maps.items():
        node = ns.new("ShaderNodeTexImage")
        node.image = img
        if kind == "Color":
            ls.new(node.outputs["Color"], bs.inputs["Base Color"])
        else:
            normal = ns.new("ShaderNodeNormalMap")
            normal.inputs["Strength"].default_value = 0.65
            ls.new(node.outputs["Color"], normal.inputs["Color"])
            ls.new(normal.outputs[0], bs.inputs["Normal"])
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    obj.data.uv_layers.remove(obj.data.uv_layers[old_uv])
    white = obj.data.color_attributes.new(
        name="Color", type="FLOAT_COLOR", domain="POINT"
    )
    white.data.foreach_set("color", np.ones(len(white.data) * 4, dtype=np.float32))
    # Purge atlas and other unused imported data before saving each self-contained source.
    for m in list(bpy.data.materials):
        if m.users == 0:
            bpy.data.materials.remove(m)
    for img in list(bpy.data.images):
        if img.users == 0:
            bpy.data.images.remove(img)


def rig_fish(obj, name):
    positions = np.array([v.co[:] for v in obj.data.vertices])
    lo = positions.min(axis=0)
    hi = positions.max(axis=0)
    center = (lo + hi) / 2
    scale = 1.80 / (hi[0] - lo[0])
    # Nose +.68, tail -1.12: preserve existing contact/size convention.
    for v in obj.data.vertices:
        x, y, z = v.co
        v.co = coord(((x - hi[0]) * scale + 0.68, (z - center[2]) * scale, -y * scale))
    depth = max(abs(v.co.y) for v in obj.data.vertices)
    bones = [
        ("Root", (0.35, 0, 0), (0.65, 0, 0), None),
        ("Spine", (0.20, 0, 0), (-0.24, 0, 0), "Root"),
        ("Peduncle", (-0.24, 0, 0), (-0.64, 0, 0), "Spine"),
        ("Caudal", (-0.64, 0, 0), (-1.10, 0, 0), "Peduncle"),
        ("Fin_L", (0.08, -0.05, depth * 0.65), (-0.2, -0.18, depth * 1.12), "Spine"),
        ("Fin_R", (0.08, -0.05, -depth * 0.65), (-0.2, -0.18, -depth * 1.12), "Spine"),
    ]
    rig = armature(name + "_Rig", bones)
    groups = {b: obj.vertex_groups.new(name=b) for b, *_ in bones}
    for v in obj.data.vertices:
        x, neg_z, y = v.co
        z = -neg_z
        w = chain_weights(
            x,
            [(-0.77, "Caudal"), (-0.41, "Peduncle"), (-0.02, "Spine"), (0.32, "Root")],
        )
        # Pectoral tips emerge from the flank. Soft taper keeps the attached body intact.
        fin = (
            max(0, min(1, (abs(z) / depth - 0.72) / 0.25))
            * max(0, 1 - abs((x + 0.03) / 0.42))
            * max(0, 1 - abs((y + 0.08) / 0.30))
        )
        if fin > 0:
            w = {k: a * (1 - fin * 0.75) for k, a in w.items()}
            w["Fin_L" if z > 0 else "Fin_R"] = fin * 0.75
        w = dict(sorted(w.items(), key=lambda p: p[1], reverse=True)[:4])
        total = sum(w.values())
        for key, value in w.items():
            groups[key].add([v.index], value / total, "REPLACE")
    obj.modifiers.new("Fish_skin", "ARMATURE").object = rig
    obj.parent = rig

    def pose(clip, u, r):
        t = math.tau * u
        e = 1 if clip == "Swim" else 0.42
        # Yaw wave travels from a quiet head to the tail; delayed fan gives a propulsive stroke.
        rotate(r, "Root", (0, -0.025 * e * math.sin(t), 0.012 * math.sin(t)))
        rotate(r, "Spine", (0, 0.09 * e * math.sin(t - 0.35), 0.012 * e * math.sin(t)))
        rotate(
            r,
            "Peduncle",
            (0, 0.24 * e * math.sin(t - 0.95), 0.018 * e * math.sin(t - 0.7)),
        )
        rotate(
            r,
            "Caudal",
            (0, 0.38 * e * math.sin(t - 1.6), 0.025 * e * math.sin(t - 1.2)),
        )
        for side in [-1, 1]:
            rotate(
                r,
                "Fin_L" if side > 0 else "Fin_R",
                (
                    0.07 * math.sin(t + side * 0.5),
                    side * (0.10 + 0.18 * math.sin(2 * t + side * 0.5)),
                    0.04 * math.cos(t),
                ),
            )

    actions(rig, [("Idle", 2.4), ("Swim", 1.2)], pose)
    rig["asset_id"] = name
    rig["source_collection"] = "tropical fish 3d model.glb"
    rig["animation_contract"] = "In-place, +X forward; native simulation owns position."
    return rig


def main():
    source = Path(sys.argv[sys.argv.index("--") + 1]).resolve()
    if hashlib.sha256(source.read_bytes()).hexdigest() != SOURCE_SHA:
        raise RuntimeError("Source does not match this collection")
    cache = ROOT / "build/character-previews/fish-separated.blend"
    cache.parent.mkdir(parents=True, exist_ok=True)
    separate(source, cache)
    for name, *_ in FISH:
        bpy.ops.wm.open_mainfile(filepath=str(cache))
        obj = bpy.data.objects[name]
        for other in list(bpy.data.objects):
            if other != obj:
                bpy.data.objects.remove(other, do_unlink=True)
        for mesh in list(bpy.data.meshes):
            if mesh.users == 0:
                bpy.data.meshes.remove(mesh)
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bake(obj, name)
        rig = rig_fish(obj, name)
        bpy.context.scene.render.fps = 30
        bpy.ops.object.select_all(action="DESELECT")
        rig.select_set(True)
        bpy.context.view_layer.objects.active = rig
        bpy.ops.wm.save_as_mainfile(
            filepath=str(ROOT / "art/characters" / f"{name}.blend")
        )
        exporter = ROOT / "tools/blender/export_character.py"
        exec(
            compile(exporter.read_text(), str(exporter), "exec"),
            {"__file__": str(exporter)},
        )
        print("FISH_READY", name, flush=True)
    (ROOT / "art/characters/fish.provenance.json").write_text(
        json.dumps(
            {
                "source_name": source.name,
                "sha256": SOURCE_SHA,
                "provided_by": "Project owner",
                "source_generator_metadata": "Tripo",
                "source_triangles": 1883328,
                "adaptation": "Separated twelve connected fish, reduced to 7800 triangles each, baked 512px color/normal maps, normalized size, weighted six-bone rigs and authored idle/swim loops. Legacy parrotfish/blue_wrasse slot names contain the supplied pink anthias/cyan chromis.",
                "assets": [f[0] for f in FISH],
                "license": "User-supplied artwork; reserved separately from Apache-2.0 source. See ASSET_LICENSE.md.",
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
