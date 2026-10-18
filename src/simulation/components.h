// SPDX-License-Identifier: Apache-2.0
#pragma once
#include <cmath>
#include <cstdint>
namespace reef {
struct V2 {
    float x = 0, y = 0;
    V2 operator+(V2 b) const { return {x + b.x, y + b.y}; }
    V2 operator-(V2 b) const { return {x - b.x, y - b.y}; }
    V2 operator*(float s) const { return {x * s, y * s}; }
    float length() const { return std::sqrt(x * x + y * y); }
    V2 unit() const {
        float l = length();
        return l > .0001f ? *this * (1 / l) : V2{};
    }
};
enum class EntityKind : uint8_t { None, Sabrina, Fish, Jelly, Crystal };
struct Entity {
    int slot = -1;
    uint64_t generation = 0;
    EntityKind kind = EntityKind::None;
};
inline bool operator==(Entity a, Entity b) {
    return a.slot == b.slot && a.generation == b.generation && a.kind == b.kind;
}
inline bool operator!=(Entity a, Entity b) {
    return !(a == b);
}
inline constexpr Entity no_entity{};
enum class Water { Calm, Visit };
enum class JellyState { Hidden, Arriving, Hungry, Leaving };
enum class Spell { Resting, Charging, Flying, Burst };
struct Motion {
    V2 pos{}, velocity{}, direction{1, 0};
};
struct FishAppearance {
    float size = 1, depth = 0, birth = 0;
    int species = 0;
};
struct FishSwimming {
    float phase = 0, turn_in = 0, cruise_heading = 0, speed = 1, tail_phase = 0;
};
struct WaterContact {
    V2 impulse{};
    float turn_time = 0;
};
struct FishRescue {
    V2 teleport_from{}, teleport_to{};
    float shield = 0, teleport = 0;
    int rescued_cycle = -1, wave_rescued_cycle = -1;
    uint64_t wave_hit = 0;
    float wave_flow = 0, song_escort = 0;
    int song_risk_cycle = -1, song_rescued_cycle = -1;
};
struct FishLife {
    bool active = false;
    float pop_time = 0;
};
struct JellyBehaviour {
    JellyState state = JellyState::Hidden;
    float time = 0, hunger = 20, turn_in = 0, eat_wait = 0, phase = 0;
    int eaten = 0, kind = 0, exit_side = 1;
    float song_check = 0, song_interest = 0, repel_flash = 0;
};
struct CrystalOrbit {
    V2 offset;
    float depth;
};
struct CrystalMagic {
    Spell state = Spell::Resting;
    float time = 0;
    Entity target{};
    V2 charge_offset{};
    float charge_depth = 1, depth = 1;
};
struct Health {
    float health = 100, heal_delay = 0, hurt_flash = 0, stunned = 0, invulnerable = 0;
};
struct Swimming {
    float facing = 1, phase = 0, pitch = .65f, yaw = 0;
};
struct Song {
    bool active = false, locked = false;
    float energy = 100, strength = 0;
};
struct WaterWave {
    float cooldown = 0, time = 0;
    bool active = false;
    uint64_t cast_id = 0;
    V2 origin{}, direction{1, 0};
};
// Fixed archetypes: composition of plain components, no entity inheritance or callbacks.
struct FishEntity {
    Entity id{};
    Motion motion;
    FishAppearance appearance;
    FishSwimming swimming;
    WaterContact contact;
    FishRescue rescue;
    FishLife life;
};
struct JellyEntity {
    Entity id{};
    Motion motion{{}, {}, {}};
    JellyBehaviour behaviour;
};
struct CrystalEntity {
    Entity id{};
    Motion motion;
    CrystalMagic magic;
};
struct SabrinaEntity {
    Entity id{};
    Motion motion{{-6, 6}, {}, {1, 0}};
    Health vitality;
    Swimming swimming;
    Song song;
    WaterWave wave;
    float cooldown = 0;
    float throw_time = -1;
};
struct Settings {
    bool gentle = true, aquarium = false;
    float rest_seconds = 55;
    int visitor_limit = 2;
    bool reduced_motion = false;
};
struct Session {
    Water water = Water::Calm;
    float time = 0, rest_remaining = 55, visit_time = 0, spawn_in = 7;
    int cycle = 0;
    bool game_over = false;
};
struct Score {
    int score = 0, last_score = 0, total_score = 0, best_score = 0, rescues = 0, visit_rescues = 0,
        total_eaten = 0, last_survivors = 0, last_variety = 0, visit_start_fish = 0;
};
struct Input {
    V2 move{};
    bool cast = false, toggle_song = false, wave = false;
};
struct Events {
    int spawned = 0, teleported = 0, eaten = 0, repelled = 0;
    bool wave_cast = false, song_empty = false, game_over = false;
    bool cast = false, launch = false, hit = false, visit_started = false, visit_ended = false,
         leaving = false;
};
} // namespace reef
