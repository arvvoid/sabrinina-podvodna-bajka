# Validation history

Entries below record checks at the time of each revision; later entries supersede earlier descriptions of removed features.

## Vulkan default and highlight correction — 2026-10-18

- The Mobile renderer now prefers Vulkan on Linux/Windows/Android and Metal on macOS/iOS. Built-in Direct3D 12, Vulkan and OpenGL fallbacks are enabled where Godot supports them. Mobile/TV packaging remains future work; Direct3D 12 and Metal were not locally tested.
- Actual Linux graphical launches selected Vulkan 1.4 on the NVIDIA RTX 3070 Laptop GPU. Forced OpenGL selected Mesa Intel UHD TGL GT1. Hiding Vulkan's ICD through a process-only `VK_DRIVER_FILES` override reproduced initialization failure and verified automatic OpenGL fallback, followed by a successful capture and normal exit. The override was not saved to the project or system.
- Compared fixed-pose High and Balanced views. OpenGL's High shadow pass clipped hair and skin highlights; Compatibility now omits that pass while retaining High antialiasing and water detail. Mobile retains High shadows. Reduced sunlight, fill and ambient energy, filmic highlight compression and a less reflective Sabrina material preserve pale surface detail. Background and silhouette colors now declare their authored color space for linear rendering.
- Fixed the shared sphere mesh initialization order: plankton previously received a null mesh before character construction. The Vulkan null-MultiMesh errors no longer occur.
- Release build, all three CTest suites, all 16 model contracts and 54 native runtime/input checks passed. The headless smoke run reached game over at step 10,375, with two visits and eight fish. No simulation rules changed in this revision.
- Inspected actual Vulkan and OpenGL High close-ups and a 20-second Vulkan capture with all 72 fish; each run reported the 60 FPS cap. These are short desktop observations, not sustained or mobile benchmarks. Some graphical exits still emit the previously documented four-instance ObjectDB teardown warning; headless verification exits cleanly.
- `--capture-graphics-high`, `--capture-graphics-balanced` and `--capture-graphics-low` reproduce the same paused close-up without modifying saved preferences. The temporary comparison images and logs are outside the repository.

## Population and swimming / crystal animation — 2026-10-18

- Fish births and empty-aquarium refills now wait while any jellyfish is arriving, hungry or leaving. Population tests exercise every visible state, a visitor in the final slot, both empty and populated water, and recovery after the final departure. The population timer is frozen during these states.
- Sabrina’s saved Blender source and GLB retain the same 38,126 triangles, 32 bones and 2K texture. Eight clips now include a 1.6-second Throw: gather to the right palm, draw in, extend and release at 1.1 seconds, then recover. Existing crystal recharge and hit rewards are unchanged. Wave and crystal gestures do not overlap.
- Reduced the upper-body arch, aimed swimming arms alongside the torso, and limited upward neck correction during dives. Arm keys preserve segment lengths and restrained wrist motion. Quaternion hemisphere continuity fixed actual intermediate forearm flips; the GLB validator now rejects abrupt joint rotations.
- Palm position and local attachment offset are exported with the same pose samples used for arm contacts. Simulation checks cover held and released crystal positions at nine combinations of dive/climb and facing angles. Native checks compare the exported palm against the actual rendered skeleton throughout the wind-up, and verify head direction at steep pitches.
- Release build, all three CTest suites, all 16 asset contracts and 54 native runtime/input checks passed. The 12,000-step smoke run completed two visits with eight fish and 3,010 points; fewer births during danger intentionally change the earlier population and score results.
- Inspected close-up dive, climb, grip and release poses and frames from a complete graphical throw toward a jellyfish. The impact briefly compresses the jellyfish before departure. Preview files remain temporary, outside the maintained asset tree. Graphical audio shutdown still produces the previously observed ObjectDB warnings; headless checks exit cleanly.

## Cleanup and remake credits — 2026-10-18

- Release build, all three CTest suites and all 53 native runtime checks passed. Credits checks cover all three languages, author names, the original Scratch URL, hidden narration controls and the help panel’s distinct controls.
- The 12,000-step runtime smoke test passed: two visits, 25 fish and 4,305 points.
- Model validation passed for all 16 final source/export pairs: Sabrina, twelve fish, jellyfish, reef and crystal. Inventory comparison found no extra Blender sources or GLB exports outside the manifests.
- Removed about 287 MiB of project-local obsolete inputs/candidates, backups, old preview videos, duplicate audio, an old engine executable and the bundled Scratch reference archive. External originals in Downloads and the separate Scratch project were not modified.
- Removed story text and all three intro narrations. The 12 audio files still referenced by the game retain provenance entries; unused jellyfish recordings were also removed. Gameplay audio and original help narration remain available.
- English, Croatian and Italian credits, the menu, Croatian help, gameplay and revised boot artwork were visually inspected at 1280 × 720. The boot and voiced help captures still report the existing audio-related ObjectDB warnings on exit; headless checks exit cleanly. The original-project button opens the supplied Scratch URL through the operating system; automated checks do not launch an external browser.
- README credits identify Luka Dezulovic as developer/remake author and Sabrina Dezulovic as the originator of the idea, link to Scratch and document the probable complete Rust/Bevy rewrite as future work.

## Aquarium revision — 2026-10-17

## Local checks

- Linux x86-64, Godot 4.7.2 stable, pinned godot-cpp 4.5 API bindings; release C++ build passed.
- Independent simulation checks passed for initial population, calm spawning, charging and travelling crystal, cooldown, fish persistence, crystal rescue, visit scoring, occasional multiple visitors, natural departure, sting protection, bounded peaceful aquarium, replenishment, deterministic seeds and swimming bounds.
- Native runtime smoke test passed for 12,000 physics steps (200 simulated seconds), completing three visits with 42 living fish and 4,395 total points in the seeded automated run.
- Simulation checks cover forward propulsion without sideways drift, Sabrina’s stroke and no idle drift; nearby versus out-of-range song response; shrinking radius; early stopping and restarting; exhaustion lockout and full recharge; long crystal cooldown; one-fish hand selection, direction away from danger, protection, cooldown, repeat-score prevention, stun release and no grabbing in calm water. Added arrival and simultaneous fish-contact consumption, red-pop expiry, health loss, recovery delay/rate, contact immunity, zero-health game over, frozen terminal state and reset. All pass.
- 30 native input/menu checks passed: synthetic keyboard, mouse and gamepad movement/casting, pause, home, language, song, audio slider, rest slider, settings navigation, original story loading, peaceful aquarium selection, staged loading completion, song lockout through the UI and the energy meter, UTF-8 Croatian labels and glyph coverage, left/right/up/down rendered fish heading, fatal contact, modal game over and retry.
- This revision’s Croatian menu/settings, redesigned HUD, red fish contact, stunned red Sabrina, zero-health retry screen and full school were inspected at 1280 × 720; updated boot art at 1920 × 1080. Shell-button focus contrast and compact HUD borders were corrected. The bundled font renders Croatian diacritics. No engine errors or leak warnings appeared.
- The initial full-population revision reported 43–44 fps on Mesa Intel UHD Graphics (TGL GT1), balanced settings. Baking static fish details/fins into shared meshes reduced each fish to four draw surfaces without removing geometry; the subsequent 20-second 72-fish capture and contact-effect capture both reported 60 fps. This is a short local observation, not a sustained frame-time benchmark or minimum-spec guarantee.
- This machine reports an unavailable Nvidia DRI driver before successfully selecting Intel OpenGL. Rendering works on Intel. VSync remains off with a 60 fps cap because the original local VSync path was slow.
- Story recordings decoded and played through the graphical story screen. All 19 original audio assets have source hashes and durations recorded in `game/assets/audio/PROVENANCE.json`; the original story text is retained in all three languages. Listening and balancing with the author remain useful.

## Entity/component refactor

- Gameplay now builds as the independent `reef_simulation` static library. Both the complete build and `SABRINA_BUILD_GAME=OFF` build pass the gameplay and entity-lifecycle CTest suites.
- Four seeded traces, each 18,000 steps, matched the pre-refactor implementation exactly on this Linux release toolchain. The trace includes simulation state and events, with aquarium, Gentle, Lively and fatal-contact scenarios. Hashes: `11729615818106077241`, `11718617323609571831`, `17996629465026964929`, `6271452615893997604`. Run `build/simulation_trace` for a same-toolchain comparison; these are not portable cross-platform golden values.
- Entity tests cover default/malformed/wrong-kind handles, retained pop records, generation changes on slot reuse, stale hand and crystal references, reset invalidation and independent world snapshots.
- All 15 extracted shader bodies match the previous embedded source after removing whitespace/comments. They are now formatted external resources loaded through a shared typed catalog. No `set_code` calls or embedded shader strings remain in the native sources.
- All 32 Godot input/menu/resource checks pass, including loading every shader and checking shared shader references with independent material instances. The smoke test still completes 12,000 steps with three visits, 42 fish and 4,395 points.
- Graphical full-population and hand-rescue captures were inspected at 1280 × 720. Both report 60 fps on the same Intel/Mesa renderer with balanced settings; no Godot shader/runtime errors appeared. This confirms a short local run without a visible regression, not a speedup claim or sustained benchmark.

## Blender character migration

- Blender 5.2.2 LTS produced eight editable sources and self-contained GLB exports. Sabrina: 32,476 triangles / 29 bones / eight clips; each of six fish: 3,006 triangles / six bones / two clips; jellyfish: 9,520 triangles / 28 bones / three clips.
- The asset validator passes for all eight files: named joints and clips, non-static animation, finite transforms, normalized weights, mesh attributes, embedded textures and triangle budgets. Exporting directly from the saved Sabrina `.blend` with `export_character.py` was tested and revalidated, independently of the geometry builder.
- Native release build and both CTest suites pass. All 38 native input/resource checks pass, now including imported skin bindings, required clips, live fish tail deformation, vertex-color material flags and paused animation playback. The 12,000-step smoke test retains three visits, 42 fish and 4,395 points. All four simulation trace hashes remain unchanged from the entity/component refactor.
- The real game uses imported skeletons for Sabrina, foreground fish and jellyfish. A recorded showcase exercises swimming, turns, singing and hand attachment. In-game design, full-school and boot captures and isolated Blender source renders were inspected. Character materials use the authored vertex colors; the initial white-material import issue was corrected in the native asset loader.
- A 20-second 72-fish capture at 1280 × 720, balanced graphics, reports 60 fps on Intel UHD Graphics (TGL GT1) / Mesa. The normal animation showcase also reports 60 fps. Movie-writer FPS is not a gameplay benchmark. These are short local checks, not sustained performance or mobile validation.
- Some rapid graphical capture exits report four leaked ObjectDB instances. A verbose reproduction identifies two synthesized `AudioStreamWAV` resources and their playback objects, despite the existing stop/clear shutdown path. Normal full-school, headless tests and a longer verbose animation capture finish without that warning. This intermittent audio teardown issue remains; no character mesh, skin or animation resources were reported leaked.
- Seven procedural character shaders were removed; eight shared environment/effect shaders remain. The core gameplay systems, original recordings and story were not changed by this migration.

## Supplied Sabrina and ruins adaptation

- The supplied Sabrina GLB contained 1,989,868 triangles, no rig/animation, and multiple reconstructed reference-sheet views. The complete front figure had a usable back and side profile. It was isolated, neighbouring fragments removed, and reduced to 57,999 triangles with one material surface. New UVs and three 2K maps retain the supplied detail with a rainbow tail and shell costume. A 30-bone rig supplies eight in-place clips; six tail joints and flexible fluke tips create the swimming wave. Arm/hip-fin separation uses surface distance rather than proximity alone.
- The supplied ruins had 1,861,166 triangles. Its game export has 150,000 triangles, one material and three 2K maps. The frozen turtle region and rear blue wall with painted fish were removed. It is placed behind the swimming plane and uses textured, moving caustics in the shared environment shader. The original downloaded files remain unchanged; source hashes and adaptation notes are recorded alongside the editable deliveries.
- Both saved Blender files were exported directly through the artist export commands and validated. The validator now covers character texture dimensions and the reef's source, embedded maps, triangle budget and stage bounds. Both CTest suites and all 39 native input/resource checks pass, including loading the authored reef. The smoke result remains three visits, 42 fish and 4,395 points; all four simulation trace hashes are unchanged.
- In-game scene and animation captures show the supplied character and ruins together. A 20-second full-population capture reports 60 fps at 1280 × 720 on the same Intel/Mesa renderer, balanced quality. This remains a short local observation, not a mobile or minimum-spec guarantee. The intermittent synthesized-audio teardown warning can still occur on graphical capture exit.
- The imported meshes are reduced triangulated reconstructions, not hand-retopologized animation topology or a modular environment kit. Sabrina's face retains its supplied expression. Further facial animation, skin-weight polish, modular scenery and sustained/device performance testing remain future work.

## Clean checkout and CI

The launcher retains the verified `--import --frame-delay 800` workaround for Godot's GDExtension cold-import documentation cleanup race. Normal gameplay has no frame delay.

The GitHub workflow builds the native extension and runs simulation tests on Linux and Windows. Linux also runs the real Godot smoke and input checks. Consult the workflow run associated with the revision for remote results.

## Remaining validation

- Physical gamepads and disconnect/reconnect behavior.
- Longer child/parent playtests of pacing, scoring, legibility and sound levels.
- Sustained performance at full fish population, high graphics quality and varied display resolutions.
- Native Windows/macOS execution, packaging and distribution.
- Android/iOS/touch and Apple TV (not implemented in this desktop prototype).

## Supplied fish and animation refinement — 2026-10-18

- Split the supplied twelve-fish collection into individual editable Blender sources and GLBs. Every fish has 7,800 triangles, six weighted bones, one material surface, embedded 512px color/normal maps, and looping Idle/Swim clips. Sabrina remains 57,999 triangles and 30 bones with selective face smoothing and softer shading.
- Asset validation passes for all fourteen characters plus the reef, including normalized weights, map budgets, required clips, closed rotation loops and a minimum readable tail stroke in both Idle and Swim. Native runtime checks pass **41 checks**, including every fish variant and Sabrina's full resting tail cycle.
- C++ build, both CTest suites and the 12,000-step native smoke test pass. The smoke result remains three visits, 42 fish and 4,395 points. All four deterministic simulation trace hashes remain unchanged from the entity refactor.
- Inspected Blender close-ups of Sabrina's face and an imported fish, then actual-game frames of resting, swimming and turning. The longer animation capture starts with four seconds of stationary treading before swimming, singing and hand rescue. Preview: `build/character-previews/living-reef.mp4` (local build artifact).
- The 20-second 72-fish capture reported 60 FPS at the Balanced cap on the local Intel UHD Graphics (TGL GT1), Godot Compatibility renderer. This is a desktop observation, not a mobile performance guarantee. Movie capture averaged about 1.6 ms CPU render and 7.8 ms GPU render per frame; those timings exclude other work and recording overhead.
- The existing graphical-exit synthesized-audio ObjectDB warning remains. Headless runtime checks finish cleanly. Sabrina's expression remains the supplied one; this pass does not add facial animation or manual quad retopology.


## Swimming-pose Sabrina, soft contacts and idle fix — 2026-10-18

- Replaced the earlier reference-sheet figure with the supplied complete swimming-pose GLB. The 1,912,079-triangle source is now a 58,000-triangle, one-surface model with three 2K maps, 30 bones and eight clips. The rainbow outfit, supporting idle tail stroke and delayed hair motion remain. Native pitch/yaw follows travel direction and settles into a curved resting tread. Original downloads are unchanged.
- Added native soft capsule contacts for fish pairs and Sabrina’s torso, head, arms, tail and flukes. Contacts exchange gentle normal impulses, apply water drag and steer fish apart. Held fish and crystal transit keep their authored paths. Three CTest suites pass, including coincident spawns, low-restitution rebounds, rotated body contacts, wall bounds, 30 Hz fast swimming, rescue exemptions, unchanged health/score on bumps and deterministic replay.
- All character/environment asset contracts and **43 native runtime checks** pass. A repeated-idle regression reproduced the reported upper-body spinning before the fix and passes afterwards: native pose corrections now restore the unmodified pose before each evaluation, including bones without animation tracks. The tests also check displayed-tail/contact-chain alignment and paused animation.
- The 12,000-step native smoke test passes with three visits, 42 fish and 4,285 points. Soft contacts intentionally change fish trajectories and therefore the previous trace/score values are no longer golden results.
- Inspected fresh in-game frames from idle, ascending/descending swimming, turning, song and gentle-contact recordings. Current local previews are `build/character-previews/swimming-and-contacts.mp4` and `gentle-bumps.mp4`. Obsolete screenshots, duplicate movies, import caches and Blender backups were removed; current editable sources and original downloaded models remain. Boot and README artwork were refreshed.
- A 20-second 72-fish capture at 1280 × 720, Balanced quality, reports 60 FPS on Intel UHD Graphics (TGL GT1), Godot Compatibility/Mesa. The final animation recording reports approximately 1.57 ms CPU render and 7.75 ms GPU render per frame; recording timings exclude other work and are not a gameplay benchmark. Sustained/mobile performance is still untested.
- Some graphical capture exits still report 4–8 ObjectDB instances; headless runtime checks finish cleanly. This pre-existing teardown class of warning remains outside the movement fix.
- Inspected the optional Smart Mesh FBX: 5,525 vertices, 3,274 quads, 2,208 triangles (8,756 triangulated faces), one mesh/material, no textures, weights, skeleton or animation. It remains an unshipped candidate for texture/detail transfer and a lower-cost rigged model.


## Directional wave, longer visitors and foreground depth — 2026-10-18

- Replaced automatic fish grabbing with an explicit water wave: right mouse button, F, controller B or HUD button; 3 / 4-second recharge, a 0.75-second travelling front reaching 5.5 units, and one push per fish per cast. Fish outside the front area and crystal transit are excluded. Jellyfish are unaffected. A safe push away from a visitor within six units earns 20 points once per fish per visit.
- Read the original Scratch Jellyfish `Lov` blocks directly. They use the song flag and a 1–100 random roll against `15 + 5 * cycle + 1`, without a distance test. This revision rolls every 2.5 seconds while singing, caps chance at 50%, and holds successful interest for 3.5 seconds or until singing stops. Fish still require the shrinking visible radius.
- Visitor hunger now lasts 90 / 120 seconds; feeding no longer shortens it. Fullness requires 10 / 14 meals and at least 60 / 80 seconds. The overall ceiling is 100 / 130 seconds; the crystal remains an early exit. Every meal increases a shared visual/contact size with diminishing growth. Bell, crest and tentacle envelopes consume all simultaneous contacts. Popping fish stop at the surface; Sabrina is separated even while immune; overlapping visitors separate.
- Refined forearms, wrist flexion and head motion. Sabrina has seven authored clips, replacing the two unused grab clips with a two-hand Wave clip. The repeated-idle regression remains passing.
- Lower-left near reef geometry now extends ahead of swimmers, preserving its XY silhouette and texture coordinates. The 150,000-triangle budget and one material are unchanged. The asset validator requires foreground vertices in the lower-left corner and keeps the main play area clear. Distant texture mip bias, softer normals and haze provide a subtle depth cue; this is not a full-screen depth-of-field blur.
- All three CTest suites and asset validation pass. Tests cover wave area/direction/cooldown, multiple fish, no jellyfish impulse, stun lockout, repeat-score prevention, longer minimum visits, probabilistic distant song attraction, bounded roll frequency, all-surface simultaneous eating, growth with every meal, invulnerable physical contact and visitor separation. **47 native runtime checks pass**, including all four wave control paths.
- The 12,000-step smoke run completes two longer visits with 16 fish and 3,195 points. Gameplay traces intentionally change with these mechanics.
- Inspected wave casting, relaxed idle/swim poses and actual foreground occlusion in game. Current local recordings: `build/character-previews/wave-and-visitors.mp4` and `swimming-and-contacts.mp4`. Updated README and boot screenshots. A 20-second 72-fish capture still reports 60 FPS at 1280 × 720, Balanced, on Intel UHD Graphics TGL GT1 / Mesa Compatibility. This is a short desktop check, not mobile validation. The existing graphical-exit ObjectDB warning remains.

## Crystal, foreground and balance pass — 18 October 2026

- Imported the supplied textured crystal as an editable Blender source and 14,000-triangle, single-surface GLB with three embedded 1K maps. Asset validation now covers props as well as characters and scenery. The untextured FBX and the new unrigged T-pose mermaid were inspected, not substituted for the working character.
- The crystal has a close orbit and matching trail, smooth collection to the hands, a faceted material, cast wake, impact burst and visible rescue bubbles. The trail uses fixed-time samples and initializes at the launch position. Visual phases stop when paused; reduced motion slows orbit and shimmer.
- Split foreground faces into a separate depth band instead of stretching a continuous mesh through actor depth. Validation checks every reef primitive is entirely behind or entirely in front of the swimming band. Inspected the lower-left static capture and a 12-second crossing recording at several points.
- Three native suites pass, with new regressions for orbital continuity, sustained wave displacement, song escort scoring/no repeat farming, protected and expired fish contacts, and crystal impact against a grown jellyfish surface with at most one visitor repelled.
- 48 native input/runtime integration checks pass, including the new imported crystal/material check. The 12,000-tick smoke run completes with two visits, 15 fish and 3,730 points. Graphical captures show no shader errors. The 72-fish capture still reports 60 FPS at 1280×720 on the local Intel UHD (TGL GT1), Compatibility/Mesa. Recording is not a mobile benchmark. Some graphical exits retain the previously noted audio-related ObjectDB warnings; headless verification exits cleanly.
- [Balance report](BALANCE.md) records the seeded five-minute runs, controlled song/wave comparisons, scoring rationale and limitations of bot playtesting. In-game help was updated in English, Croatian and Italian.


## Neutral-pose Sabrina rig — 18 October 2026

- Replaced the swimming-pose reconstruction with the owner's textured neutral-pose FBX. The editable source now has 38,126 triangles, 32 bones, one material with an embedded 2K rainbow color map, and seven clips. Collarbone joints support the shoulders; restrained elbow/wrist poses replace the compounded arm oscillations. The existing tail stroke timing and hip-relative tail/contact landmarks are retained.
- Forward wave casting aims the arm segments without stretching bones. Native neck/head correction uses posed axes and limits gaze at every speed. Hair roots counter the gaze correction; the original anti-accumulation safeguard remains in place.
- The Blender exporter also emits compact arm-contact samples from the authored clips. Native contacts and presentation share pose selection/phase, and arm depth projects through turns. The collision solver remains independent of Godot and no rigid-body chain is added to character animation.
- All character/prop/environment asset checks, three CTest suites and **50 native input/runtime checks** pass. New checks cover collarbone hierarchy, bounded gaze at rest/ascent/descent, and arm/contact alignment across all seven clips. Existing repeated-idle, paused playback and tail-contact checks still pass. The 12,000-tick smoke run completes two visits with 25 fish and 4,305 points; contact trajectory changes make this an observed result, not a fixed scoring target.
- Visually inspected Blender poses and the actual in-game close-up sequence: rest, swimming both directions, ascent, descent, song and casting. `build/character-previews/sabrina-rig.mp4` is the current local preview; `--capture-rig` reproduces the sequence. The model still has some coarse reconstructed hair/fin edges, with no facial/blink or individual-finger animation. Mobile performance has not been measured. The previously documented graphical-exit ObjectDB warnings remain; headless checks exit cleanly.


## Upper-tail deformation repair — 18 October 2026

- The skirt-like waist artifact came from incorrect region seeds: hip-fin and upper-tail vertices inherited hair weights and were stretched toward the head. Added front/back/side hip and fin seeds and removed two hair seeds that landed on the hip fins. The side fins now follow the tail; the source mesh and animation timing are unchanged.
- Reproduced the error on the original 99-vertex upper-tail core: maximum hair influence was 0.999002 before the correction, below 1e-18 afterwards. Preparation now rejects hair or arm influence above 0.001 in that source-space core.
- Re-exported the Blender source, GLB and rainbow texture. All asset contracts, three native suites and 50 runtime/input checks pass. Visually inspected the corrected Blender swim pose and in-game swimming, reverse view and ascent; the updated local preview is `build/character-previews/sabrina-waist-fixed.mp4`.
