// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Fixed-capacity entity storage, generation-safe handles and seeded randomness.
use crate::*;

pub const FISH_COUNT: usize = 72;

pub const JELLY_COUNT: usize = 4;
pub const FINAL_ROUND: i32 = 10;
pub const NORMAL_VISITOR_LIMIT: i32 = 3;

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

pub fn allocate_entity(world: &mut World, kind: EntityKind, slot: i32) -> Entity {
    let entity = Entity {
        slot,
        generation: world.next_generation,
        kind,
    };
    world.next_generation += 1;
    entity
}

pub fn alive(world: &World) -> i32 {
    world.fish.iter().filter(|fish| fish.life.active).count() as i32
}

pub fn hungry(world: &World) -> i32 {
    world
        .jelly
        .iter()
        .filter(|jellyfish| {
            matches!(
                jellyfish.behaviour.state,
                JellyState::Arriving | JellyState::Hungry
            )
        })
        .count() as i32
}

pub fn find_fish(world: &World, entity: Entity) -> Option<&FishEntity> {
    if entity.kind != EntityKind::Fish || entity.generation == 0 {
        return None;
    }
    world
        .fish
        .get(entity.slot as usize)
        .filter(|fish| fish.id == entity)
}

pub fn find_jelly(world: &World, entity: Entity) -> Option<&JellyEntity> {
    if entity.kind != EntityKind::Jelly || entity.generation == 0 {
        return None;
    }
    world
        .jelly
        .get(entity.slot as usize)
        .filter(|jellyfish| jellyfish.id == entity)
}

pub fn add_fish(world: &mut World, count: i32, from_edge: bool) -> i32 {
    if count <= 0 {
        return 0;
    }
    let mut added = 0;
    for i in 0..FISH_COUNT {
        if world.fish[i].life.active || world.fish[i].life.pop_time > 0.0 {
            continue;
        }
        let mut fish = FishEntity {
            id: allocate_entity(world, EntityKind::Fish, i as i32),
            ..FishEntity::default()
        };
        fish.life.active = true;
        fish.appearance.species = world.rng.range(0.0, 5.999) as usize;
        fish.appearance.size = world.rng.range(0.55, 1.18);
        fish.appearance.depth = world.rng.range(-1.7, 0.6);
        fish.swimming.phase = world.rng.range(0.0, 6.283);
        fish.motion.pos = v(
            if from_edge {
                if world.rng.range(0.0, 1.0) < 0.5 {
                    -15.0
                } else {
                    15.0
                }
            } else {
                world.rng.range(-14.0, 14.0)
            },
            world.rng.range(3.0, 15.0),
        );
        let heading = if world.rng.range(0.0, 1.0) < 0.5 {
            0.0
        } else {
            3.14159
        } + world.rng.range(-0.25, 0.25);
        fish.swimming.cruise_heading = heading;
        fish.swimming.speed = 0.8 + fish.appearance.size * 0.65;
        fish.swimming.tail_phase = world.rng.range(0.0, 6.283);
        fish.motion.direction = v(heading.cos(), heading.sin());
        fish.motion.velocity = fish.motion.direction * fish.swimming.speed;
        fish.swimming.turn_in = world.rng.range(2.0, 6.0);
        fish.appearance.birth = if from_edge { 0.0 } else { 1.0 };
        world.fish[i] = fish;
        added += 1;
        if added >= count {
            break;
        }
    }
    added
}

pub fn reset(world: &mut World, seed: u32, mut settings: Settings) {
    // Keep generations increasing across resets so old spell targets stay invalid.
    let generation = world.next_generation;
    *world = World::default();
    world.next_generation = generation;
    world.sabrina.id = allocate_entity(world, EntityKind::Sabrina, 0);
    world.crystal.id = allocate_entity(world, EntityKind::Crystal, 0);
    world.rng = Rng::new(seed);
    settings.rest_seconds = settings.rest_seconds.clamp(25.0, 150.0);
    settings.visitor_limit = settings.visitor_limit.clamp(1, NORMAL_VISITOR_LIMIT);
    world.settings = settings;
    world.session.rest_remaining = random_rest_duration(world);
    add_fish(world, 10, false);
}
