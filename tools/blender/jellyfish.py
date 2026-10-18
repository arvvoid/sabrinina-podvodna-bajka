# SPDX-License-Identifier: MIT
import math
from geometry import material, surface, ellipsoid, tube, chain_weights, finish_meshes
from rigging import armature, actions, rotate


def build():
    skin = material("Jelly_Skin", roughness=0.24, metallic=0.18)
    detail = material("Jelly_Face", roughness=0.24)
    parts = []

    def add(obj):
        parts.append(obj)
        return obj

    bones = [
        ("Root", (0, 0, 0), (0, 0.25, 0), None),
        ("Bell", (0, 0, 0), (0, 0.8, 0), "Root"),
        ("Mouth", (0, 0.14, 0.70), (0, 0.28, 0.70), "Bell"),
    ]
    for k in range(8):
        a = k * math.tau / 8
        x = 0.62 * math.cos(a)
        z = 0.48 * math.sin(a)
        length = 1.25 + 0.26 * math.sin(k * 1.7)
        for i in range(3):
            bones.append(
                (
                    f"Tentacle_{k}_{i}",
                    (x, -0.05 - i * length / 3, z),
                    (x, -0.05 - (i + 1) * length / 3, z),
                    "Bell" if i == 0 else f"Tentacle_{k}_{i-1}",
                )
            )
    bones += [("Crest", (-0.12, 0.84, -0.05), (0.02, 1.20, -0.05), "Bell")]
    rig = armature("Jelly_Rig", bones)

    def bell(u, v):
        a = u * math.pi * 0.53
        b = v * math.tau
        r = math.sin(a) * (0.79 + 0.025 * math.cos(b * 10) * u**5)
        return (
            r * math.cos(b),
            0.04 + 0.81 * math.cos(a) + 0.045 * math.cos(b * 10) * u**5,
            r * 0.86 * math.sin(b),
        )

    def pearl(u, v, p):
        stripe = (0.5 + 0.5 * math.cos(v * math.tau * 10)) ** 8
        return (
            0.54 + 0.18 * stripe + 0.06 * u,
            0.29 + 0.17 * stripe + 0.06 * u,
            0.83 + 0.10 * stripe,
            1,
        )

    add(surface("Scalloped_bell", 22, 48, bell, skin, pearl, "Bell"))

    # Soft scalloped rolled hem and internal frill.
    def hem(u, v):
        a = v * math.tau
        phi = u * math.tau
        r = 0.78 + 0.055 * math.cos(phi)
        return (
            r * math.cos(a),
            0.055 * math.sin(phi) - 0.015 + 0.04 * math.cos(a * 10),
            r * 0.86 * math.sin(a),
        )

    add(surface("Bell_frill", 8, 48, hem, skin, (0.80, 0.48, 0.83, 1), "Bell"))
    for k in range(8):
        a = k * math.tau / 8
        x = 0.62 * math.cos(a)
        z = 0.48 * math.sin(a)
        length = 1.25 + 0.26 * math.sin(k * 1.7)
        points = [
            (x, 0, z),
            (x * 1.05, -0.35, z + 0.04),
            (x * 0.75, -length * 0.8, z - 0.09),
            (x * 0.83, -length, z + 0.09),
        ]
        wt = lambda u, v, p, k=k, length=length: chain_weights(
            p[1],
            [
                (-length, f"Tentacle_{k}_2"),
                (-length * 0.5, f"Tentacle_{k}_1"),
                (-0.1, f"Tentacle_{k}_0"),
            ],
        )
        color = lambda u, v, p: (0.64 + 0.17 * u, 0.39 + 0.18 * u, 0.85 + 0.10 * u, 1)
        add(
            tube(
                "Ribbon_tentacle_" + str(k),
                points,
                0.055 if k % 2 else 0.075,
                skin,
                color,
                wt,
                18,
                8,
                taper=0.12,
                flat=0.70,
            )
        )
    for side in [-1, 1]:
        x = side * 0.26
        y = 0.39 + (0.03 if side > 0 else 0)
        add(
            ellipsoid(
                "Eye_white",
                (x, y, 0.66),
                (0.19, 0.22, 0.095),
                detail,
                (0.96, 0.95, 0.84, 1),
                "Bell",
                12,
                18,
            )
        )
        add(
            ellipsoid(
                "Iris",
                (x + 0.025, y - 0.01, 0.748),
                (0.095, 0.12, 0.033),
                detail,
                (0.10, 0.30, 0.35, 1),
                "Bell",
                10,
                16,
            )
        )
        add(
            ellipsoid(
                "Pupil",
                (x + 0.029, y - 0.015, 0.778),
                (0.052, 0.074, 0.017),
                detail,
                (0.018, 0.026, 0.049, 1),
                "Bell",
                8,
                12,
            )
        )
        add(
            ellipsoid(
                "Eye_glint",
                (x + 0.008, y + 0.028, 0.797),
                (0.027, 0.031, 0.009),
                detail,
                (1, 0.99, 0.90, 1),
                "Bell",
                6,
                10,
            )
        )
        add(
            ellipsoid(
                "Rosy_cheek",
                (side * 0.47, 0.20, 0.58),
                (0.10, 0.043, 0.018),
                detail,
                (0.96, 0.37, 0.49, 1),
                "Bell",
                8,
                14,
            )
        )
        add(
            tube(
                "Expressive_brow",
                [
                    (x - 0.14, y + 0.22, 0.61),
                    (x - 0.06, y + 0.30, 0.66),
                    (x + 0.06, y + 0.27, 0.65),
                    (x + 0.13, y + 0.20, 0.61),
                ],
                0.025,
                skin,
                (0.47, 0.21, 0.66, 1),
                "Bell",
                12,
                7,
                taper=0.5,
            )
        )
    add(
        ellipsoid(
            "Smiling_mouth",
            (0.02, 0.13, 0.70),
            (0.145, 0.085, 0.035),
            detail,
            (0.19, 0.035, 0.15, 1),
            "Mouth",
            10,
            18,
        )
    )
    add(
        ellipsoid(
            "Tongue",
            (0.045, 0.09, 0.729),
            (0.075, 0.025, 0.015),
            detail,
            (0.98, 0.32, 0.47, 1),
            "Mouth",
            7,
            12,
        )
    )
    add(
        ellipsoid(
            "Funny_tooth",
            (-0.025, 0.18, 0.733),
            (0.041, 0.037, 0.015),
            detail,
            (0.98, 0.94, 0.78, 1),
            "Mouth",
            7,
            10,
        )
    )
    for k in range(3):
        add(
            tube(
                "Crest_plume_" + str(k),
                [
                    (-0.18 + k * 0.12, 0.78, -0.05),
                    (-0.24 + k * 0.12, 1.03, -0.05),
                    (-0.14 + k * 0.14, 1.26, -0.05),
                    (-0.05 + k * 0.14, 1.07, -0.02),
                ],
                0.065,
                skin,
                (0.85, 0.47, 0.82, 1),
                "Crest",
                12,
                8,
                taper=0.45,
            )
        )
    # Small luminous freckles are vertices in the same skinned surface.
    for k in range(12):
        a = k * 2.399
        yy = 0.46 + 0.23 * math.sin(k * 1.5)
        x = 0.59 * math.sin(a)
        if abs(x) < 0.40:
            continue
        z = 0.86 * math.sqrt(max(0, 0.79**2 - x * x - (yy - 0.04) ** 2))
        add(
            ellipsoid(
                "Freckle_" + str(k),
                (x, yy, z + 0.018),
                (0.026, 0.022, 0.012),
                skin,
                (0.94, 0.75, 0.96, 1),
                "Bell",
                5,
                8,
            )
        )
    objects = finish_meshes(rig, parts)

    def pose(clip, u, r):
        t = u * math.tau
        p = 1 + 0.075 * math.sin(t)
        r.pose.bones["Bell"].scale = (p, 1 - 0.105 * math.sin(t), p)
        for k in range(8):
            for i in range(3):
                lag = t - i * 0.70 + k * 0.32
                rotate(
                    r,
                    f"Tentacle_{k}_{i}",
                    (
                        0.13 * math.sin(lag),
                        0.055 * math.cos(lag),
                        (0.10 + 0.04 * i) * math.cos(lag + 0.45),
                    ),
                )
        rotate(r, "Crest", (0.07 * math.sin(t - 0.6), 0, 0.13 * math.sin(t - 0.8)))
        if clip == "Nibble":
            r.pose.bones["Mouth"].scale = (1, 1.4 + 0.2 * math.sin(t), 1)
        if clip == "Drift":
            rotate(
                r, "Bell", (0.04 * math.sin(t), 0.08 * math.sin(t), 0.065 * math.sin(t))
            )

    actions(rig, [("Pulse", 2.0), ("Nibble", 0.8), ("Drift", 3.2)], pose)
    return rig, objects
