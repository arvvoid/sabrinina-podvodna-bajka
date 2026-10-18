// SPDX-License-Identifier: Apache-2.0
#include "common.h"
#include "simulation/contacts.h"
#include "systems.h"
namespace reef {
CrystalOrbit crystal_orbit(const World &w, float seconds_behind) {
    float phase = (w.session.time - seconds_behind) * .9f * (w.settings.reduced_motion ? .25f : 1.f);
    return {{1.2f * std::cos(phase), .85f + .28f * std::sin(phase * 2)}, .7f + .85f * std::sin(phase)};
}
void touch_fish(World &w, FishEntity &f, Events &e) {
    if (!f.life.active || f.rescue.shield > 0)
        return;
    f.rescue.teleport_from = f.motion.pos;
    f.rescue.teleport_to = bounds(w.sabrina.motion.pos + V2{random(w, -2.2, 2.2), random(w, -1.4, 1.4)});
    f.rescue.teleport = 1.2;
    f.rescue.shield = 4;
    ++e.teleported;
    if (w.session.water == Water::Visit && f.rescue.rescued_cycle != w.session.cycle) {
        f.rescue.rescued_cycle = w.session.cycle;
        w.points.score += 80;
        ++w.points.rescues;
        ++w.points.visit_rescues;
    }
}
void systems::crystal(World &w, float dt, Input input, Events &e) {
    if (input.cast && w.sabrina.cooldown <= 0 && w.sabrina.vitality.stunned <= 0 &&
        w.crystal.magic.state == Spell::Resting && !w.sabrina.wave.active) {
        w.crystal.magic.state = Spell::Charging;
        w.crystal.magic.time = 0;
        w.sabrina.throw_time = 0;
        auto orbit = crystal_orbit(w);
        w.crystal.magic.charge_offset = orbit.offset;
        w.crystal.magic.charge_depth = orbit.depth;
        w.crystal.magic.target = no_entity;
        float closest = 1000;
        for (const auto &j : w.jelly)
            if (j.behaviour.state == JellyState::Hungry || j.behaviour.state == JellyState::Arriving) {
                float distance = (j.motion.pos - w.sabrina.motion.pos).length();
                if (distance < closest) {
                    closest = distance;
                    w.crystal.magic.target = j.id;
                }
            }
        e.cast = true;
    }
    w.crystal.magic.time += dt;
    if (w.crystal.magic.state == Spell::Charging)
        w.sabrina.throw_time = w.crystal.magic.time;
    if (w.crystal.magic.state == Spell::Resting) {
        w.crystal.motion.pos = w.sabrina.motion.pos + crystal_orbit(w).offset;
        w.crystal.magic.depth = crystal_orbit(w).depth;
    } else if (w.crystal.magic.state == Spell::Charging) {
        float gather = std::clamp(w.crystal.magic.time / .4f, 0.f, 1.f);
        gather = gather * gather * (3 - 2 * gather);
        auto palm = sabrina_palm(w.sabrina);
        w.crystal.motion.pos =
            (w.sabrina.motion.pos + w.crystal.magic.charge_offset) * (1 - gather) + palm.offset * gather;
        w.crystal.magic.depth = w.crystal.magic.charge_depth * (1 - gather) + palm.depth * gather;
        for (auto &f : w.fish)
            if (f.life.active &&
                (f.motion.pos - w.crystal.motion.pos).length() < .7f + w.crystal.magic.time * .8f)
                touch_fish(w, f, e);
        if (w.crystal.magic.time >= 1.1f) {
            const auto *target = find_jelly(w, w.crystal.magic.target);
            w.crystal.motion.direction =
                target ? (target->motion.pos - w.crystal.motion.pos).unit()
                       : V2{w.sabrina.swimming.facing * std::cos(w.sabrina.swimming.pitch),
                            std::sin(w.sabrina.swimming.pitch)};
            w.crystal.magic.state = Spell::Flying;
            w.crystal.magic.time = 0;
            e.launch = true;
            w.sabrina.cooldown = recharge_duration(w);
        }
    } else if (w.crystal.magic.state == Spell::Flying) {
        w.crystal.magic.depth += (1.f - w.crystal.magic.depth) * std::min(1.f, dt * 8);
        if (const auto *target = find_jelly(w, w.crystal.magic.target)) {
            if (target->behaviour.state == JellyState::Hungry ||
                target->behaviour.state == JellyState::Arriving)
                w.crystal.motion.direction = (target->motion.pos - w.crystal.motion.pos).unit();
        }
        V2 from = w.crystal.motion.pos;
        w.crystal.motion.pos = w.crystal.motion.pos + w.crystal.motion.direction * dt * 12;
        for (auto &f : w.fish)
            if (f.life.active && (f.motion.pos - w.crystal.motion.pos).length() < 1.2f)
                touch_fish(w, f, e);
        for (auto &j : w.jelly) {
            if (j.behaviour.state != JellyState::Hungry && j.behaviour.state != JellyState::Arriving)
                continue;
            bool hit = false;
            for (auto surface : jelly_body(j))
                hit = hit || contact_gap({from, w.crystal.motion.pos, .30f}, surface) <= 0;
            if (hit) {
                send_home(j);
                j.behaviour.repel_flash = .65f;
                ++e.repelled;
                w.points.score += 300;
                w.crystal.magic.state = Spell::Burst;
                w.crystal.magic.time = 0;
                break; // One relic repels one visitor; overlapping visitors cannot multiply its reward.
            }
        }
        if (std::abs(w.crystal.motion.pos.x) > 17 || w.crystal.motion.pos.y < .5f ||
            w.crystal.motion.pos.y > 18 || w.crystal.magic.time > 4) {
            w.crystal.magic.state = Spell::Burst;
            w.crystal.magic.time = 0;
        }
    } else if (w.crystal.magic.time > .7f) {
        w.crystal.magic.state = Spell::Resting;
        w.crystal.magic.time = 0;
    }
}
} // namespace reef
