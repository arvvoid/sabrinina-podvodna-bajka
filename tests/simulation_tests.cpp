// SPDX-License-Identifier: MIT
#include "simulation/contacts.h"
#include "simulation/sabrina_pose.h"
#include "simulation/simulation.h"
#include "simulation/systems/systems.h"
#include <algorithm>
#include <cstdlib>
#include <iostream>
using namespace reef;
void check(bool x, const char *m) {
    if (!x) {
        std::cerr << "FAILED: " << m << '\n';
        std::exit(1);
    }
}
void advance(World &s, float seconds, Input in = {}) {
    for (int i = 0; i < int(seconds * 60); ++i)
        reef::step(s, 1.f / 60, in);
}
int main() {
    World s;
    reef::reset(s);
    check(reef::alive(s) == 10, "ten original fish");
    check(reef::add_fish(s, 0) == 0, "zero spawn");
    advance(s, 20);
    check(s.session.water == Water::Calm && reef::alive(s) > 10, "quiet aquarium spawns fish");
    // All visible visitor states block both scheduled births and empty refills.
    for (auto state : {JellyState::Arriving, JellyState::Hungry, JellyState::Leaving}) {
        for (bool empty : {false, true}) {
            World population_world;
            reset(population_world);
            if (empty)
                for (auto &fish : population_world.fish)
                    fish.life.active = false;
            population_world.jelly[2].behaviour.state = state;
            population_world.session.spawn_in = 0;
            int before = alive(population_world);
            Events events;
            systems::population_and_visits(population_world, 1.f, events);
            check(events.spawned == 0 && alive(population_world) == before &&
                      population_world.session.spawn_in == 0,
                  "population stays frozen until every jellyfish has left, even when empty");
            population_world.jelly[2].behaviour.state = JellyState::Hidden;
            systems::population_and_visits(population_world, 1.f / 60, events);
            check(events.spawned > 0 && alive(population_world) > before,
                  "population resumes when the final visitor disappears");
        }
    }
    for (float yaw : {0.f, -1.4f, -3.14159265f})
        for (float pitch : {-1.1f, 0.f, 1.1f}) {
            World casting;
            reset(casting);
            casting.sabrina.swimming.yaw = yaw;
            casting.sabrina.swimming.pitch = pitch;
            Events events;
            systems::crystal(casting, 1.f / 60, {{}, true}, events);
            for (int i = 0; i < 80 && casting.crystal.magic.state == Spell::Charging; ++i) {
                systems::crystal(casting, 1.f / 60, {}, events);
                if (casting.crystal.magic.state == Spell::Charging && casting.crystal.magic.time > .4f) {
                    auto palm = sabrina_palm(casting.sabrina);
                    check((palm.offset - casting.crystal.motion.pos).length() < .001f &&
                              std::abs(palm.depth - casting.crystal.magic.depth) < .001f,
                          "held crystal stays in palm through dive, climb and left/right turns");
                    check(sabrina_pose(casting.sabrina) == SabrinaPose::Throw,
                          "charging chooses the dedicated throw pose");
                }
            }
            auto release = sabrina_palm(casting.sabrina);
            check(events.launch && casting.crystal.magic.state == Spell::Flying &&
                      (release.offset - casting.crystal.motion.pos).length() < .001f,
                  "crystal launches from the hand without teleporting to a fixed offset");
        }
    int population = reef::alive(s);
    reef::begin_visit(s);
    check(reef::hungry(s) == 1, "first visit one jelly");
    s.jelly[0].motion.pos = {0, 8};
    s.jelly[0].behaviour.state = JellyState::Hungry;
    s.sabrina.motion.pos = {-5, 8};
    s.fish[0].motion.pos = {-4.1f, 8.3f};
    s.fish[0].rescue.shield = 0;
    reef::step(s, 1.f / 60, {{}, true});
    check(s.crystal.magic.state == Spell::Charging, "crystal charges first");
    advance(s, 1.2);
    check(s.crystal.magic.state == Spell::Flying && s.sabrina.cooldown > 0, "crystal flies and recharges");
    advance(s, 1);
    check(s.session.water == Water::Calm && s.points.last_score > 0, "magic ends visit with score");
    check(reef::alive(s) >= population - 1, "fish persist after visit");
    check(s.points.rescues > 0, "crystal returns touched fish");
    reef::reset(s);
    s.session.cycle = 2;
    reef::begin_visit(s);
    check(reef::hungry(s) == 2, "occasional multiple visitors");
    reef::reset(s);
    reef::begin_visit(s);
    s.sabrina.vitality.invulnerable = 200;
    s.settings.rest_seconds = 150;
    advance(s, 110);
    check(s.session.water == Water::Calm, "visits end without input");
    reef::reset(s);
    reef::begin_visit(s);
    s.jelly[0].behaviour.state = JellyState::Hungry;
    s.jelly[0].motion.pos = s.sabrina.motion.pos;
    reef::step(s, 1.f / 60, {});
    check(s.sabrina.vitality.stunned > 0 && s.sabrina.vitality.invulnerable > s.sabrina.vitality.stunned,
          "sting has recovery protection");
    Settings config;
    config.aquarium = true;
    reef::reset(s, 7, config);
    advance(s, 500);
    check(s.session.cycle == 0 && reef::alive(s) == 72, "aquarium remains peaceful and bounded");
    for (auto &f : s.fish) {
        check(f.motion.pos.x >= -15.3 && f.motion.pos.x <= 15.3 && f.motion.pos.y >= 2 &&
                  f.motion.pos.y <= 16.2,
              "fish stay in water");
        f.life.active = false;
    }
    reef::step(s, 1.f / 60, {});
    check(reef::alive(s) >= 3, "empty aquarium replenishes");
    // Cruise smoothly with actual propulsion in the direction of the nose.
    reef::reset(s, 19, config);
    s.session.spawn_in = 1000;
    for (auto &f : s.fish)
        f.life.active = false;
    auto &swimmer = s.fish[0];
    swimmer.life.active = true;
    swimmer.motion.pos = {-8, 9};
    swimmer.motion.direction = {1, 0};
    swimmer.appearance.size = .8f;
    swimmer.swimming.turn_in = 100;
    swimmer.swimming.phase = 0;
    s.sabrina.motion.pos = {-14, 2};
    float lowest = 9, highest = 9;
    int turns = 0;
    float prior = 0, previous_turn = 0;
    for (int i = 0; i < 600; ++i) {
        reef::step(s, 1.f / 60, {});
        lowest = std::min(lowest, swimmer.motion.pos.y);
        float forward = swimmer.motion.velocity.x * swimmer.motion.direction.x +
                        swimmer.motion.velocity.y * swimmer.motion.direction.y;
        float sideways = swimmer.motion.velocity.x * swimmer.motion.direction.y -
                         swimmer.motion.velocity.y * swimmer.motion.direction.x;
        check(forward > .1f && std::abs(sideways) < .0001f,
              "fish propulsion follows heading without sideways sliding");
        highest = std::max(highest, swimmer.motion.pos.y);
        if (i > 30 && (swimmer.motion.direction.y - prior) * previous_turn < 0)
            ++turns;
        previous_turn = swimmer.motion.direction.y - prior;
        prior = swimmer.motion.direction.y;
    }
    check(highest - lowest > .25f, "cruise has gentle vertical variation");
    reef::reset(s, 19, config);
    lowest = s.sabrina.motion.pos.y;
    highest = s.sabrina.motion.pos.y;
    for (int i = 0; i < 120; ++i) {
        reef::step(s, 1.f / 60, {{1, 0}});
        lowest = std::min(lowest, s.sabrina.motion.pos.y);
        highest = std::max(highest, s.sabrina.motion.pos.y);
    }
    check(s.sabrina.motion.pos.x > 5 && highest - lowest > .25f,
          "Sabrina's strokes advance with a gentle wave");
    advance(s, 2);
    auto resting = s.sabrina.motion.pos;
    advance(s, 2);
    check((s.sabrina.motion.pos - resting).length() < .01f,
          "swim oscillation does not drift a resting player");
    // Song affects only fish within its shrinking audible radius.
    World silent;
    reef::reset(s, 19, config);
    silent = s;
    s.sabrina.motion.pos = {0, 8};
    silent.sabrina.motion.pos = s.sabrina.motion.pos;
    for (auto *world : {&s, &silent}) {
        world->session.spawn_in = 1000;
        for (auto &f : world->fish)
            f.life.active = false;
        world->fish[0].life.active = true;
        world->fish[0].motion.pos = {1.8f, 8};
        world->fish[1].life.active = true;
        world->fish[1].motion.pos = {-12, 12};
    }
    check(reef::toggle_song(s), "full song starts");
    advance(s, 2);
    advance(silent, 2);
    check((s.fish[0].motion.pos - silent.fish[0].motion.pos).length() > .2f, "near fish responds to song");
    check((s.fish[1].motion.pos - silent.fish[1].motion.pos).length() < .001f,
          "distant fish cannot hear song");
    float radius = reef::song_radius(s);
    advance(s, 3);
    check(reef::song_radius(s) < radius && s.sabrina.song.energy < 60, "radius shrinks with energy");
    check(reef::toggle_song(s) && !s.sabrina.song.active, "song can stop early to conserve energy");
    advance(s, 1);
    check(reef::toggle_song(s), "partly used song can resume");
    int empty_events = 0;
    for (int i = 0; i < 600; ++i)
        empty_events += reef::step(s, 1.f / 60, {}).song_empty;
    check(empty_events == 1 && s.sabrina.song.locked && !s.sabrina.song.active,
          "depletion ends song and locks it exactly once");
    check(!reef::toggle_song(s), "partly recharged exhausted song stays locked");
    advance(s, 15);
    check(s.sabrina.song.locked && !reef::toggle_song(s), "lock remains until complete recharge");
    advance(s, 6);
    check(!s.sabrina.song.locked && s.sabrina.song.energy == 100 && reef::toggle_song(s),
          "full recharge unlocks song");
    reef::reset(s, 27, config);
    reef::step(s, 1.f / 60, {{}, true});
    advance(s, 1.2f);
    check(s.sabrina.cooldown > 149, "crystal has a long recharge");
    reef::toggle_song(s);
    advance(s, 12.1f);
    advance(s, 20.1f);
    check(!s.sabrina.song.locked && s.sabrina.song.energy == 100 && s.sabrina.cooldown > 115,
          "song replenishes while crystal is still cooling");
    advance(s, 120);
    check(s.sabrina.cooldown == 0, "crystal eventually recharges");
    // The orbit follows Sabrina and the charge gathers smoothly to her hands.
    reef::reset(s, 19, config);
    advance(s, .5f);
    V2 orbital = s.crystal.motion.pos - s.sabrina.motion.pos;
    advance(s, .5f);
    check(((s.crystal.motion.pos - s.sabrina.motion.pos) - orbital).length() > .3f, "ready crystal orbits");
    V2 before_charge = s.crystal.motion.pos;
    reef::step(s, 1.f / 60, {{}, true});
    check((s.crystal.motion.pos - before_charge).length() < .2f, "charge starts at orbit without snapping");
    // A sustained wave must noticeably move fish, not disappear into drag in one frame.
    reef::reset(s, 19, config);
    s.session.spawn_in = 1000;
    s.sabrina.motion.pos = {-7, 8};
    for (auto &f : s.fish)
        f.life.active = false;
    s.fish[0].life.active = true;
    s.fish[0].appearance.birth = 1;
    s.fish[0].appearance.size = .6f;
    s.fish[0].motion.pos = {-3, 8.5f};
    s.fish[0].motion.direction = {1, 0};
    s.fish[0].swimming.cruise_heading = 0;
    s.fish[0].swimming.turn_in = 100;
    silent = s;
    reef::step(s, 1.f / 60, {{}, false, false, true});
    reef::step(silent, 1.f / 60, {});
    advance(s, 1.4f);
    advance(silent, 1.4f);
    check(s.fish[0].motion.pos.x - silent.fish[0].motion.pos.x > 2.f,
          "wave carries fish more than two extra units");
    // Song scoring requires a sustained escort out of danger, not merely singing.
    reef::reset(s);
    reef::begin_visit(s);
    for (auto &f : s.fish)
        f.life.active = false;
    s.sabrina.motion.pos = {0, 8};
    s.sabrina.song.active = true;
    s.sabrina.song.strength = 1;
    auto &escort = s.fish[0];
    escort.life.active = true;
    escort.appearance.birth = 1;
    escort.motion.pos = {1, 8};
    s.jelly[0].motion.pos = {4, 8};
    systems::fish_swimming(s, 1.f / 60);
    check(s.points.score == 0, "song does not award points simply for attracting fish");
    s.jelly[0].motion.pos = {-12, 12};
    for (int i = 0; i < 120; ++i)
        systems::fish_swimming(s, 1.f / 60);
    check(s.points.score == 30, "song rewards an escort into clear water once");
    for (int i = 0; i < 120; ++i)
        systems::fish_swimming(s, 1.f / 60);
    check(s.points.score == 30, "song escort reward cannot be farmed in the same visit");
    // Returning crystal fish are protected, but still displaced outside solid jelly surfaces.
    reef::reset(s);
    reef::begin_visit(s);
    s.sabrina.motion.pos = {-12, 14};
    for (auto &f : s.fish)
        f.life.active = false;
    auto &protected_fish = s.fish[0];
    protected_fish.life.active = true;
    protected_fish.appearance.birth = 1;
    s.jelly[0].behaviour.state = JellyState::Hungry;
    s.jelly[0].motion.pos = {0, 8};
    protected_fish.motion.pos = {0, 8};
    protected_fish.rescue.shield = 4;
    Events protected_events;
    systems::jellyfish_and_contacts(s, 1.f / 60, protected_events);
    check(protected_fish.life.active && protected_events.eaten == 0 &&
              (protected_fish.motion.pos - s.jelly[0].motion.pos).length() > 1,
          "crystal rescue shield prevents a pop");
    protected_fish.rescue.shield = 0;
    protected_fish.motion.pos = s.jelly[0].motion.pos;
    systems::jellyfish_and_contacts(s, 1.f / 60, protected_events);
    check(!protected_fish.life.active && protected_events.eaten == 1,
          "expired bubble restores dangerous contact");
    // The projectile uses the growing body envelope and consumes just one visitor per cast.
    reef::reset(s);
    reef::begin_visit(s);
    s.jelly[0].behaviour.state = JellyState::Hungry;
    s.jelly[0].behaviour.eaten = 40;
    s.jelly[0].motion.pos = {0, 8};
    s.jelly[1] = s.jelly[0];
    s.crystal.magic.state = Spell::Flying;
    s.crystal.motion.pos = {-2, 8.2f};
    s.crystal.motion.direction = {1, 0};
    Events impact;
    systems::crystal(s, 1.f / 60, {}, impact);
    check(impact.repelled == 1 && s.crystal.magic.state == Spell::Burst &&
              s.jelly[1].behaviour.state == JellyState::Hungry,
          "crystal bursts on the grown outer surface and repels only one visitor");
    // A bounded travelling wave replaces automatic grabbing.
    reef::reset(s);
    reef::begin_visit(s);
    s.sabrina.motion.pos = {0, 8};
    s.sabrina.motion.velocity = {};
    for (auto &f : s.fish)
        f.life.active = false;
    for (int i = 0; i < 5; ++i) {
        s.fish[i].life.active = true;
        s.fish[i].appearance.birth = 1;
        s.fish[i].appearance.size = .6f;
    }
    s.fish[0].motion.pos = {2, 8.5f};
    s.fish[1].motion.pos = {3, 9};
    s.fish[2].motion.pos = {-2, 8.5f};
    s.fish[3].motion.pos = {2, 13};
    s.fish[4].motion.pos = {2, 8.5f};
    s.fish[4].rescue.teleport = 1;
    s.jelly[0].motion.pos = {-2, 8.5f};
    auto jelly_before = s.jelly[0];
    Events wave_events;
    systems::water_wave(s, 1.f / 60, {{}, false, false, true}, wave_events);
    check(wave_events.wave_cast && s.sabrina.wave.cooldown == 3, "wave casts with a short cooldown");
    for (int i = 0; i < 50; ++i)
        systems::water_wave(s, 1.f / 60, {}, wave_events);
    check(s.fish[0].contact.impulse.x > 0 && s.fish[1].contact.impulse.x > 0,
          "wave pushes multiple fish across its front area");
    check(s.fish[2].contact.impulse.length() == 0 && s.fish[3].contact.impulse.length() == 0 &&
              s.fish[4].contact.impulse.length() == 0,
          "wave excludes rear, distant and teleporting fish");
    check((s.jelly[0].motion.pos - jelly_before.motion.pos).length() == 0 &&
              s.jelly[0].behaviour.state == jelly_before.behaviour.state,
          "wave cannot move or repel jellyfish");
    check(s.points.score == 40 && !s.sabrina.wave.active,
          "safe directional pushes score once and wave expires");
    wave_events = {};
    systems::water_wave(s, .01f, {{}, false, false, true}, wave_events);
    check(!wave_events.wave_cast, "wave cooldown blocks repeated casts");
    s.sabrina.wave.cooldown = 0;
    systems::water_wave(s, .01f, {{}, false, false, true}, wave_events);
    for (int i = 0; i < 50; ++i)
        systems::water_wave(s, 1.f / 60, {}, wave_events);
    check(s.points.score == 40, "repeat wave cannot farm points on the same fish during a visit");
    s.sabrina.wave.cooldown = 0;
    s.sabrina.vitality.stunned = 1;
    wave_events = {};
    systems::water_wave(s, .01f, {{}, false, false, true}, wave_events);
    check(!wave_events.wave_cast, "stunned Sabrina cannot cast a wave");
    s.sabrina.vitality.stunned = 0;
    s.sabrina.swimming.yaw = -3.14159265f;
    s.sabrina.swimming.facing = -1;
    systems::water_wave(s, .01f, {{}, false, false, true}, wave_events);
    check(s.sabrina.wave.direction.x < -.99f, "wave follows left-facing Sabrina");
    // Natural visits cannot vanish after a handful of fish or twenty seconds.
    reef::reset(s);
    reef::begin_visit(s);
    s.sabrina.motion.pos = {-14, 16};
    for (auto &f : s.fish)
        f.life.active = false;
    auto &visitor = s.jelly[0];
    visitor.behaviour.state = JellyState::Hungry;
    visitor.motion.pos = {10, 8};
    visitor.behaviour.eaten = 10;
    s.session.visit_time = 30;
    Events visit_events;
    systems::jellyfish_and_contacts(s, .01f, visit_events);
    check(visitor.behaviour.state == JellyState::Hungry && visitor.behaviour.hunger > 80,
          "a full jellyfish stays for the minimum visit duration");
    s.session.visit_time = 60;
    systems::jellyfish_and_contacts(s, .01f, visit_events);
    check(visitor.behaviour.state == JellyState::Leaving, "full jellyfish may leave after sixty seconds");
    int heard = 0;
    for (int seed = 1; seed <= 100; ++seed) {
        reef::reset(s, seed);
        reef::begin_visit(s);
        s.sabrina.motion.pos = {-10, 8};
        s.sabrina.song.active = true;
        s.sabrina.song.strength = 1;
        s.jelly[0].motion.pos = {10, 8};
        s.jelly[0].behaviour.state = JellyState::Hungry;
        s.jelly[0].behaviour.turn_in = 100;
        systems::jellyfish_and_contacts(s, .01f, visit_events);
        heard += s.jelly[0].behaviour.song_interest > 0;
        auto rng = s.rng;
        systems::jellyfish_and_contacts(s, .01f, visit_events);
        check(s.rng == rng, "song attraction does not reroll every frame");
        s.sabrina.song.active = false;
        systems::jellyfish_and_contacts(s, .01f, visit_events);
        check(s.jelly[0].behaviour.song_interest == 0, "stopping song ends its directional lure");
    }
    check(heard > 5 && heard < 45, "distant jellyfish sometimes hear song outside the fish radius");
    reef::reset(s);
    reef::begin_visit(s);
    s.sabrina.motion.pos = {-14, 16};
    s.jelly[0].behaviour.state = JellyState::Hungry;
    s.jelly[0].motion.pos = {0, 8};
    for (auto &f : s.fish)
        f.life.active = false;
    for (int i = 0; i < 3; ++i) {
        s.fish[i].life.active = true;
        s.fish[i].appearance.birth = 1;
        s.fish[i].appearance.size = .3f;
    }
    s.fish[0].motion.pos = {0, 9.2f};
    s.fish[1].motion.pos = {-.4f, 6.45f};
    s.fish[2].motion.pos = {1.05f, 8.1f};
    Events surface_events;
    systems::jellyfish_and_contacts(s, .001f, surface_events);
    check(surface_events.eaten == 3, "bell, outer edge and tentacle tips eat simultaneously");
    // Every contact is consumed, including simultaneous contacts and arrival.
    reef::reset(s);
    reef::begin_visit(s);
    s.sabrina.motion.pos = {-12, 14};
    for (auto &f : s.fish)
        f.life.active = false;
    s.jelly[0].motion.pos = {0, 8};
    s.jelly[0].behaviour.state = JellyState::Arriving;
    s.fish[0].life.active = s.fish[1].life.active = true;
    s.fish[0].motion.pos = {0, 8};
    s.fish[1].motion.pos = {.3f, 8};
    s.fish[0].rescue.shield = 0; // Unprotected contact is always dangerous.
    auto eaten = reef::step(s, 1.f / 60, {});
    check(eaten.eaten == 2 && !s.fish[0].life.active && !s.fish[1].life.active,
          "all contacting fish pop without an eating cooldown");
    check(s.fish[0].life.pop_time > 0, "eaten fish retain a brief red-pop visual state");
    s.session.spawn_in = 1000;
    advance(s, .6f);
    check(s.fish[0].life.pop_time == 0, "pop animation expires");
    reef::reset(s);
    reef::begin_visit(s);
    s.sabrina.motion.pos = {0, 8};
    s.jelly[0].motion.pos = s.sabrina.motion.pos;
    s.jelly[0].behaviour.state = JellyState::Hungry;
    auto hit = reef::step(s, 1.f / 60, {});
    float health = s.sabrina.vitality.health;
    check(hit.hit && health == 72 && s.sabrina.vitality.hurt_flash > 0 && s.sabrina.vitality.stunned > 0,
          "sting damages health and flashes red");
    advance(s, 1);
    check(s.sabrina.vitality.health == health, "contact immunity prevents damage every frame");
    s.settings.aquarium = true;
    advance(s, 6);
    check(s.sabrina.vitality.health == health, "health does not heal during the safety delay");
    advance(s, 2);
    check(s.sabrina.vitality.health > health && s.sabrina.vitality.health < health + 3,
          "health recovers slowly after safety delay");
    s.settings.aquarium = false;
    s.sabrina.vitality.health = 28;
    s.sabrina.vitality.invulnerable = 0;
    s.jelly[0].behaviour.state = JellyState::Hungry;
    s.jelly[0].motion.pos = s.sabrina.motion.pos;
    auto over = reef::step(s, 1.f / 60, {});
    check(over.game_over && s.session.game_over && s.sabrina.vitality.health == 0 && !s.sabrina.song.active,
          "zero health ends the game");
    float ended = s.session.time;
    auto position = s.sabrina.motion.pos;
    advance(s, 30, {{1, 1}, true, true});
    check(s.session.time == ended && s.sabrina.vitality.health == 0 &&
              (s.sabrina.motion.pos - position).length() == 0 && !reef::toggle_song(s),
          "game over freezes simulation and rejects play inputs");
    reef::reset(s);
    check(!s.session.game_over && s.sabrina.vitality.health == 100, "new game restores health");
    World a, b;
    reef::reset(a, 81);
    reef::reset(b, 81);
    advance(a, 60);
    advance(b, 60);
    check(a.fish[0].motion.pos.x == b.fish[0].motion.pos.x && reef::alive(a) == reef::alive(b),
          "seed deterministic");
    std::cout << "Aquarium simulation checks passed\n";
}
