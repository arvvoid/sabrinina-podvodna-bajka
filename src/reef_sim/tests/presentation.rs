// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
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

#[test]
fn cast_arms_follow_spell_direction_despite_body_pitch_and_turning() {
    for yaw in [0.0, -1.1, std::f32::consts::PI] {
        for pitch in [-1.1, 0.0, 0.65, 1.1] {
            for direction in [v(1.0, 0.0), v(-1.0, 0.0), v(0.6, 0.8), v(-0.6, -0.8)] {
                for wave in [true, false] {
                    let mut s = SabrinaEntity::default();
                    s.swimming.yaw = yaw;
                    s.swimming.pitch = pitch;
                    s.wave.active = wave;
                    s.wave.time = 0.2;
                    s.wave.direction = direction;
                    s.throw_time = if wave { -1.0 } else { 1.1 };
                    s.throw_direction = direction;
                    let yaw = sabrina_visual_yaw(&s);
                    let (axis, angle) = sabrina_cast_rotation(&s);
                    let x = angle.cos();
                    let y = axis[2] * angle.sin();
                    let z = -axis[1] * angle.sin();
                    let xy = v(x, y).rotate(pitch);
                    near(xy.x * yaw.cos() + z * yaw.sin(), direction.x);
                    near(xy.y, direction.y);
                    near(-xy.x * yaw.sin() + z * yaw.cos(), 0.0);
                    let palm = sabrina_palm(&s);
                    let shoulder = sabrina_body(&s)[14].a;
                    let reach = palm.offset - shoulder;
                    assert!(
                        reach.dot(direction) > 0.55,
                        "cast must extend toward spell: {reach:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn idle_sculls_moving_arms_glide_and_cast_release_is_not_mid_charge() {
    let mut s = SabrinaEntity::default();
    s.swimming.pitch = 0.0;
    for (speed, minimum, maximum) in [(0.0, 0.15, 1.0), (5.0, 0.01, 0.4)] {
        s.motion.velocity = v(speed, 0.0);
        let mut points = Vec::new();
        for i in 0..32 {
            s.swimming.phase = i as f32 / 32.0 * std::f32::consts::TAU;
            points.push(sabrina_palm(&s).offset);
        }
        let travel = points
            .iter()
            .flat_map(|a| points.iter().map(move |b| (*a - *b).length()))
            .fold(0.0_f32, f32::max);
        assert!(
            travel > minimum && travel < maximum,
            "{speed}: palm travel {travel}"
        );
    }
    s.throw_time = 0.8;
    let gathered = sabrina_palm(&s).offset;
    s.throw_time = 1.1;
    let release = sabrina_palm(&s).offset;
    assert!(release.x - gathered.x > 0.6);
}

#[test]
fn charging_wave_holds_hands_forward_and_releases_without_pose_jump() {
    for facing in [-1.0, 1.0] {
        let mut s = SabrinaEntity::default();
        s.swimming.pitch = 0.0;
        s.swimming.yaw = if facing < 0.0 {
            std::f32::consts::PI
        } else {
            0.0
        };
        s.wave.charging = true;
        s.wave.direction = v(facing, 0.0);
        s.wave.charge_time = 2.5;
        assert_eq!(sabrina_pose(&s), SabrinaPose::Wave);
        let held = sabrina_palm(&s);
        assert!((held.offset.x - s.motion.pos.x) * facing > 1.0, "{held:?}");
        let phase = sabrina_pose_phase(&s);
        let rotation = sabrina_cast_rotation(&s);
        s.wave.charging = false;
        s.wave.active = true;
        s.wave.time = 0.0;
        near(sabrina_pose_phase(&s), phase);
        let released = sabrina_palm(&s);
        near(released.offset.x, held.offset.x);
        near(released.offset.y, held.offset.y);
        assert_eq!(sabrina_cast_rotation(&s), rotation);
    }
}

#[test]
fn three_quarter_view_faces_camera_on_both_sides_without_flipping_turns() {
    let mut s = SabrinaEntity::default();
    for yaw in [0.0, std::f32::consts::PI] {
        s.swimming.yaw = yaw;
        let visual = sabrina_visual_yaw(&s);
        near((visual - yaw).abs(), std::f32::consts::FRAC_PI_6);
        assert!(-visual.sin() > 0.49);
        assert!(visual.cos() * yaw.cos() > 0.85);
    }
    s.swimming.yaw = -1.0;
    let before = sabrina_visual_yaw(&s);
    s.swimming.yaw -= 0.001;
    assert!((sabrina_visual_yaw(&s) - before).abs() < 0.002);
}

#[test]
fn swimming_arms_follow_direction_but_do_not_override_idle_or_spells() {
    let mut s = SabrinaEntity::default();
    s.swimming.pitch = -0.8;
    assert_eq!(sabrina_pose(&s), SabrinaPose::Idle);
    for facing in [-1.0, 1.0] {
        s.swimming.yaw = if facing < 0.0 {
            -std::f32::consts::PI
        } else {
            0.0
        };
        s.swimming.facing = facing;
        for (velocity, pitch, expected) in [
            (v(facing * 5.0, 0.0), 0.0, SabrinaPose::Swim),
            (v(facing * 3.0, 5.0), 0.8, SabrinaPose::Swim),
            (v(facing * 3.0, -5.0), -0.8, SabrinaPose::Dive),
            (v(0.0, -5.0), -1.1, SabrinaPose::Dive),
        ] {
            s.motion.velocity = velocity;
            s.swimming.pitch = pitch;
            assert_eq!(sabrina_pose(&s), expected);
            for frame in 0..32 {
                s.swimming.phase = frame as f32 / 32.0 * std::f32::consts::TAU;
                let palm = sabrina_palm(&s).offset - s.motion.pos;
                let forward = v(facing * pitch.cos(), pitch.sin());
                if expected == SabrinaPose::Dive {
                    assert!(palm.dot(forward) > 1.2, "Dive hands must lead the body");
                } else {
                    let shoulder = sabrina_body(&s)[14].a - s.motion.pos;
                    assert!(
                        (palm - shoulder).dot(forward) < 0.0,
                        "Gliding hands must trail the shoulder: {palm:?}, {shoulder:?}"
                    );
                }
            }
        }
    }
    s.swimming.yaw = -1.0;
    assert_eq!(sabrina_pose(&s), SabrinaPose::Dive);
    s.song.active = true;
    assert_eq!(sabrina_pose(&s), SabrinaPose::Sing);
    s.wave.active = true;
    assert_eq!(sabrina_pose(&s), SabrinaPose::Wave);
    s.throw_time = 0.5;
    assert_eq!(sabrina_pose(&s), SabrinaPose::Throw);
    s.vitality.hurt_flash = 0.1;
    assert_eq!(sabrina_pose(&s), SabrinaPose::Hurt);
}
