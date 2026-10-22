// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Tail-only recoloring keeps the authored textures and standard PBR lighting.
use crate::{
    assets::{Actor, CharacterSurface},
    presentation::Presentation,
    settings::Preferences,
};
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(crate) struct TailExtension {
    #[uniform(100)]
    pub color: Vec4,
}

impl MaterialExtension for TailExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/tail.wesl".into()
    }
}

pub(crate) type TailMaterial = ExtendedMaterial<StandardMaterial, TailExtension>;

#[derive(Component)]
struct TailSurface {
    normal: Handle<TailMaterial>,
    hurt: Handle<TailMaterial>,
}

// These authored parts are the tail body, flukes, hip fins and lower fins.
fn is_tail(name: &str) -> bool {
    matches!(
        name,
        "Sabrina_Part_1"
            | "Sabrina_Part_2"
            | "Sabrina_Part_7"
            | "Sabrina_Part_8"
            | "Sabrina_Part_9"
            | "Sabrina_Part_10"
    )
}

fn tint(preferences: &Preferences) -> Vec4 {
    preferences.tail_color.map_or(Vec4::ZERO, |[r, g, b]| {
        let color = Color::srgb_u8(r, g, b).to_linear();
        Vec4::new(color.red, color.green, color.blue, 1.0)
    })
}

pub struct TailPlugin;

impl Plugin for TailPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TailMaterial>::default())
            .add_systems(
                Update,
                (bind, update).chain().after(crate::presentation::prepare),
            );
    }
}

fn bind(
    mut commands: Commands,
    surfaces: Query<
        (Entity, &CharacterSurface, &bevy::gltf::GltfMaterialName),
        Added<CharacterSurface>,
    >,
    standard: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<TailMaterial>>,
    preferences: Res<Preferences>,
) {
    for (entity, surface, name) in &surfaces {
        if surface.actor != Actor::Sabrina || !is_tail(&name.0) {
            continue;
        }
        let (Some(normal), Some(hurt)) =
            (standard.get(&surface.normal), standard.get(&surface.hurt))
        else {
            warn!("Tail surface {} has no loaded base materials", name.0);
            continue;
        };
        let normal = materials.add(ExtendedMaterial {
            base: normal.clone(),
            extension: TailExtension {
                color: tint(&preferences),
            },
        });
        let hurt = materials.add(ExtendedMaterial {
            base: hurt.clone(),
            extension: TailExtension { color: Vec4::ZERO },
        });
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(normal.clone()), TailSurface { normal, hurt }));
    }
}

fn update(
    preferences: Res<Preferences>,
    rendered: Res<Presentation>,
    mut materials: ResMut<Assets<TailMaterial>>,
    mut surfaces: Query<(&TailSurface, &mut MeshMaterial3d<TailMaterial>)>,
) {
    let color = preferences.is_changed().then(|| tint(&preferences));
    let vitality = &rendered.0.sabrina.vitality;
    let hurt = vitality.hurt_flash > 0.0 || vitality.stunned > 0.0;
    for (surface, mut handle) in &mut surfaces {
        // Do not mark assets changed (and re-upload them) on unchanged frames.
        if let Some(color) = color
            && materials
                .get(&surface.normal)
                .is_some_and(|material| material.extension.color != color)
        {
            materials.get_mut(&surface.normal).unwrap().extension.color = color;
        }
        let desired = if hurt { &surface.hurt } else { &surface.normal };
        if handle.0 != *desired {
            handle.0 = desired.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_disables_tint_and_custom_color_is_linear() {
        let mut preferences = Preferences::default();
        assert_eq!(tint(&preferences), Vec4::ZERO);
        preferences.tail_color = Some([128, 0, 255]);
        let color = tint(&preferences);
        assert!((color.x - 0.21586).abs() < 0.0001);
        assert_eq!((color.y, color.z, color.w), (0.0, 1.0, 1.0));
    }

    #[test]
    fn only_tail_surfaces_are_bound_and_other_materials_are_untouched() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_asset::<TailMaterial>()
            .init_resource::<Preferences>()
            .init_resource::<Presentation>()
            .add_systems(Update, (bind, update).chain());
        let texture = Handle::<Image>::default();
        let base = StandardMaterial {
            base_color_texture: Some(texture.clone()),
            normal_map_texture: Some(texture.clone()),
            metallic_roughness_texture: Some(texture.clone()),
            perceptual_roughness: 0.38,
            metallic: 0.21,
            ..default()
        };
        let normal = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(base.clone());
        let hurt = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.055, 0.09),
                ..base
            });
        let mut tails = Vec::new();
        let mut others = Vec::new();
        for name in [
            "Sabrina_Hair",
            "Sabrina_Face",
            "Sabrina_Part_1",
            "Sabrina_Part_2",
            "Sabrina_Part_3",
            "Sabrina_Part_4",
            "Sabrina_Part_5",
            "Sabrina_Part_6",
            "Sabrina_Part_7",
            "Sabrina_Part_8",
            "Sabrina_Part_9",
            "Sabrina_Part_10",
        ] {
            let entity = app
                .world_mut()
                .spawn((
                    CharacterSurface {
                        actor: Actor::Sabrina,
                        normal: normal.clone(),
                        hurt: hurt.clone(),
                    },
                    bevy::gltf::GltfMaterialName(name.into()),
                    MeshMaterial3d(normal.clone()),
                ))
                .id();
            if is_tail(name) {
                tails.push(entity);
            } else {
                others.push(entity);
            }
        }
        app.update();
        assert_eq!(tails.len(), 6);
        for entity in &others {
            assert!(app.world().get::<TailSurface>(*entity).is_none());
            assert_eq!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(*entity)
                    .unwrap()
                    .0,
                normal
            );
        }
        for entity in &tails {
            assert!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(*entity)
                    .is_none()
            );
            let surface = app.world().get::<TailSurface>(*entity).unwrap();
            let material = app
                .world()
                .resource::<Assets<TailMaterial>>()
                .get(&surface.normal)
                .unwrap();
            assert_eq!(material.base.base_color_texture, Some(texture.clone()));
            assert_eq!(material.base.normal_map_texture, Some(texture.clone()));
            assert_eq!(
                material.base.metallic_roughness_texture,
                Some(texture.clone())
            );
            assert_eq!(material.base.perceptual_roughness, 0.38);
            assert_eq!(material.base.metallic, 0.21);
            assert_eq!(material.extension.color, Vec4::ZERO);
        }
        app.world_mut().resource_mut::<Preferences>().tail_color = Some([255, 0, 128]);
        app.update();
        for entity in &tails {
            let surface = app.world().get::<TailSurface>(*entity).unwrap();
            assert_eq!(
                app.world()
                    .resource::<Assets<TailMaterial>>()
                    .get(&surface.normal)
                    .unwrap()
                    .extension
                    .color,
                tint(app.world().resource::<Preferences>())
            );
        }
        app.world_mut()
            .resource_mut::<Presentation>()
            .0
            .sabrina
            .vitality
            .hurt_flash = 1.0;
        app.update();
        for entity in &tails {
            let surface = app.world().get::<TailSurface>(*entity).unwrap();
            assert_eq!(
                app.world()
                    .get::<MeshMaterial3d<TailMaterial>>(*entity)
                    .unwrap()
                    .0,
                surface.hurt
            );
            assert_eq!(
                app.world()
                    .resource::<Assets<TailMaterial>>()
                    .get(&surface.hurt)
                    .unwrap()
                    .extension
                    .color,
                Vec4::ZERO
            );
        }
        app.world_mut()
            .resource_mut::<Presentation>()
            .0
            .sabrina
            .vitality
            .hurt_flash = 0.0;
        app.world_mut().resource_mut::<Preferences>().tail_color = None;
        app.update();
        for entity in &tails {
            let surface = app.world().get::<TailSurface>(*entity).unwrap();
            assert_eq!(
                app.world()
                    .get::<MeshMaterial3d<TailMaterial>>(*entity)
                    .unwrap()
                    .0,
                surface.normal
            );
            assert_eq!(
                app.world()
                    .resource::<Assets<TailMaterial>>()
                    .get(&surface.normal)
                    .unwrap()
                    .extension
                    .color,
                Vec4::ZERO
            );
        }
        app.update();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<TailMaterial>>>()
            .clear();
        app.update();
        assert!(
            !app.world_mut()
                .resource_mut::<Messages<AssetEvent<TailMaterial>>>()
                .drain()
                .any(|event| matches!(event, AssetEvent::Modified { .. }))
        );
        assert_eq!(app.world().resource::<Assets<TailMaterial>>().len(), 12);
    }
}
