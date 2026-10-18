# Reef environment asset brief

Visual target: the owner's wide underwater reef reference, with a rock arch and planted ledges on the left, ruined columns on the right, bright surface light and an open swimming area in the centre. Keep the original side-on aquarium gameplay.

The owner has now supplied `underwater ruins 3d model.glb`. The game uses an adapted version of this coherent diorama. For future additions, supply a single environment or separate modular pieces, not a reference-sheet collage. The imported Sabrina source demonstrates why this matters: its generator reconstructed every view and close-up as additional geometry.

## Current Blender workflow

The source is `art/environments/reef_ruins.blend`; the game reads `game/assets/environments/reef_ruins.glb`. The original download remains unchanged. Its hash, original 1,861,166-triangle count and preparation steps are in `art/environments/manifest.json`.

The current delivery has 150,000 triangles, one material and embedded 2K PBR textures. The rear blue wall with painted fish and the frozen turtle region were removed. Most scenery remains behind Z=-4. The near lower-left rocks and coral extend in front of the swimmers (up to Z=5.5) so creatures are naturally occluded there. `reef_depth.py` applies this depth-only placement while preserving the original XY silhouette and UVs. It is decorative scenery and has no gameplay collision. Distant texture mip bias and softer normal detail give the rear reef a subtle loss of focus without a full-screen depth-of-field pass. Moving light uses the shared external `reef_caustics.gdshader`, retaining the imported base color, normal and roughness maps.

To edit, save the Blender source and export it without rebuilding:

```sh
blender --background art/environments/reef_ruins.blend --python tools/blender/export_environment.py
python3 tools/validate_models.py
./tools/godot --headless --editor --path game --import --frame-delay 800
./tools/run.sh
```

Commit the source, GLB, manifest and intended texture/import changes together. `prepare_reef.py -- /path/to/original.glb` is the one-time, source-specific preparation and overwrites manual edits. The present source is a combined textured mesh; separating reusable modular rocks and plants remains a future art pass. Live fish, visitors, distant silhouettes and water effects are still separate runtime objects.

## Preferred delivery for future additions

- Editable `.blend` plus `.glb`, with applied transforms, real UVs and packed or included PBR textures.
- Separate named objects: sand floor, left rock arch, ledges, right ruins, distant rocks, coral clusters, kelp, shell and pearl. Keep plants separate so we can animate them and make instances.
- No characters, fish, baked water surface, sunbeams, particles, text, lighting overlays or cameras baked into the model. The game adds those and controls their quality independently.
- Include a neutral-light preview from the game-facing side and one angled view. Inspect the back and sides for holes, duplicate figures and flattened image reconstruction before export.
- Prefer tileable rock/sand materials and reusable coral pieces. Keep hero surfaces to 2K textures where possible; use a small set of shared materials. Baked normal maps should carry small detail instead of millions of triangles.

## Layout in this game

Godot coordinates: X runs across the screen, Y is up, and +Z points toward the camera. Blender uses Z-up; the glTF exporter converts axes. The camera looks along -Z. Use a side-on or very shallow three-quarter composition, not a forward-swimming corridor.

The active aquarium is approximately 32 world units across and 16 units high. Put the sandy floor near Y=0, most background architecture behind Z=-3, distant silhouettes around Z=-18, and foreground framing near the far left/right edges. The current orthographic camera's vertical span is 19.5 units at 16:9.

Leave a broad, clearly readable central area for Sabrina, the crystal and fish. Foreground occlusion is restricted to the lower-left corner; the central swimming area stays readable. The present game does not collide swimmers with environment meshes; an obstacle course would require a separate gameplay change.

## Initial performance targets

These are authoring targets to measure, not device guarantees: roughly 100–200k visible triangles for the static reef, 6–10 shared material families, reusable instanced vegetation/coral, and separate lower-detail distant geometry. We can build desktop and mobile quality variants from the same source. Avoid one enormous mesh/material atlas containing every plant and rock.

Water movement, caustics, depth haze and changing sunbeams belong to engine materials and lighting. The reference is a lighting/composition target; matching every rendered detail at mobile frame rates is not a realistic asset-only promise.

## Engine decision

Continue with Godot and the native C++ entity/component simulation. The current work needed is asset preparation, scene composition, animation and lighting. A Rust/Bevy port would also require rebuilding the Godot presentation, UI, audio, input and packaging integration. It would not remove the mesh, rigging or optimization work.

Bevy is a valid choice for a new Rust-first ECS project and supports skeletal animation, glTF, desktop, iOS and Android according to its [official feature list](https://bevy.org/). Godot already provides the current project integration and [desktop/mobile platform tools](https://godotengine.org/features/). Apple TV should be assessed separately; neither platform list alone establishes a ready shipping path for this game.

### Foreground depth correction

The near lower-left faces now occupy their own mesh, entirely at Godot Z 4.6–5.7. The remaining reef stays at Z ≤ −3.9. No face bridges the swimming band. XY projection and UVs remain unchanged, so the orthographic silhouette meets without stretching through characters. The asset validator checks every primitive’s entire depth range; `--capture-depth-tour` moves Sabrina across the boundary for visual review. Rebuild this split from the unlayered preparation mesh; applying a second deformation to an already layered mesh is rejected.
