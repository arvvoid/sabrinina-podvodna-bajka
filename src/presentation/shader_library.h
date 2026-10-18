// SPDX-License-Identifier: Apache-2.0
#pragma once
#include <godot_cpp/classes/shader.hpp>
#include <godot_cpp/classes/shader_material.hpp>
namespace godot {
enum class ReefShader {
    Backdrop,
    SunShafts,
    ReefCaustics,
    WaterVolume,
    CrystalGlow,
    DistantLife,
    SongRing,
    WaterWave,
    CrystalRelic,
    CrystalWake,
    CrystalShield,
    Count
};

// ResourceLoader caches shader programs by path; each call creates an independent
// material so per-character parameters never leak into another material.
Ref<Shader> load_shader(ReefShader asset);
Ref<ShaderMaterial> load_shader_material(ReefShader asset);
} // namespace godot
