// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions::get_world_from_local,
    mesh_view_bindings::view,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> tint: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> settings: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> state: vec4<f32>;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn random(seed: f32) -> f32 {
    return 0.5 + 0.5 * sin(seed * 17.137 + seed * seed * 0.71);
}

@vertex
fn vertex(input: Vertex) -> Output {
    let v = decompress_vertex(input, input.instance_index);
    let id = v.color.x;
    let time = settings.x;
    let kind = i32(settings.y);
    var center = vec3<f32>(0.0);
    var size = 1.0;
    if kind == 3 {
        center = vec3<f32>(
            -18.0 + 36.0 * random(id + 900.0),
            max(0.3, (18.0 * random(id + 950.0) + time * (0.12 + 0.06 * random(id))) % 19.0),
            -5.0 + 8.0 * random(id + 999.0));
        size = mix(0.035, 0.055, random(id + 50.0));
    }
    if kind == 1 {
        let angle = id * 2.39996 + time * 2.0 * settings.z;
        let bursting = state.x >= 0.0;
        let radius = select(0.5 + 0.25 * sin(angle * 1.7), state.x * 2.8, bursting);
        center = vec3<f32>(cos(angle) * radius, sin(angle) * radius, sin(angle * 2.0) * 0.3);
        size = 0.045 * select(0.65 + 0.35 * sin(angle), max(0.0, 1.0 - state.x), bursting);
    }
    if kind == 2 {
        let angle = id * 1.047198;
        center = vec3<f32>(cos(angle), sin(angle), 0.0) * state.x * 0.8;
        size = 0.1 * (1.0 - state.x);
    }
    let model = get_world_from_local(input.instance_index);
    let world_center = model * vec4<f32>(center, 1.0);
    let scale = length(model[0].xyz);
    let offset = (v.uv * 2.0 - 1.0) * size * scale;
    let world = world_center.xyz + view.world_from_view[0].xyz * offset.x
        + view.world_from_view[1].xyz * offset.y;
    var out: Output;
    out.position = view.clip_from_world * vec4<f32>(world, 1.0);
    if id >= settings.w {
        out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    }
    out.uv = v.uv;
    return out;
}

@fragment
fn fragment(input: Output) -> @location(0) vec4<f32> {
    let offset = input.uv * 2.0 - 1.0;
    let radius_squared = dot(offset, offset);
    let soft = max(0.0, 1.0 - radius_squared);
    var alpha = soft * soft;
    if i32(settings.y) == 1 {
        let cross = exp(-abs(offset.x * offset.y) * 36.0) * soft;
        alpha = max(alpha * 0.65, cross);
    }
    let brightness = select(0.7, 1.8, i32(settings.y) == 3);
    return vec4<f32>(tint.rgb * alpha * brightness, 0.0);
}
