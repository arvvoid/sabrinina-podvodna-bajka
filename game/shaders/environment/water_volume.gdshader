// SPDX-License-Identifier: Apache-2.0
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled, fog_disabled;
uniform float movement = 1.0;
uniform int samples = 6;
void fragment() {
    vec2 p = UV;
    float density = 0.0;
    float t = TIME * .23 * movement;
    for (int i = 0; i < 12; i++) {
        if (i >= samples)
            break;
        float z = (float(i) + .5) / float(samples);
        float x = p.x + p.y * (.18 + .035 * sin(t)) + z * .12;
        float wave = sin(x * 34.0 + t + sin(z * 4.0 + t) * .35 + sin(p.y * 3.0 + t) * .18);
        float ray = pow(max(0.0, wave), 12.0);
        density += ray * exp(-z * 2.0) * (1.0 - p.y * .82) / float(samples);
    }
    float surface =
        pow(max(0.0, sin(p.x * 95.0 + sin(p.x * 18.0 + TIME * .4 * movement))), 9.0) * exp(-p.y * 32.0);
    ALBEDO = vec3(.30, .85, .72);
    ALPHA = density * .58 + surface * .17;
}
