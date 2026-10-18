// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
use reef_sim::*;

fn encounter(gentle: bool) -> World {
    let mut world = World::default();
    reset(
        &mut world,
        27,
        Settings {
            gentle,
            visitor_limit: 1,
            ..Settings::default()
        },
    );
    begin_visit(&mut world);
    world.sabrina.motion.pos = v(-10.0, 14.0);
    world.jelly[0].motion.pos = v(0.0, 8.0);
    world.jelly[0].behaviour.state = JellyState::Hungry;
    for fish in &mut world.fish {
        fish.motion.pos = v(10.0, 4.0);
    }
    world
}

#[test]
fn crystal_hit_bubbles_instead_of_dispatching_and_returns_without_a_second_hit() {
    let mut world = encounter(true);
    world.crystal.magic.state = Spell::Flying;
    world.crystal.motion.pos = v(-2.0, 8.0);
    world.crystal.motion.direction = v(1.0, 0.0);
    world.sabrina.cooldown = 12.0;
    let mut events = Events::default();
    crystal(&mut world, 0.1, Input::default(), &mut events);
    assert_eq!(world.jelly[0].behaviour.health, 60.0);
    assert_eq!(world.jelly[0].behaviour.bubble, 3.0);
    assert_eq!(world.jelly[0].behaviour.state, JellyState::Hungry);
    assert_eq!(world.crystal.magic.state, Spell::Burst);
    assert!(events.jelly_hit);
    assert_eq!(events.repelled, 0);
    assert_eq!(world.points.score, 40);
    for _ in 0..200 {
        crystal(&mut world, 1.0 / 60.0, Input::default(), &mut events);
    }
    assert_eq!(world.crystal.magic.state, Spell::Resting);
    assert_eq!(world.jelly[0].behaviour.health, 60.0);
    assert_eq!(world.points.score, 40);
    crystal(
        &mut world,
        0.01,
        Input {
            cast: true,
            ..Input::default()
        },
        &mut events,
    );
    assert_eq!(world.crystal.magic.state, Spell::Resting);
}

#[test]
fn only_gentle_autoaims_and_lively_preserves_cast_direction() {
    for gentle in [true, false] {
        let mut world = encounter(gentle);
        world.sabrina.motion.pos = v(-5.0, 8.0);
        world.sabrina.swimming.facing = -1.0;
        let mut events = Events::default();
        crystal(
            &mut world,
            0.1,
            Input {
                cast: true,
                ..Input::default()
            },
            &mut events,
        );
        assert_eq!(world.crystal.magic.target == world.jelly[0].id, gentle);
        // Changing facing during wind-up must not retarget a Lively skillshot.
        world.sabrina.swimming.facing = 1.0;
        crystal(&mut world, 1.01, Input::default(), &mut events);
        assert_eq!(world.crystal.magic.state, Spell::Flying);
        assert_eq!(world.crystal.motion.direction.x > 0.0, gentle);
    }
}

#[test]
fn missed_skillshot_returns_and_cannot_collect_points() {
    let mut world = encounter(false);
    world.crystal.magic.state = Spell::Flying;
    world.crystal.motion.pos = v(16.9, 15.0);
    world.crystal.motion.direction = v(1.0, 0.0);
    let mut events = Events::default();
    crystal(&mut world, 0.1, Input::default(), &mut events);
    assert_eq!(world.crystal.magic.state, Spell::Returning);
    for _ in 0..200 {
        crystal(&mut world, 1.0 / 60.0, Input::default(), &mut events);
    }
    assert_eq!(world.crystal.magic.state, Spell::Resting);
    assert_eq!(world.points.score, 0);
}

#[test]
fn bubble_stops_movement_stings_and_feeding_then_expires() {
    let mut world = encounter(true);
    world.sabrina.motion.pos = world.jelly[0].motion.pos;
    world.fish[0].motion.pos = world.jelly[0].motion.pos;
    world.jelly[0].behaviour.bubble = 0.1;
    world.jelly[0].motion.velocity = v(2.0, 0.0);
    let before = world.jelly[0].motion.pos;
    let mut events = Events::default();
    jellyfish::jellyfish_and_contacts(&mut world, 0.05, &mut events);
    jellyfish::jellyfish_and_contacts(&mut world, 0.05, &mut events);
    assert_eq!(world.jelly[0].motion.pos, before);
    assert_eq!(world.sabrina.vitality.health, 100.0);
    assert!(world.fish[0].life.active);
    assert_eq!(events.eaten, 0);
    jellyfish::jellyfish_and_contacts(&mut world, 0.001, &mut events);
    assert!(events.hit);
    assert!(!world.fish[0].life.active);
}

#[test]
fn damage_rewards_are_finite_and_only_zero_health_forces_retreat() {
    let mut world = encounter(true);
    let mut events = Events::default();
    for health in [60.0, 20.0] {
        damage_jelly(&mut world, 0, CRYSTAL_DAMAGE, true, &mut events);
        assert_eq!(world.jelly[0].behaviour.health, health);
        assert_eq!(world.jelly[0].behaviour.state, JellyState::Hungry);
    }
    damage_jelly(&mut world, 0, CRYSTAL_DAMAGE, true, &mut events);
    assert_eq!(world.jelly[0].behaviour.health, 0.0);
    assert_eq!(world.jelly[0].behaviour.state, JellyState::Leaving);
    assert_eq!(world.jelly[0].behaviour.bubble, 0.0);
    assert_eq!(events.repelled, 1);
    assert_eq!(world.points.score, 100 + RETREAT_POINTS);
    damage_jelly(&mut world, 0, WAVE_DAMAGE, false, &mut events);
    assert_eq!(world.points.score, 200);
    assert_eq!(events.repelled, 1);
}

#[test]
fn visitors_never_leave_because_of_time_or_eaten_count() {
    let mut world = encounter(true);
    world.session.visit_time = 10000.0;
    world.jelly[0].behaviour.eaten = 100;
    let mut events = Events::default();
    jellyfish::jellyfish_and_contacts(&mut world, 0.01, &mut events);
    assert_eq!(world.jelly[0].behaviour.state, JellyState::Hungry);
    assert_eq!(world.points.score, 0);
    assert!(!events.leaving);
}

#[test]
fn wave_only_hits_close_visitors_in_front_and_only_once() {
    for (position, hit) in [
        (v(2.0, 8.5), true),
        (v(5.0, 8.5), false),
        (v(-1.0, 8.5), false),
        (v(1.5, 14.0), false),
    ] {
        let mut world = encounter(true);
        world.sabrina.motion.pos = v(0.0, 8.0);
        world.jelly[0].motion.pos = position;
        let mut events = Events::default();
        water_wave(
            &mut world,
            0.01,
            Input {
                wave: true,
                ..Input::default()
            },
            &mut events,
        );
        for _ in 0..80 {
            water_wave(&mut world, 0.01, Input::default(), &mut events);
        }
        assert_eq!(
            world.jelly[0].behaviour.health,
            if hit { 90.0 } else { 100.0 }
        );
        assert_eq!(world.points.score, if hit { 10 } else { 0 });
        assert_eq!(world.jelly[0].behaviour.bubble, 0.0);
    }
}

#[test]
fn close_wave_hits_visible_body_including_tentacles_and_pushes_in_real_steps() {
    for side in [-1.0, 1.0] {
        for (horizontal, vertical) in [(0.4, 0.5), (2.8, 0.5), (1.5, 2.4)] {
            let mut world = encounter(true);
            world.sabrina.motion.pos = v(0.0, 8.0);
            world.sabrina.swimming.facing = side;
            world.sabrina.swimming.yaw = if side < 0.0 {
                -std::f32::consts::PI
            } else {
                0.0
            };
            world.sabrina.vitality.invulnerable = 10.0;
            world.jelly[0].motion.pos = v(horizontal * side, 8.0 + vertical);
            world.jelly[0].behaviour.bubble = 3.0;
            let before = world.jelly[0].motion.pos;
            step(
                &mut world,
                1.0 / 60.0,
                Input {
                    wave: true,
                    ..Input::default()
                },
            );
            for _ in 0..45 {
                step(&mut world, 1.0 / 60.0, Input::default());
            }
            assert_eq!(
                world.jelly[0].behaviour.health, 90.0,
                "side {side}, offset {horizontal},{vertical}"
            );
            assert!((world.jelly[0].motion.pos.x - before.x) * side > 0.7);
            assert_eq!(world.points.score, 10);
        }
    }
}

#[test]
fn returned_crystal_orbits_while_cooldown_still_blocks_casts() {
    let mut world = encounter(true);
    world.crystal.magic.state = Spell::Returning;
    world.crystal.motion.pos = world.sabrina.motion.pos + crystal_orbit(&world, 0.0).offset;
    world.sabrina.cooldown = 8.0;
    let mut events = Events::default();
    crystal(&mut world, 1.0 / 60.0, Input::default(), &mut events);
    assert_eq!(world.crystal.magic.state, Spell::Resting);
    let before = world.crystal.motion.pos;
    world.session.time += 1.0;
    crystal(
        &mut world,
        1.0 / 60.0,
        Input {
            cast: true,
            ..Input::default()
        },
        &mut events,
    );
    assert_eq!(world.crystal.magic.state, Spell::Resting);
    assert!((world.crystal.motion.pos - before).length() > 0.2);
    assert!(!events.cast);
}
#[test]
fn safe_crystal_collection_and_empty_casts_award_nothing() {
    let mut world = encounter(true);
    world.fish[0].motion.pos = world.sabrina.motion.pos + crystal_orbit(&world, 0.0).offset;
    let mut events = Events::default();
    crystal(
        &mut world,
        0.01,
        Input {
            cast: true,
            ..Input::default()
        },
        &mut events,
    );
    assert!(events.teleported > 0);
    assert_eq!(world.points.score, 0);
}

#[test]
fn threatened_crystal_collection_earns_one_shared_rescue_reward() {
    let mut world = encounter(true);
    world.sabrina.motion.pos = v(-2.0, 8.0);
    let orbit = crystal_orbit(&world, 0.0);
    world.fish[0].motion.pos = world.sabrina.motion.pos + orbit.offset;
    let mut events = Events::default();
    crystal(
        &mut world,
        0.01,
        Input {
            cast: true,
            ..Input::default()
        },
        &mut events,
    );
    assert_eq!(world.points.score, RESCUE_POINTS);
    assert_eq!(world.points.visit_rescues, 1);
    award_rescue(&mut world, 0);
    award_rescue(&mut world, 0);
    assert_eq!(world.points.score, RESCUE_POINTS);
}

#[test]
fn wave_rescue_waits_for_survival_and_real_separation() {
    for escaped in [true, false] {
        let mut world = encounter(true);
        world.fish[0].rescue.wave_escape = 0.1;
        world.fish[0].motion.pos = if escaped { v(8.0, 8.0) } else { v(2.0, 8.0) };
        let mut events = Events::default();
        recovery_and_scoring(&mut world, 0.05, &mut events);
        assert_eq!(world.points.score, 0);
        recovery_and_scoring(&mut world, 0.06, &mut events);
        assert_eq!(world.points.score, if escaped { RESCUE_POINTS } else { 0 });
        recovery_and_scoring(&mut world, 0.1, &mut events);
        assert_eq!(world.points.visit_rescues, i32::from(escaped));
    }
}

#[test]
fn survivor_bonus_requires_every_visitor_defeated_and_never_pays_twice() {
    let mut world = encounter(true);
    let count = alive(&world);
    let mut events = Events::default();
    recovery_and_scoring(&mut world, 0.01, &mut events);
    assert!(!events.visit_ended);
    assert_eq!(world.points.score, 0);
    damage_jelly(&mut world, 0, 100.0, true, &mut events);
    recovery_and_scoring(&mut world, 0.01, &mut events);
    assert!(events.visit_ended);
    assert_eq!(world.points.last_score, 200 + count * SURVIVOR_POINTS);
    assert_eq!(
        world.points.rewards[ScoreFeedback::Crystal as usize].points,
        100
    );
    assert_eq!(
        world.points.rewards[ScoreFeedback::Retreat as usize].points,
        100
    );
    assert_eq!(
        world.points.rewards[ScoreFeedback::Survivors as usize].points,
        count * SURVIVOR_POINTS
    );
    let total = world.points.total_score;
    recovery_and_scoring(&mut world, 0.01, &mut events);
    assert_eq!(world.points.total_score, total);
}

#[test]
fn multiple_visitors_must_all_be_defeated_before_survivor_bonus() {
    let mut world = encounter(true);
    world.jelly[1].behaviour.state = JellyState::Hungry;
    let mut events = Events::default();
    damage_jelly(&mut world, 0, 100.0, true, &mut events);
    recovery_and_scoring(&mut world, 0.01, &mut events);
    assert!(!events.visit_ended);
    assert_eq!(world.points.score, 200);
    damage_jelly(&mut world, 1, 100.0, true, &mut events);
    recovery_and_scoring(&mut world, 0.01, &mut events);
    assert!(events.visit_ended);
    assert_eq!(world.points.score, 400 + alive(&world) * SURVIVOR_POINTS);
}

#[test]
fn song_requires_continuous_escort_from_danger_to_clear_water() {
    for interrupted in [false, true] {
        let mut world = encounter(true);
        world.sabrina.motion.pos = v(2.0, 8.0);
        world.sabrina.song.active = true;
        world.sabrina.song.strength = 1.0;
        world.fish[0].motion.pos = v(2.0, 8.0);
        fish::fish_swimming(&mut world, 0.5);
        assert_eq!(world.points.score, 0);
        if interrupted {
            world.sabrina.song.active = false;
            fish::fish_swimming(&mut world, 0.01);
            world.sabrina.song.active = true;
        }
        world.sabrina.motion.pos = v(9.0, 8.0);
        world.fish[0].motion.pos = v(8.0, 8.0);
        fish::fish_swimming(&mut world, 1.0);
        assert_eq!(
            world.points.score,
            if interrupted { 0 } else { RESCUE_POINTS }
        );
        world.fish[0].motion.pos = v(8.0, 8.0);
        fish::fish_swimming(&mut world, 1.0);
        assert_eq!(world.points.visit_rescues, i32::from(!interrupted));
    }
}

#[test]
fn actual_wave_rescue_needs_an_endangered_fish_that_survives() {
    for survived in [false, true] {
        let mut world = encounter(true);
        world.sabrina.motion.pos = v(0.0, 8.0);
        world.jelly[0].motion.pos = v(0.0, 8.5);
        world.fish[0].motion.pos = v(2.0, 8.5);
        let mut events = Events::default();
        water_wave(
            &mut world,
            0.01,
            Input {
                wave: true,
                ..Input::default()
            },
            &mut events,
        );
        for _ in 0..50 {
            water_wave(&mut world, 0.01, Input::default(), &mut events);
        }
        assert!(world.fish[0].rescue.wave_escape > 0.0);
        assert_eq!(world.points.score, 0);
        world.fish[0].life.active = survived;
        world.fish[0].motion.pos = v(8.0, 8.5);
        recovery_and_scoring(&mut world, 1.1, &mut events);
        assert_eq!(world.points.score, if survived { RESCUE_POINTS } else { 0 });
    }
}

#[test]
fn wipeout_overrides_round_completion_and_does_not_respawn_fish() {
    let mut world = encounter(true);
    for fish in &mut world.fish {
        fish.life.active = false;
    }
    world.jelly[0].behaviour.state = JellyState::Hidden;
    world.session.spawn_in = 0.0;
    let events = step(&mut world, 1.0 / 60.0, Input::default());
    assert!(events.game_over);
    assert!(!events.visit_ended);
    assert_eq!(alive(&world), 0);
    assert_eq!(world.points.total_score, 0);
    let time = world.session.time;
    step(&mut world, 1.0 / 60.0, Input::default());
    assert_eq!(world.session.time, time);
}

#[test]
fn last_fish_eaten_during_contact_ends_round_immediately() {
    let mut world = encounter(true);
    for fish in &mut world.fish {
        fish.life.active = false;
    }
    world.fish[0].life.active = true;
    world.fish[0].motion.pos = world.jelly[0].motion.pos;
    let events = step(&mut world, 1.0 / 60.0, Input::default());
    assert_eq!(events.eaten, 1);
    assert!(events.game_over);
    assert!(!events.visit_ended);
}

#[test]
fn aquarium_stays_noncompetitive_and_recovers_empty_population() {
    let mut world = encounter(true);
    world.settings.aquarium = true;
    world.session.water = Water::Calm;
    world.jelly[0].behaviour.state = JellyState::Hidden;
    for fish in &mut world.fish {
        fish.life.active = false;
    }
    award(&mut world, 100, ScoreFeedback::Crystal);
    let events = step(&mut world, 1.0 / 60.0, Input::default());
    assert!(!events.game_over);
    assert!(alive(&world) > 0);
    assert_eq!(world.points.score, 0);
}

#[test]
fn progression_is_monotonic_bounded_and_respects_visitor_limit() {
    let mut world = encounter(true);
    let mut previous = visitor_speed(&world);
    for cycle in 2..100 {
        world.session.cycle = cycle;
        let speed = visitor_speed(&world);
        assert!(speed > previous && speed < 1.7);
        assert!(multiple_chance(cycle) > multiple_chance(cycle - 1));
        previous = speed;
    }
    let mut counts = [0; 3];
    for (index, cycle) in [1, 5, 9].into_iter().enumerate() {
        for seed in 0..200 {
            let mut world = World::default();
            reset(
                &mut world,
                seed,
                Settings {
                    visitor_limit: 3,
                    ..Settings::default()
                },
            );
            world.session.cycle = cycle - 1;
            begin_visit(&mut world);
            counts[index] += hungry(&world);
            assert!(hungry(&world) <= 3);
        }
    }
    assert_eq!(counts[0], 200);
    assert!(counts[0] < counts[1] && counts[1] < counts[2]);
    for limit in 1..=3 {
        world.session.cycle = 8;
        world.settings.visitor_limit = limit;
        begin_visit(&mut world);
        assert!(hungry(&world) <= limit);
    }
}

#[test]
fn tenth_round_always_spawns_four_and_finishes_in_victory_once() {
    for gentle in [false, true] {
        for limit in 1..=3 {
            let mut world = encounter(gentle);
            world.settings.visitor_limit = limit;
            world.session.cycle = FINAL_ROUND - 1;
            begin_visit(&mut world);
            assert_eq!(world.session.cycle, 10);
            assert_eq!(hungry(&world), 4);
            let fourth = world.jelly[3].id;
            assert!(find_jelly(&world, fourth).is_some());
            let mut events = Events::default();
            for index in 0..4 {
                damage_jelly(&mut world, index, 100.0, true, &mut events);
                recovery_and_scoring(&mut world, 0.0, &mut events);
                assert_eq!(world.session.victory, index == 3);
            }
            assert!(events.victory && events.visit_ended && !events.game_over);
            let score = 4 * 200 + alive(&world) * SURVIVOR_POINTS;
            assert_eq!(world.points.total_score, score);
            let time = world.session.time;
            let position = world.sabrina.motion.pos;
            for _ in 0..120 {
                let events = step(
                    &mut world,
                    1.0 / 60.0,
                    Input {
                        movement: v(1.0, 1.0),
                        cast: true,
                        wave: true,
                        toggle_song: true,
                    },
                );
                assert!(!events.victory && !events.visit_ended && !events.game_over);
            }
            begin_visit(&mut world);
            finish_visit(&mut world, &mut Events::default());
            assert_eq!(world.session.cycle, 10);
            assert_eq!(world.session.time, time);
            assert_eq!(world.sabrina.motion.pos, position);
            assert_eq!(world.points.total_score, score);
            reset(&mut world, 27, Settings::default());
            assert!(!world.session.finished());
            assert_eq!(world.session.cycle, 0);
            assert!(find_jelly(&world, fourth).is_none());
        }
    }
}

#[test]
fn final_round_defeat_takes_priority_over_victory() {
    for lose_fish in [false, true] {
        let mut world = encounter(true);
        world.session.cycle = FINAL_ROUND;
        send_home(&mut world.jelly[0]);
        if lose_fish {
            for fish in &mut world.fish {
                fish.life.active = false;
            }
        } else {
            world.sabrina.vitality.health = 0.0;
        }
        let mut events = Events::default();
        recovery_and_scoring(&mut world, 0.0, &mut events);
        assert!(events.game_over && world.session.game_over);
        assert!(!events.victory && !events.visit_ended && !world.session.victory);
        assert_eq!(world.points.total_score, 0);
    }
}

#[test]
fn aquarium_remains_endless_past_the_final_round() {
    let mut world = World::default();
    reset(
        &mut world,
        27,
        Settings {
            aquarium: true,
            ..Settings::default()
        },
    );
    world.session.cycle = FINAL_ROUND;
    world.session.rest_remaining = 0.0;
    begin_visit(&mut world);
    for _ in 0..600 {
        step(&mut world, 1.0 / 60.0, Input::default());
    }
    assert!(!world.session.finished());
    assert_eq!(hungry(&world), 0);
    assert_eq!(world.session.water, Water::Calm);
}

#[test]
fn complete_game_has_exactly_ten_rounds_and_accumulates_each_score() {
    let mut world = World::default();
    reset(&mut world, 27, Settings::default());
    let mut total = 0;
    for round in 1..=FINAL_ROUND {
        begin_visit(&mut world);
        assert_eq!(world.session.cycle, round);
        assert_eq!(world.points.score, 0);
        let visitors = hungry(&world);
        assert_eq!(visitors == 4, round == FINAL_ROUND);
        let mut events = Events::default();
        for index in 0..JELLY_COUNT {
            damage_jelly(&mut world, index, 100.0, true, &mut events);
        }
        recovery_and_scoring(&mut world, 0.0, &mut events);
        total += visitors * 200 + alive(&world) * SURVIVOR_POINTS;
        assert_eq!(world.points.total_score, total);
        assert!(events.visit_ended);
        assert_eq!(events.victory, round == FINAL_ROUND);
    }
    begin_visit(&mut world);
    assert_eq!(world.session.cycle, FINAL_ROUND);
    assert_eq!(world.points.total_score, total);
}
