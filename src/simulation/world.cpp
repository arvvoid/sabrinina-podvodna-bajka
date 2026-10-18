// SPDX-License-Identifier: MIT
#include "systems/common.h"
#include "systems/systems.h"
namespace reef {
int alive(const World &w) {
    int n = 0;
    for (const auto &f : w.fish)
        n += f.life.active;
    return n;
}
int hungry(const World &w) {
    int n = 0;
    for (const auto &j : w.jelly)
        n += j.behaviour.state == JellyState::Hungry || j.behaviour.state == JellyState::Arriving;
    return n;
}
int add_fish(World &w, int count, bool from_edge) {
    if (count <= 0)
        return 0;
    int added = 0;
    for (auto &f : w.fish) {
        if (f.life.active || f.life.pop_time > 0)
            continue;
        f = FishEntity{};
        f.id = allocate_entity(w, EntityKind::Fish, int(&f - w.fish.data()));
        f.life.active = true;
        f.appearance.species = int(random(w, 0, 5.999f));
        f.appearance.size = random(w, .55, 1.18);
        f.appearance.depth = random(w, -1.7, .6);
        f.swimming.phase = random(w, 0, 6.283f);
        f.motion.pos = {from_edge ? (random(w, 0, 1) < .5f ? -15.f : 15.f) : random(w, -14, 14),
                        random(w, 3, 15)};
        float heading = (random(w, 0, 1) < .5f ? 0.f : 3.14159f) + random(w, -.25f, .25f);
        f.swimming.cruise_heading = heading;
        f.swimming.speed = .8f + f.appearance.size * .65f;
        f.swimming.tail_phase = random(w, 0, 6.283f);
        f.motion.direction = {std::cos(heading), std::sin(heading)};
        f.motion.velocity = f.motion.direction * f.swimming.speed;
        f.swimming.turn_in = random(w, 2, 6);
        f.appearance.birth = from_edge ? 0 : 1;
        if (++added >= count)
            break;
    }
    return added;
}
void reset(World &w, uint32_t seed, Settings config) {
    const auto generation = w.next_generation;
    w = World{};
    w.next_generation = generation;
    w.sabrina.id = allocate_entity(w, EntityKind::Sabrina, 0);
    w.crystal.id = allocate_entity(w, EntityKind::Crystal, 0);
    w.rng.seed(seed);
    w.settings = config;
    w.settings.rest_seconds = std::clamp(w.settings.rest_seconds, 25.f, 150.f);
    w.settings.visitor_limit = std::clamp(w.settings.visitor_limit, 1, 3);
    w.session.rest_remaining = w.settings.rest_seconds;
    add_fish(w, 10, false);
}

Entity allocate_entity(World &w, EntityKind kind, int slot) {
    return {slot, w.next_generation++, kind};
}
const FishEntity *find_fish(const World &w, Entity e) {
    if (e.kind != EntityKind::Fish || e.slot < 0 || e.slot >= fish_count || e.generation == 0)
        return nullptr;
    const auto &f = w.fish[e.slot];
    return f.id == e ? &f : nullptr;
}
FishEntity *find_fish(World &w, Entity e) {
    return const_cast<FishEntity *>(find_fish(static_cast<const World &>(w), e));
}
const JellyEntity *find_jelly(const World &w, Entity e) {
    if (e.kind != EntityKind::Jelly || e.slot < 0 || e.slot >= jelly_count || e.generation == 0)
        return nullptr;
    const auto &j = w.jelly[e.slot];
    return j.id == e ? &j : nullptr;
}
JellyEntity *find_jelly(World &w, Entity e) {
    return const_cast<JellyEntity *>(find_jelly(static_cast<const World &>(w), e));
}
} // namespace reef
