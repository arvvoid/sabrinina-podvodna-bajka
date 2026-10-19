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
}

impl RenderBackend {
    pub fn from_flag(argument: &str) -> Option<Self> {
        match argument {
            "--dx12" => Some(Self::Dx12),
            "--vulkan" => Some(Self::Vulkan),
            "--metal" => Some(Self::Metal),
            _ => None,
        }
    }

    pub fn flag(self) -> &'static str {
        match self {
            Self::Dx12 => "--dx12",
            Self::Vulkan => "--vulkan",
            Self::Metal => "--metal",
        }
    }
}

#[derive(Resource, Default)]
pub struct Options {
    pub capture: Option<String>,
    pub school: bool,
    pub verify: bool,
    pub fullhd: bool,
    pub capture_fxaa: bool,
    pub direct_draw: bool,
    pub backend: Option<RenderBackend>,
}

impl Options {
    pub fn from_args() -> Self {
        let mut options = Self {
            capture_fxaa: std::env::var("SABRINA_CAPTURE_AA").as_deref() == Ok("fxaa"),
            ..Self::default()
        };
        for argument in std::env::args() {
            if argument == "--fullhd" {
                options.fullhd = true;
            }
            if argument == "--verify-input" {
                options.verify = true;
            }
            if argument == "--direct-draw" {
                options.direct_draw = true;
            }
            if let Some(backend) = RenderBackend::from_flag(&argument) {
                options.backend = Some(backend);
            }
            if let Some(kind) = argument.strip_prefix("--capture-") {
                options.capture = Some(kind.into());
                options.school = kind == "school";
            }
        }
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
    for pad in &pads {
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
        pause(&mut ui)
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
            k.starts_with("graphics-") || (k.starts_with("pose-") && k != "pose-idle") || k == "rig"
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
    // Consume one-shot actions once, even when a rendered frame runs several ticks.
    pending.0.cast = false;
    pending.0.wave = false;
    pending.0.toggle_song = false;
    simulation.0.settings.reduced_motion = prefs.reduced_motion;
    previous.0 = Some(simulation.0.clone());
    let frame_events = sim::step(&mut simulation.0, 1.0 / 60.0, input);
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
