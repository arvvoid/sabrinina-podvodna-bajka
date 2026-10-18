// SPDX-License-Identifier: MIT
//! Soft body contacts and pose-based collision shapes.
use crate::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct Capsule {
    pub a: V2,
    pub b: V2,
    pub radius: f32,
}
fn cap(a: V2, b: V2, radius: f32) -> Capsule {
    Capsule { a, b, radius }
}
fn closest(a: Capsule, b: Capsule) -> (V2, V2) {
    let d1 = a.b - a.a;
    let d2 = b.b - b.a;
    let r = a.a - b.a;
    let aa = d1.dot(d1);
    let ee = d2.dot(d2);
    let ff = d2.dot(r);
    let mut s = 0.0;
    let mut t;
    if aa < 1e-8 && ee < 1e-8 {
        return (a.a, b.a);
    }
    if aa < 1e-8 {
        t = (ff / ee).clamp(0.0, 1.0)
    } else {
        let c = d1.dot(r);
        if ee < 1e-8 {
            s = (-c / aa).clamp(0.0, 1.0);
            t = 0.0
        } else {
            let bb = d1.dot(d2);
            let denom = aa * ee - bb * bb;
            if denom > 1e-8 {
                s = ((bb * ff - c * ee) / denom).clamp(0.0, 1.0)
            }
            t = (bb * s + ff) / ee;
            if t < 0.0 {
                t = 0.0;
                s = (-c / aa).clamp(0.0, 1.0)
            } else if t > 1.0 {
                t = 1.0;
                s = ((bb - c) / aa).clamp(0.0, 1.0)
            }
        }
    }
    (a.a + d1 * s, b.a + d2 * t)
}
fn impulse(f: &mut FishEntity, change: V2, normal: V2) {
    f.contact.impulse = f.contact.impulse + change;
    let speed = f.contact.impulse.length();
    if speed > 6.0 {
        f.contact.impulse = f.contact.impulse * (6.0 / speed)
    }
    f.motion.velocity = f.motion.velocity + change;
    f.contact.turn_time = 0.65;
    let escape = f.motion.direction + normal * 1.8;
    f.swimming.cruise_heading = escape.y.atan2(escape.x);
    f.swimming.turn_in = f.swimming.turn_in.max(1.2);
}
fn fallback(a: usize, b: usize) -> V2 {
    let angle = ((a * 73 + b * 151) % 360) as f32 * 0.0174532925;
    v(angle.cos(), angle.sin())
}
fn arm_sample(s: &SabrinaEntity, index: usize) -> [f32; 3] {
    let poses = &crate::generated_contacts::ARMS[sabrina_pose(s) as usize];
    let sample = sabrina_pose_phase(s) * 31.0;
    let frame = (sample as usize).min(30);
    let blend = sample - frame as f32;
    let a = poses[frame][index];
    let b = poses[frame + 1][index];
    [
        a[0] + (b[0] - a[0]) * blend,
        a[1] + (b[1] - a[1]) * blend,
        a[2] + (b[2] - a[2]) * blend,
    ]
}
pub fn sabrina_palm_offset() -> f32 {
    crate::generated_contacts::PALM_OFFSET
}
pub fn sabrina_palm(s: &SabrinaEntity) -> CrystalOrbit {
    let [x, y, z] = arm_sample(s, 6);
    let xy = v(x, y).rotate(s.swimming.pitch);
    CrystalOrbit {
        offset: s.motion.pos + v(xy.x * s.swimming.yaw.cos() + z * s.swimming.yaw.sin(), xy.y),
        depth: 0.5 - xy.x * s.swimming.yaw.sin() + z * s.swimming.yaw.cos(),
    }
}
pub fn fish_body(f: &FishEntity) -> Capsule {
    let size = f.appearance.size * (0.3 + 0.7 * f.appearance.birth);
    let axis = f.motion.direction.unit();
    cap(
        f.motion.pos - axis * (0.38 * size),
        f.motion.pos + axis * (0.23 * size),
        0.43 * size,
    )
}
pub fn sabrina_body(s: &SabrinaEntity) -> [Capsule; 16] {
    let point = |p: V2| {
        let p = p.rotate(s.swimming.pitch);
        s.motion.pos + v(p.x * s.swimming.yaw.cos(), p.y)
    };
    let shape = |a, b, r| cap(point(a), point(b), r);
    let mut body = [Capsule::default(); 16];
    body[0] = shape(v(0.82, 0.95), v(0.65, 0.72), 0.32);
    body[1] = shape(v(0.65, 0.72), v(0.0, 0.0), 0.40);
    let rest = [
        v(0.0, 0.0),
        v(-0.4224, -0.14784),
        v(-0.88704, -0.261888),
        v(-1.35168, -0.29568),
        v(-1.73184, -0.2112),
        v(-2.04864, -0.06336),
        v(-2.28096, 0.04224),
    ];
    let radii = [0.38, 0.35, 0.30, 0.26, 0.22, 0.20];
    let swimming = s.motion.velocity.length() > 0.864
        && !s.song.active
        && !s.wave.active
        && s.throw_time < 0.0
        && s.vitality.stunned <= 0.0
        && s.vitality.hurt_flash <= 0.0;
    let effort = if swimming { 1.0 } else { 0.75 };
    let phase = sabrina_pose_phase(s) * 6.2831853;
    let mut angle = 0.0;
    let speed = (s.motion.velocity.length() / 7.2).min(1.0);
    let resting =
        ((s.swimming.pitch - 0.2) / 0.45).clamp(0.0, 1.0) * (1.0 - (speed * 2.0).clamp(0.0, 1.0));
    let mut end = v(0.0, 0.0);
    for i in 0..6 {
        angle += if i == 0 {
            0.30
        } else if i == 1 {
            0.10
        } else {
            0.0
        } * resting;
        angle += effort * (0.13 + 0.028 * i as f32) * (phase - i as f32 * 0.72).sin();
        let next = end + (rest[i + 1] - rest[i]).rotate(angle);
        body[i + 2] = shape(end, next, radii[i]);
        end = next;
    }
    let fan = angle + 0.26 * effort * (phase - 4.1).sin();
    let tip = fan + 0.20 * effort * (phase - 4.7).sin();
    for side in 0..2 {
        let sign = if side == 0 { 1.0 } else { -1.0 };
        let middle = end + v(-0.4224, sign * 0.4224).rotate(fan);
        let edge = middle + v(-0.38016, sign * 0.2112).rotate(tip);
        body[8 + side * 2] = shape(end, middle, 0.22);
        body[9 + side * 2] = shape(middle, edge, 0.20);
    }
    let arm_point = |index| {
        let [x, y, z] = arm_sample(s, index);
        let xy = v(x, y).rotate(s.swimming.pitch);
        s.motion.pos + v(xy.x * s.swimming.yaw.cos() + z * s.swimming.yaw.sin(), xy.y)
    };
    for side in 0..2 {
        body[12 + side * 2] = cap(arm_point(side * 3), arm_point(side * 3 + 1), 0.16);
        body[13 + side * 2] = cap(arm_point(side * 3 + 1), arm_point(side * 3 + 2), 0.14);
    }
    body
}
pub fn jelly_scale(j: &JellyEntity) -> f32 {
    0.95 + 0.9 * (1.0 - (-0.07 * j.behaviour.eaten as f32).exp())
}
pub fn contact_displacement(moving: Capsule, solid: Capsule) -> V2 {
    let (a, b) = closest(moving, solid);
    let delta = a - b;
    let distance = delta.length();
    let overlap = moving.radius + solid.radius - distance;
    if overlap <= 0.0 {
        return v(0.0, 0.0);
    }
    let mut normal = if distance > 0.0001 {
        delta * (1.0 / distance)
    } else {
        ((moving.a + moving.b) - (solid.a + solid.b)).unit()
    };
    if normal.length() < 0.1 {
        normal = v(1.0, 0.0)
    }
    normal * (overlap + 0.002)
}
pub fn jelly_body(j: &JellyEntity) -> [Capsule; 3] {
    let size = jelly_scale(j);
    let p = |x, y| j.motion.pos + v(x, y) * size;
    [
        cap(p(0.0, 0.05), p(0.0, 0.38), 0.90 * size),
        cap(p(-0.42, -0.3), p(-0.42, -1.25), 0.40 * size),
        cap(p(0.42, -0.3), p(0.42, -1.25), 0.40 * size),
    ]
}
pub fn contact_gap(a: Capsule, b: Capsule) -> f32 {
    let (p, q) = closest(a, b);
    (p - q).length() - a.radius - b.radius
}
pub fn soft_contacts(w: &mut World) {
    let body = sabrina_body(&w.sabrina);
    for _ in 0..6 {
        for i in 0..FISH_COUNT {
            let mut a = w.fish[i];
            if !a.life.active || a.rescue.teleport > 0.0 {
                continue;
            }
            for j in i + 1..FISH_COUNT {
                let b = &mut w.fish[j];
                if !b.life.active
                    || b.rescue.teleport > 0.0
                    || (a.motion.pos - b.motion.pos).length() > 3.0
                {
                    continue;
                }
                let ca = fish_body(&a);
                let cb = fish_body(b);
                let (p, q) = closest(ca, cb);
                let delta = p - q;
                let distance = delta.length();
                let penetration = ca.radius + cb.radius - distance;
                if penetration <= 0.0 {
                    continue;
                }
                let normal = if distance > 0.0001 {
                    delta * (1.0 / distance)
                } else {
                    fallback(i, j)
                };
                let ia = 1.0 / (ca.radius * ca.radius);
                let ib = 1.0 / (cb.radius * cb.radius);
                let correction = (penetration + 0.001).min(0.45);
                a.motion.pos = bounds(a.motion.pos + normal * (correction * ia / (ia + ib)));
                b.motion.pos = bounds(b.motion.pos - normal * (correction * ib / (ia + ib)));
                let closing = (a.motion.velocity - b.motion.velocity).dot(normal);
                if closing < 0.0 {
                    let force = -(1.0 + 0.16) * closing / (ia + ib);
                    impulse(&mut a, normal * (force * ia), normal);
                    impulse(b, normal * (-force * ib), normal * -1.0);
                }
            }
            for capsule in body {
                let ca = fish_body(&a);
                let (p, q) = closest(ca, capsule);
                let delta = p - q;
                let distance = delta.length();
                let penetration = ca.radius + capsule.radius - distance;
                if penetration <= 0.0 {
                    continue;
                }
                let normal = if distance > 0.0001 {
                    delta * (1.0 / distance)
                } else {
                    fallback(i, 97)
                };
                a.motion.pos = bounds(a.motion.pos + normal * (penetration + 0.002).min(0.55));
                let closing = (a.motion.velocity - w.sabrina.motion.velocity).dot(normal);
                if closing < 0.0 {
                    impulse(&mut a, normal * (-1.08 * closing), normal)
                }
            }
            w.fish[i] = a;
        }
    }
}
