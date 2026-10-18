// SPDX-License-Identifier: MIT
#pragma once
#include <array>
#include <godot_cpp/classes/animation_player.hpp>
#include <godot_cpp/classes/mesh_instance3d.hpp>
#include <godot_cpp/classes/packed_scene.hpp>
#include <godot_cpp/classes/skeleton3d.hpp>
#include <vector>
namespace godot {
enum class CharacterAsset {
    Sabrina,
    Clownfish,
    BlueTang,
    Butterflyfish,
    Bannerfish,
    Parrotfish,
    BlueWrasse,
    FireGoby,
    RoyalTang,
    YellowTang,
    EmperorAngelfish,
    CoralAnthias,
    Cardinalfish,
    Jellyfish,
    Count
};
using CharacterLibrary = std::array<Ref<PackedScene>, static_cast<int>(CharacterAsset::Count)>;
// Presentation components only. Godot owns these nodes; simulation owns all behavior.
struct BonePose {
    int bone;
    Quaternion rotation;
};
struct RigVisual {
    Node3D *root = nullptr;
    Skeleton3D *skeleton = nullptr;
    AnimationPlayer *player = nullptr;
    std::vector<MeshInstance3D *> meshes;
    StringName clip;
    // Pose before native corrections, including bones with optimized-out animation tracks.
    std::vector<BonePose> unmodified_rotations;
};
bool load_character_library(CharacterLibrary &library);
RigVisual instantiate_rig(Node3D *parent, const Ref<PackedScene> &scene);
void dispose_rig(RigVisual &rig);
void animate_rig(RigVisual &rig, const StringName &clip, double dt, float speed = 1);
Vector3 bone_attachment(const RigVisual &rig, int bone, Vector3 offset = {});
CharacterAsset fish_asset(int species, int variant = 0);
} // namespace godot
