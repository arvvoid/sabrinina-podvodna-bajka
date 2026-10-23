// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Pooled billboard emitters; particle motion is evaluated in the vertex shader.
use crate::{app::SessionUi, presentation::Presentation, settings::Preferences, visuals::Effect};
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::Indices,
    prelude::*,
    render::render_resource::{AsBindGroup, PrimitiveTopology},
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct ParticleMaterial {
    #[uniform(0)]
    tint: Vec4,
    // Time, kind, motion scale, active count.
    #[uniform(1)]
    settings: Vec4,
    // Burst progress, variation seed, reserved.
    #[uniform(2)]
    state: Vec4,
}

impl Material for ParticleMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/particles.wesl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/particles.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

#[derive(Component)]
pub(crate) struct ParticleEmitter;

pub(crate) struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ParticleMaterial>::default())
            .add_systems(Update, setup.in_set(crate::loading::LoadResources))
            .add_systems(Update, update.after(crate::presentation::prepare));
    }
}

fn particle_mesh(count: usize) -> Mesh {
    let mut positions = Vec::with_capacity(count * 4);
    let mut uvs = Vec::with_capacity(count * 4);
    let mut colors = Vec::with_capacity(count * 4);
    let mut indices = Vec::with_capacity(count * 6);
    for i in 0..count {
        let first = (i * 4) as u32;
        for uv in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
            positions.push([0.0, 0.0, 0.0]);
            uvs.push(uv);
            colors.push([i as f32, 0.0, 0.0, 1.0]);
        }
        indices.extend_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn material(tint: Color, kind: f32, count: f32) -> ParticleMaterial {
    let color = tint.to_linear();
    ParticleMaterial {
        tint: Vec4::new(color.red, color.green, color.blue, 1.0),
        settings: Vec4::new(0.0, kind, 1.0, count),
        state: Vec4::ZERO,
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    let single = meshes.add(particle_mesh(1));
    let burst_mesh = meshes.add(particle_mesh(6));
    let spark_mesh = meshes.add(particle_mesh(24));
    let plankton_mesh = meshes.add(particle_mesh(80));
    let trail = materials.add(material(Color::WHITE, 0.0, 1.0));
    for i in 0..24 {
        commands.spawn((
            Effect::Trail(i),
            Mesh3d(single.clone()),
            MeshMaterial3d(trail.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            NoFrustumCulling,
        ));
    }
    for (effect, count, kind, tint) in [
        (Effect::Plankton, 80, 3.0, Color::srgb(0.8, 0.85, 0.85)),
        (Effect::Spark, 24, 1.0, Color::srgb(0.75, 0.95, 1.0)),
    ]
    .into_iter()
    .chain((0..72).map(|i| (Effect::Pop(i), 6, 2.0, Color::srgb(1.0, 0.4, 0.45))))
    {
        commands.spawn((
            effect,
            ParticleEmitter,
            Mesh3d(match effect {
                Effect::Plankton => plankton_mesh.clone(),
                Effect::Spark => spark_mesh.clone(),
                _ => burst_mesh.clone(),
            }),
            MeshMaterial3d(materials.add(material(tint, kind, count as f32))),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            NoFrustumCulling,
        ));
    }
}

fn plankton_count(preferences: &Preferences) -> f32 {
    if preferences.reduced_motion {
        0.0
    } else {
        match preferences.quality {
            0 => 18.0,
            1 => 45.0,
            _ => 80.0,
        }
    }
}

fn update(
    rendered: Res<Presentation>,
    ui: Res<SessionUi>,
    preferences: Res<Preferences>,
    mut emitters: Query<
        (
            &Effect,
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<ParticleMaterial>,
        ),
        With<ParticleEmitter>,
    >,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    let crystal = &rendered.0.crystal;
    let resting = crystal.magic.state == reef_sim::Spell::Resting;
    let burst = crystal.magic.state == reef_sim::Spell::Burst;
    for (effect, mut transform, mut visibility, handle) in &mut emitters {
        let mut position = Vec3::ZERO;
        let mut count = 0.0;
        let mut state = Vec4::ZERO;
        let show = match *effect {
            Effect::Plankton => {
                count = plankton_count(&preferences);
                count > 0.0
            }
            Effect::Spark => {
                count = 24.0;
                position = Vec3::new(
                    crystal.motion.pos.x,
                    crystal.motion.pos.y,
                    crystal.magic.depth,
                );
                state.x = if burst {
                    crystal.magic.time / 0.7
                } else {
                    -1.0
                };
                !ui.screen.is_reading() && (!resting || rendered.0.sabrina.cooldown <= 0.0)
            }
            Effect::Pop(i) => {
                let fish = &rendered.0.fish[i];
                count = 6.0;
                position = Vec3::new(fish.motion.pos.x, fish.motion.pos.y, fish.appearance.depth);
                state.x = ((0.30 - fish.life.pop_time) / 0.30).clamp(0.0, 1.0);
                fish.life.pop_time > 0.0 && fish.life.pop_time < 0.30
            }
            _ => false,
        };
        let show = show || !ui.loaded;
        visibility.set_if_neq(if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        if !show {
            continue;
        }
        if transform.translation != position {
            transform.translation = position;
        }
        let Some(current) = materials.get(&handle.0) else {
            error!("Particle emitter has no material: {effect:?}");
            continue;
        };
        let settings = Vec4::new(
            rendered.0.session.time,
            current.settings.y,
            if preferences.reduced_motion {
                0.15
            } else {
                1.0
            },
            count,
        );
        if current.settings != settings || current.state != state {
            let mut material = materials.get_mut(&handle.0).unwrap();
            material.settings = settings;
            material.state = state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_are_batched_quads_not_spheres() {
        for count in [1, 6, 24, 80] {
            let mesh = particle_mesh(count);
            assert_eq!(mesh.count_vertices(), count * 4);
            assert_eq!(mesh.indices().unwrap().len(), count * 6);
        }
        assert!(!ParticleMaterial::enable_prepass());
        assert!(!ParticleMaterial::enable_shadows());
    }

    #[test]
    fn particle_density_preserves_quality_and_reduced_motion() {
        let mut preferences = Preferences::default();
        for (quality, expected) in [(0, 18.0), (1, 45.0), (2, 80.0)] {
            preferences.quality = quality;
            assert_eq!(plankton_count(&preferences), expected);
            preferences.reduced_motion = true;
            assert_eq!(plankton_count(&preferences), 0.0);
            preferences.reduced_motion = false;
        }
    }

    #[test]
    fn emitters_preserve_cooldowns_bursts_and_static_transforms() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ParticleMaterial>>()
            .init_resource::<Presentation>()
            .init_resource::<Preferences>()
            .insert_resource(SessionUi {
                loaded: true,
                ..default()
            })
            .add_systems(Startup, setup)
            .add_systems(Update, update);
        app.update();
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 4);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ParticleEmitter>>()
                .iter(app.world())
                .count(),
            74
        );
        let entities: Vec<_> = app
            .world_mut()
            .query::<(Entity, &Effect)>()
            .iter(app.world())
            .filter_map(|(entity, effect)| {
                matches!(effect, Effect::Plankton | Effect::Spark | Effect::Pop(0))
                    .then_some((entity, *effect))
            })
            .collect();
        let entity = |predicate: fn(&Effect) -> bool| {
            entities
                .iter()
                .find(|(_, effect)| predicate(effect))
                .unwrap()
                .0
        };
        let plankton = entity(|effect| matches!(effect, Effect::Plankton));
        let spark = entity(|effect| matches!(effect, Effect::Spark));
        let pop = entity(|effect| matches!(effect, Effect::Pop(0)));
        assert_eq!(
            *app.world().get::<Visibility>(spark).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(pop).unwrap(),
            Visibility::Hidden
        );
        {
            let mut rendered = app.world_mut().resource_mut::<Presentation>();
            rendered.0.session.time = 4.0;
            rendered.0.sabrina.cooldown = 5.0;
            rendered.0.crystal.magic.state = reef_sim::Spell::Resting;
            rendered.0.fish[0].life.pop_time = 0.15;
        }
        app.update();
        assert_eq!(
            *app.world().get::<Transform>(plankton).unwrap(),
            Transform::default()
        );
        assert_eq!(
            *app.world().get::<Visibility>(spark).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(pop).unwrap(),
            Visibility::Visible
        );
        let pop_material = &app
            .world()
            .get::<MeshMaterial3d<ParticleMaterial>>(pop)
            .unwrap()
            .0;
        assert!(
            (app.world()
                .resource::<Assets<ParticleMaterial>>()
                .get(pop_material)
                .unwrap()
                .state
                .x
                - 0.5)
                .abs()
                < 0.0001
        );
        app.world_mut().resource_mut::<Preferences>().reduced_motion = true;
        app.world_mut()
            .resource_mut::<Presentation>()
            .0
            .sabrina
            .cooldown = 0.0;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(plankton).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(spark).unwrap(),
            Visibility::Visible
        );
    }
}
