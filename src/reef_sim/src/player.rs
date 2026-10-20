// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Sabrina movement, ability timers and swimming pose state.
use crate::*;

pub fn recharge_duration(_world: &World) -> f32 {
    12.0
}

pub fn song_duration(world: &World) -> f32 {
    if world.settings.gentle { 12.0 } else { 10.0 }
}

pub fn song_recharge_duration(world: &World) -> f32 {
    if world.settings.gentle { 20.0 } else { 24.0 }
}

pub fn wave_recharge_duration(_world: &World) -> f32 {
    3.0
}

pub fn song_radius(world: &World) -> f32 {
    if world.sabrina.song.active {
        (1.4 + 5.1 * (world.sabrina.song.energy / 100.0).sqrt()) * world.sabrina.song.strength
    } else {
        0.0
    }
}

pub fn toggle_song(world: &mut World) -> bool {
    let sabrina = &mut world.sabrina;
    if sabrina.song.active {
        sabrina.song.active = false;
        return true;
    }
    if world.session.game_over
        || sabrina.song.locked
        || sabrina.song.energy <= 0.0
        || sabrina.vitality.stunned > 0.0
    {
        return false;
    }
    sabrina.song.active = true;
    true
}

pub fn timers(world: &mut World, dt: f32) {
    world.session.time += dt;
    for reward in &mut world.points.rewards {
        reward.time = (reward.time - dt).max(0.0);
    }
    let sabrina = &mut world.sabrina;
    if sabrina.throw_time >= 0.0 {
        sabrina.throw_time += dt;
        if sabrina.throw_time > 1.6 {
            sabrina.throw_time = -1.0
        }
    }
    sabrina.vitality.heal_delay = (sabrina.vitality.heal_delay - dt).max(0.0);
    sabrina.vitality.hurt_flash = (sabrina.vitality.hurt_flash - dt).max(0.0);
    for fish in &mut world.fish {
        fish.life.pop_time = (fish.life.pop_time - dt).max(0.0)
    }
    sabrina.cooldown = (sabrina.cooldown - dt).max(0.0);
    sabrina.vitality.stunned = (sabrina.vitality.stunned - dt).max(0.0);
    sabrina.vitality.invulnerable = (sabrina.vitality.invulnerable - dt).max(0.0);
}

pub fn song(world: &mut World, dt: f32, input: Input, events: &mut Events) {
    if input.toggle_song {
        toggle_song(world);
    }
    let duration = song_duration(world);
    let recharge = song_recharge_duration(world);
    let song = &mut world.sabrina.song;
    if song.active {
        song.energy = (song.energy - dt * 100.0 / duration).max(0.0);
        if song.energy <= 0.001 {
            // Exhaustion locks the song until full; voluntarily stopping does not.
            song.energy = 0.0;
            song.active = false;
            song.locked = true;
            events.song_empty = true
        }
    } else {
        song.energy = (song.energy + dt * 100.0 / recharge).min(100.0);
        if song.energy >= 99.999 {
            song.energy = 100.0;
            song.locked = false
        }
    }
}

pub fn player_swimming(world: &mut World, dt: f32, mut input: Input) {
    let target =
        find_jelly(world, world.crystal.magic.target).map(|jellyfish| jellyfish.motion.pos);
    let sabrina = &mut world.sabrina;
    sabrina.wave.cooldown = (sabrina.wave.cooldown - dt).max(0.0);
    if input.movement.length() > 1.0 {
        input.movement = input.movement.unit()
    }
    let effort = input.movement.length();
    sabrina.swimming.phase += dt
        * (1.9634954 + effort * 2.3998277)
        * if world.settings.reduced_motion {
            0.25
        } else {
            1.0
        };
    sabrina.song.strength = (sabrina.song.strength
        + dt * if sabrina.song.active && sabrina.vitality.stunned <= 0.0 {
            2.0
        } else {
            -1.2
        })
    .clamp(0.0, 1.0);
    let normal = v(-input.movement.y, input.movement.x);
    let desired = if sabrina.vitality.stunned > 0.0 {
        v(0.0, 0.0)
    } else {
        input.movement * (7.2 - sabrina.song.strength * 1.6)
            + normal * (sabrina.swimming.phase.cos() * 1.2 * effort)
    };
    sabrina.motion.velocity =
        sabrina.motion.velocity + (desired - sabrina.motion.velocity) * (dt * 9.0).min(1.0);
    sabrina.motion.pos = bounds(sabrina.motion.pos + sabrina.motion.velocity * dt);
    sabrina.motion.pos.y = sabrina.motion.pos.y.max(3.2);
    if sabrina.motion.velocity.x.abs() > 0.25 {
        sabrina.swimming.facing = if sabrina.motion.velocity.x < 0.0 {
            -1.0
        } else {
            1.0
        }
    }
    let mut moving = ((sabrina.motion.velocity.length() - 0.3) / 3.5).clamp(0.0, 1.0);
    let mut heading = sabrina
        .motion
        .velocity
        .y
        .atan2(sabrina.motion.velocity.x.abs().max(0.15));
    if world.crystal.magic.state == Spell::Charging
        && let Some(target) = target
    {
        let aim = target - sabrina.motion.pos;
        if aim.x.abs() > 0.1 {
            sabrina.swimming.facing = if aim.x < 0.0 { -1.0 } else { 1.0 }
        }
        heading = aim.y.atan2(aim.x.abs().max(0.15));
        moving = 1.0;
    }
    let target_pitch = 0.65 * (1.0 - moving) + heading.clamp(-1.1, 1.1) * moving;
    sabrina.swimming.pitch += (target_pitch - sabrina.swimming.pitch) * (1.0 - (-4.0 * dt).exp());
    let target_yaw = if sabrina.swimming.facing < 0.0 {
        -3.14159265
    } else {
        0.0
    };
    sabrina.swimming.yaw += (target_yaw - sabrina.swimming.yaw) * (1.0 - (-4.5 * dt).exp());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SabrinaPose {
    Idle,
    Swim,
    TurnLeft,
    TurnRight,
    Sing,
    Wave,
    Hurt,
    Throw,
    Dive,
}

pub fn sabrina_pose(sabrina: &SabrinaEntity) -> SabrinaPose {
    if sabrina.vitality.hurt_flash > 0.0 || sabrina.vitality.stunned > 0.0 {
        return SabrinaPose::Hurt;
    }
    if sabrina.throw_time >= 0.0 {
        return SabrinaPose::Throw;
    }
    if sabrina.wave.active {
        return SabrinaPose::Wave;
    }
    if sabrina.song.active {
        return SabrinaPose::Sing;
    }
    let effort = (sabrina.motion.velocity.length() / 7.2).min(1.0);
    // Require actual downward travel as well as pitch so idle and casts do not
    // accidentally enter the forward-reaching dive pose.
    if effort > 0.12 && sabrina.swimming.pitch < -0.3 && sabrina.motion.velocity.y < -0.5 {
        return SabrinaPose::Dive;
    }
    if sabrina.swimming.yaw.sin().abs() > 0.4 && effort > 0.15 {
        return if sabrina.swimming.facing < 0.0 {
            SabrinaPose::TurnLeft
        } else {
            SabrinaPose::TurnRight
        };
    }
    if effort > 0.12 {
        SabrinaPose::Swim
    } else {
        SabrinaPose::Idle
    }
}

pub fn sabrina_pose_phase(sabrina: &SabrinaEntity) -> f32 {
    match sabrina_pose(sabrina) {
        SabrinaPose::Throw => (sabrina.throw_time / 1.6).clamp(0.0, 1.0),
        SabrinaPose::Wave => (sabrina.wave.time / 0.75).clamp(0.0, 1.0),
        _ => (sabrina.swimming.phase % 6.28318530718) / 6.28318530718,
    }
}

/// Gentle three-quarter presentation, mirrored smoothly through turns.
pub fn sabrina_visual_yaw(sabrina: &SabrinaEntity) -> f32 {
    let yaw = sabrina.swimming.yaw;
    // Cosine mirrors the camera-facing bias continuously instead of flipping it.
    yaw - std::f32::consts::FRAC_PI_6 * yaw.cos()
}

/// Model-local axis/angle shared by the rendered arms and sampled contacts.
pub fn sabrina_cast_rotation(sabrina: &SabrinaEntity) -> ([f32; 3], f32) {
    let (direction, blend_weight) = match sabrina_pose(sabrina) {
        SabrinaPose::Wave => (
            sabrina.wave.direction,
            (sabrina.wave.time / 0.08).clamp(0.0, 1.0)
                * ((0.75 - sabrina.wave.time) / 0.25).clamp(0.0, 1.0),
        ),
        SabrinaPose::Throw => (
            sabrina.throw_direction,
            (sabrina.throw_time / 0.3).clamp(0.0, 1.0)
                * ((1.6 - sabrina.throw_time) / 0.3).clamp(0.0, 1.0),
        ),
        _ => return ([0.0, 0.0, 1.0], 0.0),
    };
    let direction = direction.unit();
    let yaw = sabrina_visual_yaw(sabrina);
    // Undo the actor's yaw, then pitch, to express the spell aim in model space.
    let local_xy = v(direction.x * yaw.cos(), direction.y).rotate(-sabrina.swimming.pitch);
    let local_z = direction.x * yaw.sin();
    // The authored reach points along +X; its cross product with the aim gives
    // the correction axis. Parallel aims need a stable fallback axis.
    let perpendicular_length = (local_xy.y * local_xy.y + local_z * local_z).sqrt();
    let axis = if perpendicular_length > 0.00001 {
        [
            0.0,
            -local_z / perpendicular_length,
            local_xy.y / perpendicular_length,
        ]
    } else {
        [0.0, 0.0, 1.0]
    };
    (axis, local_xy.x.clamp(-1.0, 1.0).acos() * blend_weight)
}
