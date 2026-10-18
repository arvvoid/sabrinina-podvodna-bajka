# SPDX-License-Identifier: MIT
"""Build a candidate from the owner's neutral-pose FBX, without replacing shipped art.

blender -b --python tools/blender/prepare_tpose_sabrina.py -- source.fbx output_dir
Subsequent edits belong in the saved .blend; export_character.py exports those edits.
"""

import colorsys
import hashlib
import heapq
import json
import math
import sys
from pathlib import Path

import bpy
import bmesh
import numpy as np
from mathutils import Matrix, Quaternion, Vector

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
from geometry import chain_weights, coord
from rigging import armature
from sabrina_animation import author_actions

SOURCE_SHA = "8ef9a1a2ede884c402ae1bf28640567d2d082179e0c4a3902991aa9365a64f7e"
SCALE = 4.8
HIP = Vector((0, 0.64, 0.055))
ORIENTATION = Matrix.Rotation(math.radians(-55), 3, "Z") @ Matrix.Rotation(
    math.radians(65), 3, "Y"
)
TAIL_HEIGHTS = [0.64, 0.56, 0.48, 0.40, 0.33, 0.26, 0.20]
TAIL = [
    Vector(((x - 0.25) * SCALE, (h - 0.14) * SCALE, (d - 0.02) * SCALE))
    for x, h, d in [
        (0.25, 0.14, 0.02),
        (0.15, 0.105, 0.04),
        (0.04, 0.078, 0.065),
        (-0.07, 0.07, 0.085),
        (-0.16, 0.09, 0.09),
        (-0.235, 0.125, 0.09),
        (-0.29, 0.15, 0.095),
    ]
]


def smooth(a, b, x):
    t = max(0.0, min(1.0, (x - a) / (b - a)))
    return t * t * (3 - 2 * t)


def upper(p):
    return ORIENTATION @ ((Vector(p) - HIP) * SCALE)


def tail_point(p):
    x, h, d = p
    if h < 0.20:
        f = (0.20 - h) / 0.20
        center = TAIL[-1] + Vector((-0.912 * f, 0, 0))
        return center + Vector((0, -x * 6.0, (d - 0.059) * SCALE))
    i = next((i for i in range(6) if h >= TAIL_HEIGHTS[i + 1]), 5)
    u = max(
        0.0, min(1.0, (TAIL_HEIGHTS[i] - h) / (TAIL_HEIGHTS[i] - TAIL_HEIGHTS[i + 1]))
    )
    center = TAIL[i].lerp(TAIL[i + 1], u)
    # Roll the broad fin into the viewing plane gradually, without a seam at the waist.
    roll = smooth(0.64, 0.20, h)
    lateral = (
        (ORIENTATION @ Vector((1, 0, 0))).lerp(Vector((0, -1, 0)), roll).normalized()
    )
    front = (ORIENTATION @ Vector((0, 0, 1))).lerp(Vector((0, 0, 1)), roll).normalized()
    return center + lateral * x * SCALE + front * (d - 0.055) * SCALE


def regions(obj):
    xyz = np.array([(v.co.x, v.co.z, -v.co.y) for v in obj.data.vertices])
    graph = [[] for _ in xyz]
    for edge in obj.data.edges:
        a, b = edge.vertices
        dist = float(np.linalg.norm(xyz[a] - xyz[b]))
        graph[a].append((b, dist))
        graph[b].append((a, dist))
    anchors = [
        [
            # Seed the entire hip surface and its side fins. Sparse front-only
            # seeds let the hair region spill onto the upper tail and stretch it.
            (0, 0.64, 0.11),
            (0, 0.64, 0.005),
            (0, 0.60, 0.105),
            (0, 0.60, 0.005),
            (0.05, 0.60, 0.055),
            (-0.05, 0.60, 0.055),
            (0.09, 0.57, 0.055),
            (-0.09, 0.57, 0.055),
            (0, 0.72, 0.10),
            (0, 0.79, 0.11),
            (0, 0.85, 0.09),
            (0, 0.91, 0.12),
            (0, 0.97, 0.07),
            (0.065, 0.80, 0.07),
            (-0.065, 0.80, 0.07),
            (0, 0.45, 0.09),
            (0, 0.25, 0.06),
            (0, 0.1, 0.06),
        ],
        [
            (0, 0.96, -0.04),
            (0, 0.86, -0.11),
            (0, 0.73, -0.11),
            (0.085, 0.69, -0.03),
            (-0.085, 0.69, -0.03),
        ],
        [
            (0.13, 0.818, 0.05),
            (0.20, 0.798, 0.05),
            (0.29, 0.785, 0.06),
            (0.33, 0.776, 0.06),
        ],
        [
            (-0.13, 0.818, 0.05),
            (-0.20, 0.798, 0.05),
            (-0.29, 0.785, 0.06),
            (-0.33, 0.776, 0.06),
        ],
    ]
    fields = []
    for group in anchors:
        distances = np.full(len(xyz), np.inf)
        queue = []
        for point in group:
            index = int(np.argmin(np.sum((xyz - np.array(point)) ** 2, axis=1)))
            distances[index] = 0
            heapq.heappush(queue, (0, index))
        while queue:
            value, index = heapq.heappop(queue)
            if value > distances[index]:
                continue
            for other, length in graph[index]:
                candidate = value + length
                if candidate < distances[other]:
                    distances[other] = candidate
                    heapq.heappush(queue, (candidate, other))
        fields.append(distances)
    distances = np.array(fields).T
    # Disconnected jewelry/strands use spatial seeds, not an arbitrary torso attachment.
    for i in np.where(~np.isfinite(distances).any(axis=1))[0]:
        distances[i] = [
            min(np.linalg.norm(xyz[i] - np.array(p)) for p in group)
            for group in anchors
        ]
    scores = np.exp(-(distances - np.min(distances, axis=1)[:, None]) / 0.012)
    scores[:, 1] *= np.clip((xyz[:, 1] - 0.55) / 0.06, 0, 1)
    scores[:, 2] *= np.clip((xyz[:, 0] - 0.065) / 0.035, 0, 1)
    scores[:, 3] *= np.clip((-xyz[:, 0] - 0.065) / 0.035, 0, 1)
    scores[:, 0] += 1e-20
    region = scores / scores.sum(axis=1)[:, None]
    # Source-space anatomy regression: the upper tail's solid core must never
    # inherit hair/arm deformation. The adjacent hanging curls are behind this
    # volume and keep their own weights. Previously these core vertices were
    # pulled toward the head, producing a skirt-like flap during swimming.
    tail_core = (
        (xyz[:, 1] > 0.50)
        & (xyz[:, 1] < 0.64)
        & (np.abs(xyz[:, 0]) < 0.065)
        & (xyz[:, 2] > 0.005)
    )
    if not tail_core.any() or np.any(region[tail_core, 0] < 0.999):
        raise ValueError("Upper tail must be isolated from hair and arm skin weights")
    return xyz, region


def rainbow_material(obj, xyz, region):
    mat = obj.data.materials[0]
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    bs = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    image = next(n.image for n in nodes if n.type == "TEX_IMAGE" and n.image)
    pixels = np.array(image.pixels[:], dtype=np.float32).reshape(
        image.size[1], image.size[0], 4
    )
    tint = obj.data.color_attributes.new(
        name="CostumeBake", type="FLOAT_COLOR", domain="CORNER"
    )
    for loop in obj.data.loops:
        x, h, d = xyz[loop.vertex_index]
        u, v = obj.data.uv_layers.active.data[loop.index].uv
        rgb = pixels[
            min(image.size[1] - 1, int(v % 1 * image.size[1])),
            min(image.size[0] - 1, int(u % 1 * image.size[0])),
            :3,
        ]
        tail = (1 - smooth(0.635, 0.665, h)) * region[loop.vertex_index, 0]
        shell = (
            smooth(0.743, 0.755, h)
            * (1 - smooth(0.795, 0.807, h))
            * smooth(0.07, 0.09, d)
            * (1 - smooth(0.06, 0.078, abs(x)))
            * (1 - smooth(0.24, 0.48, float(max(rgb))))
        )
        mask = max(tail, shell)
        hue = (0.82 - (0.64 - h) * 1.5) % 1 if tail > 0.1 else 0.78 + x * 2
        color = colorsys.hsv_to_rgb(hue, 0.68, 0.94)
        tint.data[loop.index].color = (*color, mask)
    texture = nodes.new("ShaderNodeTexImage")
    texture.image = image
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "CostumeBake"
    bw = nodes.new("ShaderNodeRGBToBW")
    links.new(texture.outputs["Color"], bw.inputs[0])
    level = nodes.new("ShaderNodeMath")
    level.operation = "MULTIPLY_ADD"
    level.inputs[1].default_value = 0.70
    level.inputs[2].default_value = 0.30
    links.new(bw.outputs[0], level.inputs[0])
    color = nodes.new("ShaderNodeMixRGB")
    color.blend_type = "MULTIPLY"
    color.inputs[0].default_value = 1
    links.new(level.outputs[0], color.inputs[1])
    links.new(vc.outputs["Color"], color.inputs[2])
    mix = nodes.new("ShaderNodeMixRGB")
    links.new(vc.outputs["Alpha"], mix.inputs[0])
    links.new(texture.outputs["Color"], mix.inputs[1])
    links.new(color.outputs[0], mix.inputs[2])
    emission = nodes.new("ShaderNodeEmission")
    links.new(mix.outputs[0], emission.inputs[0])
    out = next(n for n in nodes if n.type == "OUTPUT_MATERIAL")
    links.new(emission.outputs[0], out.inputs[0])
    baked = bpy.data.images.new(
        "Sabrina_Rainbow_Color", width=2048, height=2048, alpha=False
    )
    target = nodes.new("ShaderNodeTexImage")
    target.image = baked
    nodes.active = target
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 4
    scene.render.bake.margin = 8
    bpy.ops.object.bake(type="EMIT")
    baked.pack()
    game = bpy.data.materials.new("Sabrina_Pearlescent")
    game.use_nodes = True
    bs = game.node_tree.nodes.get("Principled BSDF")
    bs.inputs["Roughness"].default_value = 0.5
    bs.inputs["Metallic"].default_value = 0.12
    tex = game.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = baked
    game.node_tree.links.new(tex.outputs["Color"], bs.inputs["Base Color"])
    obj.data.materials.clear()
    obj.data.materials.append(game)
    obj.data.color_attributes.remove(tint)
    white = obj.data.color_attributes.new(
        name="Color", type="FLOAT_COLOR", domain="POINT"
    )
    white.data.foreach_set("color", np.ones(len(white.data) * 4, dtype=np.float32))
    bpy.data.materials.remove(mat)
    for im in list(bpy.data.images):
        if not im.users:
            bpy.data.images.remove(im)


def create_rig(obj, xyz, region):
    bones = [
        ("Root", Vector(), upper((0, 0.68, 0.055)), None),
        ("Spine", upper((0, 0.64, 0.055)), upper((0, 0.735, 0.05)), "Root"),
        ("Chest", upper((0, 0.735, 0.05)), upper((0, 0.835, 0.045)), "Spine"),
        ("Neck", upper((0, 0.835, 0.045)), upper((0, 0.875, 0.055)), "Chest"),
        ("Head", upper((0, 0.875, 0.055)), upper((0, 0.97, 0.065)), "Neck"),
    ]
    for i in range(6):
        bones.append(
            (f"Tail_{i+1}", TAIL[i], TAIL[i + 1], "Root" if i == 0 else f"Tail_{i}")
        )
    for side, sign in [("L", 1), ("R", -1)]:
        points = [
            (sign * 0.015, 0.829, 0.045),
            (sign * 0.090, 0.829, 0.045),
            (sign * 0.195, 0.798, 0.05),
            (sign * 0.286, 0.786, 0.057),
            (sign * 0.332, 0.775, 0.065),
        ]
        for i, name in enumerate(["Clavicle", "UpperArm", "Forearm", "Hand"]):
            parent = (
                "Chest"
                if i == 0
                else ["Clavicle", "UpperArm", "Forearm"][i - 1] + "_" + side
            )
            bones.append(
                (name + "_" + side, upper(points[i]), upper(points[i + 1]), parent)
            )
        fin = TAIL[-1] + Vector((-0.48, -sign * 0.43, 0))
        bones.extend(
            [
                (f"Fluke_{side}", TAIL[-1], fin, "Tail_6"),
                (
                    f"FlukeTip_{side}",
                    fin,
                    TAIL[-1] + Vector((-0.912, -sign * 0.70, 0)),
                    f"Fluke_{side}",
                ),
            ]
        )
    for section, x, d in [
        ("Back", 0, -0.065),
        ("Left", 0.085, -0.01),
        ("Right", -0.085, -0.01),
    ]:
        points = [(x, 0.94, d), (x, 0.82, d), (x, 0.70, d), (x, 0.59, d)]
        for i in range(3):
            bones.append(
                (
                    f"Hair_{section}_{i+1}",
                    upper(points[i]),
                    upper(points[i + 1]),
                    "Head" if i == 0 else f"Hair_{section}_{i}",
                )
            )
    rig = armature("Sabrina_Rig", bones)
    rig["asset_id"] = "sabrina"
    rig["source_pose"] = (
        "Neutral T-pose, hand-authored clavicle/elbow/neck deformation rig"
    )
    rig["forward_axis"] = "+X"
    groups = {name: obj.vertex_groups.new(name=name) for name, *_ in bones}
    for vertex in obj.data.vertices:
        x, h, d = xyz[vertex.index]
        if h < 0.20:
            side = "L" if x >= 0 else "R"
            body = chain_weights(
                h,
                [(0.03, f"FlukeTip_{side}"), (0.12, f"Fluke_{side}"), (0.20, "Tail_6")],
            )
        else:
            body = chain_weights(
                h,
                [
                    (0.23, "Tail_6"),
                    (0.30, "Tail_5"),
                    (0.37, "Tail_4"),
                    (0.45, "Tail_3"),
                    (0.53, "Tail_2"),
                    (0.61, "Tail_1"),
                    (0.665, "Spine"),
                    (0.78, "Chest"),
                    (0.849, "Neck"),
                    (0.884, "Head"),
                ],
            )
        section = "Back" if abs(x) < 0.045 else "Left" if x > 0 else "Right"
        hair = chain_weights(
            h,
            [
                (0.61, f"Hair_{section}_3"),
                (0.72, f"Hair_{section}_2"),
                (0.84, f"Hair_{section}_1"),
                (0.96, "Head"),
            ],
        )
        categories = [body, hair]
        for side, sign in [("L", 1), ("R", -1)]:
            categories.append(
                chain_weights(
                    sign * x,
                    [
                        (0.062, "Chest"),
                        (0.082, "Clavicle_" + side),
                        (0.113, "UpperArm_" + side),
                        (0.18, "UpperArm_" + side),
                        (0.207, "Forearm_" + side),
                        (0.273, "Forearm_" + side),
                        (0.295, "Hand_" + side),
                    ],
                )
            )
        weights = {}
        for amount, category in zip(region[vertex.index], categories):
            for name, w in category.items():
                weights[name] = weights.get(name, 0) + amount * w
        weights = dict(
            sorted(weights.items(), key=lambda item: item[1], reverse=True)[:4]
        )
        total = sum(weights.values())
        for name, w in weights.items():
            if w > 1e-8:
                groups[name].add([vertex.index], w / total, "REPLACE")
        body_point = tail_point((x, h, d)) if h < 0.64 else upper((x, h, d))
        vertex.co = coord(
            body_point.lerp(upper((x, h, d)), 1 - region[vertex.index, 0])
        )
    mod = obj.modifiers.new("Smooth source silhouette", "SUBSURF")
    mod.levels = 1
    mod.subdivision_type = "CATMULL_CLARK"
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=mod.name)
    # Subdivision interpolates weights; limit again for the portable four-influence skin.
    bpy.ops.object.vertex_group_limit_total(limit=4)
    bpy.ops.object.vertex_group_normalize_all(lock_active=False)
    obj.modifiers.new("Sabrina_skin", "ARMATURE").object = rig
    obj.parent = rig
    return rig


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    source = Path(args[0]).resolve()
    out = Path(args[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    if hashlib.sha256(source.read_bytes()).hexdigest() != SOURCE_SHA:
        raise ValueError(
            "This recipe is specific to the owner-supplied neutral-pose FBX"
        )
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(source))
    obj = next(o for o in bpy.context.scene.objects if o.type == "MESH")
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    obj.name = "Sabrina_NeutralPose"
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=1e-6)
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(obj.data)
    bm.free()
    for face in obj.data.polygons:
        face.use_smooth = True
    xyz, region = regions(obj)
    rainbow_material(obj, xyz, region)
    rig = create_rig(obj, xyz, region)
    author_actions(rig)
    bpy.context.scene.render.fps = 30
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.wm.save_as_mainfile(filepath=str(out / "sabrina.blend"))
    bpy.ops.export_scene.gltf(
        filepath=str(out / "sabrina.glb"),
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
        export_cameras=False,
        export_lights=False,
    )
    print(
        "CANDIDATE",
        out,
        "bones",
        len(rig.data.bones),
        "triangles",
        sum(len(p.vertices) - 2 for p in obj.data.polygons),
        flush=True,
    )


if __name__ == "__main__":
    main()
