// SPDX-License-Identifier: MIT
//! Crystal, song rescue and directional wave rules.
use crate::*;
pub fn crystal_orbit(w: &World, seconds_behind: f32) -> CrystalOrbit {
    let phase = (w.session.time - seconds_behind)
        * 0.9
        * if w.settings.reduced_motion { 0.25 } else { 1.0 };
    CrystalOrbit {
        offset: v(1.2 * phase.cos(), 0.85 + 0.28 * (phase * 2.0).sin()),
        depth: 0.7 + 0.85 * phase.sin(),
    }
}
fn touch_fish(w: &mut World, i: usize, e: &mut Events) {
    let f = &mut w.fish[i];
    if !f.life.active || f.rescue.shield > 0.0 {
        return;
    }
    f.rescue.teleport_from = f.motion.pos;
    f.rescue.teleport_to =
        bounds(w.sabrina.motion.pos + v(w.rng.range(-2.2, 2.2), w.rng.range(-1.4, 1.4)));
    f.rescue.teleport = 1.2;
    f.rescue.shield = 4.0;
    e.teleported += 1;
    if w.session.water == Water::Visit && f.rescue.rescued_cycle != w.session.cycle {
        f.rescue.rescued_cycle = w.session.cycle;
        w.points.score += 80;
        w.points.rescues += 1;
        w.points.visit_rescues += 1;
    }
}
pub fn crystal(w: &mut World, dt: f32, input: Input, e: &mut Events) {
    if input.cast
        && w.sabrina.cooldown <= 0.0
        && w.sabrina.vitality.stunned <= 0.0
        && w.crystal.magic.state == Spell::Resting
        && !w.sabrina.wave.active
    {
        w.crystal.magic.state = Spell::Charging;
        w.crystal.magic.time = 0.0;
        w.sabrina.throw_time = 0.0;
        let orbit = crystal_orbit(w, 0.0);
        w.crystal.magic.charge_offset = orbit.offset;
        w.crystal.magic.charge_depth = orbit.depth;
        w.crystal.magic.target = Entity::default();
        let mut closest = 1000.0;
        for j in &w.jelly {
            if matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving) {
                let distance = (j.motion.pos - w.sabrina.motion.pos).length();
                if distance < closest {
                    closest = distance;
                    w.crystal.magic.target = j.id
                }
            }
        }
        e.cast = true;
    }
    w.crystal.magic.time += dt;
    if w.crystal.magic.state == Spell::Charging {
        w.sabrina.throw_time = w.crystal.magic.time
    }
    match w.crystal.magic.state {
        Spell::Resting => {
            let o = crystal_orbit(w, 0.0);
            w.crystal.motion.pos = w.sabrina.motion.pos + o.offset;
            w.crystal.magic.depth = o.depth;
        }
        Spell::Charging => {
            let mut gather = (w.crystal.magic.time / 0.4).clamp(0.0, 1.0);
            gather = gather * gather * (3.0 - 2.0 * gather);
            let palm = sabrina_palm(&w.sabrina);
            w.crystal.motion.pos = (w.sabrina.motion.pos + w.crystal.magic.charge_offset)
                * (1.0 - gather)
                + palm.offset * gather;
            w.crystal.magic.depth =
                w.crystal.magic.charge_depth * (1.0 - gather) + palm.depth * gather;
            for i in 0..FISH_COUNT {
                if w.fish[i].life.active
                    && (w.fish[i].motion.pos - w.crystal.motion.pos).length()
                        < 0.7 + w.crystal.magic.time * 0.8
                {
                    touch_fish(w, i, e)
                }
            }
            if w.crystal.magic.time >= 1.1 {
                w.crystal.motion.direction = if let Some(j) = find_jelly(w, w.crystal.magic.target)
                {
                    (j.motion.pos - w.crystal.motion.pos).unit()
                } else {
                    v(
                        w.sabrina.swimming.facing * w.sabrina.swimming.pitch.cos(),
                        w.sabrina.swimming.pitch.sin(),
                    )
                };
                w.crystal.magic.state = Spell::Flying;
                w.crystal.magic.time = 0.0;
                e.launch = true;
                w.sabrina.cooldown = recharge_duration(w);
            }
        }
        Spell::Flying => {
            w.crystal.magic.depth += (1.0 - w.crystal.magic.depth) * (dt * 8.0).min(1.0);
            if let Some(j) = find_jelly(w, w.crystal.magic.target)
                && matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving)
            {
                w.crystal.motion.direction = (j.motion.pos - w.crystal.motion.pos).unit()
            }
            let from = w.crystal.motion.pos;
            w.crystal.motion.pos = w.crystal.motion.pos + w.crystal.motion.direction * dt * 12.0;
            for i in 0..FISH_COUNT {
                if w.fish[i].life.active
                    && (w.fish[i].motion.pos - w.crystal.motion.pos).length() < 1.2
                {
                    touch_fish(w, i, e)
                }
            }
            for j in &mut w.jelly {
                if !matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving) {
                    continue;
                }
                let hit = jelly_body(j).iter().any(|&surface| {
                    contact_gap(
                        Capsule {
                            a: from,
                            b: w.crystal.motion.pos,
                            radius: 0.30,
                        },
                        surface,
                    ) <= 0.0
                });
                if hit {
                    send_home(j);
                    j.behaviour.repel_flash = 0.65;
                    e.repelled += 1;
                    w.points.score += 300;
                    w.crystal.magic.state = Spell::Burst;
                    w.crystal.magic.time = 0.0;
                    break;
                }
            }
            if w.crystal.motion.pos.x.abs() > 17.0
                || w.crystal.motion.pos.y < 0.5
                || w.crystal.motion.pos.y > 18.0
                || w.crystal.magic.time > 4.0
            {
                w.crystal.magic.state = Spell::Burst;
                w.crystal.magic.time = 0.0;
            }
        }
        Spell::Burst => {
            if w.crystal.magic.time > 0.7 {
                w.crystal.magic.state = Spell::Resting;
                w.crystal.magic.time = 0.0
            }
        }
    }
}
pub fn water_wave(w: &mut World, dt: f32, input: Input, e: &mut Events) {
    let recharge = wave_recharge_duration(w);
    let s = &mut w.sabrina;
    let wave = &mut s.wave;
    if input.wave && wave.cooldown <= 0.0 && s.vitality.stunned <= 0.0 && s.throw_time < 0.0 {
        let pitch = if s.motion.velocity.length() > 0.6 {
            s.swimming.pitch
        } else {
            0.0
        };
        let side = s.swimming.yaw.cos();
        let side = if side.abs() < 0.2 {
            s.swimming.facing
        } else if side < 0.0 {
            -1.0
        } else {
            1.0
        };
        wave.direction = v(side * pitch.cos(), pitch.sin()).unit();
        wave.origin = s.motion.pos + v(0.0, 0.5) + wave.direction * 0.75;
        wave.active = true;
        wave.time = 0.0;
        wave.cast_id += 1;
        wave.cooldown = recharge;
        e.wave_cast = true;
    }
    if !wave.active {
        return;
    }
    let previous = wave.time * 8.0;
    wave.time += dt;
    let front = (wave.time * 8.0).min(5.5);
    for f in &mut w.fish {
        if !f.life.active || f.rescue.teleport > 0.0 || f.rescue.wave_hit == wave.cast_id {
            continue;
        }
        let offset = f.motion.pos - wave.origin;
        let ahead = offset.dot(wave.direction);
        let sideways = (offset.x * wave.direction.y - offset.y * wave.direction.x).abs();
        let radius = f.appearance.size * 0.45;
        if ahead < 0.0
            || ahead < previous - 0.35 - radius
            || ahead > front + 0.35 + radius
            || sideways > 0.65 + 0.38 * ahead + radius
        {
            continue;
        }
        f.rescue.wave_hit = wave.cast_id;
        f.contact.impulse = wave.direction * (5.8 - 0.15 * ahead);
        f.contact.turn_time = 1.1;
        f.rescue.wave_flow = 1.1;
        f.motion.direction = (f.motion.direction + wave.direction * 1.2).unit();
        f.swimming.cruise_heading = wave.direction.y.atan2(wave.direction.x);
        f.swimming.turn_in = 1.5;
        if w.session.water == Water::Visit && f.rescue.wave_rescued_cycle != w.session.cycle {
            for j in &w.jelly {
                if !matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving) {
                    continue;
                }
                let away = f.motion.pos - j.motion.pos;
                if away.length() < 6.0 && away.dot(wave.direction) > 0.0 {
                    f.rescue.wave_rescued_cycle = w.session.cycle;
                    w.points.score += 20;
                    break;
                }
            }
        }
    }
    if wave.time >= 0.75 {
        wave.active = false
    }
}
