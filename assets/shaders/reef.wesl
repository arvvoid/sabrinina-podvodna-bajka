// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
// Moving caustics extend Bevy's standard material without replacing its lighting.
import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> settings: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<uniform> sabrina: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, front);
    let world = in.world_position.xyz;
    let coordinates = (world.xz + vec2<f32>(world.y * 0.24, world.y * 0.48)) * 0.95;
    let time = settings.x * 0.32 * settings.y;
    let wave_x = sin(coordinates.x * 2.3 + sin(coordinates.y * 1.8 + time) * 1.7 - time);
    let wave_y = sin(coordinates.y * 2.5 + sin(coordinates.x * 1.7 - time) * 1.6 + time);
    let light = pow(max(0.0, 1.0 - abs(wave_x + wave_y) * 1.6), 8.0) * settings.z;
    let depth = clamp((-world.z - 8.0) / 22.0, 0.0, 1.0);
    pbr.material.base_color = vec4<f32>(mix(pbr.material.base_color.rgb, vec3<f32>(0.035, 0.35, 0.43), depth * 0.12), 1.0);
    pbr.material.perceptual_roughness = max(0.55, pbr.material.perceptual_roughness);
    pbr.material.emissive = vec4<f32>(pbr.material.base_color.rgb * 0.12 + vec3<f32>(0.12, 0.26, 0.18) * light, 0.0);
    var output: FragmentOutput;
    output.color = apply_pbr_lighting(pbr);
    output.color = main_pass_post_lighting_processing(pbr, output.color);
    // Only the named foreground mesh blends, and only close to Sabrina's silhouette.
    let distance = length((world.xy - sabrina.xy) / vec2<f32>(2.8, 2.5));
    let reveal = (1.0 - smoothstep(0.35, 1.0, distance)) * sabrina.w;
    output.color.a *= 1.0 - 0.25 * reveal * step(sabrina.z + 0.2, world.z);
    return output;
}
