// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Crystal, song rescue and directional wave rules.
use crate::*;

pub fn crystal_orbit(world: &World, seconds_behind: f32) -> CrystalOrbit {
    let phase = (world.session.time - seconds_behind)
        * 0.9
        * if world.settings.reduced_motion {
            0.25
        } else {
            1.0
        };
    CrystalOrbit {
        offset: v(1.2 * phase.cos(), 0.85 + 0.28 * (phase * 2.0).sin()),
        depth: 0.7 + 0.85 * phase.sin(),
    }
}

fn touch_fish(world: &mut World, i: usize, events: &mut Events) {
    let threatened = threat_distance(world, world.fish[i].motion.pos) < 4.0;
    let fish = &mut world.fish[i];
    if !fish.life.active || fish.rescue.shield > 0.0 {
        return;
    }
    fish.rescue.teleport_from = fish.motion.pos;
    fish.rescue.teleport_to = bounds(
        world.sabrina.motion.pos + v(world.rng.range(-2.2, 2.2), world.rng.range(-1.4, 1.4)),
    );
    fish.rescue.teleport = 1.2;
    fish.rescue.shield = 4.0;
    events.teleported += 1;
    if threatened {
        award_rescue(world, i);
    }
}

pub fn crystal(world: &mut World, dt: f32, input: Input, events: &mut Events) {
    if input.cast
        && world.sabrina.cooldown <= 0.0
        && world.sabrina.vitality.stunned <= 0.0
        && world.crystal.magic.state == Spell::Resting
        && !world.sabrina.wave.active
    {
        world.crystal.magic.state = Spell::Charging;
        world.crystal.magic.time = 0.0;
        world.sabrina.throw_time = 0.0;
        let orbit = crystal_orbit(world, 0.0);
        world.crystal.magic.charge_offset = orbit.offset;
        world.crystal.magic.charge_depth = orbit.depth;
        world.crystal.magic.target = Entity::default();
        let pitch = if world.sabrina.motion.velocity.length() > 0.6 {
            world.sabrina.swimming.pitch
        } else {
            0.0
        };
        world.crystal.magic.aim = v(world.sabrina.swimming.facing * pitch.cos(), pitch.sin());
        let mut closest = 1000.0;
        for jellyfish in world.jelly.iter().filter(|_| world.settings.gentle) {
            if matches!(
                jellyfish.behaviour.state,
                JellyState::Hungry | JellyState::Arriving
            ) {
                let distance = (jellyfish.motion.pos - world.sabrina.motion.pos).length();
                if distance < closest {
                    closest = distance;
                    world.crystal.magic.target = jellyfish.id
                }
            }
        }
        events.cast = true;
    }
    world.crystal.magic.time += dt;
    if world.crystal.magic.state == Spell::Charging {
        world.sabrina.throw_time = world.crystal.magic.time
    }
    match world.crystal.magic.state {
        Spell::Resting => {
            let o = crystal_orbit(world, 0.0);
            world.crystal.motion.pos = world.sabrina.motion.pos + o.offset;
            world.crystal.magic.depth = o.depth;
        }
        Spell::Charging => {
            let mut gather = (world.crystal.magic.time / 0.4).clamp(0.0, 1.0);
            gather = gather * gather * (3.0 - 2.0 * gather);
            let palm = sabrina_palm(&world.sabrina);
            world.crystal.motion.pos =
                (world.sabrina.motion.pos + world.crystal.magic.charge_offset) * (1.0 - gather)
                    + palm.offset * gather;
            world.crystal.magic.depth =
                world.crystal.magic.charge_depth * (1.0 - gather) + palm.depth * gather;
            for i in 0..FISH_COUNT {
                if world.fish[i].life.active
                    && (world.fish[i].motion.pos - world.crystal.motion.pos).length()
                        < 0.7 + world.crystal.magic.time * 0.8
                {
                    touch_fish(world, i, events)
                }
            }
            if world.crystal.magic.time >= 1.1 {
                world.crystal.motion.direction =
                    if let Some(jellyfish) = find_jelly(world, world.crystal.magic.target) {
                        (jellyfish.motion.pos - world.crystal.motion.pos).unit()
                    } else {
                        world.crystal.magic.aim
                    };
                world.crystal.magic.state = Spell::Flying;
                world.crystal.magic.time = 0.0;
                events.launch = true;
                world.sabrina.cooldown = recharge_duration(world);
            }
        }
        Spell::Flying => {
            world.crystal.magic.depth += (1.0 - world.crystal.magic.depth) * (dt * 8.0).min(1.0);
            if let Some(jellyfish) = find_jelly(world, world.crystal.magic.target)
                && matches!(
                    jellyfish.behaviour.state,
                    JellyState::Hungry | JellyState::Arriving
                )
            {
                world.crystal.motion.direction =
                    (jellyfish.motion.pos - world.crystal.motion.pos).unit()
            }
            let from = world.crystal.motion.pos;
            world.crystal.motion.pos =
                world.crystal.motion.pos + world.crystal.motion.direction * dt * 12.0;
            for i in 0..FISH_COUNT {
                if world.fish[i].life.active
                    && (world.fish[i].motion.pos - world.crystal.motion.pos).length() < 1.2
                {
                    touch_fish(world, i, events)
                }
            }
            for index in 0..JELLY_COUNT {
                let jellyfish = &world.jelly[index];
                if !matches!(
                    jellyfish.behaviour.state,
                    JellyState::Hungry | JellyState::Arriving
                ) {
                    continue;
                }
                let hit = jelly_body(jellyfish).iter().any(|&surface| {
                    contact_gap(
                        Capsule {
                            a: from,
                            b: world.crystal.motion.pos,
                            radius: 0.30,
                        },
                        surface,
                    ) <= 0.0
                });
                if hit {
                    damage_jelly(world, index, CRYSTAL_DAMAGE, true, events);
                    world.crystal.magic.state = Spell::Burst;
                    world.crystal.magic.time = 0.0;
                    break;
                }
            }
            if world.crystal.motion.pos.x.abs() > 17.0
                || world.crystal.motion.pos.y < 0.5
                || world.crystal.motion.pos.y > 18.0
                || world.crystal.magic.time > 1.5
            {
                world.crystal.magic.state = Spell::Returning;
                world.crystal.magic.time = 0.0;
            }
        }
        Spell::Burst => {
            if world.crystal.magic.time > 0.2 {
                world.crystal.magic.state = Spell::Returning;
                world.crystal.magic.time = 0.0
            }
        }
        Spell::Returning => {
            let orbit = crystal_orbit(world, 0.0);
            let destination = world.sabrina.motion.pos + orbit.offset;
            let offset = destination - world.crystal.motion.pos;
            world.crystal.motion.direction = offset.unit();
            world.crystal.magic.depth +=
                (orbit.depth - world.crystal.magic.depth) * (dt * 8.0).min(1.0);
            if offset.length() <= dt * 18.0 {
                world.crystal.motion.pos = destination;
                world.crystal.magic.state = Spell::Resting;
                world.crystal.magic.time = 0.0;
            } else {
                world.crystal.motion.pos = world.crystal.motion.pos + offset.unit() * dt * 18.0;
            }
        }
    }
}

pub fn water_wave(world: &mut World, dt: f32, input: Input, events: &mut Events) {
    let recharge = wave_recharge_duration(world);
    let sabrina = &mut world.sabrina;
    let wave = &mut sabrina.wave;
    if input.wave
        && wave.cooldown <= 0.0
        && sabrina.vitality.stunned <= 0.0
        && sabrina.throw_time < 0.0
    {
        let pitch = if sabrina.motion.velocity.length() > 0.6 {
            sabrina.swimming.pitch
        } else {
            0.0
        };
        let side = sabrina.swimming.yaw.cos();
        let side = if side.abs() < 0.2 {
            sabrina.swimming.facing
        } else if side < 0.0 {
            -1.0
        } else {
            1.0
        };
        wave.direction = v(side * pitch.cos(), pitch.sin()).unit();
        wave.origin = sabrina.motion.pos + v(0.0, 0.5) + wave.direction * 0.75;
        wave.active = true;
        wave.time = 0.0;
        wave.cast_id += 1;
        wave.cooldown = recharge;
        events.wave_cast = true;
    }
    if !wave.active {
        return;
    }
    let previous = wave.time * 8.0;
    wave.time += dt;
    let front = (wave.time * 8.0).min(5.5);
    let wave = *wave;
    for fish in &mut world.fish {
        if !fish.life.active || fish.rescue.teleport > 0.0 || fish.rescue.wave_hit == wave.cast_id {
            continue;
        }
        let offset = fish.motion.pos - wave.origin;
        let ahead = offset.dot(wave.direction);
        let sideways = (offset.x * wave.direction.y - offset.y * wave.direction.x).abs();
        let radius = fish.appearance.size * 0.45;
        if ahead < 0.0
            || ahead < previous - 0.35 - radius
            || ahead > front + 0.35 + radius
            || sideways > 0.65 + 0.38 * ahead + radius
        {
            continue;
        }
        // A fish can receive only one impulse from this cast, across all ticks.
        fish.rescue.wave_hit = wave.cast_id;
        fish.contact.impulse = wave.direction * (5.8 - 0.15 * ahead);
        fish.contact.turn_time = 1.1;
        fish.rescue.wave_flow = 1.1;
        fish.motion.direction = (fish.motion.direction + wave.direction * 1.2).unit();
        fish.swimming.cruise_heading = wave.direction.y.atan2(wave.direction.x);
        fish.swimming.turn_in = 1.5;
        if world.session.water == Water::Visit
            && fish.rescue.rescued_cycle != world.session.cycle
            && fish.rescue.wave_escape <= 0.0
        {
            for jellyfish in &world.jelly {
                if !dangerous(jellyfish) {
                    continue;
                }
                let away = fish.motion.pos - jellyfish.motion.pos;
                if away.length() < 4.0 && away.dot(wave.direction) > 0.0 {
                    fish.rescue.wave_escape = 1.0;
                    break;
                }
            }
        }
    }
    for index in 0..JELLY_COUNT {
        let jellyfish = &mut world.jelly[index];
        if !matches!(
            jellyfish.behaviour.state,
            JellyState::Hungry | JellyState::Arriving
        ) || jellyfish.behaviour.wave_hit == wave.cast_id
        {
            continue;
        }
        let offset = jellyfish.motion.pos - wave.origin;
        let chest = wave.origin - wave.direction * 0.75;
        // Test the bell and tentacles, not just the visitor's centre. The wave
        // starts at the chest so a very close visitor is not behind its hitbox.
        let from = (previous - 0.75).max(0.0);
        let to = (front + 0.75).min(WAVE_REACH);
        let sweep = Capsule {
            a: chest + wave.direction * from,
            b: chest + wave.direction * to,
            radius: 0.65 + 0.38 * front.min(WAVE_REACH),
        };
        let reach = Capsule {
            a: chest,
            b: chest,
            radius: WAVE_REACH,
        };
        let hit = offset.dot(wave.direction) + 0.75 > 0.0
            && from <= to
            && jelly_body(jellyfish).iter().any(|&surface| {
                contact_gap(reach, surface) <= 0.0 && contact_gap(sweep, surface) <= 0.0
            });
        if hit {
            jellyfish.behaviour.wave_hit = wave.cast_id;
            jellyfish.behaviour.knockback = wave.direction * 5.0;
            damage_jelly(world, index, WAVE_DAMAGE, false, events);
        }
    }
    if wave.time >= 0.75 {
        world.sabrina.wave.active = false
    }
}
