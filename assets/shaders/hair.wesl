// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> settings: vec4<f32>;

@fragment
fn fragment(input: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(input, front);
    pbr.material.base_color = alpha_discard(
        pbr.material.flags, pbr.material.alpha_cutoff, pbr.material.base_color);
    pbr.material.base_color = vec4<f32>(
        mix(pbr.material.base_color.rgb, vec3<f32>(1.0, 0.16, 0.12), settings.w * 0.3),
        pbr.material.base_color.a);
    var output: FragmentOutput;
    output.color = apply_pbr_lighting(pbr);
    output.color = main_pass_post_lighting_processing(pbr, output.color);
    return output;
}
