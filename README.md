# Sabrinina podvodna bajka
# Sabrina's Underwater Fairy Tale

[![Test](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/rust.yml/badge.svg)](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/rust.yml)
[![CodeQL](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/arvvoid/sabrinina-podvodna-bajka/actions/workflows/github-code-scanning/codeql)
[![Rust 1.99.1](https://img.shields.io/badge/Rust-1.99.1-orange?logo=rust)](https://www.rust-lang.org/)
[![Bevy 0.20.0](https://img.shields.io/badge/Bevy-0.20.0-blue)](https://bevy.org/)
[![Code: Apache 2.0](https://img.shields.io/badge/Code-Apache%202.0-green.svg)](LICENSE)

A small underwater game I am making with my daughter Sabrina.

![Sabrina swimming among tropical fish in the underwater reef](docs/screenshots/gameplay.png)

**This is the current Rust/Bevy version. It works and is playable, but it is still a work in progress.**

### See it in motion

[![Animated gameplay: Sabrina uses crystal magic, a close wave and song to protect the fish](docs/media/gameplay-preview.gif)](docs/media/gameplay.mp4)

[Watch or download the 20-second gameplay video (MP4, 720p)](docs/media/gameplay.mp4). The short GIF loops without sound. The full video has the combat music. Both were captured from the actual game with scripted inputs, not rendered separately.

## How it started

This started as a [Scratch game](https://scratch.mit.edu/projects/1203591608/) I made together with my daughter Sabrina when she was four. The idea and game design were hers. I was there as dad, explaining things, helping her learn and doing the technical parts with her. We ended up with an underwater world full of fish, a hungry jellyfish and Sabrina the mermaid protecting everyone with magic.

I then tried turning it into a 3D game in Godot and C++ (it works there is a branch containing that legacy version), before deciding to remake the whole thing in Rust and Bevy. The point was not to turn it into some completely different game. I wanted to keep the things Sabrina came up with: the side view, the voices, the crystal, the magic and the peaceful parts between attacks. It is still our little project, just a lot more ambitious now.

**Development:** Luka Dežulović

**Original idea:** Sabrina Dežulović

The Godot prototype is preserved on [`legacy_godot`](https://github.com/arvvoid/sabrinina-podvodna-bajka/tree/legacy_godot). `main` contains the Bevy game.

## Current status

The game is already playable, but it is still under development. There are animations to improve, gameplay to balance and plenty of testing left.

I plan to release it on Steam and eventually Android and iOS. I am not setting release dates until I am happy with the result.

For now, Linux and Windows are the main tested platforms. macOS builds are also planned, but I still need to properly test them.

## Gameplay

The idea is simple. Sabrina is a mermaid protecting an underwater reef full of tropical fish.

During peaceful periods, fish swim around naturally and the game feels like a virtual aquarium. Every now and then jellyfish appear and try to eat the fish. Sabrina can protect them using three magical abilities.

You can play through 10 rounds, try to get a good score and defeat the jellyfish, or simply switch to Aquarium mode and enjoy the underwater world without enemies or competition.

Jellyfish grow as they eat fish. Each has 100 health and must be defeated before it retreats. They can still sting Sabrina or harm fish on accidental contact while leaving the scene.

Sabrina slowly recovers health when she stays out of danger. If she loses all her health or all the fish are eaten, the game ends.

The final round always has four jellyfish. Defeat all four while keeping Sabrina and at least one fish alive to win. There is no round 11, although Aquarium mode can continue indefinitely.

### Magic abilities

**Crystal**

Sabrina has a magical crystal orbiting around her. She can throw it at jellyfish, dealing 40 damage and trapping them in an air bubble for three seconds.

After a successful hit, you have 1.5 seconds to send the crystal toward another jellyfish. You can chain up to four different targets, dealing 40, 20, 10 and 5 damage. Each target gets its own three-second bubble.

The crystal returns like a boomerang when you stop chaining attacks. Its cooldown starts when thrown and lasts 12 seconds, with two additional seconds for each successful chained hit, up to 18 seconds.

In Gentle mode the first throw uses automatic aiming. In Lively mode you have to aim by positioning Sabrina.

**Wave**

Sabrina can create a magical wave that pushes fish and jellyfish away.

A normal wave travels 5.5 world units, deals 10 damage to each jellyfish it hits and has a three-second cooldown.

Holding the button for three seconds charges a much larger wave with double the range and width. It still deals 10 damage but has a nine-second cooldown.

The wave can also push jellyfish trapped inside bubbles, although they cannot move, sting or eat while trapped.

**Song**

Sabrina can sing to gather nearby fish and attract jellyfish toward her.

This makes it possible to lead jellyfish away from the fish or bring scattered fish together. The ability consumes energy, and its range becomes smaller as energy runs out.

Once completely exhausted, it needs to recharge before being used again.

### Scoring and progression

I wanted the score to reflect what you actually accomplish rather than reward leaving the game running.

You get one point for each point of actual damage, 100 points for defeating a jellyfish, 50 points for rescuing a fish that was genuinely in danger and 25 points for each surviving fish at the end of an encounter.

A fish can only give one rescue reward per round. Simply collecting fish that were already safe gives no points. A successful rescue requires actually getting the fish out of danger.

Getting stung can cost up to 100 points.

Jellyfish become a little faster as rounds progress, and multiple jellyfish become more likely to appear. Their health and damage remain the same.

You can select Gentle or Lively difficulty and adjust the number of jellyfish. The visitor limit applies to rounds 1 through 9, since round 10 always has four.

Best scores are stored locally. They are currently single-round records and are not normalized across different difficulty settings.

## Controls

The game supports keyboard, mouse and controllers. It automatically changes the on-screen button hints depending on the controller being used.

| Action | Keyboard / Mouse | Xbox | PlayStation |
|---|---|---|---|
| Swim | Mouse, WASD or arrows | Left stick | Left stick |
| Wave | Left click or F | A | × |
| Song | Q | B | ○ |
| Crystal | Right click or Space | X or RB | □ or R1 |
| Pause | Escape | Start | Options |
| Fullscreen | F11 | Settings | Settings |

You can also activate the three abilities directly from the icons at the bottom of the screen. Their cooldowns and remaining energy are displayed there.

Holding Wave charges it, while pressing Crystal again shortly after hitting a jellyfish chains the attack.

Menus can be navigated entirely with a controller, including the D-pad and analog stick.

Controller vibration is supported with Off, Gentle and Normal settings. It is used for magic attacks, hits and damage, not continuously while swimming around.

### Windows controllers

On Windows I use native XInput by default. It works with Xbox controllers and Steam Input's Xbox controller emulation.

Steam Input users should select a Gamepad layout. PlayStation controllers running through Steam Input will normally display Xbox button hints because that is how Steam presents the emulated controller.

XInput supports up to four controllers. Input is polled separately from rendering to avoid missing short button presses at lower frame rates.

For direct Sony or generic controllers, the game also supports Bevy/GilRs through Windows Gaming Input. Start the executable with `--wgi` to use that backend instead.

Linux and macOS use Bevy/GilRs.

Vibration depends on the controller, connection and driver. I still need to do more testing with physical controllers and Steam Input on Windows.

## Customization

The game supports English, Croatian and Italian. You can change the language directly from the main menu.

Graphics, sound, controls, controller vibration and best scores are saved locally.

### Tail colors

Sabrina's tail can be customized in Settings.

Available colors are None, Natural Green, Violet, Pink, Lagoon Blue, Coral, Gold and Magic.

None keeps the original texture colors. Magic adds a subtle animated combination of violet, pink and turquoise that flows continuously across the tail and fins.

The effect is done entirely in a shader, without extra textures or material updates every frame. Reduced motion settings keep the colors static.

The tail, hair, skin and costume have a softer satin material finish to avoid excessive shiny reflections underwater.

### Graphics

There are three graphics presets.

| Preset | Antialiasing | Shadows |
|---|---|---|
| Light | Off | Off |
| Balanced | FXAA | Soft 1024×1024 shadows |
| High | 4× MSAA | Soft 2048×2048 shadows, including fish |

Balanced is the default. The first launch starts in fullscreen.

V-sync is enabled by default and can be turned off to reduce input latency or allow uncapped frame rates, although this can introduce tearing.

An optional FPS counter is available in Settings.

The interface uses a small status panel for health, fish count, round and score. Ability icons show cooldowns and keyboard or controller bindings.

## Building from source

The game is written in Rust and currently uses Bevy 0.20.0 with Rust 1.99.1.

The toolchain and dependencies are pinned through `rust-toolchain.toml` and `Cargo.lock` so builds use consistent versions.

First install Rust from https://rustup.rs/.

On Debian or Ubuntu, install the required native dependencies and run the build scripts.

```sh
sudo apt install build-essential pkg-config libasound2-dev libudev-dev libx11-dev libxrandr-dev libxi-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev

./tools/build.sh
./tools/run.sh
```

The scripts create and run an optimized release build.

On Windows, install Rust and Visual Studio C++ Build Tools with the Windows SDK and ATL components.

On macOS, install Xcode Command Line Tools.

You can also build and run directly with Cargo.

```sh
cargo run --locked --release -p sabrina
```

Compilation parallelism can be limited using `SABRINA_BUILD_JOBS=4`.

Blender is only needed if you want to edit the original models or artwork.

### Graphics backends

The default rendering backends are Direct3D 12 on Windows, Vulkan on Linux and Metal on macOS.

Windows builds contain the DXC shader compiler, so there is no need to install a separate shader compiler or Windows SDK just to play the game.

Linux supports native Wayland and X11. The game automatically selects the appropriate window system, although you can force either one using `--wayland` or `--x11`.

I have included a few command-line options mainly for development, performance comparisons and debugging.

| Option | Purpose |
|---|---|
| `--dx12` | Direct3D 12 on Windows |
| `--vulkan` | Vulkan renderer |
| `--metal` | Metal on macOS |
| `--opengl` | OpenGL/GLES testing |
| `--x11` | Force X11 on Linux |
| `--wayland` | Force Wayland on Linux |
| `--direct-draw` | Force regular draw calls |
| `--indirect-draw` | Force indirect drawing |
| `--wgi` | Alternative Windows controller backend |
| `--fullhd` | Start at 1920×1080 |
| `--verify-input` | Deterministic input testing |
| `--capture-school` | Run the 72-fish stress scene |
| `--capture-demo` | Run a scripted gameplay demo |

There are additional capture modes for gameplay, menus, animations, character poses, controllers and different languages.

The complete capture and runtime options are available in the source code and development scripts.

For example:

```sh
./tools/run.sh --vulkan --capture-school --fullhd
```

An unsupported renderer option falls back to the platform default. If an explicitly selected backend cannot initialize, the game reports the failure rather than silently switching renderers.

## Performance and testing

One of my goals is to keep the game playable on reasonably old hardware, including integrated graphics.

The game uses shared meshes and materials, GPU skinning and other optimizations to reduce unnecessary work. There is still room for improvement, especially with animated fish and CPU overhead.

Some results from machines I tested on 20 October 2026:

| Hardware | Result |
|---|---|
| Intel i7-11800H, Intel UHD Graphics | 50–60 FPS, over 60 FPS with lower settings |
| Intel i7-11800H, NVIDIA RTX 3070 Laptop | Over 400 FPS |
| Intel i7-2600K, NVIDIA GTX 1060 | Stable 60+ FPS |

These are observations from my development machines, not controlled benchmarks or minimum system requirements. Resolutions, quality settings and test conditions were not identical.

Linux with Vulkan has been the main development environment and is working well.

Windows 11 with Direct3D 12 has also been tested, including gameplay, animations, effects and a stress scene containing 72 fish.

macOS Apple Silicon builds are part of the release workflow but have not yet been tested on actual hardware.

Android and iOS have not been ported or tested yet.

### Memory usage

I also measured memory usage on Linux with an RTX 3070 Laptop GPU, Vulkan, Balanced graphics and embedded assets.

| Scene | RAM | GPU memory |
|---|---|---|
| Main menu, 720p | About 1.4 GiB | 837 MiB |
| Combat, 1080p | About 1.4 GiB | 851 MiB |
| 72-fish stress test, 1080p | About 1.35 GiB | 851 MiB |

These are measurements from one development build, not memory requirements. The RAM measurements include mapped executable data and driver allocations. Different drivers, operating systems and settings can produce different results.

There is still plenty of optimization work I can do, particularly with model geometry, textures and animation.

## Releases and packaging

I prefer keeping distribution simple.

The game assets are embedded in the executable, so there is no separate directory full of models, textures and audio files that needs to be shipped with every build.

It is not a fully statically linked application. Linux still needs its normal system libraries, graphics drivers and audio support.

The packaging script creates a portable directory with the executable and required documentation.

```sh
./tools/package-bevy.sh
```

GitHub Actions builds release packages for Linux x86_64, Windows x86_64 and macOS Apple Silicon when a version tag is pushed.

For example:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow supports regular versions and prereleases such as `v0.1.0-beta.1`.

All platform builds and checks must succeed before the release is published. Each package includes a separate SHA-256 checksum.

macOS builds are currently unsigned and not notarized.

These are normal portable desktop builds, not Steam or mobile releases.

### Update checking

Standalone desktop builds check GitHub Releases in the background, never blocking loading or gameplay. Steam builds (`--features steam`) and Android/iOS builds do not read the update cache or contact GitHub.

A fresh installation schedules its first check for a random 1–3 days later. After each attempt, the next check is scheduled for 30 days plus a random 0–3 days later. Checks only run on a launch after the saved deadline; there is no background timer or service while the game is closed.

The last attempt, next check time and latest stable version are saved in `update-check.json` beside the local settings. Restarting does not reset the schedule. The saved version can display a small localized notice in the loading screen's upper-right corner without another request. The notice disappears after installing that version or a newer one.

The notice has no buttons, links or modal dialog and never exits the game. There is no automatic updater, installer or download.

Offline and failed attempts preserve the cached version and wait until the next 30–33-day deadline. Failures only go to the diagnostic log, with no error shown to the player. The attempt is saved before contacting GitHub, and simultaneous launches cannot make duplicate requests. Unreadable/unwritable history or a clock moved behind the last attempt safely skips the check.

Automated input verification and capture runs skip the cache and network check.

## Music and sound

I made separate music for peaceful underwater exploration and jellyfish encounters.

The calmer music runs at 96 BPM and the combat music at 144 BPM. A short musical cue plays when jellyfish appear.

The music transitions between states and automatically lowers its volume when Sabrina speaks or sings.

I keep the original MIDI arrangements in `art/music/`, while the actual game uses rendered Ogg audio.

The loops have been prepared to transition smoothly without obvious gaps or interrupted reverb.

If you want to regenerate the music on Debian or Ubuntu:

```sh
sudo apt install libfluidsynth3 fluid-soundfont-gm ffmpeg python3-venv

python3 -m venv /tmp/sabrina-music-env
/tmp/sabrina-music-env/bin/pip install -r tools/requirements-music.txt
/tmp/sabrina-music-env/bin/python -m tools.render_music \
  --soundfont /usr/share/sounds/sf2/FluidR3_GM.sf2
```

None of these tools are needed to play the game. They are only used when generating the audio assets.

## Project structure

I have tried to keep the project reasonably simple and avoid unnecessary dependencies or abstractions.

The actual game rules and simulation are separated from the rendering engine where it makes sense. I want to be able to replace parts of the implementation without having to rewrite everything around them.

`src/reef_sim/` contains the game rules and fixed-step simulation independently of Bevy.

`src/sabrina/` contains the Bevy application, rendering, animations, input, audio and interface.

`assets/` contains the runtime artwork, models, textures, audio, fonts and shaders.

`art/` contains the original Blender files and other artwork sources.

`tools/` contains scripts for building, packaging, exporting and testing.

There is also more detailed documentation in [ARCHITECTURE.md](docs/ARCHITECTURE.md) and [CHARACTER_PIPELINE.md](docs/CHARACTER_PIPELINE.md).

To run the main project checks:

```sh
cargo test --locked --workspace
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
python3 tools/validate_models.py
python3 tools/validate_render_lods.py
python3 tools/export_rust_contacts.py --check
./tools/run.sh --verify-input
```

## What is next

For now I am mostly working on improving the animations, underwater lighting, performance and gameplay balance.

I also want to test the game on more Windows hardware, properly test macOS, improve controller support and eventually add touch controls for mobile devices.

Steam is the first planned store release. Android and iOS should follow when the desktop version is in good shape and I have had time to test actual mobile hardware.

I am not particularly interested in rushing it. This started as something fun to make with Sabrina, and I would rather keep it that way.

## License

The source code, shaders, scripts, tests, configuration and technical documentation are released under the [Apache License 2.0](LICENSE).

You are free to use, modify and redistribute the code, including commercially, as long as you follow the license terms.

The artwork and game content are different.

Sabrina's characters, story, world, 3D models, textures, user interface artwork, music and recordings are not covered by the Apache license. They remain all rights reserved to their respective rights holders.

The game title and logo are also reserved. Third-party assets retain their original licenses.

See [ASSET_LICENSE.md](ASSET_LICENSE.md) and [NOTICE](NOTICE) for the details.

I wanted to make the code open source because I believe software should be something people can learn from, improve and build upon.

At the same time, the characters, artwork and story are part of the little world Sabrina and I created together, and I want to keep those separate from the code license.

The game might eventually be sold on Steam, but that does not change the fact that the source code is open source.

Most importantly, I hope Sabrina will still be able to find, build and play this game many years from now, even if all the original platforms and stores are long gone.
