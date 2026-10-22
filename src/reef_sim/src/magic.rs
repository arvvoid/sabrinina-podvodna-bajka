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

pub fn crystal_chain_target(world: &World) -> Option<Entity> {
    let magic = &world.crystal.magic;
    if magic.state != Spell::Burst
        || magic.hit_count == 0
        || magic.hit_count >= magic.hits.len()
        || magic.time > CRYSTAL_CHAIN_SECONDS
    {
        return None;
    }
    world
        .jelly
        .iter()
        .filter(|j| {
            matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving)
                && j.behaviour.health > 0.0
                && !magic.hits[..magic.hit_count].contains(&j.id)
        })
        .min_by(|a, b| {
            (a.motion.pos - world.crystal.motion.pos)
                .length()
                .total_cmp(&(b.motion.pos - world.crystal.motion.pos).length())
        })
        .map(|j| j.id)
}

pub fn crystal(world: &mut World, dt: f32, input: Input, events: &mut Events) {
    if input.cast
        && world.sabrina.cooldown <= 0.0
        && world.sabrina.vitality.stunned <= 0.0
        && world.crystal.magic.state == Spell::Resting
        && !world.sabrina.wave.active
        && !world.sabrina.wave.charging
    {
        world.crystal.magic.state = Spell::Charging;
        world.crystal.magic.time = 0.0;
        world.crystal.magic.hit_count = 0;
        world.crystal.magic.hits = [Entity::default(); 4];
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
        world.sabrina.throw_direction = world.crystal.magic.aim;
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
            if let Some(target) = find_jelly(world, world.crystal.magic.target) {
                world.sabrina.throw_direction =
                    (target.motion.pos - world.sabrina.motion.pos).unit();
            }
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
            let mut travel = dt * 12.0;
            let mut homing = false;
            if let Some(jellyfish) = find_jelly(world, world.crystal.magic.target)
                && matches!(
                    jellyfish.behaviour.state,
                    JellyState::Hungry | JellyState::Arriving
                )
            {
                homing = true;
                travel = travel.min((jellyfish.motion.pos - world.crystal.motion.pos).length());
                world.crystal.motion.direction =
                    (jellyfish.motion.pos - world.crystal.motion.pos).unit()
            }
            let from = world.crystal.motion.pos;
            world.crystal.motion.pos =
                world.crystal.motion.pos + world.crystal.motion.direction * travel;
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
                ) || world.crystal.magic.hits[..world.crystal.magic.hit_count]
                    .contains(&jellyfish.id)
                {
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
                    let count = world.crystal.magic.hit_count;
                    world.crystal.magic.hits[count] = jellyfish.id;
                    world.crystal.magic.hit_count += 1;
                    damage_jelly(
                        world,
                        index,
                        CRYSTAL_DAMAGE * 0.5_f32.powi(count as i32),
                        true,
                        events,
                    );
                    if count > 0 {
                        world.sabrina.cooldown += CRYSTAL_CHAIN_COOLDOWN;
                    }
                    world.crystal.magic.state = Spell::Burst;
                    world.crystal.magic.time = 0.0;
                    break;
                }
            }
            if world.crystal.magic.state == Spell::Flying
                && ((!homing
                    && (world.crystal.motion.pos.x.abs() > 17.0
                        || world.crystal.motion.pos.y < 0.5
                        || world.crystal.motion.pos.y > 18.0
                        || world.crystal.magic.time > 1.5))
                    || world.crystal.magic.time > 5.0)
            {
                world.crystal.magic.state = Spell::Returning;
                world.crystal.magic.time = 0.0;
            }
        }
        Spell::Burst => {
            if input.cast
                && world.sabrina.vitality.stunned <= 0.0
                && let Some(target) = crystal_chain_target(world)
            {
                world.crystal.magic.target = target;
                if let Some(jellyfish) = find_jelly(world, target) {
                    world.crystal.motion.direction =
                        (jellyfish.motion.pos - world.crystal.motion.pos).unit();
                }
                world.crystal.magic.state = Spell::Flying;
                world.crystal.magic.time = 0.0;
                events.launch = true;
            } else if world.crystal.magic.time > CRYSTAL_CHAIN_SECONDS
                || (world.crystal.magic.time > 0.2 && crystal_chain_target(world).is_none())
            {
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

fn wave_hits_surface(wave: WaterWave, previous: f32, front: f32, surface: Capsule) -> bool {
    let local = |point: V2| {
        let offset = point - wave.origin;
        v(
            offset.dot(wave.direction),
            offset.x * wave.direction.y - offset.y * wave.direction.x,
        )
    };
    let a = local(surface.a);
    let b = local(surface.b);
    let low = (previous - 0.35).max(0.0) - surface.radius;
    let high = front + 0.35 + surface.radius;
    if a.x.max(b.x) < low || a.x.min(b.x) > high {
        return false;
    }
    let delta = b - a;
    let (start, end) = if delta.x.abs() > 0.00001 {
        let t0 = (low - a.x) / delta.x;
        let t1 = (high - a.x) / delta.x;
        (t0.min(t1).clamp(0.0, 1.0), t0.max(t1).clamp(0.0, 1.0))
    } else {
        (0.0, 1.0)
    };
    let a = a + delta * start;
    let b = a + delta * (end - start);
    let inside = |p: V2| p.y.abs() <= 0.65 * wave.scale + 0.38 * p.x.max(0.0) + surface.radius;
    inside(a) || inside(b) || a.y * b.y <= 0.0
}

pub fn water_wave(world: &mut World, dt: f32, input: Input, events: &mut Events) {
    let sabrina = &mut world.sabrina;
    let wave = &mut sabrina.wave;
    if sabrina.vitality.stunned > 0.0 {
        wave.cancel_charge();
    }
    if input.wave
        && wave.cooldown <= 0.0
        && !wave.charging
        && !wave.active
        && sabrina.vitality.stunned <= 0.0
        && sabrina.throw_time < 0.0
    {
        wave.charging = true;
        wave.charge_time = 0.0;
    }
    if wave.charging {
        if input.wave_held {
            wave.charge_time += dt;
        }
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
        if input.wave_held && wave.charge_time + 0.00001 < WAVE_CHARGE_SECONDS {
            return;
        }
        wave.charging = false;
        wave.scale = if wave.charge_time + 0.00001 >= WAVE_CHARGE_SECONDS {
            2.0
        } else {
            1.0
        };
        wave.active = true;
        wave.time = 0.0;
        wave.cast_id += 1;
        wave.cooldown = if wave.scale > 1.0 {
            CHARGED_WAVE_COOLDOWN
        } else {
            WAVE_COOLDOWN
        };
        events.wave_cast = true;
    }
    if !wave.active {
        return;
    }
    let previous = wave.time * 8.0 * wave.scale;
    wave.time += dt;
    let front = (wave.time * 8.0 * wave.scale).min(WAVE_REACH * wave.scale);
    let wave = *wave;
    // A jellyfish touching her head, torso or arms during the cast is hit even
    // outside the forward sweep; the margin covers water pushed off her skin.
    let pushing = sabrina_upper_body(&world.sabrina).map(|part| Capsule {
        radius: part.radius + 0.25,
        ..part
    });
    for fish in &mut world.fish {
        if !fish.life.active || fish.rescue.teleport > 0.0 || fish.rescue.wave_hit == wave.cast_id {
            continue;
        }
        let ahead = (fish.motion.pos - wave.origin).dot(wave.direction);
        if !wave_hits_surface(
            wave,
            previous,
            front,
            Capsule {
                a: fish.motion.pos,
                b: fish.motion.pos,
                radius: fish.appearance.size * 0.45,
            },
        ) {
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
        let chest = wave.origin - wave.direction * 0.75;
        // A visitor touching Sabrina often has its centre level with or just
        // behind her chest, so judge "in front" by its leading visible edge.
        let in_front = |surface: Capsule| {
            (surface.a - chest)
                .dot(wave.direction)
                .max((surface.b - chest).dot(wave.direction))
                + surface.radius
                > 0.0
        };
        let hit = jelly_body(jellyfish).iter().any(|&surface| {
            let swept = wave_hits_surface(wave, previous, front, surface);
            let touching = pushing
                .iter()
                .any(|&part| contact_gap(part, surface) <= 0.0);
            in_front(surface) && (swept || touching)
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
