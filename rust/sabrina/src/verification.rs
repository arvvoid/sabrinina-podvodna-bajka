// SPDX-License-Identifier: MIT
//! Drives actual Bevy input resources through the same live systems used by play.
use crate::{
    app::{Options, Screen, SessionUi, Simulation},
    assets::{Actor, Palm, RigPlayer},
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
}
#[derive(Component)]
struct TestPad;
pub struct VerificationPlugin;
impl Plugin for VerificationPlugin {
    fn build(&self, a: &mut App) {
        a.init_resource::<Check>()
            .add_systems(
                PreUpdate,
                drive
                    .after(bevy::input::InputSystems)
                    .before(crate::app::input),
            )
            .add_systems(
                PostUpdate,
                check_pose.after(bevy::transform::TransformSystems::Propagate),
            );
    }
}
fn pixel(window: &Window, p: Vec2) -> Vec2 {
    let z = (window.width() / 1600.0).min(window.height() / 900.0);
    p * z
        + Vec2::new(
            (window.width() - 1600.0 * z) / 2.0,
            (window.height() - 900.0 * z) / 2.0,
        )
}
fn drive(
    mut c: Commands,
    options: Res<Options>,
    mut check: ResMut<Check>,
    mut w: ResMut<Simulation>,
    mut u: ResMut<SessionUi>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut motion: ResMut<AccumulatedMouseMotion>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut pad: Query<&mut Gamepad, With<TestPad>>,
    rigs: Query<&RigPlayer>,
    mut exit: MessageWriter<AppExit>,
) {
    if !options.verify || !u.loaded || rigs.iter().count() < 76 {
        return;
    }
    check.frame += 1;
    let frame = check.frame;
    keys.reset_all();
    mouse.reset_all();
    motion.delta = Vec2::ZERO;
    if let Ok(mut p) = pad.single_mut() {
        p.digital_mut().reset_all();
        p.analog_mut().set(GamepadAxis::LeftStickX, 0.0);
        p.analog_mut().set(GamepadAxis::LeftStickY, 0.0);
    }
    match frame {
        1 => {
            assert_eq!(u.screen, Screen::Playing);
            check.initial = w.0.sabrina.motion.pos;
            c.spawn((Gamepad::default(), TestPad));
        }
        2..=40 => {
            keys.press(KeyCode::KeyW);
        }
        41 => {
            assert!(w.0.sabrina.motion.pos.y > check.initial.y + 0.5);
            println!("BEVY CHECK OK: keyboard swimming");
            keys.press(KeyCode::KeyQ);
        }
        45 => {
            assert!(w.0.sabrina.song.active);
            assert!(w.0.sabrina.song.energy < 100.0);
            println!("BEVY CHECK OK: keyboard song");
            keys.press(KeyCode::KeyQ);
        }
        50 => {
            assert!(!w.0.sabrina.song.active);
            keys.press(KeyCode::KeyF);
        }
        55 => {
            assert!(w.0.sabrina.wave.active);
            println!("BEVY CHECK OK: keyboard wave");
            w.0.sabrina.wave.active = false;
            keys.press(KeyCode::Space);
        }
        60 => {
            assert_eq!(w.0.crystal.magic.state, reef_sim::Spell::Charging);
            println!("BEVY CHECK OK: crystal hand wind-up");
        }
        150 => {
            assert!(w.0.sabrina.cooldown > 140.0);
            println!("BEVY CHECK OK: crystal launch and recharge");
            check.initial = w.0.sabrina.motion.pos;
        }
        151..=190 => {
            if let Ok(mut p) = pad.single_mut() {
                p.analog_mut().set(GamepadAxis::LeftStickX, 1.0);
            }
        }
        191 => {
            assert!(w.0.sabrina.motion.pos.x > check.initial.x + 0.5);
            println!("BEVY CHECK OK: controller swimming");
            if let Ok(mut p) = pad.single_mut() {
                p.digital_mut().press(GamepadButton::West);
            }
        }
        196 => {
            assert!(w.0.sabrina.song.active);
            println!("BEVY CHECK OK: controller song");
            w.0.sabrina.wave.cooldown = 0.0;
            if let Ok(mut p) = pad.single_mut() {
                p.digital_mut().press(GamepadButton::East);
            }
        }
        201 => {
            assert!(w.0.sabrina.wave.active);
            println!("BEVY CHECK OK: controller wave");
            keys.press(KeyCode::Escape);
        }
        205 => {
            assert_eq!(u.screen, Screen::Pause);
            check.paused_time = w.0.session.time;
        }
        235 => {
            assert_eq!(w.0.session.time, check.paused_time);
            println!("BEVY CHECK OK: pause freezes gameplay");
            keys.press(KeyCode::Escape);
        }
        240 => {
            assert_eq!(u.screen, Screen::Playing);
            check.initial = w.0.sabrina.motion.pos;
            let p = pixel(&window, Vec2::new(1350.0, 300.0));
            window.set_cursor_position(Some(p));
            motion.delta = Vec2::new(10.0, 0.0);
        }
        280 => {
            assert!(w.0.sabrina.motion.pos.x > check.initial.x);
            println!("BEVY CHECK OK: mouse swimming");
            w.0.sabrina.wave.cooldown = 0.0;
            w.0.sabrina.wave.active = false;
            w.0.sabrina.throw_time = -1.0;
            w.0.sabrina.cooldown = 0.0;
            w.0.crystal.magic.state = reef_sim::Spell::Resting;
            let p = pixel(&window, Vec2::new(1080.0, 48.0));
            window.set_cursor_position(Some(p));
            mouse.press(MouseButton::Left);
        }
        285 => {
            assert!(w.0.sabrina.wave.active);
            assert_eq!(w.0.crystal.magic.state, reef_sim::Spell::Resting);
            println!("BEVY CHECK OK: HUD wave does not cast crystal");
            w.0.sabrina.vitality.health = 10.0;
            w.0.sabrina.vitality.invulnerable = 0.0;
            w.0.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
            w.0.jelly[0].motion.pos = w.0.sabrina.motion.pos;
        }
        290 => {
            assert_eq!(u.screen, Screen::GameOver);
            keys.press(KeyCode::Escape);
        }
        295 => {
            assert_eq!(u.screen, Screen::GameOver);
            println!("BEVY CHECK OK: game-over cannot be dismissed with pause");
            let p = pixel(&window, Vec2::new(800.0, 530.0));
            window.set_cursor_position(Some(p));
            mouse.press(MouseButton::Left);
        }
        300 => {
            assert_eq!(u.screen, Screen::Playing);
            assert_eq!(w.0.sabrina.vitality.health, 100.0);
            println!("BEVY CHECK OK: retry resets the session");
            u.pointer_mode = false;
        }
        410 => {
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
    bones: Query<(Entity, &Transform), With<bevy::animation::AnimationTargetId>>,
    faces: Query<&MorphWeights>,
    named_transforms: Query<(Entity, &Name, &Transform)>,
    surfaces: Query<(
        &bevy::gltf::GltfMaterialName,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    materials: Res<Assets<StandardMaterial>>,
) {
    if !options.verify {
        return;
    }
    if check.frame == 330 {
        check.idle_hair = named_transforms
            .iter()
            .filter(|(_, name, _)| name.starts_with("Hair_") && name.ends_with("_3"))
            .map(|(entity, _, transform)| (entity, transform.rotation))
            .collect();
        assert_eq!(check.idle_hair.len(), 3);
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
        assert_eq!(translucent, 3);
        println!("BEVY CHECK OK: translucent jellyfish skin with opaque faces");
    }
    if check.frame == 400 {
        assert!(check.idle_hair.iter().all(|(entity, before)| {
            let (_, _, after) = named_transforms.get(*entity).unwrap();
            before.angle_between(after.rotation) > 0.015
        }));
        println!("BEVY CHECK OK: all three hair tips sway while resting");
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
        check.paused_pose = bones.iter().map(|(e, t)| (e, *t)).collect();
        check.paused_face = faces.single().unwrap().weights().to_vec();
        assert_eq!(check.paused_face.len(), 3);
    }
    if check.frame == 230 {
        for (entity, before) in &check.paused_pose {
            let (_, after) = bones.get(*entity).unwrap();
            assert!(before.translation.distance(after.translation) < 0.00001);
            assert!(before.rotation.dot(after.rotation).abs() > 0.99999);
        }
        assert_eq!(check.paused_face, faces.single().unwrap().weights());
        println!("BEVY CHECK OK: pause freezes skeletal animation, hair, face and crossfades");
    }
}
