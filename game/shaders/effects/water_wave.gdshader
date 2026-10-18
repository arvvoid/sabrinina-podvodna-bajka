// SPDX-License-Identifier: Apache-2.0
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled;
uniform float progress = 0.;
void fragment() {
    float side = (UV.y - .5) * 2.;
    float crest = .72 - .40 * side * side;
    float main = exp(-pow((UV.x - crest) / .075, 2.));
    float trail = .35 * exp(-pow((UV.x - crest + .22) / .09, 2.));
    float edge = (1. - smoothstep(.65, 1., abs(side)));
    float fade = smoothstep(0., .10, progress) * (1. - smoothstep(.65, 1., progress));
    ALBEDO = mix(vec3(.10, .85, 1.), vec3(.85, 1., .92), main);
    ALPHA = (main + trail) * edge * fade * .85;
}
