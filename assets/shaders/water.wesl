// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
import bevy_pbr::render::forward_io::VertexOutput;
struct Parameters {
    // clock = (time, effect kind, variation seed, effect amount).
    clock: vec4<f32>,
    color: vec4<f32>,
    // Two (phase, attenuation) pairs per vector, prepared by visuals.rs.
    depth_samples: array<vec4<f32>, 6>
}
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> parameters: Parameters;

fn ellipse(offset: vec2<f32>, radius: vec2<f32>) -> f32 {
    return (length(offset / radius) - 1.0) * min(radius.x, radius.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let time = parameters.clock.x;
    let kind = i32(parameters.clock.y);
    let seed = parameters.clock.z;
    let amount = parameters.clock.w;
    var color = parameters.color.rgb;
    var alpha = parameters.color.a;

    // Opaque background gradient.
    if (kind == 0) {
        let upper = vec3<f32>(0.00657, 0.477, 0.604);
        let lower = vec3<f32>(0.00093, 0.0432, 0.147);
        color = mix(upper, lower, smoothstep(0.0, 1.0, uv.y));
        let haze = exp(-length((uv - vec2<f32>(0.72, 0.04)) * vec2<f32>(3.0, 2.8)) * 4.0);
        color += vec3<f32>(0.0397, 0.0732, 0.0301) * haze;
        color += vec3<f32>(sin(uv.y * 20.0 + sin(uv.x * 6.0 + time * 0.06) * 0.7) * 0.008);
        alpha = 1.0;
    }

    // Soft shafts of sunlight.
    if (kind == 1) {
        let centre = 0.5 + sin(uv.y * 2.5 + time * 0.24 + seed) * 0.12;
        let width = 0.14 + uv.y * 0.23;
        let beam = exp(-pow((uv.x - centre) / width, 2.0) * 3.0);
        let fade = smoothstep(0.0, 0.12, uv.y) * (1.0 - smoothstep(0.65, 1.0, uv.y));
        color = vec3<f32>(0.28, 0.65, 0.72);
        alpha = beam * fade * (0.17 + 0.07 * sin(time * 0.45 + seed));
    }

    // Water caustics integrated through precomputed depth samples.
    if (kind == 2) {
        var density = 0.0;
        let clock = time * 0.23;
        let base = (uv.x + uv.y * (0.18 + 0.035 * sin(clock))) * 34.0 + clock + sin(uv.y * 3.0 + clock) * 0.18;
        for (var i = 0u; i < 12u; i++) {
            if (f32(i) >= amount) {
                break;
            }
            let pair = parameters.depth_samples[i / 2u];
            let depth_sample = select(pair.xy, pair.zw, i % 2u == 1u);
            let wave = max(0.0, sin(base + depth_sample.x));
            let squared = wave * wave;
            let fourth = squared * squared;
            density += fourth * fourth * fourth * depth_sample.y;
        }
        density *= 1.0 - uv.y * 0.82;
        let surface = pow(max(0.0, sin(uv.x * 95.0 + sin(uv.x * 18.0 + time * 0.4))), 9.0) * exp(-uv.y * 32.0);
        color = vec3<f32>(0.30, 0.85, 0.72);
        alpha = density * 0.58 + surface * 0.17;
    }

    // Distant silhouettes built from signed-distance shapes.
    if (kind == 3) {
        var offset = uv - vec2<f32>(0.5);
        offset.y += sin(time * 1.7 + seed + offset.x * 7.0) * 0.018 * (1.0 - smoothstep(-0.45, 0.1, offset.x));
        var distance = ellipse(offset, vec2<f32>(0.29, 0.095));
        let tail = max(abs(offset.y) - (0.035 + max(0.0, - offset.x - 0.29) * 0.95), max(offset.x + 0.27, - offset.x - 0.46));
        distance = min(distance, tail);
        let shape = i32(seed) % 4;
        if (shape == 1) {
            distance = min(distance, max(abs(offset.x + 0.02) * 0.7 - (0.16 - offset.y), max(0.07 - offset.y, offset.y - 0.19)));
            distance = min(distance, ellipse(offset - vec2<f32>(0.13, - 0.07), vec2<f32>(0.13, 0.035)));
        }
        if (shape == 2) {
            distance = ellipse(offset, vec2<f32>(0.16, 0.09));
            let wing = abs(offset.y) - 0.30 * (1.0 - smoothstep(0.0, 0.28, abs(offset.x + 0.025)));
            distance = min(distance, max(wing, abs(offset.x) - 0.25));
            distance = min(distance, max(abs(offset.y) - 0.012, max(offset.x + 0.1, - offset.x - 0.46)));
        }
        if (shape == 3) {
            distance = ellipse(offset, vec2<f32>(0.32, 0.125));
            distance = min(distance, tail);
            distance = min(distance, ellipse(offset - vec2<f32>(0.06, 0.105), vec2<f32>(0.10, 0.045)));
        }
        color = vec3<f32>(0.00194, 0.0637, 0.119);
        alpha = (1.0 - smoothstep(-0.008, 0.025, distance)) * 0.22;
    }

    // Directional wave crest.
    if (kind == 4) {
        let side = (uv.y - 0.5) * 2.0;
        let crest = 0.72 - 0.40 * side * side;
        let main = exp(-pow((uv.x - crest) / 0.075, 2.0));
        let trail = 0.35 * exp(-pow((uv.x - crest + 0.22) / 0.09, 2.0));
        let edge = 1.0 - smoothstep(0.65, 1.0, abs(side));
        let fade = smoothstep(0.0, 0.10, amount) * (1.0 - smoothstep(0.65, 1.0, amount));
        color = mix(vec3<f32>(0.10, 0.85, 1.0), vec3<f32>(0.85, 1.0, 0.92), main);
        alpha = (main + trail) * edge * fade * 0.85;
    }

    // Song and crystal-burst rings.
    if (kind == 5 || kind == 9) {
        let radius = length((uv - vec2<f32>(0.5)) * 2.0);
        color = select(vec3<f32>(1.0, 0.52, 0.82), vec3<f32>(0.51, 0.91, 1.0), kind == 9);
        alpha = exp(-pow((radius - 0.98) / 0.018, 2.0)) * amount;
    }

    // Crystal halo.
    if (kind == 6) {
        let radius = length(uv - vec2<f32>(0.5)) * 2.0;
        color = vec3<f32>(0.25, 0.65, 1.0);
        alpha = pow(max(0.0, 1.0 - radius), 3.0) * 0.5;
    }

    // Ribbons behind the flying crystal.
    if (kind == 7) {
        let x = uv.x;
        let y = (uv.y - 0.5) * 2.0;
        let bend = sin(x * 14.0 - time * 9.0) * (1.0 - x) * 0.45;
        let ribbon = exp(-pow((y - bend) / 0.075, 2.0)) + exp(-pow((y + bend) / 0.09, 2.0)) * 0.65;
        let mist = exp(-y * y * 12.0) * 0.20;
        let fade = smoothstep(0.0, 0.3, x) * (1.0 - smoothstep(0.83, 1.0, x));
        color = mix(vec3<f32>(0.16, 0.62, 1.0), vec3<f32>(0.65, 1.0, 0.93), x);
        alpha = (ribbon + mist) * fade * amount * 0.8;
    }

    // Protective fish shield.
    if (kind == 8) {
        let offset = (uv - vec2<f32>(0.5)) * 2.0;
        let radius = length(offset);
        let rim = exp(-pow((radius - 0.86) / 0.035, 2.0));
        let shine = exp(-dot(offset - vec2<f32>(-0.45, - 0.55), offset - vec2<f32>(-0.45, - 0.55)) * 160.0);
        color = vec3<f32>(0.35, 0.88, 1.0);
        alpha = (rim * 0.4 + shine * 0.7) * amount;
    }
    // Additive effects premultiply their RGB and leave destination alpha unchanged.
    if (kind != 0 && kind != 3) {
        return vec4<f32>(color * alpha, 0.0);
    }
    return vec4<f32>(color, alpha);
}
