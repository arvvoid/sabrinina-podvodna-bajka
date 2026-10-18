// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! MIDI-rendered music, arrival cues, recorded voices and magic effects.
use crate::{
    app::{FrameEvents, Screen, SessionUi, Simulation},
    settings::Preferences,
    ui::tr,
};
use bevy::{audio::Volume, prelude::*};

#[derive(Resource, Default)]
pub struct AudioRequests {
    pub help: Option<usize>,
    pub stop_voice: bool,
    pub pause_voice: bool,
}

#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) enum Channel {
    Calm,
    Visit,
    Arrival,
    Song,
    Effect,
    Voice,
}

#[derive(Resource, Default)]
struct MusicState {
    visiting: bool,
    visit: Handle<AudioSource>,
    arrival: Handle<AudioSource>,
}

pub struct ReefAudioPlugin;

impl Plugin for ReefAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioRequests>()
            .init_resource::<MusicState>()
            .add_systems(Startup, setup)
            .add_systems(Update, update);
    }
}

fn setup(mut commands: Commands, server: Res<AssetServer>, mut music: ResMut<MusicState>) {
    music.visit = server.load("audio/music_visit.ogg");
    music.arrival = server.load("audio/jelly_appears.ogg");
    for (channel, source) in [
        (Channel::Calm, server.load("audio/music_calm.ogg")),
        (Channel::Song, server.load("audio/siren_song.ogg")),
    ] {
        commands.spawn((
            channel,
            AudioPlayer::new(source),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
        ));
    }
}

fn update(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut requests: ResMut<AudioRequests>,
    mut music: ResMut<MusicState>,
    mut events: ResMut<FrameEvents>,
    simulation: Res<Simulation>,
    mut ui: ResMut<SessionUi>,
    preferences: Res<Preferences>,
    time: Res<Time>,
    mut channels: Query<(Entity, &Channel, Option<&mut AudioSink>)>,
) {
    let mut voice_playing = channels
        .iter()
        .any(|(_, channel, sink)| *channel == Channel::Voice && sink.is_none_or(|s| !s.empty()));
    if requests.stop_voice {
        for (entity, channel, _) in &channels {
            if *channel == Channel::Voice {
                commands.entity(entity).despawn();
            }
        }
        ui.caption.clear();
        voice_playing = false;
        requests.stop_voice = false;
    }
    if requests.pause_voice {
        for (_, channel, sink) in &channels {
            if *channel == Channel::Voice
                && let Some(sink) = sink
            {
                sink.toggle_playback()
            }
        }
        requests.pause_voice = false;
    }
    if let Some(lang) = requests.help.take() {
        for (entity, channel, _) in &channels {
            if *channel == Channel::Voice {
                commands.entity(entity).despawn();
            }
        }
        commands.spawn((
            Channel::Voice,
            AudioPlayer::new(server.load(format!("audio/help_{}.ogg", ["en", "hr", "it"][lang]))),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(preferences.voice)),
        ));
        voice_playing = true;
    }
    let mut effect = None;
    let mut voice = None;
    let mut arrival = false;
    // Later events intentionally take priority for the single effect/voice slot.
    for event in events.0.drain(..) {
        if event.song_empty {
            ui.notice = tr(
                "The song is resting. It must recharge fully.",
                preferences.language,
            );
            ui.notice_time = 4.0;
        }
        if event.wave_cast {
            effect = Some(("magic_burst", 0.65))
        }
        if event.cast {
            effect = Some(("magic_charge", 1.0))
        }
        if event.teleported > 0 {
            effect = Some(("magic_collect", 1.1))
        }
        if event.jelly_hit {
            effect = Some(("magic_burst", 0.8));
        }
        if event.repelled > 0 {
            effect = Some(("magic_burst", 1.0));
            voice = Some(("jelly_repelled", "That mermaid and her crystal!"));
        }
        if event.hit {
            effect = Some(("sting", 1.0));
            ui.notice = tr(
                "A little sting. Sabrina will recover soon.",
                preferences.language,
            );
            ui.notice_time = 4.0;
        }
        if event.visit_started {
            arrival = true;
            voice = Some(("jelly_arrive", "I'm hungry! Where are my little fish?"));
        }
        if event.eaten > 0 {
            voice = Some(("jelly_eat", "Oops! I ate you!"));
        }
        if event.leaving {
            voice = Some(("jelly_leave", "The jellyfish is swimming away."));
        }
        if event.game_over {
            requests.stop_voice = true;
            ui.notice_time = 0.0;
            ui.score_time = 0.0;
        }
    }
    if let Some((name, speed)) = effect {
        for (entity, channel, _) in &channels {
            if *channel == Channel::Effect {
                commands.entity(entity).despawn();
            }
        }
        commands.spawn((
            Channel::Effect,
            AudioPlayer::new(server.load(format!("audio/{name}.ogg"))),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(preferences.effects))
                .with_speed(speed),
        ));
    }
    if let Some((name, caption)) = voice
        && !voice_playing
    {
        ui.caption = tr(caption, preferences.language);
        commands.spawn((
            Channel::Voice,
            AudioPlayer::new(server.load(format!("audio/{name}.ogg"))),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(preferences.voice)),
        ));
        voice_playing = true;
    }
    let duck =
        if voice_playing { 0.25 } else { 1.0 } * (1.0 - simulation.0.sabrina.song.strength * 0.60);
    let visit = ui.started
        && !simulation.0.session.game_over
        && simulation.0.session.water == reef_sim::Water::Visit;
    if visit && (arrival || !music.visiting) {
        // Looped Ogg sources cannot seek; a fresh player starts each visit at bar one.
        for (entity, channel, _) in &channels {
            if matches!(channel, Channel::Visit | Channel::Arrival) {
                commands.entity(entity).despawn();
            }
        }
        commands.spawn((
            Channel::Visit,
            AudioPlayer::new(music.visit.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
        ));
        commands.spawn((
            Channel::Arrival,
            AudioPlayer::new(music.arrival.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(preferences.music * duck)),
        ));
    }
    music.visiting = visit;
    for (entity, channel, sink) in &mut channels {
        if *channel == Channel::Arrival && (!ui.started || simulation.0.session.game_over) {
            commands.entity(entity).despawn();
            continue;
        }
        let Some(mut sink) = sink else { continue };
        let target = match channel {
            Channel::Calm => preferences.music * duck * if visit { 0.0 } else { 1.0 },
            Channel::Visit => preferences.music * duck * if visit { 1.0 } else { 0.0 },
            Channel::Arrival => preferences.music * duck,
            Channel::Song => preferences.effects * 0.65 * simulation.0.sabrina.song.strength,
            Channel::Effect => preferences.effects,
            Channel::Voice => preferences.voice,
        };
        if matches!(channel, Channel::Calm | Channel::Visit) {
            let volume = sink.volume().to_linear();
            sink.set_volume(Volume::Linear(
                volume + (target - volume) * (time.delta_secs() * 2.0).min(1.0),
            ));
        } else {
            sink.set_volume(Volume::Linear(target));
        }
        if *channel == Channel::Song {
            if simulation.0.sabrina.song.strength > 0.01 && ui.screen == Screen::Playing {
                sink.play();
            } else {
                if !sink.is_paused() {
                    sink.pause();
                    let _ = sink.try_seek(std::time::Duration::ZERO);
                }
            }
        }
    }
    if !voice_playing {
        ui.caption.clear();
    }
}
