// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Short, non-stacking feedback from confirmed simulation events.
use crate::{
    app::{FrameEvents, Options, Screen, SessionUi},
    controls::Hints,
    settings::Preferences,
};
use bevy::{
    input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest},
    prelude::*,
    window::PrimaryWindow,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
struct Pulse {
    priority: u8,
    strong: f32,
    weak: f32,
    seconds: f32,
}

fn pulse(events: &[reef_sim::Events]) -> Option<Pulse> {
    events
        .iter()
        .filter_map(|event| {
            let (priority, strong, weak, seconds) = if event.hit {
                (4, 0.40, 0.20, 0.18)
            } else if event.jelly_hit {
                (3, 0.25, 0.30, 0.11)
            } else if event.launch {
                (2, 0.10, 0.20, 0.08)
            } else if event.wave_cast {
                (1, 0.20, 0.10, 0.14)
            } else {
                return None;
            };
            Some(Pulse {
                priority,
                strong,
                weak,
                seconds,
            })
        })
        .max_by_key(|pulse| pulse.priority)
}

#[derive(Resource, Default)]
struct Running {
    target: Option<Entity>,
    remaining: f32,
    priority: u8,
}

pub struct RumblePlugin;

impl Plugin for RumblePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Running>()
            .add_systems(Update, update.before(crate::audio::ConsumeEvents));
    }
}

fn update(
    events: Res<FrameEvents>,
    preferences: Res<Preferences>,
    ui: Res<SessionUi>,
    options: Res<Options>,
    hints: Res<Hints>,
    pads: Query<(), With<Gamepad>>,
    window: Single<&Window, With<PrimaryWindow>>,
    time: Res<Time<Real>>,
    mut running: ResMut<Running>,
    mut requests: MessageWriter<GamepadRumbleRequest>,
    mut exit: MessageReader<AppExit>,
) {
    let exiting = exit.read().next().is_some();
    let enabled = preferences.rumble > 0
        && ui.screen == Screen::Playing
        && window.focused
        && !exiting
        && options.capture.is_none()
        && !options.verify;
    let target = hints
        .active
        .filter(|entity| enabled && pads.contains(*entity));
    if running.target != target {
        if let Some(gamepad) = running.target.take()
            && pads.contains(gamepad)
        {
            requests.write(GamepadRumbleRequest::Stop { gamepad });
        }
        running.remaining = 0.0;
    }
    let Some(gamepad) = target else { return };
    running.remaining = (running.remaining - time.delta_secs()).max(0.0);
    let Some(pulse) = pulse(&events.0) else {
        return;
    };
    if running.remaining > 0.0 && pulse.priority <= running.priority {
        return;
    }
    // Add requests sum their strengths; replace rather than stack effects.
    if running.target.is_some() {
        requests.write(GamepadRumbleRequest::Stop { gamepad });
    }
    let scale = if preferences.rumble == 1 { 0.5 } else { 1.0 };
    requests.write(GamepadRumbleRequest::Add {
        gamepad,
        duration: Duration::from_secs_f32(pulse.seconds),
        intensity: GamepadRumbleIntensity {
            strong_motor: pulse.strong * scale,
            weak_motor: pulse.weak * scale,
        },
    });
    running.target = Some(gamepad);
    running.priority = pulse.priority;
    running.remaining = pulse.seconds;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin))
            .add_message::<AppExit>()
            .init_resource::<FrameEvents>()
            .init_resource::<Preferences>()
            .init_resource::<Options>()
            .init_resource::<Hints>()
            .insert_resource(SessionUi {
                screen: Screen::Playing,
                ..default()
            })
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f32(1.0 / 60.0),
            ))
            .add_plugins(RumblePlugin);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.world_mut().resource_mut::<Hints>().active = Some(pad);
        app.update();
        (app, pad, window)
    }

    fn requests(app: &mut App) -> Vec<GamepadRumbleRequest> {
        app.world_mut()
            .resource_mut::<Messages<GamepadRumbleRequest>>()
            .drain()
            .collect()
    }

    #[test]
    fn gentle_rumble_scales_and_higher_priority_replaces_without_stacking() {
        let (mut app, pad, _) = app();
        app.world_mut().resource_mut::<Preferences>().rumble = 1;
        app.world_mut().resource_mut::<FrameEvents>().0 = vec![
            reef_sim::Events {
                launch: true,
                ..default()
            };
            6
        ];
        app.update();
        let emitted = requests(&mut app);
        assert_eq!(emitted.len(), 1);
        match emitted[0] {
            GamepadRumbleRequest::Add {
                gamepad,
                intensity,
                duration,
            } => {
                assert_eq!(gamepad, pad);
                assert_eq!(intensity.strong_motor, 0.05);
                assert_eq!(intensity.weak_motor, 0.1);
                assert_eq!(duration, Duration::from_secs_f32(0.08));
            }
            _ => panic!("Expected a rumble pulse"),
        }
        app.update();
        assert!(requests(&mut app).is_empty());
        app.world_mut().resource_mut::<FrameEvents>().0 = vec![reef_sim::Events {
            hit: true,
            ..default()
        }];
        app.update();
        let emitted = requests(&mut app);
        assert_eq!(emitted.len(), 2);
        assert!(matches!(emitted[0], GamepadRumbleRequest::Stop { gamepad } if gamepad == pad));
        assert!(matches!(emitted[1], GamepadRumbleRequest::Add { gamepad, .. } if gamepad == pad));
    }

    #[test]
    fn rumble_stops_on_pause_focus_loss_disable_controller_switch_or_exit() {
        for reason in 0..5 {
            let (mut app, pad, window) = app();
            app.world_mut().resource_mut::<FrameEvents>().0 = vec![reef_sim::Events {
                hit: true,
                ..default()
            }];
            app.update();
            assert_eq!(requests(&mut app).len(), 1);
            app.world_mut().resource_mut::<FrameEvents>().0.clear();
            match reason {
                0 => app.world_mut().resource_mut::<SessionUi>().screen = Screen::Pause,
                1 => app.world_mut().get_mut::<Window>(window).unwrap().focused = false,
                2 => app.world_mut().resource_mut::<Preferences>().rumble = 0,
                3 => {
                    let other = app.world_mut().spawn(Gamepad::default()).id();
                    app.world_mut().resource_mut::<Hints>().active = Some(other);
                }
                _ => {
                    app.world_mut().write_message(AppExit::Success);
                }
            }
            app.update();
            let emitted = requests(&mut app);
            assert_eq!(emitted.len(), 1);
            assert!(matches!(emitted[0], GamepadRumbleRequest::Stop { gamepad } if gamepad == pad));
        }
    }

    #[test]
    fn no_controller_or_disconnected_controller_receives_no_requests() {
        let (mut app, pad, _) = app();
        app.world_mut().despawn(pad);
        app.world_mut().resource_mut::<FrameEvents>().0 = vec![reef_sim::Events {
            hit: true,
            ..default()
        }];
        app.update();
        assert!(requests(&mut app).is_empty());
    }

    #[test]
    fn coalesces_events_and_prioritizes_damage_with_bounded_strength() {
        let events = [
            reef_sim::Events {
                wave_cast: true,
                ..default()
            },
            reef_sim::Events {
                launch: true,
                ..default()
            },
            reef_sim::Events {
                hit: true,
                jelly_hit: true,
                ..default()
            },
        ];
        let result = pulse(&events).unwrap();
        assert_eq!(result.priority, 4);
        assert_eq!(result.strong, 0.4);
        assert_eq!(result.weak, 0.2);
        assert_eq!(result.seconds, 0.18);
        for event in [
            reef_sim::Events {
                wave_cast: true,
                ..default()
            },
            reef_sim::Events {
                launch: true,
                ..default()
            },
            reef_sim::Events {
                jelly_hit: true,
                ..default()
            },
            reef_sim::Events {
                hit: true,
                ..default()
            },
        ] {
            let result = pulse(&[event]).unwrap();
            assert!(result.strong <= 0.4 && result.weak <= 0.3);
            assert!(result.seconds <= 0.18);
        }
        assert!(
            pulse(&[reef_sim::Events {
                cast: true,
                ..default()
            }])
            .is_none()
        );
        assert!(pulse(&[]).is_none());
    }
}
