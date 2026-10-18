// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Visitor movement, feeding and contact damage.
use crate::*;

pub fn jellyfish_and_contacts(world: &mut World, dt: f32, events: &mut Events) {
    for index in 0..JELLY_COUNT {
        let mut jellyfish = world.jelly[index];
        if jellyfish.behaviour.state == JellyState::Hidden {
            continue;
        }
        jellyfish.behaviour.time += dt;
        let bubbled = jellyfish.behaviour.bubble > 0.0;
        jellyfish.behaviour.bubble = (jellyfish.behaviour.bubble - dt).max(0.0);
        if jellyfish.behaviour.knockback.length() > 0.01 {
            jellyfish.motion.pos =
                bounds(jellyfish.motion.pos + jellyfish.behaviour.knockback * dt);
        }
        jellyfish.behaviour.knockback = jellyfish.behaviour.knockback * (-4.0 * dt).exp();
        if bubbled {
            jellyfish.motion.velocity = v(0.0, 0.0);
            world.jelly[index] = jellyfish;
            continue;
        }
        jellyfish.behaviour.repel_flash = (jellyfish.behaviour.repel_flash - dt).max(0.0);
        jellyfish.behaviour.song_check = (jellyfish.behaviour.song_check - dt).max(0.0);
        jellyfish.behaviour.song_interest = (jellyfish.behaviour.song_interest - dt).max(0.0);
        if !world.sabrina.song.active {
            jellyfish.behaviour.song_interest = 0.0
        } else if jellyfish.behaviour.song_check <= 0.0
            && jellyfish.behaviour.state != JellyState::Leaving
        {
            jellyfish.behaviour.song_check = 2.5;
            if world.rng.range(0.0, 1.0) < (0.15 + 0.05 * world.session.cycle as f32).min(0.50) {
                jellyfish.behaviour.song_interest = 3.5
            }
        }
        jellyfish.behaviour.eat_wait = (jellyfish.behaviour.eat_wait - dt).max(0.0);
        match jellyfish.behaviour.state {
            JellyState::Arriving => {
                jellyfish.motion.pos = jellyfish.motion.pos
                    + v(
                        -jellyfish.behaviour.exit_side as f32 * 1.15,
                        (world.session.time + jellyfish.behaviour.phase).sin() * 0.15,
                    ) * dt;
                if jellyfish.behaviour.time > 4.5 {
                    jellyfish.behaviour.state = JellyState::Hungry;
                    jellyfish.behaviour.time = 0.0
                }
            }
            JellyState::Leaving => {
                jellyfish.motion.pos =
                    jellyfish.motion.pos + v(jellyfish.behaviour.exit_side as f32 * 1.9, 0.2) * dt;
                if jellyfish.motion.pos.x.abs() > 19.0 || jellyfish.behaviour.time > 9.0 {
                    jellyfish.behaviour.state = JellyState::Hidden
                }
            }
            _ => {
                jellyfish.behaviour.turn_in -= dt;
                if jellyfish.behaviour.turn_in <= 0.0 {
                    jellyfish.motion.direction =
                        v(world.rng.range(-1.0, 1.0), world.rng.range(-0.65, 0.65)).unit();
                    jellyfish.behaviour.turn_in = world.rng.range(3.0, 6.0)
                }
                let mut desired_direction = jellyfish.motion.direction * 0.75;
                if jellyfish.behaviour.kind != 0 {
                    let mut nearest = 100.0;
                    let mut target = v(0.0, 0.0);
                    for fish in &world.fish {
                        if fish.life.active
                            && (fish.motion.pos - jellyfish.motion.pos).length() < nearest
                        {
                            nearest = (fish.motion.pos - jellyfish.motion.pos).length();
                            target = fish.motion.pos
                        }
                    }
                    if nearest < 5.5 {
                        desired_direction = (target - jellyfish.motion.pos).unit()
                    }
                }
                if world.sabrina.song.active && jellyfish.behaviour.song_interest > 0.0 {
                    desired_direction =
                        (world.sabrina.motion.pos - jellyfish.motion.pos).unit() * 1.1
                }
                if jellyfish.motion.pos.x > 14.0 {
                    desired_direction.x = -1.0
                }
                if jellyfish.motion.pos.x < -14.0 {
                    desired_direction.x = 1.0
                }
                if jellyfish.motion.pos.y > 15.0 {
                    desired_direction.y = -0.6
                }
                if jellyfish.motion.pos.y < 3.0 {
                    desired_direction.y = 0.6
                }
                desired_direction.y +=
                    (world.session.time * 1.4 + jellyfish.behaviour.phase).sin() * 0.3;
                let speed = visitor_speed(world);
                jellyfish.motion.velocity = jellyfish.motion.velocity
                    + (desired_direction * speed - jellyfish.motion.velocity) * (dt * 2.0).min(1.0);
                jellyfish.motion.pos = jellyfish.motion.pos + jellyfish.motion.velocity * dt;
            }
        }
        if matches!(
            jellyfish.behaviour.state,
            JellyState::Hungry | JellyState::Arriving
        ) {
            let stinging = jelly_body(&jellyfish).iter().any(|&surface| {
                sabrina_body(&world.sabrina)
                    .iter()
                    .any(|&part| contact_gap(surface, part) <= 0.0)
            });
            if stinging {
                for _ in 0..6 {
                    let mut correction = v(0.0, 0.0);
                    for surface in jelly_body(&jellyfish) {
                        for part in sabrina_body(&world.sabrina) {
                            let candidate = contact_displacement(part, surface);
                            if candidate.length() > correction.length() {
                                correction = candidate
                            }
                        }
                    }
                    if correction.length() < 0.001 {
                        break;
                    }
                    world.sabrina.motion.pos = bounds(world.sabrina.motion.pos + correction);
                    world.sabrina.motion.pos.y = world.sabrina.motion.pos.y.max(3.2);
                }
                let normal = (world.sabrina.motion.pos - jellyfish.motion.pos).unit();
                let closing = world.sabrina.motion.velocity.dot(normal);
                if closing < 0.0 {
                    world.sabrina.motion.velocity =
                        world.sabrina.motion.velocity - normal * (1.15 * closing)
                }
            }
            if stinging && world.sabrina.vitality.invulnerable <= 0.0 {
                let sabrina = &mut world.sabrina;
                sabrina.vitality.stunned = if world.settings.gentle { 2.0 } else { 3.0 };
                sabrina.vitality.invulnerable = sabrina.vitality.stunned + 3.0;
                sabrina.song.active = false;
                sabrina.vitality.health = (sabrina.vitality.health
                    - if world.settings.gentle { 28.0 } else { 35.0 })
                .max(0.0);
                sabrina.vitality.heal_delay = 8.0;
                sabrina.vitality.hurt_flash = 0.85;
                world.points.score = (world.points.score - 100).max(0);
                events.hit = true;
                if sabrina.vitality.health <= 0.0 {
                    world.session.game_over = true;
                    events.game_over = true;
                    sabrina.motion.velocity = v(0.0, 0.0);
                    sabrina.song.strength = 0.0
                }
            }
            // Keep contact rules independent of the cosmetic eating animation.
            for fish in &mut world.fish {
                if fish.life.active && fish.rescue.teleport <= 0.0 {
                    let touching = jelly_body(&jellyfish)
                        .iter()
                        .any(|&surface| contact_gap(fish_body(fish), surface) <= 0.0);
                    if touching {
                        for _ in 0..3 {
                            for surface in jelly_body(&jellyfish) {
                                fish.motion.pos =
                                    fish.motion.pos + contact_displacement(fish_body(fish), surface)
                            }
                        }
                        if fish.rescue.shield > 0.0 {
                            fish.contact.impulse =
                                (fish.motion.pos - jellyfish.motion.pos).unit() * 3.0;
                            fish.contact.turn_time = 0.8;
                            continue;
                        }
                        fish.life.active = false;
                        fish.life.pop_time = 0.55;
                        fish.motion.velocity = v(0.0, 0.0);
                        jellyfish.behaviour.eaten += 1;
                        world.points.total_eaten += 1;
                        events.eaten += 1;
                        jellyfish.behaviour.eat_wait = 0.55;
                    }
                }
            }
        }
        world.jelly[index] = jellyfish;
    }
    for _ in 0..3 {
        for i in 0..JELLY_COUNT {
            for k in i + 1..JELLY_COUNT {
                let (a, b) = (world.jelly[i], world.jelly[k]);
                if a.behaviour.state == JellyState::Hidden
                    || b.behaviour.state == JellyState::Hidden
                {
                    continue;
                }
                let mut correction = v(0.0, 0.0);
                for ca in jelly_body(&a) {
                    for cb in jelly_body(&b) {
                        let candidate = contact_displacement(ca, cb);
                        if candidate.length() > correction.length() {
                            correction = candidate
                        }
                    }
                }
                world.jelly[i].motion.pos = a.motion.pos + correction * 0.5;
                world.jelly[k].motion.pos = b.motion.pos - correction * 0.5;
            }
        }
    }
}
