// SPDX-License-Identifier: MIT
//! Peaceful intervals, visitor scheduling, recovery and scoring.
use crate::*;
pub fn begin_visit(w: &mut World) {
    if w.settings.aquarium {
        return;
    }
    w.session.water = Water::Visit;
    w.session.cycle += 1;
    w.points.score = 0;
    w.points.visit_rescues = 0;
    w.session.visit_time = 0.0;
    w.points.visit_start_fish = alive(w);
    let mut count = 1;
    if w.session.cycle >= 3
        && w.settings.visitor_limit > 1
        && (w.session.cycle % 3 == 0 || w.rng.range(0.0, 1.0) < 0.25)
    {
        count = 2
    }
    if w.session.cycle >= 6 && w.settings.visitor_limit > 2 && w.session.cycle % 4 == 0 {
        count = 3
    }
    for i in 0..JELLY_COUNT {
        w.jelly[i] = JellyEntity::default();
        if i >= count {
            continue;
        }
        let id = allocate_entity(w, EntityKind::Jelly, i as i32);
        let j = &mut w.jelly[i];
        j.id = id;
        j.behaviour.state = JellyState::Arriving;
        j.behaviour.exit_side = if i % 2 == 0 { 1 } else { -1 };
        j.motion.pos = v(j.behaviour.exit_side as f32 * 18.0, w.rng.range(5.0, 14.0));
        j.motion.direction = v(-j.behaviour.exit_side as f32, w.rng.range(-0.2, 0.2));
        j.behaviour.hunger = if w.settings.gentle { 90.0 } else { 120.0 };
        j.behaviour.kind = (w.session.cycle + i as i32) % 3;
        j.behaviour.phase = w.rng.range(0.0, 6.28);
        j.behaviour.turn_in = 4.0;
    }
}
pub fn send_home(j: &mut JellyEntity) {
    j.behaviour.state = JellyState::Leaving;
    j.behaviour.time = 0.0;
    j.behaviour.exit_side = if j.motion.pos.x < 0.0 { -1 } else { 1 };
}
pub fn finish_visit(w: &mut World, e: &mut Events) {
    w.points.last_survivors = alive(w);
    let mut kinds = [false; SPECIES_COUNT];
    for f in &w.fish {
        if f.life.active {
            kinds[f.appearance.species] = true
        }
    }
    w.points.last_variety = kinds.iter().filter(|&&x| x).count() as i32;
    w.points.score +=
        w.points.last_survivors * (75 + 5 * w.session.cycle.min(8)) + w.points.last_variety * 60;
    w.points.last_score = w.points.score;
    w.points.total_score += w.points.score;
    w.points.best_score = w.points.best_score.max(w.points.score);
    w.session.water = Water::Calm;
    w.session.rest_remaining = w.settings.rest_seconds * w.rng.range(0.85, 1.2);
    e.visit_ended = true;
}
pub fn population_and_visits(w: &mut World, dt: f32, e: &mut Events) {
    let visitors = w
        .jelly
        .iter()
        .any(|j| j.behaviour.state != JellyState::Hidden);
    if !visitors {
        w.session.spawn_in -= dt;
        if w.session.spawn_in <= 0.0 {
            let count = w.rng.range(1.0, 4.99) as i32;
            e.spawned = add_fish(w, count, true);
            w.session.spawn_in = w.rng.range(9.0, 18.0)
        }
        if alive(w) == 0 {
            e.spawned += add_fish(w, 3, true);
            w.session.spawn_in = 5.0
        }
    }
    if w.settings.aquarium {
        if w.session.water == Water::Visit {
            w.session.water = Water::Calm;
            w.session.rest_remaining = w.settings.rest_seconds;
            for j in &mut w.jelly {
                if j.behaviour.state != JellyState::Hidden {
                    send_home(j)
                }
            }
        }
    } else if w.session.water == Water::Calm {
        w.session.rest_remaining -= dt;
        if w.session.rest_remaining <= 0.0 {
            if alive(w) >= 10 {
                begin_visit(w);
                e.visit_started = true
            } else {
                w.session.rest_remaining = 5.0
            }
        }
    } else {
        w.session.visit_time += dt
    }
}
pub fn recovery_and_scoring(w: &mut World, dt: f32, e: &mut Events) {
    if !w.session.game_over && w.sabrina.vitality.heal_delay <= 0.0 {
        w.sabrina.vitality.health =
            (w.sabrina.vitality.health + dt * if w.settings.gentle { 1.2 } else { 0.8 }).min(100.0)
    }
    if !w.session.game_over && w.session.water == Water::Visit && hungry(w) == 0 {
        finish_visit(w, e)
    }
}
