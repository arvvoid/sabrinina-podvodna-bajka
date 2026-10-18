// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT

use reef_sim::*;

#[test]
fn maintenance_snapshot() {
    for seed in [1, 27, 5489] {
        for gentle in [false, true] {
            let mut world = World::default();
            reset(
                &mut world,
                seed,
                Settings {
                    gentle,
                    rest_seconds: 25.0,
                    visitor_limit: 3,
                    ..Settings::default()
                },
            );
            let mut hash = 14695981039346656037_u64;
            for tick in 0..12000 {
                let phase = tick as f32 * 0.013;
                let events = step(
                    &mut world,
                    1.0 / 60.0,
                    Input {
                        movement: v(phase.cos(), (phase * 0.7).sin()),
                        cast: tick % 541 == 0,
                        toggle_song: tick % 379 == 0,
                        wave: tick % 181 == 0,
                    },
                );
                let snapshot = format!(
                    "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{}",
                    world.fish,
                    world.jelly,
                    world.sabrina,
                    world.crystal,
                    world.session,
                    world.points,
                    events,
                    world.rng.clone().next_u32()
                );
                for byte in snapshot.bytes() {
                    hash = (hash ^ byte as u64).wrapping_mul(1099511628211);
                }
                if world.session.game_over {
                    reset(
                        &mut world,
                        seed + tick,
                        Settings {
                            gentle,
                            rest_seconds: 25.0,
                            visitor_limit: 3,
                            ..Settings::default()
                        },
                    );
                }
            }
            println!("SNAPSHOT {seed} {gentle} {hash:016x}");
        }
    }
}
