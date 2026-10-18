// SPDX-License-Identifier: MIT
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled;
void fragment() {
    float r = length(UV - vec2(.5)) * 2.0;
    ALBEDO = vec3(.25, .65, 1.);
    ALPHA = pow(max(0., 1. - r), 3.) * .5;
}
