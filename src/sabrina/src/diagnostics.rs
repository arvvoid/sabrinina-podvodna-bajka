// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Persistent diagnostics, including startup failures in console-free releases.
use bevy::{
    log::{BoxedLayer, LogPlugin, tracing_subscriber},
    prelude::*,
};
use std::{fs::File, io, path::Path, sync::Mutex};

#[derive(Resource)]
struct LogFile(File);

fn create_log(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    File::create(path)
}

fn file_layer(app: &mut App) -> Option<BoxedLayer> {
    let file = app.world_mut().remove_resource::<LogFile>()?;
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(Mutex::new(file.0)),
    ))
}

pub fn init(app: &mut App) {
    let path = crate::settings::Preferences::path().with_file_name("game.log");
    let failure = match create_log(&path) {
        Ok(file) => {
            app.insert_resource(LogFile(file));
            None
        }
        Err(error) => Some(format!(
            "Cannot create diagnostic log {}: {error}",
            path.display()
        )),
    };
    app.add_plugins(LogPlugin {
        custom_layer: file_layer,
        ..default()
    });
    if let Some(failure) = failure {
        error!("{failure}");
        #[cfg(target_os = "windows")]
        show_error(&failure);
    } else {
        info!("Diagnostic log: {}", path.display());
    }
    info!("Build: {}", crate::build_info::menu_label());
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        error!("Fatal error: {panic}");
        #[cfg(all(target_os = "windows", not(debug_assertions)))]
        show_error(&format!(
            "The game encountered an error:\n{panic}\n\nDiagnostic log: {}",
            path.display()
        ));
        previous(panic);
    }));
}

#[cfg(target_os = "windows")]
fn show_error(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    let title: Vec<u16> = "Sabrina - diagnostic error\0".encode_utf16().collect();
    let message: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    // Both UTF-16 strings are terminated and live throughout the synchronous call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn creates_parent_directories_and_replaces_previous_run_log() {
        let directory =
            std::env::temp_dir().join(format!("sabrina-log-test-{}", std::process::id()));
        let path = directory.join("nested/game.log");
        let mut file = create_log(&path).unwrap();
        writeln!(file, "previous run").unwrap();
        drop(file);
        let mut file = create_log(&path).unwrap();
        writeln!(file, "new run").unwrap();
        drop(file);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new run\n");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory.join("nested")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn invalid_log_location_returns_an_explicit_error() {
        let path = std::env::temp_dir().join(format!("sabrina-log-blocked-{}", std::process::id()));
        File::create(&path).unwrap();
        assert!(create_log(&path.join("game.log")).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
