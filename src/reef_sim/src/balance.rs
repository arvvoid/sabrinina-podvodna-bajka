// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Shared combat and score rules. Damage points use actual health removed.
use crate::*;

pub const CRYSTAL_DAMAGE: f32 = 40.0;
pub const WAVE_DAMAGE: f32 = 10.0;
pub const BUBBLE_SECONDS: f32 = 3.0;
pub const WAVE_REACH: f32 = 5.5;
pub const WAVE_CHARGE_SECONDS: f32 = 3.0;
pub const WAVE_COOLDOWN: f32 = 3.0;
pub const CHARGED_WAVE_COOLDOWN: f32 = 9.0;
pub const CRYSTAL_COOLDOWN: f32 = 12.0;
pub const CRYSTAL_CHAIN_COOLDOWN: f32 = 2.0;
pub const CRYSTAL_CHAIN_SECONDS: f32 = 1.5;
pub const RESCUE_POINTS: i32 = 50;
pub const SURVIVOR_POINTS: i32 = 25;
pub const RETREAT_POINTS: i32 = 100;

pub fn award(world: &mut World, points: i32, feedback: ScoreFeedback) {
    if world.settings.aquarium || world.session.water != Water::Visit || points <= 0 {
        return;
    }
    world.points.score += points;
    let reward = &mut world.points.rewards[feedback as usize];
    reward.points = if reward.time > 0.0 {
        reward.points + points
    } else {
        points
    };
    reward.time = 2.5;
}

/// One rescue reward per fish per round, shared by all three abilities.
pub fn award_rescue(world: &mut World, index: usize) {
    if world.session.water == Water::Visit
        && !world.settings.aquarium
        && world.fish[index].rescue.rescued_cycle != world.session.cycle
    {
        world.fish[index].rescue.rescued_cycle = world.session.cycle;
        world.points.rescues += 1;
        world.points.visit_rescues += 1;
        award(world, RESCUE_POINTS, ScoreFeedback::Rescue);
    }
}

pub fn dangerous(jellyfish: &JellyEntity) -> bool {
    matches!(
        jellyfish.behaviour.state,
        JellyState::Arriving | JellyState::Hungry
    ) && jellyfish.behaviour.bubble <= 0.0
        && jellyfish.behaviour.health > 0.0
}

pub fn threat_distance(world: &World, position: V2) -> f32 {
    world
        .jelly
        .iter()
        .filter(|j| dangerous(j))
        .fold(f32::INFINITY, |distance, j| {
            distance.min((position - j.motion.pos).length())
        })
}

pub fn visitor_speed(world: &World) -> f32 {
    // Smoothly increasing, asymptotically bounded so late rounds stay playable.
    let round = (world.session.cycle - 1).max(0) as f32;
    (if world.settings.gentle { 0.8 } else { 1.1 }) + 0.9 * round / (round + 15.0)
}

pub fn multiple_chance(cycle: i32) -> f32 {
    let round = (cycle - 1).max(0) as f32;
    round / (round + 5.0)
}

pub fn damage_jelly(
    world: &mut World,
    index: usize,
    damage: f32,
    crystal: bool,
    events: &mut Events,
) {
    let jellyfish = &mut world.jelly[index];
    if !matches!(
        jellyfish.behaviour.state,
        JellyState::Arriving | JellyState::Hungry
    ) || jellyfish.behaviour.health <= 0.0
    {
        return;
    }
    let removed = damage.min(jellyfish.behaviour.health);
    events.jelly_hit = true;
    jellyfish.behaviour.health -= removed;
    jellyfish.behaviour.repel_flash = 0.65;
    if crystal && jellyfish.behaviour.health > 0.0 {
        jellyfish.behaviour.bubble = BUBBLE_SECONDS;
    }
    let defeated = jellyfish.behaviour.health <= 0.0;
    if defeated {
        jellyfish.behaviour.bubble = 0.0;
        send_home(jellyfish);
        events.repelled += 1;
    }
    award(
        world,
        removed as i32,
        if crystal {
            ScoreFeedback::Crystal
        } else {
            ScoreFeedback::Wave
        },
    );
    if defeated {
        award(world, RETREAT_POINTS, ScoreFeedback::Retreat);
    }
}
