// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Recorded voices and effects layered over synthesized, ducked music loops.

// The melody generator retains the original waveform constants.
#![allow(clippy::approx_constant)]
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
enum Channel {
    Calm,
    Visit,
    Song,
    Effect,
    Voice,
}

pub struct ReefAudioPlugin;

impl Plugin for ReefAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioRequests>()
            .add_systems(Startup, setup)
            .add_systems(Update, update);
    }
}

fn compose(visit: bool) -> AudioSource {
    // Write a mono 16-bit PCM WAV in memory; no generated audio file is needed.
    let sample_rate = 22050_u32;
    let beat = if visit { 0.45_f32 } else { 0.65 };
    let sample_count = (sample_rate as f32 * beat * 64.0) as u32;
    let mut data = Vec::with_capacity(sample_count as usize * 2 + 44);
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + sample_count * 2).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16_u32.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&sample_rate.to_le_bytes());
    data.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    data.extend_from_slice(&2_u16.to_le_bytes());
    data.extend_from_slice(&16_u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&(sample_count * 2).to_le_bytes());
    let melody = [
        76, 0, 79, 0, 81, 79, 76, 0, 74, 0, 72, 0, 74, 76, 79, 0, 76, 0, 72, 0, 69, 72, 76, 0, 74,
        0, 76, 79, 74, 0, 72, 0,
    ];
    let roots = [48, 45, 41, 43];
    for n in 0..sample_count {
        let time = n as f32 / sample_rate as f32;
        let step = (time / beat) as usize;
        let bar = step / 16;
        let local = time - step as f32 * beat;
        let mut sample = 0.0;
        let note = melody[step % 32];
        if note != 0 {
            let hz = 440.0 * 2_f32.powf((note - 69) as f32 / 12.0);
            let envelope = (local / 0.015).min(1.0) * (-local * 5.0).exp();
            sample += 0.25
                * envelope
                * ((6.283185 * hz * local).sin() + 0.2 * (6.283185 * hz * 2.0 * local).sin());
        }
        let bar_time = time - bar as f32 * 16.0 * beat;
        let fade = (bar_time / 0.6)
            .min(1.0)
            .min((16.0 * beat - bar_time) / 0.8);
        for offset in [0, 7, 16] {
            let hz = 440.0 * 2_f32.powf((roots[bar % 4] + offset - 69) as f32 / 12.0);
            sample += 0.055 * fade * (6.283185 * hz * bar_time).sin();
        }
        if visit {
            sample += 0.025 * (local * 6.283185 * 110.0).sin() * (-local * 18.0).exp();
        }
        data.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 22000.0) as i16).to_le_bytes());
    }
    AudioSource { bytes: data.into() }
}

fn setup(
    mut commands: Commands,
    mut assets: ResMut<Assets<AudioSource>>,
    server: Res<AssetServer>,
) {
    for (channel, source) in [
        (Channel::Calm, assets.add(compose(false))),
        (Channel::Visit, assets.add(compose(true))),
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
    for (_, channel, sink) in &mut channels {
        let Some(mut sink) = sink else { continue };
        let target = match channel {
            Channel::Calm => preferences.music * duck * if visit { 0.12 } else { 1.0 },
            Channel::Visit => preferences.music * duck * if visit { 1.0 } else { 0.0 },
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
