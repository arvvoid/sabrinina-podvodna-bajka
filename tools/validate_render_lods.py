#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Check derived render meshes can safely reuse the original skins/materials."""

import hashlib
import json
from validate_models import ROOT, read_glb, values


def main():
    records = json.loads((ROOT / "assets/render_lods/manifest.json").read_text())
    assert len(records) == 13
    for record in records:
        base_path = ROOT / record["base_export"]
        assert (
            hashlib.sha256(base_path.read_bytes()).hexdigest() == record["base_sha256"]
        )
        base, base_bytes = read_glb(base_path)
        lod, binary = read_glb(ROOT / record["export"])
        assert not lod.get("images") and not lod.get("animations")
        if base.get("skins"):
            assert len(base["skins"]) == len(lod["skins"]) == 1
            a, b = base["skins"][0], lod["skins"][0]
            assert [base["nodes"][j]["name"] for j in a["joints"]] == [
                lod["nodes"][j]["name"] for j in b["joints"]
            ]
            matrices_a = values(base, base_bytes, a["inverseBindMatrices"])
            matrices_b = values(lod, binary, b["inverseBindMatrices"])
            assert (
                max(
                    abs(x - y)
                    for row_a, row_b in zip(matrices_a, matrices_b)
                    for x, y in zip(row_a, row_b)
                )
                < 1e-5
            )
        by_name = {m["name"]: m for m in lod["meshes"]}
        for source in base["meshes"]:
            previous = sum(
                base["accessors"][p["indices"]]["count"] for p in source["primitives"]
            )
            for level in (1, 2):
                mesh = by_name[source["name"] + f"__LOD{level}"]
                assert len(mesh["primitives"]) == len(source["primitives"])
                count = 0
                for p in mesh["primitives"]:
                    attrs = p["attributes"]
                    assert {"POSITION", "NORMAL", "TEXCOORD_0"} <= attrs.keys()
                    positions = values(lod, binary, attrs["POSITION"])
                    indices = values(lod, binary, p["indices"])
                    assert all(0 <= i[0] < len(positions) for i in indices)
                    count += len(indices)
                    if base.get("skins"):
                        weights = values(lod, binary, attrs["WEIGHTS_0"])
                        joints = values(lod, binary, attrs["JOINTS_0"])
                        assert all(
                            abs(sum(w) - 1) < 0.0002 and min(w) >= 0 for w in weights
                        )
                        assert all(
                            0 <= j < len(lod["skins"][0]["joints"])
                            for row in joints
                            for j in row
                        )
                assert 0 < count < previous
                previous = count
        print(
            record["asset"]
            + ": LOD topology, UVs, skin order/bind matrices and source hash OK"
        )


if __name__ == "__main__":
    main()
