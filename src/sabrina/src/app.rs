// SPDX-License-Identifier: MIT
use crate::settings::Preferences;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};
use reef_sim as sim;
#[derive(Resource, Default)]
pub struct Options {
    pub capture: Option<String>,
    pub school: bool,
    pub verify: bool,
    pub fullhd: bool,
    pub capture_fxaa: bool,
}
impl Options {
    pub fn from_args() -> Self {
        let mut s = Self {
            capture_fxaa: std::env::var("SABRINA_CAPTURE_AA").as_deref() == Ok("fxaa"),
            ..Self::default()
        };
        for a in std::env::args() {
            if a == "--fullhd" {
                s.fullhd = true;
            }
            if a == "--verify-input" {
                s.verify = true;
            }
            if let Some(k) = a.strip_prefix("--capture-") {
                s.capture = Some(k.into());
                s.school = k == "school";
            }
        }
        s
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
fn setup(mut c: Commands, p: Res<Preferences>) {
    let mut w = sim::World::default();
    sim::reset(&mut w, 27, p.simulation(true));
    sim::add_fish(&mut w, 16, false);
    w.sabrina.motion.pos = sim::v(4.0, 6.0);
    c.insert_resource(Simulation(w));
}
pub fn begin(w: &mut sim::World, u: &mut SessionUi, p: &Preferences, aquarium: bool) {
    sim::reset(w, 27 + (u.elapsed * 10.0) as u32, p.simulation(aquarium));
    w.points.best_score = p.best;
    u.screen = Screen::Playing;
    u.started = true;
    u.pointer_mode = false;
    u.score_time = 0.0;
    u.caption.clear();
}
pub fn home(w: &mut sim::World, u: &mut SessionUi, p: &Preferences) {
    sim::reset(w, 27, p.simulation(true));
    sim::add_fish(w, 16, false);
    w.sabrina.motion.pos = sim::v(4.0, 6.0);
    u.screen = Screen::Menu;
    u.started = false;
    u.pointer_mode = false;
    u.caption.clear();
}
pub fn pause(u: &mut SessionUi) {
    match u.screen {
        Screen::Playing => u.screen = Screen::Pause,
        Screen::Pause => u.screen = Screen::Playing,
        Screen::Settings | Screen::Help | Screen::Credits => u.screen = u.return_to,
        _ => {}
    }
}
fn update_clock(time: Res<Time>, mut u: ResMut<SessionUi>) {
    u.elapsed += time.delta_secs();
    u.notice_time = (u.notice_time - time.delta_secs()).max(0.0);
    if !u.paused() {
        u.score_time = (u.score_time - time.delta_secs()).max(0.0)
    }
}
pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    pads: Query<&Gamepad>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut u: ResMut<SessionUi>,
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
        pause(&mut u)
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
    if u.screen != Screen::Playing {
        pending.0 = sim::Input::default();
        return;
    }
    if motion.delta.length() > 1.0 {
        u.pointer_mode = true
    }
    if movement.length() > 0.1 {
        u.pointer_mode = false
    }
    pending.0.movement = sim::v(movement.x, movement.y);
    // Pointer target is projected in tick, after any fixed-step movement.
    if u.pointer_mode
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
    mut w: ResMut<Simulation>,
    mut u: ResMut<SessionUi>,
    mut pending: ResMut<PendingInput>,
    mut events: ResMut<FrameEvents>,
    mut prefs: ResMut<Preferences>,
    options: Res<Options>,
    mut capture_action: Local<bool>,
    mut previous: ResMut<crate::presentation::Previous>,
) {
    if !u.loaded
        || u.paused()
        || options.capture.as_deref().is_some_and(|k| {
            k.starts_with("graphics-") || (k.starts_with("pose-") && k != "pose-idle") || k == "rig"
        })
    {
        return;
    }
    let mut input = pending.0;
    if let Some(kind) = &options.capture {
        let due = match kind.as_str() {
            "throw" => 3.25,
            "wave" => 3.5,
            _ => 1.0,
        };
        if !*capture_action && w.0.session.time >= due {
            *capture_action = true;
            input.cast = kind == "throw" || kind == "crystal";
            input.toggle_song = kind == "song";
            input.wave = kind == "wave";
        }
    }
    if u.pointer_mode {
        input.movement = (input.movement - w.0.sabrina.motion.pos) * 0.5;
    }
    if options.capture.as_deref() == Some("pose-idle") {
        input.movement = sim::v(0.0, 0.0);
    }
    pending.0.cast = false;
    pending.0.wave = false;
    pending.0.toggle_song = false;
    w.0.settings.reduced_motion = prefs.reduced_motion;
    previous.0 = Some(w.0.clone());
    let e = sim::step(&mut w.0, 1.0 / 60.0, input);
    if e.visit_ended {
        u.score_time = 10.0;
        prefs.best = prefs.best.max(w.0.points.best_score);
        if options.capture.is_none() && !options.verify {
            prefs.save()
        }
    }
    if e.game_over {
        u.screen = Screen::GameOver
    }
    events.0.push(e);
}
