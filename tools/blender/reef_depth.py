# SPDX-License-Identifier: Apache-2.0
"""Separate complete near-reef faces from the background; never cross actor depth.

The camera is orthographic. Keeping XY and UVs unchanged preserves the original
silhouette at the seam, while two disjoint depth bands prevent half-hidden actors.
Run on the unlayered preparation mesh, not an already deformed export.
"""

import bpy
import bmesh


def layer_foreground(obj):
    if obj.get("foreground_layer_version") == 2:
        return
    if obj.get("foreground_layer_version"):
        raise RuntimeError(
            "Rebuild from the unlayered source before changing depth layers"
        )
    near = {
        p.index
        for p in obj.data.polygons
        if all(
            obj.data.vertices[i].co.x < -7.5
            and obj.data.vertices[i].co.z < 5.5
            and obj.data.vertices[i].co.y < 11
            for i in p.vertices
        )
    }
    front = obj.copy()
    front.data = obj.data.copy()
    front.name = "Reef_Near_Left"
    bpy.context.collection.objects.link(front)
    for target, keep_near in [(obj, False), (front, True)]:
        bm = bmesh.new()
        bm.from_mesh(target.data)
        bm.faces.ensure_lookup_table()
        remove = [f for f in bm.faces if (f.index in near) != keep_near]
        bmesh.ops.delete(bm, geom=remove, context="FACES")
        unused = [v for v in bm.verts if not v.link_faces]
        bmesh.ops.delete(bm, geom=unused, context="VERTS")
        bm.to_mesh(target.data)
        bm.free()
        if keep_near:
            # Preserve source lighting normals despite compressing only its depth.
            normals = [tuple(v.normal) for v in target.data.vertices]
            for v in target.data.vertices:
                v.co.y = -5.7 + 0.10 * max(0, min(11, v.co.y))
            target.data.update()
            target.data.normals_split_custom_set_from_vertices(normals)
        target["foreground_layer_version"] = 2
        target["gameplay"] = (
            "Decorative near layer" if keep_near else "Decorative background layer"
        )
    print("Separated foreground faces:", len(near))
