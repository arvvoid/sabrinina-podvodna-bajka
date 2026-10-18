// SPDX-License-Identifier: MIT
#include "reef_game.h"
#include "simulation/contacts.h"
#include <algorithm>
#include <godot_cpp/classes/display_server.hpp>
#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/classes/image.hpp>
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/classes/input_event_joypad_button.hpp>
#include <godot_cpp/classes/input_event_joypad_motion.hpp>
#include <godot_cpp/classes/input_event_key.hpp>
#include <godot_cpp/classes/input_event_mouse_button.hpp>
#include <godot_cpp/classes/input_event_mouse_motion.hpp>
#include <godot_cpp/classes/input_map.hpp>
#include <godot_cpp/classes/os.hpp>
#include <godot_cpp/classes/scene_tree.hpp>
#include <godot_cpp/classes/viewport.hpp>
#include <godot_cpp/classes/viewport_texture.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
using namespace godot;
void ReefGame::_bind_methods() {
#define B(m) ClassDB::bind_method(D_METHOD(#m), &ReefGame::m)
    B(start_game);
    B(start_aquarium);
    B(toggle_pause);
    B(go_home);
    B(cycle_language);
    B(toggle_song);
    B(cast_wave);
    B(open_settings);
    B(open_credits);
    B(open_original_project);
    B(open_help);
    B(close_panel);
    B(replay_help);
    B(pause_help);
    B(quit_game);
    B(toggle_fullscreen);
    B(cycle_quality);
    B(toggle_motion);
    B(toggle_gentle);
    B(cycle_visitors);
    B(set_music_volume);
    B(set_effects_volume);
    B(set_voice_volume);
    B(set_rest_time);
#undef B
}
String ReefGame::tr3(const char *en, const char *hr, const char *it) const {
    return String::utf8(language == 1 ? hr : language == 2 ? it : en);
}
void ReefGame::setup_input() {
    auto *m = InputMap::get_singleton();
    auto bind = [&](const char *n, Key a, Key b, JoyAxis axis, float value) {
        m->add_action(n, .2);
        for (Key k : {a, b}) {
            Ref<InputEventKey> e;
            e.instantiate();
            e->set_physical_keycode(k);
            m->action_add_event(n, e);
        }
        Ref<InputEventJoypadMotion> e;
        e.instantiate();
        e->set_axis(axis);
        e->set_axis_value(value);
        m->action_add_event(n, e);
    };
    bind("left", KEY_A, KEY_LEFT, JOY_AXIS_LEFT_X, -1);
    bind("right", KEY_D, KEY_RIGHT, JOY_AXIS_LEFT_X, 1);
    bind("up", KEY_W, KEY_UP, JOY_AXIS_LEFT_Y, -1);
    bind("down", KEY_S, KEY_DOWN, JOY_AXIS_LEFT_Y, 1);
    m->add_action("magic");
    for (Key k : {KEY_SPACE, KEY_E}) {
        Ref<InputEventKey> e;
        e.instantiate();
        e->set_physical_keycode(k);
        m->action_add_event("magic", e);
    }
    for (JoyButton b : {JOY_BUTTON_A, JOY_BUTTON_RIGHT_SHOULDER}) {
        Ref<InputEventJoypadButton> e;
        e.instantiate();
        e->set_button_index(b);
        m->action_add_event("magic", e);
    }
    m->add_action("wave");
    Ref<InputEventKey> wave_key;
    wave_key.instantiate();
    wave_key->set_physical_keycode(KEY_F);
    m->action_add_event("wave", wave_key);
    Ref<InputEventJoypadButton> wave_pad;
    wave_pad.instantiate();
    wave_pad->set_button_index(JOY_BUTTON_B);
    m->action_add_event("wave", wave_pad);
    Ref<InputEventMouseButton> wave_mouse;
    wave_mouse.instantiate();
    wave_mouse->set_button_index(MOUSE_BUTTON_RIGHT);
    m->action_add_event("wave", wave_mouse);
}
void ReefGame::_ready() {
    for (auto a : OS::get_singleton()->get_cmdline_user_args()) {
        if (a == "--smoke")
            smoke_mode = true;
        if (a == "--verify-input")
            verify_mode = true;
        if (a.begins_with("--capture-")) {
            capture_mode = true;
            capture_kind = a.substr(10);
        }
    }
    headless = DisplayServer::get_singleton()->get_name() == "headless";
    load_settings();
    if (capture_mode && capture_kind.begins_with("hr"))
        language = 1;
    if (capture_mode && capture_kind.begins_with("it"))
        language = 2;
    if (capture_mode && capture_kind.begins_with("graphics-")) {
        quality = capture_kind.ends_with("high") ? 2 : capture_kind.ends_with("low") ? 0 : 1;
        reduced_motion = true;
    }
    setup_input();
    build_loading();
    if (!headless && !capture_mode && !verify_mode && !smoke_mode)
        DisplayServer::get_singleton()->window_set_mode(fullscreen ? DisplayServer::WINDOW_MODE_FULLSCREEN
                                                                   : DisplayServer::WINDOW_MODE_WINDOWED);
}
void ReefGame::finish_loading() {
    loading = false;
    loading_ui->hide();
    ui->show();
    refresh_text();
    go_home();
    if (smoke_mode)
        start_game();
    if (capture_mode && capture_kind != "menu" && capture_kind != "hr-menu") {
        start_game();
        reef::add_fish(sim, 14, false);
        if (capture_kind == "visit" || capture_kind == "magic") {
            reef::begin_visit(sim);
            sim.jelly[0].motion.pos = {7, 10};
            sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
        }
        if (capture_kind == "design" || capture_kind == "song" || capture_kind == "animation") {
            sim.sabrina.motion.pos = {-2, 8};
            reef::begin_visit(sim);
            sim.jelly[0].motion.pos = {-7, 11};
            sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
            camera->set_size(capture_kind == "animation" ? 13 : 10.5);
            camera->set_position({0, 8, 36});
            if (capture_kind == "song")
                sim.sabrina.song.active = true;
            hud->hide();
        }
        if (capture_kind.begins_with("graphics-")) {
            sim.settings.aquarium = true;
            sim.sabrina.motion.pos = {0, 8};
            sim.sabrina.motion.velocity = {4, 0};
            sim.sabrina.swimming.pitch = .05f;
            sim.sabrina.swimming.yaw = 0;
            sim.sabrina.swimming.phase = 1.2f;
            sim.sabrina.cooldown = 1000;
            for (auto &fish : sim.fish)
                fish.life.active = false;
            camera->set_position({0, 8.5, 36});
            camera->set_size(7.5);
            hud->hide();
            sabrina_rig.player->play("Swim", 0);
            sabrina_rig.clip = "Swim";
            update_visuals(1.f / 60);
            paused = true;
        }
        if (capture_kind == "rig" || capture_kind == "throw") {
            sim.settings.aquarium = true;
            sim.session.spawn_in = 1000;
            sim.sabrina.motion.pos = {-3, 8};
            sim.sabrina.cooldown = capture_kind == "throw" ? 0 : 1000;
            for (auto &fish : sim.fish)
                fish.life.active = false;
            camera->set_size(capture_kind == "throw" ? 11 : 7.5);
            if (capture_kind == "throw") {
                sim.settings.aquarium = false;
                reef::begin_visit(sim);
                sim.jelly[0].motion.pos = {4, 9};
                sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
                sim.jelly[0].behaviour.turn_in = 100;
            }
            hud->hide();
        }
        if (capture_kind.begins_with("pose-")) {
            sim.settings.aquarium = true;
            sim.sabrina.motion.pos = {0, 8};
            sim.sabrina.motion.velocity = {5, 0};
            sim.sabrina.swimming.phase = 1.2f;
            sim.sabrina.swimming.pitch = capture_kind.contains("dive")    ? -1.1f
                                         : capture_kind.contains("climb") ? 1.1f
                                                                          : 0.f;
            sim.sabrina.swimming.yaw = capture_kind.contains("left") ? -3.14159265f : 0;
            sim.sabrina.swimming.facing = capture_kind.contains("left") ? -1 : 1;
            sim.sabrina.cooldown = 1000;
            if (capture_kind.contains("throw")) {
                sim.sabrina.throw_time = capture_kind.contains("release") ? 1.08f : .65f;
                sim.crystal.magic.state = reef::Spell::Charging;
                sim.crystal.magic.time = sim.sabrina.throw_time;
                auto palm = reef::sabrina_palm(sim.sabrina);
                sim.crystal.motion.pos = palm.offset;
                sim.crystal.magic.depth = palm.depth;
            }
            for (auto &fish : sim.fish)
                fish.life.active = false;
            camera->set_position({0, 8, 36});
            camera->set_size(6.5);
            hud->hide();
            const char *pose_clip = capture_kind.contains("throw") ? "Throw" : "Swim";
            sabrina_rig.player->play(pose_clip, 0);
            sabrina_rig.clip = pose_clip;
            update_visuals(1.f / 60);
            paused = true;
        }
        if (capture_kind == "bumps") {
            sim.settings.aquarium = true;
            sim.session.spawn_in = 1000;
            sim.sabrina.motion.pos = {0, 7.5f};
            for (auto &fish : sim.fish)
                fish.life.active = false;
            for (int i = 0; i < 3; ++i) {
                auto &fish = sim.fish[i];
                fish.life.active = true;
                fish.appearance.birth = 1;
                fish.appearance.size = .85f;
                fish.motion.pos = i == 0   ? reef::V2{-2, 10.5f}
                                  : i == 1 ? reef::V2{2, 10.5f}
                                           : reef::V2{3, 8.0f};
                fish.motion.direction = {i == 0 ? 1.f : -1.f, 0};
                fish.motion.velocity = fish.motion.direction;
                fish.swimming.cruise_heading = i == 0 ? 0 : 3.14159265f;
                fish.swimming.turn_in = 100;
            }
            camera->set_size(10.5);
            camera->set_position({0, 8, 36});
            hud->hide();
        }
        if (capture_kind == "contact" || capture_kind == "hurt" || capture_kind == "gameover") {
            sim.sabrina.motion.pos = {-3, 8};
            reef::begin_visit(sim);
            sim.jelly[0].motion.pos = {3, 8};
            sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
            camera->set_size(10.5);
            camera->set_position({0, 8, 36});
        }
        if (capture_kind == "school")
            reef::add_fish(sim, 72, false);
        if (capture_kind == "resources") {
            sim.sabrina.song.active = true;
            sim.sabrina.song.energy = 55;
            sim.sabrina.song.strength = 1;
            sim.sabrina.cooldown = 95;
        }
        if (capture_kind == "wave") {
            reef::begin_visit(sim);
            sim.sabrina.motion.pos = {-2, 8};
            sim.jelly[0].motion.pos = {-7, 11};
            sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
            sim.sabrina.cooldown = 95;
            for (auto &f : sim.fish)
                f.life.active = false;
            for (int k = 0; k < 4; ++k) {
                auto &f = sim.fish[k];
                f.life.active = true;
                f.appearance.birth = 1;
                f.appearance.size = .65f;
                f.motion.pos = {-.2f + k * .8f, 8.4f + (k % 2) * .6f};
                f.motion.direction = {-1, 0};
                f.swimming.cruise_heading = 3.14159265f;
                f.swimming.turn_in = 100;
            }
            camera->set_size(12);
            camera->set_position({0, 8, 36});
        }
        if (capture_kind == "depth" || capture_kind == "depth-tour") {
            sim.sabrina.motion.pos = {-11, 4.2f};
            sim.sabrina.vitality.invulnerable = 100;
            reef::begin_visit(sim);
            sim.jelly[0].motion.pos = {-15, 4};
            sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
            sim.jelly[0].behaviour.eaten = 6;
            for (int k = 0; k < 6; ++k)
                sim.fish[k].motion.pos = {-14.f + k, 3.3f};
            camera->set_size(12);
            camera->set_position({-8, 5.8f, 36});
            hud->hide();
        }
        if (capture_kind == "crystal") {
            sim.sabrina.motion.pos = {-2, 8};
            reef::begin_visit(sim);
            sim.jelly[0].motion.pos = {9, 10};
            sim.jelly[0].behaviour.state = reef::JellyState::Hungry;
            camera->set_size(13);
            camera->set_position({1, 8, 36});
            hud->hide();
        }
        if (capture_kind == "locked") {
            sim.sabrina.song.energy = 15;
            sim.sabrina.song.locked = true;
            sim.sabrina.cooldown = 80;
        }
        if (capture_kind == "visitors")
            scenery_time = 43;
        if (capture_kind == "manta")
            scenery_time = 138;
        if (capture_kind == "turtle")
            scenery_time = 233;
        if (capture_kind == "settings" || capture_kind == "hr-settings")
            open_settings();
        if (capture_kind == "hr-help")
            open_help();
        if (capture_kind == "credits" || capture_kind == "hr-credits" || capture_kind == "it-credits")
            open_credits();
    }
}
void ReefGame::_exit_tree() {
    for (auto *p : {effects, voice, music, visit_music, song})
        if (p) {
            p->stop();
            p->set_stream(Ref<AudioStream>());
        }
}
void ReefGame::begin(bool aquarium) {
    started = true;
    paused = false;
    pointer_mode = false;
    game_settings.aquarium = aquarium;
    reef::reset(sim, 27 + int(elapsed * 10), game_settings);
    gameover_panel->hide();
    sim.points.best_score = best;
    menu->hide();
    hud->show();
    pause_panel->hide();
    settings_panel->hide();
    info_panel->hide();
    score_time = 0;
    voice->stop();
    caption->set_text("");
    refresh_text();
}
void ReefGame::start_game() {
    begin(false);
}
void ReefGame::start_aquarium() {
    begin(true);
}
void ReefGame::go_home() {
    started = false;
    gameover_panel->hide();
    paused = false;
    pointer_mode = false;
    auto s = game_settings;
    s.aquarium = true;
    reef::reset(sim, 27, s);
    reef::add_fish(sim, 16, false);
    sim.sabrina.motion.pos = {4, 6};
    menu->show();
    hud->hide();
    pause_panel->hide();
    settings_panel->hide();
    info_panel->hide();
    voice->stop();
    caption->set_text("");
    play_button->grab_focus();
}
void ReefGame::toggle_pause() {
    if (sim.session.game_over)
        return;
    if (settings_panel->is_visible() || info_panel->is_visible()) {
        close_panel();
        return;
    }
    if (!started)
        return;
    paused = !paused;
    pause_panel->set_visible(paused);
    if (paused)
        resume_button->grab_focus();
}
void ReefGame::cast_wave() {
    if (started && !paused && !sim.session.game_over)
        wave_pending = true;
}
void ReefGame::toggle_song() {
    if (started && !paused && sim.sabrina.vitality.stunned <= 0)
        if (!reef::toggle_song(sim))
            show_notice(tr3("Let the song recharge fully.", "Pričekaj da se pjesma potpuno obnovi.",
                            "Lascia ricaricare completamente il canto."));
    refresh_text();
}
void ReefGame::cycle_language() {
    language = (language + 1) % 3;
    refresh_text();
    save_settings();
}
void ReefGame::quit_game() {
    save_settings();
    get_tree()->quit();
}
void ReefGame::show_notice(const String &t, float seconds) {
    notice->set_text(t);
    notice_time = seconds;
}
void ReefGame::_unhandled_input(const Ref<InputEvent> &event) {
    if (capture_mode || loading)
        return;
    Ref<InputEventKey> k = event;
    if (k.is_valid() && k->is_pressed() && !k->is_echo()) {
        if (k->get_physical_keycode() == KEY_ESCAPE)
            toggle_pause();
        if (k->get_physical_keycode() == KEY_F11)
            toggle_fullscreen();
        if (k->get_physical_keycode() == KEY_Q)
            toggle_song();
    }
    Ref<InputEventJoypadButton> j = event;
    if (j.is_valid() && j->is_pressed()) {
        if (j->get_button_index() == JOY_BUTTON_START)
            toggle_pause();
        if (j->get_button_index() == JOY_BUTTON_X)
            toggle_song();
    }
    Ref<InputEventMouseMotion> m = event;
    if (m.is_valid() && m->get_relative().length() > 1 && started && !paused) {
        pointer_mode = true;
        pointer = m->get_position();
    }
    Ref<InputEventMouseButton> b = event;
    if (b.is_valid() && b->is_pressed() && b->get_button_index() == MOUSE_BUTTON_LEFT && started && !paused) {
        Input::get_singleton()->action_press("magic");
    }
    if (b.is_valid() && !b->is_pressed() && b->get_button_index() == MOUSE_BUTTON_LEFT)
        Input::get_singleton()->action_release("magic");
}
void ReefGame::_physics_process(double delta) {
    if (loading)
        return;
    if (verify_mode)
        verify_input();
    if (paused)
        return;
    reef::Input in;
    auto *i = Input::get_singleton();
    if (started) {
        Vector2 v = i->get_vector("left", "right", "up", "down");
        if (v.length() > .1)
            pointer_mode = false;
        in.move = {v.x, -v.y};
        if (pointer_mode) {
            Vector3 origin = camera->project_ray_origin(pointer), ray = camera->project_ray_normal(pointer);
            Vector3 p = origin + ray * (-origin.z / ray.z);
            in.move = {(p.x - sim.sabrina.motion.pos.x) * .5f, (p.y - sim.sabrina.motion.pos.y) * .5f};
        }
        in.cast = i->is_action_just_pressed("magic");
        in.wave = i->is_action_just_pressed("wave") || wave_pending;
        wave_pending = false;
    }
    if (smoke_mode) {
        in.move = {std::sin(sim.session.time * .4f), std::cos(sim.session.time * .3f)};
        in.cast = sim.session.water == reef::Water::Visit;
    }
    if (capture_mode && capture_kind == "magic" && sim.session.time > 1.3f && sim.session.time < 1.4f)
        in.cast = true;
    if (capture_mode && capture_kind == "wave" &&
        ((sim.session.time > .8f && sim.session.time < .84f) ||
         (sim.session.time > 4.2f && sim.session.time < 4.24f)))
        in.wave = true;
    if (capture_mode && capture_kind == "crystal" && sim.session.time > 4 && sim.session.time < 4.05f)
        in.cast = true;
    if (capture_mode && capture_kind == "depth-tour")
        in.move = {std::sin(sim.session.time * .8f) * .35f, std::cos(sim.session.time * .5f) * .13f};
    if (capture_mode && capture_kind == "animation") {
        const float t = sim.session.time;
        if (t > 4 && t < 10)
            in.move = {.45f * std::cos((t - 4) * 1.04719755f), .22f * std::sin((t - 4) * 1.04719755f)};
        sim.sabrina.song.active = t >= 10 && t < 13;
        if (t > 13.25f && t < 13.28f)
            in.wave = true;
    }
    if (capture_mode && capture_kind == "rig") {
        float t = sim.session.time;
        if (t >= 3 && t < 6)
            in.move = {.35f, 0};
        else if (t >= 6 && t < 9)
            in.move = {-.35f, 0};
        else if (t >= 9 && t < 11)
            in.move = {0, .35f};
        else if (t >= 11 && t < 13)
            in.move = {0, -.35f};
        sim.sabrina.song.active = t >= 13 && t < 15;
        if (t >= 15.3f && t < 15.34f)
            in.wave = true;
        camera->set_position({sim.sabrina.motion.pos.x, sim.sabrina.motion.pos.y, 36});
    }
    if (capture_mode && capture_kind == "throw") {
        if (sim.session.time > 1 && sim.session.time < 1.04f)
            in.cast = true;
        camera->set_position({-1, 8, 36});
    }
    if (capture_mode && capture_kind == "swim")
        in.move = {sim.session.time < 1.5f ? 1.f : -1.f, .12f};
    if (capture_mode && sim.session.time > 2.70f && sim.session.time < 2.73f) {
        if (capture_kind == "contact") {
            sim.fish[0].life.active = true;
            sim.fish[0].motion.pos = sim.jelly[0].motion.pos;
            sim.fish[0].rescue.shield = 0;
            sim.fish[0].rescue.teleport = 0;
        }
        if (capture_kind == "hurt" || capture_kind == "gameover") {
            sim.jelly[0].motion.pos = sim.sabrina.motion.pos;
            sim.sabrina.vitality.invulnerable = 0;
            if (capture_kind == "gameover")
                sim.sabrina.vitality.health = 1;
        }
    }
    sim.settings.reduced_motion = reduced_motion;
    auto e = reef::step(sim, float(delta), in);
    if (e.song_empty)
        show_notice(tr3("The song is resting. It must recharge fully.",
                        "Pjesma se odmara. Mora se potpuno obnoviti.",
                        "Il canto riposa. Deve ricaricarsi completamente."));
    if (e.wave_cast)
        play_effect("magic_burst", .65);
    if (e.cast)
        play_effect("magic_charge");
    if (e.teleported)
        play_effect("magic_collect", 1.1);
    if (e.repelled) {
        play_effect("magic_burst");
        play_voice("jelly_repelled", tr3("That mermaid and her crystal!", "Ta sirena i njezin kristal!",
                                         "Quella sirena e il suo cristallo!"));
    }
    if (e.hit) {
        refresh_text();
        play_effect("sting");
        show_notice(tr3("A little sting. Sabrina will recover soon.",
                        "Mala opeklina. Sabrina će se brzo oporaviti.",
                        "Una piccola puntura. Sabrina si riprenderà presto."));
    }
    if (e.game_over && smoke_mode) {
        UtilityFunctions::print("SMOKE COMPLETE: game over at step ", smoke_steps,
                                " visits=", sim.session.cycle, " fish=", reef::alive(sim));
        get_tree()->quit();
    }
    if (e.visit_started)
        play_voice("jelly_arrive",
                   tr3("I'm hungry! Where are my little fish?", "Gladna sam! Gdje su moje ribice?",
                       "Ho fame! Dove sono i miei pesciolini?"));
    if (e.eaten)
        play_voice("jelly_eat", tr3("Oops! I ate you!", "Ups! Pojela sam te!", "Ops! Ti ho mangiato!"));
    if (e.leaving)
        play_voice("jelly_leave",
                   tr3("The jellyfish is swimming away.", "Meduza pliva dalje.", "La medusa nuota via."));
    if (e.visit_ended) {
        score_time = 10;
        best = std::max(best, sim.points.best_score);
        save_settings();
    }
    if (e.game_over) {
        paused = true;
        pause_panel->hide();
        gameover_panel->show();
        retry_button->grab_focus();
        notice_time = 0;
        score_time = 0;
        voice->stop();
        caption->set_text("");
    }
    if (smoke_mode && ++smoke_steps >= 12000) {
        UtilityFunctions::print("SMOKE COMPLETE: visits=", sim.session.cycle, " fish=", reef::alive(sim),
                                " score=", sim.points.total_score);
        get_tree()->quit(sim.session.cycle > 0 && reef::alive(sim) > 0 ? 0 : 1);
    }
}
void ReefGame::_process(double delta) {
    if (assets_failed)
        return;
    float dt = float(delta);
    elapsed += dt;
    if (loading) {
        load_time += dt;
        switch (load_stage) {
        case 0:
            break;
        case 1:
            build_world();
            if (assets_failed) {
                get_tree()->quit(1);
                return;
            }
            break;
        case 2:
            build_characters();
            if (assets_failed) {
                get_tree()->quit(1);
                return;
            }
            {
                auto s = game_settings;
                s.aquarium = true;
                reef::reset(sim, 27, s);
                sim.sabrina.motion.pos = {7, 8};
            }
            loading_cover->hide();
            break;
        case 3:
            build_ui();
            ui->hide();
            break;
        case 4:
            build_audio();
            break;
        case 5:
            apply_graphics();
            break;
        }
        if (load_stage >= 3) {
            update_visuals(dt);
            mermaid->set_scale({1.65, 1.65, 1.65});
            mermaid->set_rotation({0, -.2, 0});
            mermaid->set_position({8, 8.5, .5});
            if (capture_kind == "boot")
                loading_bar->hide();
        }
        loading_bar->set_value(std::min(100, load_stage * 20));
        loading_status->set_text(
            (load_stage < 5 || capture_kind == "loading" || capture_kind == "boot")
                ? tr3("Please wait… bringing the reef to life", "Molimo pričekaj… greben oživljava",
                      "Attendi… la barriera prende vita")
                : tr3("Welcome to Sabrina's little sea", "Dobro došli u Sabrinino malo more",
                      "Benvenuti nel piccolo mare di Sabrina"));
        if (load_stage < 6)
            ++load_stage;
        else if (capture_mode && (capture_kind == "loading" || capture_kind == "boot") && load_time > 1.4f) {
            get_viewport()->get_texture()->get_image()->save_png("/tmp/sabrina-" + capture_kind + ".png");
            get_tree()->quit();
        } else if ((headless || load_time > 1.6f) && capture_kind != "loading" && capture_kind != "boot")
            finish_loading();
        return;
    }
    notice_time = std::max(0.f, notice_time - dt);
    score_time = std::max(0.f, score_time - dt);
    update_visuals(paused ? 0 : dt);
    update_hud();
    update_audio(dt);
    if (capture_mode && (capture_time += dt) > (capture_kind == "magic"        ? 2.5f
                                                : capture_kind == "throw"      ? 5.f
                                                : capture_kind == "crystal"    ? 10.f
                                                : capture_kind == "depth-tour" ? 12.f
                                                : capture_kind == "school"     ? 20.f
                                                : capture_kind == "animation"  ? 16.5f
                                                : capture_kind == "rig"        ? 18.f
                                                : capture_kind == "bumps"      ? 8.f
                                                : capture_kind == "wave"       ? 4.55f
                                                                               : 3.f)) {
        auto image = get_viewport()->get_texture()->get_image();
        String path = "/tmp/sabrina-" + capture_kind + ".png";
        image->save_png(path);
        UtilityFunctions::print("CAPTURE: ", path,
                                " FPS: ", Engine::get_singleton()->get_frames_per_second());
        get_tree()->quit();
    }
}
void ReefGame::load_settings() {
    Ref<ConfigFile> c;
    c.instantiate();
    c->load("user://settings.cfg");
    language = int(c->get_value("game", "language", 0));
    language = std::clamp(language, 0, 2);
    best = c->get_value("game", "best", 0);
    fullscreen = c->get_value("video", "fullscreen", true);
    quality = std::clamp(int(c->get_value("video", "quality", 1)), 0, 2);
    reduced_motion = c->get_value("video", "reduced_motion", false);
    music_gain = c->get_value("audio", "music", .3);
    effects_gain = c->get_value("audio", "effects", .55);
    voice_gain = c->get_value("audio", "voice", .9);
    game_settings.gentle = c->get_value("game", "gentle", true);
    game_settings.rest_seconds = std::clamp(float(c->get_value("game", "rest", 55)), 25.f, 150.f);
    game_settings.visitor_limit = std::clamp(int(c->get_value("game", "visitors", 2)), 1, 3);
}
void ReefGame::save_settings() {
    if (capture_mode || smoke_mode || verify_mode)
        return;
    Ref<ConfigFile> c;
    c.instantiate();
    c->set_value("game", "language", language);
    c->set_value("game", "best", best);
    c->set_value("game", "gentle", game_settings.gentle);
    c->set_value("game", "rest", game_settings.rest_seconds);
    c->set_value("game", "visitors", game_settings.visitor_limit);
    c->set_value("video", "fullscreen", fullscreen);
    c->set_value("video", "quality", quality);
    c->set_value("video", "reduced_motion", reduced_motion);
    c->set_value("audio", "music", music_gain);
    c->set_value("audio", "effects", effects_gain);
    c->set_value("audio", "voice", voice_gain);
    c->save("user://settings.cfg");
}
void ReefGame::toggle_fullscreen() {
    fullscreen = !fullscreen;
    if (!headless && !verify_mode && !capture_mode)
        DisplayServer::get_singleton()->window_set_mode(fullscreen ? DisplayServer::WINDOW_MODE_FULLSCREEN
                                                                   : DisplayServer::WINDOW_MODE_WINDOWED);
    refresh_text();
    save_settings();
}
void ReefGame::cycle_quality() {
    quality = (quality + 1) % 3;
    apply_graphics();
    refresh_text();
}
void ReefGame::toggle_motion() {
    reduced_motion = !reduced_motion;
    apply_graphics();
    refresh_text();
}
void ReefGame::toggle_gentle() {
    game_settings.gentle = !game_settings.gentle;
    sim.settings.gentle = game_settings.gentle;
    refresh_text();
}
void ReefGame::cycle_visitors() {
    game_settings.visitor_limit = game_settings.visitor_limit % 3 + 1;
    sim.settings.visitor_limit = game_settings.visitor_limit;
    refresh_text();
}
void ReefGame::set_music_volume(double v) {
    music_gain = v / 100;
}
void ReefGame::set_effects_volume(double v) {
    effects_gain = v / 100;
}
void ReefGame::set_voice_volume(double v) {
    voice_gain = v / 100;
}
void ReefGame::set_rest_time(double v) {
    game_settings.rest_seconds = v;
    sim.settings.rest_seconds = v;
    refresh_text();
}
