// SPDX-License-Identifier: Apache-2.0
#pragma once
#include "simulation/simulation.h"
#include <algorithm>
namespace reef {
inline V2 bounds(V2 p) {
    return {std::clamp(p.x, -15.3f, 15.3f), std::clamp(p.y, 2.0f, 16.2f)};
}
inline float random(World &world, float a, float b) {
    return std::uniform_real_distribution<float>(a, b)(world.rng);
}
void send_home(JellyEntity &jelly);
void touch_fish(World &world, FishEntity &fish, Events &events);
void finish_visit(World &world, Events &events);
} // namespace reef
