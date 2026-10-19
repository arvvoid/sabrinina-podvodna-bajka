// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Root-pinned, damped hair chains with stylized currents and inertial follow-through.
use crate::{
    assets::{Actor, CharacterSurface},
    presentation::Presentation,
    settings::Preferences,
};
use bevy::{
    app::AnimationSystems,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    transform::TransformSystems,
};

pub(crate) const HAIR_CHAINS: usize = 5;
pub(crate) const HAIR_SEGMENTS: usize = 5;

fn rest_angle(segment: usize) -> Vec2 {
    // Sweep the curls back from the face; currents offset this pose, not world up.
    Vec2::new([-0.24, -0.12, -0.08, -0.06, -0.04][segment - 1], 0.0)
}

#[derive(Component)]
pub(crate) struct HairBone {
    lane: usize,
    segment: usize,
    rest: Quat,
    pitch_axis: Vec3,
    yaw_axis: Vec3,
    spring: Spring,
}

impl HairBone {
    pub(crate) fn new(name: &str, rest: Quat, model_rotation: Quat) -> Option<Self> {
        let (lane, segment) = name.strip_prefix("Hair_")?.split_once('_')?;
        let lane: usize = lane.parse().ok()?;
        let segment: usize = segment.parse().ok()?;
        if lane >= HAIR_CHAINS || !(1..=HAIR_SEGMENTS).contains(&segment) {
            return None;
        }
        Some(Self {
            lane,
            segment,
            rest,
            pitch_axis: model_rotation.inverse() * Vec3::Z,
            yaw_axis: model_rotation.inverse() * Vec3::Y,
            spring: Spring {
                angle: rest_angle(segment),
                ..default()
            },
        })
    }

    fn rotation(&self) -> Quat {
        self.rest
            * Quat::from_axis_angle(self.pitch_axis, self.spring.angle.x)
            * Quat::from_axis_angle(self.yaw_axis, self.spring.angle.y)
    }
}

#[derive(Default)]
struct Spring {
    angle: Vec2,
    velocity: Vec2,
}

impl Spring {
    fn advance(&mut self, target: Vec2, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        // Substeps keep the critically damped spring stable after a slow frame.
        let dt = dt.min(0.1);
        let steps = (dt * 120.0).ceil() as u32;
        let step = dt / steps as f32;
        for _ in 0..steps {
            let acceleration = (target - self.angle) * 36.0 - self.velocity * 12.0;
            self.velocity += acceleration * step;
            self.angle += self.velocity * step;
        }
    }
}

#[derive(Resource, Default)]
struct HairMotion {
    previous: Option<(f32, Vec2, Vec2)>,
    impulse: Vec2,
}

fn angle_delta(current: f32, previous: f32) -> f32 {
    (current - previous + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI
}

fn target_angle(
    time: f32,
    effort: f32,
    motion_scale: f32,
    impulse: Vec2,
    lane: usize,
    segment: usize,
) -> Vec2 {
    let depth = segment as f32 / HAIR_SEGMENTS as f32;
    let phase = time * 1.45 - depth * 3.4 + lane as f32 * 0.58;
    let amplitude = (0.035 + effort * 0.045) * (0.6 + depth * 0.4);
    let sway = Vec2::new(
        phase.sin() * amplitude,
        (phase * 0.73 + 0.8).sin() * amplitude * 0.5,
    );
    rest_angle(segment)
        + ((sway + impulse * (0.35 + depth * 0.65)) * motion_scale)
            .clamp(Vec2::splat(-0.16), Vec2::splat(0.16))
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(crate) struct HairExtension {
    // Packed as time, motion scale, swim effort, hurt tint; matches hair.wesl.
    #[uniform(100)]
    pub settings: Vec4,
}

impl MaterialExtension for HairExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/hair.wesl".into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

pub(crate) type HairMaterial = ExtendedMaterial<StandardMaterial, HairExtension>;

pub struct HairPlugin;

impl Plugin for HairPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<HairMaterial>::default())
            .init_resource::<HairMotion>()
            .add_systems(
                Update,
                (bind, currents).chain().after(crate::presentation::prepare),
            )
            .add_systems(
                PostUpdate,
                flow.after(AnimationSystems)
                    .after(crate::assets::correct_bones)
                    .before(TransformSystems::Propagate),
            );
    }
}

fn bind(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &CharacterSurface,
            &bevy::gltf::GltfMaterialName,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Added<CharacterSurface>,
    >,
    standard: Res<Assets<StandardMaterial>>,
    mut hair: ResMut<Assets<HairMaterial>>,
) {
    for (entity, surface, name, handle) in &surfaces {
        if surface.actor != Actor::Sabrina || name.0 != "Sabrina_Hair" {
            continue;
        }
        let Some(source) = standard.get(&handle.0) else {
            warn!("Hair surface has no loaded base material");
            continue;
        };
        let mut base = source.clone();
        base.alpha_mode = AlphaMode::Opaque;
        base.metallic = 0.0;
        base.perceptual_roughness = 0.43;
        base.reflectance = 0.4;
        base.anisotropy_strength = 0.35;
        base.anisotropy_texture = None;
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((
                MeshMaterial3d(hair.add(ExtendedMaterial {
                    base,
                    extension: HairExtension {
                        settings: Vec4::ZERO,
                    },
                })),
                bevy::light::NotShadowCaster,
            ));
    }
}

fn flow(
    rendered: Res<Presentation>,
    preferences: Res<Preferences>,
    mut motion: ResMut<HairMotion>,
    mut bones: Query<(&mut Transform, &mut HairBone)>,
) {
    if bones.is_empty() {
        return;
    }
    let time = rendered.0.session.time;
    let sabrina = &rendered.0.sabrina;
    let velocity = Vec2::new(sabrina.motion.velocity.x, sabrina.motion.velocity.y);
    let heading = Vec2::new(sabrina.swimming.pitch, sabrina.swimming.yaw);
    let Some((previous_time, previous_velocity, previous_heading)) = motion.previous else {
        motion.previous = Some((time, velocity, heading));
        for (mut transform, bone) in &mut bones {
            transform.rotation = bone.rotation();
        }
        return;
    };
    let dt = time - previous_time;
    // Simulation time stops while paused: keep both the spring and its history.
    if dt == 0.0 {
        return;
    }
    motion.previous = Some((time, velocity, heading));
    if !(0.0..=0.5).contains(&dt) {
        // A restart or long discontinuity is not a physical acceleration.
        motion.impulse = Vec2::ZERO;
        for (mut transform, mut bone) in &mut bones {
            bone.spring = Spring {
                angle: rest_angle(bone.segment),
                ..default()
            };
            transform.rotation = bone.rotation();
        }
        return;
    }
    let acceleration = (velocity - previous_velocity) / dt;
    let turn = Vec2::new(
        angle_delta(heading.x, previous_heading.x),
        angle_delta(heading.y, previous_heading.y),
    ) / dt;
    let desired = Vec2::new(-turn.x * 0.035 - acceleration.y * 0.003, -turn.y * 0.018)
        .clamp(Vec2::splat(-0.10), Vec2::splat(0.10));
    motion.impulse = motion.impulse.lerp(desired, 1.0 - (-dt * 5.0).exp());
    let motion_scale = if preferences.reduced_motion {
        0.25
    } else {
        1.0
    };
    let effort = (velocity.length() / 7.2).clamp(0.0, 1.0);
    for (mut transform, mut bone) in &mut bones {
        let target = target_angle(
            time,
            effort,
            motion_scale,
            motion.impulse,
            bone.lane,
            bone.segment,
        );
        bone.spring.advance(target, dt);
        transform.rotation = bone.rotation();
    }
}

fn settings(time: f32, speed: f32, reduced_motion: bool, hurt: f32) -> Vec4 {
    Vec4::new(
        time,
        if reduced_motion { 0.25 } else { 1.0 },
        (speed / 7.2).clamp(0.0, 1.0),
        hurt.clamp(0.0, 1.0),
    )
}

fn currents(
    rendered: Res<Presentation>,
    preferences: Res<Preferences>,
    mut materials: ResMut<Assets<HairMaterial>>,
) {
    let next = settings(
        rendered.0.session.time,
        rendered.0.sabrina.motion.velocity.length(),
        preferences.reduced_motion,
        rendered.0.sabrina.vitality.hurt_flash,
    );
    for (_, material) in materials.iter_mut() {
        if material.extension.settings != next {
            material.extension.settings = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currents_use_simulation_time_and_bound_motion_and_damage() {
        assert_eq!(
            settings(3.0, 100.0, false, 5.0),
            Vec4::new(3.0, 1.0, 1.0, 1.0)
        );
        assert_eq!(
            settings(3.0, 0.0, true, 0.0),
            Vec4::new(3.0, 0.25, 0.0, 0.0)
        );
        assert_eq!(
            settings(3.0, -4.0, true, -2.0),
            Vec4::new(3.0, 0.25, 0.0, 0.0)
        );
        assert!(!HairExtension::enable_prepass());
        assert!(!HairExtension::enable_shadows());
    }

    #[test]
    fn hair_names_match_the_five_by_five_rig() {
        for lane in 0..HAIR_CHAINS {
            for segment in 1..=HAIR_SEGMENTS {
                assert!(
                    HairBone::new(
                        &format!("Hair_{lane}_{segment}"),
                        Quat::IDENTITY,
                        Quat::IDENTITY
                    )
                    .is_some()
                );
            }
        }
        for name in ["Head", "Hair_Back_1", "Hair_5_1", "Hair_0_0", "Hair_0_6"] {
            assert!(HairBone::new(name, Quat::IDENTITY, Quat::IDENTITY).is_none());
        }
    }

    #[test]
    fn spring_freezes_at_pause_and_converges_without_overshoot() {
        let target = Vec2::new(0.12, -0.08);
        let mut spring = Spring::default();
        for _ in 0..300 {
            spring.advance(target, 1.0 / 60.0);
            assert!(spring.angle.x >= 0.0 && spring.angle.x <= target.x + 0.00001);
            assert!(spring.angle.y <= 0.0 && spring.angle.y >= target.y - 0.00001);
        }
        assert!(spring.angle.distance(target) < 0.0001);
        let before = (spring.angle, spring.velocity);
        spring.advance(-target, 0.0);
        assert_eq!((spring.angle, spring.velocity), before);
        spring.advance(-target, 10.0);
        assert!(spring.angle.is_finite() && spring.velocity.is_finite());
    }

    #[test]
    fn currents_are_bounded_reduced_and_phase_delayed() {
        for time in [0.0, 1.0, 30.0, 1000.0] {
            for lane in 0..HAIR_CHAINS {
                for segment in 1..=HAIR_SEGMENTS {
                    let base = rest_angle(segment);
                    let full = target_angle(time, 1.0, 1.0, Vec2::ZERO, lane, segment);
                    let reduced = target_angle(time, 1.0, 0.25, Vec2::ZERO, lane, segment);
                    assert!((full - base).length() < 0.10);
                    assert!((reduced - base).distance((full - base) * 0.25) < 0.00001);
                    let extreme = target_angle(time, 1.0, 1.0, Vec2::splat(100.0), lane, segment);
                    assert!((extreme - base).abs().max_element() <= 0.16001);
                }
            }
        }
        assert_ne!(
            target_angle(1.0, 1.0, 1.0, Vec2::ZERO, 0, 1),
            target_angle(1.0, 1.0, 1.0, Vec2::ZERO, 0, 5),
        );
        assert!(angle_delta(-3.13, 3.13).abs() < 0.03);
    }

    #[test]
    fn spring_response_is_consistent_across_render_rates() {
        let target = Vec2::new(-0.12, 0.06);
        let mut slow = Spring::default();
        let mut fast = Spring::default();
        for _ in 0..30 {
            slow.advance(target, 1.0 / 30.0);
        }
        for _ in 0..120 {
            fast.advance(target, 1.0 / 120.0);
        }
        assert!(slow.angle.distance(fast.angle) < 0.0001);
        assert!(slow.velocity.distance(fast.velocity) < 0.0001);
    }
}
