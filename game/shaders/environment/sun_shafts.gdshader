// SPDX-License-Identifier: MIT
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled, fog_disabled;
uniform float seed = 0.0;
uniform float movement = 1.0;
void fragment() {
    float centre = .5 + sin(UV.y * 2.5 + TIME * .24 * movement + seed) * .12;
    float width = .14 + UV.y * .23;
    float beam = exp(-pow((UV.x - centre) / width, 2.0) * 3.0);
    float fade = smoothstep(0.0, .12, UV.y) * (1.0 - smoothstep(.65, 1.0, UV.y));
    ALBEDO = vec3(.28, .65, .72);
    ALPHA = beam * fade * (.17 + .07 * sin(TIME * .45 * movement + seed));
}
