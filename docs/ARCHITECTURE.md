# Architecture

The game is split into two Rust crates:

- `reef_sim` owns gameplay state, rules, collision and scoring. It has no engine dependency.
- `sabrina` handles Bevy integration: input, rendering, animation, audio, UI and settings.

## Simulation

The simulation runs at 60 Hz with a seeded MT19937 generator. `World` holds Sabrina, 72 fish slots, four jellyfish slots, spells and session state. Reused slots have generation IDs to prevent stale references.

Update order is explicit: timers, song, player movement, population, crystal, wave, fish movement, contacts, damage, then recovery and scoring. Keep this order and random draws stable when refactoring.

Input is buffered before fixed updates. Rendering interpolates simulation snapshots but never feeds positions back into gameplay. Pause freezes the displayed state and animation.

Combat constants and difficulty curves live in `balance.rs`. Competitive play ends after round 10; quiet aquarium mode is endless. Damage, rescue rewards and round completion are resolved in the simulation. Visible score feedback is stored separately from events consumed by audio.

The player's score accumulates across all rounds and does not reset between them. The HUD shows the banked total from completed rounds plus the active round's subtotal; victory shows the final aggregate across all ten rounds. Only the internal round subtotal resets when a new round begins, after its points have been added to the running total exactly once.

Rounds stay active until all departing jellyfish disappear. Only then is the survivor bonus awarded and the round subtotal banked into the running total, the peaceful interval started, and final-round victory awarded. Stings and fish losses during departure still count toward the active round; a wipeout or fatal sting takes priority over completion.

Unbubbled departing jellyfish remain threats for fish avoidance and rescue eligibility even at zero health, but cannot take further damage or become Crystal chain targets. Song and Wave rescue rewards require at least six units of clearance from every arriving, hungry or departing visitor, including bubbled visitors.

Jellyfish switch from arriving to normal hungry steering after 4.5 seconds whether or not they hear Sabrina's song. Bubbles still immobilize them; an overdue arrival transition happens when the bubble releases. Opposing visitors must not remain locked in inward arrival movement at the center.

## Rendering and animation

Actors use persistent entity roots and pooled visuals. Animated instances have separate skeletons but share meshes, textures and animation resources. Hidden fish and jellyfish stop animating; screen-size LODs use hysteresis.

Sabrina combines authored clips with runtime pose corrections, facial morphs and spring-driven hair. Collision samples are exported from the same Blender poses, keeping gameplay independent of render frame rate.

Textures receive mipmaps during loading. Water, reef and crystal effects use embedded WESL shaders. Quality settings control shadows, antialiasing and effect detail. Pooled effects avoid unnecessary transform and material updates.

Startup requests only the splash background, localized title and font before bulk loading. A dedicated 2D UI camera renders independently of the 3D scene. Render-world feedback confirms uploaded splash images and ready pipelines across two frames before the `LoadResources` set starts model/audio preloads, scene/effect setup and the concurrent update check. The loading screen stays on top during graphics warmup; readiness requires eight settled render frames but never waits for the update check.

Standalone desktop update notices use a worker thread and a persisted release cache. The first request is deferred by 1–3 days, then attempts are separated by 30 days plus 0–3 days of jitter. Saved deadlines survive restarts, including failed requests. Cached release versions only appear as passive loading-screen text and are compared against the installed version each launch. The `steam` feature and mobile targets disable this path.

Default graphics backends are DX12 on Windows, Vulkan on Linux and Metal on macOS. Command-line overrides are available for testing.

## Input, UI and audio

One active controller supplies gameplay input and button hints. Windows uses native XInput by default, with `--wgi` available for other devices; other platforms use Bevy/GilRs.

Gameplay actions remain buffered until a fixed tick consumes them. Menu navigation has separate repeat and dead-zone handling. Rumble follows confirmed gameplay events and stops on focus loss, menu entry or controller changes.



Music is rendered offline from MIDI through FluidSynth and shipped as Ogg audio. Runtime playback crossfades calm and combat loops, plays an arrival stinger and ducks music under voices and song.

## Assets and packaging

Assets are compiled into the executable and loaded through Bevy's in-memory asset reader. No extraction or source checkout is required. `SABRINA_ASSETS` selects an external asset directory for development.

Release packages contain the executable or app bundle and required license notices. Graphics, audio and windowing still depend on system libraries and drivers. The menu displays the build version, date and commit hash.

## Code map

Paths below are relative to `src/sabrina/src/`.

| Module | Responsibility |
|---|---|
| `app.rs` | Input, fixed updates and session transitions |
| `presentation.rs` | Interpolated simulation snapshots |
| `assets.rs`, `mipmaps.rs` | Models, animation, shared resources and texture setup |
| `visuals.rs`, `hair.rs` | Scene rendering, effects and secondary animation |
| `controls.rs`, `xinput.rs`, `rumble.rs` | Controllers, navigation and feedback |
| `ui.rs` | Menus, HUD and translations |
| `audio.rs` | Music, effects, voices and captions |
| `settings.rs` | Saved preferences and legacy settings import |
| `embedded_assets.rs` | In-memory asset loading |
| `verification.rs` | Live input and pause checks |

## Working on changes

- Test gameplay rules in `reef_sim`.
- Validate asset changes with the model tools.
- Regenerate `generated_contacts.rs` from Blender poses or the exporter; do not edit it manually.
- Check presentation with live input and captures.
- Profile release builds and record GPU, resolution, quality and frame-time percentiles.
- Keep source, asset and third-party license notices intact.