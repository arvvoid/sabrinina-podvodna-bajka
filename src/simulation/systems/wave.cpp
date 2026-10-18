// SPDX-License-Identifier: MIT
#include "common.h"
#include "systems.h"
namespace reef {
void systems::water_wave(World &w, float dt, Input input, Events &e) {
    auto &wave = w.sabrina.wave;
    if (input.wave && wave.cooldown <= 0 && w.sabrina.vitality.stunned <= 0 && w.sabrina.throw_time < 0) {
        float pitch = w.sabrina.motion.velocity.length() > .6f ? w.sabrina.swimming.pitch : 0;
        float side = std::cos(w.sabrina.swimming.yaw);
        side = std::abs(side) < .2f ? w.sabrina.swimming.facing : (side < 0 ? -1.f : 1.f);
        wave.direction = V2{side * std::cos(pitch), std::sin(pitch)}.unit();
        wave.origin = w.sabrina.motion.pos + V2{0, .5f} + wave.direction * .75f;
        wave.active = true;
        wave.time = 0;
        ++wave.cast_id;
        wave.cooldown = wave_recharge_duration(w);
        e.wave_cast = true;
    }
    if (!wave.active)
        return;
    float previous = wave.time * 8.f;
    wave.time += dt;
    float front = std::min(5.5f, wave.time * 8.f);
    for (auto &fish : w.fish) {
        if (!fish.life.active || fish.rescue.teleport > 0 || fish.rescue.wave_hit == wave.cast_id)
            continue;
        V2 offset = fish.motion.pos - wave.origin;
        float ahead = offset.x * wave.direction.x + offset.y * wave.direction.y;
        float sideways = std::abs(offset.x * wave.direction.y - offset.y * wave.direction.x);
        float radius = fish.appearance.size * .45f;
        if (ahead < 0 || ahead < previous - .35f - radius || ahead > front + .35f + radius ||
            sideways > .65f + .38f * ahead + radius)
            continue;
        fish.rescue.wave_hit = wave.cast_id;
        // Water impulse, not teleportation: the normal drag/contact solver handles the aftermath.
        fish.contact.impulse = wave.direction * (5.8f - .15f * ahead);
        fish.contact.turn_time = 1.1f;
        fish.rescue.wave_flow = 1.1f;
        fish.motion.direction = (fish.motion.direction + wave.direction * 1.2f).unit();
        fish.swimming.cruise_heading = std::atan2(wave.direction.y, wave.direction.x);
        fish.swimming.turn_in = 1.5f;
        if (w.session.water == Water::Visit && fish.rescue.wave_rescued_cycle != w.session.cycle) {
            for (const auto &jelly : w.jelly) {
                if (jelly.behaviour.state != JellyState::Hungry &&
                    jelly.behaviour.state != JellyState::Arriving)
                    continue;
                V2 away = fish.motion.pos - jelly.motion.pos;
                if (away.length() < 6 && away.x * wave.direction.x + away.y * wave.direction.y > 0) {
                    fish.rescue.wave_rescued_cycle = w.session.cycle;
                    w.points.score += 20;
                    break;
                }
            }
        }
    }
    if (wave.time >= .75f)
        wave.active = false;
}
} // namespace reef
