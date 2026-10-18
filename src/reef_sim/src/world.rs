// SPDX-License-Identifier: MIT
use crate::*;
pub const FISH_COUNT: usize = 72;
pub const JELLY_COUNT: usize = 3;
pub const SPECIES_COUNT: usize = 6;
// Match std::mt19937 and libstdc++ uniform_real_distribution<float>, including
// rounding before scaling. Stable slot iteration is part of the gameplay contract.
#[derive(Clone)]
pub struct Rng {
    state: [u32; 624],
    index: usize,
}
impl Rng {
    pub fn new(seed: u32) -> Self {
        let mut s = Self {
            state: [0; 624],
            index: 624,
        };
        s.state[0] = seed;
        for i in 1..624 {
            s.state[i] = 1812433253u32
                .wrapping_mul(s.state[i - 1] ^ (s.state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        s
    }
    pub fn next_u32(&mut self) -> u32 {
        if self.index == 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x80000000) | (self.state[(i + 1) % 624] & 0x7fffffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (y >> 1)
                    ^ if y & 1 != 0 { 0x9908b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        let n = (self.next_u32() as f32 / 4294967296.0).min(f32::from_bits(0x3f7fffff));
        n * (b - a) + a
    }
}
#[derive(Clone)]
pub struct World {
    pub fish: [FishEntity; FISH_COUNT],
    pub jelly: [JellyEntity; JELLY_COUNT],
    pub sabrina: SabrinaEntity,
    pub crystal: CrystalEntity,
    pub settings: Settings,
    pub session: Session,
    pub points: Score,
    pub rng: Rng,
    pub next_generation: u64,
}
impl Default for World {
    fn default() -> Self {
        Self {
            fish: [FishEntity::default(); FISH_COUNT],
            jelly: [JellyEntity::default(); JELLY_COUNT],
            sabrina: SabrinaEntity::default(),
            crystal: CrystalEntity::default(),
            settings: Settings::default(),
            session: Session::default(),
            points: Score::default(),
            rng: Rng::new(27),
            next_generation: 1,
        }
    }
}
pub fn bounds(p: V2) -> V2 {
    v(p.x.clamp(-15.3, 15.3), p.y.clamp(2.0, 16.2))
}
pub fn allocate_entity(w: &mut World, kind: EntityKind, slot: i32) -> Entity {
    let e = Entity {
        slot,
        generation: w.next_generation,
        kind,
    };
    w.next_generation += 1;
    e
}
pub fn alive(w: &World) -> i32 {
    w.fish.iter().filter(|f| f.life.active).count() as i32
}
pub fn hungry(w: &World) -> i32 {
    w.jelly
        .iter()
        .filter(|j| matches!(j.behaviour.state, JellyState::Arriving | JellyState::Hungry))
        .count() as i32
}
pub fn find_fish(w: &World, e: Entity) -> Option<&FishEntity> {
    if e.kind != EntityKind::Fish || e.generation == 0 {
        return None;
    }
    w.fish.get(e.slot as usize).filter(|f| f.id == e)
}
pub fn find_jelly(w: &World, e: Entity) -> Option<&JellyEntity> {
    if e.kind != EntityKind::Jelly || e.generation == 0 {
        return None;
    }
    w.jelly.get(e.slot as usize).filter(|j| j.id == e)
}
pub fn add_fish(w: &mut World, count: i32, from_edge: bool) -> i32 {
    if count <= 0 {
        return 0;
    }
    let mut added = 0;
    for i in 0..FISH_COUNT {
        if w.fish[i].life.active || w.fish[i].life.pop_time > 0.0 {
            continue;
        }
        let mut f = FishEntity {
            id: allocate_entity(w, EntityKind::Fish, i as i32),
            ..FishEntity::default()
        };
        f.life.active = true;
        f.appearance.species = w.rng.range(0.0, 5.999) as usize;
        f.appearance.size = w.rng.range(0.55, 1.18);
        f.appearance.depth = w.rng.range(-1.7, 0.6);
        f.swimming.phase = w.rng.range(0.0, 6.283);
        f.motion.pos = v(
            if from_edge {
                if w.rng.range(0.0, 1.0) < 0.5 {
                    -15.0
                } else {
                    15.0
                }
            } else {
                w.rng.range(-14.0, 14.0)
            },
            w.rng.range(3.0, 15.0),
        );
        let heading = if w.rng.range(0.0, 1.0) < 0.5 {
            0.0
        } else {
            3.14159
        } + w.rng.range(-0.25, 0.25);
        f.swimming.cruise_heading = heading;
        f.swimming.speed = 0.8 + f.appearance.size * 0.65;
        f.swimming.tail_phase = w.rng.range(0.0, 6.283);
        f.motion.direction = v(heading.cos(), heading.sin());
        f.motion.velocity = f.motion.direction * f.swimming.speed;
        f.swimming.turn_in = w.rng.range(2.0, 6.0);
        f.appearance.birth = if from_edge { 0.0 } else { 1.0 };
        w.fish[i] = f;
        added += 1;
        if added >= count {
            break;
        }
    }
    added
}
pub fn reset(w: &mut World, seed: u32, mut settings: Settings) {
    let generation = w.next_generation;
    *w = World::default();
    w.next_generation = generation;
    w.sabrina.id = allocate_entity(w, EntityKind::Sabrina, 0);
    w.crystal.id = allocate_entity(w, EntityKind::Crystal, 0);
    w.rng = Rng::new(seed);
    settings.rest_seconds = settings.rest_seconds.clamp(25.0, 150.0);
    settings.visitor_limit = settings.visitor_limit.clamp(1, 3);
    w.settings = settings;
    w.session.rest_remaining = settings.rest_seconds;
    add_fish(w, 10, false);
}
