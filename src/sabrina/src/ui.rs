// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Localized screens, HUD updates and shared pointer/keyboard/controller navigation.
use crate::{
    app::{self, Options, PendingInput, Screen, SessionUi, Simulation},
    audio::AudioRequests,
    controls::{Control, Direction, Hints, MenuNavigation, Style},
    settings::{Preferences, TailColor},
};
use bevy::{
    prelude::*,
    render::render_resource::AsBindGroup,
    render::renderer::RenderAdapterInfo,
    shader::ShaderRef,
    sprite::{BorderRect, TextureSlicer},
    ui::widget::NodeImageMode,
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};
use std::{collections::HashMap, sync::OnceLock};

// Layout and hit testing use the same centered logical canvas at every resolution.
pub(crate) const CANVAS_WIDTH: f32 = 1600.0;
pub(crate) const CANVAS_HEIGHT: f32 = 900.0;
pub(crate) const HUD_BACKGROUND_ALPHA: f32 = 0.28;
pub(crate) const HUD_METER_ALPHA: f32 = 0.65;

fn table() -> &'static HashMap<String, [String; 3]> {
    static TRANSLATIONS: OnceLock<HashMap<String, [String; 3]>> = OnceLock::new();
    TRANSLATIONS.get_or_init(|| serde_json::from_str(include_str!("translations.json")).unwrap())
}

pub fn tr(key: &str, lang: usize) -> String {
    table()
        .get(key)
        .map(|v| v[lang % 3].clone())
        .unwrap_or_else(|| key.into())
}

#[derive(Component)]
struct UiRoot;

#[derive(Component)]
pub(crate) struct FpsCounter;

#[derive(Component)]
struct LoadingStats;

#[derive(Component)]
struct LoadingPrompt;

const LOADING_ART_WIDTH: f32 = 1347.0;
const LOADING_ART_HEIGHT: f32 = 768.0;

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn contains(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.w
            && point.y >= self.y
            && point.y <= self.y + self.h
    }

    fn node(self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            left: px(self.x),
            top: px(self.y),
            width: px(self.w),
            height: px(self.h),
            ..default()
        }
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

fn loading_image_rect(viewport_width: f32, viewport_height: f32) -> Rect {
    let aspect = LOADING_ART_WIDTH / LOADING_ART_HEIGHT;
    let height = viewport_height.max(viewport_width / aspect);
    let width = height * aspect;
    rect(
        (CANVAS_WIDTH - width) / 2.0,
        ((CANVAS_HEIGHT - height) / 2.0).min(
            (CANVAS_HEIGHT + viewport_height) / 2.0 - 24.0 - height * 733.0 / LOADING_ART_HEIGHT,
        ),
        width,
        height,
    )
}

fn on_loading_image(image: Rect, area: Rect) -> Rect {
    let scale = image.w / LOADING_ART_WIDTH;
    rect(
        image.x + area.x * scale,
        image.y + area.y * scale,
        area.w * scale,
        area.h * scale,
    )
}

fn loading_overlay(image: Rect) -> (Rect, Rect) {
    (
        on_loading_image(image, rect(423.5, 660.0, 500.0, 30.0)),
        on_loading_image(image, rect(493.5, 700.0, 360.0, 14.0)),
    )
}

fn loading_ready_overlay(image: Rect) -> Rect {
    let (status, bar) = loading_overlay(image);
    let scale = image.w / LOADING_ART_WIDTH;
    rect(
        status.x - 12.0 * scale,
        status.y - 10.0 * scale,
        status.w + 24.0 * scale,
        bar.y + bar.h - status.y + 20.0 * scale,
    )
}

// The open water between the turtle and crystal, kept inside narrow windows.
fn loading_title_rect(image: Rect, viewport_width: f32, viewport_height: f32) -> Rect {
    let mut title = on_loading_image(image, rect(530.0, 8.0, 470.0, 470.0 / 3.0));
    let left = (CANVAS_WIDTH - viewport_width) / 2.0 + 16.0;
    let available = viewport_width - 32.0;
    if title.w > available {
        title.h *= available / title.w;
        title.w = available;
    }
    title.x = title.x.clamp(left, left + available - title.w);
    title.y = title.y.max((CANVAS_HEIGHT - viewport_height) / 2.0 + 8.0);
    title
}

fn loading_prompt_alpha(seconds: f32) -> f32 {
    0.62 + 0.38 * (0.5 + 0.5 * (seconds * 2.4).sin())
}

fn input_backend(windows: bool, wgi: bool) -> &'static str {
    match (windows, wgi) {
        (true, false) => "XInput (Xbox / Steam Input)",
        (true, true) => "Windows Gaming Input / GilRs",
        (false, _) => "GilRs",
    }
}

fn title_path(language: usize) -> &'static str {
    ["ui/title_eng.png", "ui/title_hr.png", "ui/title_it.png"][language % 3]
}

#[derive(Component, Clone, Copy)]
enum Action {
    Play,
    Aquarium,
    Pause,
    Home,
    Settings,
    Credits,
    Help,
    Language,
    SelectLanguage(usize),
    Quit,
    Fullscreen,
    Vsync,
    Fps,
    Quality,
    Motion,
    Rumble,
    TailColor,
    SelectTail(TailColor),
    Gentle,
    Visitors,
    Done,
    Crystal,
    Song,
    Wave,
    Original,
    Music,
    Effects,
    Voice,
    Rest,
    ViewRelease,
    SkipUpdate,
}

#[derive(Component)]
struct Hit {
    rect: Rect,
    action: Action,
    index: usize,
    slider: bool,
}

#[derive(Component)]
struct ButtonLabel {
    index: usize,
    primary: bool,
    action: Action,
}

#[derive(Component)]
struct Dynamic(&'static str);

#[derive(Component)]
struct Meter(&'static str, f32);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Ability {
    Crystal,
    Song,
    Wave,
}

impl Ability {
    fn state(self, world: &reef_sim::World) -> Vec4 {
        let sabrina = &world.sabrina;
        let charging = world.crystal.magic.state == reef_sim::Spell::Charging;
        let chaining = reef_sim::crystal_chain_target(world).is_some();
        let (progress, blocked, active) = match self {
            Self::Crystal => (
                if chaining {
                    1.0 - world.crystal.magic.time / reef_sim::CRYSTAL_CHAIN_SECONDS
                } else {
                    1.0 - sabrina.cooldown / reef_sim::recharge_duration(world)
                },
                world.crystal.magic.state != reef_sim::Spell::Resting && !chaining,
                charging || chaining,
            ),
            Self::Song => (
                sabrina.song.energy / 100.0,
                sabrina.song.locked,
                sabrina.song.active,
            ),
            Self::Wave => (
                if sabrina.wave.charging {
                    sabrina.wave.charge_time / reef_sim::WAVE_CHARGE_SECONDS
                } else {
                    1.0 - sabrina.wave.cooldown / reef_sim::wave_recharge_duration(world)
                },
                charging,
                sabrina.wave.active || sabrina.wave.charging,
            ),
        };
        Vec4::new(
            progress.clamp(0.0, 1.0),
            f32::from(blocked),
            f32::from(active),
            0.0,
        )
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct AbilityIcon {
    #[uniform(0)]
    state: Vec4,
    #[texture(1)]
    #[sampler(2)]
    image: Handle<Image>,
}

impl UiMaterial for AbilityIcon {
    fn fragment_shader() -> ShaderRef {
        "shaders/ability_icon.wesl".into()
    }
}

#[derive(Resource, Default)]
struct Focus {
    index: usize,
    key: String,
    last_screen: Option<Screen>,
    count: usize,
    navigation: MenuNavigation,
    controller: Option<Entity>,
}

pub struct ReefUiPlugin;

impl Plugin for ReefUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Focus>()
            .add_plugins(UiMaterialPlugin::<AbilityIcon>::default())
            .init_resource::<Hints>()
            .add_systems(Startup, setup_fps)
            .add_systems(Update, update_fps)
            .add_systems(Update, update_abilities.after(draw))
            .add_systems(
                Update,
                (update_loading_stats, pulse_loading_prompt).after(draw),
            )
            .add_systems(
                PreUpdate,
                crate::controls::update
                    .after(bevy::input::InputSystems)
                    .before(crate::app::input),
            )
            .add_systems(
                PreUpdate,
                continue_from_loading
                    .after(crate::controls::update)
                    .before(crate::app::input),
            )
            .add_systems(PreUpdate, actions.after(crate::app::input))
            .add_systems(
                Update,
                (draw, update).chain().before(crate::presentation::prepare),
            );
    }
}

fn update_abilities(
    simulation: Res<Simulation>,
    icons: Query<(&Ability, &MaterialNode<AbilityIcon>)>,
    mut materials: ResMut<Assets<AbilityIcon>>,
) {
    for (ability, handle) in &icons {
        let state = ability.state(&simulation.0);
        let Some(current) = materials.get(&handle.0) else {
            error!("Ability icon has no material: {ability:?}");
            continue;
        };
        if current.state != state {
            materials.get_mut(&handle.0).unwrap().state = state;
        }
    }
}

fn setup_fps(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        FpsCounter,
        Text::new("FPS: --"),
        TextShadow {
            offset: Vec2::new(1.0, 2.0),
            color: Color::BLACK.with_alpha(0.85),
        },
        TextFont {
            font: server.load("fonts/DejaVuSans.ttf").into(),
            font_size: 18.0.into(),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::BLACK.with_alpha(0.55)),
        Node {
            position_type: PositionType::Absolute,
            right: px(12.0),
            top: px(112.0),
            padding: UiRect::all(px(6.0)),
            ..default()
        },
        GlobalZIndex(100),
        Visibility::Hidden,
    ));
}

fn update_fps(
    preferences: Res<Preferences>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    time: Res<Time<Real>>,
    mut elapsed: Local<f32>,
    mut counter: Single<(&mut Text, &mut Visibility), With<FpsCounter>>,
) {
    let (text, visibility) = &mut *counter;
    **visibility = if preferences.show_fps {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if !preferences.show_fps {
        *elapsed = 0.25;
        return;
    }
    *elapsed += time.delta_secs();
    if *elapsed < 0.25 {
        return;
    }
    *elapsed = 0.0;
    if let Some(fps) = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diagnostic| diagnostic.smoothed())
    {
        text.0 = format!("FPS: {fps:.0}");
    }
}

struct Builder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    server: &'a AssetServer,
    root: Entity,
    lang: usize,
    index: usize,
    font: Handle<Font>,
    background_alpha: f32,
    controls: Style,
}

impl Builder<'_, '_, '_> {
    fn image(&mut self, r: Rect, name: &str) -> Entity {
        let e = self
            .commands
            .spawn((
                ImageNode {
                    image: self.server.load(format!("ui/reef_png/{name}.png")),
                    color: Color::WHITE.with_alpha(
                        if matches!(name, "panel" | "compact") || name.starts_with("button") {
                            self.background_alpha
                        } else {
                            1.0
                        },
                    ),
                    image_mode: if name == "panel"
                        || name == "compact"
                        || name.starts_with("button")
                    {
                        NodeImageMode::Sliced(TextureSlicer {
                            border: BorderRect::all(if name.starts_with("button") {
                                22.0
                            } else {
                                26.0
                            }),
                            ..default()
                        })
                    } else {
                        NodeImageMode::Stretch
                    },
                    ..default()
                },
                r.node(),
            ))
            .id();
        self.commands.entity(self.root).add_child(e);
        e
    }

    fn label(
        &mut self,
        r: Rect,
        size: f32,
        text: impl Into<String>,
        dynamic: Option<&'static str>,
    ) -> Entity {
        let e = self
            .commands
            .spawn((
                Text::new(text),
                TextFont {
                    font: self.font.clone().into(),
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(Color::srgb(0.87, 0.97, 1.0)),
                r.node(),
            ))
            .id();
        if let Some(key) = dynamic {
            self.commands.entity(e).insert(Dynamic(key));
        }
        if self.background_alpha < 1.0 {
            self.commands.entity(e).insert(TextShadow {
                offset: Vec2::new(1.0, 2.0),
                color: Color::BLACK.with_alpha(0.85),
            });
        }
        self.commands.entity(self.root).add_child(e);
        e
    }

    fn panel(&mut self, r: Rect) {
        self.image(r, if r.h < 150.0 { "compact" } else { "panel" });
        if r.h > 200.0 {
            self.image(
                rect(r.x + r.w / 2.0 - 42.0, r.y - 38.0, 84.0, 84.0),
                "shell",
            );
            self.image(rect(r.x - 36.0, r.y + r.h - 115.0, 88.0, 126.0), "coral");
            self.image(rect(r.x + r.w - 32.0, r.y + r.h - 62.0, 64.0, 64.0), "star");
        }
    }

    fn button(&mut self, r: Rect, text: &str, action: Action, primary: bool) {
        let size = if self.background_alpha < 1.0 {
            17.0
        } else {
            20.0
        };
        let e = self.image(r, if primary { "button" } else { "button_teal" });
        self.commands.entity(e).insert(Hit {
            rect: r,
            action,
            index: self.index,
            slider: false,
        });
        self.index += 1;
        let label = self.label(
            rect(r.x + 12.0, r.y + 12.0, r.w - 24.0, r.h - 16.0),
            size,
            tr(text, self.lang),
            None,
        );
        self.commands.entity(label).insert((
            TextLayout::justify(Justify::Center),
            ButtonLabel {
                index: self.index - 1,
                primary,
                action,
            },
        ));
        if primary {
            self.commands
                .entity(label)
                .insert(TextColor(Color::srgb(0.20, 0.17, 0.18)));
        }
        if self.background_alpha < 1.0 {
            let control = match action {
                Action::Song => Some(Control::Song),
                Action::Wave => Some(Control::Wave),
                Action::Pause => Some(Control::Pause),
                _ => None,
            };
            if let Some(control) = control
                && self.controls != Style::Generic
            {
                self.binding_span(label, control, size, true);
            }
        }
    }

    fn binding_span(&mut self, label: Entity, control: Control, size: f32, compact: bool) {
        let mut binding = self.controls.binding(control);
        if compact && self.controls == Style::Keyboard && matches!(control, Control::Wave) {
            binding.label = "F";
        }
        self.commands.entity(label).with_children(|parent| {
            parent.spawn((
                TextSpan::new(format!(" · {}", tr(binding.label, self.lang))),
                TextColor(binding.color),
                TextFont {
                    font: self.font.clone().into(),
                    font_size: FontSize::Px(size),
                    ..default()
                },
            ));
        });
    }

    fn control_hints(&mut self, r: Rect, size: f32) {
        for (index, (title, control)) in [
            ("Swim", Control::Swim),
            ("Crystal", Control::Crystal),
            ("Song", Control::Song),
            ("Wave", Control::Wave),
            ("Pause", Control::Pause),
        ]
        .into_iter()
        .enumerate()
        {
            let width = r.w / 5.0;
            let label = self.label(
                rect(r.x + index as f32 * width, r.y, width, r.h),
                size,
                tr(title, self.lang),
                None,
            );
            self.binding_span(label, control, size, false);
        }
    }

    fn ability(
        &mut self,
        x: f32,
        ability: Ability,
        materials: &mut Assets<AbilityIcon>,
        clickable: bool,
    ) {
        let (name, control, action, image) = match ability {
            Ability::Crystal => ("Crystal", Control::Crystal, Action::Crystal, "crystal"),
            Ability::Song => ("Song", Control::Song, Action::Song, "song"),
            Ability::Wave => ("Wave", Control::Wave, Action::Wave, "wave"),
        };
        let area = rect(x, 754.0, 92.0, 92.0);
        let frame = self
            .commands
            .spawn((
                area.node(),
                BackgroundColor(Color::srgba(0.015, 0.07, 0.11, 0.82)),
                BorderColor::all(Color::srgba(0.48, 0.88, 0.92, 0.65)),
            ))
            .id();
        self.commands.entity(frame).insert(Node {
            border: UiRect::all(px(2.0)),
            border_radius: BorderRadius::all(px(10.0)),
            ..area.node()
        });
        self.commands.entity(self.root).add_child(frame);
        if clickable {
            self.commands.entity(frame).insert(Hit {
                rect: area,
                action,
                index: self.index,
                slider: false,
            });
            self.index += 1;
        }
        let icon = self
            .commands
            .spawn((
                rect(x + 4.0, 758.0, 84.0, 84.0).node(),
                ability,
                MaterialNode(materials.add(AbilityIcon {
                    state: Vec4::X,
                    image: self.server.load(format!("ui/ability_{image}.png")),
                })),
            ))
            .id();
        self.commands.entity(self.root).add_child(icon);
        let label = self.label(
            rect(x - 20.0, 728.0, 132.0, 24.0),
            17.0,
            tr(name, self.lang),
            None,
        );
        self.commands
            .entity(label)
            .insert(TextLayout::justify(Justify::Center));
        let binding = self.controls.binding(control);
        let label = self.label(rect(x - 26.0, 850.0, 144.0, 26.0), 17.0, "", None);
        self.commands
            .entity(label)
            .insert(TextLayout::justify(Justify::Center))
            .with_children(|parent| {
                parent.spawn((
                    TextSpan::new(tr(binding.label, self.lang)),
                    TextColor(binding.color),
                    TextFont {
                        font: self.font.clone().into(),
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                ));
            });
        let label = self.label(rect(x - 20.0, 791.0, 132.0, 24.0), 18.0, "", Some(image));
        self.commands
            .entity(label)
            .insert(TextLayout::justify(Justify::Center));
    }

    fn meter(&mut self, r: Rect, color: Color, key: &'static str) {
        let e = self
            .commands
            .spawn((
                r.node(),
                BackgroundColor(Color::srgba(0.02, 0.13, 0.20, self.background_alpha)),
            ))
            .id();
        self.commands.entity(self.root).add_child(e);
        let e = self
            .commands
            .spawn((
                r.node(),
                BackgroundColor(color.with_alpha(if self.background_alpha < 1.0 {
                    HUD_METER_ALPHA
                } else {
                    1.0
                })),
                Meter(key, r.w),
            ))
            .id();
        self.commands.entity(self.root).add_child(e);
    }

    fn slider(&mut self, r: Rect, amount: f32, action: Action) {
        let e = self
            .commands
            .spawn((
                r.node(),
                BackgroundColor(Color::srgb(0.05, 0.25, 0.30)),
                Hit {
                    rect: r,
                    action,
                    index: self.index,
                    slider: true,
                },
            ))
            .id();
        self.index += 1;
        self.commands.entity(self.root).add_child(e);
        let f = self
            .commands
            .spawn((
                rect(r.x, r.y, r.w * amount, r.h).node(),
                BackgroundColor(Color::srgb(0.25, 0.87, 0.87)),
            ))
            .id();
        self.commands.entity(self.root).add_child(f);
    }
}

fn draw(
    mut commands: Commands,
    server: Res<AssetServer>,
    ui: Res<SessionUi>,
    preferences: Res<Preferences>,
    roots: Query<Entity, With<UiRoot>>,
    mut focus: ResMut<Focus>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
    options: Res<Options>,
    hints: Res<Hints>,
    adapter: Res<RenderAdapterInfo>,
    mut ability_materials: ResMut<Assets<AbilityIcon>>,
    update_check: Res<crate::updates::UpdateCheck>,
) {
    let zoom = (window.width() / CANVAS_WIDTH).min(window.height() / CANVAS_HEIGHT);
    scale.0 = zoom;
    // Rebuild only when layout or settings change; HUD values update separately.
    let mut key = format!(
        "{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{:?}:{}:{}:{}:{:?}",
        ui.screen,
        preferences.language,
        preferences.quality,
        preferences.fullscreen,
        preferences.vsync,
        preferences.reduced_motion,
        preferences.gentle,
        preferences.visitors,
        preferences.rest,
        preferences.music,
        preferences.effects,
        preferences.voice,
        preferences.best,
        window.width(),
        window.height(),
        hints.style,
        preferences.show_fps,
        preferences.rumble,
        ui.loaded && options.capture.is_none(),
        preferences.tail_color
    );
    key.push_str(&format!(
        "{:?}:{}",
        update_check.status, update_check.browser_error
    ));
    if key == focus.key {
        return;
    }
    focus.key = key;
    if focus.last_screen != Some(ui.screen) {
        focus.index = 0;
        focus.last_screen = Some(ui.screen);
        focus.navigation.reset();
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            UiRoot,
            Node {
                width: px(CANVAS_WIDTH),
                height: px(CANVAS_HEIGHT),
                position_type: PositionType::Absolute,
                left: px((window.width() / zoom - CANVAS_WIDTH) / 2.0),
                top: px((window.height() / zoom - CANVAS_HEIGHT) / 2.0),
                ..default()
            },
        ))
        .id();
    let mut builder = Builder {
        commands: &mut commands,
        server: &server,
        root,
        lang: preferences.language,
        index: 0,
        font: server.load("fonts/DejaVuSans.ttf"),
        background_alpha: 1.0,
        controls: hints.style,
    };
    if options
        .capture
        .as_deref()
        .is_some_and(|k| k.starts_with("graphics-") || k.starts_with("pose-") || k == "rig")
    {
        focus.count = 0;
        return;
    }
    if ui.screen == Screen::Loading {
        let viewport_width = window.width() / zoom;
        let viewport_height = window.height() / zoom;
        let image = loading_image_rect(viewport_width, viewport_height);
        let e = builder
            .commands
            .spawn((
                ImageNode {
                    image: server.load("ui/loading.jpg"),
                    ..default()
                },
                image.node(),
            ))
            .id();
        builder.commands.entity(root).add_child(e);
        let title = builder
            .commands
            .spawn((
                ImageNode::new(server.load(title_path(preferences.language))),
                loading_title_rect(image, viewport_width, viewport_height).node(),
            ))
            .id();
        builder.commands.entity(root).add_child(title);
        let scale = image.w / LOADING_ART_WIDTH;
        if update_check.status != crate::updates::Status::Continue {
            builder.panel(rect(370.0, 510.0, 860.0, 265.0));
            let (title, message) = match &update_check.status {
                crate::updates::Status::Available { tag, .. } => (
                    tr("A new underwater adventure awaits!", preferences.language),
                    format!(
                        "{} {}\n{}",
                        tr("New version available:", preferences.language),
                        tag,
                        tr(
                            if update_check.browser_error {
                                "Could not open your browser. Try again or skip for now."
                            } else {
                                "View the GitHub release to download it, or keep playing this version."
                            },
                            preferences.language
                        )
                    ),
                ),
                _ => (
                    tr("Checking for updates…", preferences.language),
                    tr("Just a moment before we dive in.", preferences.language),
                ),
            };
            builder.label(rect(410.0, 538.0, 780.0, 42.0), 29.0, title, None);
            builder.label(rect(410.0, 590.0, 780.0, 75.0), 21.0, message, None);
            if matches!(
                update_check.status,
                crate::updates::Status::Available { .. }
            ) {
                builder.button(
                    rect(410.0, 687.0, 370.0, 58.0),
                    "View release and exit",
                    Action::ViewRelease,
                    true,
                );
                builder.button(
                    rect(810.0, 687.0, 370.0, 58.0),
                    "Skip for now",
                    Action::SkipUpdate,
                    false,
                );
            }
            focus.count = builder.index;
            return;
        }
        let halo = loading_ready_overlay(image);
        let halo = builder
            .commands
            .spawn((
                rect(
                    halo.x - halo.w * 0.1,
                    halo.y - halo.h * 0.3,
                    halo.w * 1.2,
                    halo.h * 1.6,
                )
                .node(),
                // A soft vignette keeps text readable on bright sand without a button shape.
                BackgroundGradient::from(RadialGradient::new(
                    UiPosition::CENTER,
                    RadialGradientShape::FarthestCorner,
                    vec![
                        ColorStop::percent(Color::srgba(0.0, 0.04, 0.09, 0.78), 0.0),
                        ColorStop::percent(Color::srgba(0.0, 0.04, 0.09, 0.58), 42.0),
                        ColorStop::percent(Color::srgba(0.0, 0.04, 0.09, 0.0), 71.0),
                    ],
                )),
            ))
            .id();
        builder.commands.entity(root).add_child(halo);
        let shadow = TextShadow {
            offset: Vec2::splat(2.0 / zoom),
            color: Color::BLACK.with_alpha(0.9),
        };
        if ui.loaded && options.capture.is_none() {
            let ready = loading_ready_overlay(image);
            let prompt = builder.label(
                rect(ready.x, ready.y + ready.h * 0.25, ready.w, ready.h * 0.6),
                27.0 * scale,
                tr("Press any key to continue", preferences.language),
                None,
            );
            builder.commands.entity(prompt).insert((
                LoadingPrompt,
                TextLayout::justify(Justify::Center),
                TextColor(Color::srgb(1.0, 0.95, 0.80)),
                shadow,
            ));
        } else {
            let (status_area, bar) = loading_overlay(image);
            let status = builder.label(status_area, 21.0 * scale, "", Some("loading"));
            builder.commands.entity(status).insert((
                TextLayout::justify(Justify::Center),
                TextColor(Color::srgb(1.0, 0.95, 0.82)),
                shadow,
            ));
            let border = (1.5 * scale).max(1.0);
            let inset = border + (1.5 * scale).max(1.0);
            let frame = builder
                .commands
                .spawn((
                    Node {
                        border: UiRect::all(px(border)),
                        border_radius: BorderRadius::MAX,
                        ..bar.node()
                    },
                    BackgroundColor(Color::srgba(0.01, 0.10, 0.18, 0.72)),
                    BorderColor::all(Color::srgb(0.95, 0.78, 0.45)),
                    BoxShadow::new(
                        Color::srgba(0.0, 0.0, 0.0, 0.55),
                        px(0.0),
                        px(2.0 * scale),
                        px(0.0),
                        px(8.0 * scale),
                    ),
                ))
                .id();
            builder.commands.entity(root).add_child(frame);
            let fill = builder
                .commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(inset - border),
                        top: px(inset - border),
                        width: px(0.0),
                        height: px(bar.h - inset * 2.0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.48, 0.97, 0.93)),
                    BoxShadow::new(
                        Color::srgba(0.30, 0.95, 1.0, 0.75),
                        px(0.0),
                        px(0.0),
                        px(0.0),
                        px(7.0 * scale),
                    ),
                    Meter("loading", bar.w - inset * 2.0),
                ))
                .id();
            builder.commands.entity(frame).add_child(fill);
        }
        let avoid = loading_ready_overlay(image);
        draw_loading_stats(&mut builder, viewport_width, viewport_height, zoom, avoid);
        focus.count = 0;
        return;
    }
    if ui.started && !matches!(ui.screen, Screen::Victory | Screen::GameOver) {
        draw_hud(&mut builder, ui.screen, &mut ability_materials);
    }
    match ui.screen {
        Screen::Menu => draw_menu(&mut builder, &preferences, renderer_label(&adapter)),
        Screen::Pause => draw_pause(&mut builder),
        Screen::Settings => draw_settings(&mut builder, &preferences),
        Screen::TailColor => draw_tail_color(&mut builder, &preferences),
        Screen::Credits | Screen::Help => draw_help_or_credits(&mut builder, ui.screen),
        Screen::GameOver => draw_game_over(&mut builder),
        Screen::Victory => draw_victory(&mut builder),
        Screen::Language => draw_language(&mut builder),
        _ => {}
    }
    focus.count = builder.index;
    focus.index = focus.index.min(focus.count.saturating_sub(1));
}

fn draw_loading_stats(
    builder: &mut Builder<'_, '_, '_>,
    viewport_width: f32,
    viewport_height: f32,
    zoom: f32,
    avoid: Rect,
) {
    let stats_scale = (0.8 / zoom).max(1.0);
    let margin = 20.0 * stats_scale;
    let x = (CANVAS_WIDTH - viewport_width) / 2.0 + margin;
    let stats_width = (430.0 * stats_scale).min(viewport_width - 2.0 * margin);
    // Diagnostics sit low-left so the localized title owns the open water.
    let mut bottom = (CANVAS_HEIGHT + viewport_height) / 2.0 - margin;
    if x + stats_width > avoid.x - margin {
        bottom = bottom.min(avoid.y - margin);
    }
    let stats = builder.label(rect(x, 0.0, stats_width, 0.0), 14.0 * stats_scale, "", None);
    builder.commands.entity(stats).insert((
        LoadingStats,
        Node {
            position_type: PositionType::Absolute,
            left: px(x),
            bottom: px(CANVAS_HEIGHT - bottom),
            max_width: px(stats_width),
            padding: UiRect::axes(px(12.0 * stats_scale), px(8.0 * stats_scale)),
            border_radius: BorderRadius::all(px(10.0 * stats_scale)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.01, 0.07, 0.13, 0.62)),
        TextColor(Color::srgb(0.90, 0.96, 1.0).with_alpha(0.86)),
        TextShadow {
            offset: Vec2::splat(1.0 / zoom),
            color: Color::BLACK,
        },
        TextLayout::justify(Justify::Left),
    ));
}

fn continue_from_loading(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    options: Res<Options>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut ui: ResMut<SessionUi>,
) {
    if options.capture.is_some()
        || !ui.loaded
        || ui.screen != Screen::Loading
        || (!window.focused && !options.verify)
        || !any_loading_input_pressed(
            &keys,
            &mouse,
            pads.iter()
                .any(|pad| pad.get_just_pressed().next().is_some()),
        )
    {
        return;
    }
    ui.screen = Screen::Menu;
}

fn pulse_loading_prompt(time: Res<Time>, mut prompts: Query<&mut TextColor, With<LoadingPrompt>>) {
    let alpha = loading_prompt_alpha(time.elapsed_secs());
    for mut color in &mut prompts {
        color.0.set_alpha(alpha);
    }
}

fn any_loading_input_pressed(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    gamepad_pressed: bool,
) -> bool {
    keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || gamepad_pressed
}

fn update_loading_stats(
    mut texts: Query<&mut Text, With<LoadingStats>>,
    window: Single<&Window, With<PrimaryWindow>>,
    adapter: Res<RenderAdapterInfo>,
    options: Res<Options>,
    hints: Res<Hints>,
    pads: Query<Option<&Name>, With<Gamepad>>,
    loading: Res<crate::loading::Loading>,
    preferences: Res<Preferences>,
) {
    if texts.is_empty() {
        return;
    }
    let lang = preferences.language;
    let mut controllers: Vec<_> = pads
        .iter()
        .map(|name| {
            name.map_or_else(
                || tr("Unnamed controller", lang),
                |name| name.as_str().to_owned(),
            )
        })
        .collect();
    controllers.sort();
    let connected = if controllers.is_empty() {
        tr("none", lang)
    } else {
        controllers.join(", ")
    };
    let active = match hints.style {
        Style::Keyboard => "Keyboard / mouse",
        Style::Xbox => "Xbox",
        Style::PlayStation => "PlayStation",
        Style::Generic => "Generic gamepad",
    };
    let window_system = if cfg!(target_os = "linux") {
        format!(
            "{}: {}\n",
            tr("Window system", lang),
            options.window_system_label()
        )
    } else {
        String::new()
    };
    let value = format!(
        "{} ({}) · {} × {} px\n{}{}: {}\n{} · {} draw\nGPU: {}\n{}: {} {}\n{}: {} + {}\n{}: {} · {} ({}): {}\n{}: {}/{} · {}: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        window.physical_width(),
        window.physical_height(),
        window_system,
        tr("Build", lang),
        crate::build_info::menu_label(),
        renderer_label(&adapter),
        if options.direct_draw {
            "direct"
        } else {
            "indirect"
        },
        adapter.name,
        tr("Driver", lang),
        adapter.driver,
        adapter.driver_info,
        tr("Input", lang),
        tr("Keyboard / mouse", lang),
        input_backend(cfg!(target_os = "windows"), options.wgi),
        tr("Active hints", lang),
        tr(active, lang),
        tr("Controllers", lang),
        controllers.len(),
        connected,
        tr("Assets", lang),
        loading.assets_loaded,
        loading.assets_total,
        tr("Pipelines", lang),
        loading.pipelines_ready,
        loading.pipelines_total,
    );
    for mut text in &mut texts {
        if text.0 != value {
            text.0.clone_from(&value);
        }
    }
}

fn draw_hud(
    builder: &mut Builder<'_, '_, '_>,
    screen: Screen,
    materials: &mut Assets<AbilityIcon>,
) {
    builder.background_alpha = HUD_BACKGROUND_ALPHA;
    builder.panel(rect(18.0, 18.0, 330.0, 112.0));
    builder.label(rect(36.0, 30.0, 294.0, 26.0), 20.0, "", Some("phase"));
    builder.label(rect(36.0, 59.0, 294.0, 23.0), 17.0, "", Some("stats"));
    builder.image(rect(36.0, 88.0, 22.0, 22.0), "heart");
    builder.label(rect(65.0, 89.0, 90.0, 22.0), 16.0, "", Some("health"));
    builder.meter(
        rect(163.0, 95.0, 167.0, 9.0),
        Color::srgb(1.0, 0.4, 0.48),
        "health",
    );
    for (x, ability) in [
        (610.0, Ability::Wave),
        (754.0, Ability::Song),
        (898.0, Ability::Crystal),
    ] {
        builder.ability(x, ability, materials, screen == Screen::Playing);
    }
    let swim = builder.label(
        rect(40.0, 864.0, 450.0, 32.0),
        17.0,
        tr("Swim", builder.lang),
        None,
    );
    builder.binding_span(swim, Control::Swim, 17.0, false);
    for (index, key) in [
        "reward-crystal",
        "reward-wave",
        "reward-rescue",
        "reward-retreat",
        "reward-survivors",
    ]
    .into_iter()
    .enumerate()
    {
        builder.label(
            rect(40.0, 690.0 + index as f32 * 30.0, 450.0, 28.0),
            19.0,
            "",
            Some(key),
        );
    }
    builder.label(rect(500.0, 635.0, 600.0, 48.0), 23.0, "", Some("notice"));
    builder.label(rect(500.0, 683.0, 600.0, 36.0), 19.0, "", Some("caption"));
    if screen == Screen::Playing {
        builder.button(rect(1415.0, 24.0, 160.0, 50.0), "II", Action::Pause, false);
    }
    builder.background_alpha = 1.0;
}

fn draw_menu(builder: &mut Builder<'_, '_, '_>, preferences: &Preferences, renderer: String) {
    let lang = builder.lang;
    builder.panel(rect(64.0, 65.0, 535.0, 775.0));
    builder.label(
        rect(100.0, 108.0, 450.0, 30.0),
        17.0,
        tr("A Game by Luka & Sabrina Dežulović", lang),
        None,
    );
    let title = builder
        .commands
        .spawn((
            ImageNode::new(builder.server.load(title_path(lang))),
            rect(76.0, 145.0, 510.0, 170.0).node(),
        ))
        .id();
    builder.commands.entity(builder.root).add_child(title);
    builder.label(
        rect(100.0, 340.0, 450.0, 76.0),
        21.0,
        tr(
            "A living sea. A mischievous jellyfish.\nAnd a little mermaid with a magic crystal.",
            lang,
        ),
        None,
    );
    for (r, key, action, primary) in [
        (
            rect(100.0, 430.0, 460.0, 58.0),
            "Let's play",
            Action::Play,
            true,
        ),
        (
            rect(100.0, 510.0, 220.0, 52.0),
            "Credits",
            Action::Credits,
            false,
        ),
        (
            rect(340.0, 510.0, 220.0, 52.0),
            "Settings",
            Action::Settings,
            false,
        ),
        (
            rect(100.0, 584.0, 460.0, 52.0),
            "Quiet aquarium · no jellyfish",
            Action::Aquarium,
            false,
        ),
        (
            rect(100.0, 658.0, 220.0, 52.0),
            "How to play",
            Action::Help,
            false,
        ),
        (
            rect(340.0, 658.0, 220.0, 52.0),
            "Language…",
            Action::Language,
            false,
        ),
        (
            rect(100.0, 732.0, 220.0, 48.0),
            "Goodbye",
            Action::Quit,
            false,
        ),
    ] {
        builder.button(r, key, action, primary);
    }
    builder.label(
        rect(340.0, 732.0, 220.0, 48.0),
        17.0,
        format!("{}{}", tr("Best visit: ", lang), preferences.best),
        None,
    );
    builder.label(
        rect(100.0, 782.0, 270.0, 22.0),
        13.5,
        crate::build_info::menu_label(),
        None,
    );
    let renderer_label = builder.label(rect(375.0, 782.0, 185.0, 22.0), 13.5, renderer, None);
    builder
        .commands
        .entity(renderer_label)
        .insert(TextLayout::justify(Justify::Right));
    builder.label(
        rect(100.0, 806.0, 460.0, 16.0),
        10.0,
        "Copyright © 2026 Luka Dežulović",
        None,
    );
}

fn renderer_label(adapter: &RenderAdapterInfo) -> String {
    let backend = format!("{:?}", adapter.backend);
    let name = match backend.as_str() {
        "Dx12" => "DX12",
        "Gl" => "OpenGL",
        name => name,
    };
    format!("Renderer: {name}")
}

fn draw_pause(builder: &mut Builder<'_, '_, '_>) {
    let lang = builder.lang;
    builder.panel(rect(500.0, 190.0, 600.0, 510.0));
    builder.label(
        rect(550.0, 230.0, 500.0, 65.0),
        40.0,
        tr("A little pause", lang),
        None,
    );
    builder.label(
        rect(550.0, 305.0, 500.0, 45.0),
        21.0,
        tr("Your fish will wait for you.", lang),
        None,
    );
    builder.button(
        rect(550.0, 385.0, 500.0, 60.0),
        "Continue swimming",
        Action::Pause,
        true,
    );
    builder.button(
        rect(550.0, 465.0, 500.0, 60.0),
        "Settings",
        Action::Settings,
        false,
    );
    builder.button(
        rect(550.0, 545.0, 500.0, 60.0),
        "Return to the beginning",
        Action::Home,
        false,
    );
}

fn draw_settings(builder: &mut Builder<'_, '_, '_>, preferences: &Preferences) {
    let lang = builder.lang;
    builder.panel(rect(270.0, 70.0, 1060.0, 760.0));
    builder.label(
        rect(315.0, 103.0, 650.0, 60.0),
        36.0,
        tr("Make yourself at home", lang),
        None,
    );
    builder.button(
        rect(315.0, 190.0, 450.0, 52.0),
        &format!(
            "{}{}",
            tr("Fullscreen: ", lang),
            tr(if preferences.fullscreen { "on" } else { "off" }, lang)
        ),
        Action::Fullscreen,
        false,
    );
    builder.button(
        rect(815.0, 190.0, 465.0, 52.0),
        &format!(
            "{}{}",
            tr("Graphics: ", lang),
            tr(
                ["Light", "Balanced", "High"][preferences.quality as usize],
                lang
            )
        ),
        Action::Quality,
        false,
    );
    builder.button(
        rect(315.0, 260.0, 450.0, 52.0),
        if preferences.reduced_motion {
            "Motion: reduced"
        } else {
            "Motion: natural"
        },
        Action::Motion,
        false,
    );
    builder.button(
        rect(815.0, 260.0, 465.0, 52.0),
        if preferences.gentle {
            "Gentle play"
        } else {
            "Lively play"
        },
        Action::Gentle,
        false,
    );
    for (y, key, value, action) in [
        (345.0, "Music", preferences.music, Action::Music),
        (
            445.0,
            "Magic & sound effects",
            preferences.effects,
            Action::Effects,
        ),
        (545.0, "Original voices", preferences.voice, Action::Voice),
    ] {
        builder.label(rect(315.0, y, 450.0, 35.0), 22.0, tr(key, lang), None);
        builder.slider(rect(315.0, y + 48.0, 400.0, 24.0), value, action);
    }
    builder.label(
        rect(815.0, 345.0, 465.0, 35.0),
        22.0,
        format!("{}{:.0} s", tr("Quiet time: ", lang), preferences.rest),
        None,
    );
    builder.slider(
        rect(815.0, 393.0, 400.0, 24.0),
        (preferences.rest - 25.0) / 125.0,
        Action::Rest,
    );
    builder.button(
        rect(815.0, 465.0, 465.0, 58.0),
        &format!(
            "{}{}{}",
            tr("At most ", lang),
            preferences.visitors,
            tr(" jellyfish", lang)
        ),
        Action::Visitors,
        false,
    );
    builder.label(
        rect(815.0, 535.0, 465.0, 70.0),
        18.0,
        tr("Rounds 1–9 only. Round 10 always has four jellyfish.", lang),
        None,
    );
    builder.button(
        rect(815.0, 625.0, 465.0, 52.0),
        &format!(
            "{} {}",
            if preferences.show_fps { "☑" } else { "☐" },
            tr("Show FPS counter", lang)
        ),
        Action::Fps,
        false,
    );
    builder.button(
        rect(315.0, 625.0, 450.0, 52.0),
        &format!(
            "{}{}",
            tr("V-sync: ", lang),
            tr(if preferences.vsync { "on" } else { "off" }, lang)
        ),
        Action::Vsync,
        false,
    );
    builder.button(
        rect(315.0, 725.0, 450.0, 58.0),
        &format!(
            "{}{}",
            tr("Rumble: ", lang),
            tr(
                ["off", "Gentle", "Normal"][preferences.rumble.min(2) as usize],
                lang
            )
        ),
        Action::Rumble,
        false,
    );
    builder.button(rect(815.0, 725.0, 465.0, 58.0), "Done", Action::Done, true);
    builder.button(
        rect(1015.0, 110.0, 265.0, 52.0),
        "Tail color",
        Action::TailColor,
        false,
    );
}

fn draw_tail_color(builder: &mut Builder<'_, '_, '_>, preferences: &Preferences) {
    let lang = builder.lang;
    builder.panel(rect(370.0, 110.0, 860.0, 680.0));
    builder.label(
        rect(420.0, 145.0, 760.0, 55.0),
        36.0,
        tr("Tail color", lang),
        None,
    );
    builder.label(
        rect(420.0, 215.0, 760.0, 60.0),
        20.0,
        tr(
            "None keeps the original. Magic flows with rainbow colors.",
            lang,
        ),
        None,
    );
    for (index, color) in TailColor::PRESETS.into_iter().enumerate() {
        let x = 420.0 + (index % 2) as f32 * 400.0;
        let y = 300.0 + (index / 2) as f32 * 82.0;
        builder.button(
            rect(x, y, 360.0, 62.0),
            &format!(
                "{} {}",
                if preferences.tail_color == color {
                    "●"
                } else {
                    "○"
                },
                tr(color.label(), lang),
            ),
            Action::SelectTail(color),
            false,
        );
        if let Some([r, g, b]) = color.rgb() {
            let swatch = builder
                .commands
                .spawn((
                    rect(x + 14.0, y + 22.0, 18.0, 18.0).node(),
                    BackgroundColor(Color::srgb_u8(r, g, b)),
                ))
                .id();
            builder.commands.entity(builder.root).add_child(swatch);
        }
    }
    builder.label(
        rect(420.0, 640.0, 760.0, 32.0),
        20.0,
        format!(
            "{} {}",
            tr("Selected:", lang),
            tr(preferences.tail_color.label(), lang)
        ),
        None,
    );
    builder.button(rect(840.0, 675.0, 340.0, 58.0), "Done", Action::Done, true);
}

fn draw_help_or_credits(builder: &mut Builder<'_, '_, '_>, screen: Screen) {
    let lang = builder.lang;
    builder.panel(rect(230.0, 60.0, 1140.0, 780.0));
    let help = screen == Screen::Help;
    builder.label(
        rect(280.0, 93.0, 1040.0, 65.0),
        38.0,
        tr(
            if help {
                "Swim, sing, and look after the fish"
            } else {
                "Credits"
            },
            lang,
        ),
        None,
    );
    let text = if help {
        format!(
            "{}\n\n{}",
            tr(
                if builder.controls == Style::Keyboard {
                    "Keyboard help extras"
                } else {
                    "Controller help extras"
                },
                lang
            ),
            tr("Combat rules", lang)
        )
    } else {
        [
            include_str!("../../../assets/credits/credits_en.txt"),
            include_str!("../../../assets/credits/credits_hr.txt"),
            include_str!("../../../assets/credits/credits_it.txt"),
        ][lang]
            .into()
    };
    if help {
        builder.control_hints(rect(280.0, 173.0, 1040.0, 52.0), 15.0);
    }
    builder.label(
        rect(280.0, if help { 235.0 } else { 180.0 }, 1040.0, 440.0),
        if help { 17.0 } else { 23.0 },
        text,
        None,
    );
    if help {
        builder.label(
            rect(280.0, 680.0, 1040.0, 60.0),
            18.0,
            tr("Final round guide", lang),
            None,
        );
    } else {
        builder.button(
            rect(280.0, 745.0, 640.0, 52.0),
            "Play the original on Scratch ↗",
            Action::Original,
            false,
        );
    }
    builder.button(rect(940.0, 745.0, 380.0, 52.0), "Back", Action::Done, true);
}

fn draw_game_over(builder: &mut Builder<'_, '_, '_>) {
    let lang = builder.lang;
    builder.panel(rect(440.0, 210.0, 720.0, 470.0));
    builder.image(rect(746.0, 240.0, 108.0, 90.0), "heart");
    builder.label(
        rect(485.0, 332.0, 630.0, 62.0),
        38.0,
        tr("Game over", lang),
        None,
    );
    builder.label(rect(490.0, 405.0, 620.0, 70.0), 21.0, "", Some("defeat"));
    builder.button(
        rect(490.0, 500.0, 620.0, 62.0),
        "Swim again",
        Action::Play,
        true,
    );
    builder.button(
        rect(490.0, 586.0, 620.0, 56.0),
        "Back to our reef",
        Action::Home,
        false,
    );
}

fn draw_language(builder: &mut Builder<'_, '_, '_>) {
    builder.panel(rect(450.0, 180.0, 700.0, 540.0));
    builder.label(
        rect(510.0, 235.0, 580.0, 70.0),
        28.0,
        "Language / Jezik / Lingua",
        None,
    );
    for (index, name) in ["English", "Hrvatski", "Italiano"].into_iter().enumerate() {
        let label = if index == builder.lang {
            format!("✓ {name}")
        } else {
            name.into()
        };
        builder.button(
            rect(510.0, 325.0 + index as f32 * 80.0, 580.0, 58.0),
            &label,
            Action::SelectLanguage(index),
            index == builder.lang,
        );
    }
    builder.button(rect(510.0, 620.0, 580.0, 52.0), "Back", Action::Done, true);
}

fn draw_victory(builder: &mut Builder<'_, '_, '_>) {
    builder.panel(rect(400.0, 145.0, 800.0, 610.0));
    builder.image(rect(750.0, 180.0, 100.0, 100.0), "star");
    builder.label(
        rect(460.0, 295.0, 680.0, 62.0),
        40.0,
        tr("Victory!", builder.lang),
        None,
    );
    builder.label(
        rect(460.0, 370.0, 680.0, 75.0),
        23.0,
        tr(
            "You protected the reef through all 10 rounds!",
            builder.lang,
        ),
        None,
    );
    builder.label(rect(460.0, 460.0, 680.0, 80.0), 23.0, "", Some("victory"));
    builder.button(
        rect(460.0, 570.0, 680.0, 60.0),
        "Swim again",
        Action::Play,
        true,
    );
    builder.button(
        rect(460.0, 655.0, 680.0, 55.0),
        "Back to our reef",
        Action::Home,
        false,
    );
}

fn neighbor<'a>(current: &Hit, direction: Direction, hits: impl Iterator<Item = &'a Hit>) -> usize {
    let axis = direction.vector() * Vec2::new(1.0, -1.0);
    let center = |r: Rect| Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
    hits.filter(|hit| hit.index != current.index)
        .filter_map(|hit| {
            let delta = center(hit.rect) - center(current.rect);
            let along = delta.dot(axis);
            if along <= 1.0 {
                return None;
            }
            let across = if axis.x == 0.0 {
                (delta.x.abs() - (hit.rect.w + current.rect.w) * 0.5).max(0.0)
            } else {
                (delta.y.abs() - (hit.rect.h + current.rect.h) * 0.5).max(0.0)
            };
            Some((hit.index, across, delta.length_squared()))
        })
        .min_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then(a.2.total_cmp(&b.2))
                .then(a.0.cmp(&b.0))
        })
        .map_or(current.index, |(index, _, _)| index)
}

fn actions(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    hints: Res<Hints>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    hits: Query<&Hit>,
    mut focus: ResMut<Focus>,
    mut ui: ResMut<SessionUi>,
    mut simulation: ResMut<Simulation>,
    mut preferences: ResMut<Preferences>,
    mut pending: ResMut<PendingInput>,
    mut audio: ResMut<AudioRequests>,
    options: Res<Options>,
    mut exit: MessageWriter<AppExit>,
    time: Res<Time<Real>>,
    startup: (Local<bool>, Option<ResMut<crate::updates::UpdateCheck>>),
) {
    let (mut wave_pointer, update_check) = startup;
    if options.capture.is_some() {
        return;
    }
    if !window.focused && !options.verify {
        focus.navigation.reset();
        pending.0 = reef_sim::Input::default();
        simulation.0.sabrina.wave.cancel_charge();
        *wave_pointer = false;
        return;
    }
    if focus.last_screen != Some(ui.screen) {
        focus.navigation.reset();
        return;
    }
    if focus.controller != hints.active {
        focus.controller = hints.active;
        focus.navigation.reset();
    }
    let pad = hints.active.and_then(|entity| pads.get(entity).ok());
    if ui.screen != Screen::Playing && pad.is_some_and(|pad| pad.just_pressed(GamepadButton::East))
    {
        app::pause(&mut ui);
        pending.0 = reef_sim::Input::default();
        audio.stop_voice = true;
        preferences.save();
        return;
    }
    let zoom = (window.width() / CANVAS_WIDTH).min(window.height() / CANVAS_HEIGHT);
    let pointer = window.cursor_position().map(|pos| {
        pos / zoom
            - Vec2::new(
                (window.width() / zoom - CANVAS_WIDTH) / 2.0,
                (window.height() / zoom - CANVAS_HEIGHT) / 2.0,
            )
    });
    let menu = ui.screen != Screen::Playing;
    if menu || !mouse.pressed(MouseButton::Left) {
        *wave_pointer = false;
    }
    if !menu && mouse.just_pressed(MouseButton::Left) {
        *wave_pointer = !hits.iter().any(|hit| {
            !matches!(hit.action, Action::Wave) && pointer.is_some_and(|pos| hit.rect.contains(pos))
        });
        if !*wave_pointer {
            pending.0.wave = keys.just_pressed(KeyCode::KeyF)
                || pad.is_some_and(|pad| pad.just_pressed(GamepadButton::South));
        }
    }
    pending.0.wave_held = keys.pressed(KeyCode::KeyF)
        || pad.is_some_and(|pad| pad.pressed(GamepadButton::South))
        || (*wave_pointer && mouse.pressed(MouseButton::Left));
    if !menu
        && mouse.just_pressed(MouseButton::Right)
        && hits
            .iter()
            .any(|hit| pointer.is_some_and(|pos| hit.rect.contains(pos)))
    {
        pending.0.cast = keys.just_pressed(KeyCode::Space)
            || pad.is_some_and(|pad| {
                pad.just_pressed(GamepadButton::West)
                    || pad.just_pressed(GamepadButton::RightTrigger)
            });
    }
    let activate = menu
        && (keys.just_pressed(KeyCode::Enter)
            || keys.just_pressed(KeyCode::Space)
            || pad.is_some_and(|pad| pad.just_pressed(GamepadButton::South)));
    let controller_direction = focus.navigation.update(pad, time.delta_secs());
    let direction = [
        (KeyCode::ArrowUp, Direction::Up),
        (KeyCode::ArrowDown, Direction::Down),
        (KeyCode::ArrowLeft, Direction::Left),
        (KeyCode::ArrowRight, Direction::Right),
    ]
    .into_iter()
    .find_map(|(key, dir)| keys.just_pressed(key).then_some(dir))
    .or(controller_direction)
    .filter(|_| menu && !activate);
    let adjusting = hits
        .iter()
        .any(|hit| hit.index == focus.index && hit.slider)
        && matches!(direction, Some(Direction::Left | Direction::Right));
    if menu && !activate {
        if keys.just_pressed(KeyCode::Tab) && focus.count > 0 {
            let step = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                focus.count - 1
            } else {
                1
            };
            focus.index = (focus.index + step) % focus.count;
        } else if !adjusting
            && let Some(direction) = direction
            && let Some(current) = hits.iter().find(|hit| hit.index == focus.index)
        {
            focus.index = neighbor(current, direction, hits.iter());
        }
    }
    let mut action = None;
    let mut fraction = None;
    for hit in &hits {
        let hover = pointer.is_some_and(|pos| hit.rect.contains(pos));
        if hover && mouse.just_pressed(MouseButton::Left) {
            focus.index = hit.index;
        }
        if hover
            && (mouse.just_pressed(MouseButton::Left)
                || (hit.slider && mouse.pressed(MouseButton::Left)))
        {
            focus.index = hit.index;
            action = Some(hit.action);
            if hit.slider {
                fraction = pointer.map(|pos| ((pos.x - hit.rect.x) / hit.rect.w).clamp(0.0, 1.0));
            }
            pending.0.cast = false;
        }
        if hit.index == focus.index {
            if activate && !hit.slider {
                action = Some(hit.action)
            }
            if hit.slider && adjusting {
                let delta = if direction == Some(Direction::Right) {
                    0.05
                } else {
                    -0.05
                };
                fraction = Some(
                    (match hit.action {
                        Action::Music => preferences.music,
                        Action::Effects => preferences.effects,
                        Action::Voice => preferences.voice,
                        Action::Rest => (preferences.rest - 25.0) / 125.0,
                        _ => 0.0,
                    } + delta)
                        .clamp(0.0, 1.0),
                );
                action = Some(hit.action);
            }
        }
    }
    let Some(action) = action else { return };
    match action {
        Action::Play => {
            app::begin(&mut simulation.0, &mut ui, &preferences, false);
            audio.stop_voice = true;
        }
        Action::Aquarium => {
            app::begin(&mut simulation.0, &mut ui, &preferences, true);
            audio.stop_voice = true;
        }
        Action::Home => {
            app::home(&mut simulation.0, &mut ui, &preferences);
            audio.stop_voice = true;
        }
        Action::Pause => {
            simulation.0.sabrina.wave.cancel_charge();
            app::pause(&mut ui);
        }
        Action::Settings | Action::Credits | Action::Help => {
            ui.return_to = if ui.started {
                Screen::Pause
            } else {
                Screen::Menu
            };
            ui.screen = match action {
                Action::Settings => Screen::Settings,
                Action::Credits => Screen::Credits,
                _ => Screen::Help,
            };
            audio.stop_voice = true;
        }
        Action::Done => {
            ui.screen = if ui.screen == Screen::TailColor {
                Screen::Settings
            } else {
                ui.return_to
            };
            audio.stop_voice = true;
            preferences.save();
        }
        Action::Language => {
            ui.return_to = ui.screen;
            ui.screen = Screen::Language;
        }
        Action::SelectLanguage(language) => preferences.language = language,
        Action::Quit => {
            preferences.save();
            exit.write(AppExit::Success);
        }
        Action::Fullscreen => {
            preferences.fullscreen = !preferences.fullscreen;
            window.mode = if preferences.fullscreen {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            } else {
                WindowMode::Windowed
            }
        }
        Action::Quality => preferences.quality = (preferences.quality + 1) % 3,
        Action::Vsync => {
            preferences.vsync = !preferences.vsync;
            window.present_mode = preferences.present_mode();
        }
        Action::Fps => preferences.show_fps = !preferences.show_fps,
        Action::Motion => preferences.reduced_motion = !preferences.reduced_motion,
        Action::Rumble => preferences.rumble = (preferences.rumble + 1) % 3,
        Action::TailColor => ui.screen = Screen::TailColor,
        Action::SelectTail(color) => preferences.tail_color = color,
        Action::Gentle => {
            preferences.gentle = !preferences.gentle;
            simulation.0.settings.gentle = preferences.gentle
        }
        Action::Visitors => {
            preferences.visitors = preferences.visitors % 3 + 1;
            simulation.0.settings.visitor_limit = preferences.visitors
        }
        Action::Crystal => pending.0.cast = true,
        Action::Song => pending.0.toggle_song = true,
        Action::Wave => pending.0.wave = true,
        Action::Original => {
            let url = "https://scratch.mit.edu/projects/1203591608/";
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(url).spawn();
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(url).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .spawn();
        }
        Action::SkipUpdate => {
            if let Some(mut check) = update_check {
                check.status = crate::updates::Status::Continue;
                check.browser_error = false;
            }
        }
        Action::ViewRelease => {
            if let Some(mut check) = update_check {
                match crate::updates::open_release(&mut check) {
                    Ok(()) => {
                        exit.write(AppExit::Success);
                    }
                    Err(error) => {
                        error!("{error}");
                        check.browser_error = true;
                    }
                }
            }
        }
        Action::Music => {
            if let Some(f) = fraction {
                preferences.music = f
            }
        }
        Action::Effects => {
            if let Some(f) = fraction {
                preferences.effects = f
            }
        }
        Action::Voice => {
            if let Some(f) = fraction {
                preferences.voice = f
            }
        }
        Action::Rest => {
            if let Some(f) = fraction {
                preferences.rest = (25.0 + 125.0 * f).round();
                simulation.0.settings.rest_seconds = preferences.rest;
            }
        }
    }
    if !matches!(
        action,
        Action::Crystal
            | Action::Song
            | Action::Wave
            | Action::Pause
            | Action::SkipUpdate
            | Action::ViewRelease
    ) {
        preferences.save();
    }
}

fn reward_line(key: &str) -> Option<(reef_sim::ScoreFeedback, &'static str)> {
    match key {
        "reward-crystal" => Some((reef_sim::ScoreFeedback::Crystal, "Crystal hit")),
        "reward-wave" => Some((reef_sim::ScoreFeedback::Wave, "Wave hit")),
        "reward-rescue" => Some((reef_sim::ScoreFeedback::Rescue, "Fish rescued")),
        "reward-retreat" => Some((reef_sim::ScoreFeedback::Retreat, "Visitor repelled")),
        "reward-survivors" => Some((reef_sim::ScoreFeedback::Survivors, "Survivor bonus")),
        _ => None,
    }
}

fn hud_stats(world: &reef_sim::World, language: usize) -> String {
    format!(
        "{}{}{}{}",
        reef_sim::alive(world),
        tr(" fish   ·   ", language),
        tr("Score ", language),
        reef_sim::current_total_score(world)
    )
}

fn reward_opacity(time: f32) -> f32 {
    (time / 0.6).clamp(0.0, 1.0)
}

fn update(
    simulation: Res<Simulation>,
    ui: Res<SessionUi>,
    preferences: Res<Preferences>,
    mut texts: Query<
        (&Dynamic, &mut Text, &mut TextColor, Option<&mut TextShadow>),
        Without<ButtonLabel>,
    >,
    mut labels: Query<(&ButtonLabel, &mut TextColor, &mut Text), Without<Dynamic>>,
    mut bars: Query<(&Meter, &mut Node)>,
    focus: Res<Focus>,
    mut hits: Query<(&Hit, &mut ImageNode)>,
    library: Res<crate::assets::Library>,
    loading: Res<crate::loading::Loading>,
    server: Res<AssetServer>,
) {
    let sabrina = &simulation.0.sabrina;
    let language = preferences.language;
    for (key, mut text, mut color, shadow) in &mut texts {
        if let Some((kind, label)) = reward_line(key.0) {
            let reward = simulation.0.points.rewards[kind as usize];
            let value = if reward.time > 0.0 {
                format!("+{} · {}", reward.points, tr(label, language))
            } else {
                String::new()
            };
            text.set_if_neq(Text(value));
            color.set_if_neq(TextColor(
                Color::srgb(0.87, 0.97, 1.0).with_alpha(reward_opacity(reward.time)),
            ));
            if let Some(mut shadow) = shadow {
                let next = TextShadow {
                    color: Color::BLACK.with_alpha(0.85 * reward_opacity(reward.time)),
                    ..*shadow
                };
                shadow.set_if_neq(next);
            }
            continue;
        }
        let value = match key.0 {
            "loading" if loading.progress == 1.0 => tr("Press any key to continue", language),
            "loading" => format!(
                "{}  {:.0}%",
                tr(
                    if loading.warming {
                        "Preparing graphics…"
                    } else if library.ready {
                        "Preparing the characters…"
                    } else {
                        "Bringing the reef to life…"
                    },
                    language,
                ),
                loading.progress * 100.0
            ),
            "victory" => format!(
                "{}{}\n{}{}",
                tr("Total score: ", language),
                simulation.0.points.total_score,
                tr("Fish safe: ", language),
                simulation.0.points.last_survivors
            ),
            "phase" => {
                if simulation.0.session.water == reef_sim::Water::Visit {
                    format!(
                        "{}{}/{}",
                        tr("Round ", language),
                        simulation.0.session.cycle,
                        reef_sim::FINAL_ROUND
                    )
                } else {
                    tr("A peaceful little sea", language)
                }
            }
            "defeat" => tr(
                if reef_sim::alive(&simulation.0) == 0 {
                    "All fish were lost. Try again and protect the school!"
                } else {
                    "Sabrina needs a rest. Try again when you are ready."
                },
                language,
            ),
            "stats" => hud_stats(&simulation.0, language),
            "health" => format!("{} / 100", sabrina.vitality.health.ceil() as i32),
            "crystal" if reef_sim::crystal_chain_target(&simulation.0).is_some() => {
                format!(
                    "{} · {:.1}",
                    tr("Chain", language),
                    (reef_sim::CRYSTAL_CHAIN_SECONDS - simulation.0.crystal.magic.time).max(0.0)
                )
            }
            "crystal" if sabrina.cooldown > 0.0 => format!("{}", sabrina.cooldown.ceil() as i32),
            "song" if sabrina.song.energy < 99.99 || sabrina.song.active => {
                format!("{}%", sabrina.song.energy as i32)
            }
            "wave" if sabrina.wave.charging => format!(
                "{:.1}",
                (reef_sim::WAVE_CHARGE_SECONDS - sabrina.wave.charge_time).max(0.0)
            ),
            "wave" if sabrina.wave.cooldown > 0.0 => format!("{:.1}", sabrina.wave.cooldown),
            "caption" => ui.caption.clone(),
            "notice" => {
                if ui.notice_time > 0.0 {
                    ui.notice.clone()
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    for (meter, mut node) in &mut bars {
        let amount = match meter.0 {
            "loading" => loading.progress,
            "health" => sabrina.vitality.health / 100.0,
            "song" => sabrina.song.energy / 100.0,
            "crystal" => 1.0 - sabrina.cooldown / reef_sim::recharge_duration(&simulation.0),
            "wave" => 1.0 - sabrina.wave.cooldown / reef_sim::wave_recharge_duration(&simulation.0),
            _ => 0.0,
        };
        let width = px(meter.1 * amount.clamp(0.0, 1.0));
        if node.width != width {
            node.width = width;
        }
    }
    for (label, mut color, mut text) in &mut labels {
        let disabled = match label.action {
            Action::Song => sabrina.song.locked,
            Action::Wave => {
                sabrina.wave.cooldown > 0.0
                    || simulation.0.crystal.magic.state == reef_sim::Spell::Charging
            }
            _ => false,
        };
        let desired_color = if disabled {
            Color::srgb(0.5, 0.65, 0.66)
        } else if ui.screen == Screen::Playing {
            Color::srgb(0.87, 0.97, 1.0)
        } else if label.primary || label.index == focus.index {
            Color::srgb(0.20, 0.17, 0.18)
        } else {
            Color::srgb(0.87, 0.97, 1.0)
        };
        color.set_if_neq(TextColor(desired_color));
        if matches!(label.action, Action::Song) {
            let value = tr(
                if sabrina.song.locked {
                    "Song recharging…"
                } else if sabrina.song.active {
                    "Stop singing"
                } else {
                    "Sing"
                },
                language,
            );
            if text.0 != value {
                text.0 = value;
            }
        }
    }
    for (hit, mut image) in &mut hits {
        let file = if hit.index == focus.index {
            "button_hover"
        } else if matches!(hit.action, Action::SelectLanguage(selected) if selected == language)
            || matches!(hit.action, Action::Play | Action::Done | Action::Pause)
                && ui.screen != Screen::Playing
        {
            "button"
        } else {
            "button_teal"
        };
        let handle = server.load(format!("ui/reef_png/{file}.png"));
        if image.image != handle {
            image.image = handle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::{
        InputPlugin,
        gamepad::{RawGamepadButtonChangedEvent, RawGamepadEvent},
    };

    #[test]
    fn ability_icons_track_cooldowns_energy_and_charging() {
        let mut world = reef_sim::World::default();
        assert_eq!(Ability::Crystal.state(&world), Vec4::X);
        assert_eq!(Ability::Wave.state(&world), Vec4::X);
        world.sabrina.cooldown = reef_sim::recharge_duration(&world) * 0.5;
        world.sabrina.wave.cooldown = reef_sim::wave_recharge_duration(&world) * 0.75;
        world.sabrina.song.energy = 40.0;
        world.sabrina.song.locked = true;
        assert_eq!(Ability::Crystal.state(&world).x, 0.5);
        assert_eq!(Ability::Wave.state(&world).x, 0.25);
        assert_eq!(Ability::Song.state(&world), Vec4::new(0.4, 1.0, 0.0, 0.0));
        world.crystal.magic.state = reef_sim::Spell::Charging;
        assert_eq!(Ability::Crystal.state(&world).y, 1.0);
        assert_eq!(Ability::Wave.state(&world).y, 1.0);
        world.sabrina.song.locked = false;
        world.sabrina.song.active = true;
        assert_eq!(Ability::Song.state(&world), Vec4::new(0.4, 0.0, 1.0, 0.0));
    }

    #[test]
    fn ability_icons_show_charge_and_chain_windows_and_extended_cooldowns() {
        let mut world = reef_sim::World::default();
        world.sabrina.wave.charging = true;
        world.sabrina.wave.charge_time = 1.5;
        assert_eq!(Ability::Wave.state(&world), Vec4::new(0.5, 0.0, 1.0, 0.0));
        world.sabrina.wave.charging = false;
        world.sabrina.wave.scale = 2.0;
        world.sabrina.wave.cooldown = 4.5;
        assert_eq!(Ability::Wave.state(&world).x, 0.5);
        world.crystal.magic.state = reef_sim::Spell::Burst;
        world.crystal.magic.hit_count = 1;
        world.crystal.magic.time = 0.75;
        let id = reef_sim::allocate_entity(&mut world, reef_sim::EntityKind::Jelly, 0);
        world.jelly[0].id = id;
        world.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
        world.jelly[0].motion.pos = world.crystal.motion.pos;
        assert_eq!(
            Ability::Crystal.state(&world),
            Vec4::new(0.5, 0.0, 1.0, 0.0)
        );
        world.crystal.magic.state = reef_sim::Spell::Resting;
        world.crystal.magic.hit_count = 4;
        world.sabrina.cooldown = 9.0;
        assert_eq!(Ability::Crystal.state(&world).x, 0.5);
    }

    #[test]
    fn hud_stats_always_show_total_score_without_arrival_countdown() {
        let mut world = reef_sim::World::default();
        world.points.total_score = 700;
        world.points.score = 30;
        world.session.rest_remaining = 43.0;
        assert!(hud_stats(&world, 0).ends_with("Score 700"));
        world.session.water = reef_sim::Water::Visit;
        assert!(hud_stats(&world, 0).ends_with("Score 730"));
        assert_eq!(reward_opacity(2.5), 1.0);
        assert_eq!(reward_opacity(0.6), 1.0);
        assert_eq!(reward_opacity(0.3), 0.5);
        assert_eq!(reward_opacity(0.0), 0.0);
    }

    #[test]
    fn hud_has_one_clickable_icon_per_ability_and_no_duplicate_bars() {
        fn build(
            mut commands: Commands,
            server: Res<AssetServer>,
            mut materials: ResMut<Assets<AbilityIcon>>,
        ) {
            let root = commands.spawn_empty().id();
            let mut builder = Builder {
                commands: &mut commands,
                server: &server,
                root,
                lang: 0,
                index: 0,
                font: default(),
                background_alpha: 1.0,
                controls: Style::Keyboard,
            };
            draw_hud(&mut builder, Screen::Playing, &mut materials);
        }
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<AbilityIcon>()
            .add_systems(Startup, build);
        app.update();
        let world = app.world_mut();
        let dynamic: Vec<_> = world
            .query::<&Dynamic>()
            .iter(world)
            .map(|key| key.0)
            .collect();
        assert!(
            !dynamic
                .iter()
                .any(|key| ["visitors", "score", "reward"].contains(key))
        );
        assert_eq!(
            dynamic
                .iter()
                .filter(|key| reward_line(key).is_some())
                .count(),
            5
        );
        for (key, node) in world.query::<(&Dynamic, &Node)>().iter(world) {
            if ["phase", "stats", "health"].contains(&key.0) {
                assert!(matches!(node.left, Val::Px(x) if (36.0..=65.0).contains(&x)));
                assert!(matches!(node.top, Val::Px(y) if (30.0..=89.0).contains(&y)));
                assert!(matches!(node.width, Val::Px(w) if w <= 294.0));
                assert!(matches!(node.height, Val::Px(h) if h <= 26.0));
            }
            if reward_line(key.0).is_some() {
                assert!(matches!(node.left, Val::Px(x) if x == 40.0));
                assert!(matches!(node.top, Val::Px(y) if (690.0..840.0).contains(&y)));
            }
        }
        assert_eq!(world.query::<&Ability>().iter(world).count(), 3);
        for hit in world.query::<&Hit>().iter(world) {
            let x = match hit.action {
                Action::Wave => 610.0,
                Action::Song => 754.0,
                Action::Crystal => 898.0,
                _ => continue,
            };
            assert_eq!(hit.rect.x, x);
        }
        for predicate in [
            |action: Action| matches!(action, Action::Crystal),
            |action: Action| matches!(action, Action::Song),
            |action: Action| matches!(action, Action::Wave),
        ] {
            assert_eq!(
                world
                    .query::<&Hit>()
                    .iter(world)
                    .filter(|hit| predicate(hit.action))
                    .count(),
                1
            );
        }
        assert!(
            world
                .query::<&Meter>()
                .iter(world)
                .all(|meter| meter.0 == "health")
        );
        let compact_panels: Vec<_> = world
            .query::<(&ImageNode, &Node)>()
            .iter(world)
            .filter(|(_, node)| node.left == px(18.0) && node.top == px(18.0))
            .collect();
        assert_eq!(compact_panels.len(), 1);
        let panel = compact_panels[0].1;
        assert_eq!(panel.width, px(330.0));
        assert_eq!(panel.height, px(112.0));
        let (Val::Px(width), Val::Px(height)) = (panel.width, panel.height) else {
            panic!("HUD panel must have fixed logical dimensions");
        };
        assert!(width * height < 490.0 * 190.0 * 0.4);
        for (_, node) in world.query::<(&Meter, &Node)>().iter(world) {
            assert_eq!(node.left, px(163.0));
            assert_eq!(node.top, px(95.0));
            assert_eq!(node.height, px(9.0));
        }
        assert!(
            world
                .query::<&Hit>()
                .iter(world)
                .filter(|hit| !matches!(hit.action, Action::Pause))
                .all(|hit| hit.rect.y == 754.0)
        );
    }

    #[test]
    fn ability_material_is_not_reuploaded_when_state_is_unchanged() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AbilityIcon>()
            .insert_resource(Simulation(reef_sim::World::default()))
            .add_systems(Update, update_abilities);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AbilityIcon>>()
            .add(AbilityIcon {
                state: Vec4::ZERO,
                image: default(),
            });
        app.world_mut()
            .spawn((Ability::Crystal, MaterialNode(handle)));
        app.update();
        app.update();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<AbilityIcon>>>()
            .clear();
        for _ in 0..3 {
            app.update();
        }
        assert!(
            !app.world()
                .resource::<Messages<AssetEvent<AbilityIcon>>>()
                .iter_current_update_messages()
                .any(|event| matches!(event, AssetEvent::Modified { .. }))
        );
    }

    #[test]
    fn tail_presets_and_instructions_are_localized() {
        for language in 0..3 {
            for key in TailColor::PRESETS.into_iter().map(TailColor::label).chain([
                "Tail color",
                "None keeps the original. Magic flows with rainbow colors.",
                "Selected:",
                "Custom",
            ]) {
                assert!(table().contains_key(key), "{key}");
                assert!(!tr(key, language).is_empty());
            }
        }
    }

    #[test]
    fn tail_picker_back_preserves_settings_return_destination() {
        let mut ui = SessionUi {
            screen: Screen::TailColor,
            return_to: Screen::Pause,
            started: true,
            ..default()
        };
        app::pause(&mut ui);
        assert_eq!(ui.screen, Screen::Settings);
        app::pause(&mut ui);
        assert_eq!(ui.screen, Screen::Pause);
    }

    #[test]
    fn title_artwork_matches_translation_language_order() {
        assert_eq!(title_path(0), "ui/title_eng.png");
        assert_eq!(title_path(1), "ui/title_hr.png");
        assert_eq!(title_path(2), "ui/title_it.png");
    }

    #[test]
    fn any_keyboard_mouse_or_controller_button_can_continue_loading() {
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut mouse = ButtonInput::<MouseButton>::default();
        assert!(!any_loading_input_pressed(&keys, &mouse, false));

        keys.press(KeyCode::KeyA);
        assert!(any_loading_input_pressed(&keys, &mouse, false));
        keys.clear();
        mouse.press(MouseButton::Right);
        assert!(any_loading_input_pressed(&keys, &mouse, false));
        mouse.clear();
        assert!(any_loading_input_pressed(&keys, &mouse, true));
    }

    #[test]
    fn loading_bar_follows_artwork_and_stays_visible_on_wide_and_tall_windows() {
        for (width, height) in [(1280.0_f32, 720.0_f32), (2560.0, 1080.0), (720.0, 1280.0)] {
            let zoom = (width / CANVAS_WIDTH).min(height / CANVAS_HEIGHT);
            let image = loading_image_rect(width / zoom, height / zoom);
            let (status, bar) = loading_overlay(image);
            assert!((bar.x + bar.w / 2.0 - CANVAS_WIDTH / 2.0).abs() < 1.0);
            assert!((status.x + status.w / 2.0 - CANVAS_WIDTH / 2.0).abs() < 1.0);
            assert!(bar.y + bar.h <= (CANVAS_HEIGHT + height / zoom) / 2.0 - 23.99);
            assert!(bar.y > status.y + status.h);
            assert!((bar.w / image.w - 360.0 / LOADING_ART_WIDTH).abs() < 0.00001);
        }
    }

    #[test]
    fn localized_loading_title_stays_visible_above_mermaid() {
        for (width, height) in [(1280.0_f32, 720.0_f32), (2560.0, 1080.0), (720.0, 1280.0)] {
            let zoom = (width / CANVAS_WIDTH).min(height / CANVAS_HEIGHT);
            let (view_w, view_h) = (width / zoom, height / zoom);
            let image = loading_image_rect(view_w, view_h);
            let title = loading_title_rect(image, view_w, view_h);
            let (status, _) = loading_overlay(image);
            assert!(title.x >= (CANVAS_WIDTH - view_w) / 2.0 + 15.99);
            assert!(title.x + title.w <= (CANVAS_WIDTH + view_w) / 2.0 - 15.99);
            assert!(title.y >= (CANVAS_HEIGHT - view_h) / 2.0 + 7.99);
            assert!((title.w / title.h - 3.0).abs() < 0.001);
            assert!(title.y + title.h < status.y);
        }
        let image = loading_image_rect(CANVAS_WIDTH, CANVAS_HEIGHT);
        let title = loading_title_rect(image, CANVAS_WIDTH, CANVAS_HEIGHT);
        let hair_top = image.y + 190.0 * image.w / LOADING_ART_WIDTH;
        assert!(title.y + title.h < hair_top);
    }

    #[test]
    fn ready_prompt_pulses_without_disappearing() {
        let samples: Vec<_> = (0..240)
            .map(|i| loading_prompt_alpha(i as f32 / 60.0))
            .collect();
        assert!(samples.iter().all(|alpha| (0.62..=1.0).contains(alpha)));
        assert!(samples.iter().any(|alpha| *alpha < 0.65));
        assert!(samples.iter().any(|alpha| *alpha > 0.97));
    }

    #[test]
    fn ready_prompt_overlay_covers_loading_text_and_progress_bar() {
        let image = loading_image_rect(CANVAS_WIDTH, CANVAS_HEIGHT);
        let (status, bar) = loading_overlay(image);
        let overlay = loading_ready_overlay(image);

        for covered in [status, bar] {
            assert!(overlay.x <= covered.x);
            assert!(overlay.y <= covered.y);
            assert!(overlay.x + overlay.w >= covered.x + covered.w);
            assert!(overlay.y + overlay.h >= covered.y + covered.h);
        }
    }

    #[test]
    fn input_backend_reports_the_enabled_transport_not_the_graphics_api() {
        assert_eq!(input_backend(true, false), "XInput (Xbox / Steam Input)");
        assert_eq!(input_backend(true, true), "Windows Gaming Input / GilRs");
        assert_eq!(input_backend(false, true), "GilRs");
    }

    fn hit(index: usize, x: f32, y: f32, slider: bool) -> Hit {
        Hit {
            rect: rect(x, y, 100.0, 50.0),
            action: Action::Play,
            index,
            slider,
        }
    }

    #[test]
    fn spatial_focus_follows_rows_and_columns_without_wrapping() {
        let hits = [
            hit(0, 0.0, 0.0, false),
            hit(1, 120.0, 0.0, false),
            hit(2, 0.0, 100.0, false),
            hit(3, 120.0, 100.0, false),
        ];
        assert_eq!(neighbor(&hits[0], Direction::Right, hits.iter()), 1);
        assert_eq!(neighbor(&hits[0], Direction::Down, hits.iter()), 2);
        assert_eq!(neighbor(&hits[3], Direction::Up, hits.iter()), 1);
        assert_eq!(neighbor(&hits[3], Direction::Left, hits.iter()), 2);
        assert_eq!(neighbor(&hits[0], Direction::Up, hits.iter()), 0);
        assert_eq!(neighbor(&hits[3], Direction::Down, hits.iter()), 3);
    }

    fn input_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin))
            .add_message::<AppExit>()
            .init_resource::<Hints>()
            .init_resource::<PendingInput>()
            .init_resource::<Preferences>()
            .init_resource::<AudioRequests>()
            .insert_resource(Options {
                verify: true,
                ..default()
            })
            .insert_resource(SessionUi {
                screen: Screen::Menu,
                ..default()
            })
            .insert_resource(Simulation(reef_sim::World::default()))
            .insert_resource(Focus {
                last_screen: Some(Screen::Menu),
                count: 2,
                ..default()
            })
            .add_systems(
                PreUpdate,
                (crate::controls::update, app::input, actions)
                    .chain()
                    .after(bevy::input::InputSystems),
            );
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ));
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.world_mut().spawn(hit(0, 0.0, 0.0, false));
        app.world_mut().spawn(Hit {
            action: Action::Settings,
            ..hit(1, 120.0, 0.0, false)
        });
        app.update();
        (app, pad)
    }

    #[test]
    fn ability_icon_clicks_trigger_only_the_selected_action() {
        use bevy::ecs::system::RunSystemOnce;

        for action in [Action::Crystal, Action::Song, Action::Wave] {
            let (mut app, _) = input_app();
            let entities: Vec<_> = app
                .world_mut()
                .query_filtered::<Entity, With<Hit>>()
                .iter(app.world())
                .collect();
            for entity in entities {
                app.world_mut().despawn(entity);
            }
            app.world_mut().resource_mut::<SessionUi>().screen = Screen::Playing;
            app.world_mut().resource_mut::<Focus>().last_screen = Some(Screen::Playing);
            let area = rect(610.0, 754.0, 92.0, 92.0);
            app.world_mut().spawn(Hit {
                rect: area,
                action,
                index: 0,
                slider: false,
            });
            let mut query = app
                .world_mut()
                .query_filtered::<&mut Window, With<PrimaryWindow>>();
            let mut window = query.single_mut(app.world_mut()).unwrap();
            let zoom = (window.width() / CANVAS_WIDTH).min(window.height() / CANVAS_HEIGHT);
            let point = (Vec2::new(area.x + 46.0, area.y + 46.0)
                + Vec2::new(
                    (window.width() / zoom - CANVAS_WIDTH) / 2.0,
                    (window.height() / zoom - CANVAS_HEIGHT) / 2.0,
                ))
                * zoom;
            window.set_cursor_position(Some(point));
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            app.world_mut().run_system_once(app::input).unwrap();
            app.world_mut().run_system_cached(actions).unwrap();
            let input = &app.world().resource::<PendingInput>().0;
            assert_eq!(input.cast, matches!(action, Action::Crystal));
            assert_eq!(input.toggle_song, matches!(action, Action::Song));
            assert_eq!(input.wave, matches!(action, Action::Wave));
            assert_eq!(input.wave_held, matches!(action, Action::Wave));
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .clear();
            app.world_mut().run_system_once(app::input).unwrap();
            app.world_mut().run_system_cached(actions).unwrap();
            assert_eq!(
                app.world().resource::<PendingInput>().0.wave_held,
                matches!(action, Action::Wave),
                "Holding another HUD icon must not charge Wave"
            );
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            app.world_mut().resource_mut::<PendingInput>().0.wave_held = false;
            app.world_mut().run_system_cached(actions).unwrap();
            assert!(!app.world().resource::<PendingInput>().0.wave_held);
            app.world_mut().resource_mut::<PendingInput>().0 = default();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .reset_all();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Right);
            app.world_mut().run_system_once(app::input).unwrap();
            app.world_mut().run_system_cached(actions).unwrap();
            let input = &app.world().resource::<PendingInput>().0;
            assert!(
                !input.cast,
                "Right-clicking HUD must not throw into the scene"
            );
            assert!(!input.wave);
            assert!(!input.wave_held);
            assert!(!input.toggle_song);
        }
    }

    #[test]
    fn scene_mouse_buttons_map_to_wave_and_crystal_without_changing_keyboard() {
        use bevy::ecs::system::RunSystemOnce;

        let (mut app, _) = input_app();
        app.world_mut().resource_mut::<SessionUi>().screen = Screen::Playing;
        app.world_mut().resource_mut::<Focus>().last_screen = Some(Screen::Playing);
        for (mouse_button, key, wave, crystal) in [
            (Some(MouseButton::Left), None, true, false),
            (Some(MouseButton::Right), None, false, true),
            (None, Some(KeyCode::KeyF), true, false),
            (None, Some(KeyCode::Space), false, true),
            (None, Some(KeyCode::KeyE), false, false),
        ] {
            app.world_mut().resource_mut::<PendingInput>().0 = default();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .reset_all();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .reset_all();
            if let Some(button) = mouse_button {
                app.world_mut()
                    .resource_mut::<ButtonInput<MouseButton>>()
                    .press(button);
            }
            if let Some(key) = key {
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(key);
            }
            app.world_mut().run_system_once(app::input).unwrap();
            app.world_mut().run_system_cached(actions).unwrap();
            let pending = &app.world().resource::<PendingInput>().0;
            assert_eq!(pending.wave, wave);
            assert_eq!(pending.wave_held, wave);
            assert_eq!(pending.cast, crystal);
            assert!(!pending.toggle_song);
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .clear();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
            app.world_mut().resource_mut::<PendingInput>().0 = default();
            app.world_mut().run_system_once(app::input).unwrap();
            app.world_mut().run_system_cached(actions).unwrap();
            let pending = &app.world().resource::<PendingInput>().0;
            assert!(!pending.wave);
            assert_eq!(pending.wave_held, wave);
            assert!(
                !pending.cast,
                "Holding RMB or Space must not repeat Crystal"
            );
        }
    }

    fn button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
    }

    #[test]
    fn controller_can_skip_update_without_starting_gameplay() {
        let (mut app, pad) = input_app();
        let mut check = crate::updates::UpdateCheck::default();
        check.status = crate::updates::Status::Available {
            tag: "v0.3.0".into(),
            url: "https://github.com/arvvoid/sabrinina-podvodna-bajka/releases/tag/v0.3.0".into(),
        };
        check.browser_error = true;
        app.insert_resource(check);
        app.world_mut().resource_mut::<SessionUi>().screen = Screen::Loading;
        app.world_mut().resource_mut::<Focus>().last_screen = Some(Screen::Loading);
        for mut hit in app
            .world_mut()
            .query::<&mut Hit>()
            .iter_mut(app.world_mut())
        {
            hit.action = if hit.index == 0 {
                Action::ViewRelease
            } else {
                Action::SkipUpdate
            };
        }
        button(&mut app, pad, GamepadButton::DPadRight, 1.0);
        button(&mut app, pad, GamepadButton::DPadRight, 0.0);
        app.update();
        assert_eq!(app.world().resource::<Focus>().index, 1);
        button(&mut app, pad, GamepadButton::South, 1.0);
        button(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        let check = app.world().resource::<crate::updates::UpdateCheck>();
        assert_eq!(check.status, crate::updates::Status::Continue);
        assert!(!check.browser_error);
        let ui = app.world().resource::<SessionUi>();
        assert_eq!(ui.screen, Screen::Loading);
        assert!(!ui.loaded);
        assert!(!app.world().resource::<PendingInput>().0.cast);
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
    }

    #[test]
    fn update_prompt_has_all_three_translations() {
        for key in [
            "Checking for updates…",
            "Just a moment before we dive in.",
            "A new underwater adventure awaits!",
            "New version available:",
            "View the GitHub release to download it, or keep playing this version.",
            "Could not open your browser. Try again or skip for now.",
            "View release and exit",
            "Skip for now",
        ] {
            assert!(table().contains_key(key));
            for language in 0..3 {
                assert!(!tr(key, language).is_empty());
            }
        }
    }

    #[test]
    fn tail_picker_controller_selects_magic_resets_and_returns_to_settings() {
        let (mut app, pad) = input_app();
        let hits: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<Hit>>()
            .iter(app.world())
            .collect();
        for entity in hits {
            app.world_mut().despawn(entity);
        }
        for (index, action) in [
            Action::SelectTail(TailColor::Magic),
            Action::SelectTail(TailColor::None),
            Action::Done,
        ]
        .into_iter()
        .enumerate()
        {
            app.world_mut().spawn(Hit {
                action,
                ..hit(index, 0.0, index as f32 * 100.0, false)
            });
        }
        {
            let mut ui = app.world_mut().resource_mut::<SessionUi>();
            ui.screen = Screen::TailColor;
            ui.return_to = Screen::Pause;
        }
        {
            let mut focus = app.world_mut().resource_mut::<Focus>();
            focus.last_screen = Some(Screen::TailColor);
            focus.count = 3;
        }
        button(&mut app, pad, GamepadButton::South, 1.0);
        button(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        assert_eq!(
            app.world().resource::<Preferences>().tail_color,
            TailColor::Magic
        );
        button(&mut app, pad, GamepadButton::DPadDown, 1.0);
        button(&mut app, pad, GamepadButton::DPadDown, 0.0);
        app.update();
        button(&mut app, pad, GamepadButton::South, 1.0);
        button(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        assert_eq!(
            app.world().resource::<Preferences>().tail_color,
            TailColor::None
        );
        button(&mut app, pad, GamepadButton::DPadDown, 1.0);
        button(&mut app, pad, GamepadButton::DPadDown, 0.0);
        app.update();
        button(&mut app, pad, GamepadButton::South, 1.0);
        button(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        let ui = app.world().resource::<SessionUi>();
        assert_eq!(ui.screen, Screen::Settings);
        assert_eq!(ui.return_to, Screen::Pause);
    }

    #[test]
    fn raw_dpad_and_quick_confirm_taps_reach_menu_once() {
        let (mut app, pad) = input_app();
        button(&mut app, pad, GamepadButton::DPadRight, 1.0);
        button(&mut app, pad, GamepadButton::DPadRight, 0.0);
        app.update();
        assert_eq!(app.world().resource::<Focus>().index, 1);
        button(&mut app, pad, GamepadButton::South, 1.0);
        button(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Settings);
        assert!(!app.world().resource::<PendingInput>().0.cast);
        app.update();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Settings);
    }

    #[test]
    fn confirm_does_not_move_focus_on_a_direction_repeat() {
        let (mut app, pad) = input_app();
        button(&mut app, pad, GamepadButton::DPadRight, 1.0);
        button(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Playing);
        assert!(!app.world().resource::<PendingInput>().0.cast);
    }

    #[test]
    fn back_returns_from_submenu_without_casting_wave() {
        let (mut app, pad) = input_app();
        app.world_mut().resource_mut::<SessionUi>().screen = Screen::Settings;
        app.world_mut().resource_mut::<Focus>().last_screen = Some(Screen::Settings);
        button(&mut app, pad, GamepadButton::East, 1.0);
        app.update();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Menu);
        assert!(!app.world().resource::<PendingInput>().0.wave);
    }

    #[test]
    fn screen_transition_cannot_activate_old_hits() {
        let (mut app, pad) = input_app();
        {
            let mut ui = app.world_mut().resource_mut::<SessionUi>();
            ui.screen = Screen::Playing;
            ui.started = true;
        }
        app.world_mut().resource_mut::<Focus>().last_screen = Some(Screen::Playing);
        button(&mut app, pad, GamepadButton::Start, 1.0);
        button(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Pause);
        assert!(!app.world().resource::<PendingInput>().0.cast);
    }

    #[test]
    fn slider_adjustment_keeps_focus_and_each_fresh_tap_registers() {
        let (mut app, pad) = input_app();
        for mut hit in app
            .world_mut()
            .query::<&mut Hit>()
            .iter_mut(app.world_mut())
        {
            if hit.index == 0 {
                hit.action = Action::Music;
                hit.slider = true;
            }
        }
        for step in 1..=6 {
            button(&mut app, pad, GamepadButton::DPadRight, 1.0);
            button(&mut app, pad, GamepadButton::DPadRight, 0.0);
            app.update();
            assert_eq!(app.world().resource::<Focus>().index, 0);
            assert!(
                (app.world().resource::<Preferences>().music - (0.3 + step as f32 * 0.05)).abs()
                    < 0.001
            );
        }
    }

    #[test]
    fn newly_started_gameplay_input_is_not_discarded_with_old_menu_hits() {
        let (mut app, pad) = input_app();
        app.world_mut().resource_mut::<SessionUi>().screen = Screen::Playing;
        button(&mut app, pad, GamepadButton::East, 1.0);
        app.update();
        assert!(app.world().resource::<PendingInput>().0.toggle_song);
    }

    #[test]
    fn gameplay_controller_buttons_trigger_only_the_selected_ability() {
        for (control, expected) in [
            (GamepadButton::South, (true, false, false)),
            (GamepadButton::East, (false, true, false)),
            (GamepadButton::West, (false, false, true)),
            (GamepadButton::RightTrigger, (false, false, true)),
        ] {
            let (mut app, pad) = input_app();
            app.world_mut().resource_mut::<SessionUi>().screen = Screen::Playing;
            button(&mut app, pad, control, 1.0);
            app.update();
            let input = &app.world().resource::<PendingInput>().0;
            assert_eq!((input.wave, input.toggle_song, input.cast), expected);
        }
    }

    #[test]
    fn pause_cancels_wave_charge_and_first_resumed_wave_still_works() {
        use bevy::ecs::system::RunSystemOnce;

        let (mut app, _) = input_app();
        app.world_mut().resource_mut::<SessionUi>().screen = Screen::Playing;
        app.world_mut()
            .resource_mut::<Simulation>()
            .0
            .sabrina
            .wave
            .charging = true;
        app.world_mut()
            .resource_mut::<Simulation>()
            .0
            .sabrina
            .wave
            .charge_time = 2.0;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.world_mut().run_system_once(app::input).unwrap();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Pause);
        assert!(!app.world().resource::<Simulation>().0.sabrina.wave.charging);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.world_mut().run_system_once(app::input).unwrap();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Playing);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.world_mut().run_system_once(app::input).unwrap();
        let input = app.world().resource::<PendingInput>().0;
        let mut simulation = app.world_mut().resource_mut::<Simulation>();
        reef_sim::step(&mut simulation.0, 1.0 / 60.0, input);
        assert!(simulation.0.sabrina.wave.charging);
        assert_eq!(simulation.0.sabrina.wave.cast_id, 0);
    }

    #[test]
    fn unfocused_window_ignores_controller_actions_and_clears_pending_input() {
        let (mut app, pad) = input_app();
        app.world_mut().resource_mut::<Options>().verify = false;
        app.world_mut().resource_mut::<SessionUi>().pointer_mode = true;
        app.world_mut()
            .resource_mut::<Simulation>()
            .0
            .sabrina
            .wave
            .charging = true;
        for mut window in app
            .world_mut()
            .query::<&mut Window>()
            .iter_mut(app.world_mut())
        {
            window.focused = false;
        }
        app.world_mut().resource_mut::<PendingInput>().0.cast = true;
        button(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        assert_eq!(app.world().resource::<SessionUi>().screen, Screen::Menu);
        assert!(!app.world().resource::<PendingInput>().0.cast);
        assert!(!app.world().resource::<SessionUi>().pointer_mode);
        assert!(!app.world().resource::<Simulation>().0.sabrina.wave.charging);
    }

    #[test]
    fn confirmation_toggles_once_per_press_not_once_per_held_frame() {
        let (mut app, pad) = input_app();
        for mut hit in app
            .world_mut()
            .query::<&mut Hit>()
            .iter_mut(app.world_mut())
        {
            if hit.index == 0 {
                hit.action = Action::Vsync;
            }
        }
        button(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        assert!(!app.world().resource::<Preferences>().vsync);
        for _ in 0..10 {
            app.update();
            assert!(!app.world().resource::<Preferences>().vsync);
        }
        button(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        for expected in [true, false, true, false, true, false] {
            button(&mut app, pad, GamepadButton::South, 1.0);
            button(&mut app, pad, GamepadButton::South, 0.0);
            app.update();
            assert_eq!(app.world().resource::<Preferences>().vsync, expected);
        }
    }
}
