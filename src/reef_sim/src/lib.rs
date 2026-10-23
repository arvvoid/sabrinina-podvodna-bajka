// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Deterministic gameplay rules, independent of Bevy and rendered frame rate.

// Preserve the reference's decimal constants and expression order for replay parity.
#![allow(clippy::approx_constant, clippy::excessive_precision)]

pub mod balance;
pub mod components;
pub use balance::*;

pub mod contacts;

pub mod encounters;

pub mod fish;

mod generated_contacts;

pub mod jellyfish;

pub mod magic;

pub mod player;

pub mod presentation;

pub mod world;
pub use components::*;
pub use contacts::*;
pub use encounters::*;
pub use magic::*;
pub use player::*;
pub use world::*;
/// Advance rules in dependency order. Rendering reads a separate snapshot.
/// Keep this order and random-number consumption stable when refactoring.
pub fn step(world: &mut World, mut dt: f32, input: Input) -> Events {
    let mut events = Events::default();
    if dt <= 0.0 || world.session.finished() {
        return events;
    }
    dt = dt.min(1.0 / 30.0);
    player::timers(world, dt);
    player::song(world, dt, input, &mut events);
    player::player_swimming(world, dt, input);
    encounters::population_and_visits(world, dt, &mut events);
    magic::crystal(world, dt, input, &mut events);
    magic::water_wave(world, dt, input, &mut events);
    fish::fish_swimming(world, dt);
    contacts::soft_contacts(world);
    jellyfish::jellyfish_and_contacts(world, dt, &mut events);
    encounters::recovery_and_scoring(world, dt, &mut events);
    events
}

/// Animate an isolated reading-screen aquarium, with fish on a separate plane.
/// Fish retain schooling and mutual contacts but never react to Sabrina.
pub fn step_reading(world: &mut World, dt: f32, movement: V2) {
    assert!(world.settings.aquarium);
    assert!(dt.is_finite() && dt > 0.0 && dt <= 1.0 / 30.0);
    player::timers(world, dt);
    player::player_swimming(
        world,
        dt,
        Input {
            movement,
            ..Input::default()
        },
    );
    fish::scenery_fish_swimming(world, dt);
    contacts::scenery_fish_contacts(world);
}

#[cfg(test)]
mod reading_tests {
    use super::*;

    #[test]
    fn scenery_fish_ignore_sabrina_but_gameplay_fish_still_react() {
        let mut near = World::default();
        reset(
            &mut near,
            73,
            Settings {
                aquarium: true,
                ..Settings::default()
            },
        );
        add_fish(&mut near, 1, false);
        near.fish[0].motion.pos = v(0.0, 8.0);
        near.sabrina.motion.pos = near.fish[0].motion.pos;
        let mut far = near.clone();
        far.sabrina.motion.pos = v(-12.0, 14.0);
        let mut gameplay_near = near.clone();
        let mut gameplay_far = far.clone();
        for _ in 0..120 {
            step_reading(&mut near, 1.0 / 60.0, v(0.0, 0.0));
            step_reading(&mut far, 1.0 / 60.0, v(0.0, 0.0));
            assert_eq!(near.fish[0].motion.pos, far.fish[0].motion.pos);
            assert_eq!(near.fish[0].motion.velocity, far.fish[0].motion.velocity);
            assert_eq!(near.fish[0].contact.impulse, v(0.0, 0.0));
        }
        step(&mut gameplay_near, 1.0 / 60.0, Input::default());
        step(&mut gameplay_far, 1.0 / 60.0, Input::default());
        assert_ne!(
            gameplay_near.fish[0].motion.pos,
            gameplay_far.fish[0].motion.pos
        );
    }
}
