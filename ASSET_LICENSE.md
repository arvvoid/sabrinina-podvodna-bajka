# Licensing scope

The root Apache 2.0 license covers the project's original source code, build scripts,
tests, configuration, and technical documentation. This includes procedural
geometry, shader, and sound-generation code in `src/`.

Original standalone artwork, character designs, narrative text, music,
recordings, and branding are excluded from that grant and remain reserved by
their respective rights holders unless explicitly licensed otherwise, to the
extent those rights are held. All rights reserved. The source license grants
no trademark rights in the game's name or branding beyond its stated exceptions.

The [original Scratch project](https://scratch.mit.edu/projects/1203591608/) is the design reference. Its embedded sprites, sounds and other materials are not relicensed by this repository’s Apache 2.0 license. Their provenance and licenses must be reviewed before reuse or public redistribution.

The native build retains gameplay voices, the original help recordings and magic sounds, converted to mono 32 kHz Ogg Vorbis in `game/assets/audio/`. `PROVENANCE.json` records each retained sound’s original sprite, name, asset ID, hash and duration. These recordings are excluded from Apache 2.0. The bundled Scratch archive, original story text and intro narration have been removed from this version; the credits link to the original project instead.
No original sprite image is used in the native scene. Characters are new 3D
interpretations of the original designs; those character rights remain reserved.

The editable models in `art/characters/*.blend`, their packed texture artwork,
and the exported `game/assets/models/*.glb` include artwork made for this project and the owner-supplied Sabrina and tropical fish models.
They remain reserved with the character designs. `art/characters/sabrina.provenance.json` and `art/characters/fish.provenance.json` record supplied model names, original
hash and preparation steps. The downloaded source remains unchanged; the editable
deliveries contain the rigged neutral-pose figure and twelve individually rigged fish.

`art/environments/reef_ruins.blend` and `game/assets/environments/` derive from the
owner-supplied underwater ruins GLB. Their source hash and adaptations are recorded
in `art/environments/manifest.json`. These models and textures are reserved artwork,
not Apache-licensed source. Godot-extracted PNG texture copies have the same scope as the GLBs.
No independent redistribution license for the supplied artwork is asserted here.

`art/characters/manifest.json` records the
source/export pairs, authoring version and budgets. The authoring and export
scripts in `tools/blender/` and asset validator are covered by the root Apache 2.0 license;
this does not relicense the character designs or exported artwork.
The accompaniment is synthesized from the source composition in `reef_audio.cpp`.

`game/assets/ui/boot.png` is a native-game render of Sabrina and the reef, captured with `--capture-boot`. It includes the character artwork, brief remake origin, author credit and branding and remains reserved with those assets. Documentation screenshots are likewise game renders.

The SVG decorations in `game/assets/ui/reef/` are original vector interface artwork for this native game, reserved with the game artwork. The bundled DejaVu Sans font in `game/assets/fonts/` is distributed under its own license, reproduced in `LICENSE-DejaVu.txt`.

`third_party/godot-cpp` retains its own MIT license and copyright notices.
Godot Engine is MIT licensed; see https://godotengine.org/license/ and include
its applicable notices with binary releases. Third-party notices must accompany
redistributed dependencies. Future asset files require explicit provenance and
licensing entries here before distribution.

`art/props/ocean_crystal.blend` and `game/assets/props/` derive from the owner-supplied crystal dagger GLB. The source hash, reductions and embedded map budget are recorded in `art/props/manifest.json`. These models, extracted texture copies and packed textures are reserved artwork, not Apache-licensed source.

Include `LICENSE`, `NOTICE` and applicable third-party notices with authorized
distributions. Retaining notices does not itself grant permission to redistribute
reserved assets. This change does not revoke any previously granted licenses.
