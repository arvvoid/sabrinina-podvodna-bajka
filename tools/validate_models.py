#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate shipped rigged glTF assets without requiring Blender or Godot."""

import json, math, struct
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


def read_glb(p):
    raw = p.read_bytes()
    magic, version, length = struct.unpack_from("<III", raw)
    assert (
        magic == 0x46546C67 and version == 2 and length == len(raw)
    ), f"{p}: invalid GLB header"
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


def values(g, b, index):
    a = g["accessors"][index]
    v = g["bufferViews"][a["bufferView"]]
    code = FORMATS[a["componentType"]]
    width = WIDTHS[a["type"]]
    fmt = "<" + code * width
    size = struct.calcsize(fmt)
    stride = v.get("byteStride", size)
    offset = v.get("byteOffset", 0) + a.get("byteOffset", 0)
    assert offset + (a["count"] - 1) * stride + size <= len(b)
    out = [struct.unpack_from(fmt, b, offset + i * stride) for i in range(a["count"])]
    if a.get("normalized"):
        divisor = {5121: 255, 5123: 65535}[a["componentType"]]
        out = [tuple(x / divisor for x in row) for row in out]
    assert all(math.isfinite(x) for row in out for x in row)
    return out


def image_size(g, binary, image):
    view = g["bufferViews"][image["bufferView"]]
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
    record = json.loads((ROOT / "art/environments/manifest.json").read_text())
    assert (ROOT / record["source"]).is_file(), "Editable reef source missing"
    path = ROOT / record["export"]
    g, b = read_glb(path)
    assert path.stat().st_size == record["bytes"], "Re-export reef and update manifest"
    assert not g.get("skins") and not g.get(
        "animations"
    ), "Scenery must not contain frozen character rigs"
    triangles = 0
    vertices = []
    for mesh in g["meshes"]:
        for primitive in mesh["primitives"]:
            attrs = primitive["attributes"]
            assert {"POSITION", "NORMAL", "TEXCOORD_0"} <= set(attrs)
            positions = values(g, b, attrs["POSITION"])
            assert all(p[2] >= 4.5 for p in positions) or all(
                p[2] <= -3.9 for p in positions
            ), "Reef surface crosses the actor depth band"
            vertices.extend(positions)
            triangles += len(values(g, b, primitive["indices"])) // 3
    assert 0 < triangles <= 160000, "Reef triangle budget exceeded"
    front = [p for p in vertices if p[2] > 1]
    assert front, "Near reef must occlude the lower-left swimming plane"
    assert all(
        p[0] < -7.5 and p[1] < 5.5 for p in front
    ), "Foreground must leave the main play area clear"
    assert max(p[2] for p in vertices) < 6, "Foreground cannot approach the camera"
    assert min(p[2] for p in vertices) < -20, "Reef needs a distant layer"
    assert all("uri" not in i and max(image_size(g, b, i)) <= 2048 for i in g["images"])
    assert g["materials"][0]["pbrMetallicRoughness"].get("baseColorTexture")
    assert g["materials"][0].get("normalTexture")
    print(f"reef_ruins: {triangles} triangles; embedded 2K maps and stage bounds OK")


def validate_crystal():
    record = json.loads((ROOT / "art/props/manifest.json").read_text())
    assert (ROOT / record["source"]).is_file()
    path = ROOT / record["export"]
    g, b = read_glb(path)
    assert path.stat().st_size == record["bytes"]
    triangles = 0
    assert len(g["meshes"]) == 1 and len(g["materials"]) == 1
    for primitive in g["meshes"][0]["primitives"]:
        assert {"POSITION", "NORMAL", "TEXCOORD_0"} <= set(primitive["attributes"])
        points = values(g, b, primitive["attributes"]["POSITION"])
        assert 1.34 < max(p[1] for p in points) - min(p[1] for p in points) < 1.36
        triangles += len(values(g, b, primitive["indices"])) // 3
    assert triangles == record["triangles"] and triangles <= 14000
    assert all("uri" not in i and max(image_size(g, b, i)) <= 1024 for i in g["images"])
    assert g["materials"][0]["pbrMetallicRoughness"].get("baseColorTexture")
    assert g["materials"][0].get("normalTexture")
    print(f"ocean_crystal: {triangles} triangles; centered scale, embedded 1K maps OK")


def main():
    manifest = json.loads((ROOT / "art/characters/manifest.json").read_text())
    records = {r["asset"]: r for r in manifest["assets"]}
    assert set(records) == set(EXPECTED), "Manifest and asset contracts disagree"
    for name, (clips, bones) in EXPECTED.items():
        p = ROOT / f"game/assets/models/{name}.glb"
        g, b = read_glb(p)
        assert (
            ROOT / records[name]["source"]
        ).is_file(), f"{name}: editable source missing"
        assert (
            p.stat().st_size == records[name]["bytes"]
        ), f"{name}: re-export and update manifest"
        assert len(g.get("skins", [])) == 1, f"{name}: needs exactly one skin"
        skin = g["skins"][0]
        joints = skin["joints"]
        names = {g["nodes"][j]["name"] for j in joints}
        assert bones <= names, f"{name}: missing joints {bones-names}"
        assert len(joints) <= 64, f"{name}: bone budget exceeded"
        if name == "sabrina":
            by_name = {g["nodes"][j]["name"]: j for j in joints}
            for side in ("L", "R"):
                shoulder = g["nodes"][by_name["Clavicle_" + side]]
                assert by_name["UpperArm_" + side] in shoulder.get(
                    "children", []
                ), "Arm must inherit its collarbone"

        assert len(values(g, b, skin["inverseBindMatrices"])) == len(joints)
        animations = {a["name"]: a for a in g["animations"]}
        assert clips <= set(animations), f"{name}: missing animation clips"
        for clip in clips:
            animation = animations[clip]
            assert animation["channels"], f"{name}/{clip}: empty animation"
            animated = False
            stroke = 0.0
            for channel in animation["channels"]:
                assert channel["target"]["node"] in joints
                sampler = animation["samplers"][channel["sampler"]]
                times = values(g, b, sampler["input"])
                poses = values(g, b, sampler["output"])
                assert times == sorted(times) and len(times) == len(poses)
                if channel["target"]["path"] == "rotation":
                    dot = abs(sum(x * y for x, y in zip(poses[0], poses[-1])))
                    assert dot > 0.9999, f"{name}/{clip}: rotation jumps at loop seam"
                    bone_name = g["nodes"][channel["target"]["node"]]["name"]
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
                                min(1, abs(sum(x * y for x, y in zip(poses[0], p))))
                            )
                            for p in poses
                        )
                if len(poses) > 1 and poses[0] != poses[len(poses) // 3]:
                    animated = True
            assert animated, f"{name}/{clip}: no animated joint motion"
            if name != "jellyfish" and clip in {"Idle", "Swim"}:
                assert stroke > 0.15, f"{name}/{clip}: tail motion is too small to read"
        if name != "jellyfish":
            texture_limit = 2048 if name == "sabrina" else 512
            assert all(
                max(image_size(g, b, i)) <= texture_limit for i in g["images"]
            ), f"{name}: texture budget exceeded"
        triangles = 0
        for mesh in g["meshes"]:
            for primitive in mesh["primitives"]:
                a = primitive["attributes"]
                assert {
                    "POSITION",
                    "NORMAL",
                    "TEXCOORD_0",
                    "COLOR_0",
                    "JOINTS_0",
                    "WEIGHTS_0",
                } <= set(a)
                pos = values(g, b, a["POSITION"])
                weights = values(g, b, a["WEIGHTS_0"])
                indices = values(g, b, a["JOINTS_0"])
                assert len(pos) == len(weights) == len(indices)
                assert all(
                    abs(sum(w) - 1) < 0.0002 and min(w) >= 0 for w in weights
                ), f"{name}: invalid skin weights"
                assert all(
                    0 <= j < len(joints) for row in indices for j in row
                ), f"{name}: joint index outside skin"
                triangles += len(values(g, b, primitive["indices"])) // 3
        budget = 60000 if name == "sabrina" else 12000 if name == "jellyfish" else 8000
        assert triangles <= budget, f"{name}: triangle budget exceeded ({triangles})"
        assert all(
            "uri" not in image for image in g.get("images", [])
        ), f"{name}: textures must be embedded"
        print(
            f"{name}: {triangles} triangles, {len(joints)} bones, {len(clips)} animated clips; skin and textures OK"
        )
    validate_reef()
    validate_crystal()
    print("All character, prop and environment asset contracts passed")


if __name__ == "__main__":
    main()
