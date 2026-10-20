# Art workflow

The maintained sources are the `.blend` files under `art/`. Models use packed textures and export to self-contained GLBs in `assets/`. External high-poly inputs are not needed for ordinary editing.

| Asset | Triangles | Bones | Clips |
|---|---:|---:|---|
| Sabrina | 60,014 | 88 | Idle, Swim, Turn_Left, Turn_Right, Sing, Wave, Hurt, Throw, Dive |
| Each of thirteen fish | 7,800 | 6 | Idle, Swim |
| Jellyfish | 9,520 | 28 | Pulse, Nibble, Drift |
| Reef | 150,000 | — | — |
| Ocean crystal | 3,000 | — | — |

## Editing and exporting

Open the saved source, edit the mesh, weights, materials or actions, then save. Keep the armature's `asset_id` and named bones/actions. Export from the repository root:

```sh
blender --background art/characters/sabrina.blend --python-exit-code 1 --python tools/blender/export_character.py
blender --background art/environments/reef_ruins.blend --python tools/blender/export_environment.py
blender --background art/props/ocean_crystal.blend --python-exit-code 1 --python tools/blender/export_prop.py
blender --background art/characters/sabrina.blend --python-exit-code 1 --python tools/blender/validate_sabrina_rig.py
python3 tools/validate_models.py
```

Substitute another character's filename to export it. Exporters use the saved artwork; they do not rebuild geometry or replace poses. Commit the source and GLB together. Only the game-ready Blender sources are maintained; original high-poly inputs and Blender backup files are not needed.

Keep all thirteen fish sources and exports, plus the derived fish/reef models in
`assets/render_lods/`: they are used by the game, not surplus copies. Blender
backups, Python caches and local Cargo build outputs are disposable and ignored
by Git. The rigging, export and validation tools remain part of the maintained
pipeline.

### Sabrina's textured mermaid source

The current character uses the owner's eleven-part, textured Tripo T-pose. Its
60,014 triangles, original UVs and 33 embedded PBR images are preserved.
A uniform scale/rotation places the rest mesh in game space; the tail is now
12% longer for a more mature silhouette. The head is subtly narrowed (5%) and
slightly reduced vertically (2%), with 5% smaller eye regions; neck/scalp blends
and the facial morphs follow the same edit. Torso proportions remain unchanged.
The supplied maps range from 256px to 2K, not 4K. Both PNG and JPEG decoding are
enabled in the game. Runtime materials retain the authored metallic/roughness
maps rather than forcing all skin, shells and scales to the same finish.

The import-time authoring tool is specific to that inspected model:

```sh
blender --background --python-exit-code 1 --python tools/blender/rig_sabrina.py \
  -- /path/to/mermaid.glb /path/to/candidate.blend
```

It replaces the incorrect humanoid lower-body rig, uses the supplied finger
landmarks/weights where appropriate, creates the game clips and records the
input filename and SHA-256 on the armature. It is not a general-purpose auto-rig.
Use a candidate output when importing; normal edits and exports should use the
maintained source, without rerunning the importer over hand-edited artwork.

To deliberately replace only the nine generated actions (not geometry,
weights, textures or rig), back up artist-edited actions first, then run:

```sh
blender --background art/characters/sabrina.blend --python-exit-code 1 \
  --python tools/blender/rig_sabrina.py -- --actions-only
```

Idle retains slow hand sculling. Level/upward swimming and turns hold relaxed
arms alongside the body with small secondary movements; downward diving extends
both hands ahead. Dive selection uses smoothed body pitch below -0.3 radians,
downward velocity below -0.5 and swimming effort above 0.12; casting, singing
and hurt poses retain priority. Upper arms retain their original shoulder roll, elbows use a
single anatomical hinge and wrist bending is bounded. Shoulder/elbow pivots
follow the supplied upper-body landmarks, rather than the middle of the upper
arm/forearm; the clavicles share a small part of raised-arm motion. Skin weights concentrate
elbow and wrist deformation around the joints rather than across whole limbs.
Each elbow has a half-angle support joint, blended into the upper arm and
forearm, to retain thickness through tight bends under linear skinning.
These joints are siblings of the forearms, so hand trajectories do not change.
The wave has a fast open-palmed two-handed push. The crystal gathers, winds up,
extends at 1.1 seconds, then follows through. Runtime casting corrections align
the arms with the spell direction; sampled contacts use the same correction.
Keep the release at frame 33 of the 48-frame Throw action (30 FPS). The Blender
validator checks every frame for deformation, hand travel and loop continuity,
and checks both casts' forward reach. Both elbow cross-section dimensions must
stay within 5% of their rest thickness throughout every clip.
Export contacts again after editing actions.

For older sources with the corrected anatomical arm pivots, `--repair-elbows`
adds the two support joints, local elbow weights and sampled half-angle tracks
without replacing existing action channels, mesh geometry, proportions or textures:

```sh
blender --background art/characters/sabrina.blend --python-exit-code 1 \
  --python tools/blender/rig_sabrina.py -- --repair-elbows
```

`--repair-arms` updates only torso/arm joint weights and actions in an opened
Sabrina source, retaining the supplied finger weights. `--refine-silhouette`
applies the 12% tail extension to rest geometry, shape keys and the tail/fin
skeleton, recording its scale on the rig so repeated runs do not accumulate.
It also applies the subtle head refinement once and refreshes actions. Neither operation changes UVs or textures. The
simulation tail contact lengths must match the authored extension.

`--swimming-poses` replaces only Swim, Turn_Left, Turn_Right and Dive, preserving
the existing idle and ability actions. Re-export the model and contacts afterward.

Sabrina is presented 30 degrees toward the camera on either facing,
blending continuously through turns. Casting correction and sampled contacts
use the same presentation yaw so spell direction and the held crystal agree.

The separate supplied braided hair was inspected but not attached: its broad,
flat-ended silhouette and 120 material sections need further fitting and
consolidation. The integrated curls already fit the scalp and share the
character's textures.

Sabrina exports also sample arm contacts into `art/characters/sabrina.contacts.json` and regenerate `src/reef_sim/src/generated_contacts.rs`. These samples follow the visible clips while keeping collision independent of rendering. `python3 tools/export_rust_contacts.py --check` checks the generated Rust file.

After changing fish or reef geometry, regenerate derived meshes:

```sh
blender --background --python tools/blender/export_render_lods.py
python3 tools/validate_render_lods.py
```

This keeps the source untouched and exports two smaller meshes per asset. Runtime LODs share the original textures, materials and skin. The validator checks joint order, bind matrices, weights and geometry attributes against the base GLBs. There is no manifest or source-hash tracking, so regenerate LODs whenever the base geometry changes.

## Contracts worth preserving

- glTF converts Blender Z-up to Y-up. Fish and Sabrina swim toward +X; jellyfish face +Z. Sabrina is scaled to 0.88 in the game.
- Clips animate in place. Simulation owns world movement, turning and contact response.
- Sabrina needs `Hand_R` for crystal attachment, six connected `Tail_1` through `Tail_6` joints, and neutral `Blink`, `SoftSmile`, `Singing` morphs on the body/face mesh only. There are no thigh, knee, calf, foot or toe joints. Each hand has fifteen finger joints.
- `Fluke_L/R` and `FlukeTip_L/R` independently bend the two terminal fins. Each of the four smaller fins has a two-joint `HipFin_L/R` or `LowerFin_L/R` chain, weighted separately from the main tail.
- `Sabrina_Hair` is one opaque, 20,123-triangle sculpted surface with the original UVs and PBR maps. Export tangents. Keep scalp and face-adjacent vertices pinned to `Head`; the lengths blend across five chains named `Hair_0_1` through `Hair_4_5`. Do not add facial morphs or baked animation tracks to the hair.
- Hair's rest shape is authored in Blender. Runtime chains use a fixed swept-back bend, damped phase-delayed sway and bounded reactions to acceleration/turning, with no world-up buoyancy. The fixed bend keeps the long curls clear of the raised chin and chest. This is stylized secondary animation, not strand physics or collision simulation. Inspect motion in-game: Blender previews show only the authored hairstyle. Pause freezes the chains; reduced motion scales dynamic targets to 25%.
- The small crystal is limited to 3,100 triangles (currently 3,000, down from 14,000), with its original packed 1K maps, scale and ornament silhouette. Fine ornament facets can differ in close-ups.
- Fish share the Root, Spine, Peduncle, Caudal, Fin_L and Fin_R bone contract. Keep one material surface per fish.
- Jellyfish use separate `Jelly_Skin` and `Jelly_Face` materials. Runtime translucency applies only to the skin so the eyes and expression remain readable.
- Texture limits are 2K for Sabrina and 512px per fish. Include normals, UVs, vertex colors and normalized weights, with at most four joint influences per vertex. Sabrina's budgets are 65K triangles and 96 bones; other characters retain their smaller budgets.

Check materials and deformation in the actual game lighting, not just Blender. Use the capture options in the README for close-ups and encounter views. `tools/blender/render_preview.py` is available for authoring previews. UI SVG sources live in `assets/ui/reef/`; `python3 tools/export_bevy_ui.py` exports their runtime PNGs using ImageMagick.
