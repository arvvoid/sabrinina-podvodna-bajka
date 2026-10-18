// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Peaceful intervals, visitor scheduling, recovery and scoring.
use crate::*;

pub fn begin_visit(world: &mut World) {
    if world.settings.aquarium || world.session.finished() || world.session.cycle >= FINAL_ROUND {
        return;
    }
    world.session.water = Water::Visit;
    world.session.cycle += 1;
    world.points.score = 0;
    world.points.rewards = Default::default();
    world.points.visit_rescues = 0;
    world.session.visit_time = 0.0;
    world.points.visit_start_fish = alive(world);
    let mut count = 1;
    for _ in 1..world.settings.visitor_limit.clamp(1, NORMAL_VISITOR_LIMIT) {
        if world.rng.range(0.0, 1.0) < multiple_chance(world.session.cycle) {
            count += 1;
        }
    }
    if world.session.cycle == FINAL_ROUND {
        count = JELLY_COUNT;
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
            if world.session.cycle == FINAL_ROUND {
                if i < 2 { 6.0 } else { 13.0 }
            } else {
                world.rng.range(5.0, 14.0)
            },
        );
        jellyfish.motion.direction = v(
            -jellyfish.behaviour.exit_side as f32,
            world.rng.range(-0.2, 0.2),
        );
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
    if world.session.finished()
        || world.settings.aquarium
        || world.session.water != Water::Visit
        || hungry(world) != 0
        || alive(world) == 0
        || world.sabrina.vitality.health <= 0.0
    {
        return;
    }
    world.points.last_survivors = alive(world);
    let mut kinds = [false; SPECIES_COUNT];
    for fish in &world.fish {
        if fish.life.active {
            kinds[fish.appearance.species] = true
        }
    }
    world.points.last_variety = kinds.iter().filter(|&&x| x).count() as i32;
    award(
        world,
        world.points.last_survivors * SURVIVOR_POINTS,
        ScoreFeedback::Survivors,
    );
    world.points.last_score = world.points.score;
    world.points.total_score += world.points.score;
    world.points.best_score = world.points.best_score.max(world.points.score);
    world.session.water = Water::Calm;
    world.session.rest_remaining = world.settings.rest_seconds * world.rng.range(0.85, 1.2);
    events.visit_ended = true;
    if world.session.cycle == FINAL_ROUND {
        world.session.victory = true;
        world.sabrina.song.active = false;
        world.sabrina.song.strength = 0.0;
        world.sabrina.motion.velocity = v(0.0, 0.0);
        events.victory = true;
    }
}

pub fn population_and_visits(world: &mut World, dt: f32, events: &mut Events) {
    if world.session.finished() {
        return;
    }
    // Leaving visitors still block births until they disappear offscreen.
    let visitors = world
        .jelly
        .iter()
        .any(|jellyfish| jellyfish.behaviour.state != JellyState::Hidden);
    if !visitors && world.session.water == Water::Calm {
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
    if world.session.finished() {
        return;
    }
    // A wipeout wins over round completion, including the last visitor's defeat.
    if world.sabrina.vitality.health <= 0.0
        || (!world.settings.aquarium && world.session.water == Water::Visit && alive(world) == 0)
    {
        world.session.game_over = true;
        events.game_over = true;
        world.sabrina.song.active = false;
        world.sabrina.song.strength = 0.0;
        world.sabrina.motion.velocity = v(0.0, 0.0);
    }
    if world.session.game_over {
        return;
    }
    for index in 0..FISH_COUNT {
        if world.fish[index].life.active && world.fish[index].rescue.wave_escape > 0.0 {
            world.fish[index].rescue.wave_escape -= dt;
            if world.fish[index].rescue.wave_escape <= 0.0
                && world
                    .jelly
                    .iter()
                    .filter(|j| {
                        matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving)
                    })
                    .all(|j| (j.motion.pos - world.fish[index].motion.pos).length() >= 6.0)
            {
                award_rescue(world, index);
            }
        }
    }
    if !world.session.game_over && world.sabrina.vitality.heal_delay <= 0.0 {
        world.sabrina.vitality.health = (world.sabrina.vitality.health
            + dt * if world.settings.gentle { 1.2 } else { 0.8 })
        .min(100.0)
    }
    if !world.session.game_over && world.session.water == Water::Visit && hungry(world) == 0 {
        finish_visit(world, events)
    }
}
