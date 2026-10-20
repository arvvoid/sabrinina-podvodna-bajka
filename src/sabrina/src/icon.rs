// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Native Windows/Linux window icons; macOS uses the packaged app bundle icon.
#[cfg(not(target_os = "macos"))]
use bevy::{
    ecs::system::NonSendMarker,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    window::WindowCreated,
    winit::WINIT_WINDOWS,
};

#[cfg(not(target_os = "macos"))]
fn window_icon() -> winit::window::Icon {
    let image = Image::from_buffer(
        include_bytes!("../../../assets/ui/app_icon.png"),
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::default(),
        default(),
    )
    .expect("Cannot decode the embedded window icon");
    let size = image.size();
    winit::window::Icon::from_rgba(
        image.data.expect("Window icon must have CPU pixel data"),
        size.x,
        size.y,
    )
    .expect("Window icon must contain RGBA pixels")
}

#[cfg(not(target_os = "macos"))]
pub fn set_window_icons(
    mut created: MessageReader<WindowCreated>,
    mut icon: Local<Option<winit::window::Icon>>,
    _main_thread: NonSendMarker,
) {
    for event in created.read() {
        let icon = icon.get_or_insert_with(window_icon);
        WINIT_WINDOWS.with_borrow(|windows| {
            let window = windows
                .get_window(event.window)
                .expect("Newly created native window must exist");
            window.set_window_icon(Some(icon.clone()));
        });
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    #[test]
    fn embedded_icon_decodes_to_valid_native_rgba() {
        super::window_icon();
    }
}
