// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Drives actual Bevy input resources through the same live systems used by play.
use crate::{
    app::{FrameEvents, Options, Screen, SessionUi, Simulation},
    assets::{Actor, Palm, RigPlayer},
    audio::Channel,
};
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*, window::PrimaryWindow};

#[derive(Resource, Default)]
struct Check {
    frame: u32,
    initial: reef_sim::V2,
    paused_time: f32,
    paused_pose: Vec<(Entity, Transform)>,
    paused_face: Vec<f32>,
    idle_hair: Vec<(Entity, Quat)>,
    paused_hair_shader: Vec4,
    idle_hair_time: f32,
    music_player: Option<Entity>,
    vsync: bool,
    reduced_motion: bool,
    rumble: u8,
    ability_stage: u8,
    ability_time: f32,
}

#[derive(Component)]
struct TestPad;

pub struct VerificationPlugin;

impl Plugin for VerificationPlugin {
    fn build(&self, app: &mut App) {
        if app.world().resource::<Options>().verify {
            app.insert_resource(bevy::winit::WinitSettings::continuous());
        }
        app.init_resource::<Check>()
            .add_systems(
                Update,
                verify_splash
                    .after(crate::loading::bootstrap)
                    .before(crate::loading::LoadResources),
            )
            .add_systems(
                Update,
                prepare_update_notice
                    .in_set(crate::loading::LoadResources)
                    .after(crate::updates::start),
            )
            .add_systems(
                PreUpdate,
                verify_update_notice
                    .after(bevy::input::InputSystems)
                    .before(crate::app::input),
            )
            .add_systems(
                PreUpdate,
                drive
                    .after(bevy::input::InputSystems)
                    .before(crate::controls::update)
                    .before(crate::app::input),
            )
            .add_systems(
                PreUpdate,
                verify_reading
                    .after(drive)
                    .before(crate::controls::update)
                    .before(crate::app::input),
            )
            .add_systems(
                PostUpdate,
                check_pose.after(bevy::transform::TransformSystems::Propagate),
            );
    }
}

fn verify_splash(
    mut commands: Commands,
    options: Res<Options>,
    loading: Res<crate::loading::Loading>,
    library: Res<crate::assets::Library>,
    scene_cameras: Query<&Camera3d>,
    ui_cameras: Query<&Camera2d>,
    audio: Query<&Channel>,
    texts: Query<&Text>,
    mut checked: Local<bool>,
) {
    if !options.verify || !loading.splash_presented || *checked {
        return;
    }
    assert!(library.handles.is_empty());
    assert_eq!(loading.assets_total, 0);
    assert!(scene_cameras.is_empty());
    assert!(audio.is_empty());
    assert_eq!(ui_cameras.iter().count(), 1);
    assert!(texts.iter().any(|text| text.0.contains("0%")));
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(
            std::env::temp_dir().join("sabrina-bevy-startup.png"),
        ));
    println!("BEVY CHECK OK: splash rendered at 0% before models, audio or the 3D scene start");
    *checked = true;
}

fn prepare_update_notice(options: Res<Options>, mut check: ResMut<crate::updates::UpdateCheck>) {
    if options.verify {
        check.latest_version = Some("v9999.0.0".into());
    }
}

fn verify_update_notice(
    mut commands: Commands,
    options: Res<Options>,
    check: Res<crate::updates::UpdateCheck>,
    loading: Res<crate::loading::Loading>,
    ui: Res<SessionUi>,
    library: Res<crate::assets::Library>,
    texts: Query<&Text>,
    mut captured: Local<bool>,
    mut finished: Local<bool>,
) {
    if !options.verify || *finished || !loading.splash_presented {
        return;
    }
    assert_eq!(check.latest_version.as_deref(), Some("v9999.0.0"));
    assert!(loading.assets_total > 0);
    assert!(!library.handles.is_empty());
    if !*captured && texts.iter().any(|text| text.0.contains("v9999.0.0")) {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                std::env::temp_dir().join("sabrina-bevy-update-notice.png"),
            ));
        *captured = true;
    }
    if ui.loaded {
        assert!(*captured);
        assert_eq!(loading.progress, 1.0);
        println!("BEVY CHECK OK: update notice never blocks loading or requires an action");
        *finished = true;
    }
}

fn pixel(window: &Window, point: Vec2) -> Vec2 {
    let scale =
        (window.width() / crate::ui::CANVAS_WIDTH).min(window.height() / crate::ui::CANVAS_HEIGHT);
    point * scale
        + Vec2::new(
            (window.width() - crate::ui::CANVAS_WIDTH * scale) / 2.0,
            (window.height() - crate::ui::CANVAS_HEIGHT * scale) / 2.0,
        )
}

fn drive(
    mut commands: Commands,
    options: Res<Options>,
    mut check: ResMut<Check>,
    mut simulation: ResMut<Simulation>,
    mut ui: ResMut<SessionUi>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut motion: ResMut<AccumulatedMouseMotion>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut gamepads: Query<&mut Gamepad, With<TestPad>>,
    rigs: Query<&RigPlayer>,
    audio: Query<(Entity, &Channel, Option<&AudioSink>)>,
    mut events: ResMut<FrameEvents>,
    texts: Query<&Text>,
    controller_checks: (
        Query<(&TextSpan, &TextColor)>,
        Query<Entity, With<TestPad>>,
        Res<crate::controls::Hints>,
        ResMut<crate::settings::Preferences>,
        Query<(&Text, &Visibility), With<crate::ui::FpsCounter>>,
    ),
) {
    let (spans, test_pads, hints, preferences, fps_counter) = controller_checks;
    if !options.verify
        || !ui.loaded
        || rigs.iter().count() < 1 + reef_sim::FISH_COUNT + reef_sim::JELLY_COUNT
    {
        return;
    }
    check.frame += 1;
    let frame = check.frame;
    keys.reset_all();
    mouse.reset_all();
    motion.delta = Vec2::ZERO;
    if let Ok(mut gamepad) = gamepads.single_mut() {
        gamepad.digital_mut().reset_all();
        gamepad.analog_mut().set(GamepadAxis::LeftStickX, 0.0);
        gamepad.analog_mut().set(GamepadAxis::LeftStickY, 0.0);
    }
    if check.ability_stage == 3 {
        return;
    }
    match frame {
        1 => {
            assert_eq!(ui.screen, Screen::Playing);
            check.initial = simulation.0.sabrina.motion.pos;
            commands.spawn((Gamepad::default(), Name::new("Xbox Controller"), TestPad));
        }
        2..=40 => {
            keys.press(KeyCode::KeyW);
        }
        41 => {
            assert!(simulation.0.sabrina.motion.pos.y > check.initial.y + 0.5);
            println!("BEVY CHECK OK: keyboard swimming");
            keys.press(KeyCode::KeyQ);
        }
        45 => {
            assert!(simulation.0.sabrina.song.active);
            assert!(simulation.0.sabrina.song.energy < 100.0);
            println!("BEVY CHECK OK: keyboard song");
            keys.press(KeyCode::KeyQ);
        }
        50 => {
            assert!(!simulation.0.sabrina.song.active);
            assert!(audio.iter().any(|(_, channel, sink)| {
                *channel == Channel::Calm
                    && sink.is_some_and(|s| !s.empty() && s.volume().to_linear() > 0.0)
            }));
            println!("BEVY CHECK OK: decoded MIDI-rendered exploration music");
            keys.press(KeyCode::KeyF);
        }
        51 => {
            let pointer_position = pixel(&window, Vec2::new(1350.0, 300.0));
            window.set_cursor_position(Some(pointer_position));
            mouse.press(MouseButton::Left);
            keys.press(KeyCode::KeyE);
        }
        54 => {
            assert_eq!(simulation.0.crystal.magic.state, reef_sim::Spell::Resting);
            println!("BEVY CHECK OK: left-click Wave and E do not cast crystal");
        }
        55 => {
            assert!(simulation.0.sabrina.wave.active);
            println!("BEVY CHECK OK: keyboard wave");
            simulation.0.sabrina.wave.active = false;
            mouse.press(MouseButton::Right);
        }
        60 => {
            assert_eq!(simulation.0.crystal.magic.state, reef_sim::Spell::Charging);
            println!("BEVY CHECK OK: right-click crystal hand wind-up");
        }
        150 => {
            assert!(simulation.0.sabrina.cooldown > 9.0 && simulation.0.sabrina.cooldown <= 12.0);
            println!("BEVY CHECK OK: crystal launch and recharge");
            check.initial = simulation.0.sabrina.motion.pos;
        }
        151..=190 => {
            if let Ok(mut gamepad) = gamepads.single_mut() {
                gamepad.analog_mut().set(GamepadAxis::LeftStickX, 1.0);
            }
        }
        191 => {
            assert!(simulation.0.sabrina.motion.pos.x > check.initial.x + 0.5);
            println!("BEVY CHECK OK: controller swimming");
            if let Ok(mut gamepad) = gamepads.single_mut() {
                gamepad.digital_mut().press(GamepadButton::East);
            }
        }
        196 => {
            assert!(simulation.0.sabrina.song.active);
            println!("BEVY CHECK OK: controller song");
            simulation.0.sabrina.wave.cooldown = 0.0;
            if let Ok(mut gamepad) = gamepads.single_mut() {
                gamepad.digital_mut().press(GamepadButton::South);
            }
        }
        201 => {
            assert!(simulation.0.sabrina.wave.active);
            println!("BEVY CHECK OK: controller wave");
            keys.press(KeyCode::Escape);
        }
        205 => {
            assert_eq!(ui.screen, Screen::Pause);
            check.paused_time = simulation.0.session.time;
        }
        235 => {
            assert_eq!(simulation.0.session.time, check.paused_time);
            println!("BEVY CHECK OK: pause freezes gameplay");
            keys.press(KeyCode::Escape);
        }
        240 => {
            assert_eq!(ui.screen, Screen::Playing);
            check.initial = simulation.0.sabrina.motion.pos;
            let pointer_position = pixel(&window, Vec2::new(1350.0, 300.0));
            window.set_cursor_position(Some(pointer_position));
            motion.delta = Vec2::new(10.0, 0.0);
        }
        280 => {
            assert!(simulation.0.sabrina.motion.pos.x > check.initial.x);
            println!("BEVY CHECK OK: mouse swimming");
            simulation.0.sabrina.wave.cooldown = 0.0;
            simulation.0.sabrina.wave.active = false;
            simulation.0.sabrina.throw_time = -1.0;
            simulation.0.sabrina.cooldown = 0.0;
            simulation.0.crystal.magic.state = reef_sim::Spell::Resting;
            let pointer_position = pixel(&window, Vec2::new(656.0, 800.0));
            window.set_cursor_position(Some(pointer_position));
            mouse.press(MouseButton::Left);
        }
        285 => {
            assert!(simulation.0.sabrina.wave.active);
            assert_eq!(simulation.0.crystal.magic.state, reef_sim::Spell::Resting);
            println!("BEVY CHECK OK: HUD wave does not cast crystal");
            simulation.0.sabrina.vitality.health = 10.0;
            simulation.0.sabrina.vitality.invulnerable = 0.0;
            simulation.0.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
            simulation.0.jelly[0].motion.pos = simulation.0.sabrina.motion.pos;
        }
        290 => {
            assert_eq!(ui.screen, Screen::GameOver);
            keys.press(KeyCode::Escape);
        }
        295 => {
            assert_eq!(ui.screen, Screen::GameOver);
            println!("BEVY CHECK OK: game-over cannot be dismissed with pause");
            let pointer_position = pixel(&window, Vec2::new(800.0, 530.0));
            window.set_cursor_position(Some(pointer_position));
            mouse.press(MouseButton::Left);
        }
        300 => {
            assert_eq!(ui.screen, Screen::Playing);
            assert_eq!(simulation.0.sabrina.vitality.health, 100.0);
            println!("BEVY CHECK OK: retry resets the session");
            ui.pointer_mode = false;
        }
        310 => {
            simulation.0.settings.aquarium = false;
            simulation.0.settings.gentle = true;
            simulation.0.settings.visitor_limit = 1;
            reef_sim::begin_visit(&mut simulation.0);
            events.0.push(reef_sim::Events {
                visit_started: true,
                ..Default::default()
            });
            simulation.0.sabrina.motion.pos = reef_sim::v(-8.0, 14.0);
            simulation.0.jelly[0].motion.pos = reef_sim::v(0.0, 8.0);
            simulation.0.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
            for fish in &mut simulation.0.fish {
                fish.motion.pos = reef_sim::v(10.0, 4.0);
            }
            simulation.0.crystal.magic.state = reef_sim::Spell::Flying;
            simulation.0.crystal.motion.pos = reef_sim::v(-1.5, 8.0);
            simulation.0.crystal.motion.direction = reef_sim::v(1.0, 0.0);
            simulation.0.sabrina.cooldown = 12.0;
        }
        315 => {
            assert_eq!(simulation.0.jelly[0].behaviour.health, 60.0);
            assert!(simulation.0.jelly[0].behaviour.bubble > 0.0);
            assert_eq!(simulation.0.points.score, 40);
            assert!(
                simulation.0.points.rewards[reef_sim::ScoreFeedback::Crystal as usize].time > 0.0
            );
            println!("BEVY CHECK OK: crystal damage, bubble and immediate score feedback");
        }
        340 => {
            let arrival = audio
                .iter()
                .filter(|(_, channel, _)| **channel == Channel::Arrival)
                .count();
            assert_eq!(arrival, 1);
            assert!(audio.iter().any(|(_, channel, sink)| {
                *channel == Channel::Arrival && sink.is_some_and(|s| !s.empty())
            }));
            assert!(audio.iter().any(|(_, channel, sink)| {
                *channel == Channel::Visit && sink.is_some_and(|s| s.volume().to_linear() > 0.0)
            }));
            println!(
                "BEVY CHECK OK: decoded MIDI-rendered combat music and independent arrival stinger"
            );
            check.music_player = audio
                .iter()
                .find_map(|(entity, channel, _)| (*channel == Channel::Visit).then_some(entity));
        }
        345 => {
            events.0.push(reef_sim::Events {
                visit_started: true,
                ..Default::default()
            });
        }
        350 => {
            let visitor = audio
                .iter()
                .find_map(|(entity, channel, _)| (*channel == Channel::Visit).then_some(entity));
            assert!(visitor.is_some() && visitor != check.music_player);
            assert_eq!(
                audio
                    .iter()
                    .filter(|(_, channel, _)| **channel == Channel::Arrival)
                    .count(),
                1
            );
            println!(
                "BEVY CHECK OK: new arrival restarts combat at bar one without stacking stingers"
            );
        }
        380 => {
            assert_eq!(simulation.0.crystal.magic.state, reef_sim::Spell::Resting);
            assert_eq!(simulation.0.jelly[0].behaviour.health, 60.0);
            println!("BEVY CHECK OK: boomerang returns without repeat damage");
            for fish in &mut simulation.0.fish {
                fish.life.active = false;
            }
        }
        385 => {
            assert_eq!(ui.screen, Screen::GameOver);
            assert!(simulation.0.sabrina.vitality.health > 0.0);
            println!("BEVY CHECK OK: fish wipeout opens game-over with health remaining");
            assert!(
                audio
                    .iter()
                    .all(|(_, channel, _)| *channel != Channel::Arrival)
            );
        }
        410 => {
            reef_sim::reset(&mut simulation.0, 27, reef_sim::Settings::default());
            reef_sim::begin_visit(&mut simulation.0);
            ui.screen = Screen::Playing;
            simulation.0.sabrina.motion.pos = reef_sim::v(0.0, 8.0);
            simulation.0.jelly[0].motion.pos = reef_sim::v(1.5, 10.4);
            simulation.0.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
            simulation.0.jelly[0].behaviour.bubble = 3.0;
            check.initial = simulation.0.jelly[0].motion.pos;
            keys.press(KeyCode::KeyF);
        }
        450 => {
            assert_eq!(simulation.0.jelly[0].behaviour.health, 90.0);
            assert!(simulation.0.jelly[0].motion.pos.x > check.initial.x + 0.7);
            println!("BEVY CHECK OK: close wave hits tentacles and pushes jellyfish");
        }
        470 => {
            reef_sim::reset(&mut simulation.0, 27, reef_sim::Settings::default());
            simulation.0.session.cycle = reef_sim::FINAL_ROUND - 1;
            reef_sim::begin_visit(&mut simulation.0);
            assert_eq!(reef_sim::hungry(&simulation.0), 4);
            simulation.0.sabrina.vitality.invulnerable = 10.0;
            for (index, jellyfish) in simulation.0.jelly.iter_mut().enumerate() {
                jellyfish.motion.pos = reef_sim::v(-9.0 + index as f32 * 6.0, 13.0);
                jellyfish.behaviour.state = reef_sim::JellyState::Hungry;
                jellyfish.behaviour.bubble = 3.0;
            }
        }
        490 => {
            let mut result = reef_sim::Events::default();
            for index in 0..reef_sim::JELLY_COUNT {
                reef_sim::damage_jelly(&mut simulation.0, index, 100.0, true, &mut result);
            }
            events.0.push(result);
        }
        492 => {
            assert_eq!(ui.screen, Screen::Playing);
            assert!(!simulation.0.session.victory);
            for jellyfish in &mut simulation.0.jelly {
                assert_eq!(jellyfish.behaviour.state, reef_sim::JellyState::Leaving);
                jellyfish.behaviour.time = 9.0;
            }
        }
        495 => {
            assert_eq!(ui.screen, Screen::Victory);
            assert!(simulation.0.session.victory);
            check.paused_time = simulation.0.session.time;
            keys.press(KeyCode::Escape);
        }
        510 => {
            assert_eq!(ui.screen, Screen::Victory);
            assert_eq!(simulation.0.session.time, check.paused_time);
            println!(
                "BEVY CHECK OK: round ten victory freezes play and cannot be dismissed with pause"
            );
            let position = pixel(&window, Vec2::new(800.0, 600.0));
            window.set_cursor_position(Some(position));
            mouse.press(MouseButton::Left);
        }
        515 => {
            assert_eq!(ui.screen, Screen::Playing);
            assert!(!simulation.0.session.finished());
            assert_eq!(simulation.0.session.cycle, 0);
            crate::app::home(
                &mut simulation.0,
                &mut ui,
                &crate::settings::Preferences::default(),
            );
        }
        520 => {
            let position = pixel(&window, Vec2::new(450.0, 685.0));
            window.set_cursor_position(Some(position));
            mouse.press(MouseButton::Left);
        }
        525 => {
            assert_eq!(ui.screen, Screen::Language);
            let position = pixel(&window, Vec2::new(800.0, 435.0));
            window.set_cursor_position(Some(position));
            mouse.press(MouseButton::Left);
        }
        530 => {
            assert_eq!(ui.screen, Screen::Language);
            assert!(texts.iter().any(|text| text.0 == "Natrag"));
            keys.press(KeyCode::ArrowDown);
        }
        532 => keys.press(KeyCode::Enter),
        535 => {
            assert!(texts.iter().any(|text| text.0 == "Indietro"));
            keys.press(KeyCode::Escape);
        }
        540 => {
            assert_eq!(ui.screen, Screen::Menu);
            println!(
                "BEVY CHECK OK: language chooser selects Croatian and Italian with mouse and keyboard"
            );
            crate::app::begin(
                &mut simulation.0,
                &mut ui,
                &crate::settings::Preferences::default(),
                true,
            );
        }
        545 => {
            assert_eq!(hints.style, crate::controls::Style::Xbox);
            assert!(spans.iter().any(|(span, color)| {
                span.0 == "A"
                    && color.0
                        == crate::controls::Style::Xbox
                            .binding(crate::controls::Control::Wave)
                            .color
            }));
            commands
                .entity(test_pads.single().unwrap())
                .insert(Name::new("DualSense Wireless Controller"));
        }
        550 => {
            assert_eq!(hints.style, crate::controls::Style::PlayStation);
            for symbol in ["×", "□", "○"] {
                assert!(spans.iter().any(|(span, _)| span.0 == symbol));
            }
            commands
                .entity(test_pads.single().unwrap())
                .remove::<Gamepad>();
        }
        555 => {
            assert_eq!(hints.style, crate::controls::Style::Keyboard);
            assert!(
                !spans
                    .iter()
                    .any(|(span, _)| ["A", "×", "□", "○"].contains(&span.0.as_str()))
            );
            assert!(spans.iter().any(|(span, _)| span.0.contains("WASD")));
            println!(
                "BEVY CHECK OK: coloured Xbox/PlayStation hints and keyboard fallback update live"
            );
            assert!(!preferences.show_fps);
            ui.return_to = Screen::Playing;
            ui.screen = Screen::Settings;
        }
        556 | 557 => {
            keys.press(KeyCode::ArrowDown);
        }
        558 => {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    std::env::temp_dir().join("sabrina-bevy-slider-focus.png"),
                ));
        }
        560 | 570 => {
            let position = pixel(&window, Vec2::new(1045.0, 650.0));
            window.set_cursor_position(Some(position));
            mouse.press(MouseButton::Left);
        }
        565 => {
            assert!(preferences.show_fps);
            let (text, visibility) = fps_counter.single().unwrap();
            assert_eq!(*visibility, Visibility::Visible);
            let fps: f32 = text.0.strip_prefix("FPS: ").unwrap().parse().unwrap();
            assert!(fps.is_finite() && fps > 0.0);
            assert!(texts.iter().any(|text| text.0.starts_with("☑")));
        }
        575 => {
            assert!(!preferences.show_fps);
            assert_eq!(*fps_counter.single().unwrap().1, Visibility::Hidden);
            assert!(texts.iter().any(|text| text.0.starts_with("☐")));
            println!("BEVY CHECK OK: settings FPS checkbox shows a measured counter and hides it");
            check.vsync = preferences.vsync;
        }
        580 | 590 => {
            let position = pixel(&window, Vec2::new(540.0, 650.0));
            window.set_cursor_position(Some(position));
            mouse.press(MouseButton::Left);
        }
        585 | 595 => {
            let expected = check.vsync != (frame == 585);
            assert_eq!(preferences.vsync, expected);
            assert_eq!(window.present_mode, preferences.present_mode());
            if frame == 595 {
                println!("BEVY CHECK OK: settings V-sync button switches the present mode");
            }
        }
        600 => {
            commands
                .entity(test_pads.single().unwrap())
                .insert(Gamepad::default());
            crate::app::home(&mut simulation.0, &mut ui, &preferences);
        }
        605 | 615 => {
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::DPadDown);
        }
        610 => {
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::DPadRight);
        }
        612 => {
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::South);
        }
        614 => {
            assert_eq!(ui.screen, Screen::Settings);
            check.reduced_motion = preferences.reduced_motion;
        }
        616 => {
            // Fullscreen -> Motion; confirm toggles once rather than changing screens.
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::South);
        }
        618 => {
            assert_ne!(preferences.reduced_motion, check.reduced_motion);
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::East);
        }
        620 => {
            assert_eq!(ui.screen, Screen::Menu);
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::South);
        }
        625 => {
            assert_eq!(ui.screen, Screen::Playing);
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::Start);
        }
        630 => {
            assert_eq!(ui.screen, Screen::Pause);
            gamepads
                .single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::East);
        }
        635 => {
            assert_eq!(ui.screen, Screen::Playing);
            println!("BEVY CHECK OK: controller D-pad spatial navigation, confirm, back and pause");
            check.rumble = preferences.rumble;
            ui.return_to = Screen::Playing;
            ui.screen = Screen::Settings;
        }
        640 | 650 | 660 => {
            let position = pixel(&window, Vec2::new(540.0, 750.0));
            window.set_cursor_position(Some(position));
            mouse.press(MouseButton::Left);
        }
        645 | 655 | 665 => {
            let steps = ((frame - 635) / 10) as u8;
            assert_eq!(preferences.rumble, (check.rumble + steps) % 3);
            let label = format!(
                "{}{}",
                crate::ui::tr("Rumble: ", preferences.language),
                crate::ui::tr(
                    ["off", "Gentle", "Normal"][preferences.rumble as usize],
                    preferences.language
                )
            );
            assert!(texts.iter().any(|text| text.0 == label));
        }
        670 => {
            println!("BEVY CHECK OK: localized rumble settings cycle through all three levels");
            crate::app::begin(
                &mut simulation.0,
                &mut ui,
                &crate::settings::Preferences::default(),
                true,
            );
            check.ability_time = simulation.0.session.time;
        }
        671.. => {
            assert!(
                simulation.0.session.time - check.ability_time < 8.0,
                "Ability input verification timed out"
            );
            match check.ability_stage {
                0 => {
                    if simulation.0.sabrina.wave.cast_id == 0 {
                        keys.press(KeyCode::KeyF);
                    } else {
                        assert!(simulation.0.session.time - check.ability_time >= 3.0);
                        assert_eq!(simulation.0.sabrina.wave.scale, 2.0);
                        assert!(simulation.0.sabrina.wave.cooldown > 8.0);
                        println!(
                            "BEVY CHECK OK: held keyboard Wave auto-releases charged after 3 seconds"
                        );
                        simulation.0.sabrina.wave.cooldown = 0.0;
                        simulation.0.sabrina.wave.active = false;
                        check.ability_time = simulation.0.session.time;
                        check.ability_stage = 1;
                    }
                }
                1 => {
                    if simulation.0.sabrina.wave.cast_id == 1 {
                        let position = pixel(&window, Vec2::new(1350.0, 300.0));
                        window.set_cursor_position(Some(position));
                        mouse.press(MouseButton::Left);
                    } else {
                        assert!(simulation.0.session.time - check.ability_time >= 3.0);
                        assert_eq!(simulation.0.sabrina.wave.scale, 2.0);
                        println!(
                            "BEVY CHECK OK: held scene left-click Wave auto-releases charged after 3 seconds"
                        );
                        simulation.0.sabrina.wave.active = false;
                        simulation.0.sabrina.motion.pos = reef_sim::v(-8.0, 8.0);
                        simulation.0.sabrina.vitality.invulnerable = 10.0;
                        for index in 0..reef_sim::JELLY_COUNT {
                            let id = reef_sim::allocate_entity(
                                &mut simulation.0,
                                reef_sim::EntityKind::Jelly,
                                index as i32,
                            );
                            simulation.0.jelly[index] = reef_sim::JellyEntity {
                                id,
                                motion: reef_sim::Motion {
                                    pos: reef_sim::v(-14.0 + index as f32 * 28.0, 8.0),
                                    ..default()
                                },
                                behaviour: reef_sim::JellyBehaviour {
                                    state: if index < 2 {
                                        reef_sim::JellyState::Hungry
                                    } else {
                                        reef_sim::JellyState::Hidden
                                    },
                                    bubble: 3.0,
                                    ..default()
                                },
                            };
                        }
                        simulation.0.crystal.magic.state = reef_sim::Spell::Burst;
                        simulation.0.crystal.magic.time = 0.0;
                        simulation.0.crystal.magic.hit_count = 1;
                        simulation.0.crystal.magic.hits[0] = simulation.0.jelly[0].id;
                        simulation.0.crystal.motion.pos = reef_sim::v(-14.0, 8.0);
                        simulation.0.sabrina.cooldown = 12.0;
                        check.ability_stage = 2;
                        check.ability_time = simulation.0.session.time;
                    }
                }
                _ => {
                    if simulation.0.crystal.magic.hit_count == 1 {
                        keys.press(KeyCode::Space);
                    } else {
                        assert_eq!(simulation.0.jelly[1].behaviour.health, 80.0);
                        assert!(simulation.0.jelly[1].behaviour.bubble > 2.8);
                        assert!(simulation.0.sabrina.cooldown > 10.0);
                        println!(
                            "BEVY CHECK OK: Crystal homes across opposite sides of the scene for half damage"
                        );
                        check.ability_stage = 3;
                    }
                }
            }
        }
        _ => {}
    }
}

#[derive(Default)]
struct ReadingCheck {
    frame: u32,
    saved: Option<reef_sim::World>,
    pose: reef_sim::V2,
    offset: f32,
    bones: Vec<(Entity, Transform)>,
    reduced_motion: bool,
}

fn verify_reading(
    mut commands: Commands,
    options: Res<Options>,
    check: Res<Check>,
    mut reading: Local<ReadingCheck>,
    mut ui: ResMut<SessionUi>,
    simulation: Res<Simulation>,
    rendered: Res<crate::presentation::Presentation>,
    mut preferences: ResMut<crate::settings::Preferences>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut pads: Query<&mut Gamepad, With<TestPad>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    content: Query<(&Node, &ComputedNode, &TextFont), With<crate::ui::CrawlText>>,
    viewport: Query<(&Node, &ComputedNode), With<crate::ui::CrawlViewport>>,
    bones: Query<(Entity, &Transform, Option<&Name>), With<bevy::animation::AnimationTargetId>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !options.verify || check.ability_stage != 3 {
        return;
    }
    reading.frame += 1;
    let offset = || {
        let (node, _, _) = content.single().unwrap();
        let Val::Px(top) = node.top else {
            panic!("Crawl offset must use logical pixels")
        };
        -top
    };
    match reading.frame {
        1 => {
            reading.saved = Some(simulation.0.clone());
            reading.pose = rendered.0.sabrina.motion.pos;
            reading.reduced_motion = preferences.reduced_motion;
            reading.bones = bones
                .iter()
                .map(|(entity, transform, _)| (entity, *transform))
                .collect();
            preferences.reduced_motion = false;
            preferences.language = 1;
            ui.screen = Screen::Credits;
            ui.return_to = Screen::Pause;
            window.resolution.set(960.0, 540.0);
        }
        8 | 380 => {
            let (_, measured, font) = content.single().unwrap();
            let (node, viewport) = viewport.single().unwrap();
            assert_eq!(node.overflow, Overflow::clip());
            assert!(measured.size().y > viewport.size().y);
            assert!(viewport.size().x <= window.physical_width() as f32);
            assert!(viewport.size().y < window.physical_height() as f32);
            assert_eq!(font.font_size, FontSize::Px(38.0));
            assert_eq!(offset(), 0.0);
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    std::env::temp_dir().join(if reading.frame == 8 {
                        "sabrina-bevy-hr-credits-small.png"
                    } else {
                        "sabrina-bevy-it-help.png"
                    }),
                ));
        }
        340 => {
            assert!(
                offset() > 0.0,
                "Crawl must advance after the initial reading pause"
            );
            keys.press(KeyCode::KeyP);
        }
        342 => {
            reading.offset = offset();
            window.resolution.set(1280.0, 720.0);
        }
        360 => {
            assert!(
                (offset() - reading.offset).abs() < 0.01,
                "Pause and resize must preserve the reading position"
            );
            keys.press(KeyCode::End);
        }
        363 => {
            let (_, text, _) = content.single().unwrap();
            let (_, viewport) = viewport.single().unwrap();
            let limit = (text.size().y - viewport.size().y) * text.inverse_scale_factor();
            assert!(
                (offset() - limit).abs() < 0.1,
                "The final paragraph must be reachable"
            );
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    std::env::temp_dir().join("sabrina-bevy-credits-end.png"),
                ));
        }
        364 => keys.press(KeyCode::Home),
        365 => {
            assert_eq!(offset(), 0.0);
            preferences.reduced_motion = reading.reduced_motion;
            keys.press(KeyCode::Escape);
        }
        368 | 505 => {
            assert_eq!(ui.screen, Screen::Pause);
            let saved = reading.saved.as_ref().unwrap();
            assert_eq!(simulation.0.session.time, saved.session.time);
            assert_eq!(simulation.0.sabrina.motion.pos, saved.sabrina.motion.pos);
            assert_eq!(
                simulation.0.sabrina.vitality.health,
                saved.sabrina.vitality.health
            );
            assert_eq!(rendered.0.sabrina.motion.pos, reading.pose);
            for &(entity, before) in &reading.bones {
                let (_, after, name) = bones.get(entity).unwrap();
                assert!(before.translation.distance(after.translation) < 0.00001);
                assert!(
                    before.rotation.dot(after.rotation).abs() > 0.99999,
                    "Reading screen changed paused bone {entity:?} {name:?}: {:?} -> {:?}",
                    before.rotation,
                    after.rotation
                );
                assert!(before.scale.distance(after.scale) < 0.00001);
            }
            for i in 0..reef_sim::FISH_COUNT {
                assert_eq!(simulation.0.fish[i].motion.pos, saved.fish[i].motion.pos);
            }
            if reading.frame == 505 {
                println!(
                    "BEVY CHECK OK: clipped reading screens, resize, keyboard/controller scrolling, reduced motion, and paused-game restoration"
                );
                println!("BEVY INPUT INTEGRATION COMPLETE");
                exit.write(AppExit::Success);
            }
        }
        375 => {
            preferences.reduced_motion = true;
            preferences.language = 2;
            ui.screen = Screen::Help;
        }
        390..=420 => {
            pads.single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::DPadDown);
        }
        422 => {
            assert!(
                offset() > 50.0,
                "Controller must scroll even with reduced motion"
            );
            reading.offset = offset();
        }
        500 => {
            assert_eq!(
                offset(),
                reading.offset,
                "Reduced motion must not auto-scroll"
            );
            preferences.reduced_motion = reading.reduced_motion;
            pads.single_mut()
                .unwrap()
                .digital_mut()
                .press(GamepadButton::East);
        }
        _ => {}
    }
}

fn check_pose(
    options: Res<Options>,
    mut check: ResMut<Check>,
    palms: Query<&GlobalTransform, With<Palm>>,
    actors: Query<(&Actor, &GlobalTransform)>,
    actor_visibility: Query<(&Actor, &Visibility)>,
    effects: Query<(&crate::visuals::Effect, &Visibility)>,
    simulation: Res<Simulation>,
    bones: Query<(Entity, &Transform), With<bevy::animation::AnimationTargetId>>,
    faces: Query<&MorphWeights>,
    named_transforms: Query<(Entity, &Name, &Transform)>,
    surfaces: Query<(
        &bevy::gltf::GltfMaterialName,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    materials: Res<Assets<StandardMaterial>>,
    ui_checks: (
        Query<&ImageNode>,
        Query<(&Text, &TextColor, Option<&TextShadow>)>,
        Query<&BackgroundColor>,
    ),
    render_materials: (
        Res<Assets<crate::visuals::ReefMaterial>>,
        Res<Assets<crate::hair::HairMaterial>>,
        Res<crate::settings::Preferences>,
    ),
    lighting_checks: (
        Query<(&Msaa, &bevy::anti_alias::fxaa::Fxaa), With<Camera3d>>,
        Query<(Entity, &Actor)>,
        Query<&Children>,
        Query<
            (
                Option<&bevy::light::NotShadowCaster>,
                Option<&bevy::light::NotShadowReceiver>,
            ),
            With<Mesh3d>,
        >,
        Query<&DirectionalLight>,
        Res<bevy::light::DirectionalLightShadowMap>,
    ),
) {
    let (reef, hair, preferences) = render_materials;
    let (ui_images, ui_texts, ui_fills) = ui_checks;
    if !options.verify {
        return;
    }
    if check.frame == 55 {
        let (cameras, roots, children, surfaces, lights, shadow_map) = lighting_checks;
        assert!(cameras.iter().all(|(aa, fxaa)| *aa
            == if preferences.quality == 2 {
                Msaa::Sample4
            } else {
                Msaa::Off
            }
            && fxaa.enabled == (preferences.quality == 1)));
        assert_eq!(
            lights
                .iter()
                .filter(|light| light.shadow_maps_enabled)
                .count(),
            usize::from(preferences.quality > 0)
        );
        assert_eq!(
            shadow_map.size,
            if preferences.quality == 2 { 2048 } else { 1024 }
        );
        let mut excluded = 0;
        let mut fish_surfaces = 0;
        for (root, actor) in &roots {
            if matches!(actor, Actor::Crystal | Actor::Jelly(_)) {
                for descendant in children.iter_descendants(root) {
                    if let Ok((caster, receiver)) = surfaces.get(descendant) {
                        assert!(caster.is_some() && receiver.is_some());
                        excluded += 1;
                    }
                }
            }
            if matches!(actor, Actor::Fish(_)) {
                for descendant in children.iter_descendants(root) {
                    if let Ok((caster, receiver)) = surfaces.get(descendant) {
                        assert_eq!(caster.is_none(), preferences.quality == 2);
                        assert!(receiver.is_none());
                        fish_surfaces += 1;
                    }
                }
            }
        }
        assert!(excluded >= 9);
        assert!(fish_surfaces > 0);
        println!(
            "BEVY CHECK OK: quality AA, one tier-sized shadow map, shadow-free crystal and jellyfish"
        );
        assert!(
            ui_images
                .iter()
                .filter(|image| {
                    (image.color.alpha() - crate::ui::HUD_BACKGROUND_ALPHA).abs() < 0.001
                })
                .count()
                >= 2
        );
        assert!(
            ui_texts
                .iter()
                .all(
                    |(text, color, shadow)| (text.0.is_empty() || color.alpha() == 1.0)
                        && shadow.is_some()
                )
        );
        println!("BEVY CHECK OK: HUD backgrounds translucent; text opaque with contrast shadows");
        assert!(
            ui_fills
                .iter()
                .filter(|color| { (color.0.alpha() - crate::ui::HUD_METER_ALPHA).abs() < 0.001 })
                .count()
                >= 1
        );
        println!(
            "BEVY CHECK OK: health progress fill remains semi-transparent; abilities use icon sweeps"
        );
        assert_eq!(
            reef.iter()
                .filter(|(_, material)| material.base.alpha_mode == AlphaMode::Blend)
                .count(),
            1
        );
        assert_eq!(
            reef.iter()
                .filter(|(_, material)| material.base.alpha_mode == AlphaMode::Opaque)
                .count(),
            1
        );
        assert!(reef.iter().all(|(_, material)| material.extension.sabrina.w
            == if material.base.alpha_mode == AlphaMode::Blend {
                1.0
            } else {
                0.0
            }));
        println!("BEVY CHECK OK: only the near-left reef mesh enables the local player reveal");
    }
    if check.frame == 525 {
        assert!(
            reef.iter()
                .all(|(_, material)| material.extension.sabrina.w == 0.0)
        );
    }
    if check.frame == 480 {
        assert_eq!(
            actor_visibility
                .iter()
                .filter(|(actor, visibility)| {
                    matches!(actor, Actor::Jelly(_)) && **visibility == Visibility::Visible
                })
                .count(),
            4
        );
        println!("BEVY CHECK OK: all four final-round jellyfish rendered");
    }
    if check.frame == 375 {
        assert_eq!(simulation.0.crystal.magic.state, reef_sim::Spell::Resting);
        assert!(simulation.0.sabrina.cooldown > 0.0);
        assert!(actor_visibility.iter().any(|(actor, visibility)| {
            matches!(actor, Actor::Crystal) && *visibility == Visibility::Visible
        }));
        for (effect, visibility) in &effects {
            if matches!(
                effect,
                crate::visuals::Effect::Spark
                    | crate::visuals::Effect::Trail(_)
                    | crate::visuals::Effect::Glow
            ) {
                assert_eq!(*visibility, Visibility::Hidden);
            }
        }
        println!(
            "BEVY CHECK OK: returned crystal visible in orbit with no ready particles during cooldown"
        );
    }
    if check.frame == 450 {
        assert_eq!(simulation.0.crystal.magic.state, reef_sim::Spell::Resting);
        assert_eq!(simulation.0.sabrina.cooldown, 0.0);
        assert!(effects.iter().any(|(effect, visibility)| {
            matches!(effect, crate::visuals::Effect::Spark) && *visibility == Visibility::Visible
        }));
        println!("BEVY CHECK OK: ready orbiting crystal has particles");
    }
    if check.frame == 330 {
        check.idle_hair_time = hair.iter().next().unwrap().1.extension.settings.x;
        check.idle_hair = named_transforms
            .iter()
            .filter(|(_, name, _)| name.starts_with("Hair_") && name.ends_with("_3"))
            .map(|(entity, _, transform)| (entity, transform.rotation))
            .collect();
        assert_eq!(check.idle_hair.len(), crate::hair::HAIR_CHAINS);
        let mut translucent = 0;
        for (name, handle) in &surfaces {
            let material = materials.get(&handle.0).unwrap();
            if name.0 == "Jelly_Skin" {
                assert_eq!(material.alpha_mode, AlphaMode::Blend);
                assert!(material.base_color.alpha() < 0.7);
                translucent += 1;
            } else if name.0 == "Jelly_Face" {
                assert_eq!(material.alpha_mode, AlphaMode::Opaque);
            }
        }
        assert_eq!(translucent, reef_sim::JELLY_COUNT);
        println!("BEVY CHECK OK: translucent jellyfish skin with opaque faces");
    }
    if check.frame == 400 {
        assert!(check.idle_hair.iter().all(|(entity, before)| {
            let (_, _, after) = named_transforms.get(*entity).unwrap();
            before.angle_between(after.rotation) > 0.0001
        }));
        assert!(hair.iter().next().unwrap().1.extension.settings.x > check.idle_hair_time);
        println!("BEVY CHECK OK: all five hair chains move independently with simulation time");
    }
    if check.frame == 100 {
        let palm = palms.single().unwrap().transform_point(Vec3::new(
            0.0,
            reef_sim::sabrina_palm_offset(),
            0.0,
        ));
        let crystal = actors
            .iter()
            .find(|(a, _)| **a == Actor::Crystal)
            .unwrap()
            .1
            .translation();
        assert!(
            palm.distance(crystal) < 0.001,
            "held crystal must follow the rendered hand: {palm:?} {crystal:?}"
        );
        println!("BEVY CHECK OK: crystal follows the animated palm");
    }
    if check.frame == 206 {
        assert_eq!(hair.len(), 1);
        let material = hair.iter().next().unwrap().1;
        assert_eq!(material.base.alpha_mode, AlphaMode::Opaque);
        assert_eq!(material.base.anisotropy_strength, 0.35);
        assert!(material.base.normal_map_texture.is_some());
        assert_eq!(
            material.extension.settings.y,
            if preferences.reduced_motion {
                0.25
            } else {
                1.0
            }
        );
        check.paused_hair_shader = material.extension.settings;
        check.paused_pose = bones.iter().map(|(e, t)| (e, *t)).collect();
        check.paused_face = faces.single().unwrap().weights().to_vec();
        assert_eq!(check.paused_face.len(), 3);
    }
    if check.frame == 230 {
        assert_eq!(
            hair.iter().next().unwrap().1.extension.settings,
            check.paused_hair_shader
        );
        println!(
            "BEVY CHECK OK: one opaque hair material freezes with simulation time during pause"
        );
        for (entity, before) in &check.paused_pose {
            let (_, after) = bones.get(*entity).unwrap();
            assert!(before.translation.distance(after.translation) < 0.00001);
            assert!(before.rotation.dot(after.rotation).abs() > 0.99999);
        }
        assert_eq!(check.paused_face, faces.single().unwrap().weights());
        println!("BEVY CHECK OK: pause freezes skeletal animation, hair, face and crossfades");
    }
}
