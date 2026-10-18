# SPDX-License-Identifier: MIT
"""Author initial .blend sources and game exports. Use export_character.py after hand edits."""

import sys, json
from pathlib import Path
import bpy

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import jellyfish


def clean():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = 30


def output(name, build):
    clean()
    rig, objects = build()
    rig["asset_id"] = name
    rig["forward_axis"] = "Fish +X; Sabrina and jellyfish face +Z in Godot."
    rig["units"] = "Godot metres; Blender Z-up converted by glTF exporter."
    rig["animation_contract"] = (
        "In-place loops. Native ECS owns position, steering and contacts."
    )
    bpy.context.scene.world = bpy.data.worlds.new("Underwater_preview")
    bpy.context.scene.world.color = (0.055, 0.10, 0.13)
    # Save a usable material-preview viewport and a clearly selected rig.
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    bpy.context.view_layer.objects.active = rig
    for screen in bpy.data.screens:
        for area in screen.areas:
            if area.type == "VIEW_3D":
                area.spaces.active.shading.type = "MATERIAL"
                area.spaces.active.region_3d.view_distance = 7
    source = ROOT / "art/characters" / f"{name}.blend"
    bpy.ops.wm.save_as_mainfile(filepath=str(source))
    target = ROOT / "game/assets/models" / f"{name}.glb"
    bpy.ops.export_scene.gltf(
        filepath=str(target),
        export_format="GLB",
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
        export_materials="EXPORT",
        export_cameras=False,
        export_lights=False,
    )
    return {
        "asset": name,
        "source": str(source.relative_to(ROOT)),
        "export": str(target.relative_to(ROOT)),
        "bones": len(rig.data.bones),
        "vertices": sum(len(o.data.vertices) for o in objects),
        "triangles": sum(
            sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objects
        ),
        "surfaces": len(objects),
        "clips": [a.name for a in bpy.data.actions],
        "bytes": target.stat().st_size,
    }


args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
selection = args[0] if args else "all"
builders = [("jellyfish", jellyfish.build)]
if selection not in {"all", *[name for name, _ in builders]}:
    raise RuntimeError(
        "Only jellyfish remains procedural. Supplied characters use export_character.py."
    )
stats = []
for name, build in builders:
    if selection in ("all", name):
        stats.append(output(name, build))
manifest = ROOT / "art/characters/manifest.json"
if manifest.exists():
    old = json.loads(manifest.read_text())
    rebuilt = {s["asset"] for s in stats}
    stats = [s for s in old["assets"] if s["asset"] not in rebuilt] + stats
manifest.write_text(
    json.dumps(
        {
            "authoring_blender": bpy.app.version_string,
            "license": "Character artwork reserved; see ASSET_LICENSE.md. Authoring scripts MIT.",
            "assets": stats,
        },
        indent=2,
    )
    + "\n"
)
print("CHARACTER BUILD COMPLETE", json.dumps(stats))
