// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Active-controller selection, positional hints and deliberate menu navigation.
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    #[default]
    Keyboard,
    Xbox,
    PlayStation,
    Generic,
}

#[derive(Clone, Copy)]
pub enum Control {
    Swim,
    Crystal,
    Song,
    Wave,
    Pause,
}

pub struct Binding {
    pub label: &'static str,
    pub color: Color,
}

impl Style {
    pub fn identify(vendor: Option<u16>, name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        if vendor == Some(0x054c)
            || ["playstation", "dualshock", "dualsense", "ps4", "ps5"]
                .iter()
                .any(|part| name.contains(part))
        {
            Self::PlayStation
        } else if vendor == Some(0x045e)
            || ["xbox", "xinput", "x-box"]
                .iter()
                .any(|part| name.contains(part))
        {
            Self::Xbox
        } else {
            Self::Generic
        }
    }

    pub fn binding(self, control: Control) -> Binding {
        let label = match (self, control) {
            (Self::Keyboard, Control::Swim) => "Mouse / WASD",
            (Self::Keyboard, Control::Crystal) => "RMB / Space",
            (Self::Keyboard, Control::Song) => "Q",
            (Self::Keyboard, Control::Wave) => "LMB / F",
            (Self::Keyboard, Control::Pause) => "Esc",
            (_, Control::Swim) => "Left stick",
            (Self::Xbox, Control::Crystal) => "X",
            (Self::Xbox, Control::Song) => "B",
            (Self::Xbox, Control::Wave) => "A",
            (Self::PlayStation, Control::Crystal) => "□",
            (Self::PlayStation, Control::Song) => "○",
            (Self::PlayStation, Control::Wave) => "×",
            (Self::PlayStation, Control::Pause) => "Options",
            (Self::Generic, Control::Crystal) => "West",
            (Self::Generic, Control::Song) => "East",
            (Self::Generic, Control::Wave) => "South",
            (_, Control::Pause) => "Start",
        };
        let color = match (self, control) {
            (Self::Xbox, Control::Wave) => Color::srgb(0.45, 1.0, 0.4),
            (Self::Xbox | Self::PlayStation, Control::Song) => Color::srgb(1.0, 0.4, 0.4),
            (Self::Xbox, Control::Crystal) | (Self::PlayStation, Control::Wave) => {
                Color::srgb(0.4, 0.75, 1.0)
            }
            (Self::PlayStation, Control::Crystal) => Color::srgb(1.0, 0.55, 0.85),
            _ => Color::srgb(0.87, 0.97, 1.0),
        };
        Binding { label, color }
    }
}

#[derive(Resource, Default, PartialEq, Eq)]
pub struct Hints {
    pub style: Style,
    pub active: Option<Entity>,
}

pub fn update(
    mut hints: ResMut<Hints>,
    pads: Query<(Entity, &Gamepad, Option<&Name>)>,
    mut deflected: Local<std::collections::HashSet<Entity>>,
) {
    let selected = pads
        .iter()
        .filter(|(_, pad, _)| pad.get_just_pressed().next().is_some())
        .min_by_key(|(entity, _, _)| entity.to_bits())
        .or_else(|| {
            pads.iter()
                .filter(|(entity, pad, _)| {
                    !deflected.contains(entity) && pad.left_stick().length() > 0.55
                })
                .min_by_key(|(entity, _, _)| entity.to_bits())
        })
        .or_else(|| hints.active.and_then(|entity| pads.get(entity).ok()))
        .or_else(|| pads.iter().min_by_key(|(entity, _, _)| entity.to_bits()));
    let next = selected.map_or_else(Hints::default, |(entity, pad, name)| Hints {
        active: Some(entity),
        style: Style::identify(pad.vendor_id(), name.map_or("", Name::as_str)),
    });
    hints.set_if_neq(next);
    deflected.retain(|entity| {
        pads.get(*entity)
            .is_ok_and(|(_, pad, _)| pad.left_stick().length() > 0.3)
    });
    deflected.extend(
        pads.iter()
            .filter_map(|(entity, pad, _)| (pad.left_stick().length() > 0.55).then_some(entity)),
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn vector(self) -> Vec2 {
        match self {
            Self::Up => Vec2::Y,
            Self::Down => Vec2::NEG_Y,
            Self::Left => Vec2::NEG_X,
            Self::Right => Vec2::X,
        }
    }
}

#[derive(Default)]
pub struct MenuNavigation {
    stick: Option<Direction>,
    repeating: Option<Direction>,
    wait: f32,
    block_stick: bool,
    block_dpad: bool,
}

impl MenuNavigation {
    pub fn reset(&mut self) {
        *self = Self {
            block_stick: true,
            block_dpad: true,
            ..default()
        };
    }

    pub fn update(&mut self, pad: Option<&Gamepad>, dt: f32) -> Option<Direction> {
        let Some(pad) = pad else {
            self.reset();
            return None;
        };
        let buttons = [
            (GamepadButton::DPadUp, Direction::Up),
            (GamepadButton::DPadDown, Direction::Down),
            (GamepadButton::DPadLeft, Direction::Left),
            (GamepadButton::DPadRight, Direction::Right),
        ];
        let mut dpad = Vec2::ZERO;
        let mut fresh = false;
        for (button, direction) in buttons {
            // Include taps whose press and release both arrived in this frame.
            if pad.pressed(button) || pad.just_pressed(button) {
                dpad += direction.vector();
            }
            fresh |= pad.just_pressed(button);
        }
        if dpad == Vec2::ZERO || fresh {
            self.block_dpad = false;
        }
        let stick = pad.left_stick();
        if stick.abs().max_element() <= 0.3 {
            self.block_stick = false;
        }
        self.stick = if self.block_stick {
            None
        } else if self.stick.is_some_and(|dir| stick.dot(dir.vector()) > 0.3) {
            self.stick
        } else {
            Self::direction(stick, 0.55)
        };
        let direction = if dpad != Vec2::ZERO {
            if self.block_dpad {
                None
            } else {
                Self::direction(dpad, 0.5)
            }
        } else {
            self.stick
        };
        if direction != self.repeating || (fresh && direction.is_some()) {
            self.repeating = direction;
            self.wait = 0.4;
            return direction;
        }
        direction?;
        self.wait -= dt;
        if self.wait <= 0.0 {
            // Never catch up with several moves after a slow rendered frame.
            self.wait = 0.14;
            direction
        } else {
            None
        }
    }

    fn direction(value: Vec2, threshold: f32) -> Option<Direction> {
        if value.abs().max_element() < threshold {
            None
        } else if value.y.abs() >= value.x.abs() {
            Some(if value.y > 0.0 {
                Direction::Up
            } else {
                Direction::Down
            })
        } else {
            Some(if value.x > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stick(pad: &mut Gamepad, x: f32, y: f32) {
        pad.analog_mut().set(GamepadAxis::LeftStickX, x);
        pad.analog_mut().set(GamepadAxis::LeftStickY, y);
    }

    #[test]
    fn dpad_taps_repeat_after_delay_and_release_resets_delay() {
        let mut nav = MenuNavigation::default();
        let mut pad = Gamepad::default();
        pad.digital_mut().press(GamepadButton::DPadDown);
        assert_eq!(nav.update(Some(&pad), 0.01), Some(Direction::Down));
        pad.digital_mut().clear();
        assert_eq!(nav.update(Some(&pad), 0.39), None);
        assert_eq!(nav.update(Some(&pad), 0.02), Some(Direction::Down));
        assert_eq!(nav.update(Some(&pad), 0.13), None);
        assert_eq!(nav.update(Some(&pad), 0.02), Some(Direction::Down));
        pad.digital_mut().release(GamepadButton::DPadDown);
        assert_eq!(nav.update(Some(&pad), 0.01), None);
        pad.digital_mut().press(GamepadButton::DPadDown);
        pad.digital_mut().release(GamepadButton::DPadDown);
        assert_eq!(nav.update(Some(&pad), 0.01), Some(Direction::Down));
    }

    #[test]
    fn stick_hysteresis_locks_diagonals_and_dpad_takes_priority() {
        let mut nav = MenuNavigation::default();
        let mut pad = Gamepad::default();
        stick(&mut pad, 0.56, 0.6);
        assert_eq!(nav.update(Some(&pad), 0.01), Some(Direction::Up));
        stick(&mut pad, 0.65, 0.5);
        assert_eq!(nav.update(Some(&pad), 0.1), None);
        assert_eq!(nav.update(Some(&pad), 0.31), Some(Direction::Up));
        pad.digital_mut().press(GamepadButton::DPadLeft);
        assert_eq!(nav.update(Some(&pad), 0.01), Some(Direction::Left));
        pad.digital_mut().reset_all();
        stick(&mut pad, 0.4, 0.4);
        nav = MenuNavigation::default();
        assert_eq!(nav.update(Some(&pad), 0.5), None);
        stick(&mut pad, 0.0, -0.8);
        assert_eq!(nav.update(Some(&pad), 0.01), Some(Direction::Down));
    }

    #[test]
    fn screen_change_requires_release_and_slow_frames_move_only_once() {
        let mut nav = MenuNavigation::default();
        let mut pad = Gamepad::default();
        stick(&mut pad, 0.0, -1.0);
        pad.digital_mut().press(GamepadButton::DPadDown);
        pad.digital_mut().clear();
        nav.reset();
        assert_eq!(nav.update(Some(&pad), 1.0), None);
        pad.digital_mut().reset_all();
        stick(&mut pad, 0.0, 0.0);
        assert_eq!(nav.update(Some(&pad), 0.01), None);
        stick(&mut pad, 0.0, -1.0);
        assert_eq!(nav.update(Some(&pad), 0.01), Some(Direction::Down));
        assert_eq!(nav.update(Some(&pad), 2.0), Some(Direction::Down));
        assert_eq!(nav.update(Some(&pad), 0.01), None);
    }

    #[test]
    fn fresh_button_wins_over_another_controllers_held_stick() {
        let mut app = App::new();
        app.init_resource::<Hints>().add_systems(Update, update);
        let mut held = Gamepad::default();
        stick(&mut held, 1.0, 0.0);
        app.world_mut().spawn(held);
        app.update();
        let mut pressed = Gamepad::default();
        pressed.digital_mut().press(GamepadButton::South);
        let selected = app.world_mut().spawn(pressed).id();
        app.update();
        assert_eq!(app.world().resource::<Hints>().active, Some(selected));
        app.world_mut()
            .get_mut::<Gamepad>(selected)
            .unwrap()
            .digital_mut()
            .clear();
        app.update();
        assert_eq!(app.world().resource::<Hints>().active, Some(selected));
    }

    #[test]
    fn device_identity_and_button_positions_match_expected_layouts() {
        assert_eq!(
            Style::identify(Some(0x054c), "Wireless Controller"),
            Style::PlayStation
        );
        assert_eq!(
            Style::identify(None, "DualSense Wireless Controller"),
            Style::PlayStation
        );
        assert_eq!(Style::identify(Some(0x045e), ""), Style::Xbox);
        assert_eq!(
            Style::identify(None, "Xbox Wireless Controller"),
            Style::Xbox
        );
        assert_eq!(
            Style::identify(None, "Unknown Wireless Controller"),
            Style::Generic
        );
        for (style, labels) in [
            (Style::Keyboard, ["RMB / Space", "Q", "LMB / F"]),
            (Style::Xbox, ["X", "B", "A"]),
            (Style::PlayStation, ["□", "○", "×"]),
            (Style::Generic, ["West", "East", "South"]),
        ] {
            for (control, label) in [Control::Crystal, Control::Song, Control::Wave]
                .into_iter()
                .zip(labels)
            {
                assert_eq!(style.binding(control).label, label);
            }
        }
        assert_ne!(
            Style::Xbox.binding(Control::Crystal).color,
            Style::Xbox.binding(Control::Wave).color
        );
    }

    #[test]
    fn hotplug_activity_and_disconnect_refresh_hints() {
        let mut app = App::new();
        app.init_resource::<Hints>().add_systems(Update, update);
        app.update();
        assert_eq!(app.world().resource::<Hints>().style, Style::Keyboard);
        let xbox = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("Xbox Controller")))
            .id();
        app.update();
        assert_eq!(app.world().resource::<Hints>().style, Style::Xbox);
        let ps = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("DualShock 4")))
            .id();
        app.update();
        assert_eq!(app.world().resource::<Hints>().style, Style::Xbox);
        app.world_mut()
            .get_mut::<Gamepad>(ps)
            .unwrap()
            .digital_mut()
            .press(GamepadButton::South);
        app.update();
        assert_eq!(app.world().resource::<Hints>().style, Style::PlayStation);
        app.world_mut().entity_mut(ps).remove::<Gamepad>();
        app.update();
        assert_eq!(app.world().resource::<Hints>().style, Style::Xbox);
        app.world_mut().despawn(xbox);
        app.update();
        assert_eq!(app.world().resource::<Hints>().style, Style::Keyboard);
    }
}
