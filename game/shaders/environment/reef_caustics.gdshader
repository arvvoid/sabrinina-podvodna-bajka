// SPDX-License-Identifier: MIT
shader_type spatial;
uniform vec4 tint : source_color = vec4(1.0);
uniform bool textured = false;
uniform sampler2D albedo_texture : source_color, filter_linear_mipmap_anisotropic;
uniform sampler2D normal_texture : hint_normal, filter_linear_mipmap_anisotropic;
uniform sampler2D roughness_texture : filter_linear_mipmap_anisotropic;
uniform float movement = 1.0;
uniform float strength = 1.0;
varying vec3 world;
void vertex() {
    world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}
void fragment() {
    vec2 p = (world.xz + vec2(world.y * .24, world.y * .48)) * .95;
    float t = TIME * .32 * movement;
    float a = sin(p.x * 2.3 + sin(p.y * 1.8 + t) * 1.7 - t);
    float b = sin(p.y * 2.5 + sin(p.x * 1.7 - t) * 1.6 + t);
    float light = pow(max(0.0, 1.0 - abs(a + b) * 1.6), 8.0) * strength;
    float pores = sin(world.x * 39.0) * sin(world.y * 43.0) * sin(world.z * 37.0);
    float distance_layer = clamp((-world.z - 8.) / 22., 0., 1.);
    vec3 base = tint.rgb;
    if (textured) {
        base *= texture(albedo_texture, UV, distance_layer * 1.4).rgb;
        NORMAL_MAP = texture(normal_texture, UV).rgb;
        NORMAL_MAP_DEPTH = .65 * (1. - .4 * distance_layer);
        ROUGHNESS = max(.55, texture(roughness_texture, UV).g);
    } else {
        base *= .96 + pores * .035;
        ROUGHNESS = .64;
    }
    base = mix(base, vec3(.035, .35, .43), distance_layer * .12);
    ALBEDO = base;
    EMISSION = base * .12 + vec3(.12, .26, .18) * light;
}
