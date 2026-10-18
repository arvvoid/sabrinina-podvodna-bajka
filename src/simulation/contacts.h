// SPDX-License-Identifier: Apache-2.0
#pragma once
#include "world.h"
namespace reef {
struct Capsule {
    V2 a, b;
    float radius;
};
// Palm point in world XY, with depth for the displayed held crystal.
float sabrina_palm_offset();
CrystalOrbit sabrina_palm(const SabrinaEntity &sabrina);
Capsule fish_body(const FishEntity &fish);
std::array<Capsule, 16> sabrina_body(const SabrinaEntity &sabrina);
// Signed gap between soft body shapes; negative means penetration.
float jelly_scale(const JellyEntity &jelly);
V2 contact_displacement(Capsule moving, Capsule solid);
std::array<Capsule, 3> jelly_body(const JellyEntity &jelly);
float contact_gap(Capsule a, Capsule b);
} // namespace reef
