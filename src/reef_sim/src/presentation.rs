// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Read-only render sampling. Never feed this state back into gameplay systems.
use crate::*;

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn vector(a: V2, b: V2, t: f32) -> V2 {
    a + (b - a) * t
}

fn motion(a: Motion, b: Motion, t: f32) -> Motion {
    Motion {
        pos: vector(a.pos, b.pos, t),
        velocity: vector(a.velocity, b.velocity, t),
        direction: vector(a.direction, b.direction, t).unit(),
    }
}

/// Interpolate continuous presentation values between adjacent fixed ticks.
/// Identity and state changes use the current state, so spawning, resets and
/// spell transitions never blend an old entity or unrelated spell path.
pub fn interpolate(previous: &World, current: &World, alpha: f32) -> World {
    let t = alpha.clamp(0.0, 1.0);
    let mut out = current.clone();
    out.session.time = mix(previous.session.time, current.session.time, t);
    let a = &previous.sabrina;
    let b = &mut out.sabrina;
    if a.id == b.id {
        b.motion = motion(a.motion, b.motion, t);
        b.swimming.phase = mix(a.swimming.phase, b.swimming.phase, t);
        // These angles are continuous in the simulation, including half-turns.
        b.swimming.pitch = mix(a.swimming.pitch, b.swimming.pitch, t);
        b.swimming.yaw = mix(a.swimming.yaw, b.swimming.yaw, t);
        b.song.strength = mix(a.song.strength, b.song.strength, t);
        b.song.energy = mix(a.song.energy, b.song.energy, t);
        if a.throw_time >= 0.0 && b.throw_time >= 0.0 {
            b.throw_time = mix(a.throw_time, b.throw_time, t);
        }
        if a.wave.active && b.wave.active && a.wave.cast_id == b.wave.cast_id {
            b.wave.time = mix(a.wave.time, b.wave.time, t);
        }
    }
    for (a, b) in previous.fish.iter().zip(&mut out.fish) {
        if a.id != b.id || a.life.active != b.life.active {
            continue;
        }
        b.motion = motion(a.motion, b.motion, t);
        b.appearance.depth = mix(a.appearance.depth, b.appearance.depth, t);
        b.appearance.birth = mix(a.appearance.birth, b.appearance.birth, t);
        b.swimming.speed = mix(a.swimming.speed, b.swimming.speed, t);
        b.swimming.tail_phase = mix(a.swimming.tail_phase, b.swimming.tail_phase, t);
        if a.life.pop_time > 0.0 && b.life.pop_time > 0.0 {
            b.life.pop_time = mix(a.life.pop_time, b.life.pop_time, t);
        }
    }
    for (a, b) in previous.jelly.iter().zip(&mut out.jelly) {
        if a.id != b.id
            || a.behaviour.state == JellyState::Hidden
            || b.behaviour.state == JellyState::Hidden
        {
            continue;
        }
        b.motion = motion(a.motion, b.motion, t);
        if a.behaviour.repel_flash >= b.behaviour.repel_flash {
            b.behaviour.repel_flash = mix(a.behaviour.repel_flash, b.behaviour.repel_flash, t);
        }
    }
    let a = &previous.crystal;
    let b = &mut out.crystal;
    if a.id == b.id && a.magic.state == b.magic.state {
        b.motion = motion(a.motion, b.motion, t);
        b.magic.time = mix(a.magic.time, b.magic.time, t);
        b.magic.depth = mix(a.magic.depth, b.magic.depth, t);
    }
    out
}
