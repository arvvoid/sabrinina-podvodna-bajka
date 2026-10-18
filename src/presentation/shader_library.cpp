// SPDX-License-Identifier: Apache-2.0
#include "shader_library.h"
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
namespace godot {
namespace {
constexpr const char *paths[] = {
    "res://shaders/environment/backdrop.gdshader",      "res://shaders/environment/sun_shafts.gdshader",
    "res://shaders/environment/reef_caustics.gdshader", "res://shaders/environment/water_volume.gdshader",
    "res://shaders/effects/crystal_glow.gdshader",      "res://shaders/environment/distant_life.gdshader",
    "res://shaders/effects/song_ring.gdshader",         "res://shaders/effects/water_wave.gdshader",
    "res://shaders/effects/crystal_relic.gdshader",     "res://shaders/effects/crystal_wake.gdshader",
    "res://shaders/effects/crystal_shield.gdshader",
};
static_assert(sizeof(paths) / sizeof(paths[0]) == static_cast<int>(ReefShader::Count));
} // namespace
Ref<Shader> load_shader(ReefShader asset) {
    const int index = static_cast<int>(asset);
    if (index < 0 || index >= static_cast<int>(ReefShader::Count))
        return {};
    const char *path = paths[index];
    Ref<Shader> shader = ResourceLoader::get_singleton()->load(String::utf8(path), "Shader");
    if (shader.is_null())
        UtilityFunctions::push_error("Required reef shader could not be loaded: ", path);
    return shader;
}
Ref<ShaderMaterial> load_shader_material(ReefShader asset) {
    Ref<ShaderMaterial> material;
    material.instantiate();
    material->set_shader(load_shader(asset));
    return material;
}
} // namespace godot
