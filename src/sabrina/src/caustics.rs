// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! One neutral, world-projected light pass before tonemapping.
use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{Core3dSystems, FullscreenShader, schedule::Core3d, tonemapping::tonemapping},
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        diagnostic::RecordDiagnostics,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{
                sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled,
                uniform_buffer,
            },
            *,
        },
        renderer::{RenderContext, ViewQuery},
        texture::GpuImage,
        view::{ViewDepthStencilTexture, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};

#[derive(Component, Clone, Copy, Default, ExtractComponent, ShaderType)]
#[extract_app(RenderApp)]
pub(crate) struct CausticsSettings {
    // Time, intensity, surface height, reserved.
    parameters: Vec4,
}

#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub(crate) struct CausticsTexture(pub(crate) Handle<Image>);

pub(crate) struct CausticsPlugin;

impl Plugin for CausticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<CausticsSettings>::default(),
            UniformComponentPlugin::<CausticsSettings>::default(),
            ExtractResourcePlugin::<CausticsTexture>::default(),
        ))
        .add_systems(Startup, create_texture)
        .add_systems(Update, settings);
        app.sub_app_mut(RenderApp)
            .add_systems(RenderStartup, pipeline)
            .add_systems(
                Core3d,
                render
                    .in_set(Core3dSystems::PostProcess)
                    .before(tonemapping),
            );
    }
}

fn settings(
    ui: Res<crate::app::SessionUi>,
    preferences: Res<crate::settings::Preferences>,
    mut cameras: Query<&mut CausticsSettings>,
) {
    for mut settings in &mut cameras {
        settings.parameters = Vec4::new(
            if preferences.reduced_motion {
                0.0
            } else {
                ui.elapsed
            },
            if preferences.quality == 0 { 0.8 } else { 1.4 },
            20.0,
            0.0,
        );
    }
}

// Warped periodic cells concentrate light into connected ridges, not broad bright bands.
fn light_value(u: f32, v: f32) -> f32 {
    let point = Vec2::new(u, v) / std::f32::consts::TAU * 4.0
        + Vec2::new((v + u.sin() * 0.5).sin(), (u - v.sin() * 0.5).sin()) * 0.3;
    let cell = point.floor().as_ivec2();
    let mut nearest = f32::MAX;
    let mut second = f32::MAX;
    for y in -1..=1 {
        for x in -1..=1 {
            let neighbor = cell + IVec2::new(x, y);
            let tile = neighbor.rem_euclid(IVec2::splat(4));
            let seed = (tile.x + tile.y * 4) as f32;
            let center = neighbor.as_vec2()
                + Vec2::splat(0.5)
                + Vec2::new((seed * 12.9898 + 1.0).sin(), (seed * 7.233 + 2.0).sin()) * 0.24;
            let distance = point.distance(center);
            if distance < nearest {
                second = nearest;
                nearest = distance;
            } else {
                second = second.min(distance);
            }
        }
    }
    let ridge = (1.0 - (second - nearest) / 0.12).clamp(0.0, 1.0);
    ridge * ridge * (3.0 - 2.0 * ridge)
}

fn light_map(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32 * std::f32::consts::TAU;
            let v = y as f32 / size as f32 * std::f32::consts::TAU;
            pixels.push((light_value(u, v) * 255.0).round() as u8);
        }
    }
    pixels
}

fn create_texture(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new(
        Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        light_map(256),
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    commands.insert_resource(CausticsTexture(images.add(image)));
}

#[derive(Resource)]
struct CausticsPipeline {
    variants: [(BindGroupLayoutDescriptor, CachedRenderPipelineId); 2],
}

fn pipeline(
    mut commands: Commands,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
    server: Res<AssetServer>,
) {
    let variants = [false, true].map(|multisampled| {
        let layout = BindGroupLayoutDescriptor::new(
            "caustics_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    if multisampled {
                        texture_depth_2d_multisampled()
                    } else {
                        texture_depth_2d()
                    },
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<CausticsSettings>(true),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let id = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("world_caustics".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: server.load("shaders/caustics.wesl"),
                shader_defs: if multisampled {
                    vec!["MULTISAMPLED".into()]
                } else {
                    vec![]
                },
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
        (layout, id)
    });
    commands.insert_resource(CausticsPipeline { variants });
}

fn render(
    view: ViewQuery<(
        &ViewTarget,
        &ViewDepthStencilTexture,
        &ViewUniformOffset,
        &DynamicUniformIndex<CausticsSettings>,
        &Msaa,
    )>,
    pipeline: Res<CausticsPipeline>,
    cache: Res<PipelineCache>,
    uniforms: Res<ViewUniforms>,
    settings: Res<ComponentUniforms<CausticsSettings>>,
    texture: Res<CausticsTexture>,
    images: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
) {
    let (target, depth, view_offset, settings_offset, msaa) = view.into_inner();
    let (layout, id) = &pipeline.variants[usize::from(*msaa != Msaa::Off)];
    let (Some(pipeline), Some(view_binding), Some(settings_binding), Some(image), Some(depth)) = (
        cache.get_render_pipeline(*id),
        uniforms.uniforms.binding(),
        settings.uniforms().binding(),
        images.get(&texture.0),
        depth.attachment.depth_stencil_views().depth_only_view(),
    ) else {
        return;
    };
    let recorder = ctx.diagnostic_recorder();
    let span = recorder
        .as_deref()
        .map(|recorder| recorder.time_span(ctx.command_encoder(), "world_caustics"));
    let post = target.post_process_write();
    let group = ctx.render_device().create_bind_group(
        "caustics_bind_group",
        &cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((
            post.source,
            depth,
            view_binding,
            settings_binding,
            &image.texture_view,
            &image.sampler,
        )),
    );
    {
        let mut pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("world_caustics"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: post.destination,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[view_offset.offset, settings_offset.index()]);
        pass.draw(0..3, 0..1);
    }
    if let Some(span) = span {
        span.end(ctx.command_encoder());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_map_is_small_grayscale_and_periodic() {
        let size = 256;
        let pixels = light_map(size);
        assert_eq!(pixels.len(), (size * size) as usize);
        assert!(pixels.iter().any(|&p| p > 240));
        assert!(pixels.contains(&0));
        for y in 0..size as usize {
            let coordinate = y as f32 / size as f32 * std::f32::consts::TAU;
            assert!(
                (light_value(0.0, coordinate) - light_value(std::f32::consts::TAU, coordinate))
                    .abs()
                    < 0.0001
            );
            assert!(
                (light_value(coordinate, 0.0) - light_value(coordinate, std::f32::consts::TAU))
                    .abs()
                    < 0.0001
            );
        }
    }

    #[test]
    fn light_map_has_bright_narrow_ridges_and_dark_cell_interiors() {
        let pixels = light_map(256);
        let coverage = |threshold| {
            pixels.iter().filter(|&&p| p > threshold).count() as f32 / pixels.len() as f32
        };
        assert!((0.04..0.18).contains(&coverage(200)));
        assert!((0.1..0.3).contains(&coverage(20)));
        let mean = pixels.iter().map(|&p| p as f32 / 255.0).sum::<f32>() / pixels.len() as f32;
        assert!(
            mean < 0.15,
            "Caustics must not lift the whole surface: {mean}"
        );
    }

    #[test]
    fn caustics_respect_quality_and_reduced_motion() {
        let mut app = App::new();
        app.init_resource::<crate::app::SessionUi>()
            .init_resource::<crate::settings::Preferences>()
            .add_systems(Update, settings);
        let camera = app.world_mut().spawn(CausticsSettings::default()).id();
        app.world_mut()
            .resource_mut::<crate::app::SessionUi>()
            .elapsed = 12.0;
        app.update();
        assert_eq!(
            app.world()
                .get::<CausticsSettings>(camera)
                .unwrap()
                .parameters,
            Vec4::new(12.0, 1.4, 20.0, 0.0)
        );
        {
            let mut preferences = app
                .world_mut()
                .resource_mut::<crate::settings::Preferences>();
            preferences.reduced_motion = true;
            preferences.quality = 0;
        }
        app.update();
        assert_eq!(
            app.world()
                .get::<CausticsSettings>(camera)
                .unwrap()
                .parameters,
            Vec4::new(0.0, 0.8, 20.0, 0.0)
        );
    }
}
