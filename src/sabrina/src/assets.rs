// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Shared glTF resources, per-actor rigs and presentation-only pose corrections.
use crate::{
    app::{Options, Screen, SessionUi, Simulation, begin},
    presentation::Presentation,
    settings::Preferences,
};
use bevy::{
    app::AnimationSystems,
    camera::{primitives::MeshAabb, visibility::NoFrustumCulling},
    gltf::GltfMesh,
    prelude::*,
    transform::TransformSystems,
    world_serialization::WorldInstanceReady,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    Sabrina,
    Fish(usize),
    Jelly(usize),
    Crystal,
    Reef,
}

#[derive(Component)]
pub struct ActorVisual {
    pub model: Option<Entity>,
    pub variant: usize,
    pub yaw: f32,
}

#[derive(Component, Clone)]
pub struct RigSpec {
    pub actor: Actor,
    pub asset: usize,
}

#[derive(Component, Clone)]
pub struct RigPlayer {
    pub actor: Actor,
    pub generation: u64,
    pub clips: Arc<HashMap<String, (AnimationNodeIndex, f32)>>,
    pub current: String,
    pub phase: f32,
    blend: f32,
    outgoing: Vec<(AnimationNodeIndex, f32)>,
}

struct ReadingAnimations {
    players: Vec<(Entity, AnimationPlayer, RigPlayer)>,
    bones: Vec<(Entity, Transform, Option<Quat>)>,
}

#[derive(Component)]
pub struct CharacterSurface {
    pub actor: Actor,
    pub normal: Handle<StandardMaterial>,
    pub hurt: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct Palm;

#[derive(Component)]
struct FacialRig;

#[derive(Component)]
pub struct CorrectBone {
    pub root: Entity,
    pub name: String,
    pub original: Option<Quat>,
    pub rest_axis: Vec3,
}

#[derive(Resource, Default)]
pub struct Library {
    pub handles: Vec<Handle<Gltf>>,
    pub ready: bool,
    pub lods: HashMap<usize, Handle<Gltf>>,
}

pub const FILES: [&str; 16] = [
    "models/sabrina.glb",
    "models/clownfish.glb",
    "models/blue_tang.glb",
    "models/butterflyfish.glb",
    "models/bannerfish.glb",
    "models/parrotfish.glb",
    "models/blue_wrasse.glb",
    "models/fire_goby.glb",
    "models/royal_tang.glb",
    "models/yellow_tang.glb",
    "models/emperor_angelfish.glb",
    "models/coral_anthias.glb",
    "models/cardinalfish.glb",
    "models/jellyfish.glb",
    "environments/reef_ruins.glb",
    "props/ocean_crystal.glb",
];

type ClipMap = Arc<HashMap<String, (AnimationNodeIndex, f32)>>;

#[derive(Resource, Default)]
struct RigCache {
    graphs: HashMap<usize, (Handle<AnimationGraph>, ClipMap)>,
    materials: HashMap<
        (AssetId<StandardMaterial>, u8),
        (Handle<StandardMaterial>, Handle<StandardMaterial>),
    >,
}

#[derive(Component)]
pub struct RenderLod {
    actor: Actor,
    meshes: [Handle<Mesh>; 3],
    diameter: f32,
    level: usize,
}

fn select_lods(
    rendered: Res<crate::presentation::Presentation>,
    prefs: Res<Preferences>,
    camera: Single<(&Camera, &Projection), With<Camera3d>>,
    mut query: Query<(&mut RenderLod, &mut Mesh3d)>,
) {
    let (camera, projection) = *camera;
    let Projection::Orthographic(ortho) = projection else {
        return;
    };
    let Some(viewport) = camera.physical_viewport_size() else {
        return;
    };
    let density = viewport.y as f32 / ortho.area.height();
    for (mut lod, mut mesh) in &mut query {
        let metric = match lod.actor {
            Actor::Fish(i) => {
                density
                    * lod.diameter
                    * rendered.0.fish[i].appearance.size
                    * (0.3 + 0.7 * rendered.0.fish[i].appearance.birth)
            }
            _ => density,
        };
        let (low, high) = if lod.actor == Actor::Reef {
            (if prefs.quality == 2 { 30.0 } else { 48.0 }, 100.0)
        } else {
            (65.0, 160.0)
        };
        let level = if metric < low {
            2
        } else if metric < high {
            1
        } else {
            0
        };
        // Keep a margin around level boundaries to prevent rapid alternation.
        let threshold = if level.max(lod.level) == 2 { low } else { high };
        if level != lod.level && (metric - threshold).abs() > threshold * 0.08 {
            mesh.0 = lod.meshes[level].clone();
            lod.level = level;
        }
    }
}

fn fish_shadows(
    mut commands: Commands,
    preferences: Res<Preferences>,
    surfaces: Query<(Entity, &CharacterSurface, Has<bevy::light::NotShadowCaster>)>,
) {
    for (entity, surface, excluded) in &surfaces {
        if !matches!(surface.actor, Actor::Fish(_)) {
            continue;
        }
        if preferences.quality == 2 && excluded {
            commands
                .entity(entity)
                .remove::<bevy::light::NotShadowCaster>();
        } else if preferences.quality != 2 && !excluded {
            commands.entity(entity).insert(bevy::light::NotShadowCaster);
        }
    }
}

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Library>()
            .init_resource::<RigCache>()
            .add_systems(
                Update,
                load.in_set(crate::loading::LoadResources)
                    .before(crate::loading::ready),
            )
            .add_systems(
                Update,
                (crate::loading::ready, finish_load, sync_models)
                    .chain()
                    .after(crate::loading::LoadResources)
                    .run_if(crate::loading::resources_allowed),
            )
            .add_systems(
                Update,
                animate
                    .after(crate::presentation::prepare)
                    .after(sync_models),
            )
            .add_systems(Update, select_lods.after(crate::presentation::prepare))
            .add_systems(Update, fish_shadows.after(crate::presentation::prepare))
            .add_systems(Update, report_assets.after(select_lods))
            .add_systems(PostUpdate, restore_bones.before(AnimationSystems))
            .add_systems(
                PostUpdate,
                (correct_bones, facial_expression, attach_crystal)
                    .chain()
                    .after(AnimationSystems)
                    .before(TransformSystems::Propagate),
            );
    }
}

fn load(server: Res<AssetServer>, mut library: ResMut<Library>) {
    library.handles = FILES.iter().map(|&p| server.load(p)).collect();
    for i in (1..=12).chain(std::iter::once(14)) {
        let file = FILES[i].rsplit('/').next().unwrap();
        library
            .lods
            .insert(i, server.load(format!("render_lods/{file}")));
    }
}

pub(crate) fn finish_load(
    mut commands: Commands,
    gltfs: Res<Assets<Gltf>>,
    mut lib: ResMut<Library>,
    mut ui: ResMut<SessionUi>,
    mut simulation: ResMut<Simulation>,
    preferences: Res<Preferences>,
    options: Res<Options>,
    server: Res<AssetServer>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<Assets<bevy::gltf::GltfMaterial>>,
    mut images: ResMut<Assets<Image>>,
    rigs: Query<&RigPlayer>,
    loading: Res<crate::loading::Loading>,
) {
    if ui.loaded {
        return;
    }
    if !lib.ready {
        if !lib
            .handles
            .iter()
            .chain(lib.lods.values())
            .all(|h| gltfs.contains(h) && server.is_loaded_with_dependencies(h.id()))
        {
            return;
        }
        for handle in lib.lods.values() {
            for mesh in &gltfs.get(handle).unwrap().meshes {
                for primitive in &gltf_meshes.get(mesh).unwrap().primitives {
                    let mut mesh = meshes.get_mut(&primitive.mesh).unwrap();
                    if !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) {
                        mesh.generate_tangents()
                            .expect("Render LOD has normals and UVs");
                    }
                }
            }
        }
        crate::mipmaps::prepare(&lib, &gltfs, &materials, &mut images);
        lib.ready = true;
        for actor in [Actor::Reef, Actor::Sabrina, Actor::Crystal]
            .into_iter()
            .chain((0..reef_sim::FISH_COUNT).map(Actor::Fish))
            .chain((0..reef_sim::JELLY_COUNT).map(Actor::Jelly))
        {
            commands.spawn((
                actor,
                ActorVisual {
                    model: None,
                    variant: usize::MAX,
                    yaw: 0.0,
                },
                Transform::default(),
                Visibility::default(),
            ));
        }
        return;
    }
    if rigs.iter().count() < 1 + reef_sim::FISH_COUNT + reef_sim::JELLY_COUNT {
        return;
    }
    if loading.progress < 1.0 {
        return;
    }
    ui.loaded = true;
    if let Some(kind) = &options.capture {
        ui.screen = Screen::Menu;
        let short = kind
            .strip_prefix("hr-")
            .or_else(|| kind.strip_prefix("it-"))
            .unwrap_or(kind);
        match short {
            "boot" => ui.screen = Screen::Loading,
            "menu" => {}
            "settings" => ui.screen = Screen::Settings,
            "tail-color" => ui.screen = Screen::TailColor,
            "language" => ui.screen = Screen::Language,
            "victory" => {
                ui.screen = Screen::Victory;
                simulation.0.session.cycle = reef_sim::FINAL_ROUND;
                simulation.0.session.victory = true;
                simulation.0.points.total_score = 12500;
                simulation.0.points.last_survivors = 24;
            }
            "help" => ui.screen = Screen::Help,
            "credits" => ui.screen = Screen::Credits,
            _ => {
                begin(&mut simulation.0, &mut ui, &preferences, true);
                if options.school {
                    reef_sim::add_fish(&mut simulation.0, 72, false);
                }
                if matches!(
                    short,
                    "visit" | "crystal" | "throw" | "wave" | "song" | "combat" | "demo" | "finale"
                ) {
                    simulation.0.settings.aquarium = false;
                    if short == "finale" {
                        simulation.0.session.cycle = reef_sim::FINAL_ROUND - 1;
                    }
                    reef_sim::begin_visit(&mut simulation.0);
                }
            }
        }
    }
    if options
        .capture
        .as_deref()
        .is_some_and(|k| k.starts_with("graphics-") || k.starts_with("pose-") || k == "rig")
    {
        simulation.0.sabrina.motion.pos = reef_sim::v(0.0, 8.0);
        simulation.0.sabrina.motion.velocity = reef_sim::v(4.0, 0.0);
        simulation.0.sabrina.swimming.pitch = 0.05;
        simulation.0.sabrina.swimming.yaw = 0.0;
        simulation.0.sabrina.swimming.phase = 1.2;
        simulation.0.sabrina.cooldown = 1000.0;
        for f in &mut simulation.0.fish {
            f.life.active = false;
        }
        if let Some(k) = &options.capture {
            if k == "pose-idle" {
                simulation.0.sabrina.motion.velocity = reef_sim::v(0.0, 0.0);
                simulation.0.sabrina.swimming.pitch = 0.65;
            }
            if k.contains("dive") {
                simulation.0.sabrina.swimming.pitch = -1.1;
                simulation.0.sabrina.motion.velocity = reef_sim::v(2.0, -4.0);
            }
            if k.contains("climb") {
                simulation.0.sabrina.swimming.pitch = 1.1;
                simulation.0.sabrina.motion.velocity = reef_sim::v(2.0, 4.0);
            }
            if k.contains("left") {
                simulation.0.sabrina.swimming.yaw = std::f32::consts::PI;
                simulation.0.sabrina.swimming.facing = -1.0;
                simulation.0.sabrina.motion.velocity.x *= -1.0;
            }
            if k.starts_with("pose-wave") || k.starts_with("pose-throw") {
                let s = &mut simulation.0.sabrina;
                if k.contains("upright") {
                    s.motion.velocity = reef_sim::v(0.0, 0.0);
                    s.swimming.pitch = 0.65;
                }
                let pitch = if k.contains("upright") {
                    0.0
                } else {
                    s.swimming.pitch
                };
                let direction = reef_sim::v(
                    if k.contains("left") {
                        -pitch.cos()
                    } else {
                        pitch.cos()
                    },
                    pitch.sin(),
                );
                if k.starts_with("pose-wave") {
                    s.wave.active = true;
                    s.wave.time = 0.18;
                    s.wave.direction = direction;
                } else {
                    s.throw_time = if k.contains("gather") { 0.8 } else { 1.1 };
                    s.throw_direction = direction;
                }
            }
        }
    }
    if options.capture.as_deref() == Some("throw") {
        simulation.0.sabrina.motion.pos = reef_sim::v(-2.0, 8.0);
        simulation.0.jelly[0].motion.pos = reef_sim::v(4.0, 9.0);
        simulation.0.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
        simulation.0.jelly[0].behaviour.turn_in = 100.0;
    }
    if options.capture.as_deref() == Some("combat") {
        simulation.0.settings.gentle = true;
        simulation.0.sabrina.motion.pos = reef_sim::v(-3.0, 8.0);
        simulation.0.jelly[0].motion.pos = reef_sim::v(3.0, 9.0);
        simulation.0.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
    }
    if options.capture.as_deref() == Some("demo") {
        reef_sim::reset(
            &mut simulation.0,
            27,
            reef_sim::Settings {
                gentle: true,
                visitor_limit: 1,
                ..default()
            },
        );
        reef_sim::begin_visit(&mut simulation.0);
        simulation.0.sabrina.motion.pos = reef_sim::v(-3.0, 8.0);
        simulation.0.jelly[0].motion.pos = reef_sim::v(3.0, 9.0);
        simulation.0.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
        reef_sim::add_fish(&mut simulation.0, 16, false);
    }
    if options.verify {
        begin(&mut simulation.0, &mut ui, &preferences, true);
    }
    if let Some(name) = match options.capture.as_deref() {
        Some("xbox") => Some("Xbox Controller"),
        Some("playstation") => Some("DualSense Wireless Controller"),
        Some("generic-pad") => Some("Generic USB Controller"),
        _ => None,
    } {
        commands.spawn((Gamepad::default(), Name::new(name)));
    }
    if options
        .capture
        .as_deref()
        .is_some_and(|kind| kind.starts_with("occlusion"))
    {
        reef_sim::reset(
            &mut simulation.0,
            27,
            reef_sim::Settings {
                aquarium: true,
                ..default()
            },
        );
        simulation.0.sabrina.motion.pos = reef_sim::v(-10.0, 3.5);
        for fish in &mut simulation.0.fish {
            fish.life.active = false;
        }
    }
    info!("All 16 finished GLB assets loaded");
}

fn sync_models(
    mut commands: Commands,
    lib: Res<Library>,
    gltfs: Res<Assets<Gltf>>,
    simulation: Res<Simulation>,
    mut roots: Query<(Entity, &Actor, &mut ActorVisual)>,
) {
    if !lib.ready {
        return;
    }
    for (root, actor, mut view) in &mut roots {
        let asset = match *actor {
            Actor::Sabrina => 0,
            Actor::Fish(i) => {
                1 + simulation.0.fish[i].appearance.species + if i % 2 == 0 { 0 } else { 6 }
            }
            Actor::Jelly(_) => 13,
            Actor::Reef => 14,
            Actor::Crystal => 15,
        };
        if view.variant == asset {
            continue;
        }
        if let Some(old) = view.model {
            commands.entity(old).despawn();
        }
        let Some(gltf) = gltfs.get(&lib.handles[asset]) else {
            continue;
        };
        let spec = RigSpec {
            actor: *actor,
            asset,
        };
        let scale = if *actor == Actor::Sabrina { 0.88 } else { 1.0 };
        let model = commands
            .spawn((
                WorldAssetRoot(gltf.scenes[0].clone()),
                Transform::from_scale(Vec3::splat(scale)),
                spec,
            ))
            .observe(rig_ready)
            .id();
        commands.entity(root).add_child(model);
        view.model = Some(model);
        view.variant = asset;
    }
}

fn rig_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    specs: Query<&RigSpec>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    surfaces: Query<(
        &MeshMaterial3d<StandardMaterial>,
        Option<&bevy::gltf::GltfMaterialName>,
    )>,
    mesh_data: (
        Query<&Mesh3d>,
        Res<Assets<GltfMesh>>,
        Res<Assets<Mesh>>,
        Query<(), With<MorphWeights>>,
    ),
    transforms: Query<&Transform>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    lib: Res<Library>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut cache: ResMut<RigCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (mesh_handles, mesh_assets, meshes, morphs) = mesh_data;
    let Ok(spec) = specs.get(event.entity) else {
        return;
    };
    let Some(gltf) = gltfs.get(&lib.handles[spec.asset]) else {
        return;
    };
    let (graph, map) = cache
        .graphs
        .entry(spec.asset)
        .or_insert_with(|| {
            let mut graph = AnimationGraph::new();
            let mut map = HashMap::new();
            for (name, clip) in &gltf.named_animations {
                let node = graph.add_clip(clip.clone(), 1.0, graph.root);
                let duration = clips.get(clip).map(|a| a.duration()).unwrap_or(1.2);
                map.insert(name.to_string(), (node, duration));
            }
            (graphs.add(graph), Arc::new(map))
        })
        .clone();
    for entity in children.iter_descendants(event.entity) {
        if players.contains(entity) {
            commands.entity(entity).insert((
                AnimationGraphHandle(graph.clone()),
                RigPlayer {
                    actor: spec.actor,
                    generation: 0,
                    clips: map.clone(),
                    current: String::new(),
                    phase: if let Actor::Jelly(i) = spec.actor {
                        i as f32 * 0.47
                    } else {
                        0.0
                    },
                    blend: 1.0,
                    outgoing: Vec::new(),
                },
            ));
        }
        if matches!(spec.actor, Actor::Crystal | Actor::Jelly(_)) {
            commands
                .entity(entity)
                .insert((bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
        }
        if matches!(spec.actor, Actor::Fish(_)) {
            // High enables fish casters in fish_shadows; cheaper tiers omit the extra draw.
            commands.entity(entity).insert(bevy::light::NotShadowCaster);
        }
        if let Ok(mesh) = mesh_handles.get(entity)
            && let Some(lod_gltf) = lib.lods.get(&spec.asset).and_then(|h| gltfs.get(h))
        {
            for source in gltf.meshes.iter().filter_map(|h| mesh_assets.get(h)) {
                if let Some(primitive) = source.primitives.iter().find(|p| p.mesh == mesh.0) {
                    let lod_mesh = |level| {
                        lod_gltf
                            .named_meshes
                            .get(format!("{}__LOD{level}", source.name).as_str())
                            .and_then(|h| mesh_assets.get(h))
                            .and_then(|m| m.primitives.get(primitive.index))
                            .map(|p| p.mesh.clone())
                    };
                    if let (Some(middle), Some(small)) = (lod_mesh(1), lod_mesh(2)) {
                        let diameter = meshes
                            .get(&mesh.0)
                            .and_then(|m| m.get_aabb())
                            .map(|a| a.half_extents.max_element() * 2.0)
                            .unwrap_or(2.0);
                        commands.entity(entity).insert(RenderLod {
                            actor: spec.actor,
                            meshes: [mesh.0.clone(), middle, small],
                            diameter,
                            level: 0,
                        });
                    }
                }
            }
        }
        if let Ok((handle, material_name)) = surfaces.get(entity) {
            let variant = match spec.actor {
                Actor::Sabrina => 1,
                Actor::Jelly(i) => 2 + i as u8,
                _ => 0,
            };
            let key = (handle.0.id(), variant);
            if !cache.materials.contains_key(&key)
                && let Some(source) = materials.get(&handle.0)
            {
                let mut material = source.clone();
                if let Actor::Jelly(k) = spec.actor
                    && material_name.is_some_and(|name| name.0 == "Jelly_Skin")
                {
                    material.base_color = match k {
                        0 => Color::linear_rgb(0.76, 1.18, 1.13),
                        1 => Color::linear_rgb(1.48, 1.12, 0.84),
                        _ => Color::linear_rgb(0.60, 1.65, 1.18),
                    };
                    // Blend the bell and tendrils; keep the separate face opaque.
                    // Back-face culling avoids doubling opacity through the shell.
                    material.base_color = material.base_color.with_alpha(0.56);
                    material.alpha_mode = AlphaMode::Blend;
                    material.cull_mode = Some(bevy::render::render_resource::Face::Back);
                    material.metallic = 0.0;
                    material.perceptual_roughness = 0.24;
                    material.reflectance = 0.35;
                }
                let mut red = material.clone();
                red.base_color = Color::srgb(1.0, 0.055, 0.09);
                cache
                    .materials
                    .insert(key, (materials.add(material), materials.add(red)));
            }
            if let Some((normal, hurt)) = cache.materials.get(&key) {
                commands.entity(entity).insert((
                    MeshMaterial3d(normal.clone()),
                    CharacterSurface {
                        actor: spec.actor,
                        normal: normal.clone(),
                        hurt: hurt.clone(),
                    },
                    NoFrustumCulling,
                ));
            }
        }
        if spec.actor == Actor::Sabrina && morphs.contains(entity) {
            commands.entity(entity).insert(FacialRig);
        }
        if spec.actor == Actor::Sabrina
            && let Ok(name) = names.get(entity)
        {
            if name.starts_with("Hair_") {
                let rotation = relative_rotation(entity, event.entity, &transforms, &parents);
                if let Ok(transform) = transforms.get(entity) {
                    if let Some(bone) =
                        crate::hair::HairBone::new(name, transform.rotation, rotation)
                    {
                        commands.entity(entity).insert(bone);
                    } else {
                        warn!("Unrecognized Sabrina hair joint: {name}");
                    }
                }
            }
            if name.as_str() == "Hand_R" {
                commands.entity(entity).insert(Palm);
            }
            if [
                "Tail_1",
                "Tail_2",
                "Neck",
                "Head",
                "UpperArm_L",
                "UpperArm_R",
            ]
            .contains(&name.as_str())
            {
                let rot = relative_rotation(entity, event.entity, &transforms, &parents);
                commands.entity(entity).insert(CorrectBone {
                    root: event.entity,
                    name: name.to_string(),
                    original: None,
                    rest_axis: rot.inverse() * Vec3::Z,
                });
            }
        }
    }
}

fn relative_rotation(
    mut entity: Entity,
    root: Entity,
    transforms: &Query<&Transform>,
    parents: &Query<&ChildOf>,
) -> Quat {
    let mut rotation = Quat::IDENTITY;
    for _ in 0..64 {
        if entity == root {
            break;
        }
        if let Ok(transform) = transforms.get(entity) {
            rotation = transform.rotation * rotation
        }
        let Ok(parent) = parents.get(entity) else {
            break;
        };
        entity = parent.parent();
    }
    rotation
}

fn animate(
    rendered: Res<Presentation>,
    ui: Res<SessionUi>,
    preferences: Res<Preferences>,
    time: Res<Time>,
    mut players: Query<(Entity, &mut AnimationPlayer, &mut RigPlayer)>,
    mut bones: Query<
        (Entity, &mut Transform, Option<&mut CorrectBone>),
        With<bevy::animation::AnimationTargetId>,
    >,
    mut reading: Local<Option<ReadingAnimations>>,
) {
    if ui.loaded && ui.screen.is_reading() {
        if reading.is_none() {
            *reading = Some(ReadingAnimations {
                players: players
                    .iter()
                    .map(|(entity, player, rig)| (entity, player.clone(), rig.clone()))
                    .collect(),
                bones: bones
                    .iter()
                    .map(|(entity, transform, correction)| {
                        (
                            entity,
                            *transform,
                            correction.and_then(|bone| bone.original),
                        )
                    })
                    .collect(),
            });
        }
    } else if let Some(saved) = reading.take() {
        for (entity, saved_player, saved_rig) in saved.players {
            if let Ok((_, mut player, mut rig)) = players.get_mut(entity) {
                *player = saved_player;
                *rig = saved_rig;
            }
        }
        // Zero-weight animations do not evaluate hidden bones on return.
        for (entity, saved_transform, original) in saved.bones {
            if let Ok((_, mut transform, correction)) = bones.get_mut(entity) {
                *transform = saved_transform;
                if let Some(mut bone) = correction {
                    bone.original = original;
                }
            }
        }
        if ui.scene_paused() {
            return;
        }
    }
    let dt = if ui.scene_paused() {
        0.0
    } else {
        time.delta_secs()
            * if preferences.reduced_motion {
                0.25
            } else {
                1.0
            }
    };
    for (_, mut player, mut rig) in &mut players {
        let hidden = match rig.actor {
            Actor::Fish(i) => {
                let f = &rendered.0.fish[i];
                !f.life.active && f.life.pop_time <= 0.22
            }
            Actor::Jelly(i) => rendered.0.jelly[i].behaviour.state == reef_sim::JellyState::Hidden,
            _ => false,
        };
        if hidden {
            // Zero weights make Bevy skip every bone of an unseen rig, so it is
            // neither evaluated nor re-propagated. Visible rigs restore weights.
            if player.playing_animations().any(|(_, a)| a.weight() != 0.0) {
                for (_, a) in player.playing_animations_mut() {
                    a.set_weight(0.0);
                }
            }
            continue;
        }
        let (name, phase, speed) = match rig.actor {
            Actor::Sabrina => {
                let s = &rendered.0.sabrina;
                let names = [
                    "Idle",
                    "Swim",
                    "Turn_Left",
                    "Turn_Right",
                    "Sing",
                    "Wave",
                    "Hurt",
                    "Throw",
                    "Dive",
                ];
                (
                    names[reef_sim::sabrina_pose(s) as usize],
                    Some(reef_sim::sabrina_pose_phase(s)),
                    1.0,
                )
            }
            Actor::Fish(i) => {
                let f = &rendered.0.fish[i];
                (
                    "Swim",
                    None,
                    (6.0 + f.swimming.speed * 5.0) * 1.2 / std::f32::consts::TAU,
                )
            }
            Actor::Jelly(i) => {
                let j = &rendered.0.jelly[i];
                (
                    if j.behaviour.bubble > 0.0 {
                        "Drift"
                    } else if j.behaviour.eat_wait > 0.0 {
                        "Nibble"
                    } else if j.behaviour.state == reef_sim::JellyState::Leaving {
                        "Drift"
                    } else {
                        "Pulse"
                    },
                    None,
                    if j.behaviour.bubble > 0.0 { 0.2 } else { 1.0 },
                )
            }
            _ => continue,
        };
        let Some(&(node, duration)) = rig.clips.get(name) else {
            continue;
        };
        if rig.current != name {
            rig.outgoing = player
                .playing_animations()
                .filter(|(n, _)| **n != node)
                .map(|(n, a)| (*n, a.weight()))
                .collect();
            rig.blend = if rig.current.is_empty() { 1.0 } else { 0.0 };
            player.play(node).repeat().set_speed(0.0);
            rig.current = name.into();
            rig.phase = 0.0;
        }
        if let Actor::Fish(i) = rig.actor
            && rig.generation != rendered.0.fish[i].id.generation
        {
            rig.generation = rendered.0.fish[i].id.generation;
            rig.phase = rendered.0.fish[i].swimming.tail_phase / std::f32::consts::TAU * 1.2;
        }
        // Explicit crossfades use game time, so pausing also freezes a blend.
        let blend_time = if name == "Wave" { 0.06 } else { 0.22 };
        rig.blend = (rig.blend + dt / blend_time).min(1.0);
        let mut remaining = 1.0;
        for &(old, weight) in &rig.outgoing {
            if rig.blend >= 1.0 {
                player.stop(old);
            } else if let Some(a) = player.animation_mut(old) {
                let blend_weight = weight * (1.0 - rig.blend);
                a.set_weight(blend_weight);
                let seek = a.seek_time();
                let clip = rig.clips.iter().find(|(_, (n, _))| *n == old);
                if let Some((name, (_, length))) = clip {
                    let next = seek + dt * speed;
                    a.set_seek_time(if name == "Throw" || name == "Wave" {
                        next.min(length.next_down())
                    } else {
                        next.rem_euclid(*length)
                    });
                }
                remaining -= blend_weight;
            }
        }
        player.play(node).set_weight(remaining.max(0.0));
        rig.phase = phase
            .map(|x| (x * duration).min(duration.next_down()))
            .unwrap_or((rig.phase + dt * speed) % duration);
        player
            .play(node)
            .repeat()
            .set_speed(0.0)
            .set_seek_time(rig.phase);
    }
}

fn restore_bones(mut bones: Query<(&mut Transform, &mut CorrectBone)>) {
    for (mut transform, mut bone) in &mut bones {
        if let Some(rotation) = bone.original.take() {
            transform.rotation = rotation;
        }
    }
}

pub(crate) fn correct_bones(
    rendered: Res<Presentation>,
    preferences: Res<Preferences>,
    mut set: ParamSet<(Query<&Transform>, Query<&mut Transform>)>,
    parents: Query<&ChildOf>,
    mut bones: Query<(Entity, &mut CorrectBone)>,
) {
    let s = &rendered.0.sabrina;
    let effort = (s.motion.velocity.length() / 7.2).min(1.0);
    let rest =
        ((s.swimming.pitch - 0.2) / 0.45).clamp(0.0, 1.0) * (1.0 - (effort * 2.0).clamp(0.0, 1.0));
    let time = rendered.0.session.time;
    let motion = if preferences.reduced_motion {
        0.25
    } else {
        1.0
    };
    let gaze = (-s.swimming.pitch * 0.55).clamp(-0.65, 0.06)
        + motion * (0.018 * (time * 0.87).sin() + 0.009 * (time * 1.71).sin());
    // Parent corrections are applied before child corrections, and restored before
    // the next animation evaluation, including joints with optimized-out tracks.
    for name in [
        "Tail_1",
        "Tail_2",
        "Neck",
        "Head",
        "UpperArm_L",
        "UpperArm_R",
    ] {
        for (e, mut bone) in &mut bones {
            if bone.name != name {
                continue;
            }
            let (cast_axis, cast_angle) = reef_sim::sabrina_cast_rotation(s);
            let angle = match name {
                "Tail_1" => 0.30 * rest,
                "Tail_2" => 0.10 * rest,
                "Neck" => gaze * 0.35,
                "Head" => gaze * 0.65,
                "UpperArm_L" | "UpperArm_R" => cast_angle,
                _ => 0.0,
            };
            let axis = if name.starts_with("Tail") {
                bone.rest_axis
            } else {
                relative_rotation(e, bone.root, &set.p0(), &parents).inverse()
                    * if name.starts_with("UpperArm") {
                        Vec3::from_array(cast_axis)
                    } else {
                        Vec3::Z
                    }
            };
            if let Ok(mut t) = set.p1().get_mut(e) {
                bone.original = Some(t.rotation);
                t.rotation *= Quat::from_axis_angle(axis.normalize_or_zero(), angle);
            }
        }
    }
}

fn facial_expression(
    rendered: Res<Presentation>,
    mut faces: Query<&mut MorphWeights, With<FacialRig>>,
) {
    let time = rendered.0.session.time;
    // Unequal intervals and a quick closing/slower opening feel less mechanical.
    let cycle = time.rem_euclid(11.3);
    let blink = [2.8, 7.7, 8.05]
        .into_iter()
        .map(|start| {
            let t = cycle - start;
            if (0.0..0.08).contains(&t) {
                t / 0.08
            } else if (0.08..0.22).contains(&t) {
                1.0 - (t - 0.08) / 0.14
            } else {
                0.0
            }
        })
        .fold(0.0_f32, f32::max);
    let sing = rendered.0.sabrina.song.strength * (0.45 + 0.2 * (time * 4.7).sin());
    for mut face in &mut faces {
        let weights = face.weights_mut();
        if weights.len() == 3 {
            weights.copy_from_slice(&[blink, 0.35 + 0.1 * (time * 0.63).sin(), sing]);
        }
    }
}

// Use the rendered, blended skeleton for the held prop; collision samples remain
// deterministic simulation data. Compose locals before transform propagation.

fn attach_crystal(
    rendered: Res<Presentation>,
    palm: Query<Entity, With<Palm>>,
    parents: Query<&ChildOf>,
    mut set: ParamSet<(Query<&Transform>, Query<(&Actor, &mut Transform)>)>,
) {
    if rendered.0.crystal.magic.state != reef_sim::Spell::Charging {
        return;
    }
    let Ok(mut e) = palm.single() else { return };
    let mut point = Vec3::new(0.0, reef_sim::sabrina_palm_offset(), 0.0);
    for _ in 0..64 {
        if let Ok(t) = set.p0().get(e) {
            point = t.transform_point(point);
        }
        let Ok(parent) = parents.get(e) else { break };
        e = parent.parent();
    }
    let magic = &rendered.0.crystal.magic;
    let origin = rendered.0.sabrina.motion.pos + magic.charge_offset;
    let amount = (magic.time / 0.4).clamp(0.0, 1.0);
    let amount = amount * amount * (3.0 - 2.0 * amount);
    for (actor, mut t) in &mut set.p1() {
        if *actor == Actor::Crystal {
            t.translation = Vec3::new(origin.x, origin.y, magic.charge_depth).lerp(point, amount);
        }
    }
}

fn report_assets(
    options: Res<Options>,
    rendered: Res<Presentation>,
    mut reported: Local<bool>,
    lods: Query<(&RenderLod, &Mesh3d)>,
    surfaces: Query<&CharacterSurface>,
    meshes: Res<Assets<Mesh>>,
) {
    if *reported || (!options.verify && options.capture.is_none()) || rendered.0.session.time < 1.0
    {
        return;
    }
    let fish: Vec<_> = surfaces
        .iter()
        .filter(|s| matches!(s.actor, Actor::Fish(_)))
        .collect();
    if fish.len() != 72 {
        return;
    }
    let unique: std::collections::HashSet<_> = fish.iter().map(|s| s.normal.id()).collect();
    assert!(unique.len() <= 12, "Fish must reuse species materials");
    assert_eq!(
        lods.iter().count(),
        74,
        "Every fish and both reef meshes need render LODs"
    );
    let mut counts = [0; 3];
    let mut triangles = 0;
    for (lod, mesh) in &lods {
        counts[lod.level] += 1;
        let mesh = meshes.get(&mesh.0).unwrap();
        assert!(mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT));
        triangles += mesh.indices().unwrap().len() / 3;
    }
    println!(
        "RENDER ASSETS: 72 fish share {} normal materials; LOD counts {:?}; {} fish/reef triangles",
        unique.len(),
        counts,
        triangles
    );
    *reported = true;
}

#[cfg(test)]
mod animation_tests {
    use super::*;
    use crate::{
        app::Screen,
        presentation::{Previous, prepare},
    };

    fn assert_restored(world: &World, saved: &[(Entity, AnimationPlayer, RigPlayer)]) {
        for (entity, before, rig_before) in saved {
            let after = world.get::<AnimationPlayer>(*entity).unwrap();
            let rig = world.get::<RigPlayer>(*entity).unwrap();
            assert_eq!(rig.current, rig_before.current);
            assert_eq!(rig.phase, rig_before.phase);
            assert_eq!(rig.generation, rig_before.generation);
            assert_eq!(rig.blend, rig_before.blend);
            assert_eq!(rig.outgoing, rig_before.outgoing);
            assert_eq!(
                after.playing_animations().count(),
                before.playing_animations().count()
            );
            for (node, expected) in before.playing_animations() {
                let actual = after.animation(*node).unwrap();
                assert_eq!(actual.weight(), expected.weight());
                assert_eq!(actual.seek_time(), expected.seek_time());
                assert_eq!(actual.speed(), expected.speed());
                assert_eq!(actual.repeat_mode(), expected.repeat_mode());
                assert_eq!(actual.is_paused(), expected.is_paused());
            }
        }
    }

    #[test]
    fn reading_restores_animation_phases_and_in_progress_crossfades() {
        for reduced_motion in [false, true] {
            let mut app = App::new();
            app.init_resource::<Presentation>()
                .insert_resource(Simulation(reef_sim::World::default()))
                .init_resource::<Previous>()
                .init_resource::<Time>()
                .init_resource::<Time<Fixed>>()
                .init_resource::<Preferences>()
                .insert_resource(SessionUi {
                    loaded: true,
                    screen: Screen::Pause,
                    ..default()
                })
                .add_systems(Update, (prepare, animate).chain());
            app.world_mut().resource_mut::<Preferences>().reduced_motion = reduced_motion;
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(0.1));
            let mut world = reef_sim::World::default();
            world.session.time = 10.0;
            world.sabrina.song.active = reduced_motion;
            world.fish[0].life.active = true;
            world.fish[0].id.generation = 42;
            world.jelly[0].behaviour.state = reef_sim::JellyState::Hungry;
            app.world_mut().resource_mut::<Simulation>().0 = world.clone();
            app.world_mut().resource_mut::<Presentation>().0 = world.clone();
            world.session.time -= 1.0 / 60.0;
            app.world_mut().resource_mut::<Previous>().0 = Some(world);
            let frozen_transform = Transform::from_rotation(Quat::from_rotation_z(0.3));
            let bone = app
                .world_mut()
                .spawn((
                    bevy::animation::AnimationTargetId::from_name(&Name::new("Root")),
                    frozen_transform,
                ))
                .id();
            let clips: ClipMap = Arc::new(
                [
                    "Idle",
                    "Swim",
                    "Turn_Left",
                    "Turn_Right",
                    "Dive",
                    "Sing",
                    "Pulse",
                    "Drift",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, name)| (name.into(), (AnimationNodeIndex::new(i), 2.0)))
                .collect(),
            );
            let mut entities = Vec::new();
            for (actor, name, outgoing) in [
                (
                    Actor::Sabrina,
                    if reduced_motion { "Sing" } else { "Idle" },
                    Some("Swim"),
                ),
                (Actor::Fish(0), "Swim", None),
                (Actor::Jelly(0), "Pulse", Some("Drift")),
            ] {
                let mut player = AnimationPlayer::default();
                player
                    .play(clips[name].0)
                    .repeat()
                    .set_speed(0.0)
                    .set_seek_time(0.73)
                    .set_weight(if outgoing.is_some() { 0.4 } else { 1.0 });
                let outgoing = outgoing
                    .map(|name| {
                        let node = clips[name].0;
                        player
                            .play(node)
                            .repeat()
                            .set_speed(0.0)
                            .set_seek_time(1.13)
                            .set_weight(0.6);
                        (node, 1.0)
                    })
                    .into_iter()
                    .collect();
                entities.push(
                    app.world_mut()
                        .spawn((
                            player,
                            RigPlayer {
                                actor,
                                generation: 42,
                                clips: clips.clone(),
                                current: name.into(),
                                phase: 0.73,
                                blend: if actor == Actor::Fish(0) { 1.0 } else { 0.4 },
                                outgoing,
                            },
                        ))
                        .id(),
                );
            }
            app.update();
            let saved: Vec<_> = entities
                .iter()
                .map(|&entity| {
                    (
                        entity,
                        app.world().get::<AnimationPlayer>(entity).unwrap().clone(),
                        app.world().get::<RigPlayer>(entity).unwrap().clone(),
                    )
                })
                .collect();
            for screen in [Screen::Help, Screen::Credits] {
                app.world_mut().resource_mut::<SessionUi>().screen = screen;
                for _ in 0..20 {
                    app.update();
                }
                app.world_mut().get_mut::<Transform>(bone).unwrap().rotation =
                    Quat::from_rotation_z(1.2);
                assert_ne!(
                    app.world().get::<RigPlayer>(entities[1]).unwrap().phase,
                    saved[1].2.phase
                );
                assert!(
                    app.world()
                        .get::<AnimationPlayer>(entities[2])
                        .unwrap()
                        .playing_animations()
                        .all(|(_, a)| a.weight() == 0.0)
                );
                // Switching reading pages must not replace the gameplay snapshot.
                app.world_mut().resource_mut::<SessionUi>().screen = Screen::Credits;
                app.update();
                app.world_mut().resource_mut::<SessionUi>().screen = Screen::Pause;
                for _ in 0..3 {
                    app.update();
                    assert_restored(app.world(), &saved);
                    assert_eq!(
                        *app.world().get::<Transform>(bone).unwrap(),
                        frozen_transform
                    );
                }
            }
            app.world_mut().resource_mut::<SessionUi>().screen = Screen::Playing;
            app.update();
            assert!(app.world().get::<RigPlayer>(entities[0]).unwrap().blend > saved[0].2.blend);
            assert_ne!(
                app.world().get::<RigPlayer>(entities[1]).unwrap().phase,
                saved[1].2.phase
            );
        }
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;
    use bevy::light::{NotShadowCaster, NotShadowReceiver};

    #[test]
    fn fish_cast_only_on_high_and_crystal_exclusions_are_preserved() {
        let mut app = App::new();
        app.init_resource::<Preferences>()
            .add_systems(Update, fish_shadows);
        let mut entities = Vec::new();
        for actor in [Actor::Fish(0), Actor::Crystal, Actor::Jelly(0)] {
            let entity = app
                .world_mut()
                .spawn((
                    CharacterSurface {
                        actor,
                        normal: default(),
                        hurt: default(),
                    },
                    NotShadowCaster,
                ))
                .id();
            if actor != Actor::Fish(0) {
                app.world_mut().entity_mut(entity).insert(NotShadowReceiver);
            }
            entities.push(entity);
        }
        for quality in [0, 1, 2, 1, 2, 0] {
            app.world_mut().resource_mut::<Preferences>().quality = quality;
            app.update();
            assert_eq!(
                app.world().get::<NotShadowCaster>(entities[0]).is_none(),
                quality == 2
            );
            for &entity in &entities[1..] {
                assert!(app.world().get::<NotShadowCaster>(entity).is_some());
                assert!(app.world().get::<NotShadowReceiver>(entity).is_some());
            }
        }
    }
}
