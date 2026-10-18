// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Connected-device hints; gameplay keeps using Bevy's positional button mapping.
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
            (Self::Keyboard, Control::Crystal) => "Click / Space",
            (Self::Keyboard, Control::Song) => "Q",
            (Self::Keyboard, Control::Wave) => "RMB / F",
            (Self::Keyboard, Control::Pause) => "Esc",
            (_, Control::Swim) => "Left stick",
            (Self::Xbox, Control::Crystal) => "A",
            (Self::Xbox, Control::Song) => "X",
            (Self::Xbox, Control::Wave) => "B",
            (Self::PlayStation, Control::Crystal) => "×",
            (Self::PlayStation, Control::Song) => "□",
            (Self::PlayStation, Control::Wave) => "○",
            (Self::PlayStation, Control::Pause) => "Options",
            (Self::Generic, Control::Crystal) => "South",
            (Self::Generic, Control::Song) => "West",
            (Self::Generic, Control::Wave) => "East",
            (_, Control::Pause) => "Start",
        };
        let color = match (self, control) {
            (Self::Xbox, Control::Crystal) => Color::srgb(0.45, 1.0, 0.4),
            (Self::Xbox | Self::PlayStation, Control::Wave) => Color::srgb(1.0, 0.4, 0.4),
            (Self::Xbox, Control::Song) | (Self::PlayStation, Control::Crystal) => {
                Color::srgb(0.4, 0.75, 1.0)
            }
            (Self::PlayStation, Control::Song) => Color::srgb(1.0, 0.55, 0.85),
            _ => Color::srgb(0.87, 0.97, 1.0),
        };
        Binding { label, color }
    }
}

#[derive(Resource, Default, PartialEq, Eq)]
pub struct Hints {
    pub style: Style,
    active: Option<Entity>,
}

pub fn update(mut hints: ResMut<Hints>, pads: Query<(Entity, &Gamepad, Option<&Name>)>) {
    let selected = pads
        .iter()
        .filter(|(_, pad, _)| {
            pad.get_just_pressed().next().is_some() || pad.left_stick().length() > 0.2
        })
        .min_by_key(|(entity, _, _)| entity.to_bits())
        .or_else(|| hints.active.and_then(|entity| pads.get(entity).ok()))
        .or_else(|| pads.iter().min_by_key(|(entity, _, _)| entity.to_bits()));
    let next = selected.map_or_else(Hints::default, |(entity, pad, name)| Hints {
        active: Some(entity),
        style: Style::identify(pad.vendor_id(), name.map_or("", Name::as_str)),
    });
    hints.set_if_neq(next);
}

#[cfg(test)]
mod tests {
    use super::*;

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
            (Style::Keyboard, ["Click / Space", "Q", "RMB / F"]),
            (Style::Xbox, ["A", "X", "B"]),
            (Style::PlayStation, ["×", "□", "○"]),
            (Style::Generic, ["South", "West", "East"]),
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
