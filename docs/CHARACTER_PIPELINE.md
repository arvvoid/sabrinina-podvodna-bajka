# Art workflow

The maintained sources are the `.blend` files under `art/`. Models use packed textures and export to self-contained GLBs in `assets/`. External high-poly inputs are not needed for ordinary editing.

| Asset | Triangles | Bones | Clips |
|---|---:|---:|---|
| Sabrina | 38,126 | 32 | Idle, Swim, Turn_Left, Turn_Right, Sing, Wave, Hurt, Throw |
| Each of twelve fish | 7,800 | 6 | Idle, Swim |
| Jellyfish | 9,520 | 28 | Pulse, Nibble, Drift |
| Reef | 150,000 | — | — |

## Editing and exporting

Open the saved source, edit the mesh, weights, materials or actions, then save. Keep the armature's `asset_id` and named bones/actions. Export from the repository root:

```sh
blender --background art/characters/sabrina.blend --python tools/blender/export_character.py
blender --background art/environments/reef_ruins.blend --python tools/blender/export_environment.py
blender --background art/props/ocean_crystal.blend --python tools/blender/export_prop.py
python3 tools/validate_models.py
```

Substitute another character's filename to export it. Exporters use the saved artwork; they do not rebuild geometry or replace poses. Commit the source, GLB and updated manifest together.

Sabrina exports also sample arm contacts into `art/characters/sabrina.contacts.json` and regenerate `rust/reef_sim/src/generated_contacts.rs`. These samples follow the visible clips while keeping collision independent of rendering. `python3 tools/export_rust_contacts.py --check` checks the generated Rust file.

After changing fish or reef geometry, regenerate derived meshes:

```sh
blender --background --python tools/blender/export_render_lods.py
python3 tools/validate_render_lods.py
```

This keeps the source untouched and exports two smaller meshes per asset. Runtime LODs share the original textures, materials and skin. The validator checks source-export hashes, joint order, bind matrices, weights and geometry attributes.

## Contracts worth preserving

- glTF converts Blender Z-up to Y-up. Fish and Sabrina swim toward +X; jellyfish face +Z. Sabrina is scaled to 0.88 in the game.
- Clips animate in place. Simulation owns world movement, turning and contact response.
- Sabrina needs `Hand_R` for crystal attachment, six tail joints, three three-joint hair chains, and neutral `Blink`, `SoftSmile`, `Singing` morphs. Crown weights stay attached to the skull. Hair is a skinned mesh, not a strand simulation.
- Fish share the Root, Spine, Peduncle, Caudal, Fin_L and Fin_R bone contract. Keep one material surface per fish.
- Jellyfish use separate `Jelly_Skin` and `Jelly_Face` materials. Runtime translucency applies only to the skin so the eyes and expression remain readable.
- Texture limits are 2K for Sabrina and 512px per fish. Include normals, UVs, vertex colors and normalized weights, with at most four joint influences per vertex.

Check materials and deformation in the actual game lighting, not just Blender. Use the capture options in the README for close-ups and encounter views. `tools/blender/render_preview.py` is available for authoring previews. UI SVG sources live in `assets/ui/reef/`; `python3 tools/export_bevy_ui.py` exports their runtime PNGs using ImageMagick.
