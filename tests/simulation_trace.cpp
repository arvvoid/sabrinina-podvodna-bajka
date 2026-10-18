// SPDX-License-Identifier: MIT
// Semantic trace for before/after comparisons on the same compiler/platform.
// Excludes entity generations: restarting invalidates handles without changing play.
#include "simulation/simulation.h"
#include <cstring>
#include <iostream>
using namespace reef;
uint64_t hash_value = 14695981039346656037ull;
void value(float x) {
    uint32_t v;
    std::memcpy(&v, &x, 4);
    hash_value = (hash_value ^ v) * 1099511628211ull;
}
void value(V2 x) {
    value(x.x);
    value(x.y);
}
int main() {
    for (int scenario = 0; scenario < 4; ++scenario) {
        World s;
        Settings cfg;
        cfg.aquarium = scenario == 0;
        cfg.gentle = scenario != 2;
        cfg.visitor_limit = 3;
        cfg.rest_seconds = 25;
        reef::reset(s, 27 + scenario, cfg);
        hash_value = 14695981039346656037ull;
        for (int tick = 0; tick < 18000; ++tick) {
            Input input;
            if (scenario) {
                input.move = {std::cos(tick * .003f), std::sin(tick * .007f)};
                input.cast = tick % 10000 == 30;
                input.toggle_song = tick % 1700 == 120;
            }
            if (scenario == 3 && tick % 1200 == 0) {
                reef::begin_visit(s);
                s.jelly[0].motion.pos = s.sabrina.motion.pos;
                s.sabrina.vitality.invulnerable = 0;
            }
            auto e = reef::step(s, 1.f / 60, input);
            value(s.sabrina.motion.pos);
            value(s.sabrina.motion.velocity);
            value(s.session.time);
            value(s.sabrina.vitality.health);
            value(s.sabrina.vitality.heal_delay);
            value(s.sabrina.vitality.hurt_flash);
            value(s.sabrina.vitality.stunned);
            value(s.sabrina.vitality.invulnerable);
            value(s.sabrina.cooldown);
            value(s.sabrina.song.energy);
            value(s.sabrina.song.strength);
            value(s.sabrina.song.active);
            value(s.sabrina.song.locked);
            value(s.sabrina.wave.origin);
            value(s.sabrina.wave.direction);
            value(s.sabrina.wave.time);
            value(s.sabrina.wave.cooldown);
            value(s.sabrina.wave.active);
            value(s.session.spawn_in);
            value(s.session.rest_remaining);
            value(s.session.visit_time);
            value(s.points.score);
            value(s.points.total_score);
            value(s.session.cycle);
            value(s.points.total_eaten);
            value(s.session.game_over);
            value(s.crystal.motion.pos);
            value(s.crystal.motion.direction);
            value(s.crystal.magic.time);
            value(int(s.crystal.magic.state));
            value(s.crystal.magic.target.slot);
            for (auto &f : s.fish) {
                value(f.motion.pos);
                value(f.motion.velocity);
                value(f.motion.direction);
                value(f.rescue.teleport_from);
                value(f.rescue.teleport_to);
                value(f.appearance.size);
                value(f.appearance.depth);
                value(f.swimming.phase);
                value(f.swimming.turn_in);
                value(f.appearance.birth);
                value(f.rescue.shield);
                value(f.rescue.teleport);
                value(f.swimming.cruise_heading);
                value(f.swimming.speed);
                value(f.swimming.tail_phase);
                value(f.life.pop_time);
                value(f.appearance.species);
                value(f.rescue.rescued_cycle);
                value(f.rescue.wave_rescued_cycle);
                value(f.life.active);
            }
            for (auto &j : s.jelly) {
                value(j.motion.pos);
                value(j.motion.velocity);
                value(j.motion.direction);
                value(int(j.behaviour.state));
                value(j.behaviour.time);
                value(j.behaviour.hunger);
                value(j.behaviour.turn_in);
                value(j.behaviour.eat_wait);
                value(j.behaviour.phase);
                value(j.behaviour.eaten);
                value(j.behaviour.kind);
                value(j.behaviour.exit_side);
            }
            value(e.spawned);
            value(e.teleported);
            value(e.eaten);
            value(e.repelled);
            value(e.wave_cast);
            value(e.song_empty);
            value(e.game_over);
            value(e.cast);
            value(e.launch);
            value(e.hit);
            value(e.visit_started);
            value(e.visit_ended);
            value(e.leaving);
        }
        std::cout << scenario << ' ' << hash_value << ' ' << reef::alive(s) << ' ' << s.points.total_score
                  << ' ' << s.sabrina.vitality.health << '\n';
    }
}
