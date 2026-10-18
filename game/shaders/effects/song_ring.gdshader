// SPDX-License-Identifier: MIT
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled;
uniform float opacity = 0.2;
void fragment() {
    ALBEDO = vec3(1., .52, .82);
    ALPHA = opacity * sin(UV.x * 3.14159);
}
