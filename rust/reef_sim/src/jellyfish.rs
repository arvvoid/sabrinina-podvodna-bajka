// SPDX-License-Identifier: MIT
//! Visitor movement, feeding and contact damage.
use crate::*;
pub fn jellyfish_and_contacts(w: &mut World, dt: f32, e: &mut Events) {
    for index in 0..JELLY_COUNT {
        let mut j = w.jelly[index];
        if j.behaviour.state == JellyState::Hidden {
            continue;
        }
        j.behaviour.time += dt;
        j.behaviour.repel_flash = (j.behaviour.repel_flash - dt).max(0.0);
        j.behaviour.song_check = (j.behaviour.song_check - dt).max(0.0);
        j.behaviour.song_interest = (j.behaviour.song_interest - dt).max(0.0);
        if !w.sabrina.song.active {
            j.behaviour.song_interest = 0.0
        } else if j.behaviour.song_check <= 0.0 && j.behaviour.state != JellyState::Leaving {
            j.behaviour.song_check = 2.5;
            if w.rng.range(0.0, 1.0) < (0.15 + 0.05 * w.session.cycle as f32).min(0.50) {
                j.behaviour.song_interest = 3.5
            }
        }
        j.behaviour.eat_wait = (j.behaviour.eat_wait - dt).max(0.0);
        match j.behaviour.state {
            JellyState::Arriving => {
                j.motion.pos = j.motion.pos
                    + v(
                        -j.behaviour.exit_side as f32 * 1.15,
                        (w.session.time + j.behaviour.phase).sin() * 0.15,
                    ) * dt;
                if j.behaviour.time > 4.5 {
                    j.behaviour.state = JellyState::Hungry;
                    j.behaviour.time = 0.0
                }
            }
            JellyState::Leaving => {
                j.motion.pos = j.motion.pos + v(j.behaviour.exit_side as f32 * 1.9, 0.2) * dt;
                if j.motion.pos.x.abs() > 19.0 || j.behaviour.time > 9.0 {
                    j.behaviour.state = JellyState::Hidden
                }
            }
            _ => {
                j.behaviour.hunger -= dt;
                j.behaviour.turn_in -= dt;
                if j.behaviour.turn_in <= 0.0 {
                    j.motion.direction = v(w.rng.range(-1.0, 1.0), w.rng.range(-0.65, 0.65)).unit();
                    j.behaviour.turn_in = w.rng.range(3.0, 6.0)
                }
                let mut wish = j.motion.direction * 0.75;
                if j.behaviour.kind != 0 {
                    let mut nearest = 100.0;
                    let mut target = v(0.0, 0.0);
                    for f in &w.fish {
                        if f.life.active && (f.motion.pos - j.motion.pos).length() < nearest {
                            nearest = (f.motion.pos - j.motion.pos).length();
                            target = f.motion.pos
                        }
                    }
                    if nearest < 5.5 {
                        wish = (target - j.motion.pos).unit()
                    }
                }
                if w.sabrina.song.active && j.behaviour.song_interest > 0.0 {
                    wish = (w.sabrina.motion.pos - j.motion.pos).unit() * 1.1
                }
                if j.motion.pos.x > 14.0 {
                    wish.x = -1.0
                }
                if j.motion.pos.x < -14.0 {
                    wish.x = 1.0
                }
                if j.motion.pos.y > 15.0 {
                    wish.y = -0.6
                }
                if j.motion.pos.y < 3.0 {
                    wish.y = 0.6
                }
                wish.y += (w.session.time * 1.4 + j.behaviour.phase).sin() * 0.3;
                let speed = if w.settings.gentle { 0.8 } else { 1.1 }
                    + 0.04 * w.session.cycle.min(6) as f32;
                j.motion.velocity =
                    j.motion.velocity + (wish * speed - j.motion.velocity) * (dt * 2.0).min(1.0);
                j.motion.pos = j.motion.pos + j.motion.velocity * dt;
            }
        }
        if matches!(j.behaviour.state, JellyState::Hungry | JellyState::Arriving) {
            let stinging = jelly_body(&j).iter().any(|&surface| {
                sabrina_body(&w.sabrina)
                    .iter()
                    .any(|&part| contact_gap(surface, part) <= 0.0)
            });
            if stinging {
                for _ in 0..6 {
                    let mut correction = v(0.0, 0.0);
                    for surface in jelly_body(&j) {
                        for part in sabrina_body(&w.sabrina) {
                            let candidate = contact_displacement(part, surface);
                            if candidate.length() > correction.length() {
                                correction = candidate
                            }
                        }
                    }
                    if correction.length() < 0.001 {
                        break;
                    }
                    w.sabrina.motion.pos = bounds(w.sabrina.motion.pos + correction);
                    w.sabrina.motion.pos.y = w.sabrina.motion.pos.y.max(3.2);
                }
                let normal = (w.sabrina.motion.pos - j.motion.pos).unit();
                let closing = w.sabrina.motion.velocity.dot(normal);
                if closing < 0.0 {
                    w.sabrina.motion.velocity =
                        w.sabrina.motion.velocity - normal * (1.15 * closing)
                }
            }
            if stinging && w.sabrina.vitality.invulnerable <= 0.0 {
                let s = &mut w.sabrina;
                s.vitality.stunned = if w.settings.gentle { 2.0 } else { 3.0 };
                s.vitality.invulnerable = s.vitality.stunned + 3.0;
                s.song.active = false;
                s.vitality.health =
                    (s.vitality.health - if w.settings.gentle { 28.0 } else { 35.0 }).max(0.0);
                s.vitality.heal_delay = 8.0;
                s.vitality.hurt_flash = 0.85;
                w.points.score = (w.points.score - 100).max(0);
                e.hit = true;
                if s.vitality.health <= 0.0 {
                    w.session.game_over = true;
                    e.game_over = true;
                    s.motion.velocity = v(0.0, 0.0);
                    s.song.strength = 0.0
                }
            }
            for f in &mut w.fish {
                if f.life.active && f.rescue.teleport <= 0.0 {
                    let touching = jelly_body(&j)
                        .iter()
                        .any(|&surface| contact_gap(fish_body(f), surface) <= 0.0);
                    if touching {
                        for _ in 0..3 {
                            for surface in jelly_body(&j) {
                                f.motion.pos =
                                    f.motion.pos + contact_displacement(fish_body(f), surface)
                            }
                        }
                        if f.rescue.shield > 0.0 {
                            f.contact.impulse = (f.motion.pos - j.motion.pos).unit() * 3.0;
                            f.contact.turn_time = 0.8;
                            continue;
                        }
                        f.life.active = false;
                        f.life.pop_time = 0.55;
                        f.motion.velocity = v(0.0, 0.0);
                        j.behaviour.eaten += 1;
                        w.points.total_eaten += 1;
                        e.eaten += 1;
                        j.behaviour.eat_wait = 0.55;
                    }
                }
            }
            if j.behaviour.hunger <= 0.0
                || (j.behaviour.eaten >= if w.settings.gentle { 10 } else { 14 }
                    && w.session.visit_time >= if w.settings.gentle { 60.0 } else { 80.0 })
                || w.session.visit_time > if w.settings.gentle { 100.0 } else { 130.0 }
            {
                send_home(&mut j);
                w.points.score += 150;
                e.leaving = true;
            }
        }
        w.jelly[index] = j;
    }
    for _ in 0..3 {
        for i in 0..JELLY_COUNT {
            for k in i + 1..JELLY_COUNT {
                let (a, b) = (w.jelly[i], w.jelly[k]);
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
                w.jelly[i].motion.pos = a.motion.pos + correction * 0.5;
                w.jelly[k].motion.pos = b.motion.pos - correction * 0.5;
            }
        }
    }
}
