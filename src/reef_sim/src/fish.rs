// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Steering and swimming for the bounded fish population.
use crate::*;

pub fn fish_swimming(world: &mut World, dt: f32) {
    // Each slot sees earlier slots' updated state. Preserve this order and the
    // random draws; parallel steering would change the simulation.
    for i in 0..FISH_COUNT {
        let mut fish = world.fish[i];
        if !fish.life.active {
            continue;
        }
        fish.rescue.wave_flow = (fish.rescue.wave_flow - dt).max(0.0);
        fish.contact.impulse = fish.contact.impulse
            * (-(if fish.rescue.wave_flow > 0.0 {
                1.25
            } else {
                3.5
            }) * dt)
                .exp();
        fish.contact.turn_time = (fish.contact.turn_time - dt).max(0.0);
        fish.appearance.birth = (fish.appearance.birth + dt).min(1.0);
        fish.rescue.shield = (fish.rescue.shield - dt).max(0.0);
        if fish.rescue.teleport > 0.0 {
            fish.rescue.teleport = (fish.rescue.teleport - dt).max(0.0);
            let u = 1.0 - fish.rescue.teleport / 1.2;
            let u = u * u * (3.0 - 2.0 * u);
            fish.motion.pos = fish.rescue.teleport_from * (1.0 - u) + fish.rescue.teleport_to * u;
            world.fish[i] = fish;
            continue;
        }
        fish.swimming.phase += dt * (0.85 + 0.18 * fish.appearance.size);
        fish.swimming.turn_in -= dt;
        if fish.swimming.turn_in <= 0.0 {
            fish.swimming.cruise_heading = if fish.motion.direction.x < 0.0 {
                3.14159
            } else {
                0.0
            } + world.rng.range(-0.32, 0.32);
            fish.swimming.turn_in = world.rng.range(4.0, 9.0)
        }
        if fish.motion.pos.x > 12.3 && fish.swimming.cruise_heading.cos() > 0.0 {
            fish.swimming.cruise_heading = 3.14159 + world.rng.range(-0.25, 0.25);
            fish.swimming.turn_in = 6.0
        }
        if fish.motion.pos.x < -12.3 && fish.swimming.cruise_heading.cos() < 0.0 {
            fish.swimming.cruise_heading = world.rng.range(-0.25, 0.25);
            fish.swimming.turn_in = 6.0
        }
        let cruise = 0.85 + fish.appearance.size * 0.70;
        let desired_heading = fish.swimming.cruise_heading + 0.12 * fish.swimming.phase.sin();
        let mut desired_velocity = v(
            desired_heading.cos() * cruise,
            desired_heading.sin() * cruise,
        );
        if fish.motion.pos.x > 12.3 {
            desired_velocity.x -= ((fish.motion.pos.x - 12.3) * 2.5).min(6.0)
        }
        if fish.motion.pos.x < -12.3 {
            desired_velocity.x += ((-12.3 - fish.motion.pos.x) * 2.5).min(6.0)
        }
        if fish.motion.pos.y > 14.3 {
            desired_velocity.y -= (fish.motion.pos.y - 14.3) * 2.0
        }
        if fish.motion.pos.y < 3.8 {
            desired_velocity.y += (3.8 - fish.motion.pos.y) * 2.0
        }
        let mut separation = v(0.0, 0.0);
        let mut neighbours = 0;
        let mut school = v(0.0, 0.0);
        for (k, other) in world.fish.iter().enumerate() {
            if k == i || !other.life.active {
                continue;
            }
            let d = (fish.motion.pos - other.motion.pos).length();
            if d < 0.7 && d > 0.01 {
                separation = separation + (fish.motion.pos - other.motion.pos).unit() * (0.7 - d)
            }
            if other.appearance.species == fish.appearance.species && d < 3.0 {
                school = school + other.motion.velocity;
                neighbours += 1;
            }
        }
        desired_velocity = desired_velocity + separation * 0.8;
        if neighbours > 0 {
            desired_velocity = desired_velocity * 0.8 + school * (0.2 / neighbours as f32)
        }
        for jellyfish in &world.jelly {
            if dangerous(jellyfish) {
                let away = fish.motion.pos - jellyfish.motion.pos;
                let d = away.length();
                if d < 3.4 {
                    desired_velocity = desired_velocity + away.unit() * (3.4 - d) * 0.8
                }
            }
        }
        let away = fish.motion.pos - world.sabrina.motion.pos;
        let radius = song_radius(world);
        let hearing = world.sabrina.song.active
            && world.sabrina.song.strength > 0.05
            && away.length() < radius
            && fish.rescue.wave_flow <= 0.0;
        if hearing {
            let wake = if world.sabrina.motion.velocity.length() > 0.4 {
                world.sabrina.motion.velocity.unit() * -(radius * 0.22).min(1.4)
            } else {
                v(0.0, 0.0)
            };
            let angle = fish.swimming.phase + world.session.time * 0.22;
            let slot = world.sabrina.motion.pos
                + wake
                + v(
                    angle.cos() * (radius * 0.40).min(1.5 + 0.18 * fish.appearance.species as f32),
                    angle.sin() * (radius * 0.30).min(1.0 + 0.12 * fish.appearance.species as f32),
                );
            let pull = slot - fish.motion.pos;
            let pulse = 0.85 + 0.15 * (world.session.time * 2.2 + fish.swimming.phase).sin();
            desired_velocity = desired_velocity * (1.0 - world.sabrina.song.strength * 0.85)
                + (pull.unit() * (pull.length() * 2.0).min(3.8) * pulse
                    + world.sabrina.motion.velocity * 1.05)
                    * world.sabrina.song.strength;
        } else if !world.sabrina.song.active && hungry(world) == 0 && away.length() < 1.5 {
            let mut push = away.unit();
            let mut nearest = 1000.0;
            for jellyfish in &world.jelly {
                if jellyfish.behaviour.state == JellyState::Hungry
                    && (jellyfish.motion.pos - fish.motion.pos).length() < nearest
                {
                    nearest = (jellyfish.motion.pos - fish.motion.pos).length();
                    push = (fish.motion.pos - jellyfish.motion.pos).unit();
                }
            }
            desired_velocity = desired_velocity + push * 2.5;
        }
        let mut danger = 1000.0_f32;
        for jellyfish in &world.jelly {
            if matches!(
                jellyfish.behaviour.state,
                JellyState::Hungry | JellyState::Arriving
            ) {
                danger = danger.min((fish.motion.pos - jellyfish.motion.pos).length())
            }
        }
        fish.rescue.song_escort = if hearing {
            fish.rescue.song_escort + dt
        } else {
            fish.rescue.song_risk_cycle = -1;
            0.0
        };
        if hearing
            && threat_distance(world, fish.motion.pos) < 4.0
            && world.session.water == Water::Visit
        {
            fish.rescue.song_risk_cycle = world.session.cycle
        }
        if hearing
            && danger >= 6.0
            && fish.rescue.song_escort >= 1.0
            && world.session.water == Water::Visit
            && fish.rescue.song_risk_cycle == world.session.cycle
            && fish.rescue.song_rescued_cycle != world.session.cycle
        {
            fish.rescue.song_rescued_cycle = world.session.cycle;
            world.fish[i] = fish;
            award_rescue(world, i);
            fish = world.fish[i];
        }
        let mut heading = fish.motion.direction.y.atan2(fish.motion.direction.x);
        let target = desired_velocity.y.atan2(desired_velocity.x);
        let change = (target - heading).sin().atan2((target - heading).cos());
        let turn_rate = 1.35
            + if hearing { 1.85 } else { 0.0 }
            + if fish.contact.turn_time > 0.0 {
                1.1
            } else {
                0.0
            };
        heading += change.clamp(-turn_rate * dt, turn_rate * dt);
        fish.motion.direction = v(heading.cos(), heading.sin());
        let mut desired_speed = desired_velocity
            .length()
            .clamp(0.55, if hearing { 6.8 } else { 3.8 });
        desired_speed *= 1.0 - 0.48 * (change.abs() / 1.5).min(1.0);
        fish.swimming.speed +=
            (desired_speed - fish.swimming.speed) * (dt * if hearing { 3.5 } else { 1.8 }).min(1.0);
        fish.swimming.tail_phase += dt * (6.0 + fish.swimming.speed * 5.0);
        let thrust = 1.0 + 0.08 * (fish.swimming.tail_phase * 2.0).sin();
        fish.motion.velocity =
            fish.motion.direction * (fish.swimming.speed * thrust) + fish.contact.impulse;
        fish.motion.pos = bounds(fish.motion.pos + fish.motion.velocity * dt);
        world.fish[i] = fish;
    }
}
