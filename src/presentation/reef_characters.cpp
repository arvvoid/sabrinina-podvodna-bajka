// SPDX-License-Identifier: MIT
#include "reef_game.h"
#include "shader_library.h"
#include "simulation/contacts.h"
#include "simulation/sabrina_pose.h"
#include <algorithm>
#include <cmath>
#include <godot_cpp/classes/animation.hpp>
#include <godot_cpp/classes/mesh.hpp>
#include <godot_cpp/classes/quad_mesh.hpp>
using namespace godot;
namespace {
constexpr float PI = 3.14159265359f;
}
void ReefGame::build_sabrina() {
    mermaid = memnew(Node3D);
    add_child(mermaid);
    sabrina_rig = instantiate_rig(mermaid, character_library[static_cast<int>(CharacterAsset::Sabrina)]);
    sabrina_rig.root->set_scale({.88, .88, .88});
    hand_bone_left = sabrina_rig.skeleton->find_bone("Hand_L");
    hand_bone_right = sabrina_rig.skeleton->find_bone("Hand_R");
    animate_rig(sabrina_rig, "Idle", 0);
    wave_visual = memnew(MeshInstance3D);
    Ref<QuadMesh> front;
    front.instantiate();
    front->set_size({1, 1});
    wave_visual->set_mesh(front);
    wave_visual->set_material_override(load_shader_material(ReefShader::WaterWave));
    wave_visual->set_cast_shadows_setting(GeometryInstance3D::SHADOW_CASTING_SETTING_OFF);
    wave_visual->hide();
    add_child(wave_visual);
    for (int i = 0; i < 3; ++i) {
        auto *n = memnew(Node3D);
        add_child(n);
        auto *wave = ring(n, {}, 1, Color(1, 1, 1));
        wave->set_material_override(load_shader_material(ReefShader::SongRing));
        song_rings.push_back(n);
    }
    for (int i = 0; i < 7; ++i) {
        auto *n = memnew(Node3D);
        add_child(n);
        Color c = i % 2 ? Color(.99, .67, .91) : Color(.46, .94, 1);
        ellipsoid(n, {}, {.075, .05, .025}, c, .7);
        branch(n, {.065, 0, 0}, {.065, .25, 0}, .017, c);
        branch(n, {.065, .25, 0}, {.18, .20, 0}, .02, c);
        song_notes.push_back(n);
    }
}
void ReefGame::build_detailed_fish() {
    for (auto &fish : fish_visuals) {
        fish.root = memnew(Node3D);
        add_child(fish.root);
        fish.root->hide();
        fish.shield = memnew(MeshInstance3D);
        Ref<QuadMesh> bubble;
        bubble.instantiate();
        bubble->set_size({2.2, 1.8});
        fish.shield->set_mesh(bubble);
        fish.shield->set_material_override(load_shader_material(ReefShader::CrystalShield));
        fish.shield->set_cast_shadows_setting(GeometryInstance3D::SHADOW_CASTING_SETTING_OFF);
        add_child(fish.shield);
        fish.shield->hide();
    }
}
void ReefGame::build_jellyfish() {
    const Color tints[3] = {Color(.76, 1.18, 1.13), Color(1.48, 1.12, .84), Color(.60, 1.65, 1.18)};
    for (int k = 0; k < reef::jelly_count; ++k) {
        auto &j = jelly_visuals[k];
        j.root = memnew(Node3D);
        add_child(j.root);
        j.rig = instantiate_rig(j.root, character_library[static_cast<int>(CharacterAsset::Jellyfish)]);
        for (auto *mesh : j.rig.meshes) {
            auto resource = mesh->get_mesh();
            for (int surface = 0; surface < resource->get_surface_count(); ++surface) {
                Ref<StandardMaterial3D> original = resource->surface_get_material(surface);
                if (original.is_valid() && original->get_name() == "Jelly_Skin") {
                    Ref<StandardMaterial3D> material = original->duplicate();
                    material->set_albedo(tints[k]);
                    mesh->set_surface_override_material(surface, material);
                }
            }
        }
        animate_rig(j.rig, "Pulse", 0);
        j.rig.player->seek(k * .47, true);
    }
}
void ReefGame::update_characters(float dt, float motion) {
    float t = sim.session.time;
    scenery_time += dt;
    bool hurt = sim.sabrina.vitality.hurt_flash > 0 || sim.sabrina.vitality.stunned > 0;
    if (hurt != siren_red) {
        siren_red = hurt;
        for (auto *mesh : sabrina_rig.meshes)
            mesh->set_material_overlay(hurt ? damage_material : Ref<StandardMaterial3D>());
    }
    float effort = std::min(1.f, sim.sabrina.motion.velocity.length() / 7.2f);
    mermaid->set_position(
        {sim.sabrina.motion.pos.x, sim.sabrina.motion.pos.y + .07f * std::sin(t * 1.8f) * motion, .5});
    // The authored swim rig points +X. Share its smoothed orientation with
    // the contact shapes, including upright rest and both left/right headings.
    mermaid->set_basis(Basis(Vector3(0, 1, 0), sim.sabrina.swimming.yaw) *
                       Basis(Vector3(0, 0, 1), sim.sabrina.swimming.pitch));
    float pulse = sim.sabrina.vitality.invulnerable > 0 ? .94f + .06f * std::sin(t * 5) : 1;
    mermaid->set_scale({pulse, pulse, pulse});
    constexpr const char *clips[] = {"Idle", "Swim", "Turn_Left", "Turn_Right",
                                     "Sing", "Wave", "Hurt",      "Throw"};
    StringName clip = clips[static_cast<int>(reef::sabrina_pose(sim.sabrina))];
    animate_rig(sabrina_rig, clip, dt * (reduced_motion ? .25 : 1),
                clip == StringName("Swim") ? .75f + effort * .5f : 1.f);
    if (dt > 0) {
        // One phase clock drives the displayed tail and its native contact chain.
        auto animation = sabrina_rig.player->get_animation(clip);
        sabrina_rig.player->seek(reef::sabrina_pose_phase(sim.sabrina) * animation->get_length(), true);
        // Keep her gaze near the swimming direction while the torso rises to rest.
        // Apply after clip evaluation; never reapply to a frozen paused pose.
        auto bend = [&](const char *name, float angle, bool posed_axis = false) {
            int bone = sabrina_rig.skeleton->find_bone(name);
            auto transform = posed_axis ? sabrina_rig.skeleton->get_bone_global_pose(bone)
                                        : sabrina_rig.skeleton->get_bone_global_rest(bone);
            Vector3 axis = transform.basis.inverse().xform(Vector3(0, 0, 1)).normalized();
            auto rotation = sabrina_rig.skeleton->get_bone_pose_rotation(bone);
            sabrina_rig.unmodified_rotations.push_back({bone, rotation});
            sabrina_rig.skeleton->set_bone_pose_rotation(bone, rotation * Quaternion(axis, angle));
        };
        float rest = std::clamp((sim.sabrina.swimming.pitch - .2f) / .45f, 0.f, 1.f) *
                     (1.f - std::clamp(effort * 2.f, 0.f, 1.f));
        bend("Tail_1", .30f * rest);
        bend("Tail_2", .10f * rest);
        // Limit gaze at every speed, including steep ascents and descents. Apply
        // about the posed world axis so neck/head corrections do not add a twist.
        // A dive follows the body; trying to keep the face horizontal made the
        // neck look hyperextended. Reserve the larger correction for upright rest.
        float gaze = std::clamp(-sim.sabrina.swimming.pitch * .55f, -.65f, .06f);
        bend("Neck", gaze * .35f, true);
        bend("Head", gaze * .65f, true);
        for (const char *hair : {"Hair_Back_1", "Hair_Left_1", "Hair_Right_1"})
            bend(hair, -gaze, true);
    }
    const auto &wave = sim.sabrina.wave;
    wave_visual->set_visible(wave.active);
    if (wave.active) {
        float travel = std::min(5.5f, wave.time * 8.f);
        auto p = wave.origin + wave.direction * travel;
        wave_visual->set_position({p.x, p.y, 1.6f});
        wave_visual->set_rotation({0, 0, std::atan2(wave.direction.y, wave.direction.x)});
        wave_visual->set_scale({1.2f, 2 * (.65f + .38f * travel), 1});
        Ref<ShaderMaterial> material = wave_visual->get_material_override();
        material->set_shader_parameter("progress", wave.time / .75f);
    }
    for (int k = 0; k < reef::jelly_count; ++k) {
        const auto &j = sim.jelly[k];
        auto &v = jelly_visuals[k];
        v.root->set_visible(j.behaviour.state != reef::JellyState::Hidden);
        if (j.behaviour.state == reef::JellyState::Hidden)
            continue;
        v.root->set_position(
            {j.motion.pos.x, j.motion.pos.y + .10f * std::sin(t * 2 + j.behaviour.phase) * motion, .2});
        float s = reef::jelly_scale(j);
        float recoil = std::sin(std::clamp(j.behaviour.repel_flash / .65f, 0.f, 1.f) * PI);
        v.root->set_scale({s * (1 + .12f * recoil), s * (1 - .15f * recoil), s});
        v.root->set_rotation({0, 0, .035f * std::sin(t * 1.7f + j.behaviour.phase) * motion});
        animate_rig(v.rig,
                    j.behaviour.eat_wait > 0                         ? "Nibble"
                    : j.behaviour.state == reef::JellyState::Leaving ? "Drift"
                                                                     : "Pulse",
                    dt * (reduced_motion ? .25 : 1));
    }
    for (size_t i = 0; i < song_rings.size(); ++i) {
        auto *n = song_rings[i];
        float u = std::fmod(t * .40f * motion + float(i) / 3, 1.f);
        n->set_visible(sim.sabrina.song.active && sim.sabrina.song.strength > .05f);
        n->set_position({sim.sabrina.motion.pos.x, sim.sabrina.motion.pos.y, -.2f});
        float size = (i == 0 ? 1.f : (.2f + u * .8f)) * reef::song_radius(sim);
        n->set_scale({size, size, size});
        auto *wave = Object::cast_to<MeshInstance3D>(n->get_child(0));
        Ref<ShaderMaterial> m = wave->get_material_override();
        m->set_shader_parameter("opacity",
                                (i == 0 ? .16f : .32f * std::sin(u * PI)) * sim.sabrina.song.strength);
    }
    for (size_t i = 0; i < song_notes.size(); ++i) {
        auto *n = song_notes[i];
        float u = std::fmod(t * .28f + i / 7.f, 1.f), a = i * 2.4f + t * .30f;
        n->set_visible(sim.sabrina.song.active && sim.sabrina.song.strength > .05f);
        n->set_position({sim.sabrina.motion.pos.x + std::sin(a) * (.2f + u * .45f) * reef::song_radius(sim),
                         sim.sabrina.motion.pos.y + .35f + u * reef::song_radius(sim) * .65f, 1.2f});
        float scale = std::sin(u * PI) * sim.sabrina.song.strength;
        n->set_scale({scale, scale, scale});
        n->set_rotation({0, 0, std::sin(a) * .2f});
    }
}
