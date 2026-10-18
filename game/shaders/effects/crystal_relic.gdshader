// SPDX-License-Identifier: Apache-2.0
shader_type spatial;
render_mode cull_disabled;
uniform sampler2D albedo_texture : source_color, filter_linear_mipmap;
uniform sampler2D normal_texture : hint_normal, filter_linear_mipmap;
uniform float phase = 0.;
uniform float charge = 0.;
void fragment() {
    vec3 color = texture(albedo_texture, UV).rgb;
    float water = smoothstep(.02,.30,color.b-color.r);
    float rim = pow(1. - max(0.,dot(NORMAL,VIEW)),3.);
    float shimmer = .5+.5*sin(UV.y*28.-phase*2.);
    ALBEDO = color;
    NORMAL_MAP = texture(normal_texture, UV).rgb;
    NORMAL_MAP_DEPTH = .45;
    METALLIC = mix(.65,.24,water);
    ROUGHNESS = mix(.32,.19,water);
    EMISSION = color*water*(.18+.22*charge)+vec3(.15,.65,.85)*rim*(.10+.25*charge)*shimmer;
}
