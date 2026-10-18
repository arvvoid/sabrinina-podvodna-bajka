// SPDX-License-Identifier: Apache-2.0
#include "simulation/contacts.h"
#include "common.h"
#include "simulation/generated/sabrina_contacts.h"
#include "simulation/sabrina_pose.h"
#include "systems.h"
namespace reef {
namespace {
float dot(V2 a, V2 b) {
    return a.x * b.x + a.y * b.y;
}
struct Pair {
    V2 a, b;
};
Pair closest(Capsule a, Capsule b) {
    V2 d1 = a.b - a.a, d2 = b.b - b.a, r = a.a - b.a;
    float aa = dot(d1, d1), ee = dot(d2, d2), ff = dot(d2, r), s = 0, t = 0;
    if (aa < 1e-8f && ee < 1e-8f)
        return {a.a, b.a};
    if (aa < 1e-8f)
        t = std::clamp(ff / ee, 0.f, 1.f);
    else {
        float c = dot(d1, r);
        if (ee < 1e-8f)
            s = std::clamp(-c / aa, 0.f, 1.f);
        else {
            float bb = dot(d1, d2), denom = aa * ee - bb * bb;
            if (denom > 1e-8f)
                s = std::clamp((bb * ff - c * ee) / denom, 0.f, 1.f);
            t = (bb * s + ff) / ee;
            if (t < 0) {
                t = 0;
                s = std::clamp(-c / aa, 0.f, 1.f);
            } else if (t > 1) {
                t = 1;
                s = std::clamp((bb - c) / aa, 0.f, 1.f);
            }
        }
    }
    return {a.a + d1 * s, b.a + d2 * t};
}
bool solid(const World &w, const FishEntity &f) {
    return f.life.active && f.rescue.teleport <= 0;
}
void impulse(FishEntity &f, V2 change, V2 normal) {
    f.contact.impulse = f.contact.impulse + change;
    // Drag dissipates the impact in fish_swimming; a bounded impulse avoids launch-like bounces.
    float speed = f.contact.impulse.length();
    if (speed > 6)
        f.contact.impulse = f.contact.impulse * (6 / speed);
    f.motion.velocity = f.motion.velocity + change;
    f.contact.turn_time = .65f;
    V2 escape = f.motion.direction + normal * 1.8f;
    f.swimming.cruise_heading = std::atan2(escape.y, escape.x);
    f.swimming.turn_in = std::max(f.swimming.turn_in, 1.2f);
}
V2 fallback(int a, int b) {
    float angle = float((a * 73 + b * 151) % 360) * .0174532925f;
    return {std::cos(angle), std::sin(angle)};
}
} // namespace
float sabrina_palm_offset() {
    return sabrina_contacts::palm_offset;
}
CrystalOrbit sabrina_palm(const SabrinaEntity &s) {
    const auto &poses = sabrina_contacts::arms[static_cast<int>(sabrina_pose(s))];
    float sample = sabrina_pose_phase(s) * (sabrina_contacts::samples - 1);
    int frame = std::clamp(static_cast<int>(sample), 0, sabrina_contacts::samples - 2);
    float blend = sample - frame;
    const auto &a = poses[frame][6];
    const auto &b = poses[frame + 1][6];
    float x = a.x + (b.x - a.x) * blend, y = a.y + (b.y - a.y) * blend;
    float z = a.z + (b.z - a.z) * blend;
    float rx = x * std::cos(s.swimming.pitch) - y * std::sin(s.swimming.pitch);
    float ry = x * std::sin(s.swimming.pitch) + y * std::cos(s.swimming.pitch);
    return {s.motion.pos + V2{rx * std::cos(s.swimming.yaw) + z * std::sin(s.swimming.yaw), ry},
            .5f - rx * std::sin(s.swimming.yaw) + z * std::cos(s.swimming.yaw)};
}
Capsule fish_body(const FishEntity &f) {
    float size = f.appearance.size * (.3f + .7f * f.appearance.birth);
    V2 axis = f.motion.direction.unit();
    return {f.motion.pos - axis * (.38f * size), f.motion.pos + axis * (.23f * size), .43f * size};
}
std::array<Capsule, 16> sabrina_body(const SabrinaEntity &s) {
    auto rotate = [](V2 v, float angle) {
        float c = std::cos(angle), sn = std::sin(angle);
        return V2{v.x * c - v.y * sn, v.x * sn + v.y * c};
    };
    auto point = [&](V2 p) {
        p = rotate(p, s.swimming.pitch);
        return s.motion.pos + V2{p.x * std::cos(s.swimming.yaw), p.y};
    };
    auto shape = [&](V2 a, V2 b, float r) { return Capsule{point(a), point(b), r}; };
    std::array<Capsule, 16> body{};
    body[0] = shape({.82f, .95f}, {.65f, .72f}, .32f);
    body[1] = shape({.65f, .72f}, {0, 0}, .40f);
    // The source rig and contact chain use the same hip-relative landmarks,
    // phase and traveling-wave coefficients. Soft radii cover depth flexion.
    constexpr V2 rest[] = {{0, 0},
                           {-.4224f, -.14784f},
                           {-.88704f, -.261888f},
                           {-1.35168f, -.29568f},
                           {-1.73184f, -.2112f},
                           {-2.04864f, -.06336f},
                           {-2.28096f, .04224f}};
    constexpr float radii[] = {.38f, .35f, .30f, .26f, .22f, .20f};
    bool swimming = s.motion.velocity.length() > .864f && !s.song.active && !s.wave.active &&
                    s.throw_time < 0 && s.vitality.stunned <= 0 && s.vitality.hurt_flash <= 0;
    float effort = swimming ? 1.f : .75f, phase = sabrina_pose_phase(s) * 6.2831853f, angle = 0;
    float speed = std::min(1.f, s.motion.velocity.length() / 7.2f);
    float resting =
        std::clamp((s.swimming.pitch - .2f) / .45f, 0.f, 1.f) * (1.f - std::clamp(speed * 2.f, 0.f, 1.f));
    V2 end{};
    for (int i = 0; i < 6; ++i) {
        angle += (i == 0 ? .30f : i == 1 ? .10f : 0.f) * resting;
        angle += effort * (.13f + .028f * i) * std::sin(phase - i * .72f);
        V2 next = end + rotate(rest[i + 1] - rest[i], angle);
        body[i + 2] = shape(end, next, radii[i]);
        end = next;
    }
    float fan = angle + .26f * effort * std::sin(phase - 4.1f);
    float tip = fan + .20f * effort * std::sin(phase - 4.7f);
    for (int side = 0; side < 2; ++side) {
        float sign = side == 0 ? 1.f : -1.f;
        V2 middle = end + rotate({-.4224f, sign * .4224f}, fan);
        V2 edge = middle + rotate({-.38016f, sign * .2112f}, tip);
        body[8 + side * 2] = shape(end, middle, .22f);
        body[9 + side * 2] = shape(middle, edge, .20f);
    }
    // Authored shoulder/elbow/fingertip samples keep contacts with the new rig,
    // including casting and depth during turns, without a rendering dependency.
    const auto &poses = sabrina_contacts::arms[static_cast<int>(sabrina_pose(s))];
    float sample = sabrina_pose_phase(s) * (sabrina_contacts::samples - 1);
    int frame = std::clamp(static_cast<int>(sample), 0, sabrina_contacts::samples - 2);
    float blend = sample - frame;
    auto arm_point = [&](int index) {
        const auto &a = poses[frame][index];
        const auto &b = poses[frame + 1][index];
        V2 xy = rotate({a.x + (b.x - a.x) * blend, a.y + (b.y - a.y) * blend}, s.swimming.pitch);
        float z = a.z + (b.z - a.z) * blend;
        return s.motion.pos + V2{xy.x * std::cos(s.swimming.yaw) + z * std::sin(s.swimming.yaw), xy.y};
    };
    for (int side = 0; side < 2; ++side) {
        body[12 + side * 2] = {arm_point(side * 3), arm_point(side * 3 + 1), .16f};
        body[13 + side * 2] = {arm_point(side * 3 + 1), arm_point(side * 3 + 2), .14f};
    }
    return body;
}
float jelly_scale(const JellyEntity &j) {
    // Each meal grows her; diminishing growth keeps later visits playable.
    return .95f + .9f * (1.f - std::exp(-.07f * j.behaviour.eaten));
}
V2 contact_displacement(Capsule moving, Capsule solid) {
    auto pair = closest(moving, solid);
    V2 delta = pair.a - pair.b;
    float distance = delta.length();
    float overlap = moving.radius + solid.radius - distance;
    if (overlap <= 0)
        return {};
    V2 normal =
        distance > .0001f ? delta * (1.f / distance) : ((moving.a + moving.b) - (solid.a + solid.b)).unit();
    if (normal.length() < .1f)
        normal = {1, 0};
    return normal * (overlap + .002f);
}
std::array<Capsule, 3> jelly_body(const JellyEntity &j) {
    float size = jelly_scale(j);
    auto point = [&](V2 p) { return j.motion.pos + p * size; };
    // Broad envelopes cover the pulsing bell, crest and all swaying tentacles.
    return {{{point({0, .05f}), point({0, .38f}), .90f * size},
             {point({-.42f, -.3f}), point({-.42f, -1.25f}), .40f * size},
             {point({.42f, -.3f}), point({.42f, -1.25f}), .40f * size}}};
}
float contact_gap(Capsule a, Capsule b) {
    auto p = closest(a, b);
    return (p.a - p.b).length() - a.radius - b.radius;
}
void systems::soft_contacts(World &w, float) {
    const auto body = sabrina_body(w.sabrina);
    // A small bounded sequential solver is sufficient for the fixed 72-fish pool.
    // All visible fish share the gameplay plane; decorative depth does not disable contacts.
    for (int pass = 0; pass < 6; ++pass) {
        for (int i = 0; i < fish_count; ++i) {
            auto &a = w.fish[i];
            if (!solid(w, a))
                continue;
            for (int j = i + 1; j < fish_count; ++j) {
                auto &b = w.fish[j];
                if (!solid(w, b) || (a.motion.pos - b.motion.pos).length() > 3)
                    continue;
                auto ca = fish_body(a), cb = fish_body(b);
                auto p = closest(ca, cb);
                V2 delta = p.a - p.b;
                float distance = delta.length(), penetration = ca.radius + cb.radius - distance;
                if (penetration <= 0)
                    continue;
                V2 normal = distance > .0001f ? delta * (1 / distance) : fallback(i, j);
                float ia = 1 / (ca.radius * ca.radius), ib = 1 / (cb.radius * cb.radius);
                float correction = std::min(.45f, penetration + .001f);
                a.motion.pos = bounds(a.motion.pos + normal * (correction * ia / (ia + ib)));
                b.motion.pos = bounds(b.motion.pos - normal * (correction * ib / (ia + ib)));
                float closing = dot(a.motion.velocity - b.motion.velocity, normal);
                if (closing < 0) {
                    float force = -(1 + .16f) * closing / (ia + ib);
                    impulse(a, normal * (force * ia), normal);
                    impulse(b, normal * (-force * ib), normal * -1);
                }
            }
            for (auto capsule : body) {
                auto ca = fish_body(a);
                auto p = closest(ca, capsule);
                V2 delta = p.a - p.b;
                float distance = delta.length();
                float penetration = ca.radius + capsule.radius - distance;
                if (penetration <= 0)
                    continue;
                V2 normal = distance > .0001f ? delta * (1 / distance) : fallback(i, 97);
                a.motion.pos = bounds(a.motion.pos + normal * std::min(.55f, penetration + .002f));
                float closing = dot(a.motion.velocity - w.sabrina.motion.velocity, normal);
                if (closing < 0)
                    impulse(a, normal * (-1.08f * closing), normal);
            }
        }
    }
}
} // namespace reef
