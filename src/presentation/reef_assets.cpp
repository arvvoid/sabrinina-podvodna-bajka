// SPDX-License-Identifier: Apache-2.0
#include "reef_assets.h"
#include "shader_library.h"
#include <godot_cpp/classes/base_material3d.hpp>
#include <godot_cpp/classes/mesh.hpp>
#include <godot_cpp/classes/mesh_instance3d.hpp>
#include <godot_cpp/classes/packed_scene.hpp>
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/classes/texture2d.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
namespace godot {
Node3D *instantiate_reef(Node3D *parent, std::vector<Ref<ShaderMaterial>> &materials) {
    Ref<PackedScene> scene =
        ResourceLoader::get_singleton()->load("res://assets/environments/reef_ruins.glb", "PackedScene");
    if (scene.is_null()) {
        UtilityFunctions::push_error("Required reef environment failed to load.");
        return nullptr;
    }
    Node *instance = scene->instantiate();
    auto *root = Object::cast_to<Node3D>(instance);
    if (!root) {
        memdelete(instance);
        return nullptr;
    }
    parent->add_child(root);
    auto bind = [&](auto &&self, Node *node) -> void {
        if (auto *mesh = Object::cast_to<MeshInstance3D>(node)) {
            auto geometry = mesh->get_mesh();
            for (int i = 0; i < geometry->get_surface_count(); ++i) {
                Ref<BaseMaterial3D> source = geometry->surface_get_material(i);
                if (source.is_null())
                    continue;
                auto material = load_shader_material(ReefShader::ReefCaustics);
                material->set_shader_parameter("tint", source->get_albedo());
                material->set_shader_parameter("textured", true);
                material->set_shader_parameter("albedo_texture",
                                               source->get_texture(BaseMaterial3D::TEXTURE_ALBEDO));
                material->set_shader_parameter("normal_texture",
                                               source->get_texture(BaseMaterial3D::TEXTURE_NORMAL));
                material->set_shader_parameter("roughness_texture",
                                               source->get_texture(BaseMaterial3D::TEXTURE_ROUGHNESS));
                mesh->set_surface_override_material(i, material);
                materials.push_back(material);
            }
        }
        for (int i = 0; i < node->get_child_count(); ++i)
            self(self, node->get_child(i));
    };
    bind(bind, root);
    return root;
}
} // namespace godot
