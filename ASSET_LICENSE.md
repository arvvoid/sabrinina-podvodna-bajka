# Asset licensing

The root Apache License 2.0 covers original source code (including shaders),
build and authoring scripts, tests, configuration and technical documentation.
It does not cover the game content or branding listed below.

Sabrina's characters, story and world, models, packed textures, UI artwork,
music and recordings are **all rights reserved**, to the extent held by their
respective rights holders. The game title and logo are reserved branding;
the code license grants no trademark rights beyond its stated exceptions.
A public code release does not by itself grant permission to copy, redistribute
or commercially reuse these assets. Third-party licenses and any permissions
previously granted by the applicable rights holders remain unaffected.

## Commercial builds and platform integrations

Official commercial builds combine Apache-licensed code with separately
licensed game content. Apache 2.0 continues to apply to that code; it does
not grant rights to the reserved content embedded in the executable.
Distribution of reserved content requires the relevant rights holders'
permission. No separate commercial-build EULA is introduced here.

Any future Steamworks, Android or iOS SDK components remain subject to their
respective platform terms, not this repository's code license. Keep SDK files
or other restricted material private when redistribution is not authorized.

## 3D artwork and current quality

The 3D models were created using generative AI and Blender. They are still
work in progress: the meshes, skeletons, skin weights and animations are far
from perfect, with plenty of room for improvement as development continues.
This describes the creation process and current quality; it does not change
the asset licensing terms above.

| Assets | Origin and scope |
|---|---|
| Sabrina and twelve fish | Adapted from owner-supplied models; maintained Blender sources in `art/characters/` |
| Jellyfish | Original project artwork in `art/characters/jellyfish.blend` |
| Reef | Adapted from an owner-supplied underwater ruins model; `art/environments/reef_ruins.blend` |
| Crystal | Adapted from an owner-supplied crystal model; `art/props/ocean_crystal.blend` |
| Derived mesh levels | Same artwork and restrictions as the corresponding model |
| Voices and magic sounds | Recordings carried over from the original Scratch project |
| Calm music, visitor music and arrival stinger | Owner-supplied generated MIDI compositions in `art/music/`, rendered to `assets/audio/` with FluidSynth and FluidR3 GM |
| UI SVGs, PNGs and JPEGs, including loading screen, localized title logos and application icon | Project artwork under `assets/ui/` and `art/icons/`; the loading-screen image, English, Croatian and Italian title PNGs and application icon were supplied for this project. Derived ICO/ICNS/window icons carry the same restrictions |

The [original Scratch project](https://scratch.mit.edu/projects/1203591608/)
is the design reference. Its assets are not relicensed by this repository's
Apache 2.0 license. Its applicable licenses and any previously granted
permissions are unchanged.
Review the applicable permissions before public redistribution.

The supplied MIDI compositions and rendered music are reserved separately
from the code, like the other audio assets above. FluidR3 instrument samples
are MIT licensed; their notice is included at `assets/audio/LICENSE-FluidR3.txt`.
Neither the soundfont nor FluidSynth is bundled or required at runtime.
DejaVu Sans has its own license, included at `assets/fonts/LICENSE-DejaVu.txt`.
Alegreya, by Juan Pablo del Peral and Huerta Tipográfica, is used for player-facing
text. It is distributed unmodified from [Google Fonts](https://github.com/google/fonts/tree/main/ofl/alegreya)
under the SIL Open Font License 1.1, included at `assets/fonts/LICENSE-Alegreya.txt`.
The license permits commercial use, embedding and redistribution with the game;
the font itself may not be sold on its own. Its Latin Extended glyphs include
Croatian Č Ć Đ Š Ž and č ć đ š ž.

Include applicable third-party license notices with distributed releases.
Runtime assets and their notices are embedded in the executable. Desktop
packages also place the Alegreya, DejaVu and FluidR3 notices in `licenses/`; retain these
along with the generated Rust dependency notices in `licenses/THIRD_PARTY_LICENSES.txt`
and the root `LICENSE`, `NOTICE` and asset-license notice (packaged unchanged as
`licenses/ASSET_LICENSE.txt` from `ASSET_LICENSE.md`) when distributing an
authorized package. Keeping notices alone does not grant permission to
redistribute reserved content. Embedding assets does not
change any of the rights or restrictions described above.
