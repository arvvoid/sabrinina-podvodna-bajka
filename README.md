# Sabrinina podvodna bajka

*Sabrina’s Underwater Fairy Tale*

[![Test](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/rust.yml/badge.svg)](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/rust.yml)
[![CodeQL](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/github-code-scanning/codeql)
[![Rust 1.99.1](https://img.shields.io/badge/Rust-1.99.1-orange?logo=rust)](https://www.rust-lang.org/)
[![Bevy 0.20.0](https://img.shields.io/badge/Bevy-0.20.0-blue)](https://bevy.org/)
[![Code: Apache 2.0](https://img.shields.io/badge/Code-Apache%202.0-green.svg)](LICENSE)
[![Build targets](https://img.shields.io/badge/Build%20targets-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey)](#build-and-run)

A small underwater game about a mermaid, tropical fish and a hungry jellyfish. Swim through the reef, keep the fish safe with song and magic, or switch to quiet aquarium mode and enjoy the sea.

![Sabrina swimming among tropical fish in the underwater reef](docs/screenshots/gameplay.png)

*The current Rust and Bevy remake — an early working version, not the finished game.*

### See it in motion

[![Animated gameplay: Sabrina uses crystal magic, a close wave and song to protect the fish](docs/media/gameplay-preview.gif)](docs/media/gameplay.mp4)

[Watch or download the 18-second gameplay video (MP4, 720p)](docs/media/gameplay.mp4). The eight-second preview above loops silently; the full clip uses the game's combat music as its soundtrack. Recorded from the current game with scripted inputs, not a pre-rendered animation.

## How it started

I made the [original Scratch game](https://scratch.mit.edu/projects/1203591608/) together with my daughter Sabrina, who was four at the time. Sabrina came up with the idea and design; I helped as her dad with teaching, technical help and explanations. Together, we built a little underwater world with swimming fish, a hungry jellyfish and a mermaid who protects the fish with magic.

That Scratch project became a 3D Godot prototype, then this Rust and Bevy remake: a full standalone game growing out of our father-and-daughter project. The side-on view, voices, crystal magic and peaceful pauses come from that original game.

**Development:** Luka Dežulović

**Original idea:** Sabrina Dežulović

The Godot prototype is preserved on [`legacy_godot`](https://github.com/arvvoid/sabrinina-podvodna-bajka/tree/legacy_godot). `main` contains the Bevy game.

## Current status and release plans

This is an initial working version. It is playable, but still needs more work, refinement and playtesting before it is ready for release. Once it is ready, I plan to publish it on **Steam, Android and iOS**. These are planned releases, not currently available versions; there is no release date yet.

## Playing

Fish arrive during peaceful intervals. Each visiting jellyfish has **100 health** and stays until its health reaches zero, then retreats. There is no timeout or eaten-fish limit. **Losing all fish during a round, or all Sabrina's health, ends the game.** Jellyfish still grow as they eat; Sabrina recovers health after a safe period.

**Protect the reef through 10 rounds to win.** Round 10 is always the last and always spawns **four jellyfish**, in both Gentle and Lively modes, regardless of the normal visitor limit. Defeat all four with some health and at least one fish remaining to reach the victory screen, showing total score and surviving fish. There is no round 11. Quiet aquarium remains endless.

- **Crystal:** 40 damage and a 3-second air bubble. A bubbled jellyfish cannot swim, sting or eat, but waves can still push it. The crystal returns like a boomerang after a hit or miss; it recharges in 12 seconds from launch. Once returned, it immediately resumes orbiting, without glow or particles until ready to throw again. Gentle mode auto-aims; Lively uses a straight skillshot aimed by swimming before casting (horizontal while stationary).
- **Wave:** pushes fish away as before. Within 2.5 world units of the cast position it also pushes a jellyfish and deals 10 damage, once per visitor per cast. This checks the bell and tentacles, not just the centre, including at point-blank range. Recharge: 3 seconds.
- **Song:** gathers nearby fish, but can also attract jellyfish. Its reach shrinks with energy; exhausting it requires a complete recharge.
- **Progression:** each round slightly increases visitor speed and the probability of multiple visitors, up to the selected visitor limit. Speed growth is bounded; health and damage stay consistent.

Scoring rewards actions, not waiting: **1 point per actual damage**, **100 per defeated visitor**, **50 per genuine fish rescue**, and **25 per surviving fish** after every visitor is defeated. A fish can earn only one rescue reward per round across all abilities. Crystal collection only scores for threatened fish; wave rescues require surviving and reaching clear water; song requires escorting a threatened fish to safety. Safe fish collection, missed attacks and passive waiting earn nothing. A sting still subtracts up to 100 points. Hit, rescue and survivor rewards appear immediately, and each visitor has a health bar.

Settings offer Gentle and Lively difficulty, encounter pacing and a peaceful, noncompetitive aquarium mode. The visitor-limit setting applies to rounds 1–9 only. Best scores from the old scoring rules are reset once; the saved best remains a local single-round record, not a normalized leaderboard across custom settings. The obsolete spoken instructions have been removed; the localized written guide describes the current rules. Character voices and song remain.

| Action | Mouse / keyboard | Xbox | PlayStation |
|---|---|---|---|
| Swim | Pointer, WASD / arrows | Left stick | Left stick |
| Crystal | Left click, Space / E | A (green), or right shoulder | × (blue), or right shoulder |
| Song | Q or HUD button | X (blue) | □ (pink) |
| Wave | Right click, F or HUD button | B (red) | ○ (red) |
| Pause | Escape | Start / Menu | Options |
| Fullscreen | F11 or Settings | Settings | Settings |

HUD and help hints follow the connected controller, including button colours. With multiple controllers, hints follow the one most recently used; disconnecting all controllers restores keyboard/mouse hints. Unrecognized controllers use neutral bottom/left/right button labels. Detection uses the device name and USB vendor ID reported by the OS; remappers such as Steam Input may expose a different controller type. Keyboard/mouse input remains available while a controller is connected.

HUD backgrounds and progress fills are semi-transparent. The lower-left foreground reef fades only slightly around Sabrina when it covers her (at most 25% transparent), leaving the rest of the scenery solid.

Enable **Show FPS counter** in Settings to display a small, smoothed frames-per-second counter in the upper-right corner, below the HUD buttons. It is off by default, works in menus and during play, and is saved with your preferences.

English, Croatian and Italian are available through the main menu's **Language… / Jezik… / Lingua…** button. Select a language by name in the chooser; a checkmark identifies the selected language. Graphics, sound, controls and best score are saved locally. The loading screen uses localized text and asset-loading progress, then waits for character instances before opening the menu. The first launch uses fullscreen and Balanced graphics. Balanced has subtle soft shadows; High adds finer shadows and 2× MSAA; Light reduces effects and disables shadows. **V-sync** is on by default. Vulkan uses relaxed FIFO where the driver supports it, so an occasional late frame does not halve the frame rate; Direct3D 12 has no relaxed mode, so a late frame waits for the next refresh. Turn V-sync off in Settings to measure uncapped performance, reduce input latency or avoid that wait on slower Windows PCs (may tear).

## Build and run

Install [Rust](https://rustup.rs/). The toolchain and dependencies are pinned in `rust-toolchain.toml` and `Cargo.lock` (Rust 1.99.1, Bevy 0.20.0).

On Debian/Ubuntu, install the native dependencies:

```sh
sudo apt install build-essential pkg-config libasound2-dev libudev-dev libx11-dev libxrandr-dev libxi-dev libgl1-mesa-dev
./tools/build.sh
./tools/run.sh
```

The scripts use an optimized release build. Set `SABRINA_BUILD_JOBS=4` to limit compilation parallelism. On Windows, install Visual Studio C++ Build Tools; on macOS, install Xcode Command Line Tools. Build and run with:

```sh
cargo run --locked --release -p sabrina
```

### Graphics backend and testing matrix

| OS / target | Graphics backend | Build / distribution status | Runtime testing |
|---|---|---|---|
| Linux x86_64 | Vulkan | Desktop release workflow target; Ubuntu 24.04 baseline | Established tested desktop platform; gameplay and visual checks |
| Windows x86_64 (MSVC) | Direct3D 12 with embedded DXC | Desktop release workflow target; local release build verified | Tested on Windows 11 with an RTX 3070 Laptop GPU: rendered captures, live input/animation/effects checks and a 1080p 72-fish stress scene |
| macOS Apple Silicon | Metal | Desktop release workflow target; unsigned and not notarized | Not runtime-tested yet |
| Android (planned) | Vulkan | No mobile export available yet | Not tested |
| iOS (planned) | Metal | No mobile export available yet | Not tested |

The graphics backend defaults to Direct3D 12 on Windows, Vulkan on Linux and Metal on macOS. Backend flags allow testing supported alternatives: Windows accepts `--dx12` and `--vulkan`; macOS accepts `--metal` and `--vulkan`; Linux accepts `--vulkan` only. A backend flag not supported on the current platform is ignored with a warning and the platform default is used. An explicitly selected backend still requires a compatible graphics driver and device. There is no OpenGL fallback, and `WGPU_BACKEND` does not override this selection.

Windows x86_64 MSVC builds embed the modern DXC shader compiler in the executable. No separate `dxcompiler.dll` or installed Windows SDK is needed to play. This avoids the legacy FXC rendering path, which produced dark silhouettes on the tested NVIDIA Windows machine. To build on Windows, install Rust and Visual Studio's **Desktop development with C++** workload, including a Windows SDK and the **C++ ATL for latest build tools** component, then run the Cargo command above in PowerShell.

Windows verification also passed all 51 automated tests and strict Clippy checks. The High-quality 1080p 72-fish stress scene averaged 60.9 FPS, with no measured frames exceeding 25 ms on that machine. These results are hardware-specific, not a performance guarantee. Broader Windows hardware testing, macOS runtime, physical controllers and mobile exports still need testing; synthetic controller checks do not replace physical-controller testing.

All runtime assets are embedded at compile time: the executable runs without an `assets/` directory or the source tree, and does not extract assets to disk. This applies to local and tagged release builds. Rebuild after changing assets, or set `SABRINA_ASSETS=/absolute/path/to/assets` to use an external asset tree while developing artwork. Blender is needed only to edit artwork.

`./tools/package-bevy.sh` creates a portable directory under `dist/` containing the executable and accompanying documentation/license notices. The game is **asset-self-contained, not fully statically linked**: Linux still needs system libraries such as glibc, ALSA and libudev, and all platforms need their graphics drivers and OS windowing/audio support. Keep the accompanying notices when redistributing.

### Runtime options

Pass options after the executable name, for example `./tools/run.sh --vulkan --capture-school --fullhd`.

| Option | Description |
|---|---|
| `--dx12` | Select Direct3D 12 on Windows. This is already the Windows default; ignored on other platforms. |
| `--vulkan` | Select Vulkan on Windows or macOS for backend testing; it is the Linux default. |
| `--metal` | Select Metal on macOS. This is already the macOS default; ignored on other platforms. |
| `--direct-draw` | Disable Bevy's indirect drawing for the scene camera and use regular draw calls. Diagnostic A/B option; indirect drawing remains the default. |
| `--fullhd` | Use a 1920×1080 window instead of the default 1280×720. |
| `--verify-input` | Run with a fixed 60 Hz time step for deterministic input verification; opens in a window instead of fullscreen. |
| `--capture-<name>` | Run a visual/test scene; see the capture groups below. Captures do not save preferences. |

Capture names include `game`, `visit`, `crystal`, `throw`, `wave`, `song`, `combat`, `finale`, `school`; `boot`, `menu`, `settings`, `language`, `help`, `credits`, `victory`; `graphics-low`, `graphics-balanced`, `graphics-high`; `pose-idle`, other `pose-<name>` animation previews, and `rig`; `xbox`, `playstation`, `generic-pad`; and `occlusion` or `occlusion-solid`. Prefix a capture name with `hr-` or `it-` for Croatian or Italian, for example `--capture-hr-boot`. `--capture-school --fullhd` runs the 72-fish stress scene and reports frame times plus per-pass CPU/GPU timings. Captures are saved to the system temporary directory. `--capture-demo` runs a seeded, scripted 20-second scene for recording; it does not record video itself.

For development, `SABRINA_ASSETS=/absolute/path/to/assets` loads an external asset directory instead of the embedded assets, and `SABRINA_CAPTURE_AA=fxaa` enables FXAA in captures. `SABRINA_BUILD_JOBS=4` limits build-script compilation parallelism.

## Tagged releases

Push a version tag to build and publish a GitHub release:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds Linux x86_64 (Ubuntu 24.04 baseline), Windows x86_64 and macOS Apple Silicon packages. Archives include the asset-embedded executable, instructions, documentation and license notices, with a separate SHA-256 checksum for each download. Assets are not duplicated beside the executable. macOS packages are unsigned and not notarized. These are portable desktop packages, not Steam uploads or Android/iOS builds.

Tags must use `vMAJOR.MINOR.PATCH`, optionally followed by a prerelease suffix such as `v0.1.0-beta.1`. Tags with a suffix publish as prereleases. All platform builds and checks must succeed before publication. Release notes include installation guidance and GitHub's automatically generated changelog since the previous release. Creating a release does not replace platform playtesting or artwork-permission review.

## Music

The supplied MIDI arrangements are preserved in `art/music/`. The game plays
stereo Ogg renders: underwater exploration at 96 BPM during calm play and menus,
jellyfish combat at 144 BPM during visits, and a short reveal stinger at the
start of each visit. Music crossfades between states and ducks beneath voices
and Sabrina's song. The stinger uses the music volume setting and is independent
of magic effects.

The two loops are padded to complete musical bars, with a warm-up pass retaining
notes and reverb across the seam. A 30 ms phase blend smooths the loop boundary
without shortening the bar. The stinger plays once with its release tail.
To regenerate the checked-in audio on Debian/Ubuntu:

```sh
sudo apt install libfluidsynth3 fluid-soundfont-gm ffmpeg python3-venv
python3 -m venv /tmp/sabrina-music-env
/tmp/sabrina-music-env/bin/pip install -r tools/requirements-music.txt
/tmp/sabrina-music-env/bin/python -m tools.render_music \
  --soundfont /usr/share/sounds/sf2/FluidR3_GM.sf2
/tmp/sabrina-music-env/bin/python -m unittest discover -s tests -v
```

Rendering dependencies are authoring-only; players need neither MIDI software
nor a soundfont. See [asset licensing](ASSET_LICENSE.md) for the compositions and
the included instrument-sample notice.

## Working on the game

- `src/reef_sim/`: engine-independent components, game rules and fixed-step simulation.
- `src/sabrina/`: Bevy input, presentation, animation, rendering, audio and UI.
- `assets/`: runtime models, embedded textures, audio, fonts, UI and WESL shaders.
- `art/`: maintained Blender sources and contact samples.
- `tools/`: build, export, packaging and validation scripts.

See [architecture](docs/ARCHITECTURE.md) and [art workflow](docs/CHARACTER_PIPELINE.md).

```sh
cargo test --locked --workspace
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
python3 tools/validate_models.py
python3 tools/validate_render_lods.py
python3 tools/export_rust_contacts.py --check
./tools/run.sh --verify-input
```

## Next steps

- Refine animation, underwater lighting and encounter balance through playtesting.
- Broaden Windows hardware testing; test macOS and physical controllers.
- Add touch input and measure mobile hardware before Android/iOS/TV releases.
- Prepare store releases on Steam, Android and iOS once the game is ready.

## License

Original source code, shaders, scripts, tests, configuration and technical documentation are licensed under [Apache 2.0](LICENSE). See [NOTICE](NOTICE) for authorship. The code may be reused and redistributed, including commercially, under that license.

The game itself uses a hybrid licensing model: Sabrina's characters, story and world, 3D models, textures, UI artwork, music and recordings are **all rights reserved**, to the extent held by their respective rights holders. The title and logo are reserved branding; the code license grants no trademark rights. Third-party assets retain their own licenses. See [ASSET_LICENSE.md](ASSET_LICENSE.md).

Official builds combine Apache-licensed code with separately licensed game content; selling them does not make the code proprietary or grant permission to redistribute the reserved content. Any future Steamworks or mobile-platform SDK files remain subject to their own terms and must not be published without permission.
