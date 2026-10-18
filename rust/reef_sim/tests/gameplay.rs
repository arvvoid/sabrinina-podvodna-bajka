// SPDX-License-Identifier: MIT
use reef_sim::*;
fn world() -> World {
    let mut w = World::default();
    reset(&mut w, 27, Settings::default());
    w
}
fn advance(w: &mut World, n: usize) {
    for _ in 0..n {
        step(w, 1.0 / 60.0, Input::default());
    }
}
#[test]
fn generation_handles_reject_reused_slots() {
    let mut w = world();
    let old = w.fish[0].id;
    assert!(find_fish(&w, old).is_some());
    w.fish[0].life.active = false;
    assert_eq!(add_fish(&mut w, 1, false), 1);
    assert!(find_fish(&w, old).is_none());
    let current = w.fish[0].id;
    reset(&mut w, 27, Settings::default());
    assert!(find_fish(&w, current).is_none());
    assert!(find_jelly(&w, current).is_none());
}
#[test]
fn population_waits_until_last_visitor_disappears() {
    for state in [
        JellyState::Arriving,
        JellyState::Hungry,
        JellyState::Leaving,
    ] {
        for empty in [true, false] {
            let mut w = world();
            if empty {
                for f in &mut w.fish {
                    f.life.active = false;
                }
            }
            w.jelly[2].behaviour.state = state;
            w.jelly[2].motion.pos = v(12.0, 12.0);
            w.session.spawn_in = 0.0;
            let count = alive(&w);
            let mut e = Events::default();
            encounters::population_and_visits(&mut w, 1.0 / 60.0, &mut e);
            assert_eq!(alive(&w), count);
            assert_eq!(w.session.spawn_in, 0.0);
            assert_eq!(e.spawned, 0);
            w.jelly[2].behaviour.state = JellyState::Hidden;
            encounters::population_and_visits(&mut w, 1.0 / 60.0, &mut e);
            assert!(alive(&w) > count);
        }
    }
}
#[test]
fn song_exhaustion_requires_a_complete_recharge() {
    let mut w = world();
    assert!(toggle_song(&mut w));
    let initial = song_radius(&w);
    advance(&mut w, 60);
    assert!(song_radius(&w) > initial);
    advance(&mut w, 720);
    assert!(w.sabrina.song.locked);
    assert!(!toggle_song(&mut w));
    advance(&mut w, 1201);
    assert_eq!(w.sabrina.song.energy, 100.0);
    assert!(!w.sabrina.song.locked);
    assert!(toggle_song(&mut w));
}
#[test]
fn voluntary_song_stop_can_restart_before_full() {
    let mut w = world();
    toggle_song(&mut w);
    advance(&mut w, 60);
    toggle_song(&mut w);
    assert!(!w.sabrina.song.locked);
    assert!(toggle_song(&mut w));
}
#[test]
fn soft_contacts_resolve_overlapping_fish_without_rewards() {
    let mut w = world();
    for f in &mut w.fish {
        f.life.active = false;
    }
    for i in 0..2 {
        w.fish[i].life.active = true;
        w.fish[i].motion.pos = v(0.0, 9.0);
        w.fish[i].motion.velocity = v(0.0, 0.0);
    }
    soft_contacts(&mut w);
    assert!((w.fish[0].motion.pos - w.fish[1].motion.pos).length() > 0.4);
    assert_eq!(w.points.score, 0);
    assert_eq!(w.sabrina.vitality.health, 100.0);
}
#[test]
fn wave_pushes_fish_once_and_does_not_push_jellyfish() {
    let mut w = world();
    w.sabrina.motion.pos = v(0.0, 8.0);
    for f in &mut w.fish {
        f.life.active = false;
    }
    w.fish[0].life.active = true;
    w.fish[0].motion.pos = v(2.0, 8.5);
    w.jelly[0].behaviour.state = JellyState::Hungry;
    w.jelly[0].motion.pos = v(1.0, 8.0);
    let before = w.jelly[0].motion.pos;
    let mut e = Events::default();
    magic::water_wave(
        &mut w,
        0.016,
        Input {
            wave: true,
            ..Input::default()
        },
        &mut e,
    );
    assert!(e.wave_cast);
    for _ in 0..25 {
        magic::water_wave(&mut w, 0.016, Input::default(), &mut e);
    }
    assert!(w.fish[0].contact.impulse.x > 4.0);
    assert_eq!(w.jelly[0].motion.pos, before);
    let cast = w.sabrina.wave.cast_id;
    magic::water_wave(
        &mut w,
        0.016,
        Input {
            wave: true,
            ..Input::default()
        },
        &mut e,
    );
    assert_eq!(w.sabrina.wave.cast_id, cast);
}
#[test]
fn all_contacting_fish_are_eaten_and_grow_the_visitor() {
    let mut w = world();
    w.sabrina.motion.pos = v(-12.0, 14.0);
    for f in &mut w.fish {
        f.life.active = false;
    }
    let j = &mut w.jelly[0];
    j.behaviour.state = JellyState::Hungry;
    j.motion.pos = v(0.0, 8.0);
    j.behaviour.turn_in = 10.0;
    let before = jelly_scale(j);
    for i in 0..3 {
        w.fish[i].life.active = true;
        w.fish[i].motion.pos = v(i as f32 * 0.1, 8.0);
    }
    let mut e = Events::default();
    jellyfish::jellyfish_and_contacts(&mut w, 0.0, &mut e);
    assert_eq!(e.eaten, 3);
    assert_eq!(w.jelly[0].behaviour.eaten, 3);
    assert!(jelly_scale(&w.jelly[0]) > before);
    assert!(
        w.fish[..3]
            .iter()
            .all(|f| !f.life.active && f.life.pop_time > 0.0)
    );
}
#[test]
fn sting_stuns_and_terminal_state_is_frozen() {
    let mut w = world();
    for f in &mut w.fish {
        f.life.active = false;
    }
    w.sabrina.vitality.health = 20.0;
    w.jelly[0].motion.pos = w.sabrina.motion.pos;
    w.jelly[0].behaviour.state = JellyState::Hungry;
    let e = step(&mut w, 1.0 / 60.0, Input::default());
    assert!(e.hit && e.game_over);
    assert_eq!(w.sabrina.vitality.health, 0.0);
    let pos = w.sabrina.motion.pos;
    let t = w.session.time;
    advance(&mut w, 200);
    assert_eq!(w.session.time, t);
    assert_eq!(w.sabrina.motion.pos, pos);
}
#[test]
fn crystal_gathers_to_palm_before_release_at_every_heading() {
    for yaw in [0.0, -1.5, -std::f32::consts::PI] {
        for pitch in [-1.1, 0.0, 1.1] {
            let mut w = world();
            w.sabrina.swimming.yaw = yaw;
            w.sabrina.swimming.pitch = pitch;
            let mut e = Events::default();
            magic::crystal(
                &mut w,
                0.5,
                Input {
                    cast: true,
                    ..Input::default()
                },
                &mut e,
            );
            assert_eq!(w.crystal.magic.state, Spell::Charging);
            let palm = sabrina_palm(&w.sabrina);
            assert!((palm.offset - w.crystal.motion.pos).length() < 0.0001);
            assert!((palm.depth - w.crystal.magic.depth).abs() < 0.0001);
            assert_eq!(w.sabrina.cooldown, 0.0);
            magic::crystal(&mut w, 0.61, Input::default(), &mut e);
            assert_eq!(w.crystal.magic.state, Spell::Flying);
            assert!(e.launch);
            assert_eq!(w.sabrina.cooldown, 150.0);
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
