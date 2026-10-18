// SPDX-License-Identifier: MIT
use crate::{
    app::{self, Options, PendingInput, Screen, SessionUi, Simulation},
    audio::AudioRequests,
    settings::Preferences,
};
use bevy::{
    prelude::*,
    sprite::{BorderRect, TextureSlicer},
    ui::widget::NodeImageMode,
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};
use std::{collections::HashMap, sync::OnceLock};
fn table() -> &'static HashMap<String, [String; 3]> {
    static T: OnceLock<HashMap<String, [String; 3]>> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("translations.json")).unwrap())
}
pub fn tr(key: &str, lang: usize) -> String {
    table()
        .get(key)
        .map(|v| v[lang % 3].clone())
        .unwrap_or_else(|| key.into())
}
#[derive(Component)]
struct UiRoot;
#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
impl Rect {
    fn contains(self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
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
    Quit,
    Fullscreen,
    Quality,
    Motion,
    Gentle,
    Visitors,
    Done,
    Song,
    Wave,
    Original,
    Replay,
    VoicePause,
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
    stick_wait: f32,
}
pub struct ReefUiPlugin;
impl Plugin for ReefUiPlugin {
    fn build(&self, a: &mut App) {
        a.init_resource::<Focus>()
            .add_systems(PreUpdate, actions.after(crate::app::input))
            .add_systems(
                Update,
                (draw, update).chain().before(crate::presentation::prepare),
            );
    }
}
struct Builder<'a, 'w, 's> {
    c: &'a mut Commands<'w, 's>,
    server: &'a AssetServer,
    root: Entity,
    lang: usize,
    index: usize,
    font: Handle<Font>,
}
impl Builder<'_, '_, '_> {
    fn image(&mut self, r: Rect, name: &str) -> Entity {
        let e = self
            .c
            .spawn((
                ImageNode {
                    image: self.server.load(format!("ui/reef_png/{name}.png")),
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
        self.c.entity(self.root).add_child(e);
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
            .c
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
            self.c.entity(e).insert(Dynamic(key));
        }
        self.c.entity(self.root).add_child(e);
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
        let e = self.image(r, if primary { "button" } else { "button_teal" });
        self.c.entity(e).insert(Hit {
            rect: r,
            action,
            index: self.index,
            slider: false,
        });
        self.index += 1;
        let label = self.label(
            rect(r.x + 12.0, r.y + 12.0, r.w - 24.0, r.h - 16.0),
            20.0,
            tr(text, self.lang),
            None,
        );
        self.c.entity(label).insert((
            TextLayout::justify(Justify::Center),
            ButtonLabel {
                index: self.index - 1,
                primary,
                action,
            },
        ));
        if primary {
            self.c
                .entity(label)
                .insert(TextColor(Color::srgb(0.20, 0.17, 0.18)));
        }
    }
    fn meter(&mut self, r: Rect, color: Color, key: &'static str) {
        let e = self
            .c
            .spawn((r.node(), BackgroundColor(Color::srgb(0.02, 0.13, 0.20))))
            .id();
        self.c.entity(self.root).add_child(e);
        let e = self
            .c
            .spawn((r.node(), BackgroundColor(color), Meter(key, r.w)))
            .id();
        self.c.entity(self.root).add_child(e);
    }
    fn slider(&mut self, r: Rect, amount: f32, action: Action) {
        let e = self
            .c
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
        self.c.entity(self.root).add_child(e);
        let f = self
            .c
            .spawn((
                rect(r.x, r.y, r.w * amount, r.h).node(),
                BackgroundColor(Color::srgb(0.25, 0.87, 0.87)),
            ))
            .id();
        self.c.entity(self.root).add_child(f);
    }
}
fn draw(
    mut c: Commands,
    server: Res<AssetServer>,
    u: Res<SessionUi>,
    p: Res<Preferences>,
    roots: Query<Entity, With<UiRoot>>,
    mut focus: ResMut<Focus>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
    options: Res<Options>,
) {
    let zoom = (window.width() / 1600.0).min(window.height() / 900.0);
    scale.0 = zoom;
    let key = format!(
        "{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        u.screen,
        p.language,
        p.quality,
        p.fullscreen,
        p.reduced_motion,
        p.gentle,
        p.visitors,
        p.rest,
        p.music,
        p.effects,
        p.voice,
        p.best,
        window.width(),
        window.height()
    );
    if key == focus.key {
        return;
    }
    focus.key = key;
    if focus.last_screen != Some(u.screen) {
        focus.index = 0;
        focus.last_screen = Some(u.screen);
    }
    for root in &roots {
        c.entity(root).despawn();
    }
    let root = c
        .spawn((
            UiRoot,
            Node {
                width: px(1600),
                height: px(900),
                position_type: PositionType::Absolute,
                left: px((window.width() / zoom - 1600.0) / 2.0),
                top: px((window.height() / zoom - 900.0) / 2.0),
                ..default()
            },
        ))
        .id();
    let mut b = Builder {
        c: &mut c,
        server: &server,
        root,
        lang: p.language,
        index: 0,
        font: server.load("fonts/DejaVuSans.ttf"),
    };
    let lang = p.language;
    if options
        .capture
        .as_deref()
        .is_some_and(|k| k.starts_with("graphics-") || k.starts_with("pose-") || k == "rig")
    {
        focus.count = 0;
        return;
    }
    if u.screen == Screen::Loading {
        let e =
            b.c.spawn((
                ImageNode::new(server.load("ui/boot.png")),
                rect(0.0, 0.0, 1600.0, 900.0).node(),
            ))
            .id();
        b.c.entity(root).add_child(e);
        b.label(
            rect(120.0, 760.0, 900.0, 60.0),
            24.0,
            tr("Please wait…", lang),
            None,
        );
        focus.count = 0;
        return;
    }
    if u.started {
        b.panel(rect(24.0, 22.0, 490.0, 100.0));
        b.label(rect(48.0, 42.0, 450.0, 32.0), 24.0, "", Some("phase"));
        b.label(rect(48.0, 80.0, 450.0, 28.0), 18.0, "", Some("stats"));
        b.panel(rect(600.0, 22.0, 400.0, 92.0));
        b.image(rect(610.0, 36.0, 56.0, 56.0), "heart");
        b.label(rect(679.0, 37.0, 300.0, 32.0), 20.0, "", Some("health"));
        b.meter(
            rect(679.0, 73.0, 290.0, 18.0),
            Color::srgb(1.0, 0.4, 0.48),
            "health",
        );
        for (x, key, color) in [
            (270.0, "crystal", Color::srgb(0.55, 0.8, 1.0)),
            (630.0, "song", Color::srgb(1.0, 0.58, 0.85)),
            (990.0, "wave", Color::srgb(0.38, 1.0, 0.76)),
        ] {
            b.panel(rect(x, 790.0, 340.0, 70.0));
            b.label(rect(x + 20.0, 807.0, 300.0, 29.0), 17.0, "", Some(key));
            b.meter(rect(x + 20.0, 836.0, 300.0, 10.0), color, key);
        }
        b.image(rect(240.0, 779.0, 58.0, 58.0), "shell");
        b.image(rect(1315.0, 806.0, 58.0, 58.0), "star");
        b.label(rect(210.0,864.0,1180.0,32.0),17.0,tr("Mouse / WASD / stick · swim     Click / Space / A · crystal     Q / X · song     RMB / F / B · wave",lang),None);
        b.label(rect(500.0, 130.0, 600.0, 90.0), 21.0, "", Some("score"));
        b.label(rect(350.0, 700.0, 900.0, 48.0), 23.0, "", Some("notice"));
        b.label(rect(350.0, 748.0, 900.0, 36.0), 19.0, "", Some("caption"));
        if u.screen == Screen::Playing {
            b.button(
                rect(1020.0, 24.0, 135.0, 50.0),
                "Wave · F",
                Action::Wave,
                false,
            );
            b.button(
                rect(1170.0, 24.0, 225.0, 50.0),
                "♪ Sing · Q / X",
                Action::Song,
                false,
            );
            b.button(rect(1415.0, 24.0, 160.0, 50.0), "II", Action::Pause, false);
        }
    }
    match u.screen {
        Screen::Menu => {
            b.panel(rect(64.0, 65.0, 535.0, 775.0));
            b.label(
                rect(100.0, 94.0, 450.0, 30.0),
                17.0,
                tr("A REMAKE BY LUKA DEZULOVIC", lang),
                None,
            );
            b.label(
                rect(100.0, 137.0, 460.0, 170.0),
                44.0,
                tr("Sabrina's\nUnderwater\nFairy Tale", lang),
                None,
            );
            b.label(rect(100.0,340.0,450.0,76.0),21.0,tr("A living sea. A mischievous jellyfish.\nAnd a little mermaid with a magic crystal.",lang),None);
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
                    ["English", "Hrvatski", "Italiano"][lang],
                    Action::Language,
                    false,
                ),
                (
                    rect(100.0, 742.0, 220.0, 48.0),
                    "Goodbye",
                    Action::Quit,
                    false,
                ),
            ] {
                b.button(r, key, action, primary);
            }
            b.label(
                rect(340.0, 742.0, 220.0, 48.0),
                17.0,
                format!("{}{}", tr("Best visit: ", lang), p.best),
                None,
            );
        }
        Screen::Pause => {
            b.panel(rect(500.0, 190.0, 600.0, 510.0));
            b.label(
                rect(550.0, 230.0, 500.0, 65.0),
                40.0,
                tr("A little pause", lang),
                None,
            );
            b.label(
                rect(550.0, 305.0, 500.0, 45.0),
                21.0,
                tr("Your fish will wait for you.", lang),
                None,
            );
            b.button(
                rect(550.0, 385.0, 500.0, 60.0),
                "Continue swimming",
                Action::Pause,
                true,
            );
            b.button(
                rect(550.0, 465.0, 500.0, 60.0),
                "Settings",
                Action::Settings,
                false,
            );
            b.button(
                rect(550.0, 545.0, 500.0, 60.0),
                "Return to the beginning",
                Action::Home,
                false,
            );
        }
        Screen::Settings => {
            b.panel(rect(270.0, 70.0, 1060.0, 760.0));
            b.label(
                rect(315.0, 103.0, 970.0, 60.0),
                36.0,
                tr("Make yourself at home", lang),
                None,
            );
            b.button(
                rect(315.0, 190.0, 450.0, 52.0),
                &format!(
                    "{}{}",
                    tr("Fullscreen: ", lang),
                    tr(if p.fullscreen { "on" } else { "off" }, lang)
                ),
                Action::Fullscreen,
                false,
            );
            b.button(
                rect(815.0, 190.0, 465.0, 52.0),
                &format!(
                    "{}{}",
                    tr("Graphics: ", lang),
                    tr(["Light", "Balanced", "High"][p.quality as usize], lang)
                ),
                Action::Quality,
                false,
            );
            b.button(
                rect(315.0, 260.0, 450.0, 52.0),
                if p.reduced_motion {
                    "Motion: reduced"
                } else {
                    "Motion: natural"
                },
                Action::Motion,
                false,
            );
            b.button(
                rect(815.0, 260.0, 465.0, 52.0),
                if p.gentle {
                    "Gentle play"
                } else {
                    "Lively play"
                },
                Action::Gentle,
                false,
            );
            for (y, key, value, action) in [
                (345.0, "Music", p.music, Action::Music),
                (445.0, "Magic & sound effects", p.effects, Action::Effects),
                (545.0, "Original voices", p.voice, Action::Voice),
            ] {
                b.label(rect(315.0, y, 450.0, 35.0), 22.0, tr(key, lang), None);
                b.slider(rect(315.0, y + 48.0, 400.0, 24.0), value, action);
            }
            b.label(
                rect(815.0, 345.0, 465.0, 35.0),
                22.0,
                format!("{}{:.0} s", tr("Quiet time: ", lang), p.rest),
                None,
            );
            b.slider(
                rect(815.0, 393.0, 400.0, 24.0),
                (p.rest - 25.0) / 125.0,
                Action::Rest,
            );
            b.button(
                rect(815.0, 465.0, 465.0, 58.0),
                &format!(
                    "{}{}{}",
                    tr("At most ", lang),
                    p.visitors,
                    tr(" jellyfish", lang)
                ),
                Action::Visitors,
                false,
            );
            b.button(rect(815.0, 725.0, 465.0, 58.0), "Done", Action::Done, true);
        }
        Screen::Credits | Screen::Help => {
            b.panel(rect(230.0, 60.0, 1140.0, 780.0));
            let help = u.screen == Screen::Help;
            b.label(
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
                table()
                    .iter()
                    .find(|(k, _)| k.starts_with("Swim with mouse, WASD"))
                    .map(|(_, v)| v[lang].clone())
                    .unwrap_or_default()
            } else {
                [
                    include_str!("../../../assets/credits/credits_en.txt"),
                    include_str!("../../../assets/credits/credits_hr.txt"),
                    include_str!("../../../assets/credits/credits_it.txt"),
                ][lang]
                    .into()
            };
            b.label(
                rect(280.0, 180.0, 1040.0, 540.0),
                if help { 18.0 } else { 23.0 },
                text,
                None,
            );
            if help {
                b.button(
                    rect(280.0, 745.0, 310.0, 52.0),
                    "Listen again",
                    Action::Replay,
                    false,
                );
                b.button(
                    rect(610.0, 745.0, 310.0, 52.0),
                    "Pause / resume voice",
                    Action::VoicePause,
                    false,
                );
            } else {
                b.button(
                    rect(280.0, 745.0, 640.0, 52.0),
                    "Play the original on Scratch ↗",
                    Action::Original,
                    false,
                );
            }
            b.button(rect(940.0, 745.0, 380.0, 52.0), "Back", Action::Done, true);
        }
        Screen::GameOver => {
            b.panel(rect(440.0, 210.0, 720.0, 470.0));
            b.image(rect(746.0, 240.0, 108.0, 90.0), "heart");
            b.label(
                rect(485.0, 332.0, 630.0, 62.0),
                38.0,
                tr("Game over", lang),
                None,
            );
            b.label(
                rect(490.0, 405.0, 620.0, 70.0),
                21.0,
                tr("Sabrina needs a rest. Try again when you are ready.", lang),
                None,
            );
            b.button(
                rect(490.0, 500.0, 620.0, 62.0),
                "Swim again",
                Action::Play,
                true,
            );
            b.button(
                rect(490.0, 586.0, 620.0, 56.0),
                "Back to our reef",
                Action::Home,
                false,
            );
        }
        _ => {}
    }
    focus.count = b.index;
    focus.index = focus.index.min(focus.count.saturating_sub(1));
}
fn actions(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    hits: Query<&Hit>,
    mut focus: ResMut<Focus>,
    mut u: ResMut<SessionUi>,
    mut w: ResMut<Simulation>,
    mut p: ResMut<Preferences>,
    mut pending: ResMut<PendingInput>,
    mut audio: ResMut<AudioRequests>,
    options: Res<Options>,
    mut exit: MessageWriter<AppExit>,
    time: Res<Time>,
) {
    if options.capture.is_some() {
        return;
    }
    let zoom = (window.width() / 1600.0).min(window.height() / 900.0);
    let pointer = window.cursor_position().map(|pos| {
        pos / zoom
            - Vec2::new(
                (window.width() / zoom - 1600.0) / 2.0,
                (window.height() / zoom - 900.0) / 2.0,
            )
    });
    let mut next = keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::ArrowDown);
    let mut prev = keys.just_pressed(KeyCode::ArrowUp);
    let mut left = keys.just_pressed(KeyCode::ArrowLeft);
    let mut right = keys.just_pressed(KeyCode::ArrowRight);
    focus.stick_wait = (focus.stick_wait - time.delta_secs()).max(0.0);
    let mut stick_active = false;
    let mut activate = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    for pad in &pads {
        next |= pad.just_pressed(GamepadButton::DPadDown);
        prev |= pad.just_pressed(GamepadButton::DPadUp);
        activate |= pad.just_pressed(GamepadButton::South);
        left |= pad.just_pressed(GamepadButton::DPadLeft);
        right |= pad.just_pressed(GamepadButton::DPadRight);
        let stick = pad.left_stick();
        stick_active |= stick.length() > 0.55;
        if stick.length() > 0.55 && focus.stick_wait <= 0.0 && u.screen != Screen::Playing {
            if stick.y.abs() > stick.x.abs() {
                next |= stick.y < -0.55;
                prev |= stick.y > 0.55;
            } else {
                left |= stick.x < -0.55;
                right |= stick.x > 0.55;
            }
            focus.stick_wait = 0.25;
        }
    }
    if !stick_active {
        focus.stick_wait = 0.0;
    }
    if u.screen == Screen::Playing {
        activate = false;
        next = false;
        prev = false;
    }
    if focus.count > 0 {
        if next {
            focus.index = (focus.index + 1) % focus.count;
        }
        if prev {
            focus.index = (focus.index + focus.count - 1) % focus.count;
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
            if hit.slider && (left || right) {
                let delta = if right { 0.05 } else { -0.05 };
                fraction = Some(
                    (match hit.action {
                        Action::Music => p.music,
                        Action::Effects => p.effects,
                        Action::Voice => p.voice,
                        Action::Rest => (p.rest - 25.0) / 125.0,
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
            app::begin(&mut w.0, &mut u, &p, false);
            audio.stop_voice = true;
        }
        Action::Aquarium => {
            app::begin(&mut w.0, &mut u, &p, true);
            audio.stop_voice = true;
        }
        Action::Home => {
            app::home(&mut w.0, &mut u, &p);
            audio.stop_voice = true;
        }
        Action::Pause => app::pause(&mut u),
        Action::Settings | Action::Credits | Action::Help => {
            u.return_to = if u.started {
                Screen::Pause
            } else {
                Screen::Menu
            };
            u.screen = match action {
                Action::Settings => Screen::Settings,
                Action::Credits => Screen::Credits,
                _ => Screen::Help,
            };
            if matches!(action, Action::Help) {
                audio.help = Some(p.language)
            } else {
                audio.stop_voice = true;
            }
        }
        Action::Done => {
            u.screen = u.return_to;
            audio.stop_voice = true;
            p.save();
        }
        Action::Language => p.language = (p.language + 1) % 3,
        Action::Quit => {
            p.save();
            exit.write(AppExit::Success);
        }
        Action::Fullscreen => {
            p.fullscreen = !p.fullscreen;
            window.mode = if p.fullscreen {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            } else {
                WindowMode::Windowed
            }
        }
        Action::Quality => p.quality = (p.quality + 1) % 3,
        Action::Motion => p.reduced_motion = !p.reduced_motion,
        Action::Gentle => {
            p.gentle = !p.gentle;
            w.0.settings.gentle = p.gentle
        }
        Action::Visitors => {
            p.visitors = p.visitors % 3 + 1;
            w.0.settings.visitor_limit = p.visitors
        }
        Action::Song => pending.0.toggle_song = true,
        Action::Wave => pending.0.wave = true,
        Action::Replay => audio.help = Some(p.language),
        Action::VoicePause => audio.pause_voice = true,
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
                p.music = f
            }
        }
        Action::Effects => {
            if let Some(f) = fraction {
                p.effects = f
            }
        }
        Action::Voice => {
            if let Some(f) = fraction {
                p.voice = f
            }
        }
        Action::Rest => {
            if let Some(f) = fraction {
                p.rest = (25.0 + 125.0 * f).round();
                w.0.settings.rest_seconds = p.rest;
            }
        }
    }
    if !matches!(action, Action::Song | Action::Wave | Action::Pause) {
        p.save();
    }
}
fn update(
    w: Res<Simulation>,
    u: Res<SessionUi>,
    p: Res<Preferences>,
    mut texts: Query<(&Dynamic, &mut Text), Without<ButtonLabel>>,
    mut labels: Query<(&ButtonLabel, &mut TextColor, &mut Text), Without<Dynamic>>,
    mut bars: Query<(&Meter, &mut Node)>,
    focus: Res<Focus>,
    mut hits: Query<(&Hit, &mut ImageNode)>,
    server: Res<AssetServer>,
) {
    let s = &w.0.sabrina;
    let l = p.language;
    for (key, mut text) in &mut texts {
        let value = match key.0 {
            "phase" => tr(
                if w.0.session.water == reef_sim::Water::Visit {
                    "A jellyfish is visiting!"
                } else {
                    "A peaceful little sea"
                },
                l,
            ),
            "stats" => format!(
                "{}{}{}",
                reef_sim::alive(&w.0),
                tr(" fish   ·   ", l),
                if w.0.session.water == reef_sim::Water::Visit {
                    format!("{}{}", tr("Score ", l), w.0.points.score)
                } else if w.0.settings.aquarium {
                    tr("Take your time", l)
                } else {
                    format!(
                        "{}{} s",
                        tr("Next visit in ", l),
                        w.0.session.rest_remaining as i32 + 1
                    )
                }
            ),
            "health" => format!(
                "{}{} / 100",
                tr("Health · ", l),
                s.vitality.health.ceil() as i32
            ),
            "crystal" => format!(
                "{}{}",
                tr("Crystal · ", l),
                if s.cooldown > 0.0 {
                    let n = s.cooldown.ceil() as i32;
                    format!("{}:{:02}", n / 60, n % 60)
                } else {
                    tr("ready", l)
                }
            ),
            "song" => format!(
                "{}{}%{}",
                tr("Song · ", l),
                s.song.energy as i32,
                if s.song.locked {
                    tr(" · recharging", l)
                } else if s.song.active {
                    format!(" · {:.1} m", reef_sim::song_radius(&w.0))
                } else {
                    String::new()
                }
            ),
            "wave" => format!(
                "{}{}",
                tr("Wave · ", l),
                if s.wave.cooldown > 0.0 {
                    format!("{:.1} s", s.wave.cooldown)
                } else {
                    tr("ready · F / B", l)
                }
            ),
            "caption" => u.caption.clone(),
            "notice" => {
                if u.notice_time > 0.0 {
                    u.notice.clone()
                } else {
                    String::new()
                }
            }
            "score" => {
                if u.score_time > 0.0 {
                    format!(
                        "{}\n+{}{}{}{}",
                        tr("The sea is peaceful again", l),
                        w.0.points.last_score,
                        tr(" points · ", l),
                        w.0.points.last_survivors,
                        tr(" fish safe", l)
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
            "health" => s.vitality.health / 100.0,
            "song" => s.song.energy / 100.0,
            "crystal" => 1.0 - s.cooldown / reef_sim::recharge_duration(&w.0),
            "wave" => 1.0 - s.wave.cooldown / reef_sim::wave_recharge_duration(&w.0),
            _ => 0.0,
        };
        let width = px(meter.1 * amount.clamp(0.0, 1.0));
        if node.width != width {
            node.width = width;
        }
    }
    for (label, mut color, mut text) in &mut labels {
        let disabled = match label.action {
            Action::Song => s.song.locked,
            Action::Wave => {
                s.wave.cooldown > 0.0 || w.0.crystal.magic.state == reef_sim::Spell::Charging
            }
            _ => false,
        };
        let desired_color = if disabled {
            Color::srgb(0.5, 0.65, 0.66)
        } else if label.primary || label.index == focus.index {
            Color::srgb(0.20, 0.17, 0.18)
        } else {
            Color::srgb(0.87, 0.97, 1.0)
        };
        color.set_if_neq(TextColor(desired_color));
        if matches!(label.action, Action::Song) {
            let value = tr(
                if s.song.locked {
                    "Song recharging…"
                } else if s.song.active {
                    "♪ Stop singing · Q / X"
                } else {
                    "♪ Sing · Q / X"
                },
                l,
            );
            if text.0 != value {
                text.0 = value;
            }
        }
    }
    for (hit, mut image) in &mut hits {
        let file = if hit.index == focus.index {
            "button_hover"
        } else if matches!(hit.action, Action::Play | Action::Done | Action::Pause)
            && u.screen != Screen::Playing
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
