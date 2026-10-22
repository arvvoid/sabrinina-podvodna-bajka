// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> state: vec4<f32>;
@group(1) @binding(1) var icon: texture_2d<f32>;
@group(1) @binding(2) var icon_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(icon, icon_sampler, in.uv);
    let offset = in.uv - vec2<f32>(0.5);
    let angle = fract(atan2(offset.x, -offset.y) / 6.2831853 + 1.0);
    let reveal = select(select(0.0, 1.0, angle < state.x), 1.0, state.x >= 1.0);
    let gray = dot(sample.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let muted = vec3<f32>(gray * 0.28);
    let color = mix(muted, sample.rgb, reveal * mix(1.0, 0.35, state.y));
    return vec4<f32>(color * (1.0 + state.z * 0.12), sample.a);
}
