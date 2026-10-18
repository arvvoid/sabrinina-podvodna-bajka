// SPDX-License-Identifier: MIT
#include "common.h"
#include "systems.h"
namespace reef {
bool toggle_song(World &w) {
    if (w.sabrina.song.active) {
        w.sabrina.song.active = false;
        return true;
    }
    if (w.session.game_over || w.sabrina.song.locked || w.sabrina.song.energy <= 0 ||
        w.sabrina.vitality.stunned > 0)
        return false;
    w.sabrina.song.active = true;
    return true;
}

float recharge_duration(const World &w) {
    return w.settings.gentle ? 150.f : 180.f;
}
float song_duration(const World &w) {
    return w.settings.gentle ? 12.f : 10.f;
}
float song_recharge_duration(const World &w) {
    return w.settings.gentle ? 20.f : 24.f;
}
float wave_recharge_duration(const World &w) {
    return w.settings.gentle ? 3.f : 4.f;
}
float song_radius(const World &w) {
    return w.sabrina.song.active
               ? (1.4f + 5.1f * std::sqrt(w.sabrina.song.energy / 100.f)) * w.sabrina.song.strength
               : 0.f;
}
void systems::timers(World &w, float dt) {
    w.session.time += dt;
    if (w.sabrina.throw_time >= 0) {
        w.sabrina.throw_time += dt;
        if (w.sabrina.throw_time > 1.6f)
            w.sabrina.throw_time = -1;
    }
    w.sabrina.vitality.heal_delay = std::max(0.f, w.sabrina.vitality.heal_delay - dt);
    w.sabrina.vitality.hurt_flash = std::max(0.f, w.sabrina.vitality.hurt_flash - dt);
    for (auto &f : w.fish)
        f.life.pop_time = std::max(0.f, f.life.pop_time - dt);
    w.sabrina.cooldown = std::max(0.f, w.sabrina.cooldown - dt);
    w.sabrina.vitality.stunned = std::max(0.f, w.sabrina.vitality.stunned - dt);
    w.sabrina.vitality.invulnerable = std::max(0.f, w.sabrina.vitality.invulnerable - dt);
}
void systems::song(World &w, float dt, Input input, Events &e) {
    if (input.toggle_song)
        toggle_song(w);
    if (w.sabrina.song.active) {
        w.sabrina.song.energy = std::max(0.f, w.sabrina.song.energy - dt * 100 / song_duration(w));
        if (w.sabrina.song.energy <= .001f) {
            w.sabrina.song.energy = 0;
            w.sabrina.song.active = false;
            w.sabrina.song.locked = true;
            e.song_empty = true;
        }
    } else {
        w.sabrina.song.energy = std::min(100.f, w.sabrina.song.energy + dt * 100 / song_recharge_duration(w));
        if (w.sabrina.song.energy >= 99.999f) {
            w.sabrina.song.energy = 100;
            w.sabrina.song.locked = false;
        }
    }
}
void systems::player_swimming(World &w, float dt, Input input) {
    w.sabrina.wave.cooldown = std::max(0.f, w.sabrina.wave.cooldown - dt);
    if (input.move.length() > 1)
        input.move = input.move.unit();
    // A small perpendicular stroke gives real swimming a sinusoidal path without
    // displacing a resting player or fighting precise steering near the pointer.
    float effort = input.move.length();
    w.sabrina.swimming.phase +=
        dt * (1.9634954f + effort * 2.3998277f) * (w.settings.reduced_motion ? .25f : 1.f);
    w.sabrina.song.strength =
        std::clamp(w.sabrina.song.strength +
                       dt * (w.sabrina.song.active && w.sabrina.vitality.stunned <= 0 ? 2.f : -1.2f),
                   0.f, 1.f);
    V2 normal{-input.move.y, input.move.x};
    V2 desired = w.sabrina.vitality.stunned > 0
                     ? V2{}
                     : input.move * (7.2f - w.sabrina.song.strength * 1.6f) +
                           normal * (std::cos(w.sabrina.swimming.phase) * 1.2f * effort);
    w.sabrina.motion.velocity =
        w.sabrina.motion.velocity + (desired - w.sabrina.motion.velocity) * std::min(1.f, dt * 9);
    w.sabrina.motion.pos = bounds(w.sabrina.motion.pos + w.sabrina.motion.velocity * dt);
    w.sabrina.motion.pos.y = std::max(3.2f, w.sabrina.motion.pos.y); // Keep the longer tail above the seabed.
    if (std::abs(w.sabrina.motion.velocity.x) > .25f)
        w.sabrina.swimming.facing = w.sabrina.motion.velocity.x < 0 ? -1 : 1;
    auto &pose = w.sabrina.swimming;
    float moving = std::clamp((w.sabrina.motion.velocity.length() - .3f) / 3.5f, 0.f, 1.f);
    float heading =
        std::atan2(w.sabrina.motion.velocity.y, std::max(.15f, std::abs(w.sabrina.motion.velocity.x)));
    if (w.crystal.magic.state == Spell::Charging) {
        if (const auto *target = find_jelly(w, w.crystal.magic.target)) {
            V2 aim = target->motion.pos - w.sabrina.motion.pos;
            if (std::abs(aim.x) > .1f)
                pose.facing = aim.x < 0 ? -1 : 1;
            heading = std::atan2(aim.y, std::max(.15f, std::abs(aim.x)));
            moving = 1;
        }
    }
    float target_pitch = .65f * (1 - moving) + std::clamp(heading, -1.1f, 1.1f) * moving;
    pose.pitch += (target_pitch - pose.pitch) * (1 - std::exp(-4.f * dt));
    float target_yaw = pose.facing < 0 ? -3.14159265f : 0;
    // Both turns pass through the camera-facing half-circle, without full spins.
    float turn = target_yaw - pose.yaw;
    pose.yaw += turn * (1 - std::exp(-4.5f * dt));
}
} // namespace reef
