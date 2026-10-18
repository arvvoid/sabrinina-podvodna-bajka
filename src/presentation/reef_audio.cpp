// SPDX-License-Identifier: Apache-2.0
#include "reef_game.h"
#include <algorithm>
#include <cmath>
#include <godot_cpp/classes/audio_stream_ogg_vorbis.hpp>
#include <godot_cpp/classes/audio_stream_wav.hpp>
using namespace godot;
Ref<AudioStream> ReefGame::load_audio(const String &name) {
    return AudioStreamOggVorbis::load_from_file("res://assets/audio/" + name + ".ogg");
}
Ref<AudioStream> ReefGame::compose_music(bool visit) {
    constexpr int rate = 22050;
    const float beat = visit ? .45f : .65f;
    const int length = int(rate * beat * 64);
    PackedByteArray bytes;
    bytes.resize(length * 2);
    auto *p = bytes.ptrw();
    const int melody[32] = {76, 0, 79, 0, 81, 79, 76, 0, 74, 0, 72, 0,  74, 76, 79, 0,
                            76, 0, 72, 0, 69, 72, 76, 0, 74, 0, 76, 79, 74, 0,  72, 0};
    const int roots[4] = {48, 45, 41, 43};
    for (int n = 0; n < length; ++n) {
        float t = float(n) / rate;
        int step = int(t / beat), bar = step / 16;
        float local = t - step * beat;
        float sample = 0;
        int note = melody[step % 32];
        if (note) {
            float hz = 440 * std::pow(2.f, (note - 69) / 12.f);
            float envelope = std::min(1.f, local / .015f) * std::exp(-local * 5);
            sample += .25f * envelope *
                      (std::sin(6.283185f * hz * local) + .2f * std::sin(6.283185f * hz * 2 * local));
        }
        float btime = t - bar * 16 * beat;
        float fade = std::min({1.f, btime / .6f, (16 * beat - btime) / .8f});
        for (int offset : {0, 7, 16}) {
            float hz = 440 * std::pow(2.f, (roots[bar % 4] + offset - 69) / 12.f);
            sample += .055f * fade * std::sin(6.283185f * hz * btime);
        }
        if (visit)
            sample += .025f * std::sin(local * 6.283185f * 110) * std::exp(-local * 18);
        int16_t v = int16_t(std::clamp(sample, -1.f, 1.f) * 22000);
        p[n * 2] = v & 255;
        p[n * 2 + 1] = (uint16_t(v) >> 8) & 255;
    }
    Ref<AudioStreamWAV> w;
    w.instantiate();
    w->set_format(AudioStreamWAV::FORMAT_16_BITS);
    w->set_mix_rate(rate);
    w->set_data(bytes);
    w->set_loop_mode(AudioStreamWAV::LOOP_FORWARD);
    w->set_loop_begin(0);
    w->set_loop_end(length);
    return w;
}
void ReefGame::build_audio() {
    for (auto **p : {&effects, &voice, &music, &visit_music, &song}) {
        *p = memnew(AudioStreamPlayer);
        add_child(*p);
    }
    if (headless || verify_mode || smoke_mode)
        return;
    music->set_stream(compose_music(false));
    visit_music->set_stream(compose_music(true));
    song->set_stream(load_audio("siren_song"));
    music->set_volume_linear(0);
    visit_music->set_volume_linear(0);
    music->play();
    visit_music->play();
}
void ReefGame::play_effect(const String &name, float pitch) {
    if (headless || verify_mode || smoke_mode)
        return;
    effects->set_pitch_scale(pitch);
    effects->set_stream(load_audio(name));
    effects->play();
}
void ReefGame::play_voice(const String &name, const String &text, bool interrupt) {
    if (headless || verify_mode || smoke_mode || (!interrupt && voice->is_playing()))
        return;
    voice->set_stream(load_audio(name));
    voice->play();
    caption->set_text(text);
}
void ReefGame::update_audio(float dt) {
    if (headless || verify_mode || smoke_mode)
        return;
    float duck = (voice->is_playing() ? .25f : 1.f) * (1 - sim.sabrina.song.strength * .60f);
    auto volume = [dt](AudioStreamPlayer *p, float target) {
        float v = p->get_volume_linear();
        p->set_volume_linear(v + (target - v) * std::min(1.f, dt * 2));
    };
    bool visit = started && !sim.session.game_over && sim.session.water == reef::Water::Visit;
    volume(music, music_gain * duck * (visit ? .12f : 1));
    volume(visit_music, music_gain * duck * (visit ? 1 : 0));
    effects->set_volume_linear(effects_gain);
    voice->set_volume_linear(voice_gain);
    song->set_volume_linear(effects_gain * .65f * sim.sabrina.song.strength);
    if (sim.sabrina.song.strength > .01f && started && !paused) {
        if (!song->is_playing())
            song->play();
    } else
        song->stop();
    if (!voice->is_playing())
        caption->set_text("");
}
