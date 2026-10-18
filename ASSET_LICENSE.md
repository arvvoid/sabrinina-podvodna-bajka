# Asset licensing

The root MIT license covers original code, build and authoring scripts, tests,
configuration and technical documentation. It does not cover standalone artwork,
recordings, character designs or branding, and grants no trademark rights.

The models, packed textures, UI artwork, boot image and recordings are reserved
by their respective rights holders. A public code release does not by itself
grant permission to redistribute those assets.

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
| UI SVGs, PNGs and boot artwork | Original game artwork under `assets/ui/` |

The [original Scratch project](https://scratch.mit.edu/projects/1203591608/)
is the design reference. Its assets are not relicensed by this repository's
MIT license.
Review the applicable permissions before public redistribution.

The supplied MIDI compositions and rendered music are reserved separately
from the code, like the other audio assets above. FluidR3 instrument samples
are MIT licensed; their notice is included at `assets/audio/LICENSE-FluidR3.txt`.
Neither the soundfont nor FluidSynth is bundled or required at runtime.
DejaVu Sans has its own license, included at `assets/fonts/LICENSE-DejaVu.txt`.

Include applicable third-party license notices with distributed releases.
Runtime assets and their notices are embedded in the executable. Desktop
packages also place the DejaVu and FluidR3 notices in `licenses/`; retain these
and the root license documents when redistributing. Embedding assets does not
change any of the rights or restrictions described above.
