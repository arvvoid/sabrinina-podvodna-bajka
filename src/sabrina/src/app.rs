// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Session transitions and the handoff from per-frame input to fixed-step rules.
use crate::settings::Preferences;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};
use reef_sim as sim;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderBackend {
    Dx12,
    Vulkan,
    Metal,
    OpenGl,
}

impl RenderBackend {
    pub fn from_flag(argument: &str) -> Option<Self> {
        match argument {
            "--dx12" => Some(Self::Dx12),
            "--vulkan" => Some(Self::Vulkan),
            "--metal" => Some(Self::Metal),
            "--opengl" => Some(Self::OpenGl),
            _ => None,
        }
    }

    pub fn flag(self) -> &'static str {
        match self {
            Self::Dx12 => "--dx12",
            Self::Vulkan => "--vulkan",
            Self::Metal => "--metal",
            Self::OpenGl => "--opengl",
        }
    }
}

#[derive(Resource, Default)]
pub struct Options {
    pub capture: Option<String>,
    pub capture_frames: Option<std::path::PathBuf>,
    pub school: bool,
    pub verify: bool,
    pub fullhd: bool,
    pub capture_fxaa: bool,
    pub direct_draw: bool,
    pub backend: Option<RenderBackend>,
    pub wgi: bool,
}

impl Options {
    pub fn from_args() -> Self {
        let mut options = Self::from_arguments(std::env::args(), crate::current_platform());
        options.capture_fxaa = std::env::var("SABRINA_CAPTURE_AA").as_deref() == Ok("fxaa");
        if let Some(directory) = std::env::var_os("SABRINA_CAPTURE_FRAMES") {
            assert_eq!(
                options.capture.as_deref(),
                Some("demo"),
                "SABRINA_CAPTURE_FRAMES requires --capture-demo"
            );
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)
                .expect("SABRINA_CAPTURE_FRAMES must point to a writable directory");
            options.capture_frames = Some(directory);
        }
        options
    }

    fn from_arguments(
        arguments: impl IntoIterator<Item = impl AsRef<str>>,
        platform: crate::Platform,
    ) -> Self {
        let mut options = Self::default();
        let mut direct_draw = None;
        for argument in arguments {
            let argument = argument.as_ref();
            if argument == "--fullhd" {
                options.fullhd = true;
            }
            if argument == "--verify-input" {
                options.verify = true;
            }
            if argument == "--wgi" {
                options.wgi = true;
            }
            if argument == "--direct-draw" {
                direct_draw = Some(true);
            }
            if argument == "--indirect-draw" {
                direct_draw = Some(false);
            }
            if let Some(backend) = RenderBackend::from_flag(argument) {
                options.backend = Some(backend);
            }
            if let Some(kind) = argument.strip_prefix("--capture-") {
                options.capture = Some(kind.into());
                options.school = kind == "school";
            }
        }
        options.direct_draw = direct_draw.unwrap_or_else(|| {
            crate::selected_backend(platform, options.backend) == RenderBackend::Dx12
        });
        options
    }
}

#[derive(Resource)]
pub struct Simulation(pub sim::World);

#[derive(Resource, Default)]
pub struct PendingInput(pub sim::Input);

#[derive(Resource, Default)]
pub struct FrameEvents(pub Vec<sim::Events>);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Loading,
    Menu,
    Playing,
    Pause,
    Settings,
    Help,
    Credits,
    GameOver,
    Victory,
    Language,
}

#[derive(Resource)]
pub struct SessionUi {
    pub screen: Screen,
    pub return_to: Screen,
    pub started: bool,
    pub pointer_mode: bool,
    pub elapsed: f32,
    pub notice: String,
    pub notice_time: f32,
    pub score_time: f32,
    pub caption: String,
    pub loaded: bool,
}

impl Default for SessionUi {
    fn default() -> Self {
        Self {
            screen: Screen::Loading,
            return_to: Screen::Menu,
            started: false,
            pointer_mode: false,
            elapsed: 0.0,
            notice: String::new(),
            notice_time: 0.0,
            score_time: 0.0,
            caption: String::new(),
            loaded: false,
        }
    }
}

impl SessionUi {
    pub fn paused(&self) -> bool {
        !matches!(self.screen, Screen::Menu | Screen::Playing)
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionUi>()
            .init_resource::<PendingInput>()
            .init_resource::<FrameEvents>()
            .init_resource::<crate::presentation::Previous>()
            .init_resource::<crate::presentation::Presentation>()
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, input.after(bevy::input::InputSystems))
            .add_systems(Update, update_clock)
            .add_systems(FixedUpdate, tick)
            .add_systems(
                Update,
                crate::presentation::prepare
                    .after(crate::assets::finish_load)
                    .after(update_clock),
            );
    }
}

fn setup(mut commands: Commands, preferences: Res<Preferences>) {
    let mut world = sim::World::default();
    sim::reset(&mut world, 27, preferences.simulation(true));
    sim::add_fish(&mut world, 16, false);
    world.sabrina.motion.pos = sim::v(4.0, 6.0);
    commands.insert_resource(Simulation(world));
}

pub fn begin(
    world: &mut sim::World,
    ui: &mut SessionUi,
    preferences: &Preferences,
    aquarium: bool,
) {
    sim::reset(
        world,
        27 + (ui.elapsed * 10.0) as u32,
        preferences.simulation(aquarium),
    );
    world.points.best_score = preferences.best;
    ui.screen = Screen::Playing;
    ui.started = true;
    ui.pointer_mode = false;
    ui.score_time = 0.0;
    ui.caption.clear();
}

pub fn home(world: &mut sim::World, ui: &mut SessionUi, preferences: &Preferences) {
    sim::reset(world, 27, preferences.simulation(true));
    sim::add_fish(world, 16, false);
    world.sabrina.motion.pos = sim::v(4.0, 6.0);
    ui.screen = Screen::Menu;
    ui.started = false;
    ui.pointer_mode = false;
    ui.caption.clear();
}

pub fn pause(ui: &mut SessionUi) {
    match ui.screen {
        Screen::Playing => ui.screen = Screen::Pause,
        Screen::Pause => ui.screen = Screen::Playing,
        Screen::Settings | Screen::Help | Screen::Credits | Screen::Language => {
            ui.screen = ui.return_to
        }
        _ => {}
    }
}

fn update_clock(time: Res<Time>, mut ui: ResMut<SessionUi>) {
    ui.elapsed += time.delta_secs();
    ui.notice_time = (ui.notice_time - time.delta_secs()).max(0.0);
    if !ui.paused() {
        ui.score_time = (ui.score_time - time.delta_secs()).max(0.0)
    }
}

pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    pads: Query<&Gamepad>,
    hints: Res<crate::controls::Hints>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut ui: ResMut<SessionUi>,
    mut pending: ResMut<PendingInput>,
    mut prefs: ResMut<Preferences>,
    options: Res<Options>,
) {
    if options.capture.is_some() {
        return;
    }
    if !window.focused && !options.verify {
        pending.0 = sim::Input::default();
        ui.pointer_mode = false;
        return;
    }
    let mut pausing = keys.just_pressed(KeyCode::Escape);
    let mut toggle = keys.just_pressed(KeyCode::KeyQ);
    let mut wave = keys.just_pressed(KeyCode::KeyF) || mouse.just_pressed(MouseButton::Right);
    let mut cast = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::KeyE)
        || mouse.just_pressed(MouseButton::Left);
    let mut movement = Vec2::new(
        (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) as u8 as f32
            - (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) as u8 as f32,
        (keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)) as u8 as f32
            - (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) as u8 as f32,
    );
    if let Some(pad) = hints.active.and_then(|entity| pads.get(entity).ok()) {
        let stick = pad.left_stick();
        if stick.length() > 0.2 {
            movement += stick * ((stick.length() - 0.2) / 0.8 / stick.length());
        }
        pausing |= pad.just_pressed(GamepadButton::Start);
        toggle |= pad.just_pressed(GamepadButton::West);
        wave |= pad.just_pressed(GamepadButton::East);
        cast |=
            pad.just_pressed(GamepadButton::South) || pad.just_pressed(GamepadButton::RightTrigger);
    }
    if pausing {
        let screen = ui.screen;
        pause(&mut ui);
        if ui.screen != screen {
            pending.0 = sim::Input::default();
            prefs.save();
            return;
        }
    }
    if keys.just_pressed(KeyCode::F11) {
        prefs.fullscreen = !prefs.fullscreen;
        window.mode = if prefs.fullscreen {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            WindowMode::Windowed
        };
        prefs.save();
    }
    if ui.screen != Screen::Playing {
        pending.0 = sim::Input::default();
        return;
    }
    if motion.delta.length() > 1.0 {
        ui.pointer_mode = true
    }
    if movement.length() > 0.1 {
        ui.pointer_mode = false
    }
    pending.0.movement = sim::v(movement.x, movement.y);
    // Pointer target is projected in tick, after any fixed-step movement.
    if ui.pointer_mode
        && let (Some(pos), Ok((camera, transform))) = (window.cursor_position(), camera.single())
        && let Ok(ray) = camera.viewport_to_world(transform, pos)
    {
        let t = -ray.origin.z / ray.direction.z;
        let p = ray.origin + ray.direction * t;
        pending.0.movement = sim::v(p.x, p.y);
    }
    // UI consumes pointer clicks before tick so a button cannot also cast a crystal.
    pending.0.cast |= cast;
    pending.0.wave |= wave;
    pending.0.toggle_song |= toggle;
}

fn tick(
    mut simulation: ResMut<Simulation>,
    mut ui: ResMut<SessionUi>,
    mut pending: ResMut<PendingInput>,
    mut events: ResMut<FrameEvents>,
    mut prefs: ResMut<Preferences>,
    options: Res<Options>,
    mut capture_action: Local<bool>,
    mut demo_tick: Local<u32>,
    mut previous: ResMut<crate::presentation::Previous>,
) {
    if !ui.loaded
        || ui.paused()
        || options.capture.as_deref().is_some_and(|k| {
            k.starts_with("graphics-")
                || (k.starts_with("pose-") && k != "pose-idle" && k != "pose-stroke")
                || k == "rig"
        })
    {
        return;
    }
    let mut input = pending.0;
    if options.capture.as_deref() == Some("demo") {
        *demo_tick += 1;
        input.movement = match *demo_tick {
            120..=210 => sim::v(0.65, 0.0),
            260..=360 => sim::v(-0.65, 0.25),
            500..=620 => sim::v(-0.5, -0.25),
            740..=810 => sim::v(0.5, 0.0),
            _ => sim::v(0.0, 0.0),
        };
        input.toggle_song = matches!(*demo_tick, 360 | 480);
        input.cast = matches!(*demo_tick, 60 | 900);
        input.wave = matches!(*demo_tick, 240 | 600 | 840);
    }
    if let Some(kind) = &options.capture
        && kind != "demo"
    {
        let due = match kind.as_str() {
            "throw" => 3.25,
            "wave" => 3.5,
            "combat" => 1.5,
            _ => 1.0,
        };
        if !*capture_action && simulation.0.session.time >= due {
            *capture_action = true;
            input.cast = kind == "throw" || kind == "crystal" || kind == "combat";
            input.toggle_song = kind == "song";
            input.wave = kind == "wave";
        }
    }
    if ui.pointer_mode {
        input.movement = (input.movement - simulation.0.sabrina.motion.pos) * 0.5;
    }
    if options.capture.as_deref() == Some("pose-idle") {
        input.movement = sim::v(0.0, 0.0);
    }
    if options.capture.as_deref() == Some("pose-stroke") {
        input.movement = sim::v(0.65, 0.0);
    }
    // Consume one-shot actions once, even when a rendered frame runs several ticks.
    pending.0.cast = false;
    pending.0.wave = false;
    pending.0.toggle_song = false;
    simulation.0.settings.reduced_motion = prefs.reduced_motion;
    previous.0 = Some(simulation.0.clone());
    let frame_events = sim::step(&mut simulation.0, 1.0 / 60.0, input);
    if options.capture.as_deref() == Some("pose-stroke") {
        simulation.0.sabrina.motion.pos = sim::v(0.0, 8.0);
    }
    if frame_events.visit_ended {
        ui.score_time = 10.0;
        prefs.best = prefs.best.max(simulation.0.points.best_score);
        if options.capture.is_none() && !options.verify {
            prefs.save()
        }
    }
    if frame_events.game_over {
        ui.screen = Screen::GameOver
    } else if frame_events.victory {
        ui.screen = Screen::Victory;
        ui.score_time = 0.0;
        ui.notice_time = 0.0;
        ui.caption.clear();
    }
    events.0.push(frame_events);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::{
        InputPlugin,
        gamepad::{RawGamepadButtonChangedEvent, RawGamepadEvent},
    };
    use std::time::Duration;

    #[test]
    fn draw_defaults_follow_the_selected_backend() {
        use crate::Platform::{Apple, Other, Windows};

        for platform in [Windows, Apple, Other] {
            for flag in [
                None,
                Some("--dx12"),
                Some("--vulkan"),
                Some("--metal"),
                Some("--opengl"),
            ] {
                let options = Options::from_arguments(flag, platform);
                let backend = crate::selected_backend(platform, options.backend);
                assert_eq!(
                    options.direct_draw,
                    backend == RenderBackend::Dx12,
                    "{platform:?}, {flag:?}"
                );
            }
        }
        assert!(Options::from_arguments(["sabrina"], Windows).direct_draw);
        assert!(!Options::from_arguments(["sabrina"], Apple).direct_draw);
        assert!(!Options::from_arguments(["sabrina"], Other).direct_draw);
    }

    #[test]
    fn draw_flags_override_defaults_regardless_of_backend_flag_order() {
        use crate::Platform::{Apple, Other, Windows};

        for platform in [Windows, Apple, Other] {
            for backend in ["--dx12", "--vulkan", "--metal", "--opengl"] {
                for (draw, expected) in [("--direct-draw", true), ("--indirect-draw", false)] {
                    for arguments in [[backend, draw], [draw, backend]] {
                        let options = Options::from_arguments(arguments, platform);
                        assert_eq!(options.direct_draw, expected, "{platform:?}, {arguments:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn last_draw_flag_wins() {
        for platform in [
            crate::Platform::Windows,
            crate::Platform::Apple,
            crate::Platform::Other,
        ] {
            assert!(
                Options::from_arguments(["--indirect-draw", "--direct-draw"], platform).direct_draw
            );
            assert!(
                !Options::from_arguments(["--direct-draw", "--indirect-draw"], platform)
                    .direct_draw
            );
        }
    }

    #[test]
    fn draw_selection_preserves_other_runtime_options() {
        let options = Options::from_arguments(
            [
                "--fullhd",
                "--verify-input",
                "--capture-school",
                "--indirect-draw",
                "--dx12",
            ],
            crate::Platform::Windows,
        );
        assert!(options.fullhd);
        assert!(options.verify);
        assert!(options.school);
        assert_eq!(options.capture.as_deref(), Some("school"));
        assert_eq!(options.backend, Some(RenderBackend::Dx12));
        assert!(!options.direct_draw);
    }

    #[test]
    fn wgi_is_opt_in_and_does_not_change_renderer_selection() {
        assert!(!Options::from_arguments(["sabrina"], crate::Platform::Windows).wgi);
        let options = Options::from_arguments(["--wgi", "--vulkan"], crate::Platform::Windows);
        assert!(options.wgi);
        assert_eq!(options.backend, Some(RenderBackend::Vulkan));
        assert!(!options.direct_draw);
    }

    #[test]
    fn quick_gameplay_taps_survive_frames_with_zero_or_multiple_fixed_ticks() {
        for fps in [20.0, 60.0, 144.0, 240.0] {
            let mut app = App::new();
            let preferences = Preferences::default();
            let mut world = sim::World::default();
            sim::reset(&mut world, 27, preferences.simulation(true));
            app.add_plugins((MinimalPlugins, InputPlugin))
                .init_resource::<crate::controls::Hints>()
                .init_resource::<PendingInput>()
                .init_resource::<FrameEvents>()
                .init_resource::<crate::presentation::Previous>()
                .insert_resource(preferences)
                .insert_resource(Simulation(world))
                .insert_resource(Options {
                    verify: true,
                    ..default()
                })
                .insert_resource(SessionUi {
                    screen: Screen::Playing,
                    loaded: true,
                    ..default()
                })
                .insert_resource(Time::<Fixed>::from_hz(60.0))
                .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                    Duration::from_secs_f64(1.0 / fps),
                ))
                .add_systems(
                    PreUpdate,
                    (crate::controls::update, input)
                        .chain()
                        .after(bevy::input::InputSystems),
                )
                .add_systems(FixedUpdate, tick);
            app.world_mut().spawn((
                Window {
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
            ));
            let pad = app.world_mut().spawn(Gamepad::default()).id();
            app.update();
            for expected in [true, false, true, false, true, false] {
                for value in [1.0, 0.0] {
                    app.world_mut().write_message(RawGamepadEvent::Button(
                        RawGamepadButtonChangedEvent::new(pad, GamepadButton::West, value),
                    ));
                }
                for _ in 0..12 {
                    app.update();
                }
                assert_eq!(
                    app.world().resource::<Simulation>().0.sabrina.song.active,
                    expected,
                    "A quick song tap must be consumed exactly once at {fps} FPS"
                );
                assert!(!app.world().resource::<PendingInput>().0.toggle_song);
            }
        }
    }
}
