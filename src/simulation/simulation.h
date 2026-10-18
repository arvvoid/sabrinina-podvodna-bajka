// SPDX-License-Identifier: Apache-2.0
#pragma once
#include "world.h"
namespace reef {
// One deterministic fixed-step pipeline. No Godot, heap allocation, or virtual dispatch.
Events step(World &world, float dt, Input input);
bool toggle_song(World &world);
void begin_visit(World &world);
float recharge_duration(const World &world);
float song_duration(const World &world);
float song_recharge_duration(const World &world);
float wave_recharge_duration(const World &world);
float song_radius(const World &world);
// Shared path for the simulation anchor and the presentation trail.
CrystalOrbit crystal_orbit(const World &world, float seconds_behind = 0);
} // namespace reef
