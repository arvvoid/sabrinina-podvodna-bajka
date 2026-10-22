// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Desktop entry point: choose the renderer, find assets and install game plugins.

// Bevy systems expose their independent ECS queries and resources as parameters.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;

mod assets;

mod audio;

mod build_info;
mod controls;
mod diagnostics;
mod hair;
mod icon;

mod embedded_assets;

mod loading;
mod mipmaps;

mod presentation;

mod rumble;
mod settings;
mod tail;

mod ui;

mod verification;

mod visuals;
#[cfg(any(target_os = "windows", test))]
mod xinput;
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
    #[cfg(target_os = "linux")]
    options.apply_window_system_override();
    let mut app = App::new();
    diagnostics::init(&mut app);
    #[cfg(target_os = "linux")]
    info!("WINDOW SYSTEM: {}", options.window_system_label());
    #[cfg(not(target_os = "linux"))]
    if options.window_system.is_some() {
        warn!("Ignoring --x11/--wayland: window-system overrides are only available on Linux");
    }
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
    let gpu = gpu_settings(options.backend);
    info!(
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
    if external_assets.is_none() {
        embedded_assets::register(&mut app);
    }
    let plugins = DefaultPlugins
        .build()
        .disable::<bevy::log::LogPlugin>()
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
        });
    #[cfg(not(target_os = "macos"))]
    app.add_systems(Update, icon::set_window_icons);
    #[cfg(target_os = "windows")]
    let plugins = if options.wgi {
        info!("CONTROLLER BACKEND: Windows Gaming Input (GilRs)");
        plugins
    } else {
        info!("CONTROLLER BACKEND: native XInput (Xbox / Steam Input)");
        plugins.disable::<bevy::gilrs::GilrsPlugin>()
    };
    #[cfg(not(target_os = "windows"))]
    if options.wgi {
        warn!("Ignoring --wgi: this override is only available on Windows");
    }
    #[cfg(target_os = "windows")]
    if !options.wgi {
        app.add_plugins(xinput::XInputPlugin);
    }
    app.add_systems(
        Startup,
        |adapter: Res<bevy::render::renderer::RenderAdapterInfo>| {
            info!(
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
        .add_plugins(plugins)
        .add_plugins((
            bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
            app::GamePlugin,
            assets::CharacterPlugin,
            loading::LoadingPlugin,
            hair::HairPlugin,
            tail::TailPlugin,
            visuals::ReefPlugin,
            ui::ReefUiPlugin,
            audio::ReefAudioPlugin,
            rumble::RumblePlugin,
            verification::VerificationPlugin,
        ));
    if diagnostics {
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
    }
    app.run();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    Windows,
    Apple,
    Other,
}

fn current_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(any(target_os = "macos", target_os = "ios")) {
        Platform::Apple
    } else {
        Platform::Other
    }
}

fn default_backend(platform: Platform) -> app::RenderBackend {
    match platform {
        Platform::Windows => app::RenderBackend::Dx12,
        Platform::Apple => app::RenderBackend::Metal,
        Platform::Other => app::RenderBackend::Vulkan,
    }
}

fn backend_supported(platform: Platform, backend: app::RenderBackend) -> bool {
    match platform {
        Platform::Windows => matches!(
            backend,
            app::RenderBackend::Dx12 | app::RenderBackend::Vulkan | app::RenderBackend::OpenGl
        ),
        Platform::Apple => {
            matches!(
                backend,
                app::RenderBackend::Metal | app::RenderBackend::Vulkan | app::RenderBackend::OpenGl
            )
        }
        Platform::Other => matches!(
            backend,
            app::RenderBackend::Vulkan | app::RenderBackend::OpenGl
        ),
    }
}

fn selected_backend(
    platform: Platform,
    requested: Option<app::RenderBackend>,
) -> app::RenderBackend {
    requested
        .filter(|backend| backend_supported(platform, *backend))
        .unwrap_or_else(|| default_backend(platform))
}

fn gpu_settings(requested: Option<app::RenderBackend>) -> WgpuSettings {
    let platform = current_platform();
    let backend = selected_backend(platform, requested);
    if let Some(requested) = requested.filter(|requested| !backend_supported(platform, *requested))
    {
        warn!(
            "Ignoring {} on this platform; using {}",
            requested.flag(),
            backend.flag()
        );
    }
    let backend = match backend {
        app::RenderBackend::Dx12 => Backends::DX12,
        app::RenderBackend::Vulkan => Backends::VULKAN,
        app::RenderBackend::Metal => Backends::METAL,
        app::RenderBackend::OpenGl => Backends::GL,
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
    fn renderer_flags_parse_known_backends_only() {
        assert_eq!(
            app::RenderBackend::from_flag("--dx12"),
            Some(app::RenderBackend::Dx12)
        );
        assert_eq!(
            app::RenderBackend::from_flag("--vulkan"),
            Some(app::RenderBackend::Vulkan)
        );
        assert_eq!(
            app::RenderBackend::from_flag("--metal"),
            Some(app::RenderBackend::Metal)
        );
        assert_eq!(
            app::RenderBackend::from_flag("--opengl"),
            Some(app::RenderBackend::OpenGl)
        );
        assert_eq!(app::RenderBackend::from_flag("--vulkam"), None);
    }

    #[test]
    fn renderer_is_native_without_backend_fallbacks() {
        let backend = default_backend(current_platform());
        let gpu = gpu_settings(None);
        let expected = match backend {
            app::RenderBackend::Dx12 => Backends::DX12,
            app::RenderBackend::Vulkan => Backends::VULKAN,
            app::RenderBackend::Metal => Backends::METAL,
            app::RenderBackend::OpenGl => Backends::GL,
        };
        assert_eq!(gpu.backends, Some(expected));
        assert!(!gpu.backends.unwrap().contains(Backends::GL));
        assert_eq!(
            gpu.instance_flags
                .contains(bevy::render::settings::InstanceFlags::VALIDATION_INDIRECT_CALL),
            expected == Backends::DX12
        );
    }

    #[test]
    fn renderer_overrides_are_platform_limited() {
        use app::RenderBackend::{Dx12, Metal, Vulkan};

        assert_eq!(selected_backend(Platform::Windows, Some(Dx12)), Dx12);
        assert_eq!(selected_backend(Platform::Windows, Some(Vulkan)), Vulkan);
        assert_eq!(selected_backend(Platform::Windows, Some(Metal)), Dx12);
        assert_eq!(selected_backend(Platform::Apple, Some(Metal)), Metal);
        assert_eq!(selected_backend(Platform::Apple, Some(Vulkan)), Vulkan);
        assert_eq!(selected_backend(Platform::Apple, Some(Dx12)), Metal);
        assert_eq!(
            selected_backend(Platform::Windows, Some(app::RenderBackend::OpenGl)),
            app::RenderBackend::OpenGl
        );
        assert_eq!(
            selected_backend(Platform::Apple, Some(app::RenderBackend::OpenGl)),
            app::RenderBackend::OpenGl
        );
        assert_eq!(selected_backend(Platform::Other, Some(Vulkan)), Vulkan);
        assert_eq!(
            selected_backend(Platform::Other, Some(app::RenderBackend::OpenGl)),
            app::RenderBackend::OpenGl
        );
        assert_eq!(selected_backend(Platform::Other, Some(Dx12)), Vulkan);
        assert_eq!(selected_backend(Platform::Other, Some(Metal)), Vulkan);
    }

    #[test]
    fn opengl_override_selects_only_the_gl_backend() {
        let gpu = gpu_settings(Some(app::RenderBackend::OpenGl));
        assert_eq!(gpu.backends, Some(Backends::GL));
        assert!(
            !gpu.instance_flags
                .contains(bevy::render::settings::InstanceFlags::VALIDATION_INDIRECT_CALL)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc"))]
    #[test]
    fn windows_uses_embedded_dxc_without_a_runtime_dll() {
        assert!(matches!(
            gpu_settings(Some(app::RenderBackend::Dx12)).dx12_shader_compiler,
            bevy::render::settings::Dx12Compiler::StaticDxc
        ));
    }
}
