// SPDX-License-Identifier: Apache-2.0
#pragma once
#include "components.h"
#include <algorithm>
#include <cmath>
namespace reef {
enum class SabrinaPose { Idle, Swim, TurnLeft, TurnRight, Sing, Wave, Hurt, Throw };
// Rendering and the exported arm-contact samples must select the same authored pose.
inline SabrinaPose sabrina_pose(const SabrinaEntity &s) {
    if (s.vitality.hurt_flash > 0 || s.vitality.stunned > 0)
        return SabrinaPose::Hurt;
    if (s.throw_time >= 0)
        return SabrinaPose::Throw;
    if (s.wave.active)
        return SabrinaPose::Wave;
    if (s.song.active)
        return SabrinaPose::Sing;
    float effort = std::min(1.f, s.motion.velocity.length() / 7.2f);
    if (std::abs(std::sin(s.swimming.yaw)) > .4f && effort > .15f)
        return s.swimming.facing < 0 ? SabrinaPose::TurnLeft : SabrinaPose::TurnRight;
    return effort > .12f ? SabrinaPose::Swim : SabrinaPose::Idle;
}
inline float sabrina_pose_phase(const SabrinaEntity &s) {
    if (sabrina_pose(s) == SabrinaPose::Throw)
        return std::clamp(s.throw_time / 1.6f, 0.f, 1.f);
    return sabrina_pose(s) == SabrinaPose::Wave
               ? std::clamp(s.wave.time / .75f, 0.f, 1.f)
               : std::fmod(s.swimming.phase, 6.28318530718f) / 6.28318530718f;
}
} // namespace reef
