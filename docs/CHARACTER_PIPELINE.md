# Art workflow

Maintained Blender sources live in `art/`. They use packed textures and export to self-contained GLBs in `assets/`. Normal editing does not need the original high-poly inputs.

| Asset | Triangles | Bones | Clips |
|---|---:|---:|---|
| Sabrina | 60,014 | 88 | Idle, Swim, Turn_Left, Turn_Right, Sing, Wave, Hurt, Throw, Dive |
| Each of 13 fish | 7,800 | 6 | Idle, Swim |
| Jellyfish | 9,520 | 28 | Pulse, Nibble, Drift |
| Reef | 150,000 | — | — |
| Ocean crystal | 3,000 | — | — |

## Edit and export

Edit the saved Blender source. Keep the armature's `asset_id`, bone names and action names. Export from the repository root:

```sh
blender --background art/characters/sabrina.blend --python-exit-code 1 --python tools/blender/export_character.py
blender --background art/environments/reef_ruins.blend --python-exit-code 1 --python tools/blender/export_environment.py
blender --background art/props/ocean_crystal.blend --python-exit-code 1 --python tools/blender/export_prop.py
blender --background art/characters/sabrina.blend --python-exit-code 1 --python tools/blender/validate_sabrina_rig.py
python3 tools/validate_models.py
```

Use the same character exporter for fish and jellyfish, changing the source filename. Exporters preserve saved artwork; they do not rebuild it. Commit the Blender source, GLB and affected generated files together.

Keep all 13 fish and the derived models in `assets/render_lods/`. They are runtime assets, not spare copies.

## Sabrina

The maintained model keeps the supplied UVs and 33 packed PBR images. It has a mermaid-specific rig, a 12% longer tail and small head/eye adjustments. Torso proportions are unchanged.

Edit this source directly. `rig_sabrina.py` is a model-specific import and repair tool, not a general auto-rigger. Do not rerun a full import over hand-edited artwork.

Targeted options are available when needed:

| Option | Purpose |
|---|---|
| `--actions-only` | Replace all nine generated actions |
| `--swimming-poses` | Replace Swim, Turn_Left, Turn_Right and Dive |
| `--repair-elbows` | Add elbow supports without replacing existing action channels |
| `--repair-arms` | Update torso/arm weights and actions |
| `--refine-silhouette` | Apply the recorded silhouette changes and refresh actions |

Back up edited actions before replacing them. Run options against the saved source:

```sh
blender --background art/characters/sabrina.blend --python-exit-code 1 \
  --python tools/blender/rig_sabrina.py -- --swimming-poses
```

## Animation and contacts

Clips animate in place. The simulation owns movement, turning and collision.

Keep the crystal release at frame 33 of the 48-frame Throw action, authored at 30 FPS. Casting corrections and exported contacts share the same 30-degree presentation yaw.

Character export writes `sabrina.contacts.json` and regenerates `generated_contacts.rs`. Re-export after changing actions or contact geometry; do not edit generated Rust manually.

```sh
python3 tools/export_rust_contacts.py --check
```

The rig validator checks deformation, hand travel, cast reach and loop continuity. Elbow thickness must remain within 5% of rest in both cross-section dimensions.

## LODs

After changing fish or reef geometry:

```sh
blender --background --python tools/blender/export_render_lods.py
python3 tools/validate_render_lods.py
```

This creates two smaller meshes per asset without changing the source. LODs share the base textures, materials and skin. Regeneration is manual; there is no source-hash tracking.

## Keep these contracts

- glTF converts Blender Z-up to Y-up. Sabrina and fish face +X; jellyfish face +Z. Sabrina's runtime scale is 0.88.
- Preserve `Hand_R`, six connected tail joints, finger and fin chains, and the `Blink`, `SoftSmile` and `Singing` facial morphs.
- Hair stays one opaque surface with exported tangents. Pin scalp vertices to `Head` and preserve the five hair chains. Do not add facial morphs or baked hair animation; secondary motion runs in-game.
- Fish use `Root`, `Spine`, `Peduncle`, `Caudal`, `Fin_L` and `Fin_R`, with one material surface each.
- Keep `Jelly_Skin` and `Jelly_Face` separate. Only the skin is translucent.
- Preserve normals, UVs, vertex colors and normalized weights, with no more than four joint influences per vertex.
- Sabrina's limits are 65K triangles, 96 bones and 2K textures. Fish textures are limited to 512px; the crystal stays below 3,100 triangles.

Check materials, deformation and hair motion in the game, not just Blender. Use captures for close-ups and encounter views.

UI SVG sources live in `assets/ui/reef/`. Export runtime PNGs with `python3 tools/export_bevy_ui.py`; this requires ImageMagick.