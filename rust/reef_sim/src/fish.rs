// SPDX-License-Identifier: MIT
//! Steering and swimming for the bounded fish population.
use crate::*;
pub fn fish_swimming(w: &mut World, dt: f32) {
    for i in 0..FISH_COUNT {
        let mut f = w.fish[i];
        if !f.life.active {
            continue;
        }
        f.rescue.wave_flow = (f.rescue.wave_flow - dt).max(0.0);
        f.contact.impulse =
            f.contact.impulse * (-(if f.rescue.wave_flow > 0.0 { 1.25 } else { 3.5 }) * dt).exp();
        f.contact.turn_time = (f.contact.turn_time - dt).max(0.0);
        f.appearance.birth = (f.appearance.birth + dt).min(1.0);
        f.rescue.shield = (f.rescue.shield - dt).max(0.0);
        if f.rescue.teleport > 0.0 {
            f.rescue.teleport = (f.rescue.teleport - dt).max(0.0);
            let u = 1.0 - f.rescue.teleport / 1.2;
            let u = u * u * (3.0 - 2.0 * u);
            f.motion.pos = f.rescue.teleport_from * (1.0 - u) + f.rescue.teleport_to * u;
            w.fish[i] = f;
            continue;
        }
        f.swimming.phase += dt * (0.85 + 0.18 * f.appearance.size);
        f.swimming.turn_in -= dt;
        if f.swimming.turn_in <= 0.0 {
            f.swimming.cruise_heading = if f.motion.direction.x < 0.0 {
                3.14159
            } else {
                0.0
            } + w.rng.range(-0.32, 0.32);
            f.swimming.turn_in = w.rng.range(4.0, 9.0)
        }
        if f.motion.pos.x > 12.3 && f.swimming.cruise_heading.cos() > 0.0 {
            f.swimming.cruise_heading = 3.14159 + w.rng.range(-0.25, 0.25);
            f.swimming.turn_in = 6.0
        }
        if f.motion.pos.x < -12.3 && f.swimming.cruise_heading.cos() < 0.0 {
            f.swimming.cruise_heading = w.rng.range(-0.25, 0.25);
            f.swimming.turn_in = 6.0
        }
        let cruise = 0.85 + f.appearance.size * 0.70;
        let desired_heading = f.swimming.cruise_heading + 0.12 * f.swimming.phase.sin();
        let mut want = v(
            desired_heading.cos() * cruise,
            desired_heading.sin() * cruise,
        );
        if f.motion.pos.x > 12.3 {
            want.x -= ((f.motion.pos.x - 12.3) * 2.5).min(6.0)
        }
        if f.motion.pos.x < -12.3 {
            want.x += ((-12.3 - f.motion.pos.x) * 2.5).min(6.0)
        }
        if f.motion.pos.y > 14.3 {
            want.y -= (f.motion.pos.y - 14.3) * 2.0
        }
        if f.motion.pos.y < 3.8 {
            want.y += (3.8 - f.motion.pos.y) * 2.0
        }
        let mut separation = v(0.0, 0.0);
        let mut neighbours = 0;
        let mut school = v(0.0, 0.0);
        for (k, other) in w.fish.iter().enumerate() {
            if k == i || !other.life.active {
                continue;
            }
            let d = (f.motion.pos - other.motion.pos).length();
            if d < 0.7 && d > 0.01 {
                separation = separation + (f.motion.pos - other.motion.pos).unit() * (0.7 - d)
            }
            if other.appearance.species == f.appearance.species && d < 3.0 {
                school = school + other.motion.velocity;
                neighbours += 1;
            }
        }
        want = want + separation * 0.8;
        if neighbours > 0 {
            want = want * 0.8 + school * (0.2 / neighbours as f32)
        }
        for j in &w.jelly {
            if j.behaviour.state == JellyState::Hungry {
                let away = f.motion.pos - j.motion.pos;
                let d = away.length();
                if d < 3.4 {
                    want = want + away.unit() * (3.4 - d) * 0.8
                }
            }
        }
        let away = f.motion.pos - w.sabrina.motion.pos;
        let radius = song_radius(w);
        let hearing = w.sabrina.song.active
            && w.sabrina.song.strength > 0.05
            && away.length() < radius
            && f.rescue.wave_flow <= 0.0;
        if hearing {
            let wake = if w.sabrina.motion.velocity.length() > 0.4 {
                w.sabrina.motion.velocity.unit() * -(radius * 0.22).min(1.4)
            } else {
                v(0.0, 0.0)
            };
            let angle = f.swimming.phase + w.session.time * 0.22;
            let slot = w.sabrina.motion.pos
                + wake
                + v(
                    angle.cos() * (radius * 0.40).min(1.5 + 0.18 * f.appearance.species as f32),
                    angle.sin() * (radius * 0.30).min(1.0 + 0.12 * f.appearance.species as f32),
                );
            let pull = slot - f.motion.pos;
            let pulse = 0.85 + 0.15 * (w.session.time * 2.2 + f.swimming.phase).sin();
            want = want * (1.0 - w.sabrina.song.strength * 0.85)
                + (pull.unit() * (pull.length() * 2.0).min(3.8) * pulse
                    + w.sabrina.motion.velocity * 1.05)
                    * w.sabrina.song.strength;
        } else if !w.sabrina.song.active && hungry(w) == 0 && away.length() < 1.5 {
            let mut push = away.unit();
            let mut nearest = 1000.0;
            for j in &w.jelly {
                if j.behaviour.state == JellyState::Hungry
                    && (j.motion.pos - f.motion.pos).length() < nearest
                {
                    nearest = (j.motion.pos - f.motion.pos).length();
                    push = (f.motion.pos - j.motion.pos).unit();
                }
            }
            want = want + push * 2.5;
        }
        let mut danger = 1000.0_f32;
        for j in &w.jelly {
            if matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving) {
                danger = danger.min((f.motion.pos - j.motion.pos).length())
            }
        }
        f.rescue.song_escort = if hearing {
            f.rescue.song_escort + dt
        } else {
            0.0
        };
        if hearing && danger < 5.0 && w.session.water == Water::Visit {
            f.rescue.song_risk_cycle = w.session.cycle
        }
        if hearing
            && danger >= 6.0
            && f.rescue.song_escort >= 1.0
            && w.session.water == Water::Visit
            && f.rescue.song_risk_cycle == w.session.cycle
            && f.rescue.song_rescued_cycle != w.session.cycle
        {
            f.rescue.song_rescued_cycle = w.session.cycle;
            w.points.score += 30;
        }
        let mut heading = f.motion.direction.y.atan2(f.motion.direction.x);
        let target = want.y.atan2(want.x);
        let change = (target - heading).sin().atan2((target - heading).cos());
        let turn_rate = 1.35
            + if hearing { 1.85 } else { 0.0 }
            + if f.contact.turn_time > 0.0 { 1.1 } else { 0.0 };
        heading += change.clamp(-turn_rate * dt, turn_rate * dt);
        f.motion.direction = v(heading.cos(), heading.sin());
        let mut desired_speed = want.length().clamp(0.55, if hearing { 6.8 } else { 3.8 });
        desired_speed *= 1.0 - 0.48 * (change.abs() / 1.5).min(1.0);
        f.swimming.speed +=
            (desired_speed - f.swimming.speed) * (dt * if hearing { 3.5 } else { 1.8 }).min(1.0);
        f.swimming.tail_phase += dt * (6.0 + f.swimming.speed * 5.0);
        let thrust = 1.0 + 0.08 * (f.swimming.tail_phase * 2.0).sin();
        f.motion.velocity = f.motion.direction * (f.swimming.speed * thrust) + f.contact.impulse;
        f.motion.pos = bounds(f.motion.pos + f.motion.velocity * dt);
        w.fish[i] = f;
    }
}
