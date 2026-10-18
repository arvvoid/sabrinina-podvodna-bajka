// SPDX-License-Identifier: MIT
//! Sabrina movement, ability timers and swimming pose state.
use crate::*;
pub fn recharge_duration(w: &World) -> f32 {
    if w.settings.gentle { 150.0 } else { 180.0 }
}
pub fn song_duration(w: &World) -> f32 {
    if w.settings.gentle { 12.0 } else { 10.0 }
}
pub fn song_recharge_duration(w: &World) -> f32 {
    if w.settings.gentle { 20.0 } else { 24.0 }
}
pub fn wave_recharge_duration(w: &World) -> f32 {
    if w.settings.gentle { 3.0 } else { 4.0 }
}
pub fn song_radius(w: &World) -> f32 {
    if w.sabrina.song.active {
        (1.4 + 5.1 * (w.sabrina.song.energy / 100.0).sqrt()) * w.sabrina.song.strength
    } else {
        0.0
    }
}
pub fn toggle_song(w: &mut World) -> bool {
    let s = &mut w.sabrina;
    if s.song.active {
        s.song.active = false;
        return true;
    }
    if w.session.game_over || s.song.locked || s.song.energy <= 0.0 || s.vitality.stunned > 0.0 {
        return false;
    }
    s.song.active = true;
    true
}
pub fn timers(w: &mut World, dt: f32) {
    w.session.time += dt;
    let s = &mut w.sabrina;
    if s.throw_time >= 0.0 {
        s.throw_time += dt;
        if s.throw_time > 1.6 {
            s.throw_time = -1.0
        }
    }
    s.vitality.heal_delay = (s.vitality.heal_delay - dt).max(0.0);
    s.vitality.hurt_flash = (s.vitality.hurt_flash - dt).max(0.0);
    for f in &mut w.fish {
        f.life.pop_time = (f.life.pop_time - dt).max(0.0)
    }
    s.cooldown = (s.cooldown - dt).max(0.0);
    s.vitality.stunned = (s.vitality.stunned - dt).max(0.0);
    s.vitality.invulnerable = (s.vitality.invulnerable - dt).max(0.0);
}
pub fn song(w: &mut World, dt: f32, input: Input, e: &mut Events) {
    if input.toggle_song {
        toggle_song(w);
    }
    let duration = song_duration(w);
    let recharge = song_recharge_duration(w);
    let s = &mut w.sabrina.song;
    if s.active {
        s.energy = (s.energy - dt * 100.0 / duration).max(0.0);
        if s.energy <= 0.001 {
            s.energy = 0.0;
            s.active = false;
            s.locked = true;
            e.song_empty = true
        }
    } else {
        s.energy = (s.energy + dt * 100.0 / recharge).min(100.0);
        if s.energy >= 99.999 {
            s.energy = 100.0;
            s.locked = false
        }
    }
}
pub fn player_swimming(w: &mut World, dt: f32, mut input: Input) {
    let target = find_jelly(w, w.crystal.magic.target).map(|j| j.motion.pos);
    let s = &mut w.sabrina;
    s.wave.cooldown = (s.wave.cooldown - dt).max(0.0);
    if input.movement.length() > 1.0 {
        input.movement = input.movement.unit()
    }
    let effort = input.movement.length();
    s.swimming.phase +=
        dt * (1.9634954 + effort * 2.3998277) * if w.settings.reduced_motion { 0.25 } else { 1.0 };
    s.song.strength = (s.song.strength
        + dt * if s.song.active && s.vitality.stunned <= 0.0 {
            2.0
        } else {
            -1.2
        })
    .clamp(0.0, 1.0);
    let normal = v(-input.movement.y, input.movement.x);
    let desired = if s.vitality.stunned > 0.0 {
        v(0.0, 0.0)
    } else {
        input.movement * (7.2 - s.song.strength * 1.6)
            + normal * (s.swimming.phase.cos() * 1.2 * effort)
    };
    s.motion.velocity = s.motion.velocity + (desired - s.motion.velocity) * (dt * 9.0).min(1.0);
    s.motion.pos = bounds(s.motion.pos + s.motion.velocity * dt);
    s.motion.pos.y = s.motion.pos.y.max(3.2);
    if s.motion.velocity.x.abs() > 0.25 {
        s.swimming.facing = if s.motion.velocity.x < 0.0 { -1.0 } else { 1.0 }
    }
    let mut moving = ((s.motion.velocity.length() - 0.3) / 3.5).clamp(0.0, 1.0);
    let mut heading = s
        .motion
        .velocity
        .y
        .atan2(s.motion.velocity.x.abs().max(0.15));
    if w.crystal.magic.state == Spell::Charging
        && let Some(target) = target
    {
        let aim = target - s.motion.pos;
        if aim.x.abs() > 0.1 {
            s.swimming.facing = if aim.x < 0.0 { -1.0 } else { 1.0 }
        }
        heading = aim.y.atan2(aim.x.abs().max(0.15));
        moving = 1.0;
    }
    let target_pitch = 0.65 * (1.0 - moving) + heading.clamp(-1.1, 1.1) * moving;
    s.swimming.pitch += (target_pitch - s.swimming.pitch) * (1.0 - (-4.0 * dt).exp());
    let target_yaw = if s.swimming.facing < 0.0 {
        -3.14159265
    } else {
        0.0
    };
    s.swimming.yaw += (target_yaw - s.swimming.yaw) * (1.0 - (-4.5 * dt).exp());
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
}
pub fn sabrina_pose(s: &SabrinaEntity) -> SabrinaPose {
    if s.vitality.hurt_flash > 0.0 || s.vitality.stunned > 0.0 {
        return SabrinaPose::Hurt;
    }
    if s.throw_time >= 0.0 {
        return SabrinaPose::Throw;
    }
    if s.wave.active {
        return SabrinaPose::Wave;
    }
    if s.song.active {
        return SabrinaPose::Sing;
    }
    let effort = (s.motion.velocity.length() / 7.2).min(1.0);
    if s.swimming.yaw.sin().abs() > 0.4 && effort > 0.15 {
        return if s.swimming.facing < 0.0 {
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
pub fn sabrina_pose_phase(s: &SabrinaEntity) -> f32 {
    match sabrina_pose(s) {
        SabrinaPose::Throw => (s.throw_time / 1.6).clamp(0.0, 1.0),
        SabrinaPose::Wave => (s.wave.time / 0.75).clamp(0.0, 1.0),
        _ => (s.swimming.phase % 6.28318530718) / 6.28318530718,
    }
}
