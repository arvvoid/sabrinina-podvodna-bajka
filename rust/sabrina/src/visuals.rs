// SPDX-License-Identifier: MIT
use crate::{
    app::{Options, SessionUi},
    assets::{Actor, ActorVisual, CharacterSurface},
    presentation::Presentation,
    settings::Preferences,
};
use bevy::{
    anti_alias::fxaa::Fxaa,
    camera::{Hdr, ScalingMode},
    light::{
        CascadeShadowConfigBuilder, DirectionalLightShadowMap, NotShadowCaster, NotShadowReceiver,
        ShadowFilteringMethod,
    },
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderType},
        view::{
            Tonemapping,
            screenshot::{Screenshot, save_to_disk},
        },
    },
    shader::ShaderRef,
};
use reef_sim as sim;
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WaterParams {
    clock: Vec4,
    color: Vec4,
    depth_samples: [Vec4; 6],
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub params: WaterParams,
    pub alpha: AlphaMode,
}
impl Material for WaterMaterial {
    fn enable_shadows() -> bool {
        false
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Distant creatures turn through both faces of the quad.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ReefExtension {
    #[uniform(100)]
    pub settings: Vec4,
}
impl MaterialExtension for ReefExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/reef.wesl".into()
    }
}
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct CrystalExtension {
    #[uniform(100)]
    pub settings: Vec4,
}
impl MaterialExtension for CrystalExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/crystal.wesl".into()
    }
}
type CrystalMaterial = ExtendedMaterial<StandardMaterial, CrystalExtension>;
#[derive(Component)]
struct MagicLight;
type ReefMaterial = ExtendedMaterial<StandardMaterial, ReefExtension>;
#[derive(Component, Clone, Copy)]
enum Effect {
    Backdrop,
    Shaft(usize),
    Water,
    Distant(usize),
    Wave,
    Song(usize),
    Glow,
    Wake,
    Shield(usize),
    Trail(usize),
    Spark(usize),
    Pop(usize, usize),
    Plankton(usize),
    Note(usize),
    Burst,
}
#[derive(Component)]
struct Sun {
    shadow_layer: bool,
}
#[derive(Resource)]
struct Trail {
    points: [Vec3; 24],
    previous: Vec3,
    time: f32,
    flying: bool,
}
pub struct ReefPlugin;
impl Plugin for ReefPlugin {
    fn build(&self, a: &mut App) {
        a.add_plugins((
            MaterialPlugin::<WaterMaterial>::default(),
            MaterialPlugin::<ReefMaterial>::default(),
            MaterialPlugin::<CrystalMaterial>::default(),
        ))
        .insert_resource(Trail {
            points: [Vec3::ZERO; 24],
            previous: Vec3::ZERO,
            time: 0.0,
            flying: false,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (bind_reef, actors, effects, lighting, crystal_light, capture)
                .chain()
                .after(crate::presentation::prepare),
        );
    }
}
fn rnd(s: f32) -> f32 {
    0.5 + 0.5 * (s * 17.137 + s * s * 0.71).sin()
}
fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<WaterMaterial>>,
    mut solid: ResMut<Assets<StandardMaterial>>,
    p: Res<Preferences>,
    options: Res<Options>,
) {
    let close = options
        .capture
        .as_deref()
        .is_some_and(|k| k.starts_with("graphics-") || k.starts_with("pose-") || k == "rig");
    let casting = options.capture.as_deref() == Some("throw");
    // One small shadow map is sufficient for this fixed, orthographic stage.
    c.insert_resource(DirectionalLightShadowMap {
        size: if p.quality == 2 { 1024 } else { 512 },
    });
    c.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.65, 0.88, 0.91),
        brightness: 350.0,
        ..default()
    });
    c.spawn((
        Camera3d::default(),
        Hdr,
        bevy::core_pipeline::prepass::DepthPrepass,
        Tonemapping::Reinhard,
        ShadowFilteringMethod::Gaussian,
        if p.quality == 2 {
            Msaa::Sample2
        } else {
            Msaa::Off
        },
        Fxaa {
            enabled: false,
            ..default()
        },
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: if close {
                    7.5
                } else if casting {
                    11.0
                } else {
                    19.5
                },
            },
            far: 150.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0.0, if close || casting { 8.5 } else { 9.2 }, 36.0),
        DistanceFog {
            color: Color::srgb(0.025, 0.38, 0.48),
            falloff: FogFalloff::Exponential { density: 0.0035 },
            ..default()
        },
    ));
    // Broad scattered light remains visible in shadow. Splitting the sunlight
    // limits shadow contrast without raising the brightness of lit skin/hair.
    for shadow_layer in [true, false] {
        c.spawn((
            Sun { shadow_layer },
            DirectionalLight {
                color: Color::srgb(1.0, 0.96, 0.78),
                illuminance: 9000.0 * sunlight_weight(shadow_layer, p.quality),
                shadow_maps_enabled: shadow_layer && p.quality > 0,
                contact_shadows_enabled: false,
                ..default()
            },
            CascadeShadowConfigBuilder {
                num_cascades: 1,
                maximum_distance: 80.0,
                ..default()
            }
            .build(),
            Transform::from_rotation(Quat::from_euler(
                EulerRot::YXZ,
                -25_f32.to_radians(),
                -38_f32.to_radians(),
                -20_f32.to_radians(),
            )),
        ));
    }
    c.spawn((
        DirectionalLight {
            color: Color::srgb(0.22, 0.72, 1.0),
            illuminance: 1900.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            160_f32.to_radians(),
            25_f32.to_radians(),
            0.0,
        )),
    ));
    c.spawn((
        MagicLight,
        PointLight {
            color: Color::srgb(0.32, 0.82, 1.0),
            intensity: 0.0,
            range: 5.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
    ));
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let mut plane = |effect, kind, seed, position: Vec3, scale: Vec3, alpha| {
        let mat = mats.add(WaterMaterial {
            params: WaterParams {
                clock: Vec4::new(0.0, kind, seed, 1.0),
                color: Vec4::ONE,
                depth_samples: [Vec4::ZERO; 6],
            },
            alpha,
        });
        c.spawn((
            effect,
            NotShadowCaster,
            NotShadowReceiver,
            Mesh3d(quad.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(position).with_scale(scale),
        ));
    };
    plane(
        Effect::Backdrop,
        0.0,
        0.0,
        Vec3::new(0.0, 10.0, -25.0),
        Vec3::new(40.0, 24.0, 1.0),
        AlphaMode::Opaque,
    );
    for i in 0..6 {
        plane(
            Effect::Shaft(i),
            1.0,
            i as f32,
            Vec3::new(-11.0 + i as f32 * 5.0, 11.0, -8.0 - i as f32 * 0.15),
            Vec3::new(3.0 + (i % 2) as f32, 22.0, 1.0),
            AlphaMode::Add,
        );
    }
    plane(
        Effect::Water,
        2.0,
        0.0,
        Vec3::new(0.0, 9.2, -3.0),
        Vec3::new(38.0, 21.0, 1.0),
        AlphaMode::Add,
    );
    for i in 0..18 {
        plane(
            Effect::Distant(i),
            3.0,
            i as f32,
            Vec3::ZERO,
            Vec3::ONE,
            AlphaMode::Blend,
        );
    }
    for (effect, kind) in [
        (Effect::Wave, 4.0),
        (Effect::Glow, 6.0),
        (Effect::Wake, 7.0),
        (Effect::Burst, 9.0),
    ] {
        plane(effect, kind, 0.0, Vec3::ZERO, Vec3::ONE, AlphaMode::Add);
    }
    for i in 0..3 {
        plane(
            Effect::Song(i),
            5.0,
            0.0,
            Vec3::ZERO,
            Vec3::ONE,
            AlphaMode::Add,
        );
    }
    for i in 0..72 {
        plane(
            Effect::Shield(i),
            8.0,
            0.0,
            Vec3::ZERO,
            Vec3::ONE,
            AlphaMode::Add,
        );
    }
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(12, 8));
    let mut ball = |effect, color: Color, size: f32| {
        c.spawn((
            effect,
            NotShadowCaster,
            NotShadowReceiver,
            Mesh3d(sphere.clone()),
            MeshMaterial3d(solid.add(StandardMaterial {
                base_color: color,
                emissive: color.to_linear() * 0.6,
                perceptual_roughness: 0.68,
                ..default()
            })),
            Transform::from_scale(Vec3::splat(size)),
        ));
    };
    for i in 0..24 {
        ball(Effect::Trail(i), Color::srgb(0.25, 0.8, 1.0), 0.03);
        ball(Effect::Spark(i), Color::srgb(0.4, 0.95, 1.0), 0.02);
    }
    for i in 0..72 {
        for k in 0..6 {
            ball(Effect::Pop(i, k), Color::srgb(1.0, 0.34, 0.36), 0.1);
        }
    }
    for i in 0..80 {
        ball(Effect::Plankton(i), Color::srgb(0.45, 0.82, 0.91), 0.018);
    }
    // The original song uses small eighth notes, not particle dots.
    let stem = meshes.add(Cylinder::new(1.0, 1.0));
    for i in 0..7 {
        let color = if i % 2 == 0 {
            Color::srgb(0.46, 0.94, 1.0)
        } else {
            Color::srgb(0.99, 0.67, 0.91)
        };
        let material = solid.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * 0.7,
            ..default()
        });
        let parent = c
            .spawn((Effect::Note(i), Transform::default(), Visibility::default()))
            .id();
        let head = c
            .spawn((
                NotShadowCaster,
                NotShadowReceiver,
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_scale(Vec3::new(0.075, 0.05, 0.025)),
            ))
            .id();
        c.entity(parent).add_child(head);
        for (start, end, radius) in [
            (
                Vec3::new(0.065, 0.0, 0.0),
                Vec3::new(0.065, 0.25, 0.0),
                0.017,
            ),
            (
                Vec3::new(0.065, 0.25, 0.0),
                Vec3::new(0.18, 0.20, 0.0),
                0.02,
            ),
        ] {
            let delta = end - start;
            let part = c
                .spawn((
                    NotShadowCaster,
                    NotShadowReceiver,
                    Mesh3d(stem.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation((start + end) * 0.5)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, delta.normalize()))
                        .with_scale(Vec3::new(radius, delta.length(), radius)),
                ))
                .id();
            c.entity(parent).add_child(part);
        }
    }
}
fn bind_reef(
    mut c: Commands,
    query: Query<
        (Entity, &CharacterSurface, &MeshMaterial3d<StandardMaterial>),
        Added<CharacterSurface>,
    >,
    standard: Res<Assets<StandardMaterial>>,
    mut reef: ResMut<Assets<ReefMaterial>>,
    mut crystal: ResMut<Assets<CrystalMaterial>>,
) {
    for (e, surface, handle) in &query {
        if surface.actor == Actor::Crystal
            && let Some(base) = standard.get(&handle.0)
        {
            let mut base = base.clone();
            base.cull_mode = None;
            base.double_sided = true;
            c.entity(e)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(crystal.add(ExtendedMaterial {
                    base,
                    extension: CrystalExtension {
                        settings: Vec4::ZERO,
                    },
                })));
        }
        if surface.actor != Actor::Reef {
            continue;
        }
        if let Some(base) = standard.get(&handle.0) {
            c.entity(e)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(reef.add(ExtendedMaterial {
                    base: base.clone(),
                    extension: ReefExtension {
                        settings: Vec4::new(0.0, 1.0, 1.0, 0.0),
                    },
                })));
        }
    }
}
fn actors(
    w: Res<Presentation>,
    u: Res<SessionUi>,
    p: Res<Preferences>,
    time: Res<Time>,
    mut actors: Query<(&Actor, &mut ActorVisual, &mut Transform, &mut Visibility)>,
    mut surfaces: Query<(&CharacterSurface, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let dt = if u.paused() { 0.0 } else { time.delta_secs() };
    let t = w.0.session.time;
    let motion = if p.reduced_motion { 0.15 } else { 1.0 };
    for (actor, mut view, mut tr, mut vis) in &mut actors {
        let mut show = true;
        match *actor {
            Actor::Sabrina => {
                let s = &w.0.sabrina;
                tr.translation = Vec3::new(
                    s.motion.pos.x,
                    s.motion.pos.y + 0.07 * (t * 1.8).sin() * motion,
                    0.5,
                );
                tr.rotation =
                    Quat::from_rotation_y(s.swimming.yaw) * Quat::from_rotation_z(s.swimming.pitch);
                let pulse = if s.vitality.invulnerable > 0.0 {
                    0.94 + 0.06 * (t * 5.0).sin()
                } else {
                    1.0
                };
                tr.scale = Vec3::splat(pulse);
            }
            Actor::Fish(i) => {
                let f = &w.0.fish[i];
                show = f.life.active || f.life.pop_time > 0.22;
                let size = f.appearance.size
                    * (0.3 + 0.7 * f.appearance.birth)
                    * if f.life.pop_time > 0.0 {
                        1.0 + (0.55 - f.life.pop_time) * 1.2
                    } else {
                        1.0
                    };
                tr.translation = Vec3::new(f.motion.pos.x, f.motion.pos.y, f.appearance.depth);
                let target = if f.motion.direction.x < 0.0 {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                let diff = (target - view.yaw).sin().atan2((target - view.yaw).cos());
                view.yaw += diff * (1.0 - (-7.45 * dt).exp());
                tr.rotation = Quat::from_rotation_y(view.yaw)
                    * Quat::from_rotation_z(f.motion.direction.y.clamp(-1.0, 1.0).asin());
                tr.scale = Vec3::splat(size);
            }
            Actor::Jelly(i) => {
                let j = &w.0.jelly[i];
                show = j.behaviour.state != sim::JellyState::Hidden;
                let s = sim::jelly_scale(j);
                let recoil =
                    ((j.behaviour.repel_flash / 0.65).clamp(0.0, 1.0) * std::f32::consts::PI).sin();
                tr.translation = Vec3::new(
                    j.motion.pos.x,
                    j.motion.pos.y + 0.10 * (t * 2.0 + j.behaviour.phase).sin() * motion,
                    0.2,
                );
                tr.rotation =
                    Quat::from_rotation_z(0.035 * (t * 1.7 + j.behaviour.phase).sin() * motion);
                tr.scale = Vec3::new(s * (1.0 + 0.12 * recoil), s * (1.0 - 0.15 * recoil), s);
            }
            Actor::Crystal => {
                let c = &w.0.crystal;
                let flying = c.magic.state == sim::Spell::Flying;
                let charging = c.magic.state == sim::Spell::Charging;
                show = c.magic.state != sim::Spell::Burst
                    && (c.magic.state != sim::Spell::Resting || w.0.sabrina.cooldown <= 0.0);
                tr.translation = Vec3::new(c.motion.pos.x, c.motion.pos.y, c.magic.depth);
                tr.scale = Vec3::splat(if flying { 0.85 } else { 0.72 });
                let orbit = t * 0.9 * if p.reduced_motion { 0.25 } else { 1.0 };
                tr.rotation = if charging {
                    Quat::from_rotation_y(w.0.sabrina.swimming.yaw)
                        * Quat::from_rotation_z(w.0.sabrina.swimming.pitch - 0.65)
                } else {
                    Quat::from_euler(
                        EulerRot::YXZ,
                        0.35 * (orbit * 1.3).sin(),
                        0.12 * orbit.sin(),
                        if flying {
                            c.motion.direction.y.atan2(c.motion.direction.x) - 1.15
                        } else {
                            0.16 * (orbit * 2.0).sin()
                        },
                    )
                };
            }
            Actor::Reef => {}
        }
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (surface, mut handle) in &mut surfaces {
        let red = match surface.actor {
            Actor::Sabrina => {
                w.0.sabrina.vitality.hurt_flash > 0.0 || w.0.sabrina.vitality.stunned > 0.0
            }
            Actor::Fish(i) => w.0.fish[i].life.pop_time > 0.0,
            _ => false,
        };
        let desired = if red { &surface.hurt } else { &surface.normal };
        if handle.0 != *desired {
            handle.0 = desired.clone();
        }
    }
}
fn effects(
    w: Res<Presentation>,
    u: Res<SessionUi>,
    p: Res<Preferences>,
    time: Res<Time>,
    mut trail: ResMut<Trail>,
    mut query: Query<(
        &Effect,
        &mut Transform,
        &mut Visibility,
        Option<&MeshMaterial3d<WaterMaterial>>,
    )>,
    mut water: ResMut<Assets<WaterMaterial>>,
) {
    let t = w.0.session.time;
    let dt = if u.paused() { 0.0 } else { time.delta_secs() };
    let motion = if p.reduced_motion { 0.15 } else { 1.0 };
    let s = &w.0.sabrina;
    let c = &w.0.crystal;
    let cp = Vec3::new(c.motion.pos.x, c.motion.pos.y, c.magic.depth);
    let flying = c.magic.state == sim::Spell::Flying;
    let resting = c.magic.state == sim::Spell::Resting;
    let burst = c.magic.state == sim::Spell::Burst;
    let charging = c.magic.state == sim::Spell::Charging;
    let ready = s.cooldown <= 0.0;
    if flying {
        if !trail.flying {
            trail.points.fill(cp);
            trail.previous = cp;
            trail.time = 0.0;
        }
        trail.time += dt;
        while trail.time >= 1.0 / 60.0 {
            trail.time -= 1.0 / 60.0;
            for i in (1..24).rev() {
                trail.points[i] = trail.points[i - 1];
            }
            trail.points[0] = trail.previous.lerp(
                cp,
                if dt > 0.0 {
                    (1.0 - trail.time / dt).clamp(0.0, 1.0)
                } else {
                    1.0
                },
            );
        }
    }
    trail.flying = flying;
    trail.previous = cp;
    for (effect, mut tr, mut vis, handle) in &mut query {
        let mut show = true;
        let mut amount = 1.0;
        match *effect {
            Effect::Backdrop => {}
            Effect::Shaft(i) => {
                show = p.quality > 0 && i < if p.quality == 1 { 4 } else { 6 };
                tr.rotation = Quat::from_rotation_z(
                    (-17.0 + 4.0 * (u.elapsed * 0.15 + i as f32 * 0.5).sin() * motion).to_radians(),
                );
            }
            Effect::Water => {
                amount = match p.quality {
                    0 => 3.0,
                    1 => 6.0,
                    _ => 12.0,
                }
            }
            Effect::Distant(i) => {
                let speed = if i < 4 {
                    0.42
                } else {
                    0.65 + 0.06 * (i % 4) as f32
                };
                let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                tr.translation = Vec3::new(
                    ((t * speed + i as f32 * 7.13) % 54.0 - 27.0) * side,
                    8.0 + (i % 5) as f32 * 1.7 + 0.55 * (t * 0.14 + i as f32).sin(),
                    -18.0 - (i % 3) as f32,
                );
                let size = if i < 4 {
                    7.0 + i as f32 * 0.65
                } else {
                    1.2 + 0.35 * (i % 4) as f32
                };
                tr.scale = Vec3::new(size * side, size * 0.65, 1.0);
            }
            Effect::Wave => {
                let wave = &s.wave;
                show = wave.active;
                let travel = (wave.time * 8.0).min(5.5);
                let p = wave.origin + wave.direction * travel;
                tr.translation = Vec3::new(p.x, p.y, 1.6);
                tr.rotation = Quat::from_rotation_z(wave.direction.y.atan2(wave.direction.x));
                tr.scale = Vec3::new(1.2, 2.0 * (0.65 + 0.38 * travel), 1.0);
                amount = wave.time / 0.75;
            }
            Effect::Song(i) => {
                show = s.song.active && s.song.strength > 0.05;
                let f = (t * 0.40 * motion + i as f32 / 3.0) % 1.0;
                let size = if i == 0 { 1.0 } else { 0.2 + f * 0.8 } * sim::song_radius(&w.0);
                tr.translation = Vec3::new(s.motion.pos.x, s.motion.pos.y, -0.2);
                tr.scale = Vec3::new(size * 2.0, size * 2.0, 1.0);
                amount = if i == 0 {
                    0.16
                } else {
                    0.32 * (f * std::f32::consts::PI).sin()
                } * s.song.strength;
            }
            Effect::Glow => {
                show = !burst && (!resting || ready);
                tr.translation = cp - Vec3::Z * 0.15;
                tr.scale = Vec3::new(2.4, 2.4, 1.0);
            }
            Effect::Wake => {
                show = flying;
                tr.translation =
                    cp - Vec3::new(c.motion.direction.x, c.motion.direction.y, 0.0) * 1.65;
                tr.rotation =
                    Quat::from_rotation_z(c.motion.direction.y.atan2(c.motion.direction.x));
                tr.scale = Vec3::new(3.8, 1.35, 1.0);
                amount = (c.magic.time * 5.0).min(1.0);
            }
            Effect::Shield(i) => {
                let f = &w.0.fish[i];
                show = f.life.active && f.rescue.shield > 0.0;
                tr.translation =
                    Vec3::new(f.motion.pos.x, f.motion.pos.y, f.appearance.depth + 0.8);
                tr.scale = Vec3::new(2.2, 1.8, 1.0) * f.appearance.size;
                amount = f.rescue.shield.min(1.0);
            }
            Effect::Trail(i) => {
                show = flying || (resting && ready);
                tr.translation = if resting {
                    let o = sim::crystal_orbit(&w.0, i as f32 * 0.05);
                    Vec3::new(
                        s.motion.pos.x + o.offset.x,
                        s.motion.pos.y + o.offset.y,
                        o.depth,
                    )
                } else {
                    let fraction = (trail.time * 60.0).clamp(0.0, 1.0);
                    if i == 0 {
                        trail.points[0].lerp(cp, fraction)
                    } else {
                        trail.points[i].lerp(trail.points[i - 1], fraction)
                    }
                };
                tr.scale =
                    Vec3::splat(if flying { 0.075 } else { 0.028 } * (1.0 - i as f32 / 24.0));
            }
            Effect::Spark(i) => {
                show = !resting || ready;
                let a = i as f32 * 2.39996 + t * 2.0 * motion;
                let r = if burst {
                    c.magic.time * 4.0
                } else {
                    0.5 + 0.25 * (a * 1.7).sin()
                };
                tr.translation = cp + Vec3::new(a.cos() * r, a.sin() * r, (a * 2.0).sin() * 0.3);
                tr.scale = Vec3::splat(
                    0.035
                        * if burst {
                            (1.0 - c.magic.time / 0.7).max(0.0)
                        } else {
                            0.65 + 0.35 * a.sin()
                        },
                );
            }
            Effect::Pop(i, k) => {
                let f = &w.0.fish[i];
                show = f.life.pop_time > 0.0 && f.life.pop_time < 0.30;
                let a = k as f32 * std::f32::consts::PI / 3.0;
                let u = ((0.30 - f.life.pop_time) / 0.30).clamp(0.0, 1.0);
                tr.translation = Vec3::new(
                    f.motion.pos.x + a.cos() * u * 0.8,
                    f.motion.pos.y + a.sin() * u * 0.8,
                    f.appearance.depth,
                );
                tr.scale = Vec3::splat(0.1 * (1.0 - u));
            }
            Effect::Plankton(i) => {
                show = !p.reduced_motion
                    && i < match p.quality {
                        0 => 18,
                        1 => 45,
                        _ => 80,
                    };
                tr.translation = Vec3::new(
                    -18.0 + 36.0 * rnd(i as f32 + 900.0),
                    ((18.0 * rnd(i as f32 + 950.0) + t * (0.12 + 0.06 * rnd(i as f32))) % 19.0)
                        .max(0.3),
                    -5.0 + 8.0 * rnd(i as f32 + 999.0),
                );
            }
            Effect::Note(i) => {
                show = s.song.active && s.song.strength > 0.05;
                let u = (t * 0.28 + i as f32 / 7.0) % 1.0;
                let a = i as f32 * 2.4 + t * 0.30;
                tr.translation = Vec3::new(
                    s.motion.pos.x + a.sin() * (0.2 + u * 0.45) * sim::song_radius(&w.0),
                    s.motion.pos.y + 0.35 + u * sim::song_radius(&w.0) * 0.65,
                    1.2,
                );
                tr.scale = Vec3::splat((u * std::f32::consts::PI).sin() * s.song.strength);
                tr.rotation = Quat::from_rotation_z(a.sin() * 0.2);
            }
            Effect::Burst => {
                show = charging || burst;
                let r = if burst {
                    1.0 + c.magic.time * 6.0
                } else {
                    0.8 + c.magic.time
                };
                tr.translation = cp;
                tr.scale = Vec3::new(r * 2.0, r * 2.0, 1.0);
                amount = 0.5;
            }
        }
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(handle) = handle
            && let Some(mut m) = water.get_mut(&handle.0)
        {
            m.params.clock.x = t * if p.reduced_motion { 0.15 } else { 1.0 };
            m.params.clock.w = amount;
            if matches!(*effect, Effect::Water) {
                // Sample depths do not depend on the pixel. Calculate their
                // phase/attenuation once instead of millions of times in WGSL.
                let clock = m.params.clock.x * 0.23;
                for i in 0..amount as usize {
                    let z = (i as f32 + 0.5) / amount;
                    let pair = i % 2 * 2;
                    m.params.depth_samples[i / 2][pair] = z * 4.08 + (z * 4.0 + clock).sin() * 0.35;
                    m.params.depth_samples[i / 2][pair + 1] = (-z * 2.0).exp() / amount;
                }
            }
        }
    }
}
// Preserve total sunlight when changing shadow strength. Balanced contributes
// only 12% shadowed light; broad fill keeps faces readable in the reef's shade.
fn sunlight_weight(shadow_layer: bool, quality: u8) -> f32 {
    let shadow = if quality == 2 { 0.22 } else { 0.12 };
    if shadow_layer { shadow } else { 1.0 - shadow }
}
fn lighting(
    p: Res<Preferences>,
    u: Res<SessionUi>,
    mut sun: Query<(&Sun, &mut DirectionalLight, &mut Transform)>,
    options: Res<Options>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut cameras: Query<(&mut Fxaa, &mut Msaa), With<Camera3d>>,
    mut reef: ResMut<Assets<ReefMaterial>>,
) {
    let motion = if p.reduced_motion { 0.15 } else { 1.0 };
    let size = if p.quality == 2 { 1024 } else { 512 };
    if shadow_map.size != size {
        shadow_map.size = size;
    }
    for (layer, mut light, mut tr) in &mut sun {
        light.illuminance = 7800.0
            * sunlight_weight(layer.shadow_layer, p.quality)
            * (1.15 + (0.10 * (u.elapsed * 0.27).sin() + 0.05 * (u.elapsed * 0.61).sin()) * motion);
        light.shadow_maps_enabled = layer.shadow_layer && p.quality > 0;
        tr.rotation = Quat::from_euler(
            EulerRot::YXZ,
            (-25.0 + 7.0 * (u.elapsed * 0.09).sin() * motion).to_radians(),
            (-38.0 + 5.0 * (u.elapsed * 0.11).sin() * motion).to_radians(),
            -20_f32.to_radians(),
        );
    }
    for (mut fxaa, mut msaa) in &mut cameras {
        let probe_fxaa = options.capture.is_some() && options.capture_fxaa;
        let enabled = p.quality == 2 && probe_fxaa;
        let samples = if p.quality == 2 && !probe_fxaa {
            Msaa::Sample2
        } else {
            Msaa::Off
        };
        if *msaa != samples {
            *msaa = samples;
        }
        if fxaa.enabled != enabled {
            fxaa.enabled = enabled;
        }
    }
    for (_, m) in reef.iter_mut() {
        m.extension.settings = Vec4::new(
            u.elapsed,
            if p.reduced_motion { 0.0 } else { 1.0 },
            if p.quality == 0 { 0.45 } else { 1.0 },
            0.0,
        );
    }
}
fn capture(
    mut c: Commands,
    options: Res<Options>,
    u: Res<SessionUi>,
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut taken: Local<bool>,
    mut exit: MessageWriter<AppExit>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut frame_times: Local<Vec<f32>>,
    real_time: Res<Time<Real>>,
) {
    let Some(kind) = &options.capture else { return };
    if !u.loaded {
        return;
    }
    *elapsed += time.delta_secs();
    let deadline = if options.school { 20.0 } else { 4.0 };
    if *elapsed > if options.school { 5.0 } else { 1.0 } && !*taken {
        frame_times.push(real_time.delta_secs() * 1000.0);
    }
    if *elapsed > deadline && !*taken {
        if !frame_times.is_empty() {
            frame_times.sort_by(f32::total_cmp);
            let percentile = |p: f32| frame_times[((frame_times.len() - 1) as f32 * p) as usize];
            let mean = frame_times.iter().sum::<f32>() / frame_times.len() as f32;
            println!(
                "FRAME PACING {kind}: mean {mean:.2} ms, p50 {:.2}, p95 {:.2}, p99 {:.2}, max {:.2}; {}/{} frames over 25 ms",
                percentile(0.5),
                percentile(0.95),
                percentile(0.99),
                frame_times.last().unwrap(),
                frame_times.iter().filter(|&&t| t > 25.0).count(),
                frame_times.len()
            );
        }
        for diagnostic in diagnostics.iter() {
            if diagnostic.path().as_str().starts_with("render/") {
                println!("RENDER {}: {:?}", diagnostic.path(), diagnostic.average());
            }
        }
        *taken = true;
        if let Some(fps) = diagnostics
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.average())
        {
            println!("CAPTURE {kind}: recent mean {fps:.1} FPS");
        }
        c.spawn(Screenshot::primary_window()).observe(save_to_disk(
            std::env::temp_dir().join(format!("sabrina-bevy-{kind}.png")),
        ));
    }
    if *elapsed > deadline + 2.0 {
        exit.write(AppExit::Success);
    }
}

fn crystal_light(
    w: Res<Presentation>,
    mut lights: Query<(&mut PointLight, &mut Transform), With<MagicLight>>,
    mut materials: ResMut<Assets<CrystalMaterial>>,
) {
    let c = &w.0.crystal;
    let charging = c.magic.state == sim::Spell::Charging;
    let burst = c.magic.state == sim::Spell::Burst;
    let ready = w.0.sabrina.cooldown <= 0.0;
    for (mut light, mut t) in &mut lights {
        t.translation = Vec3::new(c.motion.pos.x, c.motion.pos.y, c.magic.depth + 0.6);
        light.intensity = 180.0
            * if charging {
                1.4
            } else if burst {
                0.8
            } else if ready || c.magic.state == sim::Spell::Flying {
                0.45
            } else {
                0.0
            };
    }
    for (_, m) in materials.iter_mut() {
        m.extension.settings = Vec4::new(
            w.0.session.time,
            if charging {
                (c.magic.time / 1.1).min(1.0)
            } else {
                0.0
            },
            0.0,
            0.0,
        );
    }
}
