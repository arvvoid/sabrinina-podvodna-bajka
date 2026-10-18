// SPDX-License-Identifier: MIT
#include "common.h"
#include "systems.h"
namespace reef {
void begin_visit(World &w) {
    if (w.settings.aquarium)
        return;
    w.session.water = Water::Visit;
    ++w.session.cycle;
    w.points.score = 0;
    w.points.visit_rescues = 0;
    w.session.visit_time = 0;
    w.points.visit_start_fish = alive(w);
    int count = 1;
    if (w.session.cycle >= 3 && w.settings.visitor_limit > 1 &&
        (w.session.cycle % 3 == 0 || random(w, 0, 1) < .25f))
        count = 2;
    if (w.session.cycle >= 6 && w.settings.visitor_limit > 2 && w.session.cycle % 4 == 0)
        count = 3;
    for (int i = 0; i < jelly_count; ++i) {
        w.jelly[i] = JellyEntity{};
        if (i >= count)
            continue;
        auto &j = w.jelly[i];
        j.id = allocate_entity(w, EntityKind::Jelly, i);
        j.behaviour.state = JellyState::Arriving;
        j.behaviour.exit_side = i % 2 == 0 ? 1 : -1;
        j.motion.pos = {j.behaviour.exit_side * 18.f, random(w, 5, 14)};
        j.motion.direction = {float(-j.behaviour.exit_side), random(w, -.2, .2)};
        j.behaviour.hunger = w.settings.gentle ? 90 : 120;
        j.behaviour.kind = (w.session.cycle + i) % 3;
        j.behaviour.phase = random(w, 0, 6.28);
        j.behaviour.turn_in = 4;
    }
}
void send_home(JellyEntity &j) {
    j.behaviour.state = JellyState::Leaving;
    j.behaviour.time = 0;
    j.behaviour.exit_side = j.motion.pos.x < 0 ? -1 : 1;
}
void finish_visit(World &w, Events &e) {
    w.points.last_survivors = alive(w);
    std::array<bool, species_count> kinds{};
    for (const auto &f : w.fish)
        if (f.life.active)
            kinds[f.appearance.species] = true;
    w.points.last_variety = 0;
    for (bool k : kinds)
        w.points.last_variety += k;
    w.points.score +=
        w.points.last_survivors * (75 + 5 * std::min(w.session.cycle, 8)) + w.points.last_variety * 60;
    w.points.last_score = w.points.score;
    w.points.total_score += w.points.score;
    w.points.best_score = std::max(w.points.best_score, w.points.score);
    w.session.water = Water::Calm;
    w.session.rest_remaining = w.settings.rest_seconds * random(w, .85, 1.2);
    e.visit_ended = true;
}
void systems::population_and_visits(World &w, float dt, Events &e) {
    const bool visitors_present = std::any_of(w.jelly.begin(), w.jelly.end(), [](const auto &j) {
        return j.behaviour.state != JellyState::Hidden;
    });
    // Freeze the population clock through arrival, feeding and the full departure.
    // Even an empty aquarium waits until the final visitor is off screen.
    if (!visitors_present) {
        w.session.spawn_in -= dt;
        if (w.session.spawn_in <= 0) {
            e.spawned = add_fish(w, int(random(w, 1, 4.99)));
            w.session.spawn_in = random(w, 9, 18);
        }
        if (alive(w) == 0) {
            e.spawned += add_fish(w, 3);
            w.session.spawn_in = 5;
        }
    }
    if (w.settings.aquarium) {
        if (w.session.water == Water::Visit) {
            w.session.water = Water::Calm;
            w.session.rest_remaining = w.settings.rest_seconds;
            for (auto &j : w.jelly)
                if (j.behaviour.state != JellyState::Hidden)
                    send_home(j);
        }
    } else if (w.session.water == Water::Calm) {
        w.session.rest_remaining -= dt;
        if (w.session.rest_remaining <= 0) {
            if (alive(w) >= 10) {
                begin_visit(w);
                e.visit_started = true;
            } else
                w.session.rest_remaining = 5;
        }
    } else
        w.session.visit_time += dt;
}
void systems::recovery_and_scoring(World &w, float dt, Events &e) {
    if (!w.session.game_over && w.sabrina.vitality.heal_delay <= 0)
        w.sabrina.vitality.health =
            std::min(100.f, w.sabrina.vitality.health + dt * (w.settings.gentle ? 1.2f : .8f));
    if (!w.session.game_over && w.session.water == Water::Visit && hungry(w) == 0)
        finish_visit(w, e);
}
} // namespace reef
