// SPDX-License-Identifier: Apache-2.0
#pragma once
#include "simulation/simulation.h"
namespace reef::systems {
void timers(World &world, float dt);
void song(World &world, float dt, Input input, Events &events);
void player_swimming(World &world, float dt, Input input);
void population_and_visits(World &world, float dt, Events &events);
void crystal(World &world, float dt, Input input, Events &events);
void water_wave(World &world, float dt, Input input, Events &events);
void fish_swimming(World &world, float dt);
void soft_contacts(World &world, float dt);
void jellyfish_and_contacts(World &world, float dt, Events &events);
void recovery_and_scoring(World &world, float dt, Events &events);
} // namespace reef::systems
