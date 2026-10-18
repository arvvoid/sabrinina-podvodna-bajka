# Contributing

Keep changes focused and explain what changes for the player. Run the checks in the README; inspect the game when changing animation, materials, UI or input.

Gameplay belongs in `reef_sim`, without Bevy types or rendering dependencies. Use plain components and free systems. Preserve the fixed update order where systems depend on each other. Avoid allocations in simulation loops and measure before adding optimization.

Presentation belongs in `sabrina`; shader code lives in `assets/shaders/`. Share meshes, materials and animation graphs where possible. Comments should explain a constraint or a non-obvious choice, rather than repeat the code.

Edit the maintained Blender files and commit their exports together. Keep generated contact samples and LODs current. Do not commit build outputs, screenshots from temporary checks, editor backups or credentials.

Source contributions use the MIT license. Artwork and audio need explicit provenance and licensing; the code license does not cover them.
