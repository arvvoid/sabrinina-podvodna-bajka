// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
use reef_sim::*;

fn world() -> World {
    let mut world = World::default();
    reset(&mut world, 27, Settings::default());
    world
}

fn advance(world: &mut World, n: usize) {
    for _ in 0..n {
        step(world, 1.0 / 60.0, Input::default());
    }
}

#[test]
fn generation_handles_reject_reused_slots() {
    let mut world = world();
    let old = world.fish[0].id;
    assert!(find_fish(&world, old).is_some());
    world.fish[0].life.active = false;
    assert_eq!(add_fish(&mut world, 1, false), 1);
    assert!(find_fish(&world, old).is_none());
    let current = world.fish[0].id;
    reset(&mut world, 27, Settings::default());
    assert!(find_fish(&world, current).is_none());
    assert!(find_jelly(&world, current).is_none());
}

#[test]
fn population_waits_until_last_visitor_disappears() {
    for state in [
        JellyState::Arriving,
        JellyState::Hungry,
        JellyState::Leaving,
    ] {
        for empty in [true, false] {
            let mut world = world();
            if empty {
                for fish in &mut world.fish {
                    fish.life.active = false;
                }
            }
            world.jelly[2].behaviour.state = state;
            world.jelly[2].motion.pos = v(12.0, 12.0);
            world.session.spawn_in = 0.0;
            let count = alive(&world);
            let mut events = Events::default();
            encounters::population_and_visits(&mut world, 1.0 / 60.0, &mut events);
            assert_eq!(alive(&world), count);
            assert_eq!(world.session.spawn_in, 0.0);
            assert_eq!(events.spawned, 0);
            world.jelly[2].behaviour.state = JellyState::Hidden;
            encounters::population_and_visits(&mut world, 1.0 / 60.0, &mut events);
            assert!(alive(&world) > count);
        }
    }
}

#[test]
fn song_exhaustion_requires_a_complete_recharge() {
    let mut world = world();
    assert!(toggle_song(&mut world));
    let initial = song_radius(&world);
    advance(&mut world, 60);
    assert!(song_radius(&world) > initial);
    advance(&mut world, 720);
    assert!(world.sabrina.song.locked);
    assert!(!toggle_song(&mut world));
    advance(&mut world, 1201);
    assert_eq!(world.sabrina.song.energy, 100.0);
    assert!(!world.sabrina.song.locked);
    assert!(toggle_song(&mut world));
}

#[test]
fn voluntary_song_stop_can_restart_before_full() {
    let mut world = world();
    toggle_song(&mut world);
    advance(&mut world, 60);
    toggle_song(&mut world);
    assert!(!world.sabrina.song.locked);
    assert!(toggle_song(&mut world));
}

#[test]
fn soft_contacts_resolve_overlapping_fish_without_rewards() {
    let mut world = world();
    for fish in &mut world.fish {
        fish.life.active = false;
    }
    for i in 0..2 {
        world.fish[i].life.active = true;
        world.fish[i].motion.pos = v(0.0, 9.0);
        world.fish[i].motion.velocity = v(0.0, 0.0);
    }
    soft_contacts(&mut world);
    assert!((world.fish[0].motion.pos - world.fish[1].motion.pos).length() > 0.4);
    assert_eq!(world.points.score, 0);
    assert_eq!(world.sabrina.vitality.health, 100.0);
}

#[test]
fn wave_pushes_fish_and_damages_close_jellyfish_once() {
    let mut world = world();
    world.sabrina.motion.pos = v(0.0, 8.0);
    for fish in &mut world.fish {
        fish.life.active = false;
    }
    world.fish[0].life.active = true;
    world.fish[0].motion.pos = v(2.0, 8.5);
    world.jelly[0].behaviour.state = JellyState::Hungry;
    world.jelly[0].motion.pos = v(1.5, 8.5);
    let mut events = Events::default();
    magic::water_wave(
        &mut world,
        0.016,
        Input {
            wave: true,
            ..Input::default()
        },
        &mut events,
    );
    assert!(events.wave_cast);
    for _ in 0..25 {
        magic::water_wave(&mut world, 0.016, Input::default(), &mut events);
    }
    assert!(world.fish[0].contact.impulse.x > 4.0);
    assert_eq!(world.jelly[0].behaviour.health, 90.0);
    assert!(world.jelly[0].behaviour.knockback.x > 0.0);
    let cast = world.sabrina.wave.cast_id;
    magic::water_wave(
        &mut world,
        0.016,
        Input {
            wave: true,
            ..Input::default()
        },
        &mut events,
    );
    assert_eq!(world.sabrina.wave.cast_id, cast);
}

#[test]
fn all_contacting_fish_are_eaten_and_grow_the_visitor() {
    let mut world = world();
    world.sabrina.motion.pos = v(-12.0, 14.0);
    for fish in &mut world.fish {
        fish.life.active = false;
    }
    let jellyfish = &mut world.jelly[0];
    jellyfish.behaviour.state = JellyState::Hungry;
    jellyfish.motion.pos = v(0.0, 8.0);
    jellyfish.behaviour.turn_in = 10.0;
    let before = jelly_scale(jellyfish);
    for i in 0..3 {
        world.fish[i].life.active = true;
        world.fish[i].motion.pos = v(i as f32 * 0.1, 8.0);
    }
    let mut events = Events::default();
    jellyfish::jellyfish_and_contacts(&mut world, 0.0, &mut events);
    assert_eq!(events.eaten, 3);
    assert_eq!(world.jelly[0].behaviour.eaten, 3);
    assert!(jelly_scale(&world.jelly[0]) > before);
    assert!(
        world.fish[..3]
            .iter()
            .all(|fish| !fish.life.active && fish.life.pop_time > 0.0)
    );
}

#[test]
fn sting_stuns_and_terminal_state_is_frozen() {
    let mut world = world();
    for fish in &mut world.fish {
        fish.life.active = false;
    }
    world.sabrina.vitality.health = 20.0;
    world.jelly[0].motion.pos = world.sabrina.motion.pos;
    world.jelly[0].behaviour.state = JellyState::Hungry;
    let events = step(&mut world, 1.0 / 60.0, Input::default());
    assert!(events.hit && events.game_over);
    assert_eq!(world.sabrina.vitality.health, 0.0);
    let pos = world.sabrina.motion.pos;
    let t = world.session.time;
    advance(&mut world, 200);
    assert_eq!(world.session.time, t);
    assert_eq!(world.sabrina.motion.pos, pos);
}

#[test]
fn crystal_gathers_to_palm_before_release_at_every_heading() {
    for yaw in [0.0, -1.5, -std::f32::consts::PI] {
        for pitch in [-1.1, 0.0, 1.1] {
            let mut world = world();
            world.sabrina.swimming.yaw = yaw;
            world.sabrina.swimming.pitch = pitch;
            let mut events = Events::default();
            magic::crystal(
                &mut world,
                0.5,
                Input {
                    cast: true,
                    ..Input::default()
                },
                &mut events,
            );
            assert_eq!(world.crystal.magic.state, Spell::Charging);
            let palm = sabrina_palm(&world.sabrina);
            assert!((palm.offset - world.crystal.motion.pos).length() < 0.0001);
            assert!((palm.depth - world.crystal.magic.depth).abs() < 0.0001);
            assert_eq!(world.sabrina.cooldown, 0.0);
            magic::crystal(&mut world, 0.61, Input::default(), &mut events);
            assert_eq!(world.crystal.magic.state, Spell::Flying);
            assert!(events.launch);
            assert_eq!(world.sabrina.cooldown, 12.0);
        }
    }
}

#[test]
fn time_step_is_capped_and_nonpositive_steps_do_nothing() {
    let mut a = world();
    let mut b = a.clone();
    step(&mut a, 1.0, Input::default());
    step(&mut b, 1.0 / 30.0, Input::default());
    assert_eq!(a.sabrina.motion.pos, b.sabrina.motion.pos);
    assert_eq!(a.session.time, b.session.time);
    let t = a.session.time;
    step(&mut a, -1.0, Input::default());
    assert_eq!(a.session.time, t);
}

#[test]
fn seeded_rng_matches_mt19937_reference() {
    let mut r = world::Rng::new(5489);
    assert_eq!(r.next_u32(), 3499211612);
    assert_eq!(r.next_u32(), 581869302);
    assert_eq!(r.next_u32(), 3890346734);
}
