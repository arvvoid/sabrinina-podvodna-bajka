// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> color: vec4<f32>;

@fragment
fn fragment(input: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(input, front);
    let normal_dx = dpdx(pbr.N);
    let normal_dy = dpdy(pbr.N);
    pbr.material.base_color = alpha_discard(
        pbr.material.flags, pbr.material.alpha_cutoff, pbr.material.base_color);
    if color.w > 0.0 {
        let source = pbr.material.base_color.rgb;
        // Lift baked dark paint in perceptual space so it cannot bury the chosen hue.
        let detail = max(source.r, max(source.g, source.b));
        let recolored = color.rgb * sqrt(detail);
        pbr.material.base_color = vec4<f32>(
            recolored, pbr.material.base_color.a);
        // Filter subpixel normal-map highlights, after the roughness texture is sampled.
        let roughness = max(pbr.material.perceptual_roughness, 0.42);
        let variance = min(0.15 * (dot(normal_dx, normal_dx) + dot(normal_dy, normal_dy)), 0.18);
        let roughness_squared = roughness * roughness;
        let filtered = sqrt(sqrt(min(roughness_squared * roughness_squared + variance, 1.0)));
        pbr.material.perceptual_roughness = filtered;
        pbr.material.reflectance *= 0.6;
    }
    var output: FragmentOutput;
    output.color = apply_pbr_lighting(pbr);
    output.color = main_pass_post_lighting_processing(pbr, output.color);
    return output;
}
