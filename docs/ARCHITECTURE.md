# Native entity/component architecture

The gameplay core is a plain C++17 world, composed of data components and updated by free functions. Godot owns rendering, audio, input and interface widgets. There are no gameplay entity classes, inheritance trees, virtual updates, per-entity nodes running logic, or gameplay scripts.

## Source map

| Location | Responsibility |
|---|---|
| `src/simulation/components.h` | Value types, entity identities, components, fixed entity compositions, input and events |
| `src/simulation/world.*` | Owned component storage, spawning, reset and handle lookup |
| `src/simulation/simulation.*` | Public simulation entry points and ordered update pipeline |
| `src/simulation/systems/player.cpp` | Timers, song energy and player swimming |
| `src/simulation/systems/fish.cpp` | Fish steering, schooling, propulsion and teleport motion |
| `src/simulation/contacts.h` / `systems/contacts.cpp` | Soft capsule contacts, animated Sabrina envelopes and water-damped fish impulses |
| `src/simulation/systems/jellyfish.cpp` | Jellyfish movement, contacts, damage and consumption |
| `src/simulation/systems/magic.cpp` | Crystal targeting, travel, rescue and repulsion |
| `src/presentation/crystal_visuals.cpp` | Imported relic, orbit, fixed-time cast trail and effects |
| `src/simulation/systems/wave.cpp` | Directional water wave, per-cast hits and safe-push scoring |
| `src/simulation/systems/encounters.cpp` | Population pacing, visits, recovery and scoring |
| `src/presentation/reef_game.*` | Godot lifecycle, input translation, settings and event dispatch |
| `src/presentation/reef_world.cpp` | Environment composition and synchronization of fish/effects |
| `src/presentation/reef_assets.*` | Imported Blender scenery and textured caustic materials |
| `src/presentation/reef_characters.cpp` | Authored rig instantiation and animation selection |
| `src/presentation/reef_ui.cpp` / `reef_audio.cpp` | Localized UI and sound |
| `src/presentation/character_assets.*` | Shared GLB library, rig components, manual animation updates and bone attachments |
| `src/presentation/shader_library.*` | Typed shader catalog and cached resource loading |
| `game/shaders/{environment,effects}/` | Editable Godot shader source |

`reef_simulation` is a separate CMake library. It has no Godot includes or dependencies and can be built with `SABRINA_BUILD_GAME=OFF`.

## Components and storage

`World` owns a 72-slot fish pool, a three-slot jellyfish pool, Sabrina, the crystal, session/scoring resources and a seeded random generator. A fish combines `Motion`, `FishAppearance`, `FishSwimming`, `FishRescue`, `WaterContact` and `FishLife`; the other entity types compose the components they need. Systems operate directly on those records.

This is a small fixed-archetype entity/component design. Component composition is determined at compile time. Pools are contiguous arrays of entity records (AoS), not separate arrays for every component and not a general dynamic ECS registry. This keeps bounded storage, direct iteration, stable presentation slots and no heap allocations during simulation updates. Adding an optional component means extending the relevant composition and systems; runtime component attachment and arbitrary queries are not implemented.

A general registry or a full structure-of-arrays layout can be introduced if profiling or a much larger population justifies it. Neither an ECS label nor removing inheritance guarantees faster execution. At the current scale, rendering and fish-neighbour work also matter: the latter remains an order-dependent O(n²) scan of at most 72 fish. No threading or spatial index was added during this behavior-preserving refactor.

## Entity lifetime

An `Entity` contains its kind, pool slot and a 64-bit generation. Cross-entity references, including the crystal target, store handles rather than raw pointers or bare slot numbers. `find_fish` and `find_jelly` validate kind, bounds and generation before returning a temporary pointer.

Spawning reinitializes all components and assigns a new generation. Eaten fish retain their record during the pop effect; the slot cannot be reused until the effect ends. Lookup establishes identity, not gameplay eligibility: systems also check active/state flags. A dormant record may still resolve until it is reused. Starting another visit replaces jellyfish identities. Reset preserves the generation counter so old handles cannot resolve in a new game.

Handles belong to their world. World copies are independent value snapshots and preserve corresponding identities; resolve a handle against the intended snapshot. Do not retain resolved pointers across reset or pool reuse. Render nodes are pooled by slot and read the current occupant each frame; they do not own simulation entities.

Initialize a world with `reef::reset(world, seed, settings)`. Spawn through `add_fish` and `begin_visit` so IDs and all component defaults are established together. Test/capture setup may deliberately override individual component fields after initialization.

## Update order

`reef::step(world, dt, input)` returns events and performs these systems in order:

1. Timers and expiry.
2. Song toggle, consumption and recharge.
3. Sabrina's swimming.
4. Population and encounter pacing.
5. Crystal magic.
6. Water wave.
7. Fish swimming.
8. Soft fish/fish and fish/Sabrina contacts.
9. Jellyfish movement and harmful contacts.
10. Health recovery and end-of-visit scoring.

The step ignores non-positive time and terminal games, and caps a step at 1/30 second as before. The host normally advances it at 60 Hz. It does not accumulate a large delta internally.

Order is intentional: contacts must precede healing, and magic/wave casting precedes fish movement. Jellyfish movement and contacts remain interleaved per jellyfish; fish neighbours are read in the original stable slot order. Reordering these loops, parallelizing them or changing random-call order can change gameplay even if individual formulas are identical.

Soft contacts use six bounded sequential passes over the fixed fish pool. Fish have oriented capsules and mass proportional to their cross-section. Sabrina is kinematic, with sixteen soft capsules covering her torso, head, arms, six tail segments and divided flukes. Her simulation phase and orientation also drive the rendered rig; tail landmarks and wave coefficients must be kept in sync with the Blender authoring script. The arm envelopes approximate sculling rather than matching every skinned vertex. Contacts remove overlap, exchange low-restitution normal impulses, and bias fish steering away; water drag dissipates the impulse. Decorative Z offsets do not opt fish out of the common gameplay plane. Crystal transit bypasses these contacts. Jellyfish envelopes grow with each meal; they block Sabrina even during sting immunity and separate from one another. Unprotected fish stop on the dangerous surface and enter the red-pop state. Crystal transit is exempt; the remainder of the four-second bubble resolves contact outward without damage. This is bounded 2.5D gameplay physics, not a general mesh rigid-body engine.

Systems return aggregated events for audio, notices and game-over UI. They never access scene nodes. The Godot `ReefGame : Node3D` class remains the required engine integration boundary; its presentation helpers and signal callbacks are not gameplay entity behavior.

## Character presentation

`RigVisual` is a presentation component holding the imported root, skeleton, animation player and mesh references. The shared character library validates imported scenes before actors are constructed. Native free functions instantiate rigs and advance animation manually, so pause stops animation too. Animation transitions blend over 0.22 seconds. Sabrina’s loop position is synchronized to the simulation stroke clock, with native resting-tail and gaze corrections applied after clip evaluation. The unmodified bone poses are restored before the next evaluation, including bones whose constant tracks Godot optimizes away, so additive corrections cannot accumulate into spins.

Simulation still owns positions, steering, contacts, resources and entity lifetimes. Presentation chooses clips from that state; animations are in place and never drive gameplay movement. Fish visual slots track entity generation to reset animation phase on reuse. A species change replaces only that slot's rig. Each of the six simulation appearance groups has two supplied fish designs; stable slot parity chooses the variant in presentation, without changing random calls, scoring or simulation components. Wave casts select an authored two-hand gesture and render a travelling crescent from the same origin, direction and clock used by the native spell. The shader is a shared external resource; it never decides which fish are affected.

Editable sources, exports and contracts are described in [CHARACTER_PIPELINE.md](CHARACTER_PIPELINE.md).

## Shaders

All eight environment/effect shaders live in `.gdshader` files. No shader source strings or `Shader::set_code` calls remain in C++. `ReefShader` identifies each file in the shared catalog. Godot's `ResourceLoader` caches shader resources; materials retain shared shader references and own their uniform values. Character skinning and PBR materials come from the Blender exports. Fish instances share their species mesh, skin, animation and material resources; skeleton poses and animation playback are per instance. The former character deformation shaders have been removed to avoid applying a second deformation over skeletal animation.

For a new shader, create the file in the matching directory, add its enum/catalog entry, load it through `load_shader` or `load_shader_material`, then bind uniforms in the presentation code. The catalog size is checked at compile time, and the native integration checks load every catalog entry and verify shader sharing between distinct materials. Actual GPU compilation is also checked with graphical captures; headless resource loading alone is insufficient.

Shader edits do not require recompiling C++. Re-run the game to load edited files. A new catalog entry or a changed C++ uniform binding does require rebuilding. Keep shader files in exported resources; the current project still targets desktop development rather than store packages.

## Verification

`ctest` runs gameplay regression tests and entity lifetime tests. `simulation_trace` hashes semantic state and events over four seeded 18,000-step scenarios: aquarium, gentle play, lively play and fatal contacts. It excludes identity generations so reset bookkeeping does not alter the gameplay digest. Use its output to compare revisions built with the same platform/toolchain; floating-point math and standard-library random distributions are not promised to be bit-identical across platforms.

The initial migration matched the pre-refactor implementation's hashes in all four scenarios. See `VALIDATION.md` for the local measurements and runtime checks.
