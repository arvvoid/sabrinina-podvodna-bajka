# Sabrinina podvodna bajka

*Sabrina’s Underwater Fairy Tale*

A small underwater game about a mermaid, tropical fish and a hungry jellyfish. Swim through the reef, keep the fish safe with song and magic, or switch to quiet aquarium mode and enjoy the sea.

![Sabrina swimming among tropical fish in the underwater reef](docs/screenshots/gameplay.png)

*The current Rust and Bevy remake — an early working version, not the finished game.*

## How it started

I made the [original Scratch game](https://scratch.mit.edu/projects/1203591608/) together with my daughter Sabrina, who was four at the time. Sabrina came up with the idea and design; I helped as her dad with teaching, technical help and explanations. Together, we built a little underwater world with swimming fish, a hungry jellyfish and a mermaid who protects the fish with magic.

That Scratch project became a 3D Godot prototype, then this Rust and Bevy remake: a full standalone game growing out of our father-and-daughter project. The side-on view, voices, crystal magic and peaceful pauses come from that original game.

**Development:** Luka Dežulović

**Original idea:** Sabrina Dežulović

The Godot prototype is preserved on [`legacy_godot`](https://github.com/arvvoid/sabrinina-podvodna-bajka/tree/legacy_godot). `main` contains the Bevy game.

## Current status and release plans

This is an initial working version. It is playable, but still needs more work, refinement and playtesting before it is ready for release. Once it is ready, I plan to publish it on **Steam, Android and iOS**. These are planned releases, not currently available versions; there is no release date yet.

## Playing

Fish arrive during peaceful intervals. When jellyfish visit, use song to gather fish, a wave to push them out of danger, or the crystal to repel a visitor. Jellyfish grow as they eat. Contact stings Sabrina; health slowly returns after a safe period. Losing all health ends the game.

Song uses energy and its reach shrinks as energy falls. If it runs out, it must recharge completely. The crystal takes much longer to recover, so save it for trouble. Rescue fish, protect different species and finish encounters with survivors to build your score. Settings offer Gentle and Lively difficulty, encounter pacing and a peaceful aquarium mode.

| Action | Mouse / keyboard | Controller |
|---|---|---|
| Swim | Pointer, WASD / arrows | Left stick |
| Crystal | Left click, Space / E | A / bottom button or right shoulder |
| Song | Q or HUD button | X / left button |
| Wave | Right click, F or HUD button | B / right button |
| Pause | Escape | Start |
| Fullscreen | F11 or Settings | Settings |

English, Croatian and Italian are available. Graphics, sound, controls and best score are saved locally. The first launch uses fullscreen and Balanced graphics. Balanced has subtle soft shadows; High adds finer shadows and 2× MSAA; Light reduces effects and disables shadows.

## Build and run

Install [Rust](https://rustup.rs/). The toolchain and dependencies are pinned in `rust-toolchain.toml` and `Cargo.lock` (Rust 1.97.1, Bevy 0.20.0).

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

Linux uses Vulkan; Windows permits Vulkan and Direct3D 12; Apple targets use Metal. `SABRINA_RENDERER=vulkan|dx12|metal|gl` overrides the backend. OpenGL is experimental and has not passed local runtime testing. Linux is the tested desktop platform; Windows/macOS runtime, physical controllers and mobile exports still need testing.

`./tools/package-bevy.sh` creates a portable directory under `dist/`. Runtime assets are found beside the executable or in the source tree; `SABRINA_ASSETS=/absolute/path` overrides this. Blender is needed only to edit artwork.

## Tagged releases

Push a version tag to build and publish a GitHub release:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds Linux x86_64 (Ubuntu 24.04 baseline), Windows x86_64 and macOS Apple Silicon packages. Archives include the executable, runtime assets, instructions, documentation and license notices, with a separate SHA-256 checksum for each download. macOS packages are unsigned and not notarized. These are portable desktop packages, not Steam uploads or Android/iOS builds.

Tags must use `vMAJOR.MINOR.PATCH`, optionally followed by a prerelease suffix such as `v0.1.0-beta.1`. Tags with a suffix publish as prereleases. All platform builds and checks must succeed before publication. Release notes include installation guidance and GitHub's automatically generated changelog since the previous release. Creating a release does not replace platform playtesting or artwork-permission review.

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

For visual checks, use `--capture-game`, `--capture-visit`, `--capture-throw`, `--capture-pose-idle` or `--capture-graphics-high`. `--capture-school --fullhd` runs a 72-fish stress scene and reports frame times. Captures go to the system temporary directory and do not save preferences.

## Next steps

- Refine animation, underwater lighting and encounter balance through playtesting.
- Test Windows, macOS and physical controllers.
- Add touch input and measure mobile hardware before Android/iOS/TV releases.
- Prepare store releases on Steam, Android and iOS once the game is ready.

## License

Code and technical documentation are MIT licensed. Artwork, recordings, character designs and branding are reserved separately; the font has its own license. See [ASSET_LICENSE.md](ASSET_LICENSE.md).
