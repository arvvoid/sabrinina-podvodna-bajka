// SPDX-License-Identifier: Apache-2.0
#pragma once
#include "components.h"
#include <array>
#include <random>
namespace reef {
inline constexpr int fish_count = 72, jelly_count = 3, species_count = 6;
// Bounded, contiguous archetype pools. Slot order is stable and is also the
// presentation binding. Entity generations distinguish successive occupants.
struct World {
    std::array<FishEntity, fish_count> fish{};
    std::array<JellyEntity, jelly_count> jelly{};
    SabrinaEntity sabrina;
    CrystalEntity crystal;
    Settings settings;
    Session session;
    Score points;
    std::mt19937 rng{27};
    uint64_t next_generation = 1;
};
Entity allocate_entity(World &world, EntityKind kind, int slot);
FishEntity *find_fish(World &world, Entity entity);
const FishEntity *find_fish(const World &world, Entity entity);
JellyEntity *find_jelly(World &world, Entity entity);
const JellyEntity *find_jelly(const World &world, Entity entity);
void reset(World &world, uint32_t seed = 27, Settings config = {});
int add_fish(World &world, int count, bool from_edge = true);
int alive(const World &world);
int hungry(const World &world);
} // namespace reef
