// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Keep runtime resources resident and warm the renderer behind the splash.
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_resource::{CachedPipelineState, PipelineCache},
    },
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default)]
struct RenderStatus {
    frame: u64,
    total: usize,
    ready: usize,
    finished: bool,
}

#[derive(Resource, Clone, Default)]
pub(crate) struct RenderFeedback(Arc<Mutex<RenderStatus>>);

#[derive(Resource, Default)]
pub struct Loading {
    handles: Vec<UntypedHandle>,
    last_render: RenderStatus,
    settled: u32,
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
            .add_systems(Startup, preload);
        app.sub_app_mut(RenderApp)
            .insert_resource(feedback)
            .add_systems(Render, report.in_set(RenderSystems::Cleanup));
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

fn report(cache: Res<PipelineCache>, feedback: Res<RenderFeedback>) {
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
}

impl Loading {
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
        if let Some(LoadState::Failed(error)) = server.get_load_state(handle) {
            panic!("Loading asset {handle:?} failed: {error}");
        }
        if let Some(RecursiveDependencyLoadState::Failed(error)) =
            server.get_recursive_dependency_load_state(handle)
        {
            panic!("Loading dependencies for {handle:?} failed: {error}");
        }
        loaded += usize::from(server.is_loaded_with_dependencies(handle));
    }
    let assets_ready = total > 0 && loaded == total;
    let rigs_ready =
        library.ready && rigs.iter().count() >= 1 + reef_sim::FISH_COUNT + reef_sim::JELLY_COUNT;
    let status = *feedback.0.lock().expect("Render feedback poisoned");
    loading.assets_loaded = loaded;
    loading.assets_total = total;
    loading.pipelines_ready = status.ready;
    loading.pipelines_total = status.total;
    let done = loading.observe(status, assets_ready, rigs_ready);
    let progress = if done {
        1.0
    } else if loading.warming {
        0.9 + 0.09 * loading.settled.min(8) as f32 / 8.0
    } else if total > 0 {
        0.8 * loaded as f32 / total as f32 + if rigs_ready { 0.1 } else { 0.0 }
    } else {
        0.0
    };
    loading.progress = loading.progress.max(progress);
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
