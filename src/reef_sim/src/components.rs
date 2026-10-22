// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Plain gameplay data shared by the fixed-step systems.
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

impl Add for V2 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        v(self.x + b.x, self.y + b.y)
    }
}

impl Sub for V2 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        v(self.x - b.x, self.y - b.y)
    }
}

impl Mul<f32> for V2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        v(self.x * s, self.y * s)
    }
}

impl V2 {
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn unit(self) -> Self {
        let l = self.length();
        if l > 0.0001 {
            self * (1.0 / l)
        } else {
            Self::default()
        }
    }

    pub fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y
    }

    pub fn rotate(self, a: f32) -> Self {
        v(
            self.x * a.cos() - self.y * a.sin(),
            self.x * a.sin() + self.y * a.cos(),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EntityKind {
    #[default]
    None,
    Sabrina,
    Fish,
    Jelly,
    Crystal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity {
    pub slot: i32,
    pub generation: u64,
    pub kind: EntityKind,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            slot: -1,
            generation: 0,
            kind: EntityKind::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Water {
    #[default]
    Calm,
    Visit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JellyState {
    #[default]
    Hidden,
    Arriving,
    Hungry,
    Leaving,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Spell {
    #[default]
    Resting,
    Charging,
    Flying,
    Burst,
    Returning,
}
macro_rules! component {
    ($name:ident { $($field:ident: $ty:ty = $value:expr),* $(,)? }) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $name {
            $(pub $field: $ty),*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $($field: $value),*
                }
            }
        }
    };
}

component!(Motion {
    pos: V2 = v(0.0, 0.0),
    velocity: V2 = v(0.0, 0.0),
    direction: V2 = v(1.0, 0.0)
});
component!(FishAppearance {
    size: f32 = 1.0,
    depth: f32 = 0.0,
    birth: f32 = 0.0,
    species: usize = 0
});
component!(FishSwimming {
    phase: f32 = 0.0,
    turn_in: f32 = 0.0,
    cruise_heading: f32 = 0.0,
    speed: f32 = 1.0,
    tail_phase: f32 = 0.0,
    avoidance: V2 = v(0.0, 0.0)
});
component!(WaterContact {
    impulse: V2 = v(0.0, 0.0),
    turn_time: f32 = 0.0
});
component!(FishRescue {
    teleport_from: V2 = v(0.0, 0.0),
    teleport_to: V2 = v(0.0, 0.0),
    shield: f32 = 0.0,
    teleport: f32 = 0.0,
    rescued_cycle: i32 = -1,
    wave_hit: u64 = 0,
    wave_flow: f32 = 0.0,
    song_escort: f32 = 0.0,
    song_risk_cycle: i32 = -1,
    song_rescued_cycle: i32 = -1,
    wave_escape: f32 = 0.0
});
component!(FishLife {
    active: bool = false,
    pop_time: f32 = 0.0
});
component!(JellyBehaviour {
    state: JellyState = JellyState::Hidden,
    time: f32 = 0.0,
    turn_in: f32 = 0.0,
    eat_wait: f32 = 0.0,
    phase: f32 = 0.0,
    eaten: i32 = 0,
    kind: i32 = 0,
    exit_side: i32 = 1,
    repel_flash: f32 = 0.0,
    health: f32 = 100.0,
    bubble: f32 = 0.0,
    wave_hit: u64 = 0,
    knockback: V2 = v(0.0, 0.0)
});
component!(CrystalMagic {
    state: Spell = Spell::Resting,
    time: f32 = 0.0,
    target: Entity = Entity::default(),
    aim: V2 = v(1.0, 0.0),
    charge_offset: V2 = v(0.0, 0.0),
    charge_depth: f32 = 1.0,
    depth: f32 = 1.0,
    hits: [Entity; 4] = [Entity::default(); 4],
    hit_count: usize = 0
});
component!(Health {
    health: f32 = 100.0,
    heal_delay: f32 = 0.0,
    hurt_flash: f32 = 0.0,
    stunned: f32 = 0.0,
    invulnerable: f32 = 0.0
});
component!(Swimming {
    facing: f32 = 1.0,
    phase: f32 = 0.0,
    pitch: f32 = 0.65,
    yaw: f32 = 0.0
});
component!(Song {
    active: bool = false,
    locked: bool = false,
    energy: f32 = 100.0,
    strength: f32 = 0.0
});
component!(WaterWave {
    cooldown: f32 = 0.0,
    time: f32 = 0.0,
    active: bool = false,
    cast_id: u64 = 0,
    origin: V2 = v(0.0, 0.0),
    direction: V2 = v(1.0, 0.0),
    charging: bool = false,
    charge_time: f32 = 0.0,
    scale: f32 = 1.0
});

impl WaterWave {
    pub fn cancel_charge(&mut self) {
        self.charging = false;
        self.charge_time = 0.0;
    }
}

component!(FishEntity {
    id: Entity = Entity::default(),
    motion: Motion = Motion::default(),
    appearance: FishAppearance = FishAppearance::default(),
    swimming: FishSwimming = FishSwimming::default(),
    contact: WaterContact = WaterContact::default(),
    rescue: FishRescue = FishRescue::default(),
    life: FishLife = FishLife::default()
});
component!(JellyEntity {
    id: Entity = Entity::default(),
    motion: Motion = Motion {
        direction: v(0.0, 0.0),
        ..Motion::default()
    },
    behaviour: JellyBehaviour = JellyBehaviour::default()
});
component!(CrystalEntity {
    id: Entity = Entity::default(),
    motion: Motion = Motion::default(),
    magic: CrystalMagic = CrystalMagic::default()
});
component!(SabrinaEntity {
    id: Entity = Entity::default(),
    motion: Motion = Motion {
        pos: v(-6.0, 6.0),
        ..Motion::default()
    },
    vitality: Health = Health::default(),
    swimming: Swimming = Swimming::default(),
    song: Song = Song::default(),
    wave: WaterWave = WaterWave::default(),
    cooldown: f32 = 0.0,
    throw_time: f32 = -1.0,
    throw_direction: V2 = v(1.0, 0.0)
});
component!(Settings {
    gentle: bool = true,
    aquarium: bool = false,
    rest_seconds: f32 = 55.0,
    visitor_limit: i32 = 2,
    reduced_motion: bool = false
});
component!(Session {
    water: Water = Water::Calm,
    time: f32 = 0.0,
    rest_remaining: f32 = 55.0,
    visit_time: f32 = 0.0,
    spawn_in: f32 = 7.0,
    cycle: i32 = 0,
    game_over: bool = false,
    victory: bool = false
});

impl Session {
    pub fn finished(&self) -> bool {
        self.game_over || self.victory
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub score: i32,
    pub last_score: i32,
    pub total_score: i32,
    pub best_score: i32,
    pub rescues: i32,
    pub visit_rescues: i32,
    pub total_eaten: i32,
    pub last_survivors: i32,
    pub last_variety: i32,
    pub visit_start_fish: i32,
    pub rewards: [ScoreReward; 6],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScoreReward {
    pub points: i32,
    pub time: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScoreFeedback {
    #[default]
    None,
    Crystal,
    Wave,
    Rescue,
    Retreat,
    Survivors,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub movement: V2,
    pub cast: bool,
    pub toggle_song: bool,
    pub wave: bool,
    pub wave_held: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Events {
    pub spawned: i32,
    pub teleported: i32,
    pub eaten: i32,
    pub repelled: i32,
    pub jelly_hit: bool,
    pub wave_cast: bool,
    pub song_empty: bool,
    pub game_over: bool,
    pub victory: bool,
    pub cast: bool,
    pub launch: bool,
    pub hit: bool,
    pub visit_started: bool,
    pub visit_ended: bool,
    pub leaving: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CrystalOrbit {
    pub offset: V2,
    pub depth: f32,
}
