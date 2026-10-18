// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
use reef_sim::{presentation::interpolate, *};

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}

fn world() -> World {
    let mut world = World::default();
    reset(
        &mut world,
        27,
        Settings {
            aquarium: true,
            ..Settings::default()
        },
    );
    world
}

#[test]
fn movement_and_pose_are_continuous_at_variable_render_rates() {
    // Constant motion makes the exact answer independent of sample placement.
    // Include mismatched refresh rates and a late frame which executes 3 ticks.
    for hz in [59.94_f64, 75.0, 144.0] {
        let mut before = world();
        let mut current = before.clone();
        let mut clock = 0.0_f64;
        let dt = 1.0_f64 / 60.0;
        let mut remainder = 0.0;
        for frame in 0..250 {
            let delta = if frame == 100 { 0.05 } else { 1.0 / hz };
            clock += delta;
            remainder += delta;
            while remainder >= dt {
                before = current.clone();
                current.session.time += dt as f32;
                current.sabrina.motion.pos.x += 6.0 * dt as f32;
                current.sabrina.swimming.phase += 2.0 * dt as f32;
                remainder -= dt;
            }
            if clock < dt {
                continue;
            }
            let render = interpolate(&before, &current, (remainder / dt) as f32);
            near(
                render.sabrina.motion.pos.x,
                -6.0 + 6.0 * (clock - dt) as f32,
            );
            near(render.sabrina.swimming.phase, 2.0 * (clock - dt) as f32);
            near(render.session.time, (clock - dt) as f32);
        }
    }
}

#[test]
fn cyclic_pose_crosses_loop_boundary_in_forward_direction() {
    let mut a = world();
    a.sabrina.swimming.phase = std::f32::consts::TAU - 0.01;
    let mut b = a.clone();
    b.sabrina.swimming.phase += 0.04;
    let render = interpolate(&a, &b, 0.5);
    near(render.sabrina.swimming.phase, std::f32::consts::TAU + 0.01);
    assert!(sabrina_pose_phase(&render.sabrina) < 0.01);
}

#[test]
fn new_entity_and_new_spell_do_not_blend_with_old_state() {
    let a = world();
    let mut b = a.clone();
    b.fish[0].id.generation += 100;
    b.fish[0].motion.pos = v(14.0, 12.0);
    b.crystal.magic.state = Spell::Burst;
    b.crystal.motion.pos = v(10.0, 4.0);
    b.sabrina.wave.active = true;
    b.sabrina.wave.cast_id = 5;
    b.sabrina.wave.time = 0.01;
    let render = interpolate(&a, &b, 0.25);
    assert_eq!(render.fish[0].motion.pos, b.fish[0].motion.pos);
    assert_eq!(render.crystal.motion.pos, b.crystal.motion.pos);
    near(render.sabrina.wave.time, 0.01);
}

#[test]
fn spells_fish_and_jelly_share_the_same_fraction() {
    let mut a = world();
    a.crystal.magic.state = Spell::Flying;
    a.sabrina.throw_time = 0.5;
    a.sabrina.wave.active = true;
    a.jelly[0].behaviour.state = JellyState::Hungry;
    let mut b = a.clone();
    b.crystal.magic.time += 0.1;
    b.crystal.motion.pos.x += 4.0;
    b.sabrina.throw_time += 0.1;
    b.sabrina.wave.time += 0.1;
    b.fish[0].motion.pos.x += 4.0;
    b.jelly[0].motion.pos.x += 4.0;
    let render = interpolate(&a, &b, 0.25);
    near(render.crystal.magic.time, 0.025);
    near(render.sabrina.throw_time, 0.525);
    near(render.sabrina.wave.time, 0.025);
    near(render.crystal.motion.pos.x, a.crystal.motion.pos.x + 1.0);
    near(render.fish[0].motion.pos.x, a.fish[0].motion.pos.x + 1.0);
    near(render.jelly[0].motion.pos.x, a.jelly[0].motion.pos.x + 1.0);
}

#[test]
fn returning_crystal_interpolates_but_does_not_blend_outbound_transition() {
    let mut before = world();
    before.crystal.magic.state = Spell::Flying;
    before.crystal.motion.pos = v(8.0, 10.0);
    let mut current = before.clone();
    current.crystal.magic.state = Spell::Returning;
    current.crystal.motion.pos = v(7.0, 9.0);
    let render = interpolate(&before, &current, 0.5);
    assert_eq!(render.crystal.motion.pos, current.crystal.motion.pos);
    before = current.clone();
    current.crystal.motion.pos = v(5.0, 7.0);
    let render = interpolate(&before, &current, 0.5);
    assert_eq!(render.crystal.motion.pos, v(6.0, 8.0));
}
