// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
import bevy_render::view::View;
import super::surface_caustics::illumination;

@group(0) @binding(0) var scene: texture_2d<f32>;
@if(MULTISAMPLED)
@group(0) @binding(1) var depth: texture_depth_multisampled_2d;
@else
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> settings: vec4<f32>;
@group(0) @binding(4) var light_map: texture_2d<f32>;
@group(0) @binding(5) var light_sampler: sampler;

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(position.xy);
    let lit = textureLoad(scene, pixel, 0);
    let raw_depth = textureLoad(depth, pixel, 0);
    let uv = (position.xy - view.viewport.xy) / view.viewport.zw;
    let ndc = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), raw_depth, 1.0);
    let homogeneous = view.world_from_clip * ndc;
    let world = homogeneous.xyz / homogeneous.w;
    let dx = dpdx(world);
    let dy = dpdy(world);
    // Derivatives must run before divergent exits; distant reef has material caustics.
    if raw_depth <= 0.0 || world.z <= -3.0 {
        return lit;
    }
    let cross_normal = cross(dx, dy);
    let derivative_normal = cross_normal / max(length(cross_normal), 0.00001);
    let normal = select(-derivative_normal, derivative_normal,
        dot(derivative_normal, view.world_position.xyz - world) >= 0.0);
    // Suppress depth discontinuities instead of projecting across silhouette edges.
    let edge = 1.0 - smoothstep(0.15, 0.5, max(length(dx), length(dy)));
    // Reef materials handle their own caustics, including transparent foreground geometry.
    // The depth pass contributes only a restrained highlight in the actor layer.
    let actor_layer = smoothstep(-3.0, -2.0, world.z);
    let intensity = illumination(light_map, light_sampler, world, normal, settings.x, settings.y)
        * edge * actor_layer * 0.18;
    // A scalar illumination boost preserves RGB ratios, including black materials.
    return vec4<f32>(lit.rgb * (1.0 + intensity * select(0.0, 1.0, raw_depth > 0.0)), lit.a);
}
