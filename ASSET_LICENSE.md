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
| UI SVGs, PNGs and boot artwork | Original game artwork under `assets/ui/` |

The [original Scratch project](https://scratch.mit.edu/projects/1203591608/)
is the design reference. Its assets are not relicensed by this repository's
MIT license.
Review the applicable permissions before public redistribution.

The synthesized accompaniment is defined in `src/sabrina/src/audio.rs`; the
source composition is covered by the code license. DejaVu Sans has its own
license, included at `assets/fonts/LICENSE-DejaVu.txt`.

Include applicable third-party license notices with distributed releases.
