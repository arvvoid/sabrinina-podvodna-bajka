// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Localized screens, HUD updates and shared pointer/keyboard/controller navigation.
use crate::{
    app::{self, Options, PendingInput, Screen, SessionUi, Simulation},
    audio::AudioRequests,
    controls::{Control, Direction, Hints, MenuNavigation, Style},
    settings::Preferences,
};
use bevy::{
    prelude::*,
    render::renderer::RenderAdapterInfo,
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
    let height = viewport_height.max(viewport_width / (1365.0 / 768.0));
    let width = height * (1365.0 / 768.0);
    rect(
        (CANVAS_WIDTH - width) / 2.0,
        ((CANVAS_HEIGHT - height) / 2.0)
            .min((CANVAS_HEIGHT + viewport_height) / 2.0 - 24.0 - height * 733.0 / 768.0),
        width,
        height,
    )
}

fn on_loading_image(image: Rect, area: Rect) -> Rect {
    let scale = image.w / 1365.0;
    rect(
        image.x + area.x * scale,
        image.y + area.y * scale,
        area.w * scale,
        area.h * scale,
    )
}

fn loading_overlay(image: Rect) -> (Rect, Rect) {
    (
        on_loading_image(image, rect(422.5, 658.0, 520.0, 24.0)),
        on_loading_image(image, rect(534.5, 708.0, 296.0, 25.0)),
    )
}

fn loading_bar_fraction(progress: f32, right: bool) -> f32 {
    (progress * 2.0 - if right { 1.0 } else { 0.0 }).clamp(0.0, 1.0)
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
    Gentle,
    Visitors,
    Done,
    Song,
    Wave,
    Original,
    Music,
    Effects,
    Voice,
    Rest,
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
            .init_resource::<Hints>()
            .add_systems(Startup, setup_fps)
            .add_systems(Update, update_fps)
            .add_systems(Update, update_loading_stats.after(draw))
            .add_systems(
                PreUpdate,
                crate::controls::update
                    .after(bevy::input::InputSystems)
                    .before(crate::app::input),
            )
            .add_systems(PreUpdate, actions.after(crate::app::input))
            .add_systems(
                Update,
                (draw, update).chain().before(crate::presentation::prepare),
            );
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
) {
    let zoom = (window.width() / CANVAS_WIDTH).min(window.height() / CANVAS_HEIGHT);
    scale.0 = zoom;
    // Rebuild only when layout or settings change; HUD values update separately.
    let key = format!(
        "{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{:?}:{}:{}",
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
        preferences.rumble
    );
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
        let (status_area, _) = loading_overlay(image);
        let e = builder
            .commands
            .spawn((
                ImageNode {
                    image: server.load("ui/loading.png"),
                    ..default()
                },
                image.node(),
            ))
            .id();
        builder.commands.entity(root).add_child(e);
        let shadow = TextShadow {
            offset: Vec2::splat(2.0 / zoom),
            color: Color::BLACK,
        };
        let text_color = TextColor(Color::srgb(1.0, 0.92, 0.60));
        let status = builder.label(status_area, 20.0 * image.w / 1365.0, "", Some("loading"));
        builder.commands.entity(status).insert((
            TextLayout::justify(Justify::Center),
            text_color,
            shadow,
        ));
        // Fill only the two narrow inlays, leaving the gold frame and shell visible.
        for (x, key) in [(548.0, "loading_left"), (710.0, "loading_right")] {
            let inlay = on_loading_image(image, rect(x, 719.0, 107.0, 2.0));
            builder.meter(inlay, Color::srgb(0.60, 0.98, 0.91), key);
        }
        let stats_scale = (0.8 / zoom).max(1.0);
        let stats_width = (600.0 * stats_scale).min(viewport_width - 48.0 * stats_scale);
        let stats = builder.label(
            rect(
                (CANVAS_WIDTH + viewport_width) / 2.0 - stats_width - 24.0 * stats_scale,
                (CANVAS_HEIGHT - viewport_height) / 2.0 + 20.0 * stats_scale,
                stats_width,
                300.0 * stats_scale,
            ),
            18.0 * stats_scale,
            "",
            None,
        );
        builder.commands.entity(stats).insert((
            LoadingStats,
            text_color,
            shadow,
            TextLayout::justify(Justify::Right),
        ));
        focus.count = 0;
        return;
    }
    if ui.started && !matches!(ui.screen, Screen::Victory | Screen::GameOver) {
        draw_hud(&mut builder, ui.screen);
    }
    match ui.screen {
        Screen::Menu => draw_menu(&mut builder, &preferences, renderer_label(&adapter)),
        Screen::Pause => draw_pause(&mut builder),
        Screen::Settings => draw_settings(&mut builder, &preferences),
        Screen::Credits | Screen::Help => draw_help_or_credits(&mut builder, ui.screen),
        Screen::GameOver => draw_game_over(&mut builder),
        Screen::Victory => draw_victory(&mut builder),
        Screen::Language => draw_language(&mut builder),
        _ => {}
    }
    focus.count = builder.index;
    focus.index = focus.index.min(focus.count.saturating_sub(1));
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
    let value = format!(
        "{} ({}) · {} × {} px\n{}: {}\n{} · {} draw\nGPU: {}\n{}: {} {}\n{}: {} + {}\n{}: {} · {} ({}): {}\n{}: {}/{} · {}: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        window.physical_width(),
        window.physical_height(),
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

fn draw_hud(builder: &mut Builder<'_, '_, '_>, screen: Screen) {
    builder.background_alpha = HUD_BACKGROUND_ALPHA;
    builder.panel(rect(24.0, 22.0, 490.0, 160.0));
    builder.label(rect(48.0, 42.0, 450.0, 32.0), 24.0, "", Some("phase"));
    builder.label(rect(48.0, 80.0, 450.0, 28.0), 18.0, "", Some("stats"));
    builder.label(rect(48.0, 116.0, 450.0, 50.0), 17.0, "", Some("visitors"));
    builder.panel(rect(600.0, 22.0, 400.0, 92.0));
    builder.image(rect(610.0, 36.0, 56.0, 56.0), "heart");
    builder.label(rect(679.0, 37.0, 300.0, 32.0), 20.0, "", Some("health"));
    builder.meter(
        rect(679.0, 73.0, 290.0, 18.0),
        Color::srgb(1.0, 0.4, 0.48),
        "health",
    );
    for (x, key, color) in [
        (270.0, "crystal", Color::srgb(0.55, 0.8, 1.0)),
        (630.0, "song", Color::srgb(1.0, 0.58, 0.85)),
        (990.0, "wave", Color::srgb(0.38, 1.0, 0.76)),
    ] {
        builder.panel(rect(x, 790.0, 340.0, 70.0));
        builder.label(rect(x + 20.0, 807.0, 300.0, 29.0), 17.0, "", Some(key));
        builder.meter(rect(x + 20.0, 836.0, 300.0, 10.0), color, key);
    }
    builder.image(rect(240.0, 779.0, 58.0, 58.0), "shell");
    builder.image(rect(1315.0, 806.0, 58.0, 58.0), "star");
    builder.control_hints(rect(40.0, 864.0, 1520.0, 32.0), 17.0);
    builder.label(rect(500.0, 130.0, 600.0, 90.0), 21.0, "", Some("score"));
    builder.label(rect(500.0, 225.0, 600.0, 160.0), 22.0, "", Some("reward"));
    builder.label(rect(350.0, 700.0, 900.0, 48.0), 23.0, "", Some("notice"));
    builder.label(rect(350.0, 748.0, 900.0, 36.0), 19.0, "", Some("caption"));
    if screen == Screen::Playing {
        builder.button(rect(1020.0, 24.0, 135.0, 50.0), "Wave", Action::Wave, false);
        builder.button(rect(1170.0, 24.0, 225.0, 50.0), "Sing", Action::Song, false);
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
        rect(315.0, 103.0, 970.0, 60.0),
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
) {
    if options.capture.is_some() {
        return;
    }
    if !window.focused && !options.verify {
        focus.navigation.reset();
        pending.0 = reef_sim::Input::default();
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
        Action::Pause => app::pause(&mut ui),
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
            ui.screen = ui.return_to;
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
        Action::Gentle => {
            preferences.gentle = !preferences.gentle;
            simulation.0.settings.gentle = preferences.gentle
        }
        Action::Visitors => {
            preferences.visitors = preferences.visitors % 3 + 1;
            simulation.0.settings.visitor_limit = preferences.visitors
        }
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
    if !matches!(action, Action::Song | Action::Wave | Action::Pause) {
        preferences.save();
    }
}

fn update(
    simulation: Res<Simulation>,
    ui: Res<SessionUi>,
    preferences: Res<Preferences>,
    mut texts: Query<(&Dynamic, &mut Text), Without<ButtonLabel>>,
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
    for (key, mut text) in &mut texts {
        let value = match key.0 {
            "loading" => format!(
                "{}  {:.0}%",
                tr(
                    if loading.progress == 1.0 {
                        "Ready!"
                    } else if loading.warming {
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
            "visitors" => {
                let health: Vec<String> = simulation
                    .0
                    .jelly
                    .iter()
                    .filter(|j| {
                        matches!(
                            j.behaviour.state,
                            reef_sim::JellyState::Hungry | reef_sim::JellyState::Arriving
                        )
                    })
                    .map(|j| {
                        format!(
                            "{:.0}/100{}",
                            j.behaviour.health,
                            if j.behaviour.bubble > 0.0 { " ○" } else { "" }
                        )
                    })
                    .collect();
                if health.is_empty() {
                    String::new()
                } else {
                    format!("{}{}", tr("Visitors · ", language), health.join("   "))
                }
            }
            "reward" => [
                (reef_sim::ScoreFeedback::Crystal, "Crystal hit"),
                (reef_sim::ScoreFeedback::Wave, "Close wave hit"),
                (reef_sim::ScoreFeedback::Rescue, "Fish rescued"),
                (reef_sim::ScoreFeedback::Retreat, "Visitor repelled"),
                (reef_sim::ScoreFeedback::Survivors, "Survivor bonus"),
            ]
            .into_iter()
            .filter_map(|(kind, label)| {
                let reward = simulation.0.points.rewards[kind as usize];
                (reward.time > 0.0).then(|| format!("+{} · {}", reward.points, tr(label, language)))
            })
            .collect::<Vec<_>>()
            .join("\n"),
            "defeat" => tr(
                if reef_sim::alive(&simulation.0) == 0 {
                    "All fish were lost. Try again and protect the school!"
                } else {
                    "Sabrina needs a rest. Try again when you are ready."
                },
                language,
            ),
            "stats" => format!(
                "{}{}{}",
                reef_sim::alive(&simulation.0),
                tr(" fish   ·   ", language),
                if simulation.0.session.water == reef_sim::Water::Visit {
                    format!("{}{}", tr("Score ", language), simulation.0.points.score)
                } else if simulation.0.settings.aquarium {
                    tr("Take your time", language)
                } else {
                    format!(
                        "{}{} s",
                        tr("Next visit in ", language),
                        simulation.0.session.rest_remaining as i32 + 1
                    )
                }
            ),
            "health" => format!(
                "{}{} / 100",
                tr("Health · ", language),
                sabrina.vitality.health.ceil() as i32
            ),
            "crystal" => format!(
                "{}{}",
                tr("Crystal · ", language),
                if sabrina.cooldown > 0.0 {
                    let n = sabrina.cooldown.ceil() as i32;
                    format!("{}:{:02}", n / 60, n % 60)
                } else {
                    tr("ready", language)
                }
            ),
            "song" => format!(
                "{}{}%{}",
                tr("Song · ", language),
                sabrina.song.energy as i32,
                if sabrina.song.locked {
                    tr(" · recharging", language)
                } else if sabrina.song.active {
                    format!(" · {:.1} m", reef_sim::song_radius(&simulation.0))
                } else {
                    String::new()
                }
            ),
            "wave" => format!(
                "{}{}",
                tr("Wave · ", language),
                if sabrina.wave.cooldown > 0.0 {
                    format!("{:.1} s", sabrina.wave.cooldown)
                } else {
                    tr("ready", language)
                }
            ),
            "caption" => ui.caption.clone(),
            "notice" => {
                if ui.notice_time > 0.0 {
                    ui.notice.clone()
                } else {
                    String::new()
                }
            }
            "score" => {
                if ui.score_time > 0.0 {
                    format!(
                        "{}\n{}{}{}{}",
                        tr("The sea is peaceful again", language),
                        simulation.0.points.last_score,
                        tr(" points · ", language),
                        simulation.0.points.last_survivors,
                        tr(" fish safe", language)
                    )
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
            "loading_left" => loading_bar_fraction(loading.progress, false),
            "loading_right" => loading_bar_fraction(loading.progress, true),
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
    fn title_artwork_matches_translation_language_order() {
        assert_eq!(title_path(0), "ui/title_eng.png");
        assert_eq!(title_path(1), "ui/title_hr.png");
        assert_eq!(title_path(2), "ui/title_it.png");
    }

    #[test]
    fn loading_bar_follows_artwork_and_stays_visible_on_wide_and_tall_windows() {
        for (width, height) in [(1280.0_f32, 720.0_f32), (2560.0, 1080.0), (720.0, 1280.0)] {
            let zoom = (width / CANVAS_WIDTH).min(height / CANVAS_HEIGHT);
            let image = loading_image_rect(width / zoom, height / zoom);
            let (status, bar) = loading_overlay(image);
            assert!((bar.x + bar.w / 2.0 - CANVAS_WIDTH / 2.0).abs() < 1.0);
            assert!(bar.y + bar.h <= (CANVAS_HEIGHT + height / zoom) / 2.0 - 23.99);
            assert!(bar.y > status.y + status.h);
            assert!((bar.w / image.w - 296.0 / 1365.0).abs() < 0.00001);
        }
        let image = loading_image_rect(CANVAS_WIDTH, CANVAS_HEIGHT);
        let (_, bar) = loading_overlay(image);
        assert!((bar.y - (image.y + 708.0 * image.w / 1365.0)).abs() < 0.001);
    }

    #[test]
    fn loading_inlays_fill_in_sequence_without_covering_the_shell() {
        for (progress, left, right) in [
            (0.0, 0.0, 0.0),
            (0.25, 0.5, 0.0),
            (0.5, 1.0, 0.0),
            (0.75, 1.0, 0.5),
            (1.0, 1.0, 1.0),
        ] {
            assert_eq!(loading_bar_fraction(progress, false), left);
            assert_eq!(loading_bar_fraction(progress, true), right);
        }
        assert_eq!(loading_bar_fraction(-1.0, false), 0.0);
        assert_eq!(loading_bar_fraction(2.0, true), 1.0);
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

    fn button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
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
        button(&mut app, pad, GamepadButton::West, 1.0);
        app.update();
        assert!(app.world().resource::<PendingInput>().0.toggle_song);
    }

    #[test]
    fn unfocused_window_ignores_controller_actions_and_clears_pending_input() {
        let (mut app, pad) = input_app();
        app.world_mut().resource_mut::<Options>().verify = false;
        app.world_mut().resource_mut::<SessionUi>().pointer_mode = true;
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
