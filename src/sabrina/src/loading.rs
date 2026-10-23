// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Keep runtime resources resident and warm the renderer behind the splash.
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState, UntypedAssetId},
    camera::CameraOutputMode,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{BlendState, CachedPipelineState, PipelineCache},
        texture::GpuImage,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default)]
struct RenderStatus {
    frame: u64,
    total: usize,
    ready: usize,
    finished: bool,
    splash_frames: u32,
}

impl RenderStatus {
    fn observe_splash(&mut self, assets_ready: bool) {
        if self.splash_frames < 2 {
            let ready = assets_ready && self.total > 0 && self.ready == self.total;
            self.splash_frames = if ready { self.splash_frames + 1 } else { 0 };
        }
    }
}

#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub(crate) struct SplashAssets {
    images: [Handle<Image>; 2],
    font: Handle<Font>,
    loaded: bool,
}

#[derive(Component)]
struct UiCamera;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LoadResources;

#[derive(Resource, Clone, Default)]
pub(crate) struct RenderFeedback(Arc<Mutex<RenderStatus>>);

#[derive(Resource, Default)]
pub struct Loading {
    handles: Vec<UntypedHandle>,
    last_render: RenderStatus,
    settled: u32,
    pub splash_presented: bool,
    pub progress: f32,
    pub warming: bool,
    pub assets_loaded: usize,
    pub assets_total: usize,
    pub pipelines_ready: usize,
    pub pipelines_total: usize,
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        let feedback = RenderFeedback::default();
        app.insert_resource(feedback.clone())
            .init_resource::<Loading>()
            .add_plugins(ExtractResourcePlugin::<SplashAssets>::default())
            .add_systems(Startup, splash)
            .configure_sets(
                Update,
                LoadResources.run_if(
                    resources_allowed.and_then(bevy::ecs::schedule::common_conditions::run_once),
                ),
            )
            .add_systems(Update, bootstrap.before(LoadResources))
            .add_systems(Update, ui_camera.after(crate::assets::finish_load))
            .add_systems(
                Update,
                preload.in_set(LoadResources).before(crate::loading::ready),
            );
        app.sub_app_mut(RenderApp)
            .insert_resource(feedback)
            .add_systems(Render, report.in_set(RenderSystems::Cleanup));
    }
}

fn splash(
    mut commands: Commands,
    server: Res<AssetServer>,
    preferences: Res<crate::settings::Preferences>,
) {
    commands.insert_resource(SplashAssets {
        images: [
            server.load("ui/loading.jpg"),
            server.load(crate::ui::title_path(preferences.language)),
        ],
        font: server.load("fonts/DejaVuSans.ttf"),
        loaded: false,
    });
    commands.spawn((
        UiCamera,
        Camera2d,
        IsDefaultUiCamera,
        Msaa::Off,
        Camera {
            order: 1,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
    ));
}

fn ui_camera(ui: Res<crate::app::SessionUi>, mut camera: Single<&mut Camera, With<UiCamera>>) {
    camera.clear_color = if ui.screen == crate::app::Screen::Loading {
        ClearColorConfig::Default
    } else {
        // The SDR UI and HDR scene have separate intermediate textures.
        ClearColorConfig::Custom(Color::NONE)
    };
}

pub(crate) fn resources_allowed(loading: Res<Loading>) -> bool {
    loading.splash_presented
}

fn asset_loaded(server: &AssetServer, handle: UntypedAssetId) -> bool {
    if let Some(LoadState::Failed(error)) = server.get_load_state(handle) {
        panic!("Loading asset {handle:?} failed: {error}");
    }
    if let Some(RecursiveDependencyLoadState::Failed(error)) =
        server.get_recursive_dependency_load_state(handle)
    {
        panic!("Loading dependencies for {handle:?} failed: {error}");
    }
    server.is_loaded_with_dependencies(handle)
}

pub(crate) fn bootstrap(
    mut splash: ResMut<SplashAssets>,
    mut loading: ResMut<Loading>,
    feedback: Res<RenderFeedback>,
    server: Res<AssetServer>,
    time: Res<Time<Real>>,
) {
    if loading.splash_presented {
        return;
    }
    splash.loaded = splash
        .images
        .iter()
        .map(|handle| handle.id().untyped())
        .chain(std::iter::once(splash.font.id().untyped()))
        .all(|handle| asset_loaded(&server, handle));
    let status = *feedback.0.lock().expect("Render feedback poisoned");
    if status.splash_frames >= 2 {
        loading.splash_presented = true;
        loading.progress = 0.05;
        info!(
            "Loading splash presented at {:.3}s; starting resources and update check",
            time.elapsed_secs_f64()
        );
    }
}

fn preload(server: Res<AssetServer>, mut loading: ResMut<Loading>) {
    for &(path, _) in crate::embedded_assets::FILES {
        let handle = match path.rsplit('.').next().unwrap() {
            "png" => server.load::<Image>(path).untyped(),
            "ttf" => server.load::<Font>(path).untyped(),
            "ogg" => server.load::<AudioSource>(path).untyped(),
            "wesl" => server.load::<bevy::shader::Shader>(path).untyped(),
            _ => continue,
        };
        loading.handles.push(handle);
    }
}

fn report(
    cache: Res<PipelineCache>,
    feedback: Res<RenderFeedback>,
    splash: Option<Res<SplashAssets>>,
    images: Res<RenderAssets<GpuImage>>,
) {
    let mut status = feedback.0.lock().expect("Render feedback poisoned");
    if status.finished {
        return;
    }
    status.frame += 1;
    status.total = 0;
    status.ready = 0;
    for pipeline in cache.pipelines() {
        status.total += 1;
        match &pipeline.state {
            CachedPipelineState::Ok(_) => status.ready += 1,
            CachedPipelineState::Err(
                bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable,
            ) => {}
            CachedPipelineState::Err(error) => panic!("Loading shader pipeline failed: {error}"),
            _ => {}
        }
    }
    status.observe_splash(splash.is_some_and(|splash| {
        splash.loaded
            && splash
                .images
                .iter()
                .all(|handle| images.get(handle).is_some())
    }));
}

impl Loading {
    fn advance(
        &mut self,
        status: RenderStatus,
        assets_ready: bool,
        rigs_ready: bool,
        update_ready: bool,
    ) -> bool {
        let graphics_ready = self.observe(status, assets_ready, rigs_ready);
        let done = graphics_ready && update_ready;
        let update_progress = if update_ready { 0.01 } else { 0.0 };
        let progress = if done {
            1.0
        } else if self.warming {
            0.9 + update_progress + 0.08 * self.settled.min(8) as f32 / 8.0
        } else if self.assets_total > 0 {
            0.05 + update_progress
                + 0.75 * self.assets_loaded as f32 / self.assets_total as f32
                + if rigs_ready { 0.1 } else { 0.0 }
        } else {
            0.0
        };
        self.progress = self.progress.max(progress);
        done
    }

    fn observe(&mut self, status: RenderStatus, assets_ready: bool, rigs_ready: bool) -> bool {
        if !assets_ready || !rigs_ready {
            self.settled = 0;
            self.last_render = status;
            return false;
        }
        self.warming = true;
        if status.frame != self.last_render.frame {
            if status.total > 0
                && status.ready == status.total
                && status.total == self.last_render.total
            {
                self.settled += 1;
            } else {
                self.settled = 0;
            }
            self.last_render = status;
        }
        // Allow extraction, uploads and pipeline specialization to settle across
        // real render frames, not just main-world updates.
        self.settled >= 8
    }
}

pub(crate) fn ready(
    mut loading: ResMut<Loading>,
    feedback: Res<RenderFeedback>,
    server: Res<AssetServer>,
    library: Res<crate::assets::Library>,
    rigs: Query<&crate::assets::RigPlayer>,
    check: Res<crate::updates::UpdateCheck>,
) {
    if loading.progress == 1.0 {
        return;
    }
    let handles = loading
        .handles
        .iter()
        .map(UntypedHandle::id)
        .chain(library.handles.iter().map(|h| h.id().untyped()))
        .chain(library.lods.values().map(|h| h.id().untyped()));
    let mut total = 0;
    let mut loaded = 0;
    for handle in handles {
        total += 1;
        loaded += usize::from(asset_loaded(&server, handle));
    }
    let assets_ready = total > 0 && loaded == total;
    let rigs_ready =
        library.ready && rigs.iter().count() >= 1 + reef_sim::FISH_COUNT + reef_sim::JELLY_COUNT;
    let status = *feedback.0.lock().expect("Render feedback poisoned");
    loading.assets_loaded = loaded;
    loading.assets_total = total;
    loading.pipelines_ready = status.ready;
    loading.pipelines_total = status.total;
    let update_ready = check.status == crate::updates::Status::Continue;
    let done = loading.advance(status, assets_ready, rigs_ready, update_ready);
    if done {
        feedback
            .0
            .lock()
            .expect("Render feedback poisoned")
            .finished = true;
        info!(
            "Loading complete: {loaded} assets, {} graphics pipelines ready",
            status.ready
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_camera_clears_old_splash_pixels_before_overlaying_the_scene() {
        let mut app = App::new();
        app.init_resource::<crate::app::SessionUi>()
            .add_systems(Update, ui_camera);
        let camera = app.world_mut().spawn((UiCamera, Camera::default())).id();
        app.update();
        assert!(matches!(
            app.world().get::<Camera>(camera).unwrap().clear_color,
            ClearColorConfig::Default
        ));
        for screen in [crate::app::Screen::Menu, crate::app::Screen::Playing] {
            app.world_mut()
                .resource_mut::<crate::app::SessionUi>()
                .screen = screen;
            app.update();
            assert!(matches!(
                app.world().get::<Camera>(camera).unwrap().clear_color,
                ClearColorConfig::Custom(color) if color == Color::NONE
            ));
        }
    }

    #[test]
    fn splash_waits_for_uploads_pipelines_and_two_rendered_frames() {
        let mut status = RenderStatus::default();
        status.observe_splash(true);
        assert_eq!(status.splash_frames, 0);
        status.total = 3;
        status.ready = 3;
        status.observe_splash(false);
        assert_eq!(status.splash_frames, 0);
        status.observe_splash(true);
        assert_eq!(status.splash_frames, 1);
        status.ready = 2;
        status.observe_splash(true);
        assert_eq!(status.splash_frames, 0);
        status.ready = 3;
        status.observe_splash(true);
        status.observe_splash(true);
        assert_eq!(status.splash_frames, 2);
        status.ready = 0;
        status.observe_splash(false);
        assert_eq!(status.splash_frames, 2);
    }

    #[test]
    fn progress_advances_while_checking_but_cannot_finish_before_update_choice() {
        let mut loading = Loading {
            splash_presented: true,
            assets_total: 10,
            ..default()
        };
        let mut status = RenderStatus {
            total: 3,
            ready: 3,
            ..default()
        };
        for loaded in 0..=10 {
            loading.assets_loaded = loaded;
            status.frame += 1;
            let previous = loading.progress;
            assert!(!loading.advance(status, loaded == 10, false, false));
            assert!(loading.progress > previous);
            assert!(loading.progress < 1.0);
        }
        for _ in 0..12 {
            status.frame += 1;
            assert!(!loading.advance(status, true, true, false));
        }
        assert!((loading.progress - 0.98).abs() < 0.0001);
        assert!(loading.progress < 1.0);
        assert!(loading.advance(status, true, true, true));
        assert_eq!(loading.progress, 1.0);
    }

    #[test]
    fn waits_for_assets_rigs_and_distinct_settled_render_frames() {
        let mut loading = Loading::default();
        let mut status = RenderStatus {
            frame: 1,
            total: 4,
            ready: 4,
            ..default()
        };
        assert!(!loading.observe(status, false, true));
        assert!(!loading.observe(status, true, false));
        for frame in 2..9 {
            status.frame = frame;
            assert!(!loading.observe(status, true, true));
            assert!(!loading.observe(status, true, true));
        }
        status.frame = 9;
        assert!(loading.observe(status, true, true));
        status.frame = 10;
        status.total = 5;
        assert!(!loading.observe(status, true, true));
        assert_eq!(loading.settled, 0);
    }

    #[test]
    fn empty_or_pending_pipeline_cache_cannot_finish() {
        let mut loading = Loading::default();
        for frame in 1..30 {
            assert!(!loading.observe(
                RenderStatus {
                    frame,
                    total: 0,
                    ready: 0,
                    ..default()
                },
                true,
                true
            ));
        }
        for frame in 30..60 {
            assert!(!loading.observe(
                RenderStatus {
                    frame,
                    total: 3,
                    ready: 2,
                    ..default()
                },
                true,
                true
            ));
        }
    }
}
