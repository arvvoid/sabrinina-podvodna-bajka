#!/usr/bin/env python3
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Validate shipped rigged glTF assets without requiring Blender or Godot."""

import json
import math
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = {
    "sabrina": (
        {
            "Idle",
            "Swim",
            "Sing",
            "Wave",
            "Throw",
            "Hurt",
            "Turn_Left",
            "Turn_Right",
        },
        {
            "Root",
            "Head",
            "Tail_1",
            "Tail_4",
            "Hand_L",
            "Hand_R",
            "Clavicle_L",
            "Clavicle_R",
        },
    ),
    "jellyfish": (
        {"Pulse", "Nibble", "Drift"},
        {"Root", "Bell", "Mouth", "Tentacle_0_0", "Tentacle_7_2"},
    ),
    **{
        n: ({"Idle", "Swim"}, {"Root", "Spine", "Peduncle", "Caudal", "Fin_L", "Fin_R"})
        for n in [
            "clownfish",
            "blue_tang",
            "butterflyfish",
            "bannerfish",
            "parrotfish",
            "blue_wrasse",
            "fire_goby",
            "royal_tang",
            "yellow_tang",
            "emperor_angelfish",
            "coral_anthias",
            "cardinalfish",
        ]
    },
}
FORMATS = {5120: "b", 5121: "B", 5122: "h", 5123: "H", 5125: "I", 5126: "f"}
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def read_glb(path):
    raw = path.read_bytes()
    magic, version, length = struct.unpack_from("<III", raw)
    assert (
        magic == 0x46546C67 and version == 2 and length == len(raw)
    ), f"{path}: invalid GLB header"
    offset = 12
    document = None
    binary = None
    while offset < len(raw):
        n, kind = struct.unpack_from("<II", raw, offset)
        chunk = raw[offset + 8 : offset + 8 + n]
        offset += 8 + n
        if kind == 0x4E4F534A:
            document = json.loads(chunk)
        elif kind == 0x004E4942:
            binary = chunk
    assert document is not None and binary is not None
    return document, binary


def values(document, binary, index):
    accessor = document["accessors"][index]
    width = WIDTHS[accessor["type"]]

    def read(view_index, offset, count, component, columns):
        view = document["bufferViews"][view_index]
        fmt = "<" + FORMATS[component] * columns
        size = struct.calcsize(fmt)
        stride = view.get("byteStride", size)
        assert offset + (count - 1) * stride + size <= view["byteLength"]
        offset += view.get("byteOffset", 0)
        assert offset + (count - 1) * stride + size <= len(binary)
        return [struct.unpack_from(fmt, binary, offset + i * stride) for i in range(count)]

    decoded = (
        read(
            accessor["bufferView"],
            accessor.get("byteOffset", 0),
            accessor["count"],
            accessor["componentType"],
            width,
        )
        if "bufferView" in accessor
        else [(0,) * width for _ in range(accessor["count"])]
    )
    if "sparse" in accessor:
        sparse = accessor["sparse"]
        indices, data = sparse["indices"], sparse["values"]
        rows = read(
            indices["bufferView"],
            indices.get("byteOffset", 0),
            sparse["count"],
            indices["componentType"],
            1,
        )
        updates = read(
            data["bufferView"],
            data.get("byteOffset", 0),
            sparse["count"],
            accessor["componentType"],
            width,
        )
        assert all(0 <= row[0] < len(decoded) for row in rows)
        assert all(rows[i][0] < rows[i + 1][0] for i in range(len(rows) - 1))
        for (row,), update in zip(rows, updates):
            decoded[row] = update
    if accessor.get("normalized"):
        divisor = {5121: 255, 5123: 65535}[accessor["componentType"]]
        decoded = [tuple(x / divisor for x in row) for row in decoded]
    assert all(math.isfinite(x) for row in decoded for x in row)
    return decoded


def image_size(document, binary, image):
    view = document["bufferViews"][image["bufferView"]]
    payload = binary[
        view.get("byteOffset", 0) : view.get("byteOffset", 0) + view["byteLength"]
    ]
    if payload.startswith(b"\x89PNG"):
        return struct.unpack_from(">II", payload, 16)
    assert payload.startswith(b"\xff\xd8"), "Expected embedded PNG or JPEG"
    offset = 2
    while offset < len(payload):
        assert payload[offset] == 255
        marker = payload[offset + 1]
        offset += 2
        if marker in (216, 217):
            continue
        size = struct.unpack_from(">H", payload, offset)[0]
        if marker in (192, 193, 194):
            height, width = struct.unpack_from(">HH", payload, offset + 3)
            return width, height
        offset += size
    raise AssertionError("Missing JPEG dimensions")


def validate_reef():
    assert (ROOT / "art/environments/reef_ruins.blend").is_file(), "Editable reef source missing"
    path = ROOT / "assets/environments/reef_ruins.glb"
    document, binary = read_glb(path)
    assert not document.get("skins") and not document.get(
        "animations"
    ), "Scenery must not contain frozen character rigs"
    triangles = 0
    vertices = []
    for mesh in document["meshes"]:
        for primitive in mesh["primitives"]:
            attrs = primitive["attributes"]
            assert {"POSITION", "NORMAL", "TEXCOORD_0"} <= set(attrs)
            positions = values(document, binary, attrs["POSITION"])
            assert all(p[2] >= 4.5 for p in positions) or all(
                p[2] <= -3.9 for p in positions
            ), "Reef surface crosses the actor depth band"
            vertices.extend(positions)
            triangles += len(values(document, binary, primitive["indices"])) // 3
    assert 0 < triangles <= 160000, "Reef triangle budget exceeded"
    front = [p for p in vertices if p[2] > 1]
    assert front, "Near reef must occlude the lower-left swimming plane"
    assert all(
        p[0] < -7.5 and p[1] < 5.5 for p in front
    ), "Foreground must leave the main play area clear"
    assert max(p[2] for p in vertices) < 6, "Foreground cannot approach the camera"
    assert min(p[2] for p in vertices) < -20, "Reef needs a distant layer"
    assert all("uri" not in i and max(image_size(document, binary, i)) <= 2048 for i in document["images"])
    assert document["materials"][0]["pbrMetallicRoughness"].get("baseColorTexture")
    assert document["materials"][0].get("normalTexture")
    print(f"reef_ruins: {triangles} triangles; embedded 2K maps and stage bounds OK")


def validate_crystal():
    assert (ROOT / "art/props/ocean_crystal.blend").is_file(), "Editable crystal source missing"
    path = ROOT / "assets/props/ocean_crystal.glb"
    document, binary = read_glb(path)
    triangles = 0
    assert len(document["meshes"]) == 1 and len(document["materials"]) == 1
    for primitive in document["meshes"][0]["primitives"]:
        assert {"POSITION", "NORMAL", "TEXCOORD_0"} <= set(primitive["attributes"])
        points = values(document, binary, primitive["attributes"]["POSITION"])
        assert 1.34 < max(p[1] for p in points) - min(p[1] for p in points) < 1.36
        triangles += len(values(document, binary, primitive["indices"])) // 3
    assert 0 < triangles <= 14000
    assert all("uri" not in i and max(image_size(document, binary, i)) <= 1024 for i in document["images"])
    assert document["materials"][0]["pbrMetallicRoughness"].get("baseColorTexture")
    assert document["materials"][0].get("normalTexture")
    print(f"ocean_crystal: {triangles} triangles; centered scale, embedded 1K maps OK")


def main():
    for name, (clips, bones) in EXPECTED.items():
        path = ROOT / f"assets/models/{name}.glb"
        document, binary = read_glb(path)
        assert (
            ROOT / f"art/characters/{name}.blend"
        ).is_file(), f"{name}: editable source missing"
        assert len(document.get("skins", [])) == 1, f"{name}: needs exactly one skin"
        skin = document["skins"][0]
        joints = skin["joints"]
        names = {document["nodes"][j]["name"] for j in joints}
        assert bones <= names, f"{name}: missing joints {bones-names}"
        assert len(joints) <= 64, f"{name}: bone budget exceeded"
        if name == "sabrina":
            by_name = {document["nodes"][j]["name"]: j for j in joints}
            for side in ("L", "R"):
                shoulder = document["nodes"][by_name["Clavicle_" + side]]
                assert by_name["UpperArm_" + side] in shoulder.get(
                    "children", []
                ), "Arm must inherit its collarbone"

        assert len(values(document, binary, skin["inverseBindMatrices"])) == len(joints)
        animations = {attributes["name"]: attributes for attributes in document["animations"]}
        assert clips <= set(animations), f"{name}: missing animation clips"
        for clip in clips:
            animation = animations[clip]
            assert animation["channels"], f"{name}/{clip}: empty animation"
            animated = False
            stroke = 0.0
            for channel in animation["channels"]:
                assert channel["target"]["node"] in joints
                sampler = animation["samplers"][channel["sampler"]]
                times = values(document, binary, sampler["input"])
                poses = values(document, binary, sampler["output"])
                assert times == sorted(times) and len(times) == len(poses)
                if channel["target"]["path"] == "rotation":
                    dot = abs(sum(x * y for x, y in zip(poses[0], poses[-1])))
                    assert dot > 0.9999, f"{name}/{clip}: rotation jumps at loop seam"
                    bone_name = document["nodes"][channel["target"]["node"]]["name"]
                    if name == "sabrina":
                        for index in range(1, len(poses)):
                            gap = times[index][0] - times[index - 1][0]
                            alignment = min(
                                1,
                                abs(
                                    sum(
                                        x * y
                                        for x, y in zip(poses[index - 1], poses[index])
                                    )
                                ),
                            )
                            assert 2 * math.acos(alignment) <= max(
                                0.12, gap * 22
                            ), f"{name}/{clip}/{bone_name}: abrupt joint flip"

                    if bone_name == ("Tail_4" if name == "sabrina" else "Caudal"):
                        stroke = max(
                            2
                            * math.acos(
                                min(1, abs(sum(x * y for x, y in zip(poses[0], pose))))
                            )
                            for pose in poses
                        )
                if len(poses) > 1 and poses[0] != poses[len(poses) // 3]:
                    animated = True
            assert animated, f"{name}/{clip}: no animated joint motion"
            if name != "jellyfish" and clip in {"Idle", "Swim"}:
                assert stroke > 0.15, f"{name}/{clip}: tail motion is too small to read"
        if name != "jellyfish":
            texture_limit = 2048 if name == "sabrina" else 512
            assert all(
                max(image_size(document, binary, i)) <= texture_limit for i in document["images"]
            ), f"{name}: texture budget exceeded"
        triangles = 0
        for mesh in document["meshes"]:
            for primitive in mesh["primitives"]:
                attributes = primitive["attributes"]
                assert {
                    "POSITION",
                    "NORMAL",
                    "TEXCOORD_0",
                    "COLOR_0",
                    "JOINTS_0",
                    "WEIGHTS_0",
                } <= set(attributes)
                pos = values(document, binary, attributes["POSITION"])
                weights = values(document, binary, attributes["WEIGHTS_0"])
                indices = values(document, binary, attributes["JOINTS_0"])
                assert len(pos) == len(weights) == len(indices)
                assert all(
                    abs(sum(w) - 1) < 0.0002 and min(w) >= 0 for w in weights
                ), f"{name}: invalid skin weights"
                assert all(
                    0 <= j < len(joints) for row in indices for j in row
                ), f"{name}: joint index outside skin"
                triangles += len(values(document, binary, primitive["indices"])) // 3
        budget = 60000 if name == "sabrina" else 12000 if name == "jellyfish" else 8000
        assert triangles <= budget, f"{name}: triangle budget exceeded ({triangles})"
        assert all(
            "uri" not in image for image in document.get("images", [])
        ), f"{name}: textures must be embedded"
        print(
            f"{name}: {triangles} triangles, {len(joints)} bones, {len(clips)} animated clips; skin and textures OK"
        )
    document, binary = read_glb(ROOT / "assets/models/sabrina.glb")
    for mesh in document["meshes"]:
        assert mesh.get("extras", {}).get("targetNames") == [
            "Blink",
            "SoftSmile",
            "Singing",
        ]
        assert all(
            w == 0 for w in mesh.get("weights", [])
        ), "Neutral facial weights must be zero"
        for primitive in mesh["primitives"]:
            assert len(primitive["targets"]) == 3
            count = document["accessors"][primitive["attributes"]["POSITION"]]["count"]
            for target in primitive["targets"]:
                delta = values(document, binary, target["POSITION"])
                assert len(delta) == count
                assert 0 < max(sum(x * x for x in row) ** 0.5 for row in delta) < 0.08
    print(
        "Sabrina: neutral facial weights and bounded Blink/SoftSmile/Singing morphs OK"
    )
    validate_reef()
    validate_crystal()
    print("All character, prop and environment asset contracts passed")


if __name__ == "__main__":
    main()
