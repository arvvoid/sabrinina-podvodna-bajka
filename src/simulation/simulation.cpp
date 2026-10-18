// SPDX-License-Identifier: MIT
#include "simulation.h"
#include "systems/systems.h"
#include <algorithm>
namespace reef {
Events step(World &w, float dt, Input input) {
    Events events;
    if (dt <= 0 || w.session.game_over)
        return events;
    dt = std::min(dt, 1.f / 30);
    systems::timers(w, dt);
    systems::song(w, dt, input, events);
    systems::player_swimming(w, dt, input);
    systems::population_and_visits(w, dt, events);
    systems::crystal(w, dt, input, events);
    systems::water_wave(w, dt, input, events);
    systems::fish_swimming(w, dt);
    systems::soft_contacts(w, dt);
    systems::jellyfish_and_contacts(w, dt, events);
    systems::recovery_and_scoring(w, dt, events);
    return events;
}
} // namespace reef
