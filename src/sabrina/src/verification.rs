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
}

#[derive(Component)]
struct TestPad;

pub struct VerificationPlugin;

impl Plugin for VerificationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Check>()
            .add_systems(
                PreUpdate,
                drive
                    .after(bevy::input::InputSystems)
                    .before(crate::controls::update)
                    .before(crate::app::input),
            )
            .add_systems(
                PostUpdate,
                check_pose.after(bevy::transform::TransformSystems::Propagate),
            );
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
    mut exit: MessageWriter<AppExit>,
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
        55 => {
            assert!(simulation.0.sabrina.wave.active);
            println!("BEVY CHECK OK: keyboard wave");
            simulation.0.sabrina.wave.active = false;
            keys.press(KeyCode::Space);
        }
        60 => {
            assert_eq!(simulation.0.crystal.magic.state, reef_sim::Spell::Charging);
            println!("BEVY CHECK OK: crystal hand wind-up");
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
                gamepad.digital_mut().press(GamepadButton::West);
            }
        }
        196 => {
            assert!(simulation.0.sabrina.song.active);
            println!("BEVY CHECK OK: controller song");
            simulation.0.sabrina.wave.cooldown = 0.0;
            if let Ok(mut gamepad) = gamepads.single_mut() {
                gamepad.digital_mut().press(GamepadButton::East);
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
            let pointer_position = pixel(&window, Vec2::new(1080.0, 48.0));
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
                span.0 == " · A"
                    && color.0
                        == crate::controls::Style::Xbox
                            .binding(crate::controls::Control::Crystal)
                            .color
            }));
            commands
                .entity(test_pads.single().unwrap())
                .insert(Name::new("DualSense Wireless Controller"));
        }
        550 => {
            assert_eq!(hints.style, crate::controls::Style::PlayStation);
            for symbol in [" · ×", " · □", " · ○"] {
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
                    .any(|(span, _)| [" · A", " · ×", " · □", " · ○"].contains(&span.0.as_str()))
            );
            assert!(spans.iter().any(|(span, _)| span.0.contains("WASD")));
            println!(
                "BEVY CHECK OK: coloured Xbox/PlayStation hints and keyboard fallback update live"
            );
            assert!(!preferences.show_fps);
            ui.return_to = Screen::Playing;
            ui.screen = Screen::Settings;
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
            println!("BEVY INPUT INTEGRATION COMPLETE");
            exit.write(AppExit::Success);
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
        Query<(&TextColor, Option<&TextShadow>), With<Text>>,
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
        for (root, actor) in &roots {
            if matches!(actor, Actor::Crystal | Actor::Jelly(_)) {
                for descendant in children.iter_descendants(root) {
                    if let Ok((caster, receiver)) = surfaces.get(descendant) {
                        assert!(caster.is_some() && receiver.is_some());
                        excluded += 1;
                    }
                }
            }
        }
        assert!(excluded >= 9);
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
                >= 8
        );
        assert!(
            ui_texts
                .iter()
                .all(|(color, shadow)| color.alpha() == 1.0 && shadow.is_some())
        );
        println!("BEVY CHECK OK: HUD backgrounds translucent; text opaque with contrast shadows");
        assert!(
            ui_fills
                .iter()
                .filter(|color| { (color.0.alpha() - crate::ui::HUD_METER_ALPHA).abs() < 0.001 })
                .count()
                >= 4
        );
        println!("BEVY CHECK OK: health and ability progress fills are semi-transparent");
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
