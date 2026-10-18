// SPDX-License-Identifier: Apache-2.0
#include "reef_game.h"
#include "shader_library.h"
#include "simulation/contacts.h"
#include <algorithm>
#include <cmath>
#include <godot_cpp/classes/packed_scene.hpp>
#include <godot_cpp/classes/quad_mesh.hpp>
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
using namespace godot;
void ReefGame::build_crystal() {
    crystal = memnew(Node3D);
    add_child(crystal);
    Ref<PackedScene> source =
        ResourceLoader::get_singleton()->load("res://assets/props/ocean_crystal.glb", "PackedScene");
    if (source.is_valid()) {
        crystal_model = Object::cast_to<Node3D>(source->instantiate());
        crystal->add_child(crystal_model);
        auto bind = [&](auto &&self, Node *node) -> void {
            if (auto *mesh = Object::cast_to<MeshInstance3D>(node)) {
                for (int i = 0; i < mesh->get_mesh()->get_surface_count(); ++i) {
                    Ref<BaseMaterial3D> original = mesh->get_mesh()->surface_get_material(i);
                    if (original.is_null())
                        continue;
                    auto m = load_shader_material(ReefShader::CrystalRelic);
                    m->set_shader_parameter("albedo_texture",
                                            original->get_texture(BaseMaterial3D::TEXTURE_ALBEDO));
                    m->set_shader_parameter("normal_texture",
                                            original->get_texture(BaseMaterial3D::TEXTURE_NORMAL));
                    mesh->set_surface_override_material(i, m);
                    crystal_materials.push_back(m);
                }
            }
            for (int i = 0; i < node->get_child_count(); ++i)
                self(self, node->get_child(i));
        };
        bind(bind, crystal_model);
    } else
        UtilityFunctions::push_error("Ocean crystal asset could not be loaded");
    auto quad = [&](Node3D *parent, ReefShader shader, float size) {
        auto *node = memnew(MeshInstance3D);
        Ref<QuadMesh> mesh;
        mesh.instantiate();
        mesh->set_size({size, size});
        node->set_mesh(mesh);
        node->set_material_override(load_shader_material(shader));
        node->set_cast_shadows_setting(GeometryInstance3D::SHADOW_CASTING_SETTING_OFF);
        parent->add_child(node);
        return node;
    };
    quad(crystal, ReefShader::CrystalGlow, 2.4)->set_position({0, 0, -.15});
    crystal_wake = quad(this, ReefShader::CrystalWake, 1);
    crystal_ring = memnew(Node3D);
    add_child(crystal_ring);
    ring(crystal_ring, {}, .65, Color(.51, .91, 1));
    magic_light = memnew(OmniLight3D);
    add_child(magic_light);
    magic_light->set_color(Color(.32, .82, 1));
    magic_light->set_param(Light3D::PARAM_RANGE, 5);
    magic_light->set_shadow(false);
    spark_root = memnew(Node3D);
    add_child(spark_root);
    for (int i = 0; i < 24; ++i)
        sparks.push_back(ellipsoid(spark_root, {}, {.025, .025, .025},
                                   i % 3 ? Color(.40, .95, 1) : Color(1, .86, .48), 1));
    for (int i = 0; i < 24; ++i)
        trail_nodes.push_back(
            ellipsoid(this, {}, {1, 1, 1}, i % 3 ? Color(.25, .80, 1) : Color(.9, .9, .6), .6));
}
void ReefGame::update_crystal(float dt) {
    const auto &c = sim.crystal;
    bool flying = c.magic.state == reef::Spell::Flying, resting = c.magic.state == reef::Spell::Resting;
    bool charging = c.magic.state == reef::Spell::Charging, burst = c.magic.state == reef::Spell::Burst;
    bool ready = sim.sabrina.cooldown <= 0;
    float motion = reduced_motion ? .25f : 1.f;
    float orbit = sim.session.time * .9f * motion;
    float depth = c.magic.depth;
    Vector3 cp(c.motion.pos.x, c.motion.pos.y, depth);
    if (charging) {
        float gather = std::clamp(c.magic.time / .4f, 0.f, 1.f);
        gather = gather * gather * (3 - 2 * gather);
        // A fixed offset in the palm's bone frame follows the actual skinned hand,
        // including turns, pose blending and the tiny visual idle bob.
        Vector3 palm = bone_attachment(sabrina_rig, hand_bone_right, {0, reef::sabrina_palm_offset(), 0});
        Vector3 origin(sim.sabrina.motion.pos.x + c.magic.charge_offset.x,
                       sim.sabrina.motion.pos.y + c.magic.charge_offset.y, c.magic.charge_depth);
        cp = origin.lerp(palm, gather);
    }
    crystal->set_position(cp);
    crystal->set_visible(!burst && (!resting || ready));
    float size = charging ? .72f : flying ? .85f : .72f;
    crystal->set_scale({size, size, size});
    if (crystal_model) {
        float angle = flying ? std::atan2(c.motion.direction.y, c.motion.direction.x) - 1.15f
                             : .16f * std::sin(orbit * 2);
        // Keep the textured broad face readable; a small tumble shows its facets.
        if (charging) {
            // Held objects do not tumble independently of the wrist.
            crystal_model->set_basis(Basis(Vector3(0, 1, 0), sim.sabrina.swimming.yaw) *
                                     Basis(Vector3(0, 0, 1), sim.sabrina.swimming.pitch - .65f));
        } else
            crystal_model->set_rotation({.12f * std::sin(orbit), .35f * std::sin(orbit * 1.3f), angle});
    }
    for (auto &m : crystal_materials) {
        m->set_shader_parameter("phase", sim.session.time * motion);
        m->set_shader_parameter("charge", charging ? c.magic.time / 1.1f : flying ? .6f : 0.f);
    }
    crystal_ring->set_visible(charging || burst);
    crystal_ring->set_position(cp);
    float radius = burst ? 1 + c.magic.time * 6 : .8f + c.magic.time;
    crystal_ring->set_scale({radius, radius, radius});
    magic_light->set_visible(quality > 0 && (!resting || ready));
    magic_light->set_position(cp);
    magic_light->set_param(Light3D::PARAM_ENERGY, charging ? 1.4 : burst ? .8 : .45);
    spark_root->set_visible(!resting || ready);
    spark_root->set_position(cp);
    for (size_t i = 0; i < sparks.size(); ++i) {
        float a = i * 2.39996f + sim.session.time * 2 * motion;
        float r = burst ? c.magic.time * 4 : .5f + .25f * std::sin(a * 1.7f);
        sparks[i]->set_position({std::cos(a) * r, std::sin(a) * r, std::sin(a * 2) * .3f});
        float s = burst ? std::max(0.f, 1 - c.magic.time / .7f) : .65f + .35f * std::sin(a);
        sparks[i]->set_scale({s * .035f, s * .035f, s * .035f});
    }
    if (flying) {
        if (!crystal_was_flying) {
            trail.fill(cp);
            crystal_previous = cp;
            crystal_trail_time = 0;
        }
        // Fixed-time history avoids a shorter tail at high frame rates and an origin streak at launch.
        crystal_trail_time += dt;
        while (crystal_trail_time >= 1.f / 60) {
            crystal_trail_time -= 1.f / 60;
            for (int i = 23; i > 0; --i)
                trail[i] = trail[i - 1];
            trail[0] =
                crystal_previous.lerp(cp, dt > 0 ? std::clamp(1 - crystal_trail_time / dt, 0.f, 1.f) : 1.f);
        }
    }
    for (int i = 0; i < 24; ++i) {
        bool show = flying || (resting && ready);
        trail_nodes[i]->set_visible(show);
        float taper = 1 - float(i) / 24;
        Vector3 pos = trail[i];
        if (resting) {
            auto pose = reef::crystal_orbit(sim, i * .05f);
            pos = {sim.sabrina.motion.pos.x + pose.offset.x, sim.sabrina.motion.pos.y + pose.offset.y,
                   pose.depth};
        }
        trail_nodes[i]->set_position(pos);
        float s = (flying ? .075f : .028f) * taper;
        trail_nodes[i]->set_scale({s, s, s});
    }
    crystal_wake->set_visible(flying);
    if (flying) {
        Vector3 direction(c.motion.direction.x, c.motion.direction.y, 0);
        crystal_wake->set_position(cp - direction * 1.65f);
        crystal_wake->set_rotation({0, 0, std::atan2(direction.y, direction.x)});
        crystal_wake->set_scale({3.8, 1.35, 1});
        Ref<ShaderMaterial> m = crystal_wake->get_material_override();
        m->set_shader_parameter("phase", sim.session.time * motion);
        m->set_shader_parameter("visibility", std::min(1.f, c.magic.time * 5));
    }
    crystal_previous = cp;
    crystal_was_flying = flying;
}
