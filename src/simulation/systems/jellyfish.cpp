// SPDX-License-Identifier: Apache-2.0
#include "common.h"
#include "simulation/contacts.h"
#include "systems.h"
namespace reef {
void systems::jellyfish_and_contacts(World &w, float dt, Events &e) {
    for (auto &j : w.jelly) {
        if (j.behaviour.state == JellyState::Hidden)
            continue;
        j.behaviour.time += dt;
        j.behaviour.repel_flash = std::max(0.f, j.behaviour.repel_flash - dt);
        j.behaviour.song_check = std::max(0.f, j.behaviour.song_check - dt);
        j.behaviour.song_interest = std::max(0.f, j.behaviour.song_interest - dt);
        if (!w.sabrina.song.active)
            j.behaviour.song_interest = 0;
        else if (j.behaviour.song_check <= 0 && j.behaviour.state != JellyState::Leaving) {
            // Original Scratch: 15 + 5*cycle percent, independent of distance.
            // Roll at bounded intervals, never every rendered frame; cap escalation for children.
            j.behaviour.song_check = 2.5f;
            if (random(w, 0, 1) < std::min(.50f, .15f + .05f * w.session.cycle))
                j.behaviour.song_interest = 3.5f;
        }
        j.behaviour.eat_wait = std::max(0.f, j.behaviour.eat_wait - dt);
        if (j.behaviour.state == JellyState::Arriving) {
            j.motion.pos = j.motion.pos + V2{float(-j.behaviour.exit_side) * 1.15f,
                                             std::sin(w.session.time + j.behaviour.phase) * .15f} *
                                              dt;
            if (j.behaviour.time > 4.5f) {
                j.behaviour.state = JellyState::Hungry;
                j.behaviour.time = 0;
            }
        } else if (j.behaviour.state == JellyState::Leaving) {
            j.motion.pos = j.motion.pos + V2{j.behaviour.exit_side * 1.9f, .2f} * dt;
            if (std::abs(j.motion.pos.x) > 19 || j.behaviour.time > 9)
                j.behaviour.state = JellyState::Hidden;
        } else {
            j.behaviour.hunger -= dt;
            j.behaviour.turn_in -= dt;
            if (j.behaviour.turn_in <= 0) {
                j.motion.direction = V2{random(w, -1, 1), random(w, -.65, .65)}.unit();
                j.behaviour.turn_in = random(w, 3, 6);
            }
            V2 wish = j.motion.direction * .75f;
            if (j.behaviour.kind != 0) {
                float nearest = 100;
                V2 target{};
                for (const auto &f : w.fish)
                    if (f.life.active && (f.motion.pos - j.motion.pos).length() < nearest) {
                        nearest = (f.motion.pos - j.motion.pos).length();
                        target = f.motion.pos;
                    }
                if (nearest < 5.5f)
                    wish = (target - j.motion.pos).unit();
            }
            if (w.sabrina.song.active && j.behaviour.song_interest > 0)
                wish = (w.sabrina.motion.pos - j.motion.pos).unit() * 1.1f;
            if (j.motion.pos.x > 14)
                wish.x = -1;
            if (j.motion.pos.x < -14)
                wish.x = 1;
            if (j.motion.pos.y > 15)
                wish.y = -.6;
            if (j.motion.pos.y < 3)
                wish.y = .6;
            wish.y += std::sin(w.session.time * 1.4f + j.behaviour.phase) * .3f;
            float speed = (w.settings.gentle ? .8f : 1.1f) + .04f * std::min(w.session.cycle, 6);
            j.motion.velocity =
                j.motion.velocity + (wish * speed - j.motion.velocity) * std::min(1.f, dt * 2);
            j.motion.pos = j.motion.pos + j.motion.velocity * dt;
        }
        if (j.behaviour.state == JellyState::Hungry || j.behaviour.state == JellyState::Arriving) {
            bool stinging = false;
            for (auto surface : jelly_body(j))
                for (auto part : sabrina_body(w.sabrina))
                    stinging = stinging || contact_gap(surface, part) <= 0;
            if (stinging) {
                // Resolve even during sting immunity: she cannot swim through a visitor.
                for (int pass = 0; pass < 6; ++pass) {
                    V2 correction{};
                    for (auto surface : jelly_body(j))
                        for (auto part : sabrina_body(w.sabrina)) {
                            V2 candidate = contact_displacement(part, surface);
                            if (candidate.length() > correction.length())
                                correction = candidate;
                        }
                    if (correction.length() < .001f)
                        break;
                    w.sabrina.motion.pos = bounds(w.sabrina.motion.pos + correction);
                    w.sabrina.motion.pos.y = std::max(3.2f, w.sabrina.motion.pos.y);
                }
                V2 normal = (w.sabrina.motion.pos - j.motion.pos).unit();
                float closing =
                    w.sabrina.motion.velocity.x * normal.x + w.sabrina.motion.velocity.y * normal.y;
                if (closing < 0)
                    w.sabrina.motion.velocity = w.sabrina.motion.velocity - normal * (1.15f * closing);
            }
            if (stinging && w.sabrina.vitality.invulnerable <= 0) {
                w.sabrina.vitality.stunned = w.settings.gentle ? 2.f : 3.f;
                w.sabrina.vitality.invulnerable = w.sabrina.vitality.stunned + 3;
                w.sabrina.song.active = false;
                w.sabrina.vitality.health =
                    std::max(0.f, w.sabrina.vitality.health - (w.settings.gentle ? 28.f : 35.f));
                w.sabrina.vitality.heal_delay = 8;
                w.sabrina.vitality.hurt_flash = .85f;
                w.points.score = std::max(0, w.points.score - 100);
                e.hit = true;
                if (w.sabrina.vitality.health <= 0) {
                    w.session.game_over = true;
                    e.game_over = true;
                    w.sabrina.motion.velocity = {};
                    w.sabrina.song.strength = 0;
                }
            }
            for (auto &f : w.fish)
                if (f.life.active && f.rescue.teleport <= 0) {
                    bool touching = false;
                    for (auto surface : jelly_body(j))
                        touching = touching || contact_gap(fish_body(f), surface) <= 0;
                    if (touching) {
                        // The red pop starts at the surface rather than inside/beyond the jellyfish.
                        for (int pass = 0; pass < 3; ++pass)
                            for (auto surface : jelly_body(j))
                                f.motion.pos = f.motion.pos + contact_displacement(fish_body(f), surface);
                        if (f.rescue.shield > 0) {
                            f.contact.impulse = (f.motion.pos - j.motion.pos).unit() * 3;
                            f.contact.turn_time = .8f;
                            continue;
                        }
                        f.life.active = false;
                        f.life.pop_time = .55f;
                        f.motion.velocity = {};
                        ++j.behaviour.eaten;
                        ++w.points.total_eaten;
                        ++e.eaten;
                        j.behaviour.eat_wait = .55f;
                    }
                }
            if (j.behaviour.hunger <= 0 ||
                (j.behaviour.eaten >= (w.settings.gentle ? 10 : 14) &&
                 w.session.visit_time >= (w.settings.gentle ? 60.f : 80.f)) ||
                w.session.visit_time > (w.settings.gentle ? 100.f : 130.f)) {
                send_home(j);
                w.points.score += 150;
                e.leaving = true;
            }
        }
    }
    // Visitors themselves also occupy space, including their hanging tentacles.
    for (int pass = 0; pass < 3; ++pass)
        for (int i = 0; i < jelly_count; ++i)
            for (int k = i + 1; k < jelly_count; ++k) {
                auto &a = w.jelly[i];
                auto &b = w.jelly[k];
                if (a.behaviour.state == JellyState::Hidden || b.behaviour.state == JellyState::Hidden)
                    continue;
                V2 correction{};
                for (auto ca : jelly_body(a))
                    for (auto cb : jelly_body(b)) {
                        auto candidate = contact_displacement(ca, cb);
                        if (candidate.length() > correction.length())
                            correction = candidate;
                    }
                a.motion.pos = a.motion.pos + correction * .5f;
                b.motion.pos = b.motion.pos - correction * .5f;
            }
}
} // namespace reef
