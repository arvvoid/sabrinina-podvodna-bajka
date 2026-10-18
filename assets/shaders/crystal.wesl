// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
// Blue regions shimmer like water; the remaining filigree stays metallic.
import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> settings: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, front);
    let color = pbr.material.base_color.rgb;
    let water = smoothstep(0.02, 0.30, color.b - color.r);
    let rim = pow(1.0 - max(0.0, dot(pbr.N, pbr.V)), 3.0);
    let shimmer = 0.5 + 0.5 * sin(in.uv.y * 28.0 - settings.x * 2.0);
    pbr.material.metallic = mix(0.65, 0.24, water);
    pbr.material.perceptual_roughness = mix(0.32, 0.19, water);
    pbr.material.emissive = vec4<f32>(
        color * water * (0.18 + 0.22 * settings.y)
            + vec3<f32>(0.15, 0.65, 0.85) * rim * (0.10 + 0.25 * settings.y) * shimmer,
        0.0,
    );
    var output: FragmentOutput;
    output.color = apply_pbr_lighting(pbr);
    output.color = main_pass_post_lighting_processing(pbr, output.color);
    return output;
}
