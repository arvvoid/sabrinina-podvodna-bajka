# SPDX-License-Identifier: MIT
"""Small authoring helpers. Coordinates are game axes: X right, Y up, Z toward camera."""

import math
import bpy
from mathutils import Vector


def coord(p):
    return Vector((p[0], -p[2], p[1]))


def material(name, color=(1, 1, 1, 1), roughness=0.45, metallic=0):
    m = bpy.data.materials.new(name)
    m.diffuse_color = color
    m.use_nodes = True
    bs = m.node_tree.nodes.get("Principled BSDF")
    bs.inputs["Base Color"].default_value = color
    bs.inputs["Roughness"].default_value = roughness
    bs.inputs["Metallic"].default_value = metallic
    vc = m.node_tree.nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Color"
    m.node_tree.links.new(vc.outputs["Color"], bs.inputs["Base Color"])
    return m


def mesh(name, verts, faces, mat, colors=None, weights=None, uv=None):
    data = bpy.data.meshes.new(name)
    data.from_pydata([coord(p) for p in verts], [], faces)
    data.update()
    obj = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(obj)
    data.materials.append(mat)
    for f in data.polygons:
        f.use_smooth = True
    if colors is None:
        colors = [(1, 1, 1, 1)] * len(verts)
    layer = data.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    for i, c in enumerate(colors):
        layer.data[i].color = (*c[:3], c[3] if len(c) > 3 else 1)
    tex = data.uv_layers.new(name="UVMap")
    for f in data.polygons:
        for li in f.loop_indices:
            tex.data[li].uv = uv[data.loops[li].vertex_index] if uv else (0, 0)
    if weights:
        names = sorted({bone for item in weights for bone in item})
        groups = {n: obj.vertex_groups.new(name=n) for n in names}
        for i, item in enumerate(weights):
            total = sum(item.values())
            for bone, w in item.items():
                if w > 1e-6:
                    groups[bone].add([i], w / total, "REPLACE")
    return obj


def surface(name, rows, sides, shape, mat, color, weight, closed=False):
    verts = []
    colors = []
    weights = []
    uv = []
    faces = []
    for i in range(rows + 1):
        for j in range(sides + 1):
            u, v = i / rows, j / sides
            p = shape(u, v)
            verts.append(p)
            colors.append(color(u, v, p) if callable(color) else color)
            weights.append(weight(u, v, p) if callable(weight) else {weight: 1})
            uv.append((u, v))
    for i in range(rows):
        for j in range(sides):
            a = i * (sides + 1) + j
            b = a + sides + 1
            faces.append((a, a + 1, b + 1, b))
    if closed:
        for ring, reverse in [(0, True), (rows, False)]:
            first = ring * (sides + 1)
            p = tuple(
                sum(verts[first + j][k] for j in range(sides)) / sides for k in range(3)
            )
            idx = len(verts)
            verts.append(p)
            colors.append(colors[first])
            weights.append(weights[first])
            uv.append((ring / rows, 0.5))
            for j in range(sides):
                faces.append(
                    (idx, first + j + 1, first + j)
                    if reverse
                    else (idx, first + j, first + j + 1)
                )
    return mesh(name, verts, faces, mat, colors, weights, uv)


def ellipsoid(name, center, scale, mat, color, bone, rings=12, sides=20):
    def shape(u, v):
        a = math.pi * u
        b = math.tau * v
        return (
            center[0] + scale[0] * math.sin(a) * math.cos(b),
            center[1] + scale[1] * math.cos(a),
            center[2] + scale[2] * math.sin(a) * math.sin(b),
        )

    return surface(name, rings, sides, shape, mat, color, bone)


def bezier(points, t):
    a, b, c, d = [Vector(p) for p in points]
    q = 1 - t
    return a * q * q * q + 3 * b * q * q * t + 3 * c * q * t * t + d * t * t * t


def tube(
    name,
    points,
    radius,
    mat,
    color,
    weight,
    rings=18,
    sides=8,
    taper=0.1,
    flat=1,
    root_taper=0,
):
    def shape(u, v):
        p = bezier(points, u)
        tangent = (
            bezier(points, min(1, u + 0.001)) - bezier(points, max(0, u - 0.001))
        ).normalized()
        normal = Vector((-tangent.y, tangent.x, 0)).normalized()
        if normal.length < 0.1:
            normal = Vector((0, 1, 0))
        binormal = tangent.cross(normal).normalized()
        r = radius * (1 - u * (1 - taper))
        if root_taper:
            r *= min(1, u / root_taper)
        return (
            p
            + normal * (math.cos(v * math.tau) * r)
            + binormal * (math.sin(v * math.tau) * r * flat)
        )

    return surface(name, rings, sides, shape, mat, color, weight, closed=True)


def chain_weights(y, stops):
    # Ordered (height, bone) pairs; linear blends between adjacent deformation bones.
    stops = sorted(stops)
    if y <= stops[0][0]:
        return {stops[0][1]: 1}
    if y >= stops[-1][0]:
        return {stops[-1][1]: 1}
    for (a, an), (b, bn) in zip(stops, stops[1:]):
        if a <= y <= b:
            t = (y - a) / (b - a)
            return {an: 1 - t, bn: t}


def join_surfaces(objects, name):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.object.join()
    obj = bpy.context.object
    obj.name = name
    # Recalculate outside normals after joining parametric patches.
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.remove_doubles(threshold=0.00001)
    bpy.ops.mesh.dissolve_degenerate(threshold=0.00001)
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    return obj


def finish_meshes(rig, objects):
    # One skinned mesh per material, with real vertex groups preserved for editing.
    groups = {}
    for obj in objects:
        groups.setdefault(obj.data.materials[0].name, []).append(obj)
    result = []
    for name, parts in groups.items():
        obj = join_surfaces(parts, name)
        obj.parent = rig
        mod = obj.modifiers.new("Skin", "ARMATURE")
        mod.object = rig
        obj["authoring_note"] = (
            "Edit vertices and vertex weights; export this .blend without rebuilding."
        )
        result.append(obj)
    return result
