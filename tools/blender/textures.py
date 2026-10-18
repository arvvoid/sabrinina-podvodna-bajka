# SPDX-License-Identifier: MIT
"""Small packed normal textures: editable with the .blend and embedded in glTF."""

import bpy
import numpy as np
from pathlib import Path


def scale_normal(mat, name, rows=28, columns=18, strength=0.30):
    size = 512
    y, x = np.mgrid[0:size, 0:size].astype(np.float32) / size
    row = np.floor(x * rows)
    cx = np.mod(y * columns + row * 0.5, 1) - 0.5
    cy = np.mod(x * rows, 1) - 0.45
    dist = np.sqrt((cx / 0.55) ** 2 + (cy / 0.66) ** 2)
    h = np.exp(-dist * dist * 2) * 0.7 + np.exp(-(((dist - 0.88) / 0.055) ** 2)) * 0.13
    dy, dx = np.gradient(h)
    n = np.stack(
        (
            -dx * strength * size / rows,
            -dy * strength * size / columns,
            np.ones_like(h),
        ),
        axis=-1,
    )
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    rgba = np.concatenate(
        (n * 0.5 + 0.5, np.ones((size, size, 1), dtype=np.float32)), axis=-1
    )
    im = bpy.data.images.new(name, width=size, height=size, alpha=True)
    im.colorspace_settings.name = "Non-Color"
    im.pixels.foreach_set(rgba.ravel())
    im.pack()
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = im
    tex.label = "Packed scale normal; edit or replace in Blender"
    normal = mat.node_tree.nodes.new("ShaderNodeNormalMap")
    normal.inputs["Strength"].default_value = 1
    mat.node_tree.links.new(tex.outputs["Color"], normal.inputs["Color"])
    mat.node_tree.links.new(
        normal.outputs["Normal"],
        mat.node_tree.nodes.get("Principled BSDF").inputs["Normal"],
    )


def fin_normal(mat, name):
    size = 256
    y, x = np.mgrid[0:size, 0:size].astype(np.float32) / size
    h = np.cos((y * 32 + x * 0.8) * np.pi * 2) * (0.15 + 0.85 * x)
    dy, dx = np.gradient(h)
    n = np.stack((-dx * 0.18, -dy * 0.35, np.ones_like(h)), axis=-1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    rgba = np.concatenate(
        (n * 0.5 + 0.5, np.ones((size, size, 1), dtype=np.float32)), axis=-1
    )
    im = bpy.data.images.new(name, width=size, height=size, alpha=True)
    im.colorspace_settings.name = "Non-Color"
    im.pixels.foreach_set(rgba.ravel())
    im.pack()
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = im
    normal = mat.node_tree.nodes.new("ShaderNodeNormalMap")
    mat.node_tree.links.new(tex.outputs["Color"], normal.inputs["Color"])
    mat.node_tree.links.new(
        normal.outputs["Normal"],
        mat.node_tree.nodes.get("Principled BSDF").inputs["Normal"],
    )
