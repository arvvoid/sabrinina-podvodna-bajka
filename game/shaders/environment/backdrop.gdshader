// SPDX-License-Identifier: Apache-2.0
shader_type spatial;
render_mode unshaded, fog_disabled;
// Authored display colors must also convert correctly on linear HDR renderers.
uniform vec4 upper_color : source_color = vec4(.075, .72, .80, 1.0);
uniform vec4 lower_color : source_color = vec4(.012, .23, .42, 1.0);
uniform vec4 haze_color : source_color = vec4(.22, .30, .19, 1.0);
void fragment() {
    vec2 uv = UV;
    vec3 c = mix(upper_color.rgb, lower_color.rgb, smoothstep(0.0, 1.0, uv.y));
    float haze = exp(-length((uv - vec2(.72, .04)) * vec2(3.0, 2.8)) * 4.0);
    c += haze_color.rgb * haze;
    float water = sin(uv.y * 20.0 + sin(uv.x * 6.0 + TIME * .06) * .7) * .008;
    ALBEDO = c + water;
}
