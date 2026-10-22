// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
fn illumination(
    light_map: texture_2d<f32>,
    light_sampler: sampler,
    world: vec3<f32>,
    normal: vec3<f32>,
    time: f32,
    strength: f32,
) -> f32 {
    let coordinates = (world.xz + world.y * vec2<f32>(0.4, 0.65)) * 0.16;
    let a = textureSampleLevel(light_map, light_sampler, coordinates + time * vec2<f32>(0.018, -0.012), 0.0).r;
    let b = textureSampleLevel(light_map, light_sampler, coordinates * 1.13 + time * vec2<f32>(-0.013, 0.016), 0.0).r;
    let facing = 0.25 + 0.75 * max(dot(normal, normalize(vec3<f32>(0.32, 0.88, 0.35))), 0.0);
    let depth_fade = exp(-max(20.0 - world.y, 0.0) * 0.035);
    return min(1.6, max(a, b * 0.8) * strength * depth_fade * facing);
}
