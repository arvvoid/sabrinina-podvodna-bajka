// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Desktop entry point: choose the renderer, find assets and install game plugins.

// Bevy systems expose their independent ECS queries and resources as parameters.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod app;

mod assets;

mod audio;

mod build_info;
mod controls;

mod embedded_assets;

mod mipmaps;

mod presentation;

mod settings;

mod ui;

mod verification;

mod visuals;
use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::{MonitorSelection, WindowMode},
};

fn main() {
    let options = app::Options::from_args();
    let mut prefs = settings::Preferences::load();
    if let Some(kind) = &options.capture {
        if kind.starts_with("hr-") {
            prefs.language = 1;
        }
        if kind.starts_with("it-") {
            prefs.language = 2;
        }
        if kind.starts_with("graphics-") {
            prefs.quality = match kind.as_str() {
                "graphics-low" => 0,
                "graphics-high" => 2,
                _ => 1,
            };
        }
    }
    let gpu = gpu_settings();
    println!(
        "RENDER CONFIG: {:?}, DX12 compiler {:?}, indirect drawing {}",
        gpu.backends,
        gpu.dx12_shader_compiler,
        if options.direct_draw { "off" } else { "on" }
    );
    let external_assets = match std::env::var("SABRINA_ASSETS") {
        Ok(path) => {
            let directory = std::path::Path::new(&path)
                .canonicalize()
                .expect("SABRINA_ASSETS must point to an existing asset directory");
            assert!(
                directory.is_dir(),
                "SABRINA_ASSETS must point to a directory"
            );
            Some(
                directory
                    .into_os_string()
                    .into_string()
                    .expect("SABRINA_ASSETS path must be UTF-8"),
            )
        }
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => panic!("Invalid SABRINA_ASSETS: {error}"),
    };
    let time_update = if options.verify {
        bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
            1.0 / 60.0,
        ))
    } else {
        bevy::time::TimeUpdateStrategy::Automatic
    };
    let resolution = if options.fullhd {
        (1920, 1080)
    } else {
        (1280, 720)
    };
    let mode = if prefs.fullscreen && options.capture.is_none() && !options.verify {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
    let diagnostics = options.capture.is_some();
    let decorations = options.capture.as_deref() != Some("demo");
    let present_mode = prefs.present_mode();
    let mut app = App::new();
    if external_assets.is_none() {
        embedded_assets::register(&mut app);
    }
    app.add_systems(
        Startup,
        |adapter: Res<bevy::render::renderer::RenderAdapterInfo>| {
            println!(
                "RENDER DEVICE: {} ({:?}), driver {} {}",
                adapter.name, adapter.backend, adapter.driver, adapter.driver_info
            );
        },
    );
    app.insert_resource(ClearColor(Color::srgb(0.015, 0.16, 0.28)))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(time_update)
        .insert_resource(options)
        .insert_resource(prefs)
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: external_assets.unwrap_or_else(|| "assets".into()),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Sabrinina podvodna bajka".into(),
                        resolution: resolution.into(),
                        mode,
                        decorations,
                        present_mode,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(gpu)),
                    ..default()
                }),
        )
        .add_plugins((
            bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
            app::GamePlugin,
            assets::CharacterPlugin,
            visuals::ReefPlugin,
            ui::ReefUiPlugin,
            audio::ReefAudioPlugin,
            verification::VerificationPlugin,
        ));
    if diagnostics {
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
    }
    app.run();
}

fn gpu_settings() -> WgpuSettings {
    let backend = if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
        Backends::METAL
    } else if cfg!(target_os = "windows") {
        Backends::DX12
    } else {
        Backends::VULKAN
    };
    let mut gpu = WgpuSettings {
        backends: Some(backend),
        ..default()
    };
    #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc"))]
    {
        gpu.dx12_shader_compiler = bevy::render::settings::Dx12Compiler::StaticDxc;
    }
    // WgpuSettings defaults also inspect WGPU_BACKEND. Keep the validation
    // required by our actual backend even when that variable requests another.
    if backend == Backends::DX12 {
        gpu.instance_flags
            .insert(bevy::render::settings::InstanceFlags::VALIDATION_INDIRECT_CALL);
    } else {
        gpu.instance_flags
            .remove(bevy::render::settings::InstanceFlags::VALIDATION_INDIRECT_CALL);
    }
    gpu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_is_native_without_backend_fallbacks() {
        let gpu = gpu_settings();
        let expected = if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            Backends::METAL
        } else if cfg!(target_os = "windows") {
            Backends::DX12
        } else {
            Backends::VULKAN
        };
        assert_eq!(gpu.backends, Some(expected));
        assert!(!gpu.backends.unwrap().contains(Backends::GL));
        assert_eq!(
            gpu.instance_flags
                .contains(bevy::render::settings::InstanceFlags::VALIDATION_INDIRECT_CALL),
            expected == Backends::DX12
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc"))]
    #[test]
    fn windows_uses_embedded_dxc_without_a_runtime_dll() {
        assert!(matches!(
            gpu_settings().dx12_shader_compiler,
            bevy::render::settings::Dx12Compiler::StaticDxc
        ));
    }
}
