// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! One interpolated state shared by models, skeletons and attached effects.
use crate::app::{SessionUi, Simulation};
use bevy::prelude::*;
use reef_sim as sim;

#[derive(Resource, Default)]
pub struct Previous(pub Option<sim::World>);

#[derive(Resource, Default)]
pub struct Presentation(pub sim::World);

#[derive(Default)]
pub struct ReadingPerformance {
    world: Option<sim::World>,
    previous: Option<sim::World>,
    saved: Option<sim::World>,
    accumulator: f32,
    // Facing yaw owned by the simulation, kept apart from the choreographed spin.
    yaw: f32,
}

pub const READING_LOOP: f32 = 28.6;
/// Shown frozen with reduced motion: the close-up pass.
pub const READING_STILL: f32 = 11.5;
// In front of the entire route, including the enlarged close pass, but behind the camera at z=36.
const READING_FISH_DEPTH_OFFSET: f32 = 29.0;

/// Timed waypoints for the credits/help swim, in seconds and stage units.
/// The stage camera is fixed and orthographic; x/y are screen space and z
/// only decides reef occlusion, with apparent size taken from `reading_pose`.
const READING_ROUTE: [(f32, [f32; 3]); 28] = [
    // Distant entrance: offscreen beyond 21:9, behind the right ruins, then wide across.
    (0.0, [25.5, 11.3, -22.0]),
    (1.0, [19.5, 11.6, -21.0]),
    (2.0, [13.5, 12.3, -20.5]),
    (3.2, [6.0, 13.3, -19.0]),
    (4.2, [0.0, 13.6, -16.5]),
    (5.0, [-5.5, 12.6, -15.0]),
    // Behind the left rock arch, out through its opening towards the viewer.
    (5.8, [-10.0, 11.2, -20.0]),
    (6.6, [-13.2, 9.2, -24.0]),
    (7.4, [-12.6, 7.4, -23.5]),
    (8.0, [-11.0, 6.8, -20.0]),
    (8.8, [-8.0, 6.3, -11.0]),
    (9.8, [-3.5, 5.6, -3.0]),
    // Sweep close to the viewer without lingering; rise to keep the enlarged tail in frame.
    (10.5, [0.5, 6.5, 8.0]),
    (11.5, [4.2, 8.0, 16.0]),
    (12.5, [7.0, 8.5, 15.0]),
    (13.25, [9.0, 9.0, 11.0]),
    // A quick rising pirouette already carries her away from the camera.
    (13.75, [9.8, 9.3, 10.0]),
    (14.5, [10.1, 9.8, 8.0]),
    (15.25, [10.0, 10.2, 6.0]),
    (15.75, [9.0, 10.5, 3.0]),
    // Climb away and loop high behind the left reef.
    (17.0, [5.0, 10.6, -3.0]),
    (18.6, [-3.0, 12.8, -10.0]),
    (20.2, [-10.5, 15.0, -14.0]),
    (21.6, [-6.0, 16.8, -20.0]),
    (23.0, [4.0, 15.8, -23.0]),
    (24.2, [11.5, 13.2, -24.0]),
    // Recede behind the right ruins; the loop closes offscreen.
    (25.8, [15.5, 11.6, -25.0]),
    (27.2, [21.0, 11.2, -24.0]),
];

fn reading_key(index: isize) -> (f32, Vec3) {
    let count = READING_ROUTE.len() as isize;
    let (time, [x, y, z]) = READING_ROUTE[index.rem_euclid(count) as usize];
    (
        time + index.div_euclid(count) as f32 * READING_LOOP,
        Vec3::new(x, y, z),
    )
}

/// Closed C1 cubic Hermite route through timed keys, so speed follows key spacing.
pub fn reading_position(seconds: f32) -> Vec3 {
    let s = seconds.rem_euclid(READING_LOOP);
    let index = READING_ROUTE.partition_point(|&(time, _)| time <= s) as isize - 1;
    let (t0, p0) = reading_key(index - 1);
    let (t1, p1) = reading_key(index);
    let (t2, p2) = reading_key(index + 1);
    let (t3, p3) = reading_key(index + 2);
    let m1 = (p2 - p0) / (t2 - t0);
    let m2 = (p3 - p1) / (t3 - t1);
    let h = t2 - t1;
    let u = (s - t1) / h;
    let (u2, u3) = (u * u, u * u * u);
    (2.0 * u3 - 3.0 * u2 + 1.0) * p1
        + (u3 - 2.0 * u2 + u) * h * m1
        + (-2.0 * u3 + 3.0 * u2) * p2
        + (u3 - u2) * h * m2
}

fn smoothstep(start: f32, end: f32, x: f32) -> f32 {
    let t = ((x - start) / (end - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadingPose {
    pub position: Vec3,
    /// The orthographic camera needs explicit perspective-like scaling.
    pub scale: f32,
    /// Continuous pirouette yaw; adds one full turn per loop, never wrapping.
    pub spin: f32,
    /// Extra turn of the face towards the viewer, mirrored by facing.
    pub face: f32,
    /// Roll into horizontal (x/z) turns.
    pub bank: f32,
}

pub fn reading_pose(seconds: f32) -> ReadingPose {
    let position = reading_position(seconds);
    let s = seconds.rem_euclid(READING_LOOP);
    let loops = seconds.div_euclid(READING_LOOP);
    let velocity = |t: f32| (reading_position(t + 0.05) - reading_position(t - 0.05)) / 0.1;
    let v = velocity(seconds);
    // A wide difference window keeps the roll smooth across C1 key joins.
    let a = (velocity(seconds + 0.75) - velocity(seconds - 0.75)) / 1.5;
    let lateral = (v.x * a.z - v.z * a.x) / Vec2::new(v.x, v.z).length().max(0.3);
    ReadingPose {
        position,
        scale: (0.04 * position.z).exp(),
        spin: -std::f32::consts::TAU * (loops + smoothstep(13.6, 15.4, s)),
        face: std::f32::consts::FRAC_PI_6
            * smoothstep(10.0, 11.0, s)
            * (1.0 - smoothstep(12.5, 13.25, s)),
        bank: (0.4 * lateral).atan().clamp(-0.45, 0.45),
    }
}

impl ReadingPerformance {
    fn present(
        &mut self,
        rendered: &mut sim::World,
        dt: f32,
        preferences: &crate::settings::Preferences,
    ) {
        let world = self.world.get_or_insert_with(|| {
            self.saved = Some(rendered.clone());
            let mut world = sim::World::default();
            sim::reset(&mut world, 73, preferences.simulation(true));
            sim::add_fish(&mut world, 16, false);
            // Reduced motion holds the close-up instead of the offscreen loop start.
            world.session.time = if preferences.reduced_motion {
                READING_STILL
            } else {
                0.0
            };
            let p = reading_position(world.session.time);
            world.sabrina.motion.pos = sim::v(p.x, p.y);
            world.sabrina.swimming.yaw = 0.0;
            world
        });
        if !preferences.reduced_motion {
            self.accumulator += dt.min(0.1);
            while self.accumulator >= 1.0 / 60.0 {
                self.accumulator -= 1.0 / 60.0;
                let next = reading_position(world.session.time + 1.0 / 60.0);
                let now = reading_position(world.session.time);
                // Swim with full strokes whenever travelling, including depth
                // travel that is invisible on screen.
                let travel = (next - now) * 60.0;
                let speed = travel.length();
                let effort = smoothstep(0.25, 0.7, speed) * (0.55 + speed / 6.0).min(0.95);
                let movement = travel.truncate().normalize_or_zero() * effort;
                self.previous = Some(world.clone());
                world.sabrina.swimming.yaw = self.yaw;
                sim::step_reading(world, 1.0 / 60.0, sim::v(movement.x, movement.y));
                // Ease steering back in during the final spin, not as a second abrupt half-turn.
                let phase = world.session.time.rem_euclid(READING_LOOP);
                let steering = 1.0 - smoothstep(12.9, 13.25, phase) + smoothstep(14.6, 16.4, phase);
                self.yaw += (world.sabrina.swimming.yaw - self.yaw) * steering;
                world.sabrina.motion.pos = sim::v(next.x, next.y);
                world.sabrina.swimming.yaw = self.yaw + reading_pose(world.session.time).spin;
            }
        }
        if let Some(previous) = &self.previous {
            *rendered = sim::presentation::interpolate(previous, world, self.accumulator * 60.0);
        } else {
            rendered.clone_from(world);
        }
        // Offset only the presented copy so fish effects share the layer, without accumulation.
        for fish in &mut rendered.fish {
            fish.appearance.depth += READING_FISH_DEPTH_OFFSET;
        }
    }

    fn restore(&mut self, rendered: &mut sim::World) {
        if let Some(saved) = self.saved.take() {
            *rendered = saved;
        }
        self.world = None;
        self.previous = None;
        self.accumulator = 0.0;
        self.yaw = 0.0;
    }
}

pub fn prepare(
    simulation: Res<Simulation>,
    previous: Res<Previous>,
    fixed: Res<Time<Fixed>>,
    ui: Res<SessionUi>,
    mut rendered: ResMut<Presentation>,
    time: Res<Time>,
    preferences: Res<crate::settings::Preferences>,
    mut reading: Local<ReadingPerformance>,
) {
    if ui.loaded && ui.screen.is_reading() {
        reading.present(&mut rendered.0, time.delta_secs(), &preferences);
        return;
    }
    reading.restore(&mut rendered.0);
    let current = &simulation.0;
    let adjacent = previous.0.as_ref().filter(|p| {
        (p.session.time + fixed.timestep().as_secs_f32() - current.session.time).abs() < 0.00001
    });
    if let Some(previous) = adjacent {
        // Freeze the last displayed pose, rather than jumping to a tick boundary.
        if !ui.paused() {
            rendered.0 =
                sim::presentation::interpolate(previous, current, fixed.overstep_fraction());
        }
    } else {
        // Loading, retry, menu reset and deliberately frozen asset previews.
        rendered.0.clone_from(current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_route_is_smooth_closed_and_follows_the_choreography() {
        let mut previous = reading_position(-1.0 / 60.0);
        let mut max_speed: f32 = 0.0;
        for frame in 0..=(READING_LOOP * 60.0).ceil() as usize {
            let time = frame as f32 / 60.0;
            let p = reading_position(time);
            assert!(p.is_finite() && (2.0..18.0).contains(&p.y));
            let speed = (p - previous).length() * 60.0;
            max_speed = max_speed.max(speed);
            previous = p;
            let pose = reading_pose(time);
            assert!(pose.bank.abs() <= 0.45 && (0.3..2.0).contains(&pose.scale));
        }
        assert!(max_speed < 20.0, "route must not teleport ({max_speed})");
        const { assert!(READING_LOOP < 30.0) };
        assert!((reading_position(0.0) - reading_position(READING_LOOP)).length() < 1e-4);
        let speed = |t: f32| (reading_position(t + 0.1) - reading_position(t - 0.1)).length() / 0.2;
        // Start hidden beyond a 21:9 screen and small/distant.
        assert!(reading_position(0.0).x > 23.0 && reading_position(2.4).z < -18.0);
        // Out through the left rock arch opening, from behind it to in front of it.
        let arch = reading_position(7.6);
        assert!((-13.0..-9.5).contains(&arch.x) && (5.6..8.4).contains(&arch.y) && arch.z < -20.0);
        assert!(reading_position(8.8).z > -12.0);
        // Close, face-turned sweep with no lingering showcase pause.
        for t in [11.0, 11.5, 12.5] {
            let pose = reading_pose(t);
            assert!(pose.position.z > 12.0 && pose.scale > 1.6 && speed(t) > 2.5);
            assert!(pose.face > 0.4);
        }
        assert_eq!(reading_pose(8.0).face, 0.0);
        assert!(reading_pose(11.5).scale > 1.85);
        // One full turn in 1.8 seconds, rising and retreating rather than hovering.
        assert!(speed(14.5) > 2.0);
        assert_eq!(reading_pose(13.6).spin, 0.0);
        assert!((reading_pose(15.4).spin + std::f32::consts::TAU).abs() < 1e-5);
        assert!(reading_position(15.25).y - reading_position(13.25).y > 1.0);
        assert!(reading_pose(15.75).scale < reading_pose(11.5).scale * 0.65);
        // Spin is continuous across the loop seam.
        let seam = reading_pose(READING_LOOP - 0.01).spin - reading_pose(READING_LOOP + 0.01).spin;
        assert!(seam.abs() < 1e-4);
        // Wide return arc behind the reef, then recede behind the right ruins.
        assert!(reading_position(21.6).y > 16.0 && reading_position(23.0).z < -20.0);
        assert!(reading_position(28.0).x > 22.0);
        // Position and velocity remain continuous at every timed key, including the seam.
        for &(t, _) in &READING_ROUTE {
            let left = (reading_position(t) - reading_position(t - 0.005)) / 0.005;
            let right = (reading_position(t + 0.005) - reading_position(t)) / 0.005;
            assert!((left - right).length() < 0.5, "velocity jump at {t}");
        }
    }

    #[test]
    fn reading_performance_is_separate_and_restores_the_paused_pose() {
        let preferences = crate::settings::Preferences::default();
        let mut rendered = sim::World::default();
        sim::reset(&mut rendered, 27, preferences.simulation(false));
        sim::begin_visit(&mut rendered);
        rendered.session.time = 123.0;
        rendered.sabrina.motion.pos = sim::v(-7.0, 8.0);
        rendered.sabrina.vitality.health = 42.0;
        let saved = rendered.clone();
        let mut performance = ReadingPerformance::default();
        for _ in 0..600 {
            performance.present(&mut rendered, 1.0 / 60.0, &preferences);
        }
        assert!(rendered.settings.aquarium);
        assert!(rendered.session.time > 9.0);
        assert!(
            rendered
                .jelly
                .iter()
                .all(|j| j.behaviour.state == sim::JellyState::Hidden)
        );
        assert_ne!(rendered.fish[0].motion.pos, saved.fish[0].motion.pos);
        performance.restore(&mut rendered);
        assert_eq!(rendered.session.time, saved.session.time);
        assert_eq!(rendered.sabrina.motion.pos, saved.sabrina.motion.pos);
        assert_eq!(rendered.sabrina.vitality.health, 42.0);
        for i in 0..sim::FISH_COUNT {
            assert_eq!(rendered.fish[i].motion.pos, saved.fish[i].motion.pos);
            assert_eq!(
                rendered.fish[i].appearance.depth,
                saved.fish[i].appearance.depth
            );
        }
        assert_eq!(
            rendered.jelly[0].behaviour.state,
            saved.jelly[0].behaviour.state
        );
        assert!(performance.world.is_none());
    }

    #[test]
    fn pirouette_layers_over_the_simulated_facing_without_unwinding() {
        let preferences = crate::settings::Preferences::default();
        let mut rendered = sim::World::default();
        let mut performance = ReadingPerformance::default();
        let mut previous_yaw = 0.0;
        let mut previous_turn = 0.0;
        for frame in 1..=(17 * 60) {
            performance.present(&mut rendered, 1.0 / 60.0, &preferences);
            let sabrina = &performance.world.as_ref().unwrap().sabrina;
            if frame == 12 * 60 {
                assert_eq!(sim::sabrina_pose(sabrina), sim::SabrinaPose::Swim);
                assert!(
                    sabrina.swimming.pitch.abs() < 0.5,
                    "glide, not drift upright"
                );
            }
            let turn = sabrina.swimming.yaw - previous_yaw;
            if (13 * 60..=17 * 60).contains(&frame) {
                assert!(turn.abs() < 0.15, "abrupt turn at frame {frame}: {turn}");
                assert!(
                    (turn - previous_turn).abs() < 0.025,
                    "angular jerk at frame {frame}"
                );
            }
            if frame >= 15 * 60 + 30 {
                assert_ne!(sim::sabrina_pose(sabrina), sim::SabrinaPose::Idle);
            }
            previous_yaw = sabrina.swimming.yaw;
            previous_turn = turn;
        }
        let world = performance.world.as_ref().unwrap();
        assert!((-std::f32::consts::PI - 1e-3..1e-3).contains(&performance.yaw));
        let spin = world.sabrina.swimming.yaw - performance.yaw;
        assert!((spin + std::f32::consts::TAU).abs() < 1e-3);
    }

    #[test]
    fn reading_fish_stay_in_front_for_two_loops_without_depth_drift() {
        let preferences = crate::settings::Preferences::default();
        let mut rendered = sim::World::default();
        let mut performance = ReadingPerformance::default();
        for _ in 0..(READING_LOOP * 120.0).ceil() as usize {
            performance.present(&mut rendered, 1.0 / 60.0, &preferences);
            let pose = reading_pose(rendered.session.time);
            for fish in rendered.fish.iter().filter(|f| f.life.active) {
                assert!((27.3..29.6).contains(&fish.appearance.depth));
                assert!(fish.appearance.depth - pose.position.z > 10.0);
            }
            for fish in &performance.world.as_ref().unwrap().fish {
                assert!(
                    fish.appearance.depth < 1.0,
                    "offset must only affect presentation"
                );
            }
        }
    }

    #[test]
    fn reduced_motion_keeps_the_reading_background_stationary() {
        let preferences = crate::settings::Preferences {
            reduced_motion: true,
            ..default()
        };
        let mut rendered = sim::World::default();
        let mut performance = ReadingPerformance::default();
        performance.present(&mut rendered, 0.1, &preferences);
        let position = rendered.sabrina.motion.pos;
        let fish = rendered.fish[0].motion.pos;
        let fish_depth = rendered.fish[0].appearance.depth;
        for _ in 0..100 {
            performance.present(&mut rendered, 0.1, &preferences);
        }
        assert_eq!(rendered.session.time, READING_STILL);
        assert_eq!(rendered.sabrina.motion.pos, position);
        assert_eq!(rendered.fish[0].motion.pos, fish);
        assert_eq!(rendered.fish[0].appearance.depth, fish_depth);
        let hero = reading_position(READING_STILL);
        assert_eq!(position, sim::v(hero.x, hero.y));
    }
}
