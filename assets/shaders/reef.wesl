// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
// Standard reef lighting with a local foreground reveal.
import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
import bevy_pbr::render::mesh_view_bindings::globals;
import super::surface_caustics::illumination;
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> caustics: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<uniform> sabrina: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var light_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var light_sampler: sampler;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, front);
    let world = in.world_position.xyz;
    var output: FragmentOutput;
    output.color = apply_pbr_lighting(pbr);
    let light = illumination(light_map, light_sampler, world, pbr.N, globals.time * caustics.x, caustics.y);
    output.color = vec4<f32>(output.color.rgb * (1.0 + light), output.color.a);
    output.color = main_pass_post_lighting_processing(pbr, output.color);
    // Only the named foreground mesh blends, and only close to Sabrina's silhouette.
    let distance = length((world.xy - sabrina.xy) / vec2<f32>(2.8, 2.5));
    let reveal = (1.0 - smoothstep(0.35, 1.0, distance)) * sabrina.w;
    output.color.a *= 1.0 - 0.25 * reveal * step(sabrina.z + 0.2, world.z);
    return output;
}
