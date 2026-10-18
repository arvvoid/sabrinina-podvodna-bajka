// SPDX-License-Identifier: Apache-2.0
// Exercises the real Godot input path and menu signals with synthetic events.
#include "reef_game.h"
#include "shader_library.h"
#include "simulation/contacts.h"
#include "simulation/sabrina_pose.h"
#include <godot_cpp/classes/font.hpp>
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/classes/input_event_joypad_button.hpp>
#include <godot_cpp/classes/input_event_joypad_motion.hpp>
#include <godot_cpp/classes/input_event_key.hpp>
#include <godot_cpp/classes/input_event_mouse_button.hpp>
#include <godot_cpp/classes/input_event_mouse_motion.hpp>
#include <godot_cpp/classes/mesh.hpp>
#include <godot_cpp/classes/scene_tree.hpp>
#include <godot_cpp/classes/skin.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
using namespace godot;
void ReefGame::verify_input() {
    ++verify_frame;
    auto require = [&](bool condition, const char *message) {
        if (!condition) {
            verify_failed = true;
            UtilityFunctions::printerr("INPUT CHECK FAILED: ", message);
            get_tree()->quit(1);
        } else
            UtilityFunctions::print("INPUT CHECK OK: ", message);
    };
    auto key = [](Key code, bool pressed) {
        Ref<InputEventKey> e;
        e.instantiate();
        e->set_physical_keycode(code);
        e->set_pressed(pressed);
        Input::get_singleton()->parse_input_event(e);
    };
    auto stick = [](float value) {
        Ref<InputEventJoypadMotion> e;
        e.instantiate();
        e->set_axis(JOY_AXIS_LEFT_X);
        e->set_axis_value(value);
        Input::get_singleton()->parse_input_event(e);
    };
    auto click = [](bool pressed) {
        Ref<InputEventMouseButton> e;
        e.instantiate();
        e->set_button_index(MOUSE_BUTTON_LEFT);
        e->set_pressed(pressed);
        e->set_position(Vector2(1000, 450));
        Input::get_singleton()->parse_input_event(e);
    };
    switch (verify_frame) {
    case 1: {
        {
            bool loaded = true;
            for (int i = 0; i < static_cast<int>(ReefShader::Count); ++i) {
                auto shader = load_shader(static_cast<ReefShader>(i));
                loaded = loaded && shader.is_valid() && !shader->get_code().is_empty();
            }
            require(loaded, "all external shader resources load");
            require(crystal_model && !crystal_materials.empty(),
                    "textured crystal relic loads with shared shader");
            auto first = load_shader_material(ReefShader::SongRing);
            auto second = load_shader_material(ReefShader::SongRing);
            require(first != second && first->get_shader() == second->get_shader(),
                    "materials own parameters and share cached shader programs");
        }
        require(sabrina_rig.skeleton && sabrina_rig.skeleton->get_bone_count() >= 26 && hand_bone_left >= 0 &&
                    hand_bone_right >= 0,
                "Blender mermaid has a skeleton and hand attachments");
        require(reef_environment && !reef_materials.empty(), "authored reef loads with underwater materials");
        bool clips = true;
        for (const char *name : {"Idle", "Swim", "Sing", "Wave", "Hurt", "Turn_Left", "Turn_Right"})
            clips = clips && sabrina_rig.player->has_animation(name);
        require(clips, "all authored mermaid animation clips are available");
        bool skinned = true;
        for (auto *mesh : sabrina_rig.meshes)
            skinned = skinned && mesh->get_skin().is_valid();
        require(skinned, "mermaid meshes retain exported skin bindings");
        bool colored = true;
        for (auto *mesh : sabrina_rig.meshes) {
            auto geometry = mesh->get_mesh();
            for (int surface = 0; surface < geometry->get_surface_count(); ++surface) {
                Ref<StandardMaterial3D> material = geometry->surface_get_material(surface);
                colored = colored && material.is_valid() &&
                          material->get_flag(BaseMaterial3D::FLAG_ALBEDO_FROM_VERTEX_COLOR);
            }
        }
        require(colored, "imported character materials use authored vertex colors");
        {
            auto &rig = fish_visuals[0].rig;
            int bone = rig.skeleton->find_bone("Caudal");
            Quaternion before = rig.skeleton->get_bone_pose_rotation(bone);
            animate_rig(rig, "Swim", .23);
            require(bone >= 0 && std::abs(before.dot(rig.skeleton->get_bone_pose_rotation(bone))) < .9999,
                    "fish swimming clip moves its skeletal tail");
        }
        {
            // Idle must visibly tread water, even when world velocity is zero.
            animate_rig(sabrina_rig, "Idle", 0);
            sabrina_rig.player->seek(0, true);
            int tail = sabrina_rig.skeleton->find_bone("Tail_4");
            auto before = sabrina_rig.skeleton->get_bone_pose_rotation(tail);
            float minimum_dot = 1;
            for (int quarter = 0; quarter < 8; ++quarter) {
                animate_rig(sabrina_rig, "Idle", .4);
                minimum_dot =
                    std::min(minimum_dot,
                             float(std::abs(before.dot(sabrina_rig.skeleton->get_bone_pose_rotation(tail)))));
            }
            require(minimum_dot < .99, "resting mermaid continues a visible skeletal tail stroke");
        }
        {
            bool variants = true;
            for (int species = 0; species < reef::species_count; ++species) {
                for (int variant = 0; variant < 2; ++variant) {
                    auto rig = instantiate_rig(
                        this, character_library[static_cast<int>(fish_asset(species, variant))]);
                    variants = variants && rig.skeleton && rig.skeleton->find_bone("Caudal") >= 0 &&
                               rig.player->has_animation("Swim") && rig.player->has_animation("Idle");
                    dispose_rig(rig);
                }
            }
            require(variants, "all twelve supplied fish instantiate with swimming rigs");
        }
        {
            auto saved = sim.sabrina;
            bool aligned = true;
            sabrina_rig.player->play("Swim", 0);
            sabrina_rig.clip = "Swim";
            for (float phase : {0.f, 1.5f, 3.f, 4.5f}) {
                sim.sabrina.swimming.phase = phase;
                sim.sabrina.swimming.pitch = .3f;
                sim.sabrina.swimming.yaw = -.4f;
                sim.sabrina.motion.velocity = {4, 0};
                update_characters(1.f / 60, 1);
                auto shapes = reef::sabrina_body(sim.sabrina);
                int bone = sabrina_rig.skeleton->find_bone("Tail_6");
                auto location = bone_attachment(sabrina_rig, bone);

                aligned = aligned && (reef::V2{location.x, location.y} - shapes[6].b).length() < .35f;
            }
            sim.sabrina = saved;
            update_characters(1.f / 60, 1);
            require(aligned, "swimming tail contact shapes follow the displayed skeleton phase");
        }
        {
            auto saved = sim.sabrina;
            sim.sabrina.motion.velocity = {};
            sim.sabrina.swimming.pitch = .65f;
            sim.sabrina.swimming.phase = 0;
            sabrina_rig.player->play("Idle", 0);
            sabrina_rig.clip = "Idle";
            update_characters(1.f / 60, 1);
            std::array<int, 3> bones{sabrina_rig.skeleton->find_bone("Neck"),
                                     sabrina_rig.skeleton->find_bone("Head"),
                                     sabrina_rig.skeleton->find_bone("Chest")};
            std::array<Quaternion, 3> initial;
            for (int i = 0; i < 3; ++i)
                initial[i] = sabrina_rig.skeleton->get_bone_pose_rotation(bones[i]);
            bool stable = true;
            // Two full idle cycles catch offsets accumulating on optimized-out tracks.
            for (int frame = 1; frame <= 384; ++frame) {
                sim.sabrina.swimming.phase = frame * 1.9634954f / 60.f;
                update_characters(1.f / 60, 1);
                for (int i = 0; i < 3; ++i)
                    stable = stable && std::abs(initial[i].dot(
                                           sabrina_rig.skeleton->get_bone_pose_rotation(bones[i]))) > .995f;
            }
            require(stable, "idle upper body stays bounded across repeated animation cycles");
            sim.sabrina = saved;
            update_characters(1.f / 60, 1);
        }
        {
            auto saved = sim.sabrina;
            sim.sabrina.song.active = sim.sabrina.wave.active = false;
            sim.sabrina.vitality.stunned = sim.sabrina.vitality.hurt_flash = 0;
            int head = sabrina_rig.skeleton->find_bone("Head");
            // Neutral-mesh face axis after the authoring orientation (-55 Z, 65 Y).
            Vector3 local_face = sabrina_rig.skeleton->get_bone_global_rest(head).basis.inverse().xform(
                Vector3(.519837f, -.742404f, .422618f));
            bool bounded = true;
            for (float speed : {0.f, 6.f})
                for (float pitch : {-1.1f, 0.f, .65f, 1.1f}) {
                    sim.sabrina.motion.velocity = {speed, 0};
                    sim.sabrina.swimming.pitch = pitch;
                    sim.sabrina.swimming.yaw = 0;
                    sim.sabrina.swimming.phase = 1.2f;
                    update_characters(1.f / 60, 1);
                    Vector3 gaze =
                        mermaid->get_basis()
                            .xform(sabrina_rig.skeleton->get_bone_global_pose(head).basis.xform(local_face))
                            .normalized();
                    bounded = bounded && gaze.y < .75f && gaze.y > -.99f;
                    if (speed > 0)
                        bounded = bounded && gaze.dot(Vector3(std::cos(pitch), std::sin(pitch), 0)) > .55f;
                }
            require(bounded, "head gaze stays bounded while resting, ascending and diving");
            bool arms_aligned = true;
            constexpr const char *names[] = {"Idle", "Swim", "Turn_Left", "Turn_Right",
                                             "Sing", "Wave", "Hurt",      "Throw"};
            for (int mode = 0; mode < 8; ++mode) {
                sim.sabrina.motion.velocity = mode == 0 ? reef::V2{} : reef::V2{5, 0};
                sim.sabrina.swimming.yaw = mode == 2 || mode == 3 ? -.9f : 0;
                sim.sabrina.swimming.facing = mode == 2 ? -1 : 1;
                sim.sabrina.swimming.pitch = .3f;
                sim.sabrina.song.active = mode == 4;
                sim.sabrina.wave.active = mode == 5;
                sim.sabrina.vitality.stunned = mode == 6 ? 1 : 0;
                sim.sabrina.throw_time = mode == 7 ? 0 : -1;
                sabrina_rig.player->play(names[mode], 0);
                sabrina_rig.clip = names[mode];
                for (float phase : {.2f, 1.8f, 3.4f, 5.f}) {
                    sim.sabrina.swimming.phase = phase;
                    sim.sabrina.wave.time = phase / 6.2831853f * .75f;
                    if (mode == 7)
                        sim.sabrina.throw_time = phase / 6.2831853f * 1.6f;
                    update_characters(1.f / 60, 1);
                    auto shapes = reef::sabrina_body(sim.sabrina);
                    for (int side = 0; side < 2; ++side) {
                        auto shoulder = bone_attachment(
                            sabrina_rig, sabrina_rig.skeleton->find_bone(side ? "UpperArm_R" : "UpperArm_L"));
                        auto elbow = bone_attachment(
                            sabrina_rig, sabrina_rig.skeleton->find_bone(side ? "Forearm_R" : "Forearm_L"));
                        arms_aligned =
                            arms_aligned &&
                            (reef::V2{shoulder.x, shoulder.y} - shapes[12 + side * 2].a).length() < .12f &&
                            (reef::V2{elbow.x, elbow.y} - shapes[12 + side * 2].b).length() < .12f;
                    }
                }
            }
            require(arms_aligned, "arm contacts follow authored shoulders and elbows in all eight poses");
            bool grip_aligned = true;
            sim.sabrina.vitality.stunned = 0;
            sim.sabrina.wave.active = sim.sabrina.song.active = false;
            for (float yaw : {0.f, -1.5f, -3.14159265f})
                for (float pitch : {-1.1f, 0.f, 1.1f})
                    for (float time : {.45f, .7f, 1.05f}) {
                        sim.sabrina.swimming.yaw = yaw;
                        sim.sabrina.swimming.pitch = pitch;
                        sim.sabrina.throw_time = time;
                        update_characters(1.f / 60, 1);
                        auto hand = bone_attachment(sabrina_rig, hand_bone_right,
                                                    {0, reef::sabrina_palm_offset(), 0});
                        auto grip = reef::sabrina_palm(sim.sabrina);
                        grip_aligned =
                            grip_aligned &&
                            (hand - Vector3(grip.offset.x, grip.offset.y, grip.depth)).length() < .12f;
                    }
            require(grip_aligned, "exported crystal grip follows the rendered palm in all directions");
            sim.sabrina = saved;
            update_characters(1.f / 60, 1);
        }
        require(!loading && !loading_ui->is_visible(), "staged loading yields to menu");
        require(menu->is_visible() && !started, "title screen starts visible");
        {
            int prior = language;
            language = 1;
            refresh_text();
            require(credits_button->get_text() == String::utf8("Autori") &&
                        quit_button->get_text() == String::utf8("Doviđenja"),
                    "Croatian UI decodes UTF-8");
            auto font = title->get_theme_default_font();
            require(font->has_char(0x010d) && font->has_char(0x0107) && font->has_char(0x017e) &&
                        font->has_char(0x0161) && font->has_char(0x0111),
                    "bundled font covers Croatian diacritics");
            language = prior;
            refresh_text();
        }
        for (auto direction : {reef::V2{-.8f, .6f}, reef::V2{-.8f, -.6f}, reef::V2{.8f, .6f}}) {
            sim.fish[0].motion.direction = direction;
            sim.fish[0].motion.velocity = direction;
            update_visuals(.25f);
            Vector3 nose = fish_visuals[0].root->get_basis().get_column(0).normalized();
            require(nose.dot(Vector3(direction.x, direction.y, 0)) > .99f,
                    "rendered fish nose follows swimming heading");
        }
        play_button->emit_signal("pressed");
        require(started && !menu->is_visible() && hud->is_visible(), "play button starts game");
        key(KEY_D, true);
        break;
    }
    case 3:
        key(KEY_F, true);
        break;
    case 4:
        key(KEY_F, false);
        break;
    case 6:
        require(sim.sabrina.wave.active && sim.sabrina.wave.cooldown > 0,
                "keyboard F casts directional wave");
        sim.sabrina.wave.cooldown = 0;
        break;
    case 8: {
        Ref<InputEventMouseButton> e;
        e.instantiate();
        e->set_button_index(MOUSE_BUTTON_RIGHT);
        e->set_pressed(true);
        Input::get_singleton()->parse_input_event(e);
        break;
    }
    case 9: {
        Ref<InputEventMouseButton> e;
        e.instantiate();
        e->set_button_index(MOUSE_BUTTON_RIGHT);
        e->set_pressed(false);
        Input::get_singleton()->parse_input_event(e);
        break;
    }
    case 11:
        require(sim.sabrina.wave.cooldown > 0, "right mouse button casts wave");
        sim.sabrina.wave.cooldown = 0;
        break;
    case 12: {
        Ref<InputEventJoypadButton> e;
        e.instantiate();
        e->set_button_index(JOY_BUTTON_B);
        e->set_pressed(true);
        Input::get_singleton()->parse_input_event(e);
        break;
    }
    case 13: {
        Ref<InputEventJoypadButton> e;
        e.instantiate();
        e->set_button_index(JOY_BUTTON_B);
        e->set_pressed(false);
        Input::get_singleton()->parse_input_event(e);
        break;
    }
    case 15:
        require(sim.sabrina.wave.cooldown > 0, "controller B casts wave");
        sim.sabrina.wave.cooldown = 0;
        wave_button->emit_signal("pressed");
        break;
    case 18:
        require(sim.sabrina.wave.cooldown > 0, "HUD button casts wave");
        break;
    case 25:
        require(sim.sabrina.motion.pos.x > -5, "keyboard D moves Sabrina");
        key(KEY_D, false);
        toggle_pause();
        verify_value = sim.session.time;
        verify_rig_time = sabrina_rig.player->get_current_animation_position();
        break;
    case 45:
        require(sim.session.time == verify_value && pause_panel->is_visible(), "pause freezes simulation");
        require(sabrina_rig.player->get_current_animation_position() == verify_rig_time,
                "pause freezes skeletal animation");
        resume_button->emit_signal("pressed");
        verify_value = sim.sabrina.motion.pos.x;
        stick(-1);
        break;
    case 80:
        require(sim.sabrina.motion.pos.x < verify_value - 1, "controller left stick moves Sabrina");
        stick(0);
        verify_value = sim.sabrina.motion.pos.x;
        break;
    case 82: {
        Ref<InputEventMouseMotion> e;
        e.instantiate();
        e->set_position(Vector2(1000, 450));
        e->set_relative(Vector2(30, 0));
        Input::get_singleton()->parse_input_event(e);
        break;
    }
    case 115:
        require(pointer_mode && sim.sabrina.motion.pos.x > verify_value + 1, "mouse pointer steers Sabrina");
        click(true);
        break;
    case 117:
        require(sim.crystal.magic.state == reef::Spell::Charging, "mouse click casts magic");
        click(false);
        key(KEY_ESCAPE, true);
        break;
    case 120:
        require(paused && pause_panel->is_visible(), "Escape opens pause menu");
        key(KEY_ESCAPE, false);
        home_button->emit_signal("pressed");
        require(!started && menu->is_visible() && !hud->is_visible(), "home returns to title screen");
        {
            int old = language;
            language_button->emit_signal("pressed");
            require(language == (old + 1) % 3, "language selector updates");
            language = old;
            refresh_text();
        }
        play_button->emit_signal("pressed");
        {
            Ref<InputEventJoypadButton> e;
            e.instantiate();
            e->set_button_index(JOY_BUTTON_A);
            e->set_pressed(true);
            Input::get_singleton()->parse_input_event(e);
        }
        break;
    case 123:
        require(sim.crystal.magic.state == reef::Spell::Charging, "controller A casts magic");
        toggle_song();
        require(sim.sabrina.song.active, "siren song toggles");
        sim.sabrina.song.active = false;
        sim.sabrina.song.locked = true;
        sim.sabrina.song.energy = 12;
        toggle_song();
        require(!sim.sabrina.song.active, "UI cannot restart an exhausted song during recharge");
        update_hud();
        require(song_bar->get_value() == 12 &&
                    song_button->get_text().contains(tr3("Recharging", "Obnavljanje", "Ricarica")),
                "HUD shows locked song energy");
        open_settings();
        require(paused && settings_panel->is_visible(), "settings pause gameplay");
        music_slider->set_value(23);
        require(music_gain > .22f && music_gain < .24f, "music slider changes gain");
        rest_slider->set_value(90);
        require(sim.settings.rest_seconds == 90, "rest slider updates pacing");
        close_panel();
        require(pause_panel->is_visible(), "settings return to pause");
        go_home();
        for (int locale = 0; locale < 3; ++locale) {
            language = locale;
            refresh_text();
            credits_button->emit_signal("pressed");
            require(info_text->get_text().contains("Luka Dezulovic") &&
                        info_text->get_text().contains("Sabrina Dezulovic") &&
                        info_text->get_text().contains("https://scratch.mit.edu/projects/1203591608/") &&
                        original_project_button->is_visible() && !help_replay->is_visible() &&
                        !help_pause->is_visible() && !info_is_help,
                    "localized credits include authors and original Scratch project without narration");
            close_panel();
        }
        require(menu->is_visible(), "credits return to title");
        open_help();
        require(info_is_help && help_replay->is_visible() && help_pause->is_visible() &&
                    !original_project_button->is_visible() && info_text->get_text().contains("WASD"),
                "help retains controls and original help narration controls");
        close_panel();
        aquarium_button->emit_signal("pressed");
        require(sim.settings.aquarium, "quiet aquarium mode starts");
        start_game();
        sim.sabrina.vitality.health = game_settings.gentle ? 28 : 35;
        reef::begin_visit(sim);
        sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
        sim.jelly[0].motion.pos = sim.sabrina.motion.pos;
        break;
    case 125:
        require(sim.session.game_over && sim.sabrina.vitality.health == 0 && gameover_panel->is_visible(),
                "fatal contact opens game-over screen");
        toggle_pause();
        require(paused && !pause_panel->is_visible(), "pause cannot dismiss game over");
        retry_button->emit_signal("pressed");
        require(!sim.session.game_over && sim.sabrina.vitality.health == 100 &&
                    !gameover_panel->is_visible() && started,
                "retry starts a fresh game");
        UtilityFunctions::print("INPUT INTEGRATION COMPLETE");
        get_tree()->quit(verify_failed ? 1 : 0);
        break;
    }
}
