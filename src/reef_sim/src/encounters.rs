// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Peaceful intervals, visitor scheduling, recovery and scoring.
use crate::*;

pub fn begin_visit(world: &mut World) {
    if world.settings.aquarium {
        return;
    }
    world.session.water = Water::Visit;
    world.session.cycle += 1;
    world.points.score = 0;
    world.points.visit_rescues = 0;
    world.session.visit_time = 0.0;
    world.points.visit_start_fish = alive(world);
    let mut count = 1;
    if world.session.cycle >= 3
        && world.settings.visitor_limit > 1
        && (world.session.cycle % 3 == 0 || world.rng.range(0.0, 1.0) < 0.25)
    {
        count = 2
    }
    if world.session.cycle >= 6 && world.settings.visitor_limit > 2 && world.session.cycle % 4 == 0
    {
        count = 3
    }
    for i in 0..JELLY_COUNT {
        world.jelly[i] = JellyEntity::default();
        if i >= count {
            continue;
        }
        let id = allocate_entity(world, EntityKind::Jelly, i as i32);
        let jellyfish = &mut world.jelly[i];
        jellyfish.id = id;
        jellyfish.behaviour.state = JellyState::Arriving;
        jellyfish.behaviour.exit_side = if i % 2 == 0 { 1 } else { -1 };
        jellyfish.motion.pos = v(
            jellyfish.behaviour.exit_side as f32 * 18.0,
            world.rng.range(5.0, 14.0),
        );
        jellyfish.motion.direction = v(
            -jellyfish.behaviour.exit_side as f32,
            world.rng.range(-0.2, 0.2),
        );
        jellyfish.behaviour.hunger = if world.settings.gentle { 90.0 } else { 120.0 };
        jellyfish.behaviour.kind = (world.session.cycle + i as i32) % 3;
        jellyfish.behaviour.phase = world.rng.range(0.0, 6.28);
        jellyfish.behaviour.turn_in = 4.0;
    }
}

pub fn send_home(jellyfish: &mut JellyEntity) {
    jellyfish.behaviour.state = JellyState::Leaving;
    jellyfish.behaviour.time = 0.0;
    jellyfish.behaviour.exit_side = if jellyfish.motion.pos.x < 0.0 { -1 } else { 1 };
}

pub fn finish_visit(world: &mut World, events: &mut Events) {
    world.points.last_survivors = alive(world);
    let mut kinds = [false; SPECIES_COUNT];
    for fish in &world.fish {
        if fish.life.active {
            kinds[fish.appearance.species] = true
        }
    }
    world.points.last_variety = kinds.iter().filter(|&&x| x).count() as i32;
    world.points.score += world.points.last_survivors * (75 + 5 * world.session.cycle.min(8))
        + world.points.last_variety * 60;
    world.points.last_score = world.points.score;
    world.points.total_score += world.points.score;
    world.points.best_score = world.points.best_score.max(world.points.score);
    world.session.water = Water::Calm;
    world.session.rest_remaining = world.settings.rest_seconds * world.rng.range(0.85, 1.2);
    events.visit_ended = true;
}

pub fn population_and_visits(world: &mut World, dt: f32, events: &mut Events) {
    // Leaving visitors still block births until they disappear offscreen.
    let visitors = world
        .jelly
        .iter()
        .any(|jellyfish| jellyfish.behaviour.state != JellyState::Hidden);
    if !visitors {
        world.session.spawn_in -= dt;
        if world.session.spawn_in <= 0.0 {
            let count = world.rng.range(1.0, 4.99) as i32;
            events.spawned = add_fish(world, count, true);
            world.session.spawn_in = world.rng.range(9.0, 18.0)
        }
        if alive(world) == 0 {
            events.spawned += add_fish(world, 3, true);
            world.session.spawn_in = 5.0
        }
    }
    if world.settings.aquarium {
        if world.session.water == Water::Visit {
            world.session.water = Water::Calm;
            world.session.rest_remaining = world.settings.rest_seconds;
            for jellyfish in &mut world.jelly {
                if jellyfish.behaviour.state != JellyState::Hidden {
                    send_home(jellyfish)
                }
            }
        }
    } else if world.session.water == Water::Calm {
        world.session.rest_remaining -= dt;
        if world.session.rest_remaining <= 0.0 {
            if alive(world) >= 10 {
                begin_visit(world);
                events.visit_started = true
            } else {
                world.session.rest_remaining = 5.0
            }
        }
    } else {
        world.session.visit_time += dt
    }
}

pub fn recovery_and_scoring(world: &mut World, dt: f32, events: &mut Events) {
    if !world.session.game_over && world.sabrina.vitality.heal_delay <= 0.0 {
        world.sabrina.vitality.health = (world.sabrina.vitality.health
            + dt * if world.settings.gentle { 1.2 } else { 0.8 })
        .min(100.0)
    }
    if !world.session.game_over && world.session.water == Water::Visit && hungry(world) == 0 {
        finish_visit(world, events)
    }
}
