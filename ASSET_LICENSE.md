# Asset licensing

The root MIT license covers original code, build and authoring scripts, tests,
configuration and technical documentation. It does not cover standalone artwork,
recordings, character designs or branding, and grants no trademark rights.

The models, packed textures, UI artwork, boot image and recordings are reserved
by their respective rights holders. A public code release does not by itself
grant permission to redistribute those assets.

| Assets | Provenance |
|---|---|
| Sabrina and twelve fish | `art/characters/sabrina.provenance.json`, `fish.provenance.json` and `manifest.json` |
| Jellyfish | Original project artwork; `art/characters/manifest.json` |
| Reef | Supplied underwater ruins model; `art/environments/manifest.json` |
| Crystal | Supplied crystal model; `art/props/manifest.json` |
| Derived mesh levels | Same artwork as the corresponding model; `assets/render_lods/manifest.json` |
| Voices and magic sounds | Original Scratch recordings; `assets/audio/PROVENANCE.json` |
| UI SVGs, PNGs and boot artwork | Original game artwork under `assets/ui/` |

The [original Scratch project](https://scratch.mit.edu/projects/1203591608/)
is the design reference. Its assets are not relicensed by this repository's
MIT license. The audio provenance records original asset IDs, hashes and names.
Review the applicable permissions before public redistribution.

The synthesized accompaniment is defined in `rust/sabrina/src/audio.rs`; the
source composition is covered by the code license. DejaVu Sans has its own
license, included at `assets/fonts/LICENSE-DejaVu.txt`.

Keep provenance for new artwork and recordings, and include applicable
third-party license notices with distributed releases.
