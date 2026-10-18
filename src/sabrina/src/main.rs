// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Desktop entry point: choose the renderer, find assets and install game plugins.

// Bevy systems expose their independent ECS queries and resources as parameters.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod app;

mod assets;

mod audio;

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
    window::{MonitorSelection, PresentMode, WindowMode},
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
    let backend = if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
        Backends::METAL
    } else if cfg!(target_os = "windows") {
        Backends::VULKAN | Backends::DX12
    } else {
        Backends::VULKAN
    };
    let backend = std::env::var("SABRINA_RENDERER")
        .ok()
        .map(|s| match s.as_str() {
            "gl" => Backends::GL,
            "dx12" => Backends::DX12,
            "metal" => Backends::METAL,
            _ => Backends::VULKAN,
        })
        .unwrap_or(backend);
    let mut gpu = WgpuSettings {
        backends: Some(backend),
        ..default()
    };
    if !backend.contains(Backends::DX12) {
        // The DX12-only indirect validation workaround requires compute support
        // that native GL 3.3 lacks. Keep ordinary validation enabled.
        gpu.instance_flags
            .remove(bevy::render::settings::InstanceFlags::VALIDATION_INDIRECT_CALL);
    }
    let asset_path = std::env::var("SABRINA_ASSETS").unwrap_or_else(|_| {
        let packaged = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("assets")));
        packaged
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")).into())
            .to_string_lossy()
            .into_owned()
    });
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
    let mut app = App::new();
    app.add_systems(
        Startup,
        |adapter: Res<bevy::render::renderer::RenderAdapterInfo>| {
            println!(
                "RENDER DEVICE: {} ({:?}), {}",
                adapter.name, adapter.backend, adapter.driver_info
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
                    file_path: asset_path,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Sabrinina podvodna bajka".into(),
                        resolution: resolution.into(),
                        mode,
                        present_mode: PresentMode::Fifo,
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
