// SPDX-License-Identifier: MIT
#include "presentation/shader_library.h"
#include "reef_assets.h"
#include "reef_game.h"
#include <algorithm>
#include <cmath>
#include <godot_cpp/classes/array_mesh.hpp>
#include <godot_cpp/classes/cylinder_mesh.hpp>
#include <godot_cpp/classes/environment.hpp>
#include <godot_cpp/classes/multi_mesh.hpp>
#include <godot_cpp/classes/plane_mesh.hpp>
#include <godot_cpp/classes/quad_mesh.hpp>
#include <godot_cpp/classes/rendering_server.hpp>
#include <godot_cpp/classes/shader.hpp>
#include <godot_cpp/classes/torus_mesh.hpp>
#include <godot_cpp/classes/viewport.hpp>
#include <godot_cpp/classes/world_environment.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
using namespace godot;
namespace {
constexpr float PI = 3.14159265f;
float rnd(float s) {
    return .5f + .5f * std::sin(s * 17.137f + s * s * .71f);
}
} // namespace
Ref<StandardMaterial3D> ReefGame::material(Color c, float glow) {
    Ref<StandardMaterial3D> m;
    m.instantiate();
    m->set_albedo(c);
    m->set_roughness(.68);
    if (glow > 0) {
        m->set_feature(BaseMaterial3D::FEATURE_EMISSION, true);
        m->set_emission(c);
        m->set_emission_energy_multiplier(glow);
    }
    return m;
}
MeshInstance3D *ReefGame::ellipsoid(Node3D *parent, Vector3 p, Vector3 scale, Color c, float glow) {
    auto *n = memnew(MeshInstance3D);
    n->set_mesh(sphere_mesh);
    n->set_material_override(material(c, glow));
    parent->add_child(n);
    n->set_position(p);
    n->set_scale(scale);
    return n;
}
MeshInstance3D *ReefGame::cone(Node3D *parent, Vector3 p, float bottom, float top, float height, Color c) {
    auto *n = memnew(MeshInstance3D);
    Ref<CylinderMesh> m;
    m.instantiate();
    m->set_bottom_radius(bottom);
    m->set_top_radius(top);
    m->set_height(height);
    m->set_radial_segments(10);
    n->set_mesh(m);
    n->set_material_override(material(c));
    parent->add_child(n);
    n->set_position(p);
    return n;
}
MeshInstance3D *ReefGame::ring(Node3D *parent, Vector3 p, float radius, Color c) {
    auto *n = memnew(MeshInstance3D);
    Ref<TorusMesh> m;
    m.instantiate();
    m->set_inner_radius(radius - .025);
    m->set_outer_radius(radius + .025);
    m->set_rings(36);
    m->set_ring_segments(6);
    n->set_mesh(m);
    n->set_material_override(material(c, .6));
    parent->add_child(n);
    n->set_position(p);
    n->set_rotation_degrees(Vector3(90, 0, 0));
    return n;
}
MeshInstance3D *ReefGame::fin(Node3D *parent, Vector3 a, Vector3 b, Vector3 c, Color color) {
    PackedVector3Array vertices;
    vertices.push_back(a);
    vertices.push_back(b);
    vertices.push_back(c);
    vertices.push_back(c);
    vertices.push_back(b);
    vertices.push_back(a);
    Vector3 normal = (b - a).cross(c - a).normalized();
    PackedVector3Array normals;
    for (int i = 0; i < 6; ++i)
        normals.push_back(i < 3 ? normal : -normal);
    Array arrays;
    arrays.resize(Mesh::ARRAY_MAX);
    arrays[Mesh::ARRAY_VERTEX] = vertices;
    arrays[Mesh::ARRAY_NORMAL] = normals;
    Ref<ArrayMesh> mesh;
    mesh.instantiate();
    mesh->add_surface_from_arrays(Mesh::PRIMITIVE_TRIANGLES, arrays);
    auto *n = memnew(MeshInstance3D);
    n->set_mesh(mesh);
    n->set_material_override(material(color));
    parent->add_child(n);
    return n;
}
void ReefGame::branch(Node3D *parent, Vector3 a, Vector3 b, float radius, Color color) {
    Vector3 along = (b - a).normalized(), cross = Vector3(-along.y, along.x, 0).normalized();
    if (cross.length_squared() < .1f)
        cross = Vector3(1, 0, 0);
    Vector3 depth = along.cross(cross).normalized();
    PackedVector3Array vertices, normals;
    PackedInt32Array indices;
    const int rings = 8, sides = 8;
    float bend = (b - a).length() * .09f;
    for (int j = 0; j <= rings; ++j)
        for (int k = 0; k <= sides; ++k) {
            float u = float(j) / rings, angle = k * PI * 2 / sides;
            Vector3 normal = cross * std::cos(angle) + depth * std::sin(angle);
            vertices.push_back(a.lerp(b, u) + cross * (std::sin(u * PI) * bend) +
                               normal * (radius * (1 - .78f * u)));
            normals.push_back(normal);
            if (j < rings && k < sides) {
                int v = j * (sides + 1) + k, w = v + sides + 1;
                for (int n : {v, w, v + 1, v + 1, w, w + 1})
                    indices.push_back(n);
            }
        }
    Array arrays;
    arrays.resize(Mesh::ARRAY_MAX);
    arrays[Mesh::ARRAY_VERTEX] = vertices;
    arrays[Mesh::ARRAY_NORMAL] = normals;
    arrays[Mesh::ARRAY_INDEX] = indices;
    Ref<ArrayMesh> mesh;
    mesh.instantiate();
    mesh->add_surface_from_arrays(Mesh::PRIMITIVE_TRIANGLES, arrays);
    auto *n = memnew(MeshInstance3D);
    parent->add_child(n);
    n->set_mesh(mesh);
    n->set_material_override(material(color));
}
void ReefGame::build_world() {
    if (sphere_mesh.is_null()) {
        sphere_mesh.instantiate();
        sphere_mesh->set_radius(1);
        sphere_mesh->set_height(2);
        sphere_mesh->set_radial_segments(20);
        sphere_mesh->set_rings(10);
    }

    auto *world = memnew(WorldEnvironment);
    Ref<Environment> env;
    env.instantiate();
    env->set_background(Environment::BG_COLOR);
    env->set_bg_color(Color(.015, .16, .28));
    env->set_ambient_source(Environment::AMBIENT_SOURCE_COLOR);
    env->set_ambient_light_color(Color(.65, .88, .91));
    env->set_ambient_light_energy(.55);
    // Compress bright skin/hair highlights instead of clipping them to white.
    env->set_tonemapper(Environment::TONE_MAPPER_FILMIC);
    env->set_tonemap_exposure(1.0);
    env->set_tonemap_white(4.0);
    env->set_fog_enabled(true);
    env->set_fog_light_color(Color(.025, .38, .48));
    env->set_fog_density(.0035);
    world->set_environment(env);
    add_child(world);
    sun = memnew(DirectionalLight3D);
    add_child(sun);
    sun->set_rotation_degrees(Vector3(-38, -25, -20));
    sun->set_color(Color(1, .96, .78));
    sun->set_param(Light3D::PARAM_ENERGY, 1.15);
    sun->set_param(Light3D::PARAM_SPECULAR, .35);
    sun->set_shadow(true);
    auto *fill = memnew(DirectionalLight3D);
    add_child(fill);
    fill->set_rotation_degrees(Vector3(25, 160, 0));
    fill->set_color(Color(.22, .72, 1));
    fill->set_param(Light3D::PARAM_ENERGY, .25);
    fill->set_param(Light3D::PARAM_SPECULAR, .15);
    camera = memnew(Camera3D);
    add_child(camera);
    camera->set_position(Vector3(0, 9.2, 36));
    camera->look_at(Vector3(0, 9.2, 0));
    camera->set_projection(Camera3D::PROJECTION_ORTHOGONAL);
    camera->set_size(19.5);
    camera->set_current(true);
    camera->set_far(150);
    auto backdrop_shader = load_shader(ReefShader::Backdrop);
    auto *back = memnew(MeshInstance3D);
    Ref<QuadMesh> quad;
    quad.instantiate();
    quad->set_size(Vector2(40, 24));
    back->set_mesh(quad);
    Ref<ShaderMaterial> bm;
    bm.instantiate();
    bm->set_shader(backdrop_shader);
    back->set_material_override(bm);
    add_child(back);
    back->set_position(Vector3(0, 10, -25));
    reef_environment = instantiate_reef(this, reef_materials);
    if (!reef_environment) {
        assets_failed = true;
        return;
    }
    // Sun shafts are only a handful of transparent quads, behind the swimming plane.
    auto ray_shader = load_shader(ReefShader::SunShafts);
    for (int i = 0; i < 6; ++i) {
        auto *n = memnew(MeshInstance3D);
        Ref<QuadMesh> q;
        q.instantiate();
        q->set_size(Vector2(3 + i % 2, 22));
        n->set_mesh(q);
        Ref<ShaderMaterial> m;
        m.instantiate();
        m->set_shader(ray_shader);
        m->set_shader_parameter("seed", float(i));
        n->set_material_override(m);
        add_child(n);
        n->set_position(Vector3(-11 + i * 5, 11, -8 - i * .15));
        n->set_rotation_degrees(Vector3(0, 0, -17));
        shafts.push_back(n);
    }
    plankton = memnew(MultiMeshInstance3D);
    add_child(plankton);
    Ref<MultiMesh> mm;
    mm.instantiate();
    mm->set_transform_format(MultiMesh::TRANSFORM_3D);
    mm->set_mesh(sphere_mesh);
    mm->set_instance_count(80);
    plankton->set_multimesh(mm);
    plankton->set_material_override(material(Color(.45, .82, .91), .5));
    for (int i = 0; i < 80; ++i) {
        Transform3D t;
        t.basis = Basis().scaled(Vector3(.018, .018, .018));
        t.origin = Vector3(-18 + 36 * rnd(i + 900), 18 * rnd(i + 950), -5 + 8 * rnd(i + 999));
        mm->set_instance_transform(i, t);
    }
    add_water_details();
}
void ReefGame::add_water_details() {
    // A short depth integration simulates suspended light in water on the mobile-friendly renderer.
    auto water = load_shader(ReefShader::WaterVolume);
    water_volume.instantiate();
    water_volume->set_shader(water);
    auto *volume = memnew(MeshInstance3D);
    Ref<QuadMesh> q;
    q.instantiate();
    q->set_size({38, 21});
    volume->set_mesh(q);
    volume->set_material_override(water_volume);
    add_child(volume);
    volume->set_position({0, 9.2, -3});
}
void ReefGame::build_characters() {
    if (!load_character_library(character_library)) {
        assets_failed = true;
        return;
    }
    build_detailed_fish();
    build_sabrina();
    build_jellyfish();
    build_distant_life();
    build_crystal();
    damage_material = material(Color(1, .055, .09, .75), .35);
    damage_material->set_transparency(BaseMaterial3D::TRANSPARENCY_ALPHA);
    fish_pops = memnew(MultiMeshInstance3D);
    add_child(fish_pops);
    Ref<MultiMesh> pops;
    pops.instantiate();
    pops->set_transform_format(MultiMesh::TRANSFORM_3D);
    pops->set_mesh(sphere_mesh);
    pops->set_instance_count(reef::fish_count * 6);
    fish_pops->set_multimesh(pops);
    fish_pops->set_material_override(material(Color(1, .34, .36), .3));
    for (int k = 0; k < reef::fish_count * 6; ++k) {
        Transform3D tr;
        tr.basis = Basis().scaled({0, 0, 0});
        pops->set_instance_transform(k, tr);
    }
}
void ReefGame::apply_graphics() {
    if (!camera)
        return;
    // Compatibility blends shadowed lights in separate passes, clipping the pale
    // hair and skin. Keep the same lighting at every quality on that fallback.
    const bool compatibility =
        RenderingServer::get_singleton()->get_current_rendering_method() == "gl_compatibility";
    sun->set_shadow(quality == 2 && !compatibility);
    water_volume->set_shader_parameter("samples", quality == 0 ? 3 : quality == 1 ? 6 : 12);
    water_volume->set_shader_parameter("movement", reduced_motion ? 0.f : 1.f);
    for (auto &m : reef_materials) {
        m->set_shader_parameter("movement", reduced_motion ? 0.f : 1.f);
        m->set_shader_parameter("strength", quality == 0 ? .45f : 1.f);
    }
    sun->set_param(Light3D::PARAM_SHADOW_BIAS, .1);
    sun->set_param(Light3D::PARAM_SHADOW_NORMAL_BIAS, 2.0);
    for (auto &m : distant_materials)
        m->set_shader_parameter("movement", reduced_motion ? .15f : 1.f);
    get_viewport()->set_msaa_3d(quality == 2 ? Viewport::MSAA_2X : Viewport::MSAA_DISABLED);
    for (size_t i = 0; i < shafts.size(); ++i) {
        shafts[i]->set_visible(quality > 0 && i < size_t(quality == 1 ? 4 : 6));
        auto *mesh = Object::cast_to<MeshInstance3D>(shafts[i]);
        Ref<ShaderMaterial> m = mesh->get_material_override();
        m->set_shader_parameter("movement", reduced_motion ? 0.f : 1.f);
    }
    plankton->get_multimesh()->set_visible_instance_count(reduced_motion ? 0
                                                          : quality == 0 ? 18
                                                          : quality == 1 ? 45
                                                                         : 80);
}
void ReefGame::update_visuals(float dt) {
    float motion = reduced_motion ? .15f : 1.f;
    sun->set_param(Light3D::PARAM_ENERGY,
                   1.15f + (.10f * std::sin(elapsed * .27f) + .05f * std::sin(elapsed * .61f)) * motion);
    sun->set_rotation_degrees(
        {-38 + 5 * std::sin(elapsed * .11f) * motion, -25 + 7 * std::sin(elapsed * .09f) * motion, -20});
    for (size_t i = 0; i < shafts.size(); ++i)
        shafts[i]->set_rotation_degrees({0, 0, -17 + 4 * std::sin(elapsed * .15f + i * .5f) * motion});
    for (size_t i = 0; i < distant_fish.size(); ++i) {
        float speed = i < 4 ? .42f : .65f + .06f * (i % 4), side = i % 2 ? -1.f : 1.f;
        float x = std::fmod(scenery_time * speed + i * 7.13f, 54.f) - 27;
        float y = 8.f + (i % 5) * 1.7f + .55f * std::sin(scenery_time * .14f + i);
        distant_fish[i]->set_position({x * side, y, -18.f - float(i % 3)});
        distant_fish[i]->set_rotation({0, side < 0 ? PI : 0, .025f * std::cos(scenery_time * .14f + i)});
    }
    update_characters(dt, motion);
    for (int i = 0; i < reef::fish_count; ++i) {
        auto &v = fish_visuals[i];
        const auto &f = sim.fish[i];
        v.root->set_visible(f.life.active || f.life.pop_time > .22f);
        v.shield->set_visible(f.life.active && f.rescue.shield > 0);
        if (f.life.active && f.rescue.shield > 0) {
            v.shield->set_position({f.motion.pos.x, f.motion.pos.y, f.appearance.depth + .8f});
            float scale = f.appearance.size;
            v.shield->set_scale({scale, scale, 1});
            Ref<ShaderMaterial> bubble = v.shield->get_material_override();
            bubble->set_shader_parameter("opacity", std::min(1.f, f.rescue.shield));
        }
        bool red = f.life.pop_time > 0;
        if (red != v.red) {
            v.red = red;
            for (auto *mesh : v.rig.meshes)
                mesh->set_material_overlay(red ? damage_material : Ref<StandardMaterial3D>());
        }
        if (!f.life.active && f.life.pop_time <= 0)
            continue;
        if (v.species != f.appearance.species) {
            dispose_rig(v.rig);
            v.species = f.appearance.species;
            v.rig =
                instantiate_rig(v.root, character_library[static_cast<int>(fish_asset(v.species, i % 2))]);
            animate_rig(v.rig, "Swim", 0);
            for (auto *mesh : v.rig.meshes)
                mesh->set_material_overlay(red ? damage_material : Ref<StandardMaterial3D>());
        }
        if (v.entity != f.id) {
            v.entity = f.id;
            v.rig.player->seek(f.swimming.tail_phase / (2 * PI) * 1.2, true);
        }
        float size = f.appearance.size * (.3f + .7f * f.appearance.birth);
        if (f.life.pop_time > 0)
            size *= 1.f + (.55f - f.life.pop_time) * 1.2f;
        v.root->set_scale(Vector3(size, size, size));
        v.root->set_position(Vector3(f.motion.pos.x, f.motion.pos.y, f.appearance.depth));
        // Pitch has the same sign when facing left: yaw changes X/Z, never Y.
        float target = f.motion.direction.x < 0 ? PI : 0;
        float diff = std::atan2(std::sin(target - v.yaw), std::cos(target - v.yaw));
        v.yaw += diff * std::min(1.f, dt * 7);
        float pitch = std::asin(std::clamp(f.motion.direction.y, -1.f, 1.f));
        v.root->set_basis(Basis(Vector3(0, 1, 0), v.yaw) * Basis(Vector3(0, 0, 1), pitch) *
                          Basis().scaled(Vector3(size, size, size)));
        animate_rig(v.rig, "Swim", dt * (reduced_motion ? .25 : 1),
                    (6.f + f.swimming.speed * 5.f) * 1.2f / (2 * PI));
    }
    auto pops = fish_pops->get_multimesh();
    int pop_count = 0;
    for (const auto &f : sim.fish)
        if (f.life.pop_time > 0 && f.life.pop_time < .30f)
            for (int k = 0; k < 6; ++k) {
                float u = std::clamp((.30f - f.life.pop_time) / .30f, 0.f, 1.f), size = .10f * (1 - u);
                float a = k * PI / 3;
                Transform3D tr;
                tr.basis = Basis().scaled({size, size, size});
                tr.origin = {f.motion.pos.x + std::cos(a) * u * .8f, f.motion.pos.y + std::sin(a) * u * .8f,
                             f.appearance.depth};
                pops->set_instance_transform(pop_count++, tr);
            }
    pops->set_visible_instance_count(pop_count);
    update_crystal(dt);
    if (!reduced_motion) {
        auto mm = plankton->get_multimesh();
        for (int i = 0; i < mm->get_visible_instance_count(); ++i) {
            auto t = mm->get_instance_transform(i);
            t.origin.y += dt * (.12 + .06 * rnd(i));
            t.origin.x += std::sin(elapsed * .3 + i) * dt * .03;
            if (t.origin.y > 19)
                t.origin.y = .3;
            mm->set_instance_transform(i, t);
        }
    }
}

void ReefGame::build_distant_life() {
    auto silhouette = load_shader(ReefShader::DistantLife);
    Ref<QuadMesh> mesh;
    mesh.instantiate();
    mesh->set_size({1, 1});
    for (int i = 0; i < 18; ++i) {
        auto *fish = memnew(MeshInstance3D);
        add_child(fish);
        fish->set_mesh(mesh);
        Ref<ShaderMaterial> m;
        m.instantiate();
        m->set_shader(silhouette);
        m->set_shader_parameter("kind", i % 4);
        m->set_shader_parameter("seed", float(i));
        fish->set_material_override(m);
        distant_materials.push_back(m);
        distant_fish.push_back(fish);
        float size = i < 4 ? 7.f + i * .65f : 1.2f + .35f * (i % 4);
        fish->set_scale({size, size * .65f, 1});
    }
}
