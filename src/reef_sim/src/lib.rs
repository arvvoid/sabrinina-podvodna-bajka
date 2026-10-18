// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Deterministic gameplay rules, independent of Bevy and rendered frame rate.

// Preserve the reference's decimal constants and expression order for replay parity.
#![allow(clippy::approx_constant, clippy::excessive_precision)]

pub mod components;

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
    if dt <= 0.0 || world.session.game_over {
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
