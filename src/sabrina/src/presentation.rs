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
}

// A closed, smooth route: behind the ruins, around the fish, then close to the viewer.
pub fn reading_position(seconds: f32) -> Vec3 {
    let points = [
        Vec3::new(11.0, 9.0, 1.0),
        Vec3::new(18.0, 5.0, -5.0),
        Vec3::new(7.0, 3.5, -7.0),
        Vec3::new(-17.0, 7.0, -6.0),
        Vec3::new(-11.0, 14.0, -2.0),
        Vec3::new(5.0, 12.0, 2.0),
        Vec3::new(12.0, 6.0, 5.0),
        Vec3::new(-9.0, 6.5, 4.0),
        Vec3::new(-19.0, 11.0, -4.0),
        Vec3::new(2.0, 15.0, -5.0),
    ];
    let phase = seconds.rem_euclid(100.0) / 10.0;
    let index = phase.floor() as usize;
    let t = phase.fract();
    let a = points[(index + points.len() - 1) % points.len()];
    let b = points[index];
    let c = points[(index + 1) % points.len()];
    let d = points[(index + 2) % points.len()];
    0.5 * ((2.0 * b)
        + (-a + c) * t
        + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t
        + (-a + 3.0 * b - 3.0 * c + d) * t * t * t)
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
            let p = reading_position(0.0);
            world.sabrina.motion.pos = sim::v(p.x, p.y);
            world
        });
        if !preferences.reduced_motion {
            self.accumulator += dt.min(0.1);
            while self.accumulator >= 1.0 / 60.0 {
                self.accumulator -= 1.0 / 60.0;
                let next = reading_position(world.session.time + 1.0 / 60.0);
                let now = reading_position(world.session.time);
                let movement = (next - now) * 60.0 / 4.0;
                self.previous = Some(world.clone());
                sim::step(
                    world,
                    1.0 / 60.0,
                    sim::Input {
                        movement: sim::v(movement.x, movement.y),
                        ..default()
                    },
                );
                world.sabrina.motion.pos = sim::v(next.x, next.y);
            }
        }
        if let Some(previous) = &self.previous {
            *rendered = sim::presentation::interpolate(previous, world, self.accumulator * 60.0);
        } else {
            rendered.clone_from(world);
        }
    }

    fn restore(&mut self, rendered: &mut sim::World) {
        if let Some(saved) = self.saved.take() {
            *rendered = saved;
        }
        self.world = None;
        self.previous = None;
        self.accumulator = 0.0;
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
    fn reading_route_is_smooth_closed_and_visits_foreground_and_ruins() {
        let mut near = false;
        let mut behind = false;
        let mut offscreen = false;
        for frame in 0..6000 {
            let time = frame as f32 / 60.0;
            let p = reading_position(time);
            let next = reading_position(time + 1.0 / 60.0);
            assert!(p.is_finite() && (next - p).length() < 0.1);
            assert!((2.0..17.0).contains(&p.y));
            near |= p.z > 4.0;
            behind |= p.z < -6.0;
            offscreen |= p.x.abs() > 18.0;
        }
        assert!(near && behind && offscreen);
        assert_eq!(reading_position(0.0), reading_position(100.0));
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
        }
        assert_eq!(
            rendered.jelly[0].behaviour.state,
            saved.jelly[0].behaviour.state
        );
        assert!(performance.world.is_none());
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
        for _ in 0..100 {
            performance.present(&mut rendered, 0.1, &preferences);
        }
        assert_eq!(rendered.session.time, 0.0);
        assert_eq!(rendered.sabrina.motion.pos, position);
        assert_eq!(rendered.fish[0].motion.pos, fish);
    }
}
