# Contributing

The project is currently an early desktop prototype. Keep changes focused and describe the player-visible outcome and validation.

Original source contributions are accepted under the repository's Apache 2.0 license. Artwork, audio, narrative content, and third-party resources require a separate explicit license/provenance entry in `ASSET_LICENSE.md`; do not assume the source license covers them.

For gameplay changes, run the simulation tests. For input or scene changes, also run the native smoke and input checks documented in the README. Inspect the running game for visual changes.

Keep gameplay in C++, preserve the fixed-step simulation's independence from Godot where practical, and measure before adding complex optimization. Avoid per-frame allocations in simulation loops. Do not commit engine binaries, build outputs, local settings, or credentials.
