// SPDX-License-Identifier: Apache-2.0
#include "simulation/contacts.h"
#include "simulation/simulation.h"
#include "simulation/systems/systems.h"
#include <cstdlib>
#include <iostream>
using namespace reef;
void check(bool ok, const char *message) {
    if (!ok) {
        std::cerr << "FAILED: " << message << '\n';
        std::exit(1);
    }
}
World empty() {
    World w;
    reset(w, 42, {true, true});
    for (auto &f : w.fish)
        f.life.active = false;
    w.session.spawn_in = 10000;
    w.sabrina.motion.pos = {12, 15};
    return w;
}
void fish(FishEntity &f, V2 position, V2 velocity) {
    f.life.active = true;
    f.appearance.size = 1;
    f.appearance.birth = 1;
    f.motion.pos = position;
    f.motion.velocity = velocity;
    f.motion.direction = velocity.length() > .01f ? velocity.unit() : V2{1, 0};
    f.swimming.cruise_heading = std::atan2(f.motion.direction.y, f.motion.direction.x);
    f.swimming.turn_in = 100;
}
int main() {
    auto w = empty();
    auto &a = w.fish[0];
    auto &b = w.fish[1];
    fish(a, {-.5f, 8}, {1, 0});
    fish(b, {.5f, 8}, {-1, 0});
    systems::soft_contacts(w, 1.f / 60);
    check(contact_gap(fish_body(a), fish_body(b)) > -.005f, "head-on bodies separate");
    check(a.motion.velocity.x < 0 && b.motion.velocity.x > 0, "head-on fish gently bounce apart");
    check(a.motion.velocity.length() < 1 && b.motion.velocity.length() < 1,
          "water contact dissipates energy");
    check(std::abs(a.motion.velocity.x + b.motion.velocity.x) < .001f,
          "equal fish exchange equal opposite impulses");
    fish(a, {0, 8}, {0, 0});
    fish(b, {0, 8}, {0, 0});
    systems::soft_contacts(w, 1.f / 60);
    check(contact_gap(fish_body(a), fish_body(b)) > -.005f, "coincident spawns resolve without NaN");
    for (float yaw : {0.f, 1.57f, 3.14159265f})
        for (float pitch : {-.7f, 0.f, .55f, .98f}) {
            w = empty();
            w.sabrina.motion.pos = {0, 9};
            w.sabrina.swimming.yaw = yaw;
            w.sabrina.swimming.pitch = pitch;
            for (int part = 0; part < 16; ++part) {
                auto shapes = sabrina_body(w.sabrina);
                fish(w.fish[0], (shapes[part].a + shapes[part].b) * .5f, {0, 0});
                systems::soft_contacts(w, 1.f / 60);
                for (auto shape : shapes)
                    check(contact_gap(fish_body(w.fish[0]), shape) > -.025f,
                          "fish clears every rotated mermaid body part");
            }
        }
    w = empty();
    w.sabrina.motion.pos = {0, 9};
    fish(w.fish[0], {0, 9}, {1, 0});
    w.fish[0].rescue.teleport = 1;
    systems::soft_contacts(w, 1.f / 60);
    check((w.fish[0].motion.pos - V2{0, 9}).length() == 0, "magic transit retains its authored path");
    w = empty();
    fish(w.fish[0], {-14.9f, 8}, {-1, 0});
    fish(w.fish[1], {-14.7f, 8}, {-1, 0});
    for (int i = 0; i < 30; ++i)
        systems::soft_contacts(w, 1.f / 60);
    for (int i = 0; i < 2; ++i)
        check(w.fish[i].motion.pos.x >= -15.3f && std::isfinite(w.fish[i].motion.pos.y),
              "wall contacts remain bounded and finite");
    w = empty();
    w.sabrina.motion.pos = {-4, 9};
    fish(w.fish[0], {0, 9}, {-1, 0});
    fish(w.fish[1], {0, 13}, {1, 0});
    fish(w.fish[2], {10, 5}, {1, 0});
    for (int tick = 0; tick < 70; ++tick) {
        step(w, 1.f / 30, {{1, 0}});
        for (auto shape : sabrina_body(w.sabrina))
            check(contact_gap(fish_body(w.fish[0]), shape) > -.04f,
                  "fast swimming cannot tunnel through a fish");
    }
    check(w.sabrina.vitality.health == 100 && w.points.score == 0, "gentle bumps cause no injury or scoring");
    auto repeat = w;
    for (int tick = 0; tick < 120; ++tick) {
        step(w, 1.f / 60, {{-1, .3f}});
        step(repeat, 1.f / 60, {{-1, .3f}});
    }
    for (int i = 0; i < fish_count; ++i)
        check((w.fish[i].motion.pos - repeat.fish[i].motion.pos).length() == 0,
              "contact simulation is deterministic");
    w = empty();
    begin_visit(w); // The test aquarium intentionally skips begin_visit, so set the visitor explicitly.
    auto &jelly = w.jelly[0];
    jelly.behaviour.state = JellyState::Hungry;
    jelly.motion.pos = {0, 9};
    w.sabrina.motion.pos = {.3f, 9};
    w.sabrina.vitality.invulnerable = 20;
    Events contact_events;
    systems::jellyfish_and_contacts(w, .01f, contact_events);
    bool clear = true;
    for (auto a : sabrina_body(w.sabrina))
        for (auto b : jelly_body(jelly))
            clear = clear && contact_gap(a, b) > -.02f;
    check(clear && w.sabrina.vitality.health == 100,
          "sting immunity still blocks passing through a jellyfish");
    float previous_size = jelly_scale(jelly);
    for (int meals = 1; meals <= 40; ++meals) {
        jelly.behaviour.eaten = meals;
        check(jelly_scale(jelly) > previous_size, "every meal increases visible/contact size");
        previous_size = jelly_scale(jelly);
    }
    auto &other = w.jelly[1];
    other.behaviour.state = JellyState::Hungry;
    other.motion.pos = jelly.motion.pos;
    systems::jellyfish_and_contacts(w, .01f, contact_events);
    clear = true;
    for (auto a : jelly_body(jelly))
        for (auto b : jelly_body(other))
            clear = clear && contact_gap(a, b) > -.04f;
    check(clear, "overlapping jellyfish separate instead of passing through each other");
    std::cout << "Soft-contact checks passed\n";
}
