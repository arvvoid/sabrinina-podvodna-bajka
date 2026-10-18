// SPDX-License-Identifier: MIT
#include "common.h"
#include "systems.h"
namespace reef {
void systems::fish_swimming(World &w, float dt) {
    for (auto &f : w.fish) {
        if (!f.life.active)
            continue;
        f.rescue.wave_flow = std::max(0.f, f.rescue.wave_flow - dt);
        f.contact.impulse = f.contact.impulse * std::exp(-(f.rescue.wave_flow > 0 ? 1.25f : 3.5f) * dt);
        f.contact.turn_time = std::max(0.f, f.contact.turn_time - dt);
        f.appearance.birth = std::min(1.f, f.appearance.birth + dt);
        f.rescue.shield = std::max(0.f, f.rescue.shield - dt);
        if (f.rescue.teleport > 0) {
            f.rescue.teleport = std::max(0.f, f.rescue.teleport - dt);
            float u = 1 - f.rescue.teleport / 1.2f;
            u = u * u * (3 - 2 * u);
            f.motion.pos = f.rescue.teleport_from * (1 - u) + f.rescue.teleport_to * u;
            continue;
        }
        // Cruise in shallow arcs. Turn the heading before accelerating forward;
        // never add a sideways velocity that makes a fish drift like a balloon.
        f.swimming.phase += dt * (.85f + .18f * f.appearance.size);
        f.swimming.turn_in -= dt;
        if (f.swimming.turn_in <= 0) {
            f.swimming.cruise_heading = (f.motion.direction.x < 0 ? 3.14159f : 0.f) + random(w, -.32f, .32f);
            f.swimming.turn_in = random(w, 4, 9);
        }
        if (f.motion.pos.x > 12.3f && std::cos(f.swimming.cruise_heading) > 0) {
            f.swimming.cruise_heading = 3.14159f + random(w, -.25f, .25f);
            f.swimming.turn_in = 6;
        }
        if (f.motion.pos.x < -12.3f && std::cos(f.swimming.cruise_heading) < 0) {
            f.swimming.cruise_heading = random(w, -.25f, .25f);
            f.swimming.turn_in = 6;
        }
        float cruise = .85f + f.appearance.size * .70f;
        float desired_heading = f.swimming.cruise_heading + .12f * std::sin(f.swimming.phase);
        V2 want = {std::cos(desired_heading) * cruise, std::sin(desired_heading) * cruise};
        // Start a broad turn well before reaching the glass.
        if (f.motion.pos.x > 12.3f)
            want.x -= std::min(6.f, (f.motion.pos.x - 12.3f) * 2.5f);
        if (f.motion.pos.x < -12.3f)
            want.x += std::min(6.f, (-12.3f - f.motion.pos.x) * 2.5f);
        if (f.motion.pos.y > 14.3f)
            want.y -= (f.motion.pos.y - 14.3f) * 2;
        if (f.motion.pos.y < 3.8f)
            want.y += (3.8f - f.motion.pos.y) * 2;
        V2 separation{};
        int neighbours = 0;
        V2 school{};
        for (const auto &other : w.fish) {
            if (&f == &other || !other.life.active)
                continue;
            float d = (f.motion.pos - other.motion.pos).length();
            if (d < .7f && d > .01f)
                separation = separation + (f.motion.pos - other.motion.pos).unit() * (.7f - d);
            if (other.appearance.species == f.appearance.species && d < 3) {
                school = school + other.motion.velocity;
                ++neighbours;
            }
        }
        want = want + separation * .8f;
        if (neighbours > 0)
            want = want * .8f + school * (.2f / neighbours);
        for (const auto &j : w.jelly)
            if (j.behaviour.state == JellyState::Hungry) {
                V2 away = f.motion.pos - j.motion.pos;
                float d = away.length();
                if (d < 3.4)
                    want = want + away.unit() * (3.4f - d) * .8f;
            }
        V2 away = f.motion.pos - w.sabrina.motion.pos;
        bool hearing = w.sabrina.song.active && w.sabrina.song.strength > .05f &&
                       away.length() < song_radius(w) && f.rescue.wave_flow <= 0;
        if (hearing) {
            // The song gathers a loose, swaying school behind Sabrina rather
            // than pulling every fish onto the same point.
            V2 wake = w.sabrina.motion.velocity.length() > .4f
                          ? w.sabrina.motion.velocity.unit() * -std::min(1.4f, song_radius(w) * .22f)
                          : V2{};
            float angle = f.swimming.phase + w.session.time * .22f;
            V2 slot =
                w.sabrina.motion.pos + wake +
                V2{std::cos(angle) * std::min(song_radius(w) * .40f, 1.5f + .18f * f.appearance.species),
                   std::sin(angle) * std::min(song_radius(w) * .30f, 1.f + .12f * f.appearance.species)};
            V2 pull = slot - f.motion.pos;
            float pulse = .85f + .15f * std::sin(w.session.time * 2.2f + f.swimming.phase);
            want = want * (1 - w.sabrina.song.strength * .85f) +
                   (pull.unit() * std::min(3.8f, pull.length() * 2.f) * pulse +
                    w.sabrina.motion.velocity * 1.05f) *
                       w.sabrina.song.strength;
        } else if (!w.sabrina.song.active && hungry(w) == 0 && away.length() < 1.5f) {
            V2 push = away.unit();
            float nearest = 1000;
            for (const auto &j : w.jelly)
                if (j.behaviour.state == JellyState::Hungry &&
                    (j.motion.pos - f.motion.pos).length() < nearest) {
                    nearest = (j.motion.pos - f.motion.pos).length();
                    push = (f.motion.pos - j.motion.pos).unit();
                }
            want = want + push * 2.5f;
        }
        // Reward an actual escort from danger to clear water, once per fish/visit.
        float danger = 1000;
        for (const auto &j : w.jelly)
            if (j.behaviour.state == JellyState::Hungry || j.behaviour.state == JellyState::Arriving)
                danger = std::min(danger, (f.motion.pos - j.motion.pos).length());
        f.rescue.song_escort = hearing ? f.rescue.song_escort + dt : 0;
        if (hearing && danger < 5 && w.session.water == Water::Visit)
            f.rescue.song_risk_cycle = w.session.cycle;
        if (hearing && danger >= 6 && f.rescue.song_escort >= 1 && w.session.water == Water::Visit &&
            f.rescue.song_risk_cycle == w.session.cycle && f.rescue.song_rescued_cycle != w.session.cycle) {
            f.rescue.song_rescued_cycle = w.session.cycle;
            w.points.score += 30;
        }
        float heading = std::atan2(f.motion.direction.y, f.motion.direction.x);
        float target = std::atan2(want.y, want.x);
        float change = std::atan2(std::sin(target - heading), std::cos(target - heading));
        float turn_rate = 1.35f + (hearing ? 1.85f : 0.f) + (f.contact.turn_time > 0 ? 1.1f : 0);
        heading += std::clamp(change, -turn_rate * dt, turn_rate * dt);
        f.motion.direction = {std::cos(heading), std::sin(heading)};
        float desired_speed = std::clamp(want.length(), .55f, hearing ? 6.8f : 3.8f);
        // Slow down into a turn, then accelerate out with faster tail beats.
        desired_speed *= 1.f - .48f * std::min(1.f, std::abs(change) / 1.5f);
        f.swimming.speed += (desired_speed - f.swimming.speed) * std::min(1.f, dt * (hearing ? 3.5f : 1.8f));
        f.swimming.tail_phase += dt * (6.f + f.swimming.speed * 5.f);
        float thrust = 1.f + .08f * std::sin(f.swimming.tail_phase * 2);
        f.motion.velocity = f.motion.direction * (f.swimming.speed * thrust) + f.contact.impulse;
        f.motion.pos = bounds(f.motion.pos + f.motion.velocity * dt);
    }
}
} // namespace reef
