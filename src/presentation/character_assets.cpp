// SPDX-License-Identifier: Apache-2.0
#include "character_assets.h"
#include <godot_cpp/classes/animation.hpp>
#include <godot_cpp/classes/mesh.hpp>
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/classes/standard_material3d.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
namespace godot {
namespace {
constexpr const char *paths[] = {
    "res://assets/models/sabrina.glb",           "res://assets/models/clownfish.glb",
    "res://assets/models/blue_tang.glb",         "res://assets/models/butterflyfish.glb",
    "res://assets/models/bannerfish.glb",        "res://assets/models/parrotfish.glb",
    "res://assets/models/blue_wrasse.glb",       "res://assets/models/fire_goby.glb",
    "res://assets/models/royal_tang.glb",        "res://assets/models/yellow_tang.glb",
    "res://assets/models/emperor_angelfish.glb", "res://assets/models/coral_anthias.glb",
    "res://assets/models/cardinalfish.glb",      "res://assets/models/jellyfish.glb"};
static_assert(sizeof(paths) / sizeof(paths[0]) == static_cast<int>(CharacterAsset::Count));
void collect(Node *node, RigVisual &rig) {
    if (auto *skeleton = Object::cast_to<Skeleton3D>(node))
        rig.skeleton = skeleton;
    if (auto *player = Object::cast_to<AnimationPlayer>(node))
        rig.player = player;
    if (auto *mesh = Object::cast_to<MeshInstance3D>(node))
        rig.meshes.push_back(mesh);
    for (int i = 0; i < node->get_child_count(); ++i)
        collect(node->get_child(i), rig);
}
} // namespace
bool load_character_library(CharacterLibrary &library) {
    for (int i = 0; i < static_cast<int>(CharacterAsset::Count); ++i) {
        library[i] = ResourceLoader::get_singleton()->load(paths[i], "PackedScene");
        if (library[i].is_null()) {
            UtilityFunctions::push_error("Required character model failed to load: ", paths[i]);
            return false;
        }
        // Validate the resource contract before building any of the live actors.
        Node *probe = library[i]->instantiate();
        if (!probe)
            return false;
        RigVisual rig;
        collect(probe, rig);
        bool valid = Object::cast_to<Node3D>(probe) && rig.skeleton && rig.player && !rig.meshes.empty();
        if (valid) {
            for (auto *mesh : rig.meshes) {
                auto geometry = mesh->get_mesh();
                for (int surface = 0; surface < geometry->get_surface_count(); ++surface) {
                    Ref<StandardMaterial3D> material = geometry->surface_get_material(surface);
                    if (material.is_valid()) {
                        material->set_flag(BaseMaterial3D::FLAG_ALBEDO_FROM_VERTEX_COLOR, true);
                        if (i == static_cast<int>(CharacterAsset::Sabrina)) {
                            // This source shares one material across skin, hair and tail.
                            // Broad, restrained highlights keep pale features readable.
                            material->set_metallic(.04f);
                            material->set_roughness(.62f);
                            material->set_specular(.28f);
                        }
                    }
                }
            }

            const auto clips = rig.player->get_animation_list();
            valid = clips.size() > 0;
            for (int k = 0; k < clips.size(); ++k) {
                auto animation = rig.player->get_animation(clips[k]);
                if (clips[k] != "RESET")
                    animation->set_loop_mode(Animation::LOOP_LINEAR);
            }
        }
        memdelete(probe);
        if (!valid) {
            UtilityFunctions::push_error("Character is missing its mesh, skeleton or animation: ", paths[i]);
            return false;
        }
    }
    return true;
}
RigVisual instantiate_rig(Node3D *parent, const Ref<PackedScene> &scene) {
    RigVisual rig;
    rig.root = Object::cast_to<Node3D>(scene->instantiate());
    parent->add_child(rig.root);
    collect(rig.root, rig);
    // Advance explicitly so pausing/reduced motion and pooled actors remain synchronized.
    rig.player->set_callback_mode_process(AnimationMixer::ANIMATION_CALLBACK_MODE_PROCESS_MANUAL);
    return rig;
}
void dispose_rig(RigVisual &rig) {
    if (rig.root) {
        rig.root->get_parent()->remove_child(rig.root);
        rig.root->queue_free();
    }
    rig = {};
}
void animate_rig(RigVisual &rig, const StringName &clip, double dt, float speed) {
    if (!rig.player)
        return;
    bool changed = rig.clip != clip;
    if (changed || dt > 0) {
        // Constant tracks may be removed during import. Animation playback cannot
        // be relied on to erase our previous additive pose on those bones.
        for (const auto &pose : rig.unmodified_rotations)
            rig.skeleton->set_bone_pose_rotation(pose.bone, pose.rotation);
        rig.unmodified_rotations.clear();
    }
    if (changed) {
        rig.player->play(clip, .22);
        rig.clip = clip;
    }
    rig.player->set_speed_scale(speed);
    if (changed || dt > 0)
        rig.player->advance(dt);
}
Vector3 bone_attachment(const RigVisual &rig, int bone, Vector3 offset) {
    if (!rig.skeleton || bone < 0)
        return rig.root->get_global_position();
    return rig.skeleton->to_global(rig.skeleton->get_bone_global_pose(bone).xform(offset));
}
CharacterAsset fish_asset(int species, int variant) {
    return static_cast<CharacterAsset>(static_cast<int>(CharacterAsset::Clownfish) + species +
                                       (variant % 2) * 6);
}
} // namespace godot
