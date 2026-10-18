# Blender character workflow

The game renders authored, skinned GLB characters. Editable Blender sources live outside the Godot project so players and CI do not need Blender. Gameplay stays in the native entity/component simulation; animation is a presentation concern.

## Sources and budgets

Authoring version: Blender **5.2.2 LTS**. Each source contains one named armature, weighted meshes, UV coordinates, vertex colors, packed textures and editable pose-key actions. Sabrina derives from the owner-supplied neutral-pose FBX; all twelve fish derive from the supplied fish GLB. Their hashes and preparation are recorded in `sabrina.provenance.json` and `fish.provenance.json`. The jellyfish retains its original project source.

| Sources in `art/characters/` | Triangles each | Bones | Material surfaces | Actions |
|---|---:|---:|---:|---|
| `sabrina.blend` | 38,126 | 32 | 1 | Idle, Swim, Turn_Left, Turn_Right, Sing, Wave, Hurt, Throw |
| Twelve fish sources listed in `fish.provenance.json` | 7,800 | 6 | 1 | Idle, Swim |
| `jellyfish.blend` | 9,520 | 28 | 2 | Pulse, Nibble, Drift |

The fish collection includes clownfish, two blue tang designs, a yellow tang, copperband butterflyfish, bannerfish, emperor angelfish, pink and orange anthias, cyan chromis, fire goby and cardinalfish. The legacy `parrotfish` and `blue_wrasse` asset names now hold the pink anthias and cyan chromis. Twelve visual designs pair with the existing six simulation appearance groups, preserving scoring, seeded simulation and resource balance. Pool-slot parity selects the visual variant without extra random calls.

The manifest records source/export pairs and statistics. Sabrina uses one embedded 2K color map with constant roughness/metallic values. The supplied neutral-pose source has no normal map. Each fish uses 512px color and normal maps, shared across instances of that design; its roughness and metallic values are constant. Godot extracts texture copies beside each GLB and generates mesh LODs during import. The original fish collection contains about 1.88 million triangles; the twelve game meshes total 93,600.

Sabrina uses the supplied neutral-pose figure, with one subdivision pass to soften its silhouette. The neutral arms permit separate collarbone, shoulder, elbow and wrist weights. Explicit front/back/side hip and fin seeds separate the upper tail from nearby hair; preparation rejects any hair/arm influence in the solid upper-tail core. Eight keyed clips keep a quiet upper torso, restrained palm sculling and a travelling six-joint tail wave. Wave casting aims the arm segments forward without stretching them. The separate 1.6-second Throw clip reaches, draws back, releases at 1.1 seconds and follows through. Keys stay in a consistent quaternion hemisphere so interpolation cannot flip an elbow. A modest forward spine angle and restrained elbows keep the body streamlined; native neck/head correction follows dives instead of pulling the face back toward horizontal; hair roots counter that correction. The model remains a reconstructed surface with some coarse hair/fin edges; facial animation, blinking and individual finger articulation are not included. Mobile device budgets still need platform testing.

Only the final Blender source/export pairs and required textures are maintained in the project. Discarded inputs, backup files and old previews have been removed. Initial preparation recipes below are optional reconstruction tools and require externally supplied inputs; ordinary editing and exporting needs only the saved Blender sources.

## Edit and export

1. Open the relevant `.blend` in Blender. The armature is selected. Use Material Preview to see vertex colors and packed texture detail.
2. Edit the mesh, material or weights. In the Dope Sheet's Action Editor, select an action to edit its pose keys. Preserve the named attachment bones and required actions below. Keep actions saved with a fake user so unused clips survive saving.
3. Save the Blender file. From the repository root, export that saved source:

```sh
blender --background art/characters/sabrina.blend --python tools/blender/export_character.py
python3 tools/validate_models.py
./tools/godot --headless --editor --path game --import --frame-delay 800
./tools/run.sh
```

The exporter reads the armature's `asset_id`, exports only the rig and its skinned meshes, and updates the manifest. It does not regenerate geometry, weights or keyframes. Substitute another source filename to export another character. Commit the `.blend`, corresponding `.glb`, manifest and any intentional `.glb.import` changes together. Blender backup files and Python caches are ignored.

Blender is an authoring dependency only. The game reads `game/assets/models/*.glb` through the cached native character library. Shader or material edits should be checked in the actual underwater lighting as well as Blender's preview.

## Coordinate and animation contracts

- glTF converts Blender Z-up into Godot Y-up. One source unit is one world unit. Fish and the authored Sabrina swim rig face Godot +X; jellyfish face +Z. Native presentation scales Sabrina to 0.88 and applies existing fish size variation.
- Use in-place animation. Native simulation owns world position, steering, contacts and resource timing. Animated root translation must not move the gameplay entity.
- Sabrina's `Hand_L` and `Hand_R` support relaxed wrist strokes and the two-hand wave-casting gesture. Tail_1 through Tail_6 carry a phase-delayed wave, with delayed flukes, tips and hair. The 3.2-second resting loop actively treads water with the tail and sculls the hands; the 1.8-second swim loop increases the stroke and counters it through the chest. Singing and wave casting retain a supporting tail stroke.
- Each fish preserves Root, Spine, Peduncle, Caudal, Fin_L and Fin_R. Swim is a 1.2-second cycle. Native playback scales its rate to the fish's stroke frequency and seeds its phase from simulation state.
- Jellyfish preserve Root, Bell, Mouth and the eight three-joint tentacle chains. The `Jelly_Skin` material name lets native code apply three visitor tints without duplicating meshes.
- Runtime loops clips and blends transitions over 0.22 seconds. State selection controls singing, wave casting and hurt. Sabrina’s stroke phase, yaw and pitch come from simulation so her tail contacts follow the visible pose. Native resting corrections curl the upper tail and keep her gaze forward. Pause freezes animation; reduced motion slows the stroke clock. Turns use a camera-facing half-circle without automatic full spins.
- Sabrina uses a baked base-color map with constant roughness and metallic values. Fish use baked color and normal maps; jellyfish use the `Color` vertex attribute. Textured models retain white vertex colors for the same loading contract. Native loading explicitly enables vertex-color albedo on imported materials. Avoid adding another shader that deforms the same character vertices.
- Fish instances share mesh, material, skin and animation resources; each has its own skeleton pose. One surface per fish bounds draw submission overhead. The current animation system is CPU-driven; it is not a GPU crowd renderer.

## Initial authoring scripts

`tools/blender/geometry.py`, `rigging.py` and `textures.py` provide reusable mesh, weight, animation and packed-texture helpers. The jellyfish builder defines its form and keyed poses. The old procedural fish and Sabrina builders have been removed so a general rebuild cannot overwrite supplied characters.

**Rebuilding replaces the corresponding saved source and export, including manual edits.** Use these only when intentionally regenerating:

```sh
blender --background --factory-startup --python tools/blender/build_characters.py -- jellyfish
blender --background --factory-startup --python tools/blender/prepare_imported_fish.py -- '/path/to/tropical fish 3d model.glb'
```

The fish preparation checks the source hash, separates twelve connected mesh islands, reduces each to 7,800 triangles, rebakes textures, normalizes forward direction/size, and weights the body, peduncle, caudal fan and pectoral fins. Its coordinate-based identification is specific to this collection.

Normal artwork maintenance uses `export_character.py`, not the builder.

The current preparation builds a **candidate directory**, leaving shipped artwork untouched until reviewed:

```sh
blender --background --python tools/blender/prepare_tpose_sabrina.py -- '/path/to/mermaid+3d+model.fbx' build/tpose-candidate
```

Keep the source FBX beside its `.fbm` texture directory. The recipe checks its source hash, separates body/hair/arm regions using surface distances, bakes the rainbow costume, adds collarbones, subdivides the silhouette, limits skinning to four influences, and creates eight editable actions. To adopt a reviewed candidate, save it as `art/characters/sabrina.blend` and run the normal exporter.

The native tail contact chain retains the same hip-relative landmarks, stroke phase and wave coefficients. `export_character.py` also runs `export_sabrina_contacts.py` for Sabrina: 32 shoulder/elbow/fingertip and right-palm samples per action become `src/simulation/generated/sabrina_contacts.h`. These compact samples keep the engine-independent soft-contact solver aligned with the rendered arms, including casting and yaw. The exported right-palm offset also supplies the rendered crystal attachment; the simulation uses matching samples for its launch position. Commit this generated header with the mesh and manifest. `sabrina_pose.h` shares clip selection and timing between simulation contacts and presentation. Rebuild C++ after exporting changed poses.

`tools/blender/sabrina_animation.py` holds the shared pose recipe used by initial preparation. Normal editing still uses the saved Blender actions. The recipe can intentionally replace only the animation keys while keeping the reviewed mesh, weights and materials:

```sh
blender --background art/characters/sabrina.blend --python tools/blender/sabrina_animation.py
blender --background art/characters/sabrina.blend --python tools/blender/export_character.py
```

## Preview and checks

Render an isolated source pose without modifying the saved source:

```sh
mkdir -p build/character-previews
blender --background art/characters/sabrina.blend --python tools/blender/render_preview.py -- build/character-previews/sabrina.png 20 Swim
```

Capture the actual game or record the scripted swim/turn/song/wave showcase:

```sh
./tools/run.sh -- --capture-pose-dive
./tools/run.sh -- --capture-pose-climb-left
./tools/run.sh -- --capture-pose-throw-release
./tools/run.sh -- --capture-throw
./tools/run.sh -- --capture-design
./tools/run.sh -- --capture-school
./tools/run.sh -- --capture-bumps
./tools/run.sh --write-movie "$PWD/build/character-previews/rig.avi" --fixed-fps 30 -- --capture-rig
./tools/run.sh -- --capture-wave
./tools/run.sh -- --capture-depth
./tools/run.sh --write-movie "$PWD/build/character-previews/swimming.avi" --fixed-fps 30 -- --capture-animation
```

The asset validator runs in CI without Blender. It checks required clips/bones, animated joint motion, closed rotation loops, minimum readable tail strokes in Idle/Swim, finite values, normalized skin weights, embedded textures and triangle budgets. Native input checks additionally verify skin bindings, all twelve imported fish, a full idle tail stroke, rendered-tail/contact alignment, bounded upper-body rotation over repeated idle cycles, gaze limits at rest/ascent/descent, authored arm/contact alignment in every clip and paused playback. Visual inspection is still needed for silhouette, clipping, deformation and how motion feels.

See the [Blender glTF documentation](https://docs.blender.org/manual/en/latest/addons/import_export/scene_gltf2.html) for export/material conventions and [Godot Skeleton3D](https://docs.godotengine.org/en/stable/classes/class_skeleton3d.html) for the native attachment interface. Licensing is recorded in `ASSET_LICENSE.md`.

### Additional supplied assets, October 18

The supplied `mermaid 3d model.zip` contains an 8,713-triangle / 5,356-vertex neutral-pose FBX and one 4096×4096 color texture, with no pre-existing armature or animation. It now supplies Sabrina's shipped mesh and the new 32-bone rig described above. The external original ZIP is not bundled. Project-local extracted inputs and reviewed candidates have been removed; source/archive hashes remain in `sabrina.provenance.json`. The saved Blender file is the maintained authoring source.

The ocean crystal uses the textured GLB, reduced from 1,656,883 to 14,000 triangles with three 1K maps. The 8,434-triangle FBX has no textures and was not used. Editable source: `art/props/ocean_crystal.blend`; export: `game/assets/props/ocean_crystal.glb`. Rebuild using `tools/blender/prepare_crystal.py -- <source.glb>` in Blender. The shared relic, wake and protection-bubble shaders live in `game/shaders/effects/`; presentation is isolated in `crystal_visuals.cpp`. The supplied baked water ribbons are retained, with animated glints and a separate cast trail.
