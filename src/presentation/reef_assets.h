// SPDX-License-Identifier: Apache-2.0
#pragma once
#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/classes/shader_material.hpp>
#include <vector>
namespace godot {
// Imported scenery is presentation data; it never owns gameplay collision or movement.
Node3D *instantiate_reef(Node3D *parent, std::vector<Ref<ShaderMaterial>> &materials);
} // namespace godot
