# Architecture

The game has two crates. `reef_sim` owns rules and state without depending on an engine. `sabrina` maps that state to Bevy entities, input, audio, UI and rendering.

## Simulation and presentation

`reef_sim::World` contains plain components for Sabrina, 72 fish slots, three jellyfish slots, encounters, spells and scoring. Reused slots have generation IDs so a stale target cannot refer to a newly spawned fish. The simulation runs at 60 Hz and uses a seeded MT19937 generator.

Update order matters: timers, song, player movement, population and visits, crystal, wave, fish movement, contacts, jellyfish damage, then recovery and scoring. Keep dependent rules in this order. Independent presentation work can use Bevy's scheduler.

Rendering interpolates previous and current simulation snapshots. Input runs before fixed updates; presentation never feeds positions back into collision or scoring. Generations and spell transitions prevent interpolation across unrelated states. Pausing freezes the displayed snapshot, skeletal motion and effects.

## Rendering and animation

`Actor` identifies a persistent entity root; `ActorVisual` holds its model and orientation. Each animated model has its own skeleton but shares meshes, textures, animation graphs and clip maps. Fish swap shared normal/damage material handles. Screen-size LODs retain the same skin and materials, with hysteresis to avoid flickering between levels.

Embedded textures receive mipmaps once during loading. Color filtering happens in linear light; normal maps are renormalized. Trilinear sampling and anisotropy reduce distant shimmer. A depth prepass reduces hidden-surface shading.

Sabrina's authored clips blend over 0.22 seconds. Runtime neck, tail and hair corrections are restored before evaluating the next frame, so they cannot accumulate into unwanted spins. Hair currents travel from restrained roots toward freer tips, including while resting. Facial morphs handle blinking, smiling and singing. The crystal follows the blended right palm during a throw. Collision samples come from the same Blender poses, independently of rendered frame rate.

Jellyfish skin uses alpha blending; the separate face remains opaque. Jellyfish and small fish do not cast shadows. Balanced uses a 512-pixel filtered shadow map and 12% shadowed sunlight; High uses 1024 pixels and 22%, plus 2× MSAA. Unshadowed fill keeps the reef soft. Light disables shadows. Crystal and water effects do not cast shadows.

Water, reef caustics and crystal materials use external WESL files in `assets/shaders/`. Quality settings select 3, 6 or 12 water-light samples. Shader constants are calculated once per frame.

## Code map

| File in `src/sabrina/src/` | Responsibility |
|---|---|
| `app.rs` | Fixed update, input, actions and session transitions |
| `presentation.rs` | Interpolated render snapshot |
| `assets.rs` | Loading, shared resources, LODs, animation and attachments |
| `mipmaps.rs` | Texture mip generation during loading |
| `visuals.rs` | Camera, lighting, entities and pooled effects |
| `ui.rs` | Menus, HUD, focus navigation and translations |
| `audio.rs` | Recordings, synthesized music and captions |
| `settings.rs` | Saved preferences and first-run import of old Godot settings |
| `verification.rs` | Live input and pause checks |

Use simulation tests for rule changes, model validators for asset changes, and live input/capture checks for presentation. Benchmark release builds and record the actual GPU, resolution and quality setting. Frame-time percentiles are more useful than a single FPS reading.

## Maintenance notes

- UI layout is built by named screen helpers in `ui.rs`; `draw` handles rebuilding, while `actions` and `update` handle interaction and live HUD values. Pointer hit tests and the input verifier share the logical canvas dimensions.
- Keep simulation slot iteration, random draws, expression order and fixed-system order stable during readability refactors. Interpolated presentation must never become input to gameplay.
- `generated_contacts.rs` is generated data, not handwritten source. Edit Blender poses or `tools/export_rust_contacts.py` and regenerate it; the generator also owns its copyright header.
- Source files carry the project's copyright notice and MIT SPDX identifier. These notices do not change the separate artwork and audio terms in `ASSET_LICENSE.md`.
