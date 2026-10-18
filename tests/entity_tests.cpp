// SPDX-License-Identifier: Apache-2.0
#include "simulation/simulation.h"
#include "simulation/systems/systems.h"
#include <cstdlib>
#include <iostream>
#include <type_traits>
using namespace reef;
namespace {
void require(bool pass, const char *message) {
    if (!pass) {
        std::cerr << "FAILED: " << message << '\n';
        std::exit(1);
    }
}
} // namespace
int main() {
    static_assert(std::is_trivially_copyable_v<FishEntity>);
    static_assert(std::is_trivially_copyable_v<JellyEntity>);
    static_assert(std::is_trivially_copyable_v<SabrinaEntity>);
    World w;
    reset(w);
    Entity first = w.fish[0].id;
    require(find_fish(w, first) == &w.fish[0], "entity resolves its current occupant");
    require(!find_fish(w, no_entity), "null entity is rejected");
    Entity malformed = first;
    malformed.slot = fish_count;
    require(!find_fish(w, malformed), "out of bounds handle is rejected");
    malformed = first;
    malformed.kind = EntityKind::Jelly;
    require(!find_fish(w, malformed), "wrong archetype is rejected");
    w.fish[0].life.active = false;
    w.fish[0].life.pop_time = .55f;
    add_fish(w, 1);
    require(w.fish[0].id == first, "pop retains its entity until presentation has finished");
    w.fish[0].life.pop_time = 0;
    add_fish(w, 1);
    require(w.fish[0].id != first && !find_fish(w, first), "reusing a slot invalidates its old handle");
    Events events;
    auto before_reset = w.fish[0].id;
    reset(w);
    require(!find_fish(w, before_reset), "reset invalidates handles from the previous game");
    begin_visit(w);
    auto previous_jelly = w.jelly[0].id;
    w.crystal.magic.state = Spell::Flying;
    w.crystal.magic.target = previous_jelly;
    w.crystal.motion.pos = {0, 8};
    w.crystal.motion.direction = {0, 1};
    begin_visit(w);
    w.jelly[0].motion.pos = {12, 8};
    require(!find_jelly(w, previous_jelly), "new visit invalidates previous jelly handles");
    systems::crystal(w, 1.f / 60, {}, events);
    require(w.crystal.motion.direction.x == 0 && w.crystal.motion.direction.y == 1,
            "stale crystal target cannot home on a new occupant");
    World snapshot = w;
    w.fish[0].motion.pos = {100, 100};
    require((snapshot.fish[0].motion.pos - w.fish[0].motion.pos).length() > 1,
            "world snapshots own their component data");
    require(find_fish(snapshot, snapshot.fish[0].id) == &snapshot.fish[0],
            "handles resolve within copied worlds");
    std::cout << "Entity lifecycle checks passed\n";
}
