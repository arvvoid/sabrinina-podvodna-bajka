// SPDX-License-Identifier: MIT
// Repeatable bot scenarios, not a substitute for child playtesting.
#include "simulation/simulation.h"
#include <algorithm>
#include <cstdlib>
#include <iostream>
using namespace reef;
int main(int argc, char **argv) {
    int selected = argc > 1 ? std::atoi(argv[1]) : -1;
    for (bool gentle : {true, false}) {
        for (int policy = 0; policy < 5; ++policy) {
            if (selected >= 0 && policy != selected)
                continue;
            double eaten = 0, health = 0, score = 0, visits = 0, dead = 0, stings = 0;
            for (unsigned seed = 1; seed <= 24; ++seed) {
                World w;
                Settings cfg;
                cfg.gentle = gentle;
                cfg.visitor_limit = 2;
                reset(w, seed, cfg);
                for (int tick = 0; tick < 60 * 300 && !w.session.game_over; ++tick) {
                    Input in;
                    // All active policies use the same slow oval patrol and flee close visitors.
                    if (policy) {
                        float t = w.session.time;
                        V2 goal{8 * std::sin(t * .12f), 9 + 3 * std::sin(t * .21f)};
                        V2 steering = (goal - w.sabrina.motion.pos) * .25f;
                        for (const auto &j : w.jelly)
                            if (j.behaviour.state == JellyState::Hungry) {
                                V2 away = w.sabrina.motion.pos - j.motion.pos;
                                if (away.length() < 5)
                                    steering = steering + away.unit() * (5 - away.length());
                            }
                        in.move = steering.unit() * std::min(.65f, steering.length());
                        bool danger = w.session.water == Water::Visit;
                        if (policy == 2 || policy == 4) {
                            in.toggle_song =
                                (!w.sabrina.song.active && !w.sabrina.song.locked &&
                                 w.sabrina.song.energy >= 90 && danger) ||
                                (w.sabrina.song.active && (w.sabrina.song.energy < 25 || !danger));
                        }
                        if (policy == 3 || policy == 4)
                            in.wave = danger;
                        if (policy == 4)
                            in.cast = danger && w.session.visit_time > 15;
                    }
                    auto e = step(w, 1.f / 60, in);
                    stings += e.hit;
                }
                eaten += w.points.total_eaten;
                health += w.sabrina.vitality.health;
                score += w.points.total_score;
                visits += w.session.cycle;
                dead += w.session.game_over;
            }
            const char *names[] = {"idle", "patrol", "song", "wave", "all"};
            std::cout << (gentle ? "gentle" : "lively") << ',' << names[policy] << ",eaten=" << eaten / 24
                      << ",health=" << health / 24 << ",score=" << score / 24 << ",visits=" << visits / 24
                      << ",deaths=" << dead << ",stings=" << stings / 24 << std::endl;
        }
    }
    // Isolated control: a stationary singer should collect a nearby school promptly.
    for (bool singing : {false, true}) {
        double near = 0;
        for (unsigned seed = 1; seed <= 24; ++seed) {
            World w;
            Settings cfg;
            cfg.aquarium = true;
            reset(w, seed, cfg);
            w.session.spawn_in = 1000;
            w.sabrina.motion.pos = {0, 9};
            for (auto &f : w.fish)
                f.life.active = false;
            add_fish(w, 12, false);
            for (int k = 0; k < 12; ++k) {
                auto &f = w.fish[k];
                float a = k * 6.2831853f / 12;
                f.motion.pos = {std::cos(a) * 4.5f, 9 + std::sin(a) * 4.5f};
                f.appearance.birth = 1;
            }
            for (int tick = 0; tick < 360; ++tick) {
                Input in;
                in.toggle_song = singing && tick == 0;
                step(w, 1.f / 60, in);
            }
            for (auto &f : w.fish)
                if (f.life.active && (f.motion.pos - w.sabrina.motion.pos).length() < 3.5f)
                    ++near;
        }
        std::cout << "gather," << singing << ",near=" << near / 24 << "/12\n";
    }
}
