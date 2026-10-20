// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Native Windows Xbox/Steam Input transport into Bevy's usual input messages.
use bevy::{
    input::gamepad::{
        GamepadConnection, GamepadConnectionEvent, GamepadRumbleRequest,
        RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
    },
    prelude::*,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const SLOTS: usize = 4;
const BUTTONS: [(u16, GamepadButton); 14] = [
    (0x0001, GamepadButton::DPadUp),
    (0x0002, GamepadButton::DPadDown),
    (0x0004, GamepadButton::DPadLeft),
    (0x0008, GamepadButton::DPadRight),
    (0x0010, GamepadButton::Start),
    (0x0020, GamepadButton::Select),
    (0x0040, GamepadButton::LeftThumb),
    (0x0080, GamepadButton::RightThumb),
    (0x0100, GamepadButton::LeftTrigger),
    (0x0200, GamepadButton::RightTrigger),
    (0x1000, GamepadButton::South),
    (0x2000, GamepadButton::East),
    (0x4000, GamepadButton::West),
    (0x8000, GamepadButton::North),
];
const AXES: [GamepadAxis; 4] = [
    GamepadAxis::LeftStickX,
    GamepadAxis::LeftStickY,
    GamepadAxis::RightStickX,
    GamepadAxis::RightStickY,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Reading {
    buttons: u16,
    sticks: [i16; 4],
    triggers: [u8; 2],
}

fn stick(value: i16) -> f32 {
    value as f32 / if value < 0 { 32768.0 } else { 32767.0 }
}

struct Sample {
    slot: usize,
    reading: Option<Reading>,
}

enum MotorRequest {
    Add(Duration, [f32; 2]),
    Stop,
}

#[derive(Default)]
struct Motors {
    effects: Vec<(Duration, [f32; 2])>,
}

impl Motors {
    fn apply(&mut self, request: MotorRequest, now: Duration) {
        match request {
            MotorRequest::Add(duration, intensity) => {
                self.effects.push((now + duration, intensity))
            }
            MotorRequest::Stop => self.effects.clear(),
        }
    }

    fn speeds(&mut self, now: Duration) -> [u16; 2] {
        self.effects.retain(|(deadline, _)| *deadline > now);
        let mut sum = [0.0_f32; 2];
        for (_, intensity) in &self.effects {
            for motor in 0..2 {
                sum[motor] += intensity[motor].clamp(0.0, 1.0);
            }
        }
        sum.map(|value| (value.min(1.0) * u16::MAX as f32) as u16)
    }
}

#[derive(Resource)]
struct Backend {
    samples: Mutex<Receiver<Sample>>,
    requests: Sender<(usize, MotorRequest)>,
    entities: [Option<Entity>; SLOTS],
    readings: [Option<Reading>; SLOTS],
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    suppress_edges: bool,
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            error!("XInput polling thread panicked");
        }
    }
}

#[cfg(target_os = "windows")]
pub struct XInputPlugin;

#[cfg(target_os = "windows")]
impl Plugin for XInputPlugin {
    fn build(&self, app: &mut App) {
        let (sample_tx, samples) = mpsc::channel();
        let (requests, request_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("sabrina-xinput".into())
            .spawn(move || {
                poll(
                    sample_tx,
                    request_rx,
                    worker_stop,
                    native::read,
                    native::vibrate,
                )
            })
            .expect("Failed to start XInput polling thread");
        app.insert_resource(Backend {
            samples: Mutex::new(samples),
            requests,
            entities: [None; SLOTS],
            readings: [None; SLOTS],
            stop,
            thread: Some(thread),
            suppress_edges: false,
        })
        .add_systems(PreUpdate, receive.before(bevy::input::InputSystems))
        .add_systems(
            PreUpdate,
            clear_focus_edges
                .after(bevy::input::InputSystems)
                .before(crate::controls::update)
                .before(crate::app::input),
        )
        .add_systems(PostUpdate, rumble);
    }
}

fn receive(
    mut commands: Commands,
    mut backend: ResMut<Backend>,
    mut connections: MessageWriter<GamepadConnectionEvent>,
    mut events: MessageWriter<RawGamepadEvent>,
    mut buttons: MessageWriter<RawGamepadButtonChangedEvent>,
    mut axes: MessageWriter<RawGamepadAxisChangedEvent>,
    mut focus: MessageReader<bevy::window::WindowFocused>,
) {
    // XInput also polls in the background; don't replay desktop clicks on refocus.
    backend.suppress_edges = focus.read().next().is_some();
    focus.clear();
    let samples: Vec<_> = backend
        .samples
        .lock()
        .expect("XInput sample queue poisoned")
        .try_iter()
        .collect();
    for Sample { slot, reading } in samples {
        let previous = backend.readings[slot];
        backend.readings[slot] = reading;
        if previous.is_none() && reading.is_none() {
            continue;
        }
        let entity = *backend.entities[slot].get_or_insert_with(|| commands.spawn_empty().id());
        if previous.is_none() || reading.is_none() {
            let connection = GamepadConnectionEvent::new(
                entity,
                if reading.is_some() {
                    GamepadConnection::Connected {
                        name: format!("Xbox Controller (XInput slot {})", slot + 1),
                        vendor_id: Some(0x045e),
                        product_id: None,
                    }
                } else {
                    GamepadConnection::Disconnected
                },
            );
            connections.write(connection.clone());
            events.write(connection.into());
        }
        let Some(reading) = reading else { continue };
        let old = previous.unwrap_or_default();
        for (mask, button) in BUTTONS {
            if previous.is_none() || (old.buttons & mask) != (reading.buttons & mask) {
                let event = RawGamepadButtonChangedEvent::new(
                    entity,
                    button,
                    if reading.buttons & mask != 0 {
                        1.0
                    } else {
                        0.0
                    },
                );
                events.write(event.into());
                buttons.write(event);
            }
        }
        for (index, axis) in AXES.into_iter().enumerate() {
            if previous.is_none() || old.sticks[index] != reading.sticks[index] {
                let event =
                    RawGamepadAxisChangedEvent::new(entity, axis, stick(reading.sticks[index]));
                events.write(event.into());
                axes.write(event);
            }
        }
        for (index, button) in [GamepadButton::LeftTrigger2, GamepadButton::RightTrigger2]
            .into_iter()
            .enumerate()
        {
            if previous.is_none() || old.triggers[index] != reading.triggers[index] {
                let event = RawGamepadButtonChangedEvent::new(
                    entity,
                    button,
                    reading.triggers[index] as f32 / 255.0,
                );
                events.write(event.into());
                buttons.write(event);
            }
        }
    }
}

fn clear_focus_edges(backend: Res<Backend>, mut pads: Query<&mut Gamepad>) {
    if backend.suppress_edges {
        for entity in backend.entities.iter().flatten() {
            if let Ok(mut pad) = pads.get_mut(*entity) {
                pad.digital_mut().clear();
            }
        }
    }
}

fn rumble(
    backend: Res<Backend>,
    mut requests: MessageReader<GamepadRumbleRequest>,
    mut exit: MessageReader<AppExit>,
) {
    if exit.read().next().is_some() {
        backend.stop.store(true, Ordering::Relaxed);
        return;
    }
    for request in requests.read() {
        let Some(slot) = backend
            .entities
            .iter()
            .position(|entity| *entity == Some(request.gamepad()))
        else {
            warn!(
                "XInput rumble requested for unknown controller {:?}",
                request.gamepad()
            );
            continue;
        };
        let command = match *request {
            GamepadRumbleRequest::Add {
                duration,
                intensity,
                ..
            } => MotorRequest::Add(duration, [intensity.strong_motor, intensity.weak_motor]),
            GamepadRumbleRequest::Stop { .. } => MotorRequest::Stop,
        };
        backend
            .requests
            .send((slot, command))
            .expect("XInput polling thread stopped unexpectedly");
    }
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use windows_sys::Win32::UI::Input::XboxController::{
        XINPUT_STATE, XINPUT_VIBRATION, XInputGetState, XInputSetState,
    };

    pub(super) fn read(slot: usize) -> Result<Reading, u32> {
        let mut state = XINPUT_STATE::default();
        // SAFETY: slot is in 0..4 and state is a writable, correctly sized XINPUT_STATE.
        let result = unsafe { XInputGetState(slot as u32, &mut state) };
        if result != 0 {
            return Err(result);
        }
        let pad = state.Gamepad;
        Ok(Reading {
            buttons: pad.wButtons,
            sticks: [pad.sThumbLX, pad.sThumbLY, pad.sThumbRX, pad.sThumbRY],
            triggers: [pad.bLeftTrigger, pad.bRightTrigger],
        })
    }

    pub(super) fn vibrate(slot: usize, speeds: [u16; 2]) {
        let vibration = XINPUT_VIBRATION {
            wLeftMotorSpeed: speeds[0],
            wRightMotorSpeed: speeds[1],
        };
        // SAFETY: slot is in 0..4 and vibration lives for the duration of this call.
        let result = unsafe { XInputSetState(slot as u32, &vibration) };
        if result != 0 && result != 1167 {
            warn!("XInputSetState failed for slot {slot}: Windows error {result}");
        }
    }
}

fn poll(
    samples: Sender<Sample>,
    requests: Receiver<(usize, MotorRequest)>,
    stop: Arc<AtomicBool>,
    mut read: impl FnMut(usize) -> Result<Reading, u32>,
    mut vibrate: impl FnMut(usize, [u16; 2]),
) {
    let start = Instant::now();
    let mut previous = [None; SLOTS];
    let mut errors = [None; SLOTS];
    let mut motors: [Motors; SLOTS] = std::array::from_fn(|_| Motors::default());
    let mut sent = [[0; 2]; SLOTS];
    let mut next_probe = [Duration::ZERO; SLOTS];
    while !stop.load(Ordering::Relaxed) {
        let now = start.elapsed();
        for (slot, request) in requests.try_iter() {
            motors[slot].apply(request, now);
        }
        for slot in 0..SLOTS {
            // Probing disconnected XInput slots is expensive; don't do it at input rate.
            if previous[slot].is_none() && now < next_probe[slot] {
                motors[slot].apply(MotorRequest::Stop, now);
                continue;
            }
            let reading = match read(slot) {
                Ok(reading) => {
                    errors[slot] = None;
                    Some(reading)
                }
                Err(code) => {
                    if code != 1167 && errors[slot] != Some(code) {
                        warn!("XInputGetState failed for slot {slot}: Windows error {code}");
                    }
                    errors[slot] = Some(code);
                    None
                }
            };
            if reading.is_none() {
                next_probe[slot] = now + Duration::from_secs(1);
            }
            if reading != previous[slot] {
                if reading.is_none() {
                    motors[slot].apply(MotorRequest::Stop, now);
                    sent[slot] = [0; 2];
                }
                if samples.send(Sample { slot, reading }).is_err() {
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
                previous[slot] = reading;
            }
            let speeds = motors[slot].speeds(now);
            if reading.is_some() && speeds != sent[slot] {
                vibrate(slot, speeds);
                sent[slot] = speeds;
            }
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    for slot in 0..SLOTS {
        vibrate(slot, [0; 2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> (App, Sender<Sample>, Receiver<(usize, MotorRequest)>) {
        let (tx, samples) = mpsc::channel();
        let (requests, rx) = mpsc::channel();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin))
            .add_message::<AppExit>()
            .add_message::<bevy::window::WindowFocused>()
            .insert_resource(Backend {
                samples: Mutex::new(samples),
                requests,
                entities: [None; SLOTS],
                readings: [None; SLOTS],
                stop: Arc::new(AtomicBool::new(false)),
                thread: None,
                suppress_edges: false,
            })
            .init_resource::<crate::controls::Hints>()
            .add_systems(PreUpdate, receive.before(bevy::input::InputSystems))
            .add_systems(
                PreUpdate,
                crate::controls::update.after(bevy::input::InputSystems),
            )
            .add_systems(
                PreUpdate,
                clear_focus_edges
                    .after(bevy::input::InputSystems)
                    .before(crate::controls::update),
            )
            .add_systems(PostUpdate, rumble);
        (app, tx, rx)
    }

    fn sample(tx: &Sender<Sample>, slot: usize, reading: Option<Reading>) {
        tx.send(Sample { slot, reading }).unwrap();
    }

    #[test]
    fn native_button_mapping_and_quick_taps_survive_render_frame_batching() {
        let (mut app, tx, _) = app();
        sample(&tx, 0, Some(Reading::default()));
        app.update();
        let entity = app.world().resource::<Backend>().entities[0].unwrap();
        for (mask, button) in BUTTONS {
            sample(
                &tx,
                0,
                Some(Reading {
                    buttons: mask,
                    ..default()
                }),
            );
            sample(&tx, 0, Some(Reading::default()));
            app.update();
            let pad = app.world().get::<Gamepad>(entity).unwrap();
            assert!(pad.just_pressed(button), "{button:?}");
            assert!(pad.just_released(button), "{button:?}");
            assert!(!pad.pressed(button));
            app.update();
            assert!(
                !app.world()
                    .get::<Gamepad>(entity)
                    .unwrap()
                    .just_pressed(button)
            );
        }
    }

    #[test]
    fn refocus_does_not_replay_background_presses_but_preserves_held_stick() {
        let (mut app, tx, _) = app();
        sample(&tx, 0, Some(Reading::default()));
        app.update();
        sample(
            &tx,
            0,
            Some(Reading {
                buttons: 0x1000,
                sticks: [i16::MAX, 0, 0, 0],
                ..default()
            }),
        );
        app.world_mut().write_message(bevy::window::WindowFocused {
            window: Entity::PLACEHOLDER,
            focused: true,
        });
        app.update();
        let entity = app.world().resource::<Backend>().entities[0].unwrap();
        let pad = app.world().get::<Gamepad>(entity).unwrap();
        assert!(!pad.just_pressed(GamepadButton::South));
        assert_eq!(pad.left_stick(), Vec2::X);
        sample(&tx, 0, Some(Reading::default()));
        app.update();
        sample(
            &tx,
            0,
            Some(Reading {
                buttons: 0x1000,
                ..default()
            }),
        );
        app.update();
        assert!(
            app.world()
                .get::<Gamepad>(entity)
                .unwrap()
                .just_pressed(GamepadButton::South)
        );
    }

    #[test]
    fn native_dpad_navigates_all_directions_and_holds_repeat_without_new_edges() {
        let (mut app, tx, _) = app();
        for (mask, direction) in [
            (1, crate::controls::Direction::Up),
            (2, crate::controls::Direction::Down),
            (4, crate::controls::Direction::Left),
            (8, crate::controls::Direction::Right),
        ] {
            sample(
                &tx,
                0,
                Some(Reading {
                    buttons: mask,
                    ..default()
                }),
            );
            app.update();
            let entity = app.world().resource::<Backend>().entities[0].unwrap();
            let mut nav = crate::controls::MenuNavigation::default();
            assert_eq!(
                nav.update(app.world().get::<Gamepad>(entity), 0.01),
                Some(direction)
            );
            app.update();
            assert_eq!(
                nav.update(app.world().get::<Gamepad>(entity), 0.41),
                Some(direction)
            );
            sample(&tx, 0, Some(Reading::default()));
            app.update();
        }
    }

    #[test]
    fn axes_triggers_and_hotplug_use_normal_bevy_components_and_active_hints() {
        let (mut app, tx, _) = app();
        assert_eq!(stick(i16::MIN), -1.0);
        assert_eq!(stick(i16::MAX), 1.0);
        assert_eq!(stick(0), 0.0);
        sample(
            &tx,
            2,
            Some(Reading {
                sticks: [i16::MAX, i16::MIN, 0, 0],
                triggers: [0, 255],
                ..default()
            }),
        );
        app.update();
        let entity = app.world().resource::<Backend>().entities[2].unwrap();
        let pad = app.world().get::<Gamepad>(entity).unwrap();
        assert_eq!(pad.left_stick(), Vec2::new(1.0, -1.0));
        assert!(pad.pressed(GamepadButton::RightTrigger2));
        assert!(!pad.pressed(GamepadButton::LeftTrigger2));
        assert_eq!(
            app.world().resource::<crate::controls::Hints>().active,
            Some(entity)
        );
        sample(&tx, 2, None);
        app.update();
        assert!(app.world().get::<Gamepad>(entity).is_none());
        assert_eq!(
            app.world().resource::<crate::controls::Hints>().active,
            None
        );
        sample(&tx, 2, Some(Reading::default()));
        app.update();
        assert_eq!(app.world().resource::<Backend>().entities[2], Some(entity));
        assert!(
            !app.world()
                .get::<Gamepad>(entity)
                .unwrap()
                .pressed(GamepadButton::RightTrigger2)
        );
    }

    #[test]
    fn rumble_routes_to_the_same_xinput_slot_and_exit_stops_the_worker() {
        let (mut app, tx, rx) = app();
        sample(&tx, 3, Some(Reading::default()));
        app.update();
        let gamepad = app.world().resource::<Backend>().entities[3].unwrap();
        app.world_mut().write_message(GamepadRumbleRequest::Add {
            gamepad,
            duration: Duration::from_millis(80),
            intensity: bevy::input::gamepad::GamepadRumbleIntensity {
                strong_motor: 0.4,
                weak_motor: 0.2,
            },
        });
        app.world_mut()
            .write_message(GamepadRumbleRequest::Stop { gamepad });
        app.update();
        assert!(
            matches!(rx.try_recv().unwrap(), (3, MotorRequest::Add(duration, [0.4, 0.2])) if duration == Duration::from_millis(80))
        );
        assert!(matches!(rx.try_recv().unwrap(), (3, MotorRequest::Stop)));
        app.world_mut().write_message(AppExit::Success);
        app.update();
        assert!(
            app.world()
                .resource::<Backend>()
                .stop
                .load(Ordering::Relaxed)
        );
    }

    #[test]
    fn motor_deadlines_stop_without_render_updates_and_replace_in_order() {
        let mut motors = Motors::default();
        let zero = Duration::ZERO;
        motors.apply(
            MotorRequest::Add(Duration::from_millis(80), [0.4, 0.2]),
            zero,
        );
        assert_eq!(motors.speeds(zero), [26214, 13107]);
        assert_eq!(motors.speeds(Duration::from_millis(80)), [0, 0]);
        motors.apply(MotorRequest::Add(Duration::from_secs(1), [0.8, 0.6]), zero);
        motors.apply(MotorRequest::Add(Duration::from_secs(1), [0.8, 0.6]), zero);
        assert_eq!(motors.speeds(zero), [65535, 65535]);
        motors.apply(MotorRequest::Stop, zero);
        motors.apply(
            MotorRequest::Add(Duration::from_millis(100), [0.2, 0.0]),
            zero,
        );
        assert_eq!(motors.speeds(zero), [13107, 0]);
        motors.apply(MotorRequest::Stop, zero);
        assert_eq!(motors.speeds(zero), [0, 0]);
    }

    #[test]
    fn polling_thread_captures_between_frames_and_expires_motors_without_a_frame() {
        let (sample_tx, samples) = mpsc::channel();
        let (requests, request_rx) = mpsc::channel();
        let (motor_tx, motor_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut step = 0;
            poll(
                sample_tx,
                request_rx,
                worker_stop,
                |slot| {
                    if slot != 1 {
                        return Err(1167);
                    }
                    let buttons = if step == 1 { 0x1008 } else { 0 };
                    step += 1;
                    Ok(Reading {
                        buttons,
                        ..default()
                    })
                },
                |slot, speeds| motor_tx.send((slot, speeds)).unwrap(),
            );
        });
        // No App::update calls: both edges must already be queued by the worker.
        for expected in [0, 0x1008, 0] {
            let sample = samples.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_eq!(sample.slot, 1);
            assert_eq!(sample.reading.unwrap().buttons, expected);
        }
        requests
            .send((1, MotorRequest::Add(Duration::from_millis(40), [0.4, 0.2])))
            .unwrap();
        assert_eq!(
            motor_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            (1, [26214, 13107])
        );
        assert_eq!(
            motor_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            (1, [0, 0])
        );
        requests
            .send((1, MotorRequest::Add(Duration::from_secs(10), [1.0, 1.0])))
            .unwrap();
        assert_eq!(
            motor_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            (1, [65535, 65535])
        );
        stop.store(true, Ordering::Relaxed);
        thread.join().unwrap();
        assert_eq!(
            motor_rx.try_iter().collect::<Vec<_>>(),
            (0..SLOTS).map(|slot| (slot, [0, 0])).collect::<Vec<_>>()
        );
    }
}
