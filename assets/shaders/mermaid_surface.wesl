// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
fn satin_roughness(normal: vec3<f32>, roughness: f32) -> f32 {
    let dx = dpdx(normal);
    let dy = dpdy(normal);
    let variance = min(0.15 * (dot(dx, dx) + dot(dy, dy)), 0.18);
    let squared = max(roughness, 0.58) * max(roughness, 0.58);
    return sqrt(sqrt(min(squared * squared + variance, 1.0)));
}
