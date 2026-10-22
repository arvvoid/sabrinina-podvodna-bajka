// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_functions::get_local_from_world,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
};
import bevy_pbr::render::mesh_view_bindings::globals;
import super::mermaid_surface::satin_roughness;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> color: vec4<f32>;

@fragment
fn fragment(input: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(input, front);
    pbr.material.perceptual_roughness = satin_roughness(pbr.N, pbr.material.perceptual_roughness);
    pbr.material.reflectance *= 0.45;
    pbr.material.metallic = min(pbr.material.metallic, 0.15);
    pbr.material.base_color = alpha_discard(
        pbr.material.flags, pbr.material.alpha_cutoff, pbr.material.base_color);
    if color.w > 0.0 {
        let source = pbr.material.base_color.rgb;
        var hue = color.rgb;
        if color.w > 1.5 {
            let time = select(globals.time, 0.0, color.w > 2.5);
            // All tail primitives share model space; UV islands would split the swirl.
            let local = (get_local_from_world(input.instance_index) * input.world_position).xyz;
            let center = local.xy * 0.22 + vec2<f32>(0.3, 0.25);
            let radius_squared = dot(center, center);
            let angle = time * 0.55 - radius_squared * 6.0;
            let rotation = vec2<f32>(cos(angle), sin(angle));
            let swirl = vec2<f32>(
                dot(center, vec2<f32>(rotation.x, -rotation.y)),
                dot(center, rotation.yx));
            let phase = fract(swirl.x * 0.95 + swirl.y * 0.4 + local.z * 0.12 - time * 0.09) * 3.0;
            let violet = vec3<f32>(0.16, 0.008, 0.32);
            let pink = vec3<f32>(0.32, 0.012, 0.13);
            let turquoise = vec3<f32>(0.006, 0.22, 0.19);
            let blend = smoothstep(0.0, 1.0, fract(phase));
            hue = select(
                select(mix(violet, pink, blend), mix(pink, turquoise, blend), phase >= 1.0),
                mix(turquoise, violet, blend), phase >= 2.0);
            let detail = mix(0.65, 1.0, sqrt(max(source.r, max(source.g, source.b))));
            let rim = 1.0 - clamp(dot(pbr.N, pbr.V), 0.0, 1.0);
            // Steady color-matched sheen, without pulsing brightness.
            pbr.material.emissive += vec4<f32>(
                hue * detail * (0.04 + rim * rim * 0.06), 0.0);
            pbr.material.perceptual_roughness = max(0.5, pbr.material.perceptual_roughness * 0.88);
            pbr.material.metallic = 0.2;
        }
        // Lift baked dark paint in perceptual space so it cannot bury the chosen hue.
        let detail = max(source.r, max(source.g, source.b));
        let magic = color.w > 1.5;
        let brightness = select(sqrt(detail), mix(0.65, 1.0, sqrt(detail)), magic);
        let recolored = hue * brightness;
        pbr.material.base_color = vec4<f32>(
            recolored, pbr.material.base_color.a);
    }
    var output: FragmentOutput;
    output.color = apply_pbr_lighting(pbr);
    output.color = main_pass_post_lighting_processing(pbr, output.color);
    return output;
}
