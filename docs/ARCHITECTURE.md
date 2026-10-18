# Architecture

The game has two crates. `reef_sim` owns rules and state without depending on an engine. `sabrina` maps that state to Bevy entities, input, audio, UI and rendering.

## Simulation and presentation

`reef_sim::World` contains plain components for Sabrina, 72 fish slots, three jellyfish slots, encounters, spells and scoring. Reused slots have generation IDs so a stale target cannot refer to a newly spawned fish. The simulation runs at 60 Hz and uses a seeded MT19937 generator.

Update order matters: timers, song, player movement, population and visits, crystal, wave, fish movement, contacts, jellyfish damage, then recovery and scoring. Keep dependent rules in this order. Independent presentation work can use Bevy's scheduler.

Rendering interpolates previous and current simulation snapshots. Input runs before fixed updates; presentation never feeds positions back into collision or scoring. Generations and spell transitions prevent interpolation across unrelated states. Pausing freezes the displayed snapshot, skeletal motion and effects.

## Combat and scoring

`reef_sim/src/balance.rs` owns combat constants, bounded difficulty curves and score awards. Jellyfish start with 100 health; a crystal removes 40, a close wave 10. Only health actually removed scores, so repeated attacks cannot farm a defeated visitor. Zero health initiates a harmless retreat; elapsed time and eaten fish never do. Crystal hits apply a three-second harmless bubble, followed by return flight; misses also return. Gentle stores a generation-safe homing target, while Lively stores a fixed direction at cast time. Recharge starts at launch.

Waves affect fish across their original cone, but visitors only within 2.5 world units, once per cast ID. Knockback decays independently of visitor steering and works during bubbles. Bubbles prevent movement, feeding and stings; visual drift does not feed back into gameplay. Health bars and bubble rings use pooled water-material quads.

Rescue awards share a fish's `rescued_cycle` across all abilities. Crystal rewards only collection within four units of an active, unbubbled threat. Wave rescue candidates must survive one second and reach six-unit clearance; song requires at least one second of escort from four-unit danger to six-unit clearance. Per-fish rescue flags prevent repeated ability switching from farming points. Recovery/scoring checks a fish wipeout before awarding round completion, even when the last visitor was defeated in the same tick. Births occur only in calm water without visitors.

Score feedback lives in the simulation snapshot, not the audio event queue; audio can drain events without losing visible rewards. Each award kind has its own brief feedback line, so a final hit, retreat and survivor bonus remain visible together. Consecutive awards of the same kind briefly accumulate. Round results show the earned total, not an extra award of that amount. Settings carry a scoring version to invalidate incompatible old records without discarding other preferences. Local bests are not leaderboard-comparable across custom settings.

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
| `audio.rs` | MIDI-rendered music, arrival stinger, recordings and captions |
| `settings.rs` | Saved preferences and first-run import of old Godot settings |
| `verification.rs` | Live input and pause checks |

Use simulation tests for rule changes, model validators for asset changes, and live input/capture checks for presentation. Benchmark release builds and record the actual GPU, resolution and quality setting. Frame-time percentiles are more useful than a single FPS reading.

Music sources live in `art/music/`; `tools/render_music.py` renders them through
FluidSynth into the three Ogg assets. Exported loops cover complete bars and
retain the previous pass's release/reverb tail, with a 30 ms phase blend at the
boundary that preserves bar length. Runtime playback does not
synthesize MIDI. Calm and visitor loops crossfade; each visit restarts the combat
track and triggers a dedicated one-shot arrival channel. Voices and song duck
music, and the music slider controls both loops and the stinger. Magic effects
cannot replace the stinger. The audio asset directory, including the FluidR3
notice, is already included by both packaging paths.

`src/sabrina/build.rs` embeds the UTC build date and 12-character commit hash.
Local builds identify as `DEV`; tagged GitHub Actions builds display their
release tag. The menu shows all three fields so test builds and packages are
easy to identify. Git HEAD and the active ref are build-script inputs, keeping
the hash current after commits without a clean rebuild, including packed refs
and worktrees. Source and dependency-manifest changes refresh the build date;
an unchanged cached binary keeps its original build date. The hash comes from
the checked-out commit (with GitHub's SHA as a fallback when Git is unavailable).
Source archives without Git metadata display `unknown` with a build warning,
never a fabricated hash. Build-script rule/date tests can be run with
`rustc --edition=2024 --test src/sabrina/build.rs -o target/sabrina-build-tests`
followed by `target/sabrina-build-tests`.

## Maintenance notes

- UI layout is built by named screen helpers in `ui.rs`; `draw` handles rebuilding, while `actions` and `update` handle interaction and live HUD values. Pointer hit tests and the input verifier share the logical canvas dimensions.
- Keep simulation slot iteration, random draws, expression order and fixed-system order stable during readability refactors. Interpolated presentation must never become input to gameplay.
- `generated_contacts.rs` is generated data, not handwritten source. Edit Blender poses or `tools/export_rust_contacts.py` and regenerate it; the generator also owns its copyright header.
- Source files carry the project's copyright notice and MIT SPDX identifier. These notices do not change the separate artwork and audio terms in `ASSET_LICENSE.md`.
